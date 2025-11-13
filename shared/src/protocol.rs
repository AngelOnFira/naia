use std::time::Duration;

use naia_socket_shared::{LinkConditionerConfig, SocketConfig};

use crate::{
    connection::compression_config::CompressionConfig,
    messages::{
        channels::{
            channel::{Channel, ChannelDirection, ChannelMode, ChannelSettings},
            channel_kinds::ChannelKinds,
            default_channels::DefaultChannelsPlugin,
            system_channel::SystemChannel,
        },
        fragment::FragmentedMessage,
        message::Message,
        message_kinds::MessageKinds,
    },
    world::component::{component_kinds::ComponentKinds, replicate::Replicate},
    EntityEventMessage, ReliableSettings, Request, RequestOrResponse,
};

pub mod error;
pub use error::ProtocolError;

// Protocol Plugin
pub trait ProtocolPlugin {
    fn build(&self, protocol: &mut Protocol);
}

// Protocol
pub struct Protocol {
    pub channel_kinds: ChannelKinds,
    pub message_kinds: MessageKinds,
    pub component_kinds: ComponentKinds,
    /// Used to configure the underlying socket
    pub socket: SocketConfig,
    /// The duration between each tick
    pub tick_interval: Duration,
    /// Configuration used to control compression parameters
    pub compression: Option<CompressionConfig>,
    /// Whether or not Client Authoritative Entities will be allowed
    pub client_authoritative_entities: bool,
    locked: bool,
}

impl Default for Protocol {
    fn default() -> Self {
        let mut message_kinds = MessageKinds::new();
        message_kinds.add_message::<FragmentedMessage>();
        message_kinds.add_message::<RequestOrResponse>();
        message_kinds.add_message::<EntityEventMessage>();

        let mut channel_kinds = ChannelKinds::new();
        channel_kinds.add_channel::<SystemChannel>(ChannelSettings::new(
            ChannelMode::OrderedReliable(ReliableSettings::default()),
            ChannelDirection::Bidirectional,
        ));

        Self {
            channel_kinds,
            message_kinds,
            component_kinds: ComponentKinds::new(),
            socket: SocketConfig::new(None, None),
            tick_interval: Duration::from_millis(50),
            compression: None,
            client_authoritative_entities: false,
            locked: false,
        }
    }
}

impl Protocol {
    pub fn builder() -> Self {
        Self::default()
    }

    pub fn add_plugin<P: ProtocolPlugin>(&mut self, plugin: P) -> &mut Self {
        self.check_lock();
        plugin.build(self);
        self
    }

    pub fn link_condition(&mut self, config: LinkConditionerConfig) -> &mut Self {
        self.check_lock();
        self.socket.link_condition = Some(config);
        self
    }

    pub fn rtc_endpoint(&mut self, path: String) -> &mut Self {
        self.check_lock();
        self.socket.rtc_endpoint_path = path;
        self
    }

    pub fn get_rtc_endpoint(&self) -> String {
        self.socket.rtc_endpoint_path.clone()
    }

    pub fn tick_interval(&mut self, duration: Duration) -> &mut Self {
        self.check_lock();
        self.tick_interval = duration;
        self
    }

    pub fn compression(&mut self, config: CompressionConfig) -> &mut Self {
        self.check_lock();
        self.compression = Some(config);
        self
    }

    pub fn enable_client_authoritative_entities(&mut self) -> &mut Self {
        self.check_lock();
        self.client_authoritative_entities = true;
        self
    }

    pub fn add_default_channels(&mut self) -> &mut Self {
        self.check_lock();
        let plugin = DefaultChannelsPlugin;
        plugin.build(self);
        self
    }

    pub fn add_channel<C: Channel>(
        &mut self,
        direction: ChannelDirection,
        mode: ChannelMode,
    ) -> &mut Self {
        self.check_lock();
        self.channel_kinds
            .add_channel::<C>(ChannelSettings::new(mode, direction));
        self
    }

    pub fn add_message<M: Message>(&mut self) -> &mut Self {
        self.check_lock();
        self.message_kinds.add_message::<M>();
        self
    }

    pub fn add_request<Q: Request>(&mut self) -> &mut Self {
        self.check_lock();
        // Requests and Responses are handled just like Messages
        self.message_kinds.add_message::<Q>();
        self.message_kinds.add_message::<Q::Response>();
        self
    }

    pub fn add_component<C: Replicate>(&mut self) -> &mut Self {
        self.check_lock();
        self.component_kinds.add_component::<C>();
        self
    }

    // Non-panicking builder methods

    pub fn try_add_plugin<P: ProtocolPlugin>(&mut self, plugin: P) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        plugin.build(self);
        Ok(self)
    }

    pub fn try_link_condition(&mut self, config: LinkConditionerConfig) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.socket.link_condition = Some(config);
        Ok(self)
    }

    pub fn try_rtc_endpoint(&mut self, path: String) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.socket.rtc_endpoint_path = path;
        Ok(self)
    }

    pub fn try_tick_interval(&mut self, duration: Duration) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.tick_interval = duration;
        Ok(self)
    }

    pub fn try_compression(&mut self, config: CompressionConfig) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.compression = Some(config);
        Ok(self)
    }

    pub fn try_enable_client_authoritative_entities(&mut self) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.client_authoritative_entities = true;
        Ok(self)
    }

    pub fn try_add_default_channels(&mut self) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        let plugin = DefaultChannelsPlugin;
        plugin.build(self);
        Ok(self)
    }

    pub fn try_add_channel<C: Channel>(
        &mut self,
        direction: ChannelDirection,
        mode: ChannelMode,
    ) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.channel_kinds
            .add_channel::<C>(ChannelSettings::new(mode, direction));
        Ok(self)
    }

    pub fn try_add_message<M: Message>(&mut self) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.message_kinds.add_message::<M>();
        Ok(self)
    }

    pub fn try_add_request<Q: Request>(&mut self) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.message_kinds.add_message::<Q>();
        self.message_kinds.add_message::<Q::Response>();
        Ok(self)
    }

    pub fn try_add_component<C: Replicate>(&mut self) -> Result<&mut Self, ProtocolError> {
        self.try_check_lock()?;
        self.component_kinds.add_component::<C>();
        Ok(self)
    }

    pub fn try_lock(&mut self) -> Result<(), ProtocolError> {
        self.try_check_lock()?;
        self.locked = true;
        Ok(())
    }

    pub fn lock(&mut self) {
        self.check_lock();
        self.locked = true;
    }

    /// Checks if protocol is locked without panicking
    /// Returns Err if protocol is locked
    pub fn try_check_lock(&self) -> Result<(), ProtocolError> {
        if self.locked {
            Err(ProtocolError::AlreadyLocked)
        } else {
            Ok(())
        }
    }

    /// Checks if protocol is locked, panics if it is
    /// For backward compatibility with existing code
    pub fn check_lock(&self) {
        if self.locked {
            panic!("Protocol already locked!");
        }
    }

    pub fn build(&mut self) -> Self {
        std::mem::take(self)
    }
}
