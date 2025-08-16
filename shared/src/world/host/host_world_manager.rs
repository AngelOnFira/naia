use std::{time::Duration, collections::{HashMap, HashSet, VecDeque}};

use crate::{sequence_list::SequenceList, world::{
    sync::{SenderEngine, EntityChannelReceiver, EntityChannelSender},
    host::entity_update_manager::EntityUpdateManager,
}, EntityMessage, EntityMessageReceiver, GlobalEntity, Instant, PacketIndex, HostType, ComponentKind, HostEntityGenerator, MessageIndex, EntityCommand, PacketNotifiable, LocalEntityMap, HostEntity, EntityConverterMut, GlobalWorldManagerType, ReliableSender, ChannelSender, ShortMessageIndex};

const COMMAND_RECORD_TTL: Duration = Duration::from_secs(60);
const RESEND_COMMAND_RTT_FACTOR: f32 = 1.5;

pub type CommandId = MessageIndex;
pub type SubCommandId = ShortMessageIndex;

/// Channel to perform ECS replication between server and client
/// Only handles entity commands (Spawn/despawn entity and insert/remove components)
/// Will use a reliable sender.
/// Will wait for acks from the client to know the state of the client's ECS world ("remote")
pub struct HostWorldManager {

    // host entity generator
    entity_generator: HostEntityGenerator,

    // sender
    sender: ReliableSender<EntityCommand>,

    // For Server, this contains the Entities that the Server has authority over, that it syncs to the Client
    // For Client, this contains the non-Delegated Entities that the Client has authority over, that it syncs to the Server
    host_engine: SenderEngine,

    // sent packets
    sent_command_packets: SequenceList<(Instant, Vec<(CommandId, EntityMessage<GlobalEntity>)>)>,

    // For Server, this contains the Entities that the Server has authority over, that have been delivered to the Client
    // For Client, this contains the non-Delegated Entities that the Client has authority over, that have been delivered to the Server
    delivered_commands: EntityMessageReceiver<GlobalEntity>,
}

impl HostWorldManager {
    pub fn new(host_type: HostType, user_key: u64) -> Self {
        Self {
            entity_generator: HostEntityGenerator::new(user_key),
            sender: ReliableSender::new(RESEND_COMMAND_RTT_FACTOR),
            host_engine: SenderEngine::new(true, host_type),
            sent_command_packets: SequenceList::new(),
            delivered_commands: EntityMessageReceiver::new(),
        }
    }

    pub(crate) fn entity_converter_mut<'a, 'b>(
        &'b mut self,
        global_world_manager: &'a dyn GlobalWorldManagerType,
        entity_map: &'b mut LocalEntityMap,
    ) -> EntityConverterMut<'a, 'b> {
        EntityConverterMut::new(global_world_manager, entity_map, &mut self.entity_generator)
    }

    // Collect

    pub fn take_outgoing_events(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
        delegated_world_opt: Option<&mut SenderEngine>,
    ) -> VecDeque<(CommandId, EntityCommand)> {
        for outgoing_command in self.host_engine.take_outgoing_commands() {
            self.sender.send_message(outgoing_command);
        }
        if let Some(delegated_world) = delegated_world_opt {
            for outgoing_command in delegated_world.take_outgoing_commands() {
                self.sender.send_message(outgoing_command);
            }
        }
        self.sender.collect_messages(now, rtt_millis);
        self.sender.take_next_messages()
    }

    pub fn send_outgoing_command(
        &mut self,
        command: EntityCommand,
    ) {
        self.host_engine.accept_command(command);
    }

    pub(crate) fn host_generate_entity(&mut self) -> HostEntity {
        self.entity_generator.generate_host_entity()
    }
    
    pub(crate) fn host_reserve_entity(
        &mut self,
        entity_map: &mut LocalEntityMap,
        global_entity: &GlobalEntity,
    ) -> HostEntity {
        self.entity_generator.host_reserve_entity(entity_map, global_entity)
    }

    pub(crate) fn host_removed_reserved_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> Option<HostEntity> {
        self.entity_generator.host_remove_reserved_entity(global_entity)
    }

    pub(crate) fn host_has_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.get_host_world().contains_key(global_entity)
    }

    // used when Entity first comes into Connection's scope
    pub fn host_init_entity(
        &mut self,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) {
        // add entity
        self.host_spawn_entity(global_entity);
        // add components
        for component_kind in component_kinds {
            self.host_insert_component(global_entity, &component_kind);
        }
    }

    fn host_spawn_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.host_engine.accept_command(EntityCommand::Spawn(*global_entity));
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.host_engine.accept_command(EntityCommand::Despawn(*global_entity));
    }

    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host_engine.accept_command(EntityCommand::InsertComponent(*global_entity, *component_kind));
    }

    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host_engine.accept_command(EntityCommand::RemoveComponent(*global_entity, *component_kind));
    }

    pub fn remote_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        todo!("close entity channel?");
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

    pub(crate) fn record_command_written(
        &mut self,
        packet_index: &PacketIndex,
        command_id: &CommandId,
        message: EntityMessage<GlobalEntity>,
    ) {
        let (_, sent_actions_list) = self.sent_command_packets.get_mut_scan_from_back(packet_index).unwrap();
        sent_actions_list.push((*command_id, message));
    }

    pub(crate) fn handle_dropped_command_packets(&mut self, now: &Instant) {
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

    pub(crate) fn get_host_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        self.host_engine.get_world()
    }

    pub(crate) fn get_remote_world(&self) -> &HashMap<GlobalEntity, EntityChannelReceiver> {
        self.delivered_commands.get_world()
    }

    pub(crate) fn get_updatable_world(&self) -> HashMap<GlobalEntity, HashSet<ComponentKind>> {
        let mut output = HashMap::new();
        for (global_entity, host_channel) in self.get_host_world() {

            let Some(remote_channel) = self.get_remote_world().get(global_entity) else {
                continue;
            };
            let host_component_kinds = host_channel.component_kinds();
            let joined_component_kinds = remote_channel.component_kinds_intersection(host_component_kinds);
            if joined_component_kinds.is_empty() {
                continue;
            }
            output.insert(*global_entity, joined_component_kinds);
        }
        output
    }

    pub(crate) fn process_received_commands(
        &mut self,
        local_entity_map: &mut LocalEntityMap,
        entity_update_manager: &mut EntityUpdateManager,
    ) {
        for command in self.delivered_commands.receive_messages() {
            match command {
                EntityMessage::Spawn(entity) => {
                    self.on_remote_spawn_entity(&entity);
                }
                EntityMessage::Despawn(entity) => {
                    self.on_remote_despawn_entity(local_entity_map, &entity);
                }
                EntityMessage::InsertComponent(entity, component_kind) => {
                    self.on_remote_insert_component(entity_update_manager, &entity, &component_kind);
                }
                EntityMessage::RemoveComponent(entity, component) => {
                    self.on_remote_remove_component(entity_update_manager, &entity, &component);
                }
                EntityMessage::Noop => {
                    // do nothing
                }
                _ => {
                    // Only Auth-related messages are left here
                    // Right now it doesn't seem like we need to track auth state here
                }
            }
        }
    }

    fn on_remote_spawn_entity(
        &mut self,
        _global_entity: &GlobalEntity,
    ) {
        // stubbed
    }

    pub fn on_remote_despawn_entity(
        &mut self,
        local_entity_map: &mut LocalEntityMap,
        global_entity: &GlobalEntity,
    ) {
        self.entity_generator.remove_by_global_entity(local_entity_map, global_entity);
    }

    fn on_remote_insert_component(
        &mut self,
        entity_update_manager: &mut EntityUpdateManager,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        entity_update_manager.register_component(global_entity, component_kind);
    }

    fn on_remote_remove_component(
        &mut self,
        entity_update_manager: &mut EntityUpdateManager,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        entity_update_manager.deregister_component(global_entity, component_kind);
    }
}

impl PacketNotifiable for HostWorldManager {
    fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        if let Some((_, command_list)) = self
            .sent_command_packets
            .remove_scan_from_front(&packet_index)
        {
            for (command_id, command) in command_list {
                if self.sender.deliver_message(&command_id).is_some() {
                    self.delivered_commands.buffer_message(command_id, command);
                }
            }
        }
    }
}