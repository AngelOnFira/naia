use std::{
    collections::{hash_set::Iter, HashSet},
    hash::Hash,
    net::SocketAddr,
};

use crate::{server::WorldServer, RoomKey, UserKey};

// User

#[derive(Clone)]
pub struct WorldUser {
    data_addr: Option<SocketAddr>,
    rooms_cache: HashSet<RoomKey>,
}

impl WorldUser {
    pub fn new() -> Self {
        Self {
            rooms_cache: HashSet::new(),
            data_addr: None,
        }
    }

    pub fn has_address(&self) -> bool {
        self.data_addr.is_some()
    }

    pub fn address(&self) -> SocketAddr {
        self.data_addr.unwrap()
    }

    pub fn address_opt(&self) -> Option<SocketAddr> {
        self.data_addr
    }

    pub(crate) fn set_address(&mut self, addr: &SocketAddr) {
        self.data_addr = Some(*addr);
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

// WorldUserRef

pub struct WorldUserRef<'s, E: Copy + Eq + Hash + Send + Sync> {
    server: &'s WorldServer<E>,
    key: UserKey,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync> WorldUserRef<'s, E> {
    pub(crate) fn new(server: &'s WorldServer<E>, key: &UserKey) -> Self {
        Self { server, key: *key }
    }

    pub fn key(&self) -> UserKey {
        self.key
    }

    pub fn room_count(&self) -> usize {
        self.server.user_rooms_count(&self.key).unwrap()
    }

    /// Returns an iterator of all the keys of the [`Room`]s the User belongs to
    pub fn room_keys(&self) -> impl Iterator<Item = &RoomKey> {
        self.server.user_room_keys(&self.key).unwrap()
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

    pub fn disconnect(&mut self) {
        todo!();
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
}
