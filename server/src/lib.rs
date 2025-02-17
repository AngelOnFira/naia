//! # Naia Server
//! A server that uses either UDP or WebRTC communication to send/receive
//! messages to/from connected clients, and syncs registered
//! Entities/Components to clients to whom they are in-scope.

#![deny(
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces
)]

#[macro_use]
extern crate cfg_if;

pub mod transport;
pub mod shared {
    pub use naia_shared::{
        default_channels, BigMap, BigMapKey, BitReader, BitWrite, BitWriter, ConstBitLength,
        FileBitWriter, GlobalResponseId, Random, ResponseReceiveKey, Serde, SerdeErr,
        SignedInteger, SignedVariableInteger, SocketConfig, UnsignedInteger,
        UnsignedVariableInteger,
    };
}

pub use naia_shared::SerdeBevyServer as SerdeBevy;

mod connection;
mod error;
mod events;
mod main_events;
mod world_events;
mod handshake;
mod request;
mod room;
mod server;
mod time_manager;
mod user_scope;
mod world;
mod user;
mod main_user;
mod world_user;

pub use connection::tick_buffer_messages::TickBufferMessages;
pub use error::NaiaServerError;
pub use events::{Events, Event};
pub use main_events::{
    AuthEvent, ConnectEvent, DisconnectEvent, ErrorEvent,
};
pub use world_events::{
    DelegateEntityEvent, DespawnEntityEvent,
    EntityAuthGrantEvent, EntityAuthResetEvent, InsertComponentEvent,
    MessageEvent, PublishEntityEvent, RemoveComponentEvent, RequestEvent, SpawnEntityEvent,
    UnpublishEntityEvent, UpdateComponentEvent, TickEvent,
};
pub use room::{RoomKey, RoomMut, RoomRef};
pub use server::{Server, ServerConfig};
pub use user::{UserKey, UserMut, UserRef};
pub use world_user::{WorldUser, WorldUserRef, WorldUserMut};
pub use main_user::{MainUser, MainUserRef, MainUserMut};
pub use user_scope::{UserScopeMut, UserScopeRef};
pub use world::{
    entity_mut::EntityMut, entity_owner::EntityOwner, replication_config::ReplicationConfig,
};
