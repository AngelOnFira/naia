use std::{fmt::Debug, hash::Hash};
use std::collections::{HashMap, HashSet, VecDeque};

use naia_socket_shared::Instant;

use crate::{ChannelSender, ComponentKind, EntityCommand, GlobalEntity, HostEntity, HostType, LocalWorldManager, ReliableSender};
use crate::world::host::host_world_manager::CommandId;
use crate::world::sync::{EntityChannelSender, SenderEngine};

pub struct EntityMessageSender {
    sender: ReliableSender<EntityCommand>,
    engine: SenderEngine,
}

impl EntityMessageSender {
    pub fn new(host_type: HostType, resend_command_rtt_factor: f32) -> Self {
        Self {
            sender: ReliableSender::new(resend_command_rtt_factor),
            engine: SenderEngine::new(host_type),
        }
    }

    pub fn take_outgoing_commands(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
    ) -> VecDeque<(CommandId, EntityCommand)> {
        self.sender.collect_messages(now, rtt_millis);
        self.sender.take_next_messages()
    }

    pub fn send_outgoing_command(
        &mut self,
        command: EntityCommand,
    ) {
        self.sender.send_message(command);
    }

    pub fn deliver_message(
        &mut self,
        command_id: &CommandId,
    ) -> Option<EntityCommand> {
        self.sender.deliver_message(command_id)
    }

    pub fn host_spawn_entity(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        global_entity: &GlobalEntity,
    ) {
        todo!("open entity channel");

        self.sender.send_message(EntityCommand::Spawn(*global_entity));

        on_entity_channel_opening(local_world_manager, global_entity);
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        todo!("close entity channel");

        self.sender.send_message(EntityCommand::Despawn(*global_entity));
    }

    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        todo!("open component channel");

        self.sender.send_message(EntityCommand::InsertComponent(*global_entity, *component_kind));
    }

    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        todo!("close component channel");

        self.sender.send_message(EntityCommand::RemoveComponent(*global_entity, *component_kind));
    }

    pub(crate) fn remote_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        todo!("close entity channel");
    }

    pub(crate) fn track_remote_entity(&self, local_world_manager: &mut LocalWorldManager, global_entity: &GlobalEntity) -> HostEntity {
        on_entity_channel_opening(local_world_manager, global_entity)
    }

    pub(crate) fn untrack_remote_entity(&mut self, local_world_manager: &mut LocalWorldManager, global_entity: &GlobalEntity) -> &HashSet<ComponentKind> {
        local_world_manager.set_primary_to_remote(global_entity);
        todo!("close entity channel, return component kinds")
    }

    pub(crate) fn track_remote_component(&mut self, global_entity: &GlobalEntity, component_kind: &ComponentKind) {
        todo!("open component channel???");
    }

    pub(crate) fn untrack_remote_component(&mut self, global_entity: &GlobalEntity, component_kind: &ComponentKind) {
        todo!("close component channel???");
    }

    pub(crate) fn get_host_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        self.engine.get_host_world()
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