//! Synchronization engine compile-time configuration.
#![allow(dead_code)]

pub struct EngineConfig {
    /// Maximum number of in-flight packets.
    pub max_in_flight: u16,
    /// Flush threshold for guard-band.
    pub flush_threshold: u16,
}

impl Default for EngineConfig {
    fn default() -> Self {

        /// Upper bound on un-ACKed packets (< 32_768).
        let max_in_flight: u16 = 32_767;

        /// Guard-band threshold where we flush backlog near wrap-around.
        let flush_threshold: u16 = (65_536u32 - max_in_flight as u32) as u16;

        Self {
            max_in_flight,
            flush_threshold,
        }
    }
}