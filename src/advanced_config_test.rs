//! Tests for advanced configuration features

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{
    client::{LoggingInterceptor, RequestInterceptor, RequestMiddleware, RetryConfig},
    config::ClientBuilder,
    error::Error,
    types::Model,
    Result,
};

/// Mock interceptor for testing
#[derive(Debug)]
struct MockInterceptor {
    before_request_calls: Arc<Mutex<Vec<String>>>,
    after_response_calls: Arc<Mutex<Vec<String>>>,
    error_calls: Arc<Mutex<Vec<String>>>,
}

impl MockInterceptor {
    fn new() -> Self {
        Self {
            before_request_calls: Arc::new(Mutex::new(Vec::new())),
            after_response_calls: Arc::new(Mutex::new(Vec::new())),
            error_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_before_request_calls(&self) -> Vec<String> {
        self.before_request_calls.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    fn get_after_response_calls(&self) -> Vec<String> {
        self.after_response_calls.lock().unwrap().clone()
    }

    fn get_error_calls(&self) -> Vec<String> {
        self.error_calls.lock().unwrap().clone()
    }
}

impl RequestInterceptor for MockInterceptor {
    fn before_request(&self, request: &reqwest::Request) -> Result<()> {
        let mut calls = self.before_request_calls.lock().unwrap();
        calls.push(format!("{} {}", request.method(), request.url()));
        Ok(())
    }

    fn after_response(&self, response: &reqwest::Response) -> Result<()> {
        let mut calls = self.after_response_calls.lock().unwrap();
        calls.push(format!("{} {}", response.status(), response.url()));
        Ok(())
    }

    fn on_error(&self, error: &Error) {
        let mut calls = self.error_calls.lock().unwrap();
        calls.push(error.to_string());
    }
}

/// Failing interceptor for testing error handling
#[derive(Debug)]
struct FailingInterceptor;

impl RequestInterceptor for FailingInterceptor {
    fn before_request(&self, _request: &reqwest::Request) -> Result<()> {
        Err(Error::Config("Interceptor failure".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_http_client_injection() {
        // Create a custom HTTP client with specific configuration
        let custom_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("custom-user-agent")
            .build()
            .unwrap();

        // Build client with custom HTTP client
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .http_client(custom_client)
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        // Verify the client was created successfully
        // Note: We can't directly inspect the HTTP client, but we can verify
        // that the client was built without errors
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-test-key");
    }

    #[test]
    fn test_logging_interceptor_configuration() {
        let logging_interceptor = LoggingInterceptor::new()
            .with_request_logging()
            .with_response_logging()
            .with_header_logging()
            .with_body_logging()
            .with_error_logging();

        assert!(logging_interceptor.log_requests);
        assert!(logging_interceptor.log_responses);
        assert!(logging_interceptor.log_headers);
        assert!(logging_interceptor.log_body);
        assert!(logging_interceptor.log_errors);
    }

    #[test]
    fn test_logging_interceptor_full_logging() {
        let logging_interceptor = LoggingInterceptor::new().with_full_logging();

        assert!(logging_interceptor.log_requests);
        assert!(logging_interceptor.log_responses);
        assert!(logging_interceptor.log_headers);
        assert!(logging_interceptor.log_body);
        assert!(logging_interceptor.log_errors);
    }

    #[test]
    fn test_request_middleware_configuration() {
        let middleware = RequestMiddleware::new()
            .with_request_logging()
            .with_response_logging()
            .with_header_logging()
            .with_body_logging();

        assert!(middleware.log_requests);
        assert!(middleware.log_responses);
        assert!(middleware.log_headers);
        assert!(middleware.log_body);
        assert!(middleware.interceptors.is_empty());
    }

    #[test]
    fn test_request_middleware_with_interceptor() {
        let mock_interceptor = Arc::new(MockInterceptor::new());
        let middleware = RequestMiddleware::new().with_interceptor(mock_interceptor.clone());

        assert_eq!(middleware.interceptors.len(), 1);
        assert!(!middleware.log_requests);
        assert!(!middleware.log_responses);
    }

    #[test]
    fn test_request_middleware_with_logging_interceptor() {
        let logging_interceptor = LoggingInterceptor::new().with_full_logging();
        let middleware = RequestMiddleware::new().with_logging_interceptor(logging_interceptor);

        assert_eq!(middleware.interceptors.len(), 1);
    }

    #[test]
    fn test_client_builder_with_middleware() {
        let middleware = RequestMiddleware::new()
            .with_request_logging()
            .with_response_logging();

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .middleware(middleware)
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        assert!(client.inner.middleware.log_requests);
        assert!(client.inner.middleware.log_responses);
        assert!(!client.inner.middleware.log_headers);
        assert!(!client.inner.middleware.log_body);
    }

    #[test]
    fn test_client_builder_with_logging() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .with_logging()
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        let middleware = &client.inner.middleware;
        assert!(middleware.log_requests);
        assert!(middleware.log_responses);
        assert!(middleware.log_headers);
        assert!(middleware.log_body);
    }

    #[test]
    fn test_client_builder_with_interceptor() {
        let mock_interceptor = Arc::new(MockInterceptor::new());

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .with_interceptor(mock_interceptor.clone())
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        assert_eq!(client.inner.middleware.interceptors.len(), 1);
    }

    #[test]
    fn test_client_builder_with_logging_interceptor() {
        let logging_interceptor = LoggingInterceptor::new()
            .with_request_logging()
            .with_error_logging();

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .with_logging_interceptor(logging_interceptor)
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        assert_eq!(client.inner.middleware.interceptors.len(), 1);
    }

    #[test]
    fn test_retry_config_customization() {
        let retry_config = RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 1.5,
        };

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .retry_config(retry_config.clone())
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        let client_retry_config = &client.inner.retry_config;
        assert_eq!(client_retry_config.max_retries, 5);
        assert_eq!(
            client_retry_config.initial_delay,
            Duration::from_millis(100)
        );
        assert_eq!(client_retry_config.max_delay, Duration::from_secs(10));
        assert_eq!(client_retry_config.backoff_multiplier, 1.5);
    }

    #[test]
    fn test_retry_config_default() {
        let retry_config = RetryConfig::default();

        assert_eq!(retry_config.max_retries, 3);
        assert_eq!(retry_config.initial_delay, Duration::from_millis(500));
        assert_eq!(retry_config.max_delay, Duration::from_secs(30));
        assert_eq!(retry_config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_multiple_interceptors() {
        let mock_interceptor1 = Arc::new(MockInterceptor::new());
        let mock_interceptor2 = Arc::new(MockInterceptor::new());
        let logging_interceptor = LoggingInterceptor::new().with_request_logging();

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .with_interceptor(mock_interceptor1.clone())
            .with_interceptor(mock_interceptor2.clone())
            .with_logging_interceptor(logging_interceptor)
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        // Should have 3 interceptors: 2 mock + 1 logging
        assert_eq!(client.inner.middleware.interceptors.len(), 3);
    }

    #[test]
    fn test_middleware_clone() {
        let mock_interceptor = Arc::new(MockInterceptor::new());
        let middleware = RequestMiddleware::new()
            .with_request_logging()
            .with_interceptor(mock_interceptor);

        let cloned_middleware = middleware.clone();

        assert_eq!(cloned_middleware.log_requests, middleware.log_requests);
        assert_eq!(cloned_middleware.log_responses, middleware.log_responses);
        assert_eq!(cloned_middleware.log_headers, middleware.log_headers);
        assert_eq!(cloned_middleware.log_body, middleware.log_body);
        assert_eq!(
            cloned_middleware.interceptors.len(),
            middleware.interceptors.len()
        );
    }

    #[test]
    fn test_advanced_configuration_combination() {
        let custom_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap();

        let retry_config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 1.8,
        };

        let logging_interceptor = LoggingInterceptor::new()
            .with_request_logging()
            .with_error_logging();

        let mock_interceptor = Arc::new(MockInterceptor::new());

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://custom.api.com")
            .unwrap()
            .timeout(Duration::from_secs(45))
            .max_retries(4)
            .model(Model::Claude3Haiku20240307)
            .max_tokens(2000)
            .http_client(custom_client)
            .retry_config(retry_config)
            .with_logging_interceptor(logging_interceptor)
            .with_interceptor(mock_interceptor)
            .with_logging()
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();

        // Verify configuration
        let config = &client.inner.config;
        assert_eq!(config.api_key, "sk-ant-api03-test-key");
        assert_eq!(config.base_url.as_str(), "https://custom.api.com/");
        assert_eq!(config.timeout, Duration::from_secs(45));
        assert_eq!(config.max_retries, 4);
        assert_eq!(config.model, Model::Claude3Haiku20240307);
        assert_eq!(config.max_tokens, 2000);

        // Verify retry config (should use the explicitly set retry_config, not max_retries)
        let retry_config = &client.inner.retry_config;
        assert_eq!(retry_config.max_retries, 2);
        assert_eq!(retry_config.initial_delay, Duration::from_millis(200));

        // Verify middleware
        let middleware = &client.inner.middleware;
        assert!(middleware.log_requests);
        assert!(middleware.log_responses);
        assert!(middleware.log_headers);
        assert!(middleware.log_body);

        // Should have 2 interceptors: logging + mock
        assert_eq!(middleware.interceptors.len(), 2);
    }

    #[test]
    fn test_request_interceptor_trait_methods() {
        let mock_interceptor = MockInterceptor::new();

        // Create a mock request
        let request = reqwest::Request::new(
            reqwest::Method::POST,
            "https://api.anthropic.com/v1/messages".parse().unwrap(),
        );

        // Test before_request
        let result = mock_interceptor.before_request(&request);
        assert!(result.is_ok());

        let calls = mock_interceptor.get_before_request_calls();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].contains("POST"));
        assert!(calls[0].contains("api.anthropic.com"));

        // Test on_error (we can't easily create a mock Response without additional dependencies)
        let error = Error::Config("Test error".to_string());
        mock_interceptor.on_error(&error);

        let calls = mock_interceptor.get_error_calls();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].contains("Test error"));
    }

    #[test]
    fn test_failing_interceptor() {
        let failing_interceptor = FailingInterceptor;

        let request = reqwest::Request::new(
            reqwest::Method::POST,
            "https://api.anthropic.com/v1/messages".parse().unwrap(),
        );

        let result = failing_interceptor.before_request(&request);
        assert!(result.is_err());

        if let Err(Error::Config(msg)) = result {
            assert_eq!(msg, "Interceptor failure");
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn test_logging_interceptor_default() {
        let interceptor = LoggingInterceptor::default();

        assert!(!interceptor.log_requests);
        assert!(!interceptor.log_responses);
        assert!(!interceptor.log_headers);
        assert!(!interceptor.log_body);
        assert!(!interceptor.log_errors);
    }

    #[test]
    fn test_logging_interceptor_new() {
        let interceptor = LoggingInterceptor::new();

        assert!(!interceptor.log_requests);
        assert!(!interceptor.log_responses);
        assert!(!interceptor.log_headers);
        assert!(!interceptor.log_body);
        assert!(!interceptor.log_errors);
    }

    #[test]
    fn test_request_middleware_default() {
        let middleware = RequestMiddleware::default();

        assert!(!middleware.log_requests);
        assert!(!middleware.log_responses);
        assert!(!middleware.log_headers);
        assert!(!middleware.log_body);
        assert!(middleware.interceptors.is_empty());
    }

    #[test]
    fn test_request_middleware_new() {
        let middleware = RequestMiddleware::new();

        assert!(!middleware.log_requests);
        assert!(!middleware.log_responses);
        assert!(!middleware.log_headers);
        assert!(!middleware.log_body);
        assert!(middleware.interceptors.is_empty());
    }

    #[test]
    fn test_request_middleware_full_logging() {
        let middleware = RequestMiddleware::new().with_full_logging();

        assert!(middleware.log_requests);
        assert!(middleware.log_responses);
        assert!(middleware.log_headers);
        assert!(middleware.log_body);
    }

    // Note: Integration tests for per-request timeout overrides would require
    // actual HTTP requests, which are better suited for integration tests
    // rather than unit tests. The timeout functionality is tested through
    // the client method signatures and configuration validation.
}
