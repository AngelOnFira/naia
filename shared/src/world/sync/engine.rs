//! Stub implementation of the replication `Engine` as outlined in REFACTOR_PLAN.
#![allow(dead_code)]

use std::marker::PhantomData;

use crate::world::entity::entity_message::EntityMessage;
use crate::world::entity::local_entity::RemoteEntity;

/// Generic sync engine – heavily simplified stub.
pub struct Engine<Tmpl> {
    outgoing_events: Vec<EntityMessage<RemoteEntity>>,
    _phantom: PhantomData<Tmpl>,
    // In a real implementation: HashMap<Path, Stream>
}

impl<Tmpl> Engine<Tmpl> {
    pub fn new() -> Self {
        Self { outgoing_events: Vec::new(), _phantom: PhantomData }
    }

    /// Feed a message into the engine.
    pub fn push(&mut self, msg: EntityMessage<RemoteEntity>) {
        // For now, immediately queue the message for output.
        self.outgoing_events.push(msg);
    }

    /// Drain emitted events in FIFO order.
    pub fn drain(&mut self) -> Vec<EntityMessage<RemoteEntity>> {
        std::mem::take(&mut self.outgoing_events)
    }
} 