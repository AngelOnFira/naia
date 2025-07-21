//! Core event definitions used by the sync engine.
#![allow(dead_code)]

use std::vec::Vec;

/// Discriminates the semantic meaning of an [`Event`] payload.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MsgKind {
    /// Spawn an entity.
    Spawn,
    /// Despawn an entity.
    Despawn,
    /// Insert a component.
    Insert,
    /// Remove a component.
    Remove,
    /// Catch-all placeholder while stubbing.
    Other(u8),
}

/// One segment of an entity/component path.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PathSeg {
    Entity(u64),      // EntityId – concrete type TBD
    Comp(u32),        // ComponentKind – concrete type TBD
}

/// The atomic unit processed by the [`Engine`].
#[derive(Clone, Debug)]
pub struct Event {
    pub seq: u16,
    pub path: Vec<PathSeg>,
    pub kind: MsgKind,
    pub payload: Vec<u8>, // opaque to engine for now
}

impl Event {
    pub fn new(seq: u16, path: Vec<PathSeg>, kind: MsgKind, payload: Vec<u8>) -> Self {
        Self { seq, path, kind, payload }
    }
} 