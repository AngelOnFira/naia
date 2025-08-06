use std::hash::Hash;
use std::collections::HashSet;

use crate::{ComponentKind, HostType};
use crate::world::sync::auth_channel_sender::AuthChannelSender;

pub struct EntityChannelSender {
    host_type: HostType,
    component_channels: HashSet<ComponentKind>,
    auth_channel: AuthChannelSender,
}

impl EntityChannelSender {
    pub(crate) fn new(host_type: HostType) -> Self {
        Self {
            host_type,
            component_channels: HashSet::new(),
            auth_channel: AuthChannelSender::new(),
        }
    }
    
    pub(crate) fn component_kinds(&self) -> &HashSet<ComponentKind> {
        &self.component_channels
    }
}