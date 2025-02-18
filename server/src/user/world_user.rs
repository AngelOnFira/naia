use std::{
    collections::{hash_set::Iter, HashSet},
    hash::Hash,
    net::SocketAddr,
};

use crate::{server::WorldServer, RoomKey, UserKey, UserMut};

// User

#[derive(Clone)]
pub struct WorldUser {
    data_addr: SocketAddr,
    rooms_cache: HashSet<RoomKey>,
}

impl WorldUser {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            data_addr: address,
            rooms_cache: HashSet::new(),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.data_addr
    }

    // Rooms

    pub(crate) fn cache_room(&mut self, room_key: &RoomKey) {
        self.rooms_cache.insert(*room_key);
    }

    pub(crate) fn uncache_room(&mut self, room_key: &RoomKey) {
        self.rooms_cache.remove(room_key);
    }

    pub(crate) fn room_keys(&self) -> &HashSet<RoomKey> {
        &self.rooms_cache
    }

    pub(crate) fn room_count(&self) -> usize {
        self.rooms_cache.len()
    }
}

// WorldUserMut
pub struct WorldUserMut<'s, E: Copy + Eq + Hash + Send + Sync> {
    server: &'s mut WorldServer<E>,
    key: UserKey,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync> WorldUserMut<'s, E> {
    pub(crate) fn new(server: &'s mut WorldServer<E>, key: &UserKey) -> Self {
        Self { server, key: *key }
    }

    pub fn key(&self) -> UserKey {
        self.key
    }

    pub fn address(&self) -> SocketAddr {
        self.server.user_address(&self.key).unwrap()
    }

    pub fn disconnect(&mut self) {
        self.server.user_queue_disconnect(&self.key);
    }

    // Rooms

    pub fn enter_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.server.room_add_user(room_key, &self.key);

        self
    }

    pub fn leave_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.server.room_remove_user(room_key, &self.key);

        self
    }

    pub fn room_count(&self) -> usize {
        self.server.user_rooms_count(&self.key).unwrap()
    }

    /// Returns an iterator of all the keys of the [`Room`]s the User belongs to
    pub fn room_keys(&self) -> Iter<RoomKey> {
        self.server.user_room_keys(&self.key).unwrap()
    }

    pub fn upgrade(self) -> UserMut<'s, E> {
        UserMut::new(None, self.server, &self.key)

    }
}
