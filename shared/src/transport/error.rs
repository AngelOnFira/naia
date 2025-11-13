use thiserror::Error;

/// Errors that can occur during HTTP request/response parsing in UDP transport
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HttpParseError {
    /// Missing required component in HTTP request/response
    #[error("Missing required component in HTTP {message_type}: {component}. The HTTP message is malformed")]
    MissingComponent {
        message_type: &'static str,
        component: &'static str,
    },

    /// Invalid HTTP method in request
    #[error("Invalid HTTP method '{method}' in request. Method must be a valid HTTP verb")]
    InvalidMethod {
        method: String,
    },

    /// Invalid status code in response
    #[error("Invalid HTTP status code '{status_code}' in response. Status code must be a valid u16 integer")]
    InvalidStatusCode {
        status_code: String,
    },

    /// Failed to build HTTP request
    #[error("Failed to build HTTP request. Invalid request parameters provided")]
    RequestBuildFailed,

    /// Failed to build HTTP response
    #[error("Failed to build HTTP response with status code {status_code}. Invalid response parameters provided")]
    ResponseBuildFailed {
        status_code: u16,
    },
}

/// Errors that can occur during UDP transport operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    /// HTTP parse error
    #[error("HTTP parse error: {0}")]
    HttpParse(#[from] HttpParseError),
}
