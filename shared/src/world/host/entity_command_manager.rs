use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use super::{
    entity_command::EntityCommand, host_world_manager::CommandId,
};
use crate::{ChannelSender, EntityMessage, EntityMessageReceiver, GlobalEntity, Instant, ReliableSender, PacketIndex, HostType, ComponentKind};
use crate::sequence_list::SequenceList;
use crate::world::sync::EntityChannel;

const COMMAND_RECORD_TTL: Duration = Duration::from_secs(60);
const RESEND_COMMAND_RTT_FACTOR: f32 = 1.5;

/// Channel to perform ECS replication between server and client
/// Only handles entity commands (Spawn/despawn entity and insert/remove components)
/// Will use a reliable sender.
/// Will wait for acks from the client to know the state of the client's ECS world ("remote")
pub struct EntityCommandManager {
    outgoing_commands: ReliableSender<EntityCommand>,
    sent_command_packets: SequenceList<(Instant, Vec<(CommandId, EntityMessage<GlobalEntity>)>)>,
    delivered_commands: EntityMessageReceiver<GlobalEntity>,
}

impl EntityCommandManager {
    pub fn new(host_type: HostType) -> Self {
        Self {
            outgoing_commands: ReliableSender::new(RESEND_COMMAND_RTT_FACTOR),
            sent_command_packets: SequenceList::new(),
            delivered_commands: EntityMessageReceiver::new(host_type.invert()),
        }
    }

    // Collect

    pub fn take_outgoing_commands(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
    ) -> VecDeque<(CommandId, EntityCommand)> {
        self.outgoing_commands.collect_messages(now, rtt_millis);
        self.outgoing_commands.take_next_messages()
    }
    
    pub fn send_outgoing_command(
        &mut self,
        command: EntityCommand,
    ) {
        self.outgoing_commands.send_message(command);
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
        self.delivered_commands.receive_messages(false)
    }

    pub fn track_remote_entity(
        &mut self,
        entity: &GlobalEntity,
        component_kinds: &Vec<ComponentKind>,
    ) {
        self.delivered_commands.track_hosts_redundant_remote_entity(entity, component_kinds);
    }
    
    pub fn untrack_remote_entity(
        &mut self,
        entity: &GlobalEntity,
    ) {
        self.delivered_commands.untrack_hosts_redundant_remote_entity(entity);
    }

    pub(crate) fn get_remote_world(&self) -> &HashMap<GlobalEntity, EntityChannel> {
        self.delivered_commands.get_remote_world()
    }
}
