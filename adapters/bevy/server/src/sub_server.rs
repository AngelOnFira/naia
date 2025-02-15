
// SubServer

use bevy_ecs::prelude::Resource;

use naia_server::{RoomKey, Server as NaiaServer, UserKey};
use naia_bevy_shared::{Channel, ComponentKind, EntityAndGlobalEntityConverter, EntityDoesNotExistError, GlobalEntity, Message};

use crate::{main_server::MainServer, world_entity::WorldEntity, Replicate, WorldId, room::RoomMut};

#[derive(Resource)]
pub struct SubServer {
    world_id: WorldId,
    server: NaiaServer<WorldEntity>,
}

impl SubServer {

    pub(crate) fn wrap(world_id: WorldId, server: NaiaServer<WorldEntity>) -> Self {
        Self {
            world_id,
            server,
        }
    }

    pub fn set_world_id(&mut self, world_id: WorldId) {
        self.world_id = world_id;
    }

    pub(crate) fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn to_main(self) -> MainServer {
        MainServer::wrap(self.server)
    }

    // Messages

    pub(crate) fn send_message<C: Channel, M: Message>(&mut self, user_key: &UserKey, message: &M) {
        self.server.send_message::<C, M>(user_key, message);
    }

    /// Rooms

    pub(crate) fn make_room(&mut self) -> RoomMut {
        let room_key = {
            let room_mut = self.server.make_room();
            let room_key = room_mut.key();
            room_key
        };

        RoomMut::new(self.world_id(), self.server.room_mut(&room_key))
    }

    pub(crate) fn room_mut(&mut self, room_key: &RoomKey) -> RoomMut {
        RoomMut::new(self.world_id(), self.server.room_mut(room_key))
    }

    // Replication


    pub(crate) fn enable_replication(&mut self, world_entity: &WorldEntity) {
        self.server.enable_entity_replication(world_entity);
    }

    pub(crate) fn disable_replication(&mut self, world_entity: &WorldEntity) {
        self.server.disable_entity_replication(world_entity);
    }

    pub(crate) fn pause_replication(&mut self, world_entity: &WorldEntity) {
        self.server.pause_entity_replication(world_entity);
    }

    pub(crate) fn resume_replication(&mut self, world_entity: &WorldEntity) {
        self.server.resume_entity_replication(world_entity);
    }

    // World

    pub(crate) fn despawn_entity_worldless(&mut self, world_entity: &WorldEntity) {
        self.server.despawn_entity_worldless(world_entity);
    }

    pub(crate) fn insert_component_worldless(&mut self, world_entity: &WorldEntity, component: &mut dyn Replicate) {
        self.server.insert_component_worldless(world_entity, component);
    }

    pub(crate) fn remove_component_worldless(&mut self, world_entity: &WorldEntity, component_kind: &ComponentKind) {
        self.server.remove_component_worldless(world_entity, component_kind);
    }

    // Scopes

    pub(crate) fn scope_checks(&self) -> Vec<(RoomKey, UserKey, WorldEntity)> {
        self.server.scope_checks()
    }
}

impl<'w> EntityAndGlobalEntityConverter<WorldEntity> for SubServer {
    fn global_entity_to_entity(
        &self,
        global_entity: &GlobalEntity,
    ) -> Result<WorldEntity, EntityDoesNotExistError> {
        self.server.global_entity_to_entity(global_entity)
    }

    fn entity_to_global_entity(
        &self,
        world_entity: &WorldEntity,
    ) -> Result<GlobalEntity, EntityDoesNotExistError> {
        self.server.entity_to_global_entity(world_entity)
    }
}