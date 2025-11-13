use naia_shared::{
    Channel, ChannelDirection, ChannelMode, Message, Protocol, ProtocolError, ReliableSettings,
    Replicate,
};

// Test message type
#[derive(Channel)]
pub struct TestChannel;

#[derive(Message)]
pub struct TestMessage {
    value: u8,
}

use naia_shared::Property;

#[derive(Replicate)]
pub struct TestComponent {
    pub value: Property<u8>,
}

// Helper function to create a locked protocol
fn create_locked_protocol() -> Protocol {
    let mut protocol = Protocol::builder();
    protocol.lock();
    protocol
}

#[test]
fn test_try_add_plugin_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    struct TestPlugin;
    impl naia_shared::ProtocolPlugin for TestPlugin {
        fn build(&self, _protocol: &mut Protocol) {}
    }

    let result = protocol.try_add_plugin(TestPlugin);

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_link_condition_on_locked_protocol() {
    use naia_shared::LinkConditionerConfig;

    let mut protocol = create_locked_protocol();
    let config = LinkConditionerConfig::average_condition();

    let result = protocol.try_link_condition(config);

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_rtc_endpoint_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_rtc_endpoint("/rtc".to_string());

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_tick_interval_on_locked_protocol() {
    use std::time::Duration;

    let mut protocol = create_locked_protocol();

    let result = protocol.try_tick_interval(Duration::from_millis(100));

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
#[cfg(feature = "zstd")]
fn test_try_compression_on_locked_protocol() {
    use naia_shared::{CompressionConfig, CompressionMode};

    let mut protocol = create_locked_protocol();
    let config = CompressionConfig::new(CompressionMode::Zstd { level: 3 }, None);

    let result = protocol.try_compression(config);

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_enable_client_authoritative_entities_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_enable_client_authoritative_entities();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_add_default_channels_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_add_default_channels();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_add_channel_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_add_channel::<TestChannel>(
        ChannelDirection::Bidirectional,
        ChannelMode::OrderedReliable(ReliableSettings::default()),
    );

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_add_message_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_add_message::<TestMessage>();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[derive(Message)]
pub struct TestRequest {
    value: u8,
}

#[derive(Message)]
pub struct TestResponse {
    result: u8,
}

impl naia_shared::Request for TestRequest {
    type Response = TestResponse;
}

impl naia_shared::Response for TestResponse {}

#[test]
fn test_try_add_request_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_add_request::<TestRequest>();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_add_component_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_add_component::<TestComponent>();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_lock_on_locked_protocol() {
    let mut protocol = create_locked_protocol();

    let result = protocol.try_lock();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_check_lock_on_locked_protocol() {
    let protocol = create_locked_protocol();

    let result = protocol.try_check_lock();

    assert!(result.is_err());
    match result {
        Err(ProtocolError::AlreadyLocked) => {
            // Success
        }
        _ => panic!("Expected AlreadyLocked error"),
    }
}

#[test]
fn test_try_check_lock_on_unlocked_protocol() {
    let protocol = Protocol::builder();

    let result = protocol.try_check_lock();

    assert!(result.is_ok());
}

#[test]
fn test_try_methods_on_unlocked_protocol() {
    let mut protocol = Protocol::builder();

    // All these should succeed
    assert!(protocol.try_add_message::<TestMessage>().is_ok());
    assert!(protocol.try_add_channel::<TestChannel>(
        ChannelDirection::Bidirectional,
        ChannelMode::OrderedReliable(ReliableSettings::default()),
    ).is_ok());
    assert!(protocol.try_add_component::<TestComponent>().is_ok());

    // Lock should succeed once
    assert!(protocol.try_lock().is_ok());
}

#[test]
fn test_builder_pattern_with_try_methods() {
    use std::time::Duration;

    let mut protocol = Protocol::builder();

    // Test chaining with try_methods using ? operator simulation
    let result = protocol
        .try_tick_interval(Duration::from_millis(100))
        .and_then(|p| p.try_add_message::<TestMessage>())
        .and_then(|p| p.try_add_channel::<TestChannel>(
            ChannelDirection::Bidirectional,
            ChannelMode::OrderedReliable(ReliableSettings::default()),
        ))
        .and_then(|p| p.try_add_component::<TestComponent>());

    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "Protocol already locked!")]
fn test_panicking_check_lock_still_panics() {
    let protocol = create_locked_protocol();

    // This should panic
    protocol.check_lock();
}

#[test]
#[should_panic(expected = "Protocol already locked!")]
fn test_panicking_lock_still_panics() {
    let mut protocol = create_locked_protocol();

    // This should panic
    protocol.lock();
}

#[test]
#[should_panic(expected = "Protocol already locked!")]
fn test_panicking_builder_methods_still_panic() {
    let mut protocol = create_locked_protocol();

    // This should panic
    protocol.add_message::<TestMessage>();
}
