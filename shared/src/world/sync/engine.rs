//! Stub implementation of the replication `Engine` as outlined in REFACTOR_PLAN.
#![allow(dead_code)]

use std::hash::Hash;

use crate::{world::entity::entity_message::EntityMessage, MessageIndex};

pub struct Engine<E: Copy + Hash + Eq> {
    outgoing_events: Vec<EntityMessage<E>>,
}

impl<E: Copy + Hash + Eq> Engine<E> {
    pub fn new() -> Self {
        Self {
            outgoing_events: Vec::new(),
        }
    }

    /// Feed a de-duplicated, unordered message into the engine.
    pub fn push(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<E>
    ) {
        self.outgoing_events.push(msg);
    }

    /// Drain messages from the engine in appropriate order.
    pub fn drain(&mut self) -> Vec<EntityMessage<E>> {
        std::mem::take(&mut self.outgoing_events)
    }
} 