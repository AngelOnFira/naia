use naia_shared::Protocol;

/// Builder for creating test protocols with custom components
pub struct TestProtocol;

impl TestProtocol {
    /// Create a minimal protocol for testing
    pub fn minimal() -> Protocol {
        Protocol::builder()
            .enable_client_authoritative_entities()
            .build()
    }
    
    /// Create a protocol with client authoritative entities enabled (for delegation tests)
    pub fn with_client_auth() -> Protocol {
        Protocol::builder()
            .enable_client_authoritative_entities()
            .build()
    }
}

