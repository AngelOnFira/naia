//! Synchronization engine (new implementation under construction).
//! Currently contains stubs used by the thin facade in `EntityMessageReceiver`.

#![allow(dead_code)]

pub mod config;
pub mod event;
pub mod engine;

// Re-export the main types so callers can `use naia_shared::world::sync::*`.
pub use config::*;
pub use engine::Engine;
pub use event::{Event, PathSeg, MsgKind};

#[cfg(test)]
pub mod tests; 