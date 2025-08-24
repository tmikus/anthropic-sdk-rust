//! Error types for the Anthropic SDK

use std::time::Duration;
use thiserror::Error;

/// Main error type for the Anthropic SDK
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// API error response from Anthropic
    #[error("API error: {status} - {message}")]
    Api {
        status: reqwest::StatusCode,
        message: String,
        error_type: Option<String>,
        request_id: Option<String>,
    },

    /// Authentication failed - invalid API key or missing credentials
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limit exceeded - too many requests
    #[error("Rate limit exceeded{}", match .retry_after {
        Some(duration) => format!(", retry after {:?}", duration),
        None => String::new(),
    })]
    RateLimit { 
        retry_after: Option<Duration>,
        request_id: Option<String>,
    },

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error - invalid client setup or parameters
    #[error("Configuration error: {0}")]
    Config(String),

    /// Stream processing error
    #[error("Stream error: {0}")]
    Stream(String),

    /// URL parsing error
    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    /// Network connectivity error
    #[error("Network error: {0}")]
    Network(String),

    /// Request timeout error
    #[error("Request timeout after {timeout:?}")]
    Timeout { 
        timeout: Duration,
        request_id: Option<String>,
    },

    /// Invalid request parameters
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Server returned invalid response format
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Model not found or unavailable
    #[error("Model error: {0}")]
    Model(String),

    /// Tool execution error
    #[error("Tool error: {0}")]
    Tool(String),

    /// Content processing error (images, documents, etc.)
    #[error("Content processing error: {0}")]
    Content(String),
}

/// Error categories for easier error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (connectivity, timeouts)
    Network,
    /// Authentication and authorization errors
    Auth,
    /// Rate limiting errors
    RateLimit,
    /// Client configuration errors
    Config,
    /// Request validation errors
    Request,
    /// Server-side errors
    Server,
    /// Data processing errors
    Processing,
    /// Stream-specific errors
    Stream,
}

impl Error {
    /// Create a new API error
    pub fn api(
        status: reqwest::StatusCode,
        message: impl Into<String>,
        error_type: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self::Api {
            status,
            message: message.into(),
            error_type,
            request_id,
        }
    }

    /// Create a new rate limit error
    pub fn rate_limit(retry_after: Option<Duration>, request_id: Option<String>) -> Self {
        Self::RateLimit {
            retry_after,
            request_id,
        }
    }

    /// Create a new timeout error
    pub fn timeout(timeout: Duration, request_id: Option<String>) -> Self {
        Self::Timeout {
            timeout,
            request_id,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            // Network errors are generally retryable
            Error::Http(e) => {
                e.is_timeout() || e.is_connect() || e.is_request()
            }
            Error::Network(_) => true,
            Error::Timeout { .. } => true,
            
            // API errors - retry on server errors and rate limits
            Error::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            Error::RateLimit { .. } => true,
            
            // Stream errors might be retryable depending on context
            Error::Stream(_) => false, // Conservative approach
            
            // Client errors are generally not retryable
            Error::Authentication(_) |
            Error::Config(_) |
            Error::InvalidRequest(_) |
            Error::Serialization(_) |
            Error::Url(_) |
            Error::InvalidResponse(_) |
            Error::Model(_) |
            Error::Tool(_) |
            Error::Content(_) => false,
        }
    }

    /// Get the request ID if available
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Error::Api { request_id, .. } |
            Error::RateLimit { request_id, .. } |
            Error::Timeout { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Error::Http(_) | Error::Network(_) | Error::Timeout { .. } => ErrorCategory::Network,
            Error::Authentication(_) => ErrorCategory::Auth,
            Error::RateLimit { .. } => ErrorCategory::RateLimit,
            Error::Config(_) => ErrorCategory::Config,
            Error::InvalidRequest(_) | Error::Url(_) => ErrorCategory::Request,
            Error::Api { status, .. } => {
                if status.is_client_error() {
                    if *status == reqwest::StatusCode::UNAUTHORIZED || 
                       *status == reqwest::StatusCode::FORBIDDEN {
                        ErrorCategory::Auth
                    } else if *status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        ErrorCategory::RateLimit
                    } else {
                        ErrorCategory::Request
                    }
                } else {
                    ErrorCategory::Server
                }
            }
            Error::Serialization(_) | Error::InvalidResponse(_) | 
            Error::Model(_) | Error::Tool(_) | Error::Content(_) => ErrorCategory::Processing,
            Error::Stream(_) => ErrorCategory::Stream,
        }
    }

    /// Check if the error is a client error (4xx status codes)
    pub fn is_client_error(&self) -> bool {
        match self {
            Error::Api { status, .. } => status.is_client_error(),
            Error::Authentication(_) |
            Error::Config(_) |
            Error::InvalidRequest(_) |
            Error::Url(_) => true,
            _ => false,
        }
    }

    /// Check if the error is a server error (5xx status codes)
    pub fn is_server_error(&self) -> bool {
        match self {
            Error::Api { status, .. } => status.is_server_error(),
            _ => false,
        }
    }

    /// Check if the error is network-related
    pub fn is_network_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::Network)
    }

    /// Check if the error is authentication-related
    pub fn is_auth_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::Auth)
    }

    /// Check if the error is rate limit-related
    pub fn is_rate_limit_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::RateLimit)
    }

    /// Get retry delay suggestion for retryable errors
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            Error::RateLimit { retry_after, .. } => *retry_after,
            Error::Api { status, .. } if status.is_server_error() => {
                Some(Duration::from_secs(1)) // Default 1 second for server errors
            }
            Error::Http(_) | Error::Network(_) | Error::Timeout { .. } => {
                Some(Duration::from_millis(500)) // Default 500ms for network errors
            }
            _ => None,
        }
    }
}

/// Result type alias for the Anthropic SDK
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn test_api_error_creation() {
        let error = Error::api(
            StatusCode::BAD_REQUEST,
            "Invalid request",
            Some("invalid_request_error".to_string()),
            Some("req_123".to_string()),
        );

        match error {
            Error::Api { status, message, error_type, request_id } => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(message, "Invalid request");
                assert_eq!(error_type, Some("invalid_request_error".to_string()));
                assert_eq!(request_id, Some("req_123".to_string()));
            }
            _ => panic!("Expected Api error"),
        }
    }

    #[test]
    fn test_rate_limit_error_creation() {
        let retry_after = Duration::from_secs(60);
        let error = Error::rate_limit(Some(retry_after), Some("req_456".to_string()));

        match error {
            Error::RateLimit { retry_after: Some(duration), request_id } => {
                assert_eq!(duration, Duration::from_secs(60));
                assert_eq!(request_id, Some("req_456".to_string()));
            }
            _ => panic!("Expected RateLimit error"),
        }
    }

    #[test]
    fn test_timeout_error_creation() {
        let timeout = Duration::from_secs(30);
        let error = Error::timeout(timeout, Some("req_789".to_string()));

        match error {
            Error::Timeout { timeout: t, request_id } => {
                assert_eq!(t, Duration::from_secs(30));
                assert_eq!(request_id, Some("req_789".to_string()));
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[test]
    fn test_is_retryable() {
        // Retryable errors
        assert!(Error::Network("Connection failed".to_string()).is_retryable());
        assert!(Error::Timeout { 
            timeout: Duration::from_secs(30), 
            request_id: None 
        }.is_retryable());
        assert!(Error::api(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server error",
            None,
            None
        ).is_retryable());
        assert!(Error::api(
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limited",
            None,
            None
        ).is_retryable());
        assert!(Error::rate_limit(None, None).is_retryable());

        // Non-retryable errors
        assert!(!Error::Authentication("Invalid API key".to_string()).is_retryable());
        assert!(!Error::Config("Invalid config".to_string()).is_retryable());
        assert!(!Error::InvalidRequest("Bad request".to_string()).is_retryable());
        assert!(!Error::Stream("Stream error".to_string()).is_retryable());
        assert!(!Error::api(
            StatusCode::BAD_REQUEST,
            "Bad request",
            None,
            None
        ).is_retryable());
    }

    #[test]
    fn test_request_id_extraction() {
        let error_with_id = Error::api(
            StatusCode::BAD_REQUEST,
            "Error",
            None,
            Some("req_123".to_string())
        );
        assert_eq!(error_with_id.request_id(), Some("req_123"));

        let rate_limit_with_id = Error::rate_limit(None, Some("req_456".to_string()));
        assert_eq!(rate_limit_with_id.request_id(), Some("req_456"));

        let timeout_with_id = Error::timeout(Duration::from_secs(30), Some("req_789".to_string()));
        assert_eq!(timeout_with_id.request_id(), Some("req_789"));

        let error_without_id = Error::Config("Config error".to_string());
        assert_eq!(error_without_id.request_id(), None);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(Error::Network("test".to_string()).category(), ErrorCategory::Network);
        assert_eq!(Error::Authentication("test".to_string()).category(), ErrorCategory::Auth);
        assert_eq!(Error::rate_limit(None, None).category(), ErrorCategory::RateLimit);
        assert_eq!(Error::Config("test".to_string()).category(), ErrorCategory::Config);
        assert_eq!(Error::InvalidRequest("test".to_string()).category(), ErrorCategory::Request);
        assert_eq!(Error::Serialization(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::Other, "test"
        ))).category(), ErrorCategory::Processing);
        assert_eq!(Error::Stream("test".to_string()).category(), ErrorCategory::Stream);

        // API error categories
        assert_eq!(Error::api(StatusCode::UNAUTHORIZED, "test", None, None).category(), ErrorCategory::Auth);
        assert_eq!(Error::api(StatusCode::FORBIDDEN, "test", None, None).category(), ErrorCategory::Auth);
        assert_eq!(Error::api(StatusCode::TOO_MANY_REQUESTS, "test", None, None).category(), ErrorCategory::RateLimit);
        assert_eq!(Error::api(StatusCode::BAD_REQUEST, "test", None, None).category(), ErrorCategory::Request);
        assert_eq!(Error::api(StatusCode::INTERNAL_SERVER_ERROR, "test", None, None).category(), ErrorCategory::Server);
    }

    #[test]
    fn test_client_server_error_detection() {
        let client_error = Error::api(StatusCode::BAD_REQUEST, "Bad request", None, None);
        assert!(client_error.is_client_error());
        assert!(!client_error.is_server_error());

        let server_error = Error::api(StatusCode::INTERNAL_SERVER_ERROR, "Server error", None, None);
        assert!(!server_error.is_client_error());
        assert!(server_error.is_server_error());

        let auth_error = Error::Authentication("Invalid key".to_string());
        assert!(auth_error.is_client_error());
        assert!(!auth_error.is_server_error());

        let network_error = Error::Network("Connection failed".to_string());
        assert!(!network_error.is_client_error());
        assert!(!network_error.is_server_error());
    }

    #[test]
    fn test_specific_error_type_detection() {
        assert!(Error::Network("test".to_string()).is_network_error());
        assert!(!Error::Authentication("test".to_string()).is_network_error());

        assert!(Error::Authentication("test".to_string()).is_auth_error());
        assert!(!Error::Network("test".to_string()).is_auth_error());

        assert!(Error::rate_limit(None, None).is_rate_limit_error());
        assert!(!Error::Authentication("test".to_string()).is_rate_limit_error());
    }

    #[test]
    fn test_retry_delay_suggestions() {
        let rate_limit = Error::rate_limit(Some(Duration::from_secs(60)), None);
        assert_eq!(rate_limit.retry_delay(), Some(Duration::from_secs(60)));

        let server_error = Error::api(StatusCode::INTERNAL_SERVER_ERROR, "Error", None, None);
        assert_eq!(server_error.retry_delay(), Some(Duration::from_secs(1)));

        let network_error = Error::Network("Connection failed".to_string());
        assert_eq!(network_error.retry_delay(), Some(Duration::from_millis(500)));

        let timeout_error = Error::timeout(Duration::from_secs(30), None);
        assert_eq!(timeout_error.retry_delay(), Some(Duration::from_millis(500)));

        let auth_error = Error::Authentication("Invalid key".to_string());
        assert_eq!(auth_error.retry_delay(), None);
    }

    #[test]
    fn test_error_display() {
        let api_error = Error::api(
            StatusCode::BAD_REQUEST,
            "Invalid request",
            Some("invalid_request_error".to_string()),
            Some("req_123".to_string())
        );
        let display = format!("{}", api_error);
        assert!(display.contains("API error: 400 Bad Request - Invalid request"));

        let rate_limit = Error::rate_limit(Some(Duration::from_secs(60)), None);
        let display = format!("{}", rate_limit);
        assert!(display.contains("Rate limit exceeded, retry after 60s"));

        let timeout = Error::timeout(Duration::from_secs(30), None);
        let display = format!("{}", timeout);
        assert!(display.contains("Request timeout after 30s"));
    }

    #[test]
    fn test_error_from_conversions() {
        // Test serde_json::Error conversion
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_error.is_err());
        let error: Error = json_error.unwrap_err().into();
        matches!(error, Error::Serialization(_));

        // Test url::ParseError conversion
        let url_error = url::Url::parse("not a url");
        assert!(url_error.is_err());
        let error: Error = url_error.unwrap_err().into();
        matches!(error, Error::Url(_));
    }

    #[test]
    fn test_error_category_equality() {
        assert_eq!(ErrorCategory::Network, ErrorCategory::Network);
        assert_ne!(ErrorCategory::Network, ErrorCategory::Auth);
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_result() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_error() -> Result<String> {
            Err(Error::Config("test error".to_string()))
        }

        assert!(returns_result().is_ok());
        assert!(returns_error().is_err());
    }
}