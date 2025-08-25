//! Comprehensive unit tests for error handling

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use pretty_assertions::assert_eq;
    use std::time::Duration;

    #[test]
    fn test_error_display() {
        let network_error = Error::Network("Connection failed".to_string());
        assert_eq!(
            network_error.to_string(),
            "Network error: Connection failed"
        );

        let auth_error = Error::Authentication("Invalid API key".to_string());
        assert_eq!(
            auth_error.to_string(),
            "Authentication failed: Invalid API key"
        );

        let api_error = Error::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            message: "Invalid request".to_string(),
            error_type: Some("validation_error".to_string()),
            request_id: Some("req-123".to_string()),
        };
        assert_eq!(
            api_error.to_string(),
            "API error: 400 Bad Request - Invalid request"
        );

        let rate_limit_error = Error::RateLimit {
            retry_after: Some(Duration::from_secs(60)),
            request_id: Some("req-456".to_string()),
        };
        assert_eq!(
            rate_limit_error.to_string(),
            "Rate limit exceeded, retry after 60s"
        );

        let config_error = Error::Config("Invalid configuration".to_string());
        assert_eq!(
            config_error.to_string(),
            "Configuration error: Invalid configuration"
        );

        let stream_error = Error::Stream("Stream interrupted".to_string());
        assert_eq!(stream_error.to_string(), "Stream error: Stream interrupted");

        let timeout_error = Error::Timeout {
            timeout: Duration::from_secs(30),
            request_id: None,
        };
        assert_eq!(timeout_error.to_string(), "Request timeout after 30s");

        let invalid_request_error = Error::InvalidRequest("Missing required field".to_string());
        assert_eq!(
            invalid_request_error.to_string(),
            "Invalid request: Missing required field"
        );

        let invalid_response_error = Error::InvalidResponse("Malformed JSON".to_string());
        assert_eq!(
            invalid_response_error.to_string(),
            "Invalid response format: Malformed JSON"
        );
    }

    #[test]
    fn test_error_categorization() {
        // Network errors
        let network_error = Error::Network("Connection failed".to_string());
        assert!(network_error.is_network_error());
        assert!(network_error.is_retryable());
        assert!(!network_error.is_auth_error());
        assert!(!network_error.is_rate_limit_error());
        assert!(!network_error.is_server_error());

        // Authentication errors
        let auth_error = Error::Authentication("Invalid API key".to_string());
        assert!(auth_error.is_auth_error());
        assert!(!auth_error.is_retryable());
        assert!(!auth_error.is_network_error());
        assert!(!auth_error.is_rate_limit_error());
        assert!(!auth_error.is_server_error());

        // Rate limit errors
        let rate_limit_error = Error::RateLimit {
            retry_after: Some(Duration::from_secs(60)),
            request_id: Some("req-123".to_string()),
        };
        assert!(rate_limit_error.is_rate_limit_error());
        assert!(rate_limit_error.is_retryable());
        assert!(!rate_limit_error.is_auth_error());
        assert!(!rate_limit_error.is_network_error());
        assert!(!rate_limit_error.is_server_error());

        // Server errors (5xx)
        let server_error = Error::Api {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            message: "Server error".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(server_error.is_server_error());
        assert!(server_error.is_retryable());
        assert!(!server_error.is_auth_error());
        assert!(!server_error.is_rate_limit_error());
        assert!(!server_error.is_network_error());

        // Client errors (4xx, non-auth, non-rate-limit)
        let client_error = Error::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            message: "Bad request".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(!client_error.is_server_error());
        assert!(!client_error.is_retryable());
        assert!(!client_error.is_auth_error());
        assert!(!client_error.is_rate_limit_error());
        assert!(!client_error.is_network_error());

        // Configuration errors
        let config_error = Error::Config("Invalid config".to_string());
        assert!(!config_error.is_retryable());
        assert!(!config_error.is_auth_error());
        assert!(!config_error.is_rate_limit_error());
        assert!(!config_error.is_network_error());
        assert!(!config_error.is_server_error());

        // Timeout errors
        let timeout_error = Error::Timeout {
            timeout: Duration::from_secs(30),
            request_id: None,
        };
        assert!(timeout_error.is_retryable());
        assert!(!timeout_error.is_auth_error());
        assert!(!timeout_error.is_rate_limit_error());
        assert!(timeout_error.is_network_error()); // Timeout is a network error
        assert!(!timeout_error.is_server_error());
    }

    #[test]
    fn test_request_id_extraction() {
        let api_error = Error::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            message: "Bad request".to_string(),
            error_type: Some("validation_error".to_string()),
            request_id: Some("req-123".to_string()),
        };
        assert_eq!(api_error.request_id(), Some("req-123"));

        let rate_limit_error = Error::RateLimit {
            retry_after: Some(Duration::from_secs(60)),
            request_id: Some("req-456".to_string()),
        };
        assert_eq!(rate_limit_error.request_id(), Some("req-456"));

        let network_error = Error::Network("Connection failed".to_string());
        assert_eq!(network_error.request_id(), None);
    }

    // Note: retry_after method doesn't exist in the current Error implementation
    // This test would need to be implemented if the method is added

    #[test]
    fn test_error_network_creation() {
        // Test creating network errors directly
        let error = Error::Network("Connection failed".to_string());

        match error {
            Error::Network(msg) => {
                assert_eq!(msg, "Connection failed");
            }
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_error_from_serde_json() {
        // Create a serde_json error by parsing invalid JSON
        let json_error: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error = Error::from(json_error);

        match error {
            Error::Serialization(_) => {
                // Expected conversion - serde_json::Error converts to Serialization
            }
            _ => panic!("Expected Serialization error from serde_json::Error"),
        }
    }

    #[test]
    fn test_error_debug_format() {
        let api_error = Error::Api {
            status: reqwest::StatusCode::NOT_FOUND,
            message: "Resource not found".to_string(),
            error_type: Some("not_found".to_string()),
            request_id: Some("req-789".to_string()),
        };

        let debug_str = format!("{:?}", api_error);
        assert!(debug_str.contains("Api"));
        assert!(debug_str.contains("404"));
        assert!(debug_str.contains("Resource not found"));
        assert!(debug_str.contains("not_found"));
        assert!(debug_str.contains("req-789"));
    }

    #[test]
    fn test_error_chain() {
        // Test that errors can be chained properly
        let error = Error::Config("Invalid configuration".to_string());

        // Test that the error displays correctly
        assert_eq!(
            error.to_string(),
            "Configuration error: Invalid configuration"
        );

        // Test error categorization
        assert!(!error.is_retryable());
        assert!(!error.is_network_error());
    }

    #[test]
    fn test_specific_status_code_handling() {
        // Test 401 Unauthorized
        let auth_401 = Error::Api {
            status: reqwest::StatusCode::UNAUTHORIZED,
            message: "Unauthorized".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(auth_401.is_auth_error());
        assert!(!auth_401.is_retryable());

        // Test 403 Forbidden
        let auth_403 = Error::Api {
            status: reqwest::StatusCode::FORBIDDEN,
            message: "Forbidden".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(auth_403.is_auth_error());
        assert!(!auth_403.is_retryable());

        // Test 429 Too Many Requests
        let rate_limit_429 = Error::Api {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            message: "Rate limited".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(rate_limit_429.is_rate_limit_error());
        assert!(rate_limit_429.is_retryable());

        // Test 500 Internal Server Error
        let server_500 = Error::Api {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal error".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(server_500.is_server_error());
        assert!(server_500.is_retryable());

        // Test 502 Bad Gateway
        let server_502 = Error::Api {
            status: reqwest::StatusCode::BAD_GATEWAY,
            message: "Bad gateway".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(server_502.is_server_error());
        assert!(server_502.is_retryable());

        // Test 503 Service Unavailable
        let server_503 = Error::Api {
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            message: "Service unavailable".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(server_503.is_server_error());
        assert!(server_503.is_retryable());

        // Test 504 Gateway Timeout
        let server_504 = Error::Api {
            status: reqwest::StatusCode::GATEWAY_TIMEOUT,
            message: "Gateway timeout".to_string(),
            error_type: None,
            request_id: None,
        };
        assert!(server_504.is_server_error());
        assert!(server_504.is_retryable());
    }

    // Note: Error equality tests would require PartialEq derive on Error enum
    // This test would need to be implemented if PartialEq is added to Error

    // Note: Error clone tests would require Clone derive on Error enum
    // This test would need to be implemented if Clone is added to Error

    #[test]
    fn test_rate_limit_without_retry_after() {
        let rate_limit_error = Error::RateLimit {
            retry_after: None,
            request_id: Some("req-123".to_string()),
        };

        assert_eq!(rate_limit_error.to_string(), "Rate limit exceeded");
        assert!(rate_limit_error.is_rate_limit_error());
        assert!(rate_limit_error.is_retryable());
        // Note: retry_after method doesn't exist in current implementation
    }

    #[test]
    fn test_timeout_error_formatting() {
        let timeout_1s = Error::Timeout {
            timeout: Duration::from_secs(1),
            request_id: None,
        };
        assert_eq!(timeout_1s.to_string(), "Request timeout after 1s");

        let timeout_30s = Error::Timeout {
            timeout: Duration::from_secs(30),
            request_id: None,
        };
        assert_eq!(timeout_30s.to_string(), "Request timeout after 30s");

        let timeout_ms = Error::Timeout {
            timeout: Duration::from_millis(500),
            request_id: None,
        };
        assert!(timeout_ms.to_string().contains("500ms") || timeout_ms.to_string().contains("0.5"));
    }

    #[test]
    fn test_error_send_sync() {
        // Verify that Error implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Error>();
        assert_sync::<Error>();
    }

    #[test]
    fn test_error_std_error_trait() {
        let error = Error::Config("Test error".to_string());

        // Test that it implements std::error::Error
        let std_error: &dyn std::error::Error = &error;
        assert_eq!(std_error.to_string(), "Configuration error: Test error");

        // Test source chain (should be None for our simple errors)
        assert!(std_error.source().is_none());
    }
}
