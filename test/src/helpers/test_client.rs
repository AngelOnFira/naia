/// Test harness for Client - SIMPLIFIED VERSION
/// 
/// NOTE: Full E2E testing with Server/Client requires World implementations
/// which are game-specific (Bevy, Hecs, etc.). This is a placeholder that
/// documents the intended API. Actual testing happens at the shared level
/// (LocalWorldManager, RemoteEntityChannel, etc.) in regression tests.
pub struct TestClient;

impl TestClient {
    /// Create a new test client (stub - see note above)
    pub fn new() -> Self {
        Self
    }
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}

