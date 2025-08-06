use std::collections::HashMap;

use crate::{world::sync::{EntityChannelSender, config::EngineConfig}, GlobalEntity, HostType};

pub struct SenderEngine {
    host_type: HostType,
    pub config: EngineConfig,
    entity_channels: HashMap<GlobalEntity, EntityChannelSender>,
}

impl SenderEngine {

    pub(crate) fn new(host_type: HostType) -> Self {
        Self {
            host_type,
            config: EngineConfig::default(),
            entity_channels: HashMap::new(),
        }
    }

    pub(crate) fn get_host_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        &self.entity_channels
    }
}