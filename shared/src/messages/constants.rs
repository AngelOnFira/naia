// Message size thresholds

/// Messages larger than this threshold (in bytes) should be sent via QUIC streams
/// instead of datagrams to avoid fragmentation issues.
///
/// Rationale:
/// - Datagrams: Fast, low latency, parallel delivery, but limited to ~400 bytes before fragmentation
/// - Streams: Reliable, unlimited size, but serial delivery (head-of-line blocking per stream)
///
/// Threshold of 32KB means:
/// - Small messages (<32KB): Use datagrams with fragmentation (~80 fragments max)
/// - Large messages (≥32KB): Use streams (no fragmentation, OS handles reliability)
pub const STREAM_THRESHOLD_BYTES: usize = 32_000; // 32 KB

/// Maximum message size for datagram-based channels (with fragmentation)
/// Messages exceeding this on unreliable channels will be rejected
pub const FRAGMENTATION_LIMIT_BYTES: usize = 400;
pub const FRAGMENTATION_LIMIT_BITS: u32 = (FRAGMENTATION_LIMIT_BYTES as u32) * 8;
