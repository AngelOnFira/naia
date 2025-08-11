use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use crate::{sequence_list::SequenceList, world::{entity::entity_message_sender::EntityMessageSender, sync::{EntityChannelReceiver, EntityChannelSender}}, EntityMessage, EntityMessageReceiver, GlobalEntity, Instant, PacketIndex, HostType, ComponentKind, LocalWorldManager, MessageIndex, EntityCommand, PacketNotifiable};

const COMMAND_RECORD_TTL: Duration = Duration::from_secs(60);
const RESEND_COMMAND_RTT_FACTOR: f32 = 1.5;

pub type CommandId = MessageIndex;

/// Channel to perform ECS replication between server and client
/// Only handles entity commands (Spawn/despawn entity and insert/remove components)
/// Will use a reliable sender.
/// Will wait for acks from the client to know the state of the client's ECS world ("remote")
pub struct HostWorldManager {

    // For Server, this contains the Entities that the Server has authority over, that it syncs to the Client
    // For Client, this contains the non-Delegated Entities that the Client has authority over, that it syncs to the Server
    outgoing_commands: EntityMessageSender,

    // sent packets
    sent_command_packets: SequenceList<(Instant, Vec<(CommandId, EntityMessage<GlobalEntity>)>)>,

    // For Server, this contains the Entities that the Server has authority over, that have been delivered to the Client
    // For Client, this contains the non-Delegated Entities that the Client has authority over, that have been delivered to the Server
    delivered_commands: EntityMessageReceiver<GlobalEntity>,
}

impl HostWorldManager {
    pub fn new(host_type: HostType) -> Self {
        Self {
            outgoing_commands: EntityMessageSender::new(host_type, RESEND_COMMAND_RTT_FACTOR),
            sent_command_packets: SequenceList::new(),
            delivered_commands: EntityMessageReceiver::new(host_type.invert()),
        }
    }

    // Collect

    pub fn take_outgoing_events(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
    ) -> VecDeque<(CommandId, EntityCommand)> {
        self.outgoing_commands.take_outgoing_commands(now, rtt_millis)
    }
    
    pub fn send_outgoing_command(
        &mut self,
        command: EntityCommand,
    ) {
        self.outgoing_commands.send_outgoing_command(command);
    }

    pub fn host_has_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.get_host_world().contains_key(global_entity)
    }

    // used when Entity first comes into Connection's scope
    pub fn host_init_entity(
        &mut self,
        world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) {
        // add entity
        self.host_spawn_entity(world_manager, global_entity);
        // add components
        for component_kind in component_kinds {
            self.host_insert_component(global_entity, &component_kind);
        }
    }

    pub fn host_spawn_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
    ) {
        self.outgoing_commands.host_spawn_entity(local_world_manager, global_entity);
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.outgoing_commands.host_despawn_entity(global_entity);
    }

    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.outgoing_commands.host_insert_component(global_entity, component_kind);
    }

    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.outgoing_commands.host_remove_component(global_entity, component_kind);
    }

    pub fn remote_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.outgoing_commands.remote_despawn_entity(global_entity);
    }

    pub(crate) fn insert_sent_command_packet(&mut self, packet_index: &PacketIndex, now: Instant) {
        if !self
            .sent_command_packets
            .contains_scan_from_back(packet_index)
        {
            self
                .sent_command_packets
                .insert_scan_from_back(*packet_index, (now, Vec::new()));
        }
    }

    pub fn record_command_written(
        &mut self,
        packet_index: &PacketIndex,
        command_id: &CommandId,
        message: EntityMessage<GlobalEntity>,
    ) {
        let (_, sent_actions_list) = self.sent_command_packets.get_mut_scan_from_back(packet_index).unwrap();
        sent_actions_list.push((*command_id, message));
    }

    pub fn handle_dropped_command_packets(&mut self, now: &Instant) {
        let mut pop = false;

        loop {
            if let Some((_, (time_sent, _))) = self.sent_command_packets.front() {
                if time_sent.elapsed(now) > COMMAND_RECORD_TTL {
                    pop = true;
                }
            } else {
                return;
            }
            if pop {
                self.sent_command_packets.pop_front();
            } else {
                return;
            }
        }
    }
    
    pub fn notify_packet_delivered(
        &mut self,
        packet_index: PacketIndex,
    ) {
        if let Some((_, command_list)) = self
            .sent_command_packets
            .remove_scan_from_front(&packet_index)
        {
            for (command_id, command) in command_list {
                if self.outgoing_commands.deliver_message(&command_id).is_some() {
                    self.delivered_commands.buffer_message(command_id, command);
                }
            }
        }
    }
    
    pub fn take_delivered_commands(
        &mut self,
    ) -> Vec<EntityMessage<GlobalEntity>> {
        self.delivered_commands.receive_messages()
    }

    pub fn get_host_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        self.outgoing_commands.get_world()
    }

    pub fn get_remote_world(&self) -> &HashMap<GlobalEntity, EntityChannelReceiver> {
        self.delivered_commands.get_world()
    }

    pub fn entity_release_authority(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.send_outgoing_command(EntityCommand::ReleaseAuthority(*global_entity));
    }
}

impl PacketNotifiable for HostWorldManager {
    fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        self.notify_packet_delivered(packet_index);
    }
}