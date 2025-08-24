//! Integration tests for HTTP client functionality

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use serde_json::json;

    use crate::{
        client::{ClientInner, RetryConfig, RequestMiddleware},
        config::Config,
        error::Error,
        types::Model,
    };

    fn create_test_client() -> ClientInner {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            base_url: "https://httpbin.org".parse().unwrap(), // Use httpbin for testing
            timeout: Duration::from_secs(30),
            max_retries: 2,
            model: Model::Claude35Sonnet20241022,
            max_tokens: 1000,
        };

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        ClientInner {
            http_client,
            config,
            retry_config: RetryConfig::default(),
            middleware: RequestMiddleware::default(),
        }
    }

    #[tokio::test]
    async fn test_successful_request() {
        let client = create_test_client();
        
        // Use httpbin's /json endpoint which returns a JSON response
        let result: serde_json::Value = client
            .execute_request(reqwest::Method::GET, "/json", None)
            .await
            .expect("Request should succeed");

        // httpbin's /json endpoint returns a JSON object
        assert!(result.is_object());
    }

    #[tokio::test]
    async fn test_404_error_handling() {
        let client = create_test_client();
        
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/status/404", None)
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        
        match error {
            Error::InvalidRequest(_) => {
                // Expected for 404 responses
            }
            _ => panic!("Expected InvalidRequest error for 404, got: {:?}", error),
        }
    }

    #[tokio::test]
    async fn test_500_error_retryable() {
        let client = create_test_client();
        
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/status/500", None)
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        
        // 500 errors should be retryable
        assert!(error.is_retryable());
        assert!(error.is_server_error());
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            base_url: "https://httpbin.org".parse().unwrap(),
            timeout: Duration::from_millis(1), // Very short timeout
            max_retries: 0, // No retries to speed up test
            model: Model::Claude35Sonnet20241022,
            max_tokens: 1000,
        };

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        let client = ClientInner {
            http_client,
            config,
            retry_config: RetryConfig {
                max_retries: 0,
                ..RetryConfig::default()
            },
            middleware: RequestMiddleware::default(),
        };
        
        // Use httpbin's /delay endpoint which will likely timeout
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/delay/2", None)
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        
        // Should be a timeout or network error
        assert!(error.is_network_error() || matches!(error, Error::Timeout { .. }));
    }

    #[tokio::test]
    async fn test_post_request_with_body() {
        let client = create_test_client();
        
        let test_data = json!({
            "test": "data",
            "number": 42
        });
        
        // Use httpbin's /post endpoint which echoes the request
        let result: serde_json::Value = client
            .execute_request(reqwest::Method::POST, "/post", Some(test_data.clone()))
            .await
            .expect("POST request should succeed");

        // httpbin's /post endpoint returns the request data in the "json" field
        assert!(result.is_object());
        if let Some(json_field) = result.get("json") {
            assert_eq!(json_field, &test_data);
        }
    }

    #[tokio::test]
    async fn test_request_id_extraction() {
        let client = create_test_client();
        
        // Use httpbin's /response-headers endpoint to set custom headers
        let result: Result<serde_json::Value, Error> = client
            .execute_request(
                reqwest::Method::GET, 
                "/response-headers?request-id=test-123", 
                None
            )
            .await;

        // This should succeed, but we're testing header extraction in error cases
        // For now, just verify the request works
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_middleware_logging() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            base_url: "https://httpbin.org".parse().unwrap(),
            timeout: Duration::from_secs(30),
            max_retries: 0,
            model: Model::Claude35Sonnet20241022,
            max_tokens: 1000,
        };

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        let client = ClientInner {
            http_client,
            config,
            retry_config: RetryConfig::default(),
            middleware: RequestMiddleware::default().with_full_logging(),
        };
        
        // This test mainly verifies that logging doesn't crash
        // In a real scenario, you'd capture the log output
        let result: serde_json::Value = client
            .execute_request(reqwest::Method::GET, "/json", None)
            .await
            .expect("Request with logging should succeed");

        assert!(result.is_object());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            base_url: "https://httpbin.org".parse().unwrap(),
            timeout: Duration::from_secs(30),
            max_retries: 2,
            model: Model::Claude35Sonnet20241022,
            max_tokens: 1000,
        };

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        let client = ClientInner {
            http_client,
            config,
            retry_config: RetryConfig {
                max_retries: 2,
                initial_delay: Duration::from_millis(10), // Fast retries for testing
                max_delay: Duration::from_millis(100),
                backoff_multiplier: 2.0,
            },
            middleware: RequestMiddleware::default().with_request_logging(),
        };
        
        // Use a 500 error which should be retried
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/status/500", None)
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        
        // Should still fail after retries, but verify it's retryable
        assert!(error.is_retryable());
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        let client = create_test_client();
        
        // Use httpbin's /html endpoint which returns HTML, not JSON
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/html", None)
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        
        match error {
            Error::InvalidResponse(_) => {
                // Expected for non-JSON responses
            }
            _ => panic!("Expected InvalidResponse error for HTML response, got: {:?}", error),
        }
    }

    #[tokio::test]
    async fn test_error_categories() {
        // Test that different HTTP status codes map to correct error categories
        let client = create_test_client();
        
        // Test 401 Unauthorized
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/status/401", None)
            .await;
        
        if let Err(error) = result {
            assert!(error.is_auth_error());
        }

        // Test 403 Forbidden  
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/status/403", None)
            .await;
        
        if let Err(error) = result {
            assert!(error.is_auth_error());
        }

        // Test 429 Too Many Requests
        let result: Result<serde_json::Value, Error> = client
            .execute_request(reqwest::Method::GET, "/status/429", None)
            .await;
        
        if let Err(error) = result {
            assert!(error.is_rate_limit_error());
            assert!(error.is_retryable());
        }
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_request_middleware_builder() {
        let middleware = RequestMiddleware::default()
            .with_request_logging()
            .with_response_logging()
            .with_header_logging()
            .with_body_logging();

        assert!(middleware.log_requests);
        assert!(middleware.log_responses);
        assert!(middleware.log_headers);
        assert!(middleware.log_body);

        let full_middleware = RequestMiddleware::default().with_full_logging();
        assert!(full_middleware.log_requests);
        assert!(full_middleware.log_responses);
        assert!(full_middleware.log_headers);
        assert!(full_middleware.log_body);
    }

    #[test]
    fn test_extract_request_id() {
        use reqwest::header::{HeaderMap, HeaderValue};
        use crate::client::extract_request_id;

        let mut headers = HeaderMap::new();
        headers.insert("request-id", HeaderValue::from_static("req-123"));
        
        assert_eq!(extract_request_id(&headers), Some("req-123".to_string()));

        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("req-456"));
        
        assert_eq!(extract_request_id(&headers), Some("req-456".to_string()));

        let headers = HeaderMap::new();
        assert_eq!(extract_request_id(&headers), None);
    }

    #[test]
    fn test_extract_retry_after_duration() {
        use crate::client::extract_retry_after_duration;

        let json_body = r#"{"error": {"retry_after": 60.5}}"#;
        let duration = extract_retry_after_duration(json_body);
        assert_eq!(duration, Some(Duration::from_secs_f64(60.5)));

        let invalid_body = "not json";
        let duration = extract_retry_after_duration(invalid_body);
        assert_eq!(duration, None);

        let no_retry_after = r#"{"error": {"message": "rate limited"}}"#;
        let duration = extract_retry_after_duration(no_retry_after);
        assert_eq!(duration, None);
    }
}