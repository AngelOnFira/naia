// These tests are only compiled when the transport_udp feature is enabled
#![cfg(feature = "transport_udp")]

use naia_shared::{
    transport_udp::{try_bytes_to_request, try_bytes_to_response},
    HttpParseError, TransportError,
};

// ============================================================================
// HTTP Request Parsing Tests
// ============================================================================

#[test]
fn test_try_bytes_to_request_valid() {
    let request_bytes = b"GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";

    let result = try_bytes_to_request(request_bytes);
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.method(), "GET");
    assert_eq!(request.uri().path(), "/path");
}

#[test]
fn test_try_bytes_to_request_with_body() {
    let request_bytes = b"POST /api HTTP/1.1\r\nHost: example.com\r\nContent-Length: 13\r\n\r\nHello, World!";

    let result = try_bytes_to_request(request_bytes);
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.method(), "POST");
    assert_eq!(request.uri().path(), "/api");
    assert_eq!(request.body(), b"Hello, World!");
}

#[test]
fn test_try_bytes_to_request_empty() {
    let request_bytes = b"";

    let result = try_bytes_to_request(request_bytes);
    assert!(result.is_err());

    match result {
        Err(TransportError::HttpParse(HttpParseError::MissingComponent { message_type, .. })) => {
            assert_eq!(message_type, "request");
        }
        _ => panic!("Expected MissingComponent error"),
    }
}

#[test]
fn test_try_bytes_to_request_missing_path() {
    let request_bytes = b"GET\r\n\r\n";

    let result = try_bytes_to_request(request_bytes);
    assert!(result.is_err());
}

// Note: The http crate's Method::from_str is very permissive and accepts
// extension methods per RFC 7231. It's difficult to create a truly invalid method
// that will fail parsing, so we test the error path with a unit test on the error type itself.
// The InvalidMethod error is still reachable in theory with a malformed method string.

#[test]
fn test_try_bytes_to_request_no_headers() {
    let request_bytes = b"GET /path HTTP/1.1\r\n\r\n";

    let result = try_bytes_to_request(request_bytes);
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.method(), "GET");
}

// ============================================================================
// HTTP Response Parsing Tests
// ============================================================================

#[test]
fn test_try_bytes_to_response_valid() {
    let response_bytes = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[test]
fn test_try_bytes_to_response_with_body() {
    let response_bytes = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.body(), b"Hello, World!");
}

#[test]
fn test_try_bytes_to_response_404() {
    let response_bytes = b"HTTP/1.1 404 Not Found\r\n\r\n";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
fn test_try_bytes_to_response_empty() {
    let response_bytes = b"";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_err());

    match result {
        Err(TransportError::HttpParse(HttpParseError::MissingComponent { message_type, .. })) => {
            assert_eq!(message_type, "response");
        }
        _ => panic!("Expected MissingComponent error"),
    }
}

#[test]
fn test_try_bytes_to_response_invalid_status_code() {
    let response_bytes = b"HTTP/1.1 INVALID OK\r\n\r\n";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_err());

    match result {
        Err(TransportError::HttpParse(HttpParseError::InvalidStatusCode { status_code })) => {
            assert_eq!(status_code, "INVALID");
        }
        _ => panic!("Expected InvalidStatusCode error"),
    }
}

#[test]
fn test_try_bytes_to_response_missing_status_code() {
    let response_bytes = b"HTTP/1.1\r\n\r\n";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_err());
}

#[test]
fn test_try_bytes_to_response_no_status_text() {
    // Status text is optional in HTTP
    let response_bytes = b"HTTP/1.1 200\r\n\r\n";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

// ============================================================================
// Error Type Tests
// ============================================================================

#[test]
fn test_http_parse_error_display() {
    let error = HttpParseError::MissingComponent {
        message_type: "request",
        component: "path",
    };

    let error_str = format!("{}", error);
    assert!(error_str.contains("Missing required component"));
    assert!(error_str.contains("request"));
    assert!(error_str.contains("path"));
}

#[test]
fn test_http_parse_error_invalid_method_display() {
    let error = HttpParseError::InvalidMethod {
        method: "BADMETHOD".to_string(),
    };

    let error_str = format!("{}", error);
    assert!(error_str.contains("Invalid HTTP method"));
    assert!(error_str.contains("BADMETHOD"));
}

#[test]
fn test_http_parse_error_invalid_status_code_display() {
    let error = HttpParseError::InvalidStatusCode {
        status_code: "ABC".to_string(),
    };

    let error_str = format!("{}", error);
    assert!(error_str.contains("Invalid HTTP status code"));
    assert!(error_str.contains("ABC"));
}

#[test]
fn test_transport_error_from_http_parse_error() {
    let http_error = HttpParseError::RequestBuildFailed;
    let transport_error: TransportError = http_error.into();

    let error_str = format!("{}", transport_error);
    assert!(error_str.contains("HTTP parse error"));
}

#[test]
fn test_http_parse_error_properties() {
    let error = HttpParseError::InvalidMethod {
        method: "TEST".to_string(),
    };

    // Test that error can be cloned
    let error_clone = error.clone();
    assert_eq!(error, error_clone);

    // Test that error can be compared
    let same_error = HttpParseError::InvalidMethod {
        method: "TEST".to_string(),
    };
    assert_eq!(error, same_error);

    let different_error = HttpParseError::InvalidMethod {
        method: "OTHER".to_string(),
    };
    assert_ne!(error, different_error);
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<HttpParseError>();
    assert_sync::<HttpParseError>();
    assert_send::<TransportError>();
    assert_sync::<TransportError>();
}

// ============================================================================
// Complex Request/Response Tests
// ============================================================================

#[test]
fn test_try_bytes_to_request_with_multiple_headers() {
    let request_bytes = b"POST /api HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/json\r\nAuthorization: Bearer token123\r\n\r\n";

    let result = try_bytes_to_request(request_bytes);
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.method(), "POST");
    assert!(request.headers().contains_key("host"));
    assert!(request.headers().contains_key("content-type"));
    assert!(request.headers().contains_key("authorization"));
}

#[test]
fn test_try_bytes_to_response_with_multiple_headers() {
    let response_bytes = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 42\r\nCache-Control: no-cache\r\n\r\n";

    let result = try_bytes_to_response(response_bytes);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers().contains_key("content-type"));
    assert!(response.headers().contains_key("content-length"));
    assert!(response.headers().contains_key("cache-control"));
}

#[test]
fn test_round_trip_request() {
    use naia_shared::transport_udp::{bytes_to_request, request_to_bytes};

    // Create a request
    let original_request = http::Request::builder()
        .method("GET")
        .uri("/test")
        .header("Host", "example.com")
        .body(vec![])
        .unwrap();

    // Convert to bytes
    let bytes = request_to_bytes(original_request);

    // Try to parse back
    let result = try_bytes_to_request(&bytes);
    assert!(result.is_ok());

    let parsed_request = result.unwrap();
    assert_eq!(parsed_request.method(), "GET");
    assert_eq!(parsed_request.uri().path(), "/test");
}

#[test]
fn test_round_trip_response() {
    use naia_shared::transport_udp::{bytes_to_response, response_to_bytes};

    // Create a response
    let original_response = http::Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .body(b"Hello".to_vec())
        .unwrap();

    // Convert to bytes
    let bytes = response_to_bytes(original_response);

    // Try to parse back
    let result = try_bytes_to_response(&bytes);
    assert!(result.is_ok());

    let parsed_response = result.unwrap();
    assert_eq!(parsed_response.status().as_u16(), 200);
    assert_eq!(parsed_response.body(), b"Hello");
}
