use std::{
    collections::hash_set::Iter,
    hash::Hash,
    net::SocketAddr,
};

use naia_shared::BigMapKey;

use crate::{server::{MainServer, WorldServer}, MainUserMut, MainUserRef, RoomKey, WorldUserMut, WorldUserRef};

// UserKey
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct UserKey(u64);

impl BigMapKey for UserKey {
    fn to_u64(&self) -> u64 {
        self.0
    }

    fn from_u64(value: u64) -> Self {
        UserKey(value)
    }
}

// UserRef

pub struct UserRef<'s, E: Copy + Eq + Hash + Send + Sync> {
    main_user_ref: MainUserRef<'s>,
    world_user_ref: WorldUserRef<'s, E>,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync> UserRef<'s, E> {
    pub(crate) fn new(main: &'s MainServer, world: &'s WorldServer<E>, key: &UserKey) -> Self {

        let main_user_ref = MainUserRef::new(main, key);
        let world_user_ref = WorldUserRef::new(world, key);

        Self {
            main_user_ref,
            world_user_ref,
        }
    }

    pub fn key(&self) -> UserKey {
        self.main_user_ref.key()
    }

    pub fn address(&self) -> SocketAddr {
        self.main_user_ref.address()
    }

    pub fn room_count(&self) -> usize {
        self.world_user_ref.room_count()
    }

    /// Returns an iterator of all the keys of the [`Room`]s the User belongs to
    pub fn room_keys(&self) -> impl Iterator<Item = &RoomKey> {
        self.world_user_ref.room_keys()
    }
}

// UserMut
pub struct UserMut<'s, E: Copy + Eq + Hash + Send + Sync> {
    main_user_ref: MainUserMut<'s>,
    world_user_ref: WorldUserMut<'s, E>,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync> UserMut<'s, E> {
    pub(crate) fn new(main: &'s mut MainServer, world: &'s mut WorldServer<E>, key: &UserKey) -> Self {

        let main_user_mut = MainUserMut::new(main, key);
        let world_user_mut = WorldUserMut::new(world, key);

        Self {
            main_user_ref: main_user_mut,
            world_user_ref: world_user_mut,
        }
    }

    pub fn key(&self) -> UserKey {
        self.main_user_ref.key()
    }

    pub fn address(&self) -> SocketAddr {
        self.main_user_ref.address()
    }

    pub fn disconnect(&mut self) {
        self.main_user_ref.disconnect();
    }

    // Rooms

    pub fn enter_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.world_user_ref.enter_room(room_key);

        self
    }

    pub fn leave_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.world_user_ref.leave_room(room_key);

        self
    }

    pub fn room_count(&self) -> usize {
        self.world_user_ref.room_count()
    }

    /// Returns an iterator of all the keys of the [`Room`]s the User belongs to
    pub fn room_keys(&self) -> Iter<RoomKey> {
        self.world_user_ref.room_keys()
    }
}
