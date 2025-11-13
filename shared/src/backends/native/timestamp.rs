use std::time::SystemTime;

/// Error type for timestamp operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    /// System time is before UNIX epoch
    SystemTimeBeforeEpoch,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeError::SystemTimeBeforeEpoch => {
                write!(f, "System time is before UNIX epoch")
            }
        }
    }
}

impl std::error::Error for TimeError {}

pub struct Timestamp;

impl Timestamp {
    /// Returns the current timestamp in seconds since UNIX epoch.
    ///
    /// # Errors
    /// Returns `TimeError::SystemTimeBeforeEpoch` if system time is before UNIX epoch.
    pub fn try_now() -> Result<u64, TimeError> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|_| TimeError::SystemTimeBeforeEpoch)
    }

    /// Returns the current timestamp in seconds since UNIX epoch.
    ///
    /// # Panics
    /// Panics if system time is before UNIX epoch.
    /// For non-panicking version, use `try_now`.
    #[deprecated(since = "0.24.2", note = "Use try_now for safe error handling")]
    pub fn now() -> u64 {
        Self::try_now()
            .expect("Timestamp::now: system time is before UNIX epoch")
    }
}
