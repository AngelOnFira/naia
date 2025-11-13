use naia_shared::{
    Channel, ChannelDirection, ChannelKind, ChannelKinds, ChannelMode, ChannelSettings,
    FakeEntityConverter, HostType, Message, MessageContainer, MessageContainerError, MessageKind,
    MessageKinds, MessageKindsError, MessageManager, MessageManagerError, Protocol,
    ReliableSettings, TickBufferSettings,
};

// Test message type
#[derive(Channel)]
pub struct TestChannel;

#[derive(Message)]
pub struct TestMessage {
    value: u8,
}

// Helper function to create a simple protocol
fn create_test_protocol() -> Protocol {
    let mut protocol = Protocol::builder();
    protocol.add_channel::<TestChannel>(
        ChannelDirection::Bidirectional,
        ChannelMode::UnorderedReliable(ReliableSettings::default()),
    );
    protocol.add_message::<TestMessage>();
    protocol.build()
}

#[test]
fn test_message_kinds_net_id_not_found() {
    let message_kinds = MessageKinds::new();

    // Try to look up a net_id that doesn't exist
    let net_id = 999u16;
    let result = message_kinds.try_net_id_to_kind(&net_id);

    assert!(result.is_err());
    match result {
        Err(MessageKindsError::NetIdNotFound { net_id: id }) => {
            assert_eq!(id, net_id);
        }
        _ => panic!("Expected NetIdNotFound error"),
    }
}

#[test]
fn test_message_kinds_kind_not_found() {
    let message_kinds = MessageKinds::new();

    // Try to look up a message kind that was never registered
    let unregistered_kind = MessageKind::of::<TestMessage>();
    let result = message_kinds.try_kind_to_net_id(&unregistered_kind);

    assert!(result.is_err());
    match result {
        Err(MessageKindsError::MessageKindNotFound) => {
            // Success
        }
        _ => panic!("Expected MessageKindNotFound error"),
    }
}

#[test]
fn test_message_kinds_builder_not_found() {
    let message_kinds = MessageKinds::new();

    // Try to get builder for unregistered message kind
    let unregistered_kind = MessageKind::of::<TestMessage>();
    let result = message_kinds.try_kind_to_builder(&unregistered_kind);

    assert!(result.is_err());
    match result {
        Err(MessageKindsError::MessageKindNotFound) => {
            // Success
        }
        _ => panic!("Expected MessageKindNotFound error"),
    }
}

#[test]
fn test_message_kinds_registered_messages() {
    let protocol = create_test_protocol();
    let message_kinds = &protocol.message_kinds;

    // Test that registered message kind works
    let kind = MessageKind::of::<TestMessage>();
    let result = message_kinds.try_kind_to_net_id(&kind);

    assert!(result.is_ok());
    let net_id = result.unwrap();

    // Test reverse lookup
    let kind_result = message_kinds.try_net_id_to_kind(&net_id);
    assert!(kind_result.is_ok());
    assert_eq!(kind_result.unwrap(), kind);

    // Test builder lookup
    let builder_result = message_kinds.try_kind_to_builder(&kind);
    assert!(builder_result.is_ok());
}

#[test]
fn test_message_manager_channel_not_found_for_sending() {
    let protocol = create_test_protocol();
    let mut manager = MessageManager::new(HostType::Server, &protocol.channel_kinds);

    // Create a channel kind that doesn't exist in the manager
    #[derive(Channel)]
    pub struct UnregisteredChannel;
    let unregistered_channel = ChannelKind::of::<UnregisteredChannel>();

    let test_message = Box::new(TestMessage { value: 42 });
    let mut converter = FakeEntityConverter;
    let message_container = MessageContainer::from_write(test_message, &mut converter);

    // Try to send message on unregistered channel
    let result = manager.try_send_message(
        &protocol.message_kinds,
        &mut converter,
        &unregistered_channel,
        message_container,
    );

    assert!(result.is_err());
    match result {
        Err(MessageManagerError::ChannelNotConfiguredForSending { .. }) => {
            // Success
        }
        _ => panic!("Expected ChannelNotConfiguredForSending error"),
    }
}

#[test]
fn test_message_manager_send_request_channel_not_found() {
    use naia_shared::GlobalRequestId;

    let protocol = create_test_protocol();
    let mut manager = MessageManager::new(HostType::Server, &protocol.channel_kinds);

    #[derive(Channel)]
    pub struct UnregisteredChannel;
    let unregistered_channel = ChannelKind::of::<UnregisteredChannel>();

    let test_message = Box::new(TestMessage { value: 42 });
    let mut converter = FakeEntityConverter;
    let message_container = MessageContainer::from_write(test_message, &mut converter);

    let global_request_id = GlobalRequestId::new(1);

    // Try to send request on unregistered channel
    let result = manager.try_send_request(
        &protocol.message_kinds,
        &mut converter,
        &unregistered_channel,
        global_request_id,
        message_container,
    );

    assert!(result.is_err());
    match result {
        Err(MessageManagerError::ChannelNotConfiguredForSending { .. }) => {
            // Success
        }
        _ => panic!("Expected ChannelNotConfiguredForSending error"),
    }
}

// Note: LocalResponseId doesn't have a public constructor, so we can't easily test
// try_send_response without accessing internal APIs. The method is tested indirectly
// through the manager's usage in integration tests.

#[test]
fn test_message_container_bit_length_on_read() {
    // Create a message container from a read operation (bit_length should be None)
    let test_message = Box::new(TestMessage { value: 42 });
    let message_container = MessageContainer::from_read(test_message);

    // Try to get bit_length (should fail)
    let result = message_container.try_bit_length();

    assert!(result.is_err());
    match result {
        Err(MessageContainerError::BitLengthNotAvailable) => {
            // Success
        }
        _ => panic!("Expected BitLengthNotAvailable error"),
    }
}

#[test]
fn test_message_container_bit_length_on_write() {
    // Create a message container from a write operation (bit_length should be available)
    let test_message = Box::new(TestMessage { value: 42 });
    let mut converter = FakeEntityConverter;
    let message_container = MessageContainer::from_write(test_message, &mut converter);

    // Try to get bit_length (should succeed)
    let result = message_container.try_bit_length();

    assert!(result.is_ok());
    assert!(result.unwrap() > 0);
}

#[test]
fn test_message_manager_send_on_valid_channel() {
    let protocol = create_test_protocol();
    let mut manager = MessageManager::new(HostType::Server, &protocol.channel_kinds);
    let channel = ChannelKind::of::<TestChannel>();

    let test_message = Box::new(TestMessage { value: 42 });
    let mut converter = FakeEntityConverter;
    let message_container = MessageContainer::from_write(test_message, &mut converter);

    // Should succeed
    let result = manager.try_send_message(
        &protocol.message_kinds,
        &mut converter,
        &channel,
        message_container,
    );

    assert!(result.is_ok());
}

// ============================================================================
// Channel Error Handling Tests
// ============================================================================

#[test]
fn test_channel_settings_try_new_valid_tick_buffered() {
    // TickBuffered is only valid with ClientToServer direction
    let result = ChannelSettings::try_new(
        ChannelMode::TickBuffered(TickBufferSettings::default()),
        ChannelDirection::ClientToServer,
    );

    assert!(result.is_ok());
    let settings = result.unwrap();
    assert!(settings.tick_buffered());
}

#[test]
fn test_channel_settings_try_new_invalid_tick_buffered_server_to_client() {
    // TickBuffered is NOT valid with ServerToClient direction
    let result = ChannelSettings::try_new(
        ChannelMode::TickBuffered(TickBufferSettings::default()),
        ChannelDirection::ServerToClient,
    );

    assert!(result.is_err());
    match result {
        Err(ChannelError::InvalidTickBufferedDirection) => {
            // Success
        }
        _ => panic!("Expected InvalidTickBufferedDirection error"),
    }
}

#[test]
fn test_channel_settings_try_new_invalid_tick_buffered_bidirectional() {
    // TickBuffered is NOT valid with Bidirectional direction
    let result = ChannelSettings::try_new(
        ChannelMode::TickBuffered(TickBufferSettings::default()),
        ChannelDirection::Bidirectional,
    );

    assert!(result.is_err());
    match result {
        Err(ChannelError::InvalidTickBufferedDirection) => {
            // Success
        }
        _ => panic!("Expected InvalidTickBufferedDirection error"),
    }
}

#[test]
fn test_channel_settings_try_new_non_tick_buffered() {
    // Non-TickBuffered modes should work with any direction
    let directions = vec![
        ChannelDirection::ClientToServer,
        ChannelDirection::ServerToClient,
        ChannelDirection::Bidirectional,
    ];

    for direction in directions {
        let result = ChannelSettings::try_new(
            ChannelMode::OrderedReliable(ReliableSettings::default()),
            direction,
        );
        assert!(result.is_ok());
    }
}

#[test]
fn test_channel_kinds_try_channel_not_found() {
    let channel_kinds = ChannelKinds::new();

    // Create a channel kind that doesn't exist
    #[derive(Channel)]
    pub struct UnregisteredChannel;
    let unregistered_kind = ChannelKind::of::<UnregisteredChannel>();

    // Try to get settings for unregistered channel
    let result = channel_kinds.try_channel(&unregistered_kind);

    assert!(result.is_err());
    match result {
        Err(ChannelError::ChannelKindNotFound) => {
            // Success
        }
        _ => panic!("Expected ChannelKindNotFound error"),
    }
}

#[test]
fn test_channel_kinds_try_channel_found() {
    let protocol = create_test_protocol();
    let channel_kinds = &protocol.channel_kinds;

    let test_channel = ChannelKind::of::<TestChannel>();
    let result = channel_kinds.try_channel(&test_channel);

    assert!(result.is_ok());
    let settings = result.unwrap();
    assert!(settings.reliable());
}

#[test]
fn test_channel_kinds_try_net_id_to_kind_not_found() {
    let channel_kinds = ChannelKinds::new();

    // Try to look up a net_id that doesn't exist
    let net_id = 999u16;
    let result = channel_kinds.try_net_id_to_kind(&net_id);

    assert!(result.is_err());
    match result {
        Err(ChannelError::NetIdNotFound { net_id: id }) => {
            assert_eq!(id, net_id);
        }
        _ => panic!("Expected NetIdNotFound error"),
    }
}

#[test]
fn test_channel_kinds_try_net_id_to_kind_found() {
    let protocol = create_test_protocol();
    let channel_kinds = &protocol.channel_kinds;

    let test_channel = ChannelKind::of::<TestChannel>();

    // First get the net_id for the channel
    let net_id_result = channel_kinds.try_kind_to_net_id(&test_channel);
    assert!(net_id_result.is_ok());
    let net_id = net_id_result.unwrap();

    // Now try to look it up by net_id
    let result = channel_kinds.try_net_id_to_kind(&net_id);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), test_channel);
}

#[test]
fn test_channel_kinds_try_kind_to_net_id_not_found() {
    let channel_kinds = ChannelKinds::new();

    // Create a channel kind that doesn't exist
    #[derive(Channel)]
    pub struct UnregisteredChannel;
    let unregistered_kind = ChannelKind::of::<UnregisteredChannel>();

    // Try to get net_id for unregistered channel
    let result = channel_kinds.try_kind_to_net_id(&unregistered_kind);

    assert!(result.is_err());
    match result {
        Err(ChannelError::ChannelKindNotFound) => {
            // Success
        }
        _ => panic!("Expected ChannelKindNotFound error"),
    }
}

#[test]
fn test_channel_kinds_try_kind_to_net_id_found() {
    let protocol = create_test_protocol();
    let channel_kinds = &protocol.channel_kinds;

    let test_channel = ChannelKind::of::<TestChannel>();
    let result = channel_kinds.try_kind_to_net_id(&test_channel);

    assert!(result.is_ok());
    let net_id = result.unwrap();

    // Verify reverse lookup works
    let kind_result = channel_kinds.try_net_id_to_kind(&net_id);
    assert!(kind_result.is_ok());
    assert_eq!(kind_result.unwrap(), test_channel);
}

#[test]
#[should_panic(expected = "TickBuffered Messages are only allowed to be sent from Client to Server")]
fn test_channel_settings_panicking_new_still_panics() {
    // Original panicking method should still panic for backward compatibility
    let _ = ChannelSettings::new(
        ChannelMode::TickBuffered(TickBufferSettings::default()),
        ChannelDirection::ServerToClient,
    );
}

// Note: The panicking methods (net_id_to_kind, kind_to_net_id, kind_to_builder) are private
// and only called internally. They maintain backward compatibility with existing code paths
// that expect panics. The public try_* methods provide the non-panicking alternatives.

#[test]
#[should_panic(expected = "Channel not configured correctly")]
fn test_message_manager_panicking_methods_still_panic() {
    let protocol = create_test_protocol();
    let mut manager = MessageManager::new(HostType::Server, &protocol.channel_kinds);

    #[derive(Channel)]
    pub struct UnregisteredChannel;
    let unregistered_channel = ChannelKind::of::<UnregisteredChannel>();

    let test_message = Box::new(TestMessage { value: 42 });
    let mut converter = FakeEntityConverter;
    let message_container = MessageContainer::from_write(test_message, &mut converter);

    // This should panic
    manager.send_message(
        &protocol.message_kinds,
        &mut converter,
        &unregistered_channel,
        message_container,
    );
}

#[test]
#[should_panic(expected = "bit_length should never be called")]
fn test_message_container_panicking_method_still_panics() {
    let test_message = Box::new(TestMessage { value: 42 });
    let message_container = MessageContainer::from_read(test_message);

    // This should panic
    let _ = message_container.bit_length();
}
