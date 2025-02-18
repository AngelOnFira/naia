use std::{collections::hash_set::Iter, hash::Hash, net::SocketAddr};

use naia_shared::BigMapKey;

use crate::{
    server::{MainServer, WorldServer},
    MainUserMut, RoomKey, WorldUserMut,
};

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
    server: &'s WorldServer<E>,
    key: UserKey,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync> UserRef<'s, E> {
    pub(crate) fn new(server: &'s WorldServer<E>, key: &UserKey) -> Self {
        Self { server, key: *key }
    }

    pub fn key(&self) -> UserKey {
        self.key
    }

    pub fn address(&self) -> SocketAddr {
        self.server.user_address(&self.key).unwrap()
    }

    pub fn room_count(&self) -> usize {
        self.server.user_rooms_count(&self.key).unwrap()
    }

    /// Returns an iterator of all the keys of the [`Room`]s the User belongs to
    pub fn room_keys(&self) -> impl Iterator<Item = &RoomKey> {
        self.server.user_room_keys(&self.key).unwrap()
    }
}

// UserMut
pub struct UserMut<'s, E: Copy + Eq + Hash + Send + Sync> {
    main_user_mut_opt: Option<MainUserMut<'s>>,
    world_user_mut: WorldUserMut<'s, E>,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync> UserMut<'s, E> {
    pub(crate) fn new(
        main_opt: Option<&'s mut MainServer>,
        world: &'s mut WorldServer<E>,
        key: &UserKey,
    ) -> Self {
        let main_user_mut_opt = main_opt.map(|server| MainUserMut::new(server, key));
        let world_user_mut = WorldUserMut::new(world, key);

        Self {
            main_user_mut_opt,
            world_user_mut,
        }
    }

    pub fn key(&self) -> UserKey {
        self.world_user_mut.key()
    }

    pub fn address(&self) -> SocketAddr {
        self.world_user_mut.address()
    }

    pub fn disconnect(&mut self) {
        if let Some(main_user_mut) = &mut self.main_user_mut_opt {
            main_user_mut.disconnect();
        } else {
            self.world_user_mut.disconnect();
        }
    }

    // Rooms

    pub fn enter_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.world_user_mut.enter_room(room_key);

        self
    }

    pub fn leave_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.world_user_mut.leave_room(room_key);

        self
    }

    pub fn room_count(&self) -> usize {
        self.world_user_mut.room_count()
    }

    /// Returns an iterator of all the keys of the [`Room`]s the User belongs to
    pub fn room_keys(&self) -> Iter<RoomKey> {
        self.world_user_mut.room_keys()
    }
}
