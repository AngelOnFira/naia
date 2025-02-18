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
mod handshake;
mod request;
mod room;
mod server;
mod time_manager;
mod user_scope;
mod world;
mod user;

pub use connection::tick_buffer_messages::TickBufferMessages;
pub use error::NaiaServerError;
pub use events::{Events, Event,
    AuthEvent, ConnectEvent, DisconnectEvent, ErrorEvent,
    DelegateEntityEvent, DespawnEntityEvent,
    EntityAuthGrantEvent, EntityAuthResetEvent, InsertComponentEvent,
    MessageEvent, PublishEntityEvent, RemoveComponentEvent, RequestEvent, SpawnEntityEvent,
    UnpublishEntityEvent, UpdateComponentEvent, TickEvent,
};
pub use room::{RoomKey, RoomMut, RoomRef};
pub use server::{Server, ServerConfig, WorldServer, MainServer};
pub use user::{UserKey, MainUser, MainUserRef, MainUserMut, UserMut, UserRef, WorldUser, WorldUserRef, WorldUserMut};
pub use user_scope::{UserScopeMut, UserScopeRef};
pub use world::{
    entity_mut::EntityMut, entity_owner::EntityOwner, replication_config::ReplicationConfig,
};
