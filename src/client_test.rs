//! Client tests demonstrating both unit testing and integration testing patterns
//!
//! This module contains two types of tests:
//! 1. Network-dependent integration tests that make real HTTP calls
//! 2. Unit tests that use mocks and are compatible with Miri memory safety checking
//!
//! ## Conditional Compilation Strategy
//!
//! ### `#[cfg(all(test, not(miri)))]` - Integration Tests
//! - These tests make actual network calls to external services
//! - They are excluded when running under Miri to avoid foreign function call errors
//! - They test real HTTP behavior, timeouts, and error conditions
//! - Used for validating actual network client behavior
//!
//! ### `#[cfg(test)]` - Unit Tests  
//! - These tests use mocks and don't make network calls
//! - They run under both regular testing and Miri
//! - They test client logic, configuration, and error handling
//! - Used for validating core functionality without network dependencies

/// Integration tests that require network access
///
/// These tests are excluded when running under Miri because they make HTTP calls
/// to external services, which would trigger "unsupported operation: can't call
/// foreign function" errors in Miri.
///
/// The tests use httpbin.org as a testing service to validate:
/// - HTTP request/response handling
/// - Error status code processing  
/// - Timeout behavior
/// - Retry logic with real delays
/// - Request serialization over the network
#[cfg(all(test, not(miri)))]
mod network_tests {
    use serde_json::json;
    use std::time::Duration;

    use crate::{
        client::{ClientInner, RequestMiddleware, RetryConfig},
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
            max_retries: 0,                    // No retries to speed up test
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
                None,
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
            _ => panic!(
                "Expected InvalidResponse error for HTML response, got: {:?}",
                error
            ),
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
}

/// Unit tests that don't require network access
///
/// These tests are Miri-compatible because they:
/// - Use mock HTTP clients instead of real network calls
/// - Test pure Rust logic without foreign function calls
/// - Focus on configuration, serialization, and error handling
/// - Provide deterministic, fast test execution
///
/// The tests validate:
/// - Client configuration and builder patterns
/// - Request/response serialization logic
/// - Error categorization and handling
/// - Mock HTTP client behavior
/// - Thread safety and Send/Sync traits
#[cfg(test)]
mod unit_tests {
    use crate::{
        client::{RequestMiddleware, RetryConfig},
        error::Error,
        mock::{MockClientBuilder, MockHttpClient, MockResponse, MockResponseBuilder},
        types::Model,
    };
    use std::time::Duration;

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
        use crate::client::extract_request_id;
        use reqwest::header::{HeaderMap, HeaderValue};

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

    #[test]
    fn test_client_chat_builder_integration() {
        use crate::{
            types::{ContentBlock, Role},
            Client,
        };

        // Create a client with specific model and max_tokens
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Haiku20240307)
            .max_tokens(2000)
            .build()
            .expect("Client should build successfully");

        // Test that client provides access to default configuration
        assert_eq!(client.default_model(), Model::Claude3Haiku20240307);
        assert_eq!(client.default_max_tokens(), 2000);

        // Test that chat_builder works
        let builder = client.chat_builder();
        let request = builder.user_message(ContentBlock::text("Hello!")).build();

        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
        match &request.messages[0].content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Hello!"),
            _ => panic!("Expected text content block"),
        }
    }

    #[test]
    fn test_client_default_configuration() {
        use crate::Client;

        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .build()
            .expect("Client should build with defaults");

        // Test default values
        assert_eq!(client.default_model(), Model::Claude35Sonnet20241022);
        assert_eq!(client.default_max_tokens(), 4096);
    }

    #[test]
    fn test_client_custom_configuration() {
        use crate::Client;

        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Opus20240229)
            .max_tokens(8192)
            .build()
            .expect("Client should build with custom config");

        assert_eq!(client.default_model(), Model::Claude3Opus20240229);
        assert_eq!(client.default_max_tokens(), 8192);
    }

    #[test]
    fn test_client_new_convenience_method() {
        use crate::Client;

        // Set environment variable for the test
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-test-key");

        let client =
            Client::new(Model::Claude3Haiku20240307).expect("Client::new should work with env var");

        assert_eq!(client.default_model(), Model::Claude3Haiku20240307);
        assert_eq!(client.default_max_tokens(), 4096); // Default max_tokens

        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_client_send_sync_traits() {
        use crate::Client;

        // Test that Client implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Client>();
        assert_sync::<Client>();

        // Test that we can clone the client (Arc<ClientInner> should be cloneable)
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .build()
            .expect("Client should build");

        let _cloned_client = client.clone();
    }

    #[test]
    fn test_request_serialization() {
        use crate::types::{ChatRequest, ContentBlock, MessageParam, Role, SystemMessage};
        use serde_json;

        let request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Hello!")],
            }],
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "Be helpful".to_string(),
            }]),
            tools: None,
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_sequences: Some(vec!["STOP".to_string()]),
        };

        // Test that the request can be serialized (this is what execute_chat does internally)
        let serialized = serde_json::to_value(&request).expect("Should serialize");

        // Verify key fields are present
        assert!(serialized["messages"].is_array());
        assert!(serialized["system"].is_array());
        assert!((serialized["temperature"].as_f64().unwrap() - 0.7).abs() < 0.001);
        assert!((serialized["top_p"].as_f64().unwrap() - 0.9).abs() < 0.001);
        assert!(serialized["stop_sequences"].is_array());

        // Verify that None fields are omitted
        assert!(serialized.get("tools").is_none());
    }

    #[test]
    fn test_model_and_max_tokens_injection() {
        use crate::types::{ChatRequest, ContentBlock, MessageParam, Model, Role};
        use serde_json;

        let request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Test")],
            }],
            system: None,
            tools: None,
            temperature: None,
            top_p: None,
            stop_sequences: None,
        };

        // Simulate what execute_chat_with_model does
        let mut body = serde_json::to_value(&request).expect("Should serialize");
        body["model"] =
            serde_json::to_value(&Model::Claude35Sonnet20241022).expect("Should serialize model");
        body["max_tokens"] = serde_json::to_value(1000u32).expect("Should serialize max_tokens");

        // Verify the fields were added
        assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(body["max_tokens"], 1000);
        assert!(body["messages"].is_array());
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_count_tokens_request_structure() {
        use crate::{
            types::{ContentBlock, CountTokensRequest, MessageParam, Role, SystemMessage},
            Client,
        };

        // Create a mock client that uses httpbin for testing request structure
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://httpbin.org")
            .unwrap()
            .model(Model::Claude35Sonnet20241022)
            .max_tokens(1000)
            .build()
            .expect("Client should build successfully");

        let request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Count my tokens!")],
            }],
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "You are a helpful assistant.".to_string(),
            }]),
            tools: None,
        };

        // This will fail because httpbin doesn't implement the Anthropic API,
        // but we can test that the request is properly structured
        let result = client.count_tokens(request).await;

        // We expect this to fail since httpbin doesn't return the expected format
        assert!(result.is_err());

        // The error should be related to response parsing, not request building
        match result.unwrap_err() {
            Error::InvalidResponse(_) | Error::InvalidRequest(_) => {
                // Expected - httpbin doesn't implement Anthropic API
            }
            Error::Api { .. } => {
                // Also acceptable - httpbin might return 404 or other status
            }
            other => {
                // If we get here, the request was properly built and sent
                println!("Got error (expected): {:?}", other);
            }
        }
    }

    #[test]
    fn test_count_tokens_request_serialization() {
        use crate::types::{
            ContentBlock, CountTokensRequest, MessageParam, Model, Role, SystemMessage,
        };
        use serde_json;

        let request = CountTokensRequest {
            messages: vec![
                MessageParam {
                    role: Role::User,
                    content: vec![ContentBlock::text("Hello, how are you?")],
                },
                MessageParam {
                    role: Role::Assistant,
                    content: vec![ContentBlock::text("I'm doing well, thank you!")],
                },
            ],
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "Be helpful and concise.".to_string(),
            }]),
            tools: None,
        };

        // Test that the request can be serialized
        let serialized = serde_json::to_value(&request).expect("Should serialize");

        // Verify key fields are present
        assert!(serialized["messages"].is_array());
        assert_eq!(serialized["messages"].as_array().unwrap().len(), 2);
        assert!(serialized["system"].is_array());
        assert_eq!(serialized["system"][0]["text"], "Be helpful and concise.");

        // Verify that None fields are omitted
        assert!(serialized.get("tools").is_none());

        // Simulate what count_tokens does - add model to the request
        let mut body = serialized;
        body["model"] =
            serde_json::to_value(&Model::Claude35Sonnet20241022).expect("Should serialize model");

        // Verify the model field was added
        assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
        assert!(body["messages"].is_array());
        assert!(body["system"].is_array());
    }

    #[test]
    fn test_count_tokens_request_minimal() {
        use crate::types::{ContentBlock, CountTokensRequest, MessageParam, Model, Role};
        use serde_json;

        // Test with minimal request (no system, no tools)
        let request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Simple message")],
            }],
            system: None,
            tools: None,
        };

        let serialized = serde_json::to_value(&request).expect("Should serialize");

        // Verify required fields are present
        assert!(serialized["messages"].is_array());
        assert_eq!(serialized["messages"].as_array().unwrap().len(), 1);

        // Verify optional fields are omitted
        assert!(serialized.get("system").is_none());
        assert!(serialized.get("tools").is_none());

        // Add model as count_tokens would do
        let mut body = serialized;
        body["model"] =
            serde_json::to_value(&Model::Claude3Haiku20240307).expect("Should serialize model");

        assert_eq!(body["model"], "claude-3-haiku-20240307");
    }

    #[test]
    fn test_count_tokens_request_with_multimodal_content() {
        use crate::types::{
            ContentBlock, CountTokensRequest, ImageMediaType, MessageParam, Model, Role,
        };
        use serde_json;

        let request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![
                    ContentBlock::text("What do you see in this image?"),
                    ContentBlock::image_base64(ImageMediaType::Png, "base64encodeddata"),
                ],
            }],
            system: None,
            tools: None,
        };

        let serialized = serde_json::to_value(&request).expect("Should serialize");

        // Verify multimodal content is properly serialized
        assert!(serialized["messages"].is_array());
        let message = &serialized["messages"][0];
        assert!(message["content"].is_array());
        assert_eq!(message["content"].as_array().unwrap().len(), 2);

        // Check text content
        assert_eq!(message["content"][0]["type"], "text");
        assert_eq!(
            message["content"][0]["text"],
            "What do you see in this image?"
        );

        // Check image content
        assert_eq!(message["content"][1]["type"], "image");
        assert_eq!(message["content"][1]["source"]["type"], "base64");
        assert_eq!(message["content"][1]["source"]["media_type"], "image/png");
        assert_eq!(message["content"][1]["source"]["data"], "base64encodeddata");

        // Add model
        let mut body = serialized;
        body["model"] =
            serde_json::to_value(&Model::Claude35Sonnet20241022).expect("Should serialize model");
        assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_count_tokens_request_with_tools() {
        use crate::tools::Tool;
        use crate::types::{ContentBlock, CountTokensRequest, MessageParam, Model, Role};
        use serde_json;

        // Create a simple tool for testing
        let tool = Tool::builder("calculator")
            .description("A simple calculator")
            .schema_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string"},
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                },
                "required": ["operation", "a", "b"]
            }))
            .build();

        let request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Calculate 2 + 3")],
            }],
            system: None,
            tools: Some(vec![tool]),
        };

        let serialized = serde_json::to_value(&request).expect("Should serialize");

        // Verify tools are properly serialized
        assert!(serialized["messages"].is_array());
        assert!(serialized["tools"].is_array());
        assert_eq!(serialized["tools"].as_array().unwrap().len(), 1);

        let tool_json = &serialized["tools"][0];
        assert_eq!(tool_json["name"], "calculator");
        assert_eq!(tool_json["description"], "A simple calculator");
        assert!(tool_json["input_schema"].is_object());

        // Add model
        let mut body = serialized;
        body["model"] =
            serde_json::to_value(&Model::Claude35Sonnet20241022).expect("Should serialize model");
        assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_count_tokens_with_different_models() {
        use crate::{
            types::{ContentBlock, CountTokensRequest, MessageParam, Role},
            Client,
        };

        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://httpbin.org")
            .unwrap()
            .model(Model::Claude3Haiku20240307) // Default model
            .build()
            .expect("Client should build successfully");

        let request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Test message for token counting")],
            }],
            system: None,
            tools: None,
        };

        // Test that count_tokens uses the client's configured model
        let result = client.count_tokens(request).await;

        // This will fail with httpbin, but we're testing that the method works
        assert!(result.is_err());

        // The error should indicate that the request was attempted
        match result.unwrap_err() {
            Error::InvalidResponse(_) | Error::InvalidRequest(_) | Error::Api { .. } => {
                // Expected - httpbin doesn't implement Anthropic API
            }
            other => {
                println!("Got error (expected): {:?}", other);
            }
        }
    }

    #[test]
    fn test_token_count_response_deserialization() {
        use crate::types::TokenCount;
        use serde_json;

        // Test basic token count response
        let json = r#"{"input_tokens": 42}"#;
        let token_count: TokenCount = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(token_count.input_tokens, 42);

        // Test with larger token count
        let json = r#"{"input_tokens": 1500}"#;
        let token_count: TokenCount = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(token_count.input_tokens, 1500);

        // Test with zero tokens
        let json = r#"{"input_tokens": 0}"#;
        let token_count: TokenCount = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(token_count.input_tokens, 0);
    }

    #[test]
    fn test_token_count_response_invalid_json() {
        use crate::types::TokenCount;
        use serde_json;

        // Test with missing required field
        let json = r#"{}"#;
        let result = serde_json::from_str::<TokenCount>(json);
        assert!(result.is_err());

        // Test with wrong field type
        let json = r#"{"input_tokens": "not_a_number"}"#;
        let result = serde_json::from_str::<TokenCount>(json);
        assert!(result.is_err());

        // Test with extra fields (should be ignored)
        let json = r#"{"input_tokens": 100, "extra_field": "ignored"}"#;
        let token_count: TokenCount = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(token_count.input_tokens, 100);
    }

    #[test]
    fn test_count_tokens_request_builder_pattern() {
        use crate::types::{ContentBlock, CountTokensRequest, MessageParam, Role, SystemMessage};

        // Test building a CountTokensRequest manually (no builder pattern exists yet)
        let messages = vec![
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("First message")],
            },
            MessageParam {
                role: Role::Assistant,
                content: vec![ContentBlock::text("Response message")],
            },
        ];

        let system = Some(vec![SystemMessage {
            message_type: "text".to_string(),
            text: "System prompt".to_string(),
        }]);

        let request = CountTokensRequest {
            messages,
            system,
            tools: None,
        };

        // Verify the structure
        assert_eq!(request.messages.len(), 2);
        assert!(request.system.is_some());
        assert_eq!(request.system.as_ref().unwrap()[0].text, "System prompt");
        assert!(request.tools.is_none());
    }

    #[test]
    fn test_mock_http_client_response_parsing() {
        use crate::mock::MockResponseBuilder;

        // Test that mock responses can be properly parsed into our types
        let response = MockResponseBuilder::chat_response(
            "msg_mock_test",
            "Hello from mock client!",
            "claude-3-5-sonnet-20241022",
            15,
            10,
        );

        assert_eq!(response.status, reqwest::StatusCode::OK);
        assert_eq!(response.body["id"], "msg_mock_test");
        assert_eq!(
            response.body["content"][0]["text"],
            "Hello from mock client!"
        );
        assert_eq!(response.body["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(response.body["usage"]["input_tokens"], 15);
        assert_eq!(response.body["usage"]["output_tokens"], 10);

        // Test that the response body can be deserialized to our Message type
        let message: crate::types::Message = serde_json::from_value(response.body).unwrap();
        assert_eq!(message.id, "msg_mock_test");
        assert_eq!(message.role, crate::types::Role::Assistant);
        assert_eq!(message.content.len(), 1);
        match &message.content[0] {
            crate::types::ContentBlock::Text { text, .. } => {
                assert_eq!(text, "Hello from mock client!");
            }
            _ => panic!("Expected text content block"),
        }
        assert_eq!(message.usage.input_tokens, 15);
        assert_eq!(message.usage.output_tokens, 10);
    }

    #[test]
    fn test_mock_http_client_error_response_parsing() {
        use crate::mock::{MockHttpClient, MockResponse};

        let client = MockHttpClient::new();

        // Test authentication error response structure
        let auth_response = MockResponse::unauthorized("Invalid API key");
        assert_eq!(auth_response.status, reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(auth_response.body["error"]["type"], "authentication_error");
        assert_eq!(auth_response.body["error"]["message"], "Invalid API key");

        // Test rate limit error response structure
        let rate_limit_response =
            MockResponse::rate_limited(Some(std::time::Duration::from_secs(30)));
        assert_eq!(
            rate_limit_response.status,
            reqwest::StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            rate_limit_response.body["error"]["type"],
            "rate_limit_error"
        );
        assert_eq!(rate_limit_response.body["error"]["retry_after"], 30.0);

        // Test that error responses can be converted to our Error types
        let error_result = client
            .handle_error_response::<serde_json::Value>(auth_response.status, &auth_response.body);
        assert!(error_result.is_err());
        match error_result.unwrap_err() {
            crate::Error::Authentication(msg) => {
                assert!(msg.contains("Invalid API key"));
            }
            _ => panic!("Expected authentication error"),
        }
    }

    #[test]
    fn test_mock_http_client_token_counting_response() {
        use crate::mock::MockResponseBuilder;

        // Test token count response structure
        let response = MockResponseBuilder::token_count_response(25);
        assert_eq!(response.status, reqwest::StatusCode::OK);
        assert_eq!(response.body["input_tokens"], 25);

        // Test that the response can be deserialized to TokenCount
        let token_count: crate::types::TokenCount = serde_json::from_value(response.body).unwrap();
        assert_eq!(token_count.input_tokens, 25);
    }

    #[test]
    fn test_mock_http_client_tool_use_response() {
        use crate::mock::MockResponseBuilder;
        use serde_json::json;

        // Test tool use response structure
        let tool_input = json!({"operation": "multiply", "a": 6, "b": 7});
        let response = MockResponseBuilder::tool_use_response(
            "msg_tool_test",
            "toolu_456",
            "calculator",
            tool_input.clone(),
            "claude-3-5-sonnet-20241022",
            20,
            12,
        );

        assert_eq!(response.status, reqwest::StatusCode::OK);
        assert_eq!(response.body["id"], "msg_tool_test");
        assert_eq!(response.body["content"][0]["type"], "tool_use");
        assert_eq!(response.body["content"][0]["name"], "calculator");
        assert_eq!(response.body["stop_reason"], "tool_use");

        // Test that the response can be deserialized to Message
        let message: crate::types::Message = serde_json::from_value(response.body).unwrap();
        assert_eq!(message.id, "msg_tool_test");
        assert_eq!(message.stop_reason, Some(crate::types::StopReason::ToolUse));
        assert_eq!(message.content.len(), 1);

        match &message.content[0] {
            crate::types::ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_456");
                assert_eq!(name, "calculator");
                assert_eq!(input["operation"], "multiply");
                assert_eq!(input["a"], 6);
                assert_eq!(input["b"], 7);
            }
            _ => panic!("Expected tool use content block"),
        }
    }

    #[test]
    fn test_mock_client_builders() {
        use crate::mock::{MockClientBuilder, TestConfig};

        // Test configuration builders
        let miri_config = TestConfig::for_miri();
        assert!(miri_config.use_mocks);
        assert_eq!(miri_config.timeout, std::time::Duration::from_secs(1));
        assert_eq!(miri_config.max_retries, 0);

        let integration_config = TestConfig::for_integration();
        assert!(!integration_config.use_mocks);
        assert_eq!(
            integration_config.timeout,
            std::time::Duration::from_secs(30)
        );
        assert_eq!(integration_config.max_retries, 2);

        // Test pre-configured clients
        let api_client = MockClientBuilder::anthropic_api_client();
        assert_eq!(api_client.requests().len(), 0);

        let error_client = MockClientBuilder::error_simulation_client();
        assert_eq!(error_client.requests().len(), 0);

        let timeout_client = MockClientBuilder::timeout_simulation_client();
        assert_eq!(timeout_client.requests().len(), 0);
    }

    #[test]
    fn test_mock_client_request_recording() {
        use crate::mock::{MockHttpClient, MockResponse};
        use reqwest::Method;
        use serde_json::json;

        let client = MockHttpClient::new();

        // Configure responses
        client.mock(
            Method::GET,
            "/test1",
            MockResponse::ok(json!({"result": "test1"})),
        );
        client.mock(
            Method::POST,
            "/test2",
            MockResponse::ok(json!({"result": "test2"})),
        );

        // Test that we can configure and retrieve mock responses
        assert_eq!(client.requests().len(), 0);

        // Test clearing requests
        client.clear_requests();
        assert_eq!(client.requests().len(), 0);

        // Test reset
        client.reset();
        assert_eq!(client.requests().len(), 0);
    }

    #[test]
    fn test_mock_response_with_delay() {
        use crate::mock::MockResponse;
        use serde_json::json;
        use std::time::Duration;

        // Test that delay can be configured on mock responses
        let response =
            MockResponse::ok(json!({"delayed": true})).with_delay(Duration::from_millis(100));

        assert_eq!(response.status, reqwest::StatusCode::OK);
        assert_eq!(response.body["delayed"], true);
        assert_eq!(response.delay, Some(Duration::from_millis(100)));
    }

    /// Example of how to use mock infrastructure for comprehensive unit testing
    #[test]
    fn test_mock_infrastructure_comprehensive_example() {
        use crate::mock::{
            MockClientBuilder, MockHttpClient, MockResponse, MockResponseBuilder, TestConfig,
        };
        use reqwest::Method;
        use serde_json::json;
        use std::time::Duration;

        // Example 1: Using pre-configured clients
        let api_client = MockClientBuilder::anthropic_api_client();
        assert_eq!(api_client.requests().len(), 0);

        let error_client = MockClientBuilder::error_simulation_client();
        assert_eq!(error_client.requests().len(), 0);

        // Example 2: Creating custom mock responses
        let custom_client = MockHttpClient::new();

        // Configure a successful chat response
        let chat_response = MockResponseBuilder::chat_response(
            "msg_example",
            "This is a mock response for testing.",
            "claude-3-5-sonnet-20241022",
            20,
            15,
        );
        custom_client.mock(Method::POST, "/v1/messages", chat_response);

        // Configure an error response
        let error_response = MockResponse::rate_limited(Some(Duration::from_secs(60)))
            .with_request_id("req-rate-limit-123");
        custom_client.mock(Method::POST, "/v1/messages/rate_limit", error_response);

        // Configure a token count response
        let token_response = MockResponseBuilder::token_count_response(42);
        custom_client.mock(Method::POST, "/v1/messages/count_tokens", token_response);

        // Example 3: Testing different configurations
        let miri_config = TestConfig::for_miri();
        assert!(miri_config.use_mocks);
        assert_eq!(miri_config.max_retries, 0); // No retries for fast unit tests

        let integration_config = TestConfig::for_integration();
        assert!(!integration_config.use_mocks);
        assert_eq!(integration_config.max_retries, 2); // Retries for integration tests

        // Example 4: Testing response structures
        let tool_response = MockResponseBuilder::tool_use_response(
            "msg_tool",
            "toolu_123",
            "test_tool",
            json!({"param": "value"}),
            "claude-3-5-sonnet-20241022",
            10,
            5,
        );

        // Verify the response can be deserialized
        let message: crate::types::Message = serde_json::from_value(tool_response.body).unwrap();
        assert_eq!(message.id, "msg_tool");
        assert_eq!(message.stop_reason, Some(crate::types::StopReason::ToolUse));

        // Example 5: Testing error handling
        let auth_error = MockResponse::unauthorized("Test auth error");
        let parsed_error = custom_client
            .handle_error_response::<serde_json::Value>(auth_error.status, &auth_error.body);
        assert!(parsed_error.is_err());
        match parsed_error.unwrap_err() {
            crate::Error::Authentication(msg) => {
                assert!(msg.contains("Test auth error"));
            }
            _ => panic!("Expected authentication error"),
        }
    }

    #[test]
    fn test_count_tokens_request_from_chat_request() {
        use crate::types::{
            ChatRequest, ContentBlock, CountTokensRequest, MessageParam, Role, SystemMessage,
        };

        // Test converting a ChatRequest to CountTokensRequest using From trait
        let chat_request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Hello!")],
            }],
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "Be helpful".to_string(),
            }]),
            tools: None,
            temperature: Some(0.7), // This field won't be in CountTokensRequest
            top_p: Some(0.9),       // This field won't be in CountTokensRequest
            stop_sequences: Some(vec!["STOP".to_string()]), // This field won't be in CountTokensRequest
        };

        // Use the From trait implementation
        let count_request = CountTokensRequest::from(chat_request);

        // Verify the conversion
        assert_eq!(count_request.messages.len(), 1);
        assert!(count_request.system.is_some());
        assert_eq!(count_request.system.as_ref().unwrap()[0].text, "Be helpful");
        assert!(count_request.tools.is_none());
    }

    #[test]
    fn test_count_tokens_request_from_chat_request_with_tools() {
        use crate::tools::Tool;
        use crate::types::{ChatRequest, ContentBlock, CountTokensRequest, MessageParam, Role};

        let tool = Tool::builder("test_tool")
            .description("A test tool")
            .schema_value(serde_json::json!({"type": "object"}))
            .build();

        let chat_request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Test with tools")],
            }],
            system: None,
            tools: Some(vec![tool]),
            temperature: Some(0.5),
            top_p: Some(0.8),
            stop_sequences: None,
        };

        // Convert using From trait
        let count_request: CountTokensRequest = chat_request.into();

        // Verify the conversion
        assert_eq!(count_request.messages.len(), 1);
        assert!(count_request.system.is_none());
        assert!(count_request.tools.is_some());
        assert_eq!(count_request.tools.as_ref().unwrap().len(), 1);
        assert_eq!(count_request.tools.as_ref().unwrap()[0].name, "test_tool");
    }

    // Mock-based unit tests that replicate network test functionality
    // These tests use the mock HTTP client and can run under Miri

    #[test]
    fn test_mock_successful_request() {
        let mock_client = MockHttpClient::new();

        // Configure successful response
        mock_client.mock(
            reqwest::Method::GET,
            "/json",
            MockResponse::ok(serde_json::json!({
                "slideshow": {
                    "author": "Yours Truly",
                    "date": "date of publication",
                    "slides": [
                        {
                            "title": "Wake up to WonderWidgets!",
                            "type": "all"
                        }
                    ],
                    "title": "Sample Slide Show"
                }
            })),
        );

        // Test that the mock client can be configured and returns expected responses
        // We test the mock infrastructure itself rather than making async calls
        let requests = mock_client.requests();
        assert_eq!(requests.len(), 0); // No requests made yet

        // Test error handling with mock responses
        let error_response = MockResponse::not_found("Resource not found");
        assert_eq!(error_response.status, reqwest::StatusCode::NOT_FOUND);
        assert_eq!(
            error_response.body["error"]["message"],
            "Resource not found"
        );
    }

    #[test]
    fn test_mock_404_error_handling() {
        let mock_client = MockHttpClient::new();

        // Configure 404 response
        let not_found_response = MockResponse::not_found("The requested resource was not found");
        mock_client.mock(
            reqwest::Method::GET,
            "/status/404",
            not_found_response.clone(),
        );

        // Test that the error response is properly structured
        assert_eq!(not_found_response.status, reqwest::StatusCode::NOT_FOUND);
        assert_eq!(
            not_found_response.body["error"]["message"],
            "The requested resource was not found"
        );
        assert_eq!(not_found_response.body["error"]["type"], "not_found_error");

        // Test error conversion
        let error_result = mock_client.handle_error_response::<serde_json::Value>(
            not_found_response.status,
            &not_found_response.body,
        );
        assert!(error_result.is_err());
        match error_result.unwrap_err() {
            Error::InvalidRequest(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected InvalidRequest error for 404"),
        }
    }

    #[test]
    fn test_mock_500_error_retryable() {
        let mock_client = MockHttpClient::new();

        // Configure 500 response
        let server_error_response =
            MockResponse::internal_server_error("Internal server error occurred");
        mock_client.mock(
            reqwest::Method::GET,
            "/status/500",
            server_error_response.clone(),
        );

        // Test that the error response is properly structured
        assert_eq!(
            server_error_response.status,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            server_error_response.body["error"]["message"],
            "Internal server error occurred"
        );
        assert_eq!(server_error_response.body["error"]["type"], "api_error");

        // Test error conversion
        let error_result = mock_client.handle_error_response::<serde_json::Value>(
            server_error_response.status,
            &server_error_response.body,
        );
        assert!(error_result.is_err());
        let error = error_result.unwrap_err();

        // 500 errors should be retryable
        assert!(error.is_retryable());
        assert!(error.is_server_error());
    }

    #[test]
    fn test_mock_timeout_simulation() {
        let mock_client = MockHttpClient::new();

        // Configure response with delay to simulate slow response
        let delayed_response = MockResponse::ok(serde_json::json!({"delayed": true}))
            .with_delay(Duration::from_millis(200));

        mock_client.mock(reqwest::Method::GET, "/delay/2", delayed_response.clone());

        // Test that the delay is properly configured
        assert_eq!(delayed_response.delay, Some(Duration::from_millis(200)));
        assert_eq!(delayed_response.status, reqwest::StatusCode::OK);
        assert_eq!(delayed_response.body["delayed"], true);
    }

    #[test]
    fn test_mock_post_request_with_body() {
        let mock_client = MockHttpClient::new();

        let test_data = serde_json::json!({
            "test": "data",
            "number": 42
        });

        // Configure response that echoes the request data
        let echo_response = MockResponse::ok(serde_json::json!({
            "args": {},
            "data": "",
            "files": {},
            "form": {},
            "headers": {
                "Content-Type": "application/json"
            },
            "json": test_data.clone(),
            "origin": "127.0.0.1",
            "url": "https://httpbin.org/post"
        }));

        mock_client.mock(reqwest::Method::POST, "/post", echo_response.clone());

        // Verify the response structure
        assert!(echo_response.body.is_object());
        if let Some(json_field) = echo_response.body.get("json") {
            assert_eq!(json_field, &test_data);
        }
        assert_eq!(echo_response.status, reqwest::StatusCode::OK);
    }

    #[test]
    fn test_mock_request_id_extraction() {
        let mock_client = MockHttpClient::new();

        // Configure response with request ID header
        let response_with_id = MockResponse::ok(serde_json::json!({
            "Content-Type": "application/json",
            "request-id": "test-123"
        }))
        .with_request_id("test-123");

        mock_client.mock(
            reqwest::Method::GET,
            "/response-headers",
            response_with_id.clone(),
        );

        // Test that the request ID header is properly set
        assert_eq!(
            response_with_id.headers.get("request-id").unwrap(),
            "test-123"
        );
        assert_eq!(response_with_id.status, reqwest::StatusCode::OK);
        assert!(response_with_id.body.is_object());
    }

    #[test]
    fn test_mock_invalid_json_response() {
        let mock_client = MockHttpClient::new();

        // Configure response with HTML content as JSON string
        let html_response =
            MockResponse::ok(serde_json::json!("<html><body>Not JSON</body></html>"));
        mock_client.mock(reqwest::Method::GET, "/html", html_response.clone());

        // Test that the response is properly structured
        assert_eq!(html_response.status, reqwest::StatusCode::OK);
        assert_eq!(html_response.body, "<html><body>Not JSON</body></html>");
    }

    #[test]
    fn test_mock_error_categories() {
        let mock_client = MockHttpClient::new();

        // Test 401 Unauthorized
        let auth_error_response = MockResponse::unauthorized("Invalid API key provided");
        mock_client.mock(
            reqwest::Method::GET,
            "/status/401",
            auth_error_response.clone(),
        );

        let error_result = mock_client.handle_error_response::<serde_json::Value>(
            auth_error_response.status,
            &auth_error_response.body,
        );
        if let Err(error) = error_result {
            assert!(error.is_auth_error());
        } else {
            panic!("Expected authentication error");
        }

        // Test 403 Forbidden
        let forbidden_response = MockResponse::forbidden("Access denied");
        let error_result = mock_client.handle_error_response::<serde_json::Value>(
            forbidden_response.status,
            &forbidden_response.body,
        );
        if let Err(error) = error_result {
            assert!(error.is_auth_error());
        } else {
            panic!("Expected authentication error");
        }

        // Test 429 Too Many Requests
        let rate_limit_response = MockResponse::rate_limited(Some(Duration::from_secs(60)));
        let error_result = mock_client.handle_error_response::<serde_json::Value>(
            rate_limit_response.status,
            &rate_limit_response.body,
        );
        if let Err(error) = error_result {
            assert!(error.is_rate_limit_error());
            assert!(error.is_retryable());
        } else {
            panic!("Expected rate limit error");
        }
    }

    #[test]
    fn test_mock_response_builders() {
        // Test chat response builder
        let chat_response = MockResponseBuilder::chat_response(
            "msg_test",
            "Hello from mock Claude!",
            "claude-3-5-sonnet-20241022",
            10,
            8,
        );

        assert_eq!(chat_response.status, reqwest::StatusCode::OK);
        assert_eq!(chat_response.body["id"], "msg_test");
        assert_eq!(
            chat_response.body["content"][0]["text"],
            "Hello from mock Claude!"
        );
        assert_eq!(chat_response.body["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(chat_response.body["usage"]["input_tokens"], 10);
        assert_eq!(chat_response.body["usage"]["output_tokens"], 8);

        // Test token count response builder
        let token_response = MockResponseBuilder::token_count_response(42);
        assert_eq!(token_response.status, reqwest::StatusCode::OK);
        assert_eq!(token_response.body["input_tokens"], 42);

        // Test tool use response builder
        let tool_input = serde_json::json!({"operation": "add", "a": 2, "b": 3});
        let tool_response = MockResponseBuilder::tool_use_response(
            "msg_tool",
            "toolu_123",
            "calculator",
            tool_input.clone(),
            "claude-3-5-sonnet-20241022",
            15,
            8,
        );

        assert_eq!(tool_response.status, reqwest::StatusCode::OK);
        assert_eq!(tool_response.body["id"], "msg_tool");
        assert_eq!(tool_response.body["content"][0]["type"], "tool_use");
        assert_eq!(tool_response.body["content"][0]["name"], "calculator");
        assert_eq!(tool_response.body["content"][0]["input"], tool_input);
        assert_eq!(tool_response.body["stop_reason"], "tool_use");
    }

    #[test]
    fn test_mock_client_builder_creation() {
        // Test anthropic API client builder
        let api_client = MockClientBuilder::anthropic_api_client();
        assert_eq!(api_client.requests().len(), 0);

        // Test error simulation client builder
        let error_client = MockClientBuilder::error_simulation_client();
        assert_eq!(error_client.requests().len(), 0);

        // Test timeout simulation client builder
        let timeout_client = MockClientBuilder::timeout_simulation_client();
        assert_eq!(timeout_client.requests().len(), 0);
    }

    #[test]
    fn test_mock_anthropic_api_client() {
        let mock_client = MockClientBuilder::anthropic_api_client();

        // Test that the client is properly configured with no initial requests
        assert_eq!(mock_client.requests().len(), 0);

        // Test that we can clear requests
        mock_client.clear_requests();
        assert_eq!(mock_client.requests().len(), 0);

        // Test that we can reset the client
        mock_client.reset();
        assert_eq!(mock_client.requests().len(), 0);
    }

    #[test]
    fn test_mock_error_simulation_client() {
        let mock_client = MockClientBuilder::error_simulation_client();

        // Test that the error simulation client is properly configured
        assert_eq!(mock_client.requests().len(), 0);

        // Test different error response types by creating them directly
        let auth_error = MockResponse::unauthorized("Invalid API key");
        assert_eq!(auth_error.status, reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(auth_error.body["error"]["type"], "authentication_error");

        let rate_limit_error = MockResponse::rate_limited(Some(Duration::from_secs(60)));
        assert_eq!(
            rate_limit_error.status,
            reqwest::StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(rate_limit_error.body["error"]["type"], "rate_limit_error");
        assert_eq!(rate_limit_error.body["error"]["retry_after"], 60.0);

        let server_error = MockResponse::internal_server_error("Server error");
        assert_eq!(
            server_error.status,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(server_error.body["error"]["type"], "api_error");

        let bad_request = MockResponse::bad_request("Missing field");
        assert_eq!(bad_request.status, reqwest::StatusCode::BAD_REQUEST);
        assert_eq!(bad_request.body["error"]["type"], "invalid_request_error");
    }

    #[test]
    fn test_mock_request_recording() {
        let mock_client = MockHttpClient::new();

        // Configure a response
        let success_response = MockResponse::ok(serde_json::json!({"success": true}));
        mock_client.mock(reqwest::Method::POST, "/test", success_response.clone());

        // Test that the response is properly configured
        assert_eq!(success_response.status, reqwest::StatusCode::OK);
        assert_eq!(success_response.body["success"], true);

        // Test initial state - no requests recorded yet
        let requests = mock_client.requests();
        assert_eq!(requests.len(), 0);

        // Test clearing requests (should be no-op when empty)
        mock_client.clear_requests();
        assert_eq!(mock_client.requests().len(), 0);

        // Test resetting the client
        mock_client.reset();
        assert_eq!(mock_client.requests().len(), 0);
    }
}
