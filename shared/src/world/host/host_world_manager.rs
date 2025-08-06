use std::{
    collections::{HashMap, VecDeque},
};

use log::info;

use crate::{world::{
    host::entity_command_manager::EntityCommandManager,
    local_world_manager::LocalWorldManager,
}, ComponentKind, EntityCommand, EntityMessage, GlobalEntity, HostType, Instant, MessageIndex, PacketIndex, PacketNotifiable};
use crate::world::sync::{EntityChannelReceiver, EntityChannelSender};

pub type CommandId = MessageIndex;

/// Manages Entities for a given Client connection and keeps them in
/// sync on the Client
pub struct HostWorldManager {
    entity_command_manager: EntityCommandManager,
}

pub struct HostWorldEvents {
    pub next_send_commands: VecDeque<(CommandId, EntityCommand)>,
}

impl HostWorldEvents {
    pub fn has_events(&self) -> bool {
        !self.next_send_commands.is_empty()
    }
}

impl HostWorldManager {
    /// Create a new HostWorldManager, given the client's address
    pub fn new(host_type: HostType) -> Self {
        Self {
            entity_command_manager: EntityCommandManager::new(host_type),
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

    pub fn remote_despawn_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.entity_command_manager.remote_despawn_entity(global_entity);
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

    pub fn handle_dropped_packets(&mut self, now: &Instant) {
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

    pub fn take_outgoing_events(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
    ) -> HostWorldEvents {
        let next_send_commands = self.entity_command_manager.take_outgoing_commands(now, rtt_millis);
        HostWorldEvents {
            next_send_commands,
        }
    }

    pub fn take_delivered_commands(
        &mut self,
    ) -> Vec<EntityMessage<GlobalEntity>> {
        let delivered_commands = self.entity_command_manager.take_delivered_commands();
        for command in &delivered_commands {
            match command {
                EntityMessage::Despawn(entity) => {
                    self.remote_despawn_entity(entity);
                }
                _ => {}
            }
        }
        delivered_commands
    }

    // Auth

    pub fn entity_release_authority(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.entity_command_manager.send_outgoing_command(EntityCommand::ReleaseAuthority(*global_entity));
    }

    pub fn get_host_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        self.entity_command_manager.get_host_world()
    }

    pub fn get_remote_world(&self) -> &HashMap<GlobalEntity, EntityChannelReceiver> {
        self.entity_command_manager.get_remote_world()
    }
}

impl PacketNotifiable for HostWorldManager {
    fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        self.entity_command_manager.notify_packet_delivered(packet_index);
    }
}