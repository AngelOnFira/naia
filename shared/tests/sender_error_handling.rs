use naia_shared::{LocalRequestId, LocalRequestOrResponseId, LocalResponseId, SenderError};

/// Test that try_to_request_id properly handles response IDs
#[test]
fn test_request_or_response_id_try_to_request() {
    // Create a request ID
    let request_id: LocalRequestId = 42u16.into();
    let req_or_res = request_id.to_req_res_id();

    // Should succeed
    let result = req_or_res.try_to_request_id();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), request_id);

    // Create a response ID
    let response_id: LocalResponseId = request_id.receive_from_remote();
    let req_or_res = response_id.to_req_res_id();

    // Should fail with ExpectedRequest
    let result = req_or_res.try_to_request_id();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SenderError::ExpectedRequest);
}

/// Test that try_to_response_id properly handles request IDs
#[test]
fn test_request_or_response_id_try_to_response() {
    // Create a response ID
    let request_id: LocalRequestId = 42u16.into();
    let response_id: LocalResponseId = request_id.receive_from_remote();
    let req_or_res = response_id.to_req_res_id();

    // Should succeed
    let result = req_or_res.try_to_response_id();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), response_id);

    // Create a request ID
    let req_or_res = request_id.to_req_res_id();

    // Should fail with ExpectedResponse
    let result = req_or_res.try_to_response_id();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SenderError::ExpectedResponse);
}

/// Test that backward compatibility is maintained
#[test]
fn test_backward_compatibility_request_or_response() {
    // Original panic methods should still work for valid cases
    let request_id: LocalRequestId = 42u16.into();
    let req_or_res = request_id.to_req_res_id();

    // This should work without panicking
    let extracted = req_or_res.to_request_id();
    assert_eq!(extracted, request_id);

    // Same for response
    let response_id: LocalResponseId = request_id.receive_from_remote();
    let req_or_res = response_id.to_req_res_id();

    let extracted = req_or_res.to_response_id();
    assert_eq!(extracted, response_id);
}

/// Test SenderError variants for proper error messages
#[test]
fn test_sender_error_messages() {
    // Test EmptyMessageQueue
    let error = SenderError::EmptyMessageQueue;
    let message = format!("{}", error);
    assert!(message.contains("Message queue is empty"));

    // Test NegativeIndexDiff
    let error = SenderError::NegativeIndexDiff {
        previous: 100,
        current: 50,
        diff: -50,
    };
    let message = format!("{}", error);
    assert!(message.contains("negative"));
    assert!(message.contains("100"));
    assert!(message.contains("50"));
    assert!(message.contains("-50"));

    // Test MessageTooLarge
    let error = SenderError::MessageTooLarge {
        bits_needed: 10000,
        bits_free: 5000,
    };
    let message = format!("{}", error);
    assert!(message.contains("overflow"));
    assert!(message.contains("10000"));
    assert!(message.contains("5000"));

    // Test UnreliableMessageTooLarge
    let error = SenderError::UnreliableMessageTooLarge {
        message_name: "TestMessage".to_string(),
        bits_needed: 10000,
        bits_free: 5000,
    };
    let message = format!("{}", error);
    assert!(message.contains("TestMessage"));
    assert!(message.contains("10000"));
    assert!(message.contains("5000"));
    assert!(message.contains("Reliable channel"));

    // Test InvalidCountBitsUsage
    let error = SenderError::InvalidCountBitsUsage;
    let message = format!("{}", error);
    assert!(message.contains("BitCounter"));

    // Test ExpectedRequest
    let error = SenderError::ExpectedRequest;
    let message = format!("{}", error);
    assert!(message.contains("request"));

    // Test ExpectedResponse
    let error = SenderError::ExpectedResponse;
    let message = format!("{}", error);
    assert!(message.contains("response"));

    // Test RequestsNotSupported
    let error = SenderError::RequestsNotSupported {
        channel_type: "UnorderedUnreliable",
    };
    let message = format!("{}", error);
    assert!(message.contains("UnorderedUnreliable"));
    assert!(message.contains("not support"));

    // Test StateInconsistency
    let error = SenderError::StateInconsistency {
        reason: "test inconsistency",
    };
    let message = format!("{}", error);
    assert!(message.contains("test inconsistency"));
}

/// Test SenderError equality
#[test]
fn test_sender_error_equality() {
    let error1 = SenderError::ExpectedRequest;
    let error2 = SenderError::ExpectedRequest;
    assert_eq!(error1, error2);

    let error1 = SenderError::NegativeIndexDiff {
        previous: 100,
        current: 50,
        diff: -50,
    };
    let error2 = SenderError::NegativeIndexDiff {
        previous: 100,
        current: 50,
        diff: -50,
    };
    assert_eq!(error1, error2);

    let error1 = SenderError::MessageTooLarge {
        bits_needed: 10000,
        bits_free: 5000,
    };
    let error2 = SenderError::MessageTooLarge {
        bits_needed: 10000,
        bits_free: 5000,
    };
    assert_eq!(error1, error2);
}

/// Test SenderError cloning
#[test]
fn test_sender_error_clone() {
    let error = SenderError::MessageTooLarge {
        bits_needed: 10000,
        bits_free: 5000,
    };
    let cloned = error.clone();
    assert_eq!(error, cloned);

    let error = SenderError::StateInconsistency {
        reason: "test reason",
    };
    let cloned = error.clone();
    assert_eq!(error, cloned);
}

/// Test that SenderError is Send and Sync
#[test]
fn test_sender_error_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<SenderError>();
    assert_sync::<SenderError>();
}

/// Test error conversion and debugging
#[test]
fn test_sender_error_debug() {
    let error = SenderError::EmptyMessageQueue;
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("EmptyMessageQueue"));

    let error = SenderError::NegativeIndexDiff {
        previous: 100,
        current: 50,
        diff: -50,
    };
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("NegativeIndexDiff"));
}
