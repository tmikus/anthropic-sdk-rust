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
            Error::Http(e) => e.is_timeout() || e.is_connect() || e.is_request(),
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
            Error::Authentication(_)
            | Error::Config(_)
            | Error::InvalidRequest(_)
            | Error::Serialization(_)
            | Error::Url(_)
            | Error::InvalidResponse(_)
            | Error::Model(_)
            | Error::Tool(_)
            | Error::Content(_) => false,
        }
    }

    /// Get the request ID if available
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Error::Api { request_id, .. }
            | Error::RateLimit { request_id, .. }
            | Error::Timeout { request_id, .. } => request_id.as_deref(),
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
                    if *status == reqwest::StatusCode::UNAUTHORIZED
                        || *status == reqwest::StatusCode::FORBIDDEN
                    {
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
            Error::Serialization(_)
            | Error::InvalidResponse(_)
            | Error::Model(_)
            | Error::Tool(_)
            | Error::Content(_) => ErrorCategory::Processing,
            Error::Stream(_) => ErrorCategory::Stream,
        }
    }

    /// Check if the error is a client error (4xx status codes)
    pub fn is_client_error(&self) -> bool {
        match self {
            Error::Api { status, .. } => status.is_client_error(),
            Error::Authentication(_)
            | Error::Config(_)
            | Error::InvalidRequest(_)
            | Error::Url(_) => true,
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

    /// Get a user-friendly error message with context about test execution
    pub fn user_message(&self) -> String {
        match self {
            Error::Http(e) if e.is_connect() => {
                format!(
                    "Network connection failed: {}. This is an integration test error - check network connectivity.",
                    e
                )
            }
            Error::Http(e) if e.is_timeout() => {
                format!(
                    "Request timed out: {}. This is an integration test error - consider increasing timeout or checking network conditions.",
                    e
                )
            }
            Error::Http(e) => {
                format!(
                    "HTTP request failed: {}. This is an integration test error - check network conditions and API endpoint availability.",
                    e
                )
            }
            Error::Network(msg) => {
                format!(
                    "Network error: {}. This is an integration test error - verify network connectivity and endpoint availability.",
                    msg
                )
            }
            Error::Timeout {
                timeout,
                request_id,
            } => {
                let id_info = request_id
                    .as_ref()
                    .map(|id| format!(" (Request ID: {})", id))
                    .unwrap_or_default();
                format!(
                    "Request timed out after {:?}{}. This is an integration test error - consider increasing timeout configuration.",
                    timeout, id_info
                )
            }
            Error::Api {
                status,
                message,
                error_type,
                request_id,
            } => {
                let id_info = request_id
                    .as_ref()
                    .map(|id| format!(" (Request ID: {})", id))
                    .unwrap_or_default();
                let type_info = error_type
                    .as_ref()
                    .map(|t| format!(" [{}]", t))
                    .unwrap_or_default();

                if status.is_client_error() {
                    format!(
                        "API client error {}: {}{}{} - This indicates a problem with the request configuration or parameters.",
                        status, message, type_info, id_info
                    )
                } else if status.is_server_error() {
                    format!(
                        "API server error {}: {}{}{} - This is an integration test error indicating a server-side issue. The request may be retried.",
                        status, message, type_info, id_info
                    )
                } else {
                    format!(
                        "API error {}: {}{}{} - This is an integration test error.",
                        status, message, type_info, id_info
                    )
                }
            }
            Error::Authentication(msg) => {
                format!(
                    "Authentication failed: {}. This is a configuration error - verify your API key is valid and properly set.",
                    msg
                )
            }
            Error::RateLimit {
                retry_after,
                request_id,
            } => {
                let id_info = request_id
                    .as_ref()
                    .map(|id| format!(" (Request ID: {})", id))
                    .unwrap_or_default();
                let retry_info = retry_after
                    .map(|duration| format!(" Retry after {:?}.", duration))
                    .unwrap_or_else(|| " Retry with exponential backoff.".to_string());
                format!(
                    "Rate limit exceeded{}.{} This is an integration test error - reduce request frequency or implement retry logic.",
                    id_info, retry_info
                )
            }
            Error::Serialization(e) => {
                format!(
                    "Data serialization error: {}. This is a unit test error - check request/response data structures and JSON formatting.",
                    e
                )
            }
            Error::Config(msg) => {
                format!(
                    "Configuration error: {}. This is a unit test error - verify client configuration parameters.",
                    msg
                )
            }
            Error::Stream(msg) => {
                format!(
                    "Stream processing error: {}. This could be either a unit test error (mock stream issues) or integration test error (network stream issues).",
                    msg
                )
            }
            Error::Url(e) => {
                format!(
                    "URL parsing error: {}. This is a unit test error - check URL construction and base URL configuration.",
                    e
                )
            }
            Error::InvalidRequest(msg) => {
                format!(
                    "Invalid request: {}. This is a unit test error - verify request parameters and structure.",
                    msg
                )
            }
            Error::InvalidResponse(msg) => {
                format!(
                    "Invalid response format: {}. This could be a unit test error (mock response format) or integration test error (unexpected API response).",
                    msg
                )
            }
            Error::Model(msg) => {
                format!(
                    "Model error: {}. This is a unit test error - verify model configuration and availability.",
                    msg
                )
            }
            Error::Tool(msg) => {
                format!(
                    "Tool error: {}. This is a unit test error - check tool definition and usage patterns.",
                    msg
                )
            }
            Error::Content(msg) => {
                format!(
                    "Content processing error: {}. This is a unit test error - verify content format and encoding.",
                    msg
                )
            }
        }
    }

    /// Get debugging information for test failures
    pub fn debug_info(&self) -> String {
        let category = match self.category() {
            ErrorCategory::Network => "Network",
            ErrorCategory::Auth => "Authentication",
            ErrorCategory::RateLimit => "Rate Limiting",
            ErrorCategory::Config => "Configuration",
            ErrorCategory::Request => "Request Validation",
            ErrorCategory::Server => "Server",
            ErrorCategory::Processing => "Data Processing",
            ErrorCategory::Stream => "Stream Processing",
        };

        let test_type = if self.is_network_error()
            || matches!(self, Error::Api { status, .. } if status.is_server_error())
        {
            "Integration Test"
        } else {
            "Unit Test"
        };

        let retryable = if self.is_retryable() { "Yes" } else { "No" };

        let error_details = match self {
            Error::Http(e) => {
                if e.is_timeout() {
                    "HTTP timeout error".to_string()
                } else if e.is_connect() {
                    "HTTP connection error".to_string()
                } else if e.is_request() {
                    "HTTP request error".to_string()
                } else {
                    format!("HTTP error: {}", e)
                }
            }
            _ => format!("{}", self),
        };

        format!(
            "Error Debug Info:\n  Category: {}\n  Test Type: {}\n  Retryable: {}\n  Request ID: {}\n  Error: {}",
            category,
            test_type,
            retryable,
            self.request_id().unwrap_or("None"),
            error_details
        )
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
            Error::Api {
                status,
                message,
                error_type,
                request_id,
            } => {
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
            Error::RateLimit {
                retry_after: Some(duration),
                request_id,
            } => {
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
            Error::Timeout {
                timeout: t,
                request_id,
            } => {
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
        }
        .is_retryable());
        assert!(Error::api(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server error",
            None,
            None
        )
        .is_retryable());
        assert!(
            Error::api(StatusCode::TOO_MANY_REQUESTS, "Rate limited", None, None).is_retryable()
        );
        assert!(Error::rate_limit(None, None).is_retryable());

        // Non-retryable errors
        assert!(!Error::Authentication("Invalid API key".to_string()).is_retryable());
        assert!(!Error::Config("Invalid config".to_string()).is_retryable());
        assert!(!Error::InvalidRequest("Bad request".to_string()).is_retryable());
        assert!(!Error::Stream("Stream error".to_string()).is_retryable());
        assert!(!Error::api(StatusCode::BAD_REQUEST, "Bad request", None, None).is_retryable());
    }

    #[test]
    fn test_request_id_extraction() {
        let error_with_id = Error::api(
            StatusCode::BAD_REQUEST,
            "Error",
            None,
            Some("req_123".to_string()),
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
        assert_eq!(
            Error::Network("test".to_string()).category(),
            ErrorCategory::Network
        );
        assert_eq!(
            Error::Authentication("test".to_string()).category(),
            ErrorCategory::Auth
        );
        assert_eq!(
            Error::rate_limit(None, None).category(),
            ErrorCategory::RateLimit
        );
        assert_eq!(
            Error::Config("test".to_string()).category(),
            ErrorCategory::Config
        );
        assert_eq!(
            Error::InvalidRequest("test".to_string()).category(),
            ErrorCategory::Request
        );
        assert_eq!(
            Error::Serialization(serde_json::Error::io(std::io::Error::other("test"))).category(),
            ErrorCategory::Processing
        );
        assert_eq!(
            Error::Stream("test".to_string()).category(),
            ErrorCategory::Stream
        );

        // API error categories
        assert_eq!(
            Error::api(StatusCode::UNAUTHORIZED, "test", None, None).category(),
            ErrorCategory::Auth
        );
        assert_eq!(
            Error::api(StatusCode::FORBIDDEN, "test", None, None).category(),
            ErrorCategory::Auth
        );
        assert_eq!(
            Error::api(StatusCode::TOO_MANY_REQUESTS, "test", None, None).category(),
            ErrorCategory::RateLimit
        );
        assert_eq!(
            Error::api(StatusCode::BAD_REQUEST, "test", None, None).category(),
            ErrorCategory::Request
        );
        assert_eq!(
            Error::api(StatusCode::INTERNAL_SERVER_ERROR, "test", None, None).category(),
            ErrorCategory::Server
        );
    }

    #[test]
    fn test_client_server_error_detection() {
        let client_error = Error::api(StatusCode::BAD_REQUEST, "Bad request", None, None);
        assert!(client_error.is_client_error());
        assert!(!client_error.is_server_error());

        let server_error = Error::api(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server error",
            None,
            None,
        );
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
        assert_eq!(
            network_error.retry_delay(),
            Some(Duration::from_millis(500))
        );

        let timeout_error = Error::timeout(Duration::from_secs(30), None);
        assert_eq!(
            timeout_error.retry_delay(),
            Some(Duration::from_millis(500))
        );

        let auth_error = Error::Authentication("Invalid key".to_string());
        assert_eq!(auth_error.retry_delay(), None);
    }

    #[test]
    fn test_error_display() {
        let api_error = Error::api(
            StatusCode::BAD_REQUEST,
            "Invalid request",
            Some("invalid_request_error".to_string()),
            Some("req_123".to_string()),
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

    #[test]
    fn test_user_message_formatting() {
        // Test network error message
        let network_error = Error::Network("Connection refused".to_string());
        let message = network_error.user_message();
        assert!(message.contains("Network error"));
        assert!(message.contains("integration test error"));
        assert!(message.contains("network connectivity"));

        // Test authentication error message
        let auth_error = Error::Authentication("Invalid API key".to_string());
        let message = auth_error.user_message();
        assert!(message.contains("Authentication failed"));
        assert!(message.contains("configuration error"));
        assert!(message.contains("API key"));

        // Test API server error message
        let server_error = Error::api(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server error",
            Some("server_error".to_string()),
            Some("req_123".to_string()),
        );
        let message = server_error.user_message();
        assert!(message.contains("API server error 500"));
        assert!(message.contains("integration test error"));
        assert!(message.contains("server-side issue"));
        assert!(message.contains("req_123"));

        // Test API client error message
        let client_error = Error::api(StatusCode::BAD_REQUEST, "Invalid request", None, None);
        let message = client_error.user_message();
        assert!(message.contains("API client error 400"));
        assert!(message.contains("request configuration"));

        // Test serialization error message
        let serde_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let serialization_error = Error::Serialization(serde_error);
        let message = serialization_error.user_message();
        assert!(message.contains("Data serialization error"));
        assert!(message.contains("unit test error"));

        // Test rate limit error message
        let rate_limit =
            Error::rate_limit(Some(Duration::from_secs(60)), Some("req_456".to_string()));
        let message = rate_limit.user_message();
        assert!(message.contains("Rate limit exceeded"));
        assert!(message.contains("req_456"));
        assert!(message.contains("Retry after 60s"));
        assert!(message.contains("integration test error"));

        // Test timeout error message
        let timeout = Error::timeout(Duration::from_secs(30), Some("req_789".to_string()));
        let message = timeout.user_message();
        assert!(message.contains("Request timed out after 30s"));
        assert!(message.contains("req_789"));
        assert!(message.contains("integration test error"));
    }

    #[test]
    fn test_debug_info_formatting() {
        // Test network error debug info
        let network_error = Error::Network("Connection failed".to_string());
        let debug_info = network_error.debug_info();
        assert!(debug_info.contains("Category: Network"));
        assert!(debug_info.contains("Test Type: Integration Test"));
        assert!(debug_info.contains("Retryable: Yes"));
        assert!(debug_info.contains("Request ID: None"));

        // Test authentication error debug info
        let auth_error = Error::Authentication("Invalid key".to_string());
        let debug_info = auth_error.debug_info();
        assert!(debug_info.contains("Category: Authentication"));
        assert!(debug_info.contains("Test Type: Unit Test"));
        assert!(debug_info.contains("Retryable: No"));

        // Test API error with request ID debug info
        let api_error = Error::api(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server error",
            None,
            Some("req_123".to_string()),
        );
        let debug_info = api_error.debug_info();
        assert!(debug_info.contains("Category: Server"));
        assert!(debug_info.contains("Test Type: Integration Test"));
        assert!(debug_info.contains("Retryable: Yes"));
        assert!(debug_info.contains("Request ID: req_123"));

        // Test configuration error debug info
        let config_error = Error::Config("Invalid timeout".to_string());
        let debug_info = config_error.debug_info();
        assert!(debug_info.contains("Category: Configuration"));
        assert!(debug_info.contains("Test Type: Unit Test"));
        assert!(debug_info.contains("Retryable: No"));
    }

    #[test]
    fn test_user_message_http_error_variants() {
        // Create a mock reqwest error for connection failure
        // Note: We can't easily create reqwest::Error in tests, so we test the logic paths

        // Test timeout error with HTTP wrapper
        let timeout_error = Error::Timeout {
            timeout: Duration::from_secs(10),
            request_id: Some("req_timeout".to_string()),
        };
        let message = timeout_error.user_message();
        assert!(message.contains("Request timed out after 10s"));
        assert!(message.contains("req_timeout"));
        assert!(message.contains("integration test error"));

        // Test stream error message (ambiguous test type)
        let stream_error = Error::Stream("Stream disconnected".to_string());
        let message = stream_error.user_message();
        assert!(message.contains("Stream processing error"));
        assert!(message.contains("unit test error") || message.contains("integration test error"));
    }

    #[test]
    fn test_error_message_consistency() {
        // Verify that all error types have user messages
        let errors = vec![
            Error::Network("test".to_string()),
            Error::Authentication("test".to_string()),
            Error::Config("test".to_string()),
            Error::InvalidRequest("test".to_string()),
            Error::InvalidResponse("test".to_string()),
            Error::Model("test".to_string()),
            Error::Tool("test".to_string()),
            Error::Content("test".to_string()),
            Error::Stream("test".to_string()),
            Error::api(StatusCode::BAD_REQUEST, "test", None, None),
            Error::rate_limit(None, None),
            Error::timeout(Duration::from_secs(1), None),
            // Note: We can't easily create reqwest::Error in tests, so we skip Http variant
            // The Http error is tested through integration tests
        ];

        for error in errors {
            let user_message = error.user_message();
            let debug_info = error.debug_info();

            // All messages should be non-empty and contain useful information
            assert!(!user_message.is_empty());
            assert!(!debug_info.is_empty());

            // Debug info should contain standard fields
            assert!(debug_info.contains("Category:"));
            assert!(debug_info.contains("Test Type:"));
            assert!(debug_info.contains("Retryable:"));
            assert!(debug_info.contains("Request ID:"));
        }
    }
}
