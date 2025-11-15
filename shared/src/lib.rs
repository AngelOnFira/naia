//! # Naia Shared
//! Common functionality shared between naia-server & naia-client crates.

#![deny(trivial_numeric_casts, unstable_features, unused_import_braces)]

#[macro_use]
extern crate cfg_if;
extern crate core;

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "wbindgen", feature = "mquad"))]
    {
        // Use both protocols...
        compile_error!("wasm target for 'naia_shared' crate requires either the 'wbindgen' OR 'mquad' feature to be enabled, you must pick one.");
    }
    else if #[cfg(all(target_arch = "wasm32", not(feature = "wbindgen"), not(feature = "mquad")))]
    {
        // Use no protocols...
        compile_error!("wasm target for 'naia_shared' crate requires either the 'wbindgen' or 'mquad' feature to be enabled, you must pick one.");
    }
}

pub use naia_derive::{
    Channel, Message, MessageBevy, MessageHecs, Replicate, ReplicateBevy, ReplicateHecs,
};
pub use naia_serde::{
    BitReader, BitWrite, BitWriter, ConstBitLength, FileBitWriter, OutgoingPacket, OwnedBitReader,
    Serde, SerdeBevyClient, SerdeBevyServer, SerdeBevyShared, SerdeErr, SerdeHecs,
    SerdeIntegerConversion, SerdeInternal, SignedInteger, SignedVariableInteger, StreamWriter,
    UnsignedInteger, UnsignedVariableInteger, MTU_SIZE_BITS, MTU_SIZE_BYTES,
};
pub use naia_socket_shared::{
    generate_identity_token, link_condition_logic, IdentityToken, Instant, LinkConditionerConfig,
    Random, SocketConfig, TimeQueue,
};

mod backends;
mod bigmap;
mod connection;
mod constants;
mod game_time;
pub mod handshake;
mod key_generator;
mod messages;
mod protocol;
mod sequence_list;
mod types;
mod world;
mod wrapping_number;

mod transport;

cfg_if! {
    if #[cfg(feature = "transport_udp")]{
        pub mod transport_udp;
    }
}
pub use backends::{Timer, Timestamp};
pub use connection::{
    ack_manager::AckManager,
    bandwidth_monitor::BandwidthMonitor,
    base_connection::BaseConnection,
    compression_config::{CompressionConfig, CompressionMode},
    connection_config::ConnectionConfig,
    decoder::Decoder,
    encoder::Encoder,
    error::{ConnectionError, DecoderError, EncoderError, PacketTypeError},
    packet_notifiable::PacketNotifiable,
    packet_type::PacketType,
    ping_store::{PingIndex, PingStore},
    standard_header::StandardHeader,
};
pub use messages::{
    channels::{
        channel::{
            Channel, ChannelDirection, ChannelMode, ChannelSettings, ReliableSettings,
            TickBufferSettings,
        },
        channel_kinds::{ChannelKind, ChannelKinds},
        default_channels,
        receivers::{
            channel_receiver::ChannelReceiver, error::ReceiverError,
            ordered_reliable_receiver::OrderedReliableReceiver,
            unordered_reliable_receiver::UnorderedReliableReceiver,
        },
        senders::{
            channel_sender::{ChannelSender, MessageChannelSender},
            error::SenderError,
            reliable_sender::ReliableSender,
            request_sender::{LocalRequestId, LocalRequestOrResponseId, LocalResponseId},
        },
        system_channel::SystemChannel,
    },
    error::{
        ChannelError, FragmentationError, MessageContainerError, MessageError, MessageKindsError,
        MessageManagerError,
    },
    message::{Message, Message as MessageBevy, Message as MessageHecs, MessageBuilder},
    message_container::MessageContainer,
    message_kinds::{MessageKind, MessageKinds},
    message_manager::MessageManager,
    named::Named,
    request::{
        GlobalRequestId, GlobalResponseId, Request, Response, ResponseReceiveKey, ResponseSendKey,
    },
};
pub use world::{
    component::{
        component_kinds::{ComponentKind, ComponentKinds},
        component_update::{ComponentFieldUpdate, ComponentUpdate},
        diff_mask::DiffMask,
        entity_property::EntityProperty,
        error::{ComponentError, EntityPropertyError},
        property::{Property, PropertyError},
        property_mutate::{PropertyMutate, PropertyMutator},
        replica_ref::{
            ReplicaDynMut, ReplicaDynMutTrait, ReplicaDynMutWrapper, ReplicaDynRef,
            ReplicaDynRefTrait, ReplicaDynRefWrapper, ReplicaMutTrait, ReplicaMutWrapper,
            ReplicaRefTrait, ReplicaRefWrapper,
        },
        replicate::{
            Replicate, Replicate as ReplicateHecs, Replicate as ReplicateBevy, ReplicateBuilder,
            ReplicatedComponent,
        },
    },
    delegation::{
        auth_channel::EntityAuthAccessor,
        entity_auth_status::{EntityAuthStatus, HostEntityAuthStatus},
        host_auth_handler::HostAuthHandler,
    },
    entity::{
        entity_action::EntityAction,
        entity_action_receiver::EntityActionReceiver,
        entity_action_type::EntityActionType,
        entity_auth_event::{EntityEventMessage, EntityEventMessageAction},
        entity_converters::{
            EntityAndGlobalEntityConverter, EntityAndLocalEntityConverter, EntityConverter,
            EntityConverterMut, FakeEntityConverter, GlobalWorldManagerType,
            LocalEntityAndGlobalEntityConverter, LocalEntityAndGlobalEntityConverterMut,
        },
        error::{EntityAuthError, EntityDoesNotExistError, EntityError},
        global_entity::GlobalEntity,
        local_entity::{HostEntity, OwnedLocalEntity, RemoteEntity},
    },
    host::{
        error::WorldChannelError,
        global_diff_handler::GlobalDiffHandler,
        host_world_manager::{HostWorldEvents, HostWorldManager},
        mut_channel::{MutChannelType, MutReceiver},
        CheckedMap, CheckedSet,
    },
    local_world_manager::LocalWorldManager,
    remote::{
        entity_action_event::EntityActionEvent,
        entity_event::{EntityEvent, EntityResponseEvent},
        entity_waitlist::{EntityWaitlist, WaitlistHandle, WaitlistStore},
        error::RemoteWorldError,
        remote_world_manager::RemoteWorldManager,
    },
    shared_global_world_manager::SharedGlobalWorldManager,
    world_type::{WorldMutType, WorldRefType},
};

pub use bigmap::{BigMap, BigMapKey};
pub use game_time::{GameDuration, GameInstant, GAME_TIME_LIMIT};
pub use key_generator::KeyGenerator;
pub use messages::channels::senders::request_sender::RequestOrResponse;
pub use protocol::{Protocol, ProtocolError, ProtocolPlugin};
pub use sequence_list::{SequenceError, SequenceList};
pub use transport::error::{HttpParseError, TransportError};
pub use types::{HostType, MessageIndex, PacketIndex, ShortMessageIndex, Tick};
pub use wrapping_number::{
    sequence_greater_than, sequence_less_than, try_wrapping_diff, wrapping_diff,
    WrappingNumberError,
};
