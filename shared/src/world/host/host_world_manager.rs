use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
    net::SocketAddr,
    sync::RwLockReadGuard,
};

use log::info;

use crate::{world::{
    host::{entity_command_sender::EntityCommandManager, checked_map::{CheckedMap, CheckedSet}, entity_update_manager::EntityUpdateManager},
    entity::entity_converters::GlobalWorldManagerType, local_world_manager::LocalWorldManager,
}, ComponentKind, DiffMask, EntityAndGlobalEntityConverter, EntityCommand, EntityMessage, GlobalEntity, HostEntity, HostType, Instant, MessageIndex, PacketIndex, WorldRefType};

pub type CommandId = MessageIndex;

/// Manages Entities for a given Client connection and keeps them in
/// sync on the Client
pub struct HostWorldManager {
    host_world: CheckedMap<GlobalEntity, CheckedSet<ComponentKind>>,
    remote_world: CheckedMap<GlobalEntity, CheckedSet<ComponentKind>>,
    
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
            host_world: CheckedMap::new(),
            remote_world: CheckedMap::new(),
            entity_command_manager: EntityCommandManager::new(host_type),
            entity_update_manager: EntityUpdateManager::new(address, global_world_manager),
        }
    }
    
    // Host World

    pub fn host_has_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.host_world.contains_key(global_entity)
    }

    pub fn host_component_kinds(&self, entity: &GlobalEntity) -> Vec<ComponentKind> {
        if let Some(component_kinds) = self.host_world.get(entity) {
            component_kinds.iter().cloned().collect()
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
        self.host_world.insert(*global_entity, CheckedSet::new());

        on_entity_channel_opening(local_world_manager, global_entity);
        
        self.entity_command_manager.send_outgoing_command(EntityCommand::SpawnEntity(*global_entity));
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.host_world.remove(global_entity);
        
        self.entity_command_manager.send_outgoing_command(EntityCommand::DespawnEntity(*global_entity));
    }

    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        let Some(components) = self.host_world.get_mut(global_entity) else {
            panic!("World Channel: cannot insert component into entity that doesn't exist");
        };
        components.insert(*component_kind);

        self.entity_command_manager.send_outgoing_command(EntityCommand::InsertComponent(*global_entity, *component_kind));
    }

    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        let Some(components) = self.host_world.get_mut(global_entity) else {
            panic!("World Channel: cannot remove component from non-existent entity");
        };
        components.remove(component_kind);

        self.entity_command_manager.send_outgoing_command(EntityCommand::RemoveComponent(*global_entity, *component_kind));
    }
    
    // Remote World

    pub fn remote_despawn_entity(&mut self, local_world_manager: &mut LocalWorldManager, global_entity: &GlobalEntity) {
        self.host_world.remove(global_entity);
        self.on_remote_despawn_entity(local_world_manager, global_entity);
    }
    
    fn on_remote_spawn_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.remote_world.insert(*global_entity, CheckedSet::<ComponentKind>::new());
    }
    
    fn on_remote_despawn_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
    ) {
        self.remote_world.remove(global_entity);
        on_remote_entity_channel_closed(local_world_manager, global_entity);
    }
    
    fn on_remote_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        let Some(components) = self.remote_world.get_mut(global_entity) else {
            panic!("World Channel: cannot insert component into remote entity that doesn't exist");
        };
        components.insert(*component_kind);

        self.entity_update_manager.register_component(global_entity, component_kind);
    }
    
    fn on_remote_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        let Some(components) = self.remote_world.get_mut(global_entity) else {
            panic!("World Channel: cannot remove component from remote entity that doesn't exist");
        };
        components.remove(component_kind);

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
        self.host_world.insert(*global_entity, CheckedSet::new());
        self.remote_world.insert(*global_entity, CheckedSet::new());

        let new_host_entity = on_entity_channel_opening(local_world_manager, global_entity);

        // info!("--- tracking remote entity ---");

        self.entity_command_manager.track_remote_entity(global_entity, &component_kinds);

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
        let components = self.host_world.remove(global_entity).unwrap();
        self.remote_world.remove(global_entity);

        local_world_manager.set_primary_to_remote(global_entity);

        self.entity_command_manager.untrack_remote_entity(global_entity);

        for component in components.iter() {
            self.untrack_remote_component(global_entity, component);
        }
    }

    pub fn track_remote_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        {
            let Some(components) = self.host_world.get_mut(global_entity) else {
                panic!("World Channel: cannot insert component into host entity that doesn't exist");
            };
            components.insert(*component_kind);
        }
        
        {
            let Some(components) = self.remote_world.get_mut(global_entity) else {
                panic!("World Channel: cannot insert component into remote entity that doesn't exist");
            };
            components.insert(*component_kind);
        }

        self.entity_update_manager.register_component(global_entity, component_kind);
    }

    pub fn untrack_remote_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
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
            EntityCommand::SpawnEntity(_) | EntityCommand::DespawnEntity(_) | EntityCommand::InsertComponent(_, _) | EntityCommand::RemoveComponent(_, _) => {}
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
        HostWorldEvents {
            next_send_commands: self.entity_command_manager.take_outgoing_commands(now, rtt_millis),
            next_send_updates: self.entity_update_manager.collect_next_updates(
                world,
                converter,
                global_world_manager,
                &self.host_world,
                &self.remote_world,
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
                EntityMessage::SpawnEntity(entity) => {
                    self.on_remote_spawn_entity(&entity);
                }
                EntityMessage::DespawnEntity(entity) => {
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

fn on_entity_channel_opening(
    local_world_manager: &mut LocalWorldManager,
    global_entity: &GlobalEntity,
) -> HostEntity {
    if let Some(host_entity) = local_world_manager.remove_reserved_host_entity(global_entity) {
        // info!(
        //     "World Channel: entity channel opening with reserved host entity: {:?}",
        //     host_entity
        // );
        return host_entity;
    } else {
        let host_entity = local_world_manager.generate_host_entity();
        local_world_manager.insert_host_entity(*global_entity, host_entity);
        return host_entity;
    }
}

fn on_remote_entity_channel_closed(
    local_world_manager: &mut LocalWorldManager,
    global_entity: &GlobalEntity,
) {
    local_world_manager.remove_by_global_entity(global_entity);
}