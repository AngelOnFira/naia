//! Stub implementation of the replication `Engine` as outlined in REFACTOR_PLAN.
#![allow(dead_code)]

use std::hash::Hash;

use crate::{world::entity::entity_message::EntityMessage, MessageIndex};
use crate::world::sync::config::EngineConfig;

pub struct Engine<E: Copy + Hash + Eq> {
    pub config: EngineConfig,
    outgoing_events: Vec<EntityMessage<E>>,
}

impl<E: Copy + Hash + Eq> Default for Engine<E> {
    fn default() -> Self {
        Self {
            config: EngineConfig::default(),
            outgoing_events: Vec::new(),
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
        self.outgoing_events.push(msg);
    }

    /// Drain messages from the engine in appropriate order.
    pub fn receive_messages(&mut self) -> Vec<EntityMessage<E>> {
        std::mem::take(&mut self.outgoing_events)
    }
} 