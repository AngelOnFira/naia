//! Stub implementation of the replication `Engine` as outlined in REFACTOR_PLAN.
#![allow(dead_code)]

use std::marker::PhantomData;

use super::event::Event;

/// Minimal placeholder for `Context` so callers can drain pending commands.
/// For now it just stores a `Vec<Evt>` and returns an empty vec by default.
pub struct Context<Evt> {
    pending: Vec<Evt>,
}

impl<Evt> Context<Evt> {
    pub fn new() -> Self {
        Self { pending: Vec::new() }
    }

    pub fn drain(&mut self) -> Vec<Evt> {
        std::mem::take(&mut self.pending)
    }

    #[allow(dead_code)]
    pub fn push(&mut self, evt: Evt) {
        self.pending.push(evt);
    }
}

/// Generic sync engine – heavily simplified stub.
pub struct Engine<Tmpl> {
    _phantom: PhantomData<Tmpl>,
    // In a real implementation: HashMap<PathKey, Stream>
}

impl<Tmpl> Engine<Tmpl> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }

    /// Feed an event into the engine – currently a no-op.
    pub fn push(&mut self, _event: Event) {
        // TODO: buffering & state machine
    }

    /// Access a mutable context to drain emitted commands.
    pub fn context(&mut self) -> Context<Event> {
        Context::new()
    }
} 