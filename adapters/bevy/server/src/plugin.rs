use std::{ops::DerefMut, sync::Mutex};

use bevy_app::{App, Last, Plugin as PluginType, Startup, Update};
use bevy_ecs::{entity::Entity, schedule::IntoSystemConfigs};

use naia_bevy_shared::{BeforeReceiveEvents, Protocol, SendPackets, SharedPlugin};
use naia_server::{shared::Protocol as NaiaProtocol, Server, ServerConfig, WorldServer};

use super::{
    events::{
        AuthEvents, ConnectEvent, DespawnEntityEvent, DisconnectEvent, ErrorEvent,
        InsertComponentEvents, MessageEvents, PublishEntityEvent, RemoveComponentEvents,
        RequestEvents, SpawnEntityEvent, TickEvent, UnpublishEntityEvent, UpdateComponentEvents,
    },
    server::ServerImpl,
    systems::{before_receive_events, send_packets, send_packets_init},
};

struct PluginConfig {
    server_config: ServerConfig,
    protocol: Protocol,
}

impl PluginConfig {
    pub fn new(server_config: ServerConfig, protocol: Protocol) -> Self {
        PluginConfig {
            server_config,
            protocol,
        }
    }
}

#[derive(Clone)]
pub struct Singleton;

pub struct Plugin {
    config: Mutex<Option<PluginConfig>>,
    world_only: bool,
}

impl Plugin {
    pub fn new(server_config: ServerConfig, protocol: Protocol) -> Self {
        Self::new_impl(server_config, protocol, false)
    }

    pub fn world_only(server_config: ServerConfig, protocol: Protocol) -> Self {
        Self::new_impl(server_config, protocol, true)
    }

    fn new_impl(server_config: ServerConfig, protocol: Protocol, world_only: bool) -> Self {
        let config = PluginConfig::new(server_config, protocol);
        Self {
            config: Mutex::new(Some(config)),
            world_only,
        }
    }
}

impl PluginType for Plugin {
    fn build(&self, app: &mut App) {
        let mut config = self.config.lock().unwrap().deref_mut().take().unwrap();

        let world_data = config.protocol.take_world_data();
        world_data.add_systems(app);
        app.insert_resource(world_data);

        let server_impl = if !self.world_only {
            let server = Server::<Entity>::new(config.server_config, config.protocol.into());
            ServerImpl::full(server)
        } else {
            let protocol: NaiaProtocol = config.protocol.into();
            let NaiaProtocol {
                channel_kinds,
                message_kinds,
                component_kinds,
                tick_interval,
                compression,
                client_authoritative_entities,
                ..
            } = protocol;
            let server = WorldServer::<Entity>::new(
                config.server_config,
                compression,
                channel_kinds,
                message_kinds.clone(),
                component_kinds,
                client_authoritative_entities,
                tick_interval,
            );
            ServerImpl::world_only(server)
        };

        app
            // SHARED PLUGIN //
            .add_plugins(SharedPlugin::<Singleton>::new())
            // RESOURCES //
            .insert_resource(server_impl)
            // EVENTS //
            .add_event::<ConnectEvent>()
            .add_event::<DisconnectEvent>()
            .add_event::<ErrorEvent>()
            .add_event::<TickEvent>()
            .add_event::<MessageEvents>()
            .add_event::<RequestEvents>()
            .add_event::<AuthEvents>()
            .add_event::<SpawnEntityEvent>()
            .add_event::<DespawnEntityEvent>()
            .add_event::<PublishEntityEvent>()
            .add_event::<UnpublishEntityEvent>()
            .add_event::<InsertComponentEvents>()
            .add_event::<UpdateComponentEvents>()
            .add_event::<RemoveComponentEvents>()
            // SYSTEM SETS //
            .configure_sets(Last, SendPackets)
            // SYSTEMS //
            .add_systems(Update, before_receive_events.in_set(BeforeReceiveEvents))
            .add_systems(Startup, send_packets_init)
            .add_systems(Update, send_packets.in_set(SendPackets));
    }
}
