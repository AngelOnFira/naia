//! Synchronization engine compile-time configuration.
#![allow(dead_code)]

/// Upper bound on un-ACKed packets (< 32_768).
pub const MAX_IN_FLIGHT: u16 = 32_767;
/// Guard-band threshold where we flush backlog near wrap-around.
pub const FLUSH_THRESHOLD: u16 = (65_536u32 - MAX_IN_FLIGHT as u32) as u16; // 32 769