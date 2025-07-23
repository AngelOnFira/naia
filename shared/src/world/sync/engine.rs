//! Stub implementation of the replication `Engine` as outlined in REFACTOR_PLAN.
#![allow(dead_code)]

use std::collections::HashMap;
use std::hash::Hash;

use crate::{world::entity::entity_message::EntityMessage, ComponentKind, MessageIndex};
use crate::world::sync::config::EngineConfig;
use crate::world::sync::entity_channel::EntityChannel;

pub struct Engine<E: Copy + Hash + Eq> {
    pub config: EngineConfig,
    outgoing_events: Vec<EntityMessage<E>>,
    entity_channels: HashMap<E, EntityChannel>,
}

impl<E: Copy + Hash + Eq> Default for Engine<E> {
    fn default() -> Self {
        Self {
            config: EngineConfig::default(),
            outgoing_events: Vec::new(),
            entity_channels: HashMap::new(),
        }
    }
}

impl<E: Copy + Hash + Eq> Engine<E> {

    /// Feed a de-duplicated, unordered message into the engine.
    pub fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<E>
    ) {
        let Some(entity) = msg.entity() else {
            // was a no-op message
            return;
        };
        // If the entity channel does not exist, create it
        let entity_channel = self.entity_channels
            .entry(entity)
            .or_insert_with(EntityChannel::new);
        
        entity_channel.accept_message(id, msg.strip_entity());
        
        // Drain the entity channel and append the messages to the outgoing events
        let mut received_messages = Vec::new();
        for rmsg in entity_channel.receive_messages() {
            received_messages.push(rmsg.with_entity(entity));
        }
        self.outgoing_events.append(&mut received_messages);
    }

    /// Drain messages from the engine in appropriate order.
    pub fn receive_messages(&mut self) -> Vec<EntityMessage<E>> {
        std::mem::take(&mut self.outgoing_events)
    }
}