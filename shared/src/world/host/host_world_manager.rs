use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
    net::SocketAddr,
    sync::RwLockReadGuard,
};

use log::info;

use crate::{world::{
    host::{entity_command_manager::EntityCommandManager, checked_map::{CheckedMap, CheckedSet}, entity_update_manager::EntityUpdateManager},
    entity::entity_converters::GlobalWorldManagerType, local_world_manager::LocalWorldManager,
}, ComponentKind, DiffMask, EntityAndGlobalEntityConverter, EntityCommand, EntityMessage, GlobalEntity, HostEntity, HostType, Instant, MessageIndex, PacketIndex, WorldRefType};

pub type CommandId = MessageIndex;

/// Manages Entities for a given Client connection and keeps them in
/// sync on the Client
pub struct HostWorldManager {
    entity_command_manager: EntityCommandManager,
    entity_update_manager: EntityUpdateManager,
}

pub struct HostWorldEvents {
    pub next_send_commands: VecDeque<(CommandId, EntityCommand)>,
    pub next_send_updates: HashMap<GlobalEntity, HashSet<ComponentKind>>,
}

impl HostWorldEvents {
    pub fn has_events(&self) -> bool {
        !self.next_send_commands.is_empty() || !self.next_send_updates.is_empty()
    }
}

impl HostWorldManager {
    /// Create a new HostWorldManager, given the client's address
    pub fn new(
        host_type: HostType,
        address: &Option<SocketAddr>,
        global_world_manager: &dyn GlobalWorldManagerType,
    ) -> Self {
        Self {
            // host_world: CheckedMap::new(),
            // remote_world: CheckedMap::new(),
            entity_command_manager: EntityCommandManager::new(host_type),
            entity_update_manager: EntityUpdateManager::new(address, global_world_manager),
        }
    }
    
    // Host World

    pub fn host_has_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.entity_command_manager.get_host_world().contains_key(global_entity)
    }

    pub fn host_component_kinds(&self, entity: &GlobalEntity) -> Vec<ComponentKind> {
        if let Some(entity_channel) = self.entity_command_manager.get_host_world().get(entity) {
            entity_channel.component_kinds().iter().cloned().collect()
        } else {
            Vec::new()
        }
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

    fn host_spawn_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
    ) {
        self.entity_command_manager.host_spawn_entity(local_world_manager, global_entity);
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.entity_command_manager.host_despawn_entity(global_entity);
    }

    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.entity_command_manager.host_insert_component(global_entity, component_kind);
    }

    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.entity_command_manager.host_remove_component(global_entity, component_kind);
    }
    
    // Remote World

    pub fn remote_despawn_entity(&mut self, local_world_manager: &mut LocalWorldManager, global_entity: &GlobalEntity) {
        self.entity_command_manager.remote_despawn_entity(global_entity);
        self.on_remote_despawn_entity(local_world_manager, global_entity);
    }
    
    fn on_remote_despawn_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
    ) {
        local_world_manager.remove_by_global_entity(global_entity);
    }
    
    fn on_remote_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.entity_update_manager.register_component(global_entity, component_kind);
    }
    
    fn on_remote_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.entity_update_manager.deregister_component(global_entity, component_kind);
    }
    
    // Tracking Remote Entities

    // used when Remote Entity gains Write Authority (delegation)
    pub fn track_remote_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) -> HostEntity {
        let new_host_entity = self.entity_command_manager.track_remote_entity(local_world_manager, global_entity, &component_kinds);

        // add components
        for component_kind in component_kinds {
            self.track_remote_component(global_entity, &component_kind);
        }

        // info!("--- ---------------------- ---");

        new_host_entity
    }

    pub fn untrack_remote_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
    ) {
        let components: Vec<ComponentKind> = self.entity_command_manager.untrack_remote_entity(local_world_manager, global_entity).iter().copied().collect();

        for component in components {
            self.untrack_remote_component(global_entity, &component);
        }
    }

    pub fn track_remote_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.entity_command_manager.track_remote_component(global_entity, component_kind);
        self.entity_update_manager.register_component(global_entity, component_kind);
    }

    pub fn untrack_remote_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.entity_command_manager.untrack_remote_component(global_entity, component_kind);
        self.entity_update_manager.deregister_component(global_entity, component_kind);
    }

    // Messages

    pub fn insert_sent_command_packet(&mut self, packet_index: &PacketIndex, now: Instant) {
        self.entity_command_manager.insert_sent_command_packet(packet_index, now);
    }
    
    pub fn record_command_written(
        &mut self,
        packet_index: &PacketIndex,
        command_id: &CommandId,
        message: EntityMessage<GlobalEntity>,
    ) {
        self.entity_command_manager.record_command_written(packet_index, command_id, message);
    }

    pub fn handle_dropped_packets(&mut self, now: &Instant, rtt_millis: &f32) {
        self.entity_update_manager.handle_dropped_update_packets(now, rtt_millis);
        self.entity_command_manager.handle_dropped_command_packets(now);
    }

    pub fn send_outgoing_command(
        &mut self,
        command: EntityCommand,
    ) {
        match &command {
            EntityCommand::Spawn(_) | EntityCommand::Despawn(_) | EntityCommand::InsertComponent(_, _) | EntityCommand::RemoveComponent(_, _) => {}
            command => {
                info!("HostWorldManager: sending entity command: {:?}", command);
            }
        }
        self.entity_command_manager.send_outgoing_command(command);
    }

    pub fn take_outgoing_events<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        &mut self,
        world: &W,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        now: &Instant,
        rtt_millis: &f32,
    ) -> HostWorldEvents {
        let next_send_commands = self.entity_command_manager.take_outgoing_commands(now, rtt_millis);
        let host_world = self.entity_command_manager.get_host_world();
        let remote_world = self.entity_command_manager.get_remote_world();
        HostWorldEvents {
            next_send_commands,
            next_send_updates: self.entity_update_manager.collect_next_updates(
                world,
                converter,
                global_world_manager,
                host_world,
                remote_world,
            ),
        }
    }

    pub fn notify_packet_delivered(
        &mut self,
        packet_index: PacketIndex,
        local_world_manager: &mut LocalWorldManager,
    ) {
        // Updates
        self.entity_update_manager.notify_packet_delivered(packet_index);

        // Commands
        self.entity_command_manager.notify_packet_delivered(packet_index);
        self.process_delivered_commands(local_world_manager)
    }

    fn process_delivered_commands(&mut self, local_world_manager: &mut LocalWorldManager) {
        let delivered_commands = self.entity_command_manager.take_delivered_commands();
        for command in delivered_commands {
            match command {
                EntityMessage::Spawn(_entity) => {}
                EntityMessage::Despawn(entity) => {
                    self.on_remote_despawn_entity(local_world_manager, &entity);
                }
                EntityMessage::InsertComponent(entity, component_kind) => {
                    self.on_remote_insert_component(&entity, &component_kind);
                }
                EntityMessage::RemoveComponent(entity, component) => {
                    self.on_remote_remove_component(&entity, &component);
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

    // Updates

    pub fn get_diff_mask(
        &self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) -> RwLockReadGuard<DiffMask> {
        self.entity_update_manager.get_diff_mask(global_entity, component_kind)
    }

    pub fn record_update(
        &mut self,
        now: &Instant,
        packet_index: &PacketIndex,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
        diff_mask: DiffMask
    ) {
        self.entity_update_manager.record_update(
            now,
            packet_index,
            global_entity,
            component_kind,
            diff_mask,
        );
    }

    // Auth

    pub fn entity_release_authority(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.entity_command_manager.send_outgoing_command(EntityCommand::ReleaseAuthority(*global_entity));
    }
}