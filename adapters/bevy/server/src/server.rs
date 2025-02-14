use std::time::Duration;

use bevy_ecs::{
    entity::Entity,
    system::{ResMut, SystemParam},
    system::Res,
};

use naia_server::{shared::SocketConfig, transport::Socket, NaiaServerError, ReplicationConfig, RoomKey, TickBufferMessages, UserKey};

use naia_bevy_shared::{Channel, EntityAndGlobalEntityConverter, EntityAuthStatus, EntityDoesNotExistError, GlobalEntity, Message, Request, Response, ResponseReceiveKey, ResponseSendKey, Tick};

use crate::{sub_server::SubServer, main_server::MainServer, user_scope::{UserScopeRef, UserScopeMut}, user::{UserMut, UserRef}, room::{RoomRef, RoomMut}, world_entity::{WorldEntity, WorldId}};

// Server

enum ServerRef<'a> {
    Main(&'a MainServer),
    Sub(&'a SubServer),
}

pub(crate) enum ServerMut<'a> {
    Main(&'a mut MainServer),
    Sub(&'a mut SubServer),
}

#[derive(SystemParam)]
pub struct Server<'w> {
    main_server: Option<ResMut<'w, MainServer>>,
    sub_server: Option<ResMut<'w, SubServer>>,
    world_id: Res<'w, WorldId>,
}

impl<'w> Server<'w> {

    // helpers //

    fn get(&self) -> ServerRef {
        match (&self.main_server, &self.sub_server) {
            (Some(main_server), None) => ServerRef::Main(main_server),
            (None, Some(sub_server)) => ServerRef::Sub(sub_server),
            _ => panic!("Server::get: must have either a MainServer or SubServer resource")
        }
    }

    fn get_mut(&mut self) -> ServerMut {
        match (&mut self.main_server, &mut self.sub_server) {
            (Some(main_server), None) => ServerMut::Main(main_server),
            (None, Some(sub_server)) => ServerMut::Sub(sub_server),
            _ => panic!("Server::get_mut: must have either a MainServer or SubServer resource")
        }
    }

    // Public Methods //

    //// Connections ////

    pub fn listen<S: Into<Box<dyn Socket>>>(&mut self, socket: S) {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.listen(socket);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn is_listening(&self) -> bool {
        match self.get() {
            ServerRef::Main(server) => {
                server.is_listening()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn accept_connection(&mut self, user_key: &UserKey) {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.accept_connection(user_key);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn reject_connection(&mut self, user_key: &UserKey) {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.reject_connection(user_key);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    // Config
    pub fn socket_config(&self) -> &SocketConfig {
        match self.get() {
            ServerRef::Main(server) => {
                server.socket_config()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    //// Messages ////

    pub fn send_message<C: Channel, M: Message>(&mut self, user_key: &UserKey, message: &M) {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.send_message::<C, M>(user_key, message);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    /// Sends a message to all connected users using a given channel
    pub fn broadcast_message<C: Channel, M: Message>(&mut self, message: &M) {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.broadcast_message::<C, M>(message);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    /// Requests ///
    pub fn send_request<C: Channel, Q: Request>(
        &mut self,
        user_key: &UserKey,
        request: &Q,
    ) -> Result<ResponseReceiveKey<Q::Response>, NaiaServerError> {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.send_request::<C, Q>(user_key, request)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn send_response<S: Response>(
        &mut self,
        response_key: &ResponseSendKey<S>,
        response: &S,
    ) -> bool {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.send_response(response_key, response)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn receive_response<S: Response>(
        &mut self,
        response_key: &ResponseReceiveKey<S>,
    ) -> Option<(UserKey, S)> {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.receive_response(response_key)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn receive_tick_buffer_messages(&mut self, tick: &Tick) -> TickBufferMessages {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.receive_tick_buffer_messages(tick)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    //// Updates ////

    pub fn scope_checks(&self) -> Vec<(RoomKey, UserKey, Entity)> {
        match self.get() {
            ServerRef::Main(server) => {
                server
                    .scope_checks()
                    .iter()
                    .filter(
                        |(_, _, world_entity)| world_entity.world_id().is_main()
                    )
                    .map(
                        |(room_key, user_key, world_entity)|
                        (*room_key, *user_key, world_entity.entity())
                    )
                    .collect()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    //// Users ////

    pub fn user_exists(&self, user_key: &UserKey) -> bool {
        match self.get() {
            ServerRef::Main(server) => {
                server.user_exists(user_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn user(&self, user_key: &UserKey) -> UserRef {
        match self.get() {
            ServerRef::Main(server) => {
                server.user(user_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn user_mut(&mut self, user_key: &UserKey) -> UserMut {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.user_mut(user_key)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn user_keys(&self) -> Vec<UserKey> {
        match self.get() {
            ServerRef::Main(server) => {
                server.user_keys()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn users_count(&self) -> usize {
        match self.get() {
            ServerRef::Main(server) => {
                server.users_count()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn user_scope(&self, user_key: &UserKey) -> UserScopeRef {
        match self.get() {
            ServerRef::Main(server) => {
                server.user_scope(user_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn user_scope_mut(&mut self, user_key: &UserKey) -> UserScopeMut {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.user_scope_mut(user_key)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    //// Rooms ////

    pub fn make_room(&mut self) -> RoomMut {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.make_room()
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn room_exists(&self, room_key: &RoomKey) -> bool {
        match self.get() {
            ServerRef::Main(server) => {
                server.room_exists(room_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn room(&self, room_key: &RoomKey) -> RoomRef {
        match self.get() {
            ServerRef::Main(server) => {
                server.room(room_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn room_mut(&mut self, room_key: &RoomKey) -> RoomMut {
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.room_mut(room_key)
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn room_keys(&self) -> Vec<RoomKey> {
        match self.get() {
            ServerRef::Main(server) => {
                server.room_keys()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn rooms_count(&self) -> usize {
        match self.get() {
            ServerRef::Main(server) => {
                server.rooms_count()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    //// Ticks ////

    pub fn current_tick(&self) -> Tick {
        match self.get() {
            ServerRef::Main(server) => {
                server.current_tick()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn average_tick_duration(&self) -> Duration {
        match self.get() {
            ServerRef::Main(server) => {
                server.average_tick_duration()
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    //// Network Conditions ////

    pub fn jitter(&self, user_key: &UserKey) -> Option<f32> {
        match self.get() {
            ServerRef::Main(server) => {
                server.jitter(user_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub fn rtt(&self, user_key: &UserKey) -> Option<f32> {
        match self.get() {
            ServerRef::Main(server) => {
                server.rtt(user_key)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    // Crate-Public

    pub(crate) fn world_id(&self) -> WorldId {
        *self.world_id
    }

    // Authority

    pub(crate) fn replication_config(&self, entity: &Entity) -> Option<ReplicationConfig> {
        match self.get() {
            ServerRef::Main(server) => {
                let world_entity = WorldEntity::new(*self.world_id, *entity);
                server.replication_config(&world_entity)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub(crate) fn entity_give_authority(&mut self, user_key: &UserKey, entity: &Entity) {
        let world_id = *self.world_id;
        match self.get_mut() {
            ServerMut::Main(server) => {
                let world_entity = WorldEntity::new(world_id, *entity);
                server.entity_give_authority(user_key, &world_entity);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub(crate) fn entity_take_authority(&mut self, entity: &Entity) {
        let world_id = *self.world_id;
        match self.get_mut() {
            ServerMut::Main(server) => {
                let world_entity = WorldEntity::new(world_id, *entity);
                server.entity_take_authority(&world_entity);
            }
            ServerMut::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub(crate) fn entity_authority_status(&self, entity: &Entity) -> Option<EntityAuthStatus> {
        let world_id = *self.world_id;
        match self.get() {
            ServerRef::Main(server) => {
                let world_entity = WorldEntity::new(world_id, *entity);
                server.entity_authority_status(&world_entity)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    pub(crate) fn enable_replication(&mut self, entity: &Entity) {
        let world_entity = WorldEntity::new(*self.world_id, *entity);
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.enable_replication(&world_entity);
            }
            ServerMut::Sub(server) => {
                server.enable_replication(&world_entity);
            }
        }
    }

    pub(crate) fn disable_replication(&mut self, entity: &Entity) {
        let world_entity = WorldEntity::new(*self.world_id, *entity);
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.disable_replication(&world_entity);
            }
            ServerMut::Sub(server) => {
                server.disable_replication(&world_entity);
            }
        }
    }

    pub(crate) fn pause_replication(&mut self, entity: &Entity) {
        let world_entity = WorldEntity::new(*self.world_id, *entity);
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.pause_replication(&world_entity);
            }
            ServerMut::Sub(server) => {
                server.pause_replication(&world_entity);
            }
        }
    }

    pub(crate) fn resume_replication(&mut self, entity: &Entity) {
        let world_entity = WorldEntity::new(*self.world_id, *entity);
        match self.get_mut() {
            ServerMut::Main(server) => {
                server.resume_replication(&world_entity);
            }
            ServerMut::Sub(server) => {
                server.resume_replication(&world_entity);
            }
        }
    }
}

impl<'w> EntityAndGlobalEntityConverter<Entity> for Server<'w> {
    fn global_entity_to_entity(
        &self,
        global_entity: &GlobalEntity,
    ) -> Result<Entity, EntityDoesNotExistError> {
        match self.get() {
            ServerRef::Main(server) => {
                let world_entity = server.global_entity_to_entity(global_entity)?;
                Ok(world_entity.entity())
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }

    fn entity_to_global_entity(
        &self,
        entity: &Entity,
    ) -> Result<GlobalEntity, EntityDoesNotExistError> {
        match self.get() {
            ServerRef::Main(server) => {
                let world_entity = WorldEntity::main_new(*entity);
                server.entity_to_global_entity(&world_entity)
            }
            ServerRef::Sub(_server) => {
                panic!("SubServers do not support this method");
            }
        }
    }
}


