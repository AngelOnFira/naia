// Simple entity type for testing - just a u64 wrapper
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TestEntity(u64);

impl TestEntity {
    pub fn new(id: u64) -> Self {
        TestEntity(id)
    }
    
    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Test harness for Server - SIMPLIFIED VERSION
/// 
/// NOTE: Full E2E testing with Server/Client requires World implementations
/// which are game-specific (Bevy, Hecs, etc.). This is a placeholder that
/// documents the intended API. Actual testing happens at the shared level
/// (LocalWorldManager, RemoteEntityChannel, etc.) in regression tests.
pub struct TestServer;

impl TestServer {
    /// Create a new test server (stub - see note above)
    pub fn new() -> Self {
        Self
    }
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

