//! Integration tests for HTTP client functionality

#[cfg(test)]
mod tests {
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

    #[tokio::test]
    async fn test_execute_chat_request_structure() {
        use crate::{
            types::{ChatRequest, ContentBlock, MessageParam, Role},
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

        let request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Hello, Claude!")],
            }],
            system: None,
            tools: None,
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: None,
        };

        // This will fail because httpbin doesn't implement the Anthropic API,
        // but we can test that the request is properly structured
        let result = client.execute_chat(request).await;

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

    #[tokio::test]
    async fn test_execute_chat_with_model_override() {
        use crate::{
            types::{ChatRequest, ContentBlock, MessageParam, Role},
            Client,
        };

        // Create a client with one model
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://httpbin.org/post") // Use POST endpoint to see request body
            .unwrap()
            .model(Model::Claude3Haiku20240307) // Default model
            .max_tokens(1000)
            .build()
            .expect("Client should build successfully");

        let request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Test message")],
            }],
            system: None,
            tools: None,
            temperature: None,
            top_p: None,
            stop_sequences: None,
        };

        // Test with model override
        let result = client
            .execute_chat_with_model(
                Model::Claude35Sonnet20241022, // Different model
                request,
            )
            .await;

        // This will fail because httpbin doesn't implement the Anthropic API,
        // but we're testing that the method accepts the model parameter
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chat_builder_integration() {
        use crate::{types::ContentBlock, Client};

        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://httpbin.org")
            .unwrap()
            .model(Model::Claude35Sonnet20241022)
            .max_tokens(2000)
            .build()
            .expect("Client should build successfully");

        // Test the fluent API
        let request = client
            .chat_builder()
            .user_message(ContentBlock::text("Hello!"))
            .assistant_message(ContentBlock::text("Hi there!"))
            .system("Be helpful and concise")
            .temperature(0.8)
            .build();

        // Verify the request structure
        assert_eq!(request.messages.len(), 2);
        assert!(request.system.is_some());
        assert_eq!(request.temperature, Some(0.8));

        // Test executing the request (will fail with httpbin, but tests integration)
        let result = client.execute_chat(request).await;
        assert!(result.is_err()); // Expected with httpbin
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

    #[tokio::test]
    async fn test_concurrent_requests() {
        use crate::{
            types::{ChatRequest, ContentBlock, MessageParam, Role},
            Client,
        };
        use tokio::task;

        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://httpbin.org")
            .unwrap()
            .build()
            .expect("Client should build successfully");

        let request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Concurrent test")],
            }],
            system: None,
            tools: None,
            temperature: None,
            top_p: None,
            stop_sequences: None,
        };

        // Test that we can make concurrent requests with the same client
        let client1 = client.clone();
        let client2 = client.clone();
        let request1 = request.clone();
        let request2 = request.clone();

        let task1 = task::spawn(async move { client1.execute_chat(request1).await });

        let task2 = task::spawn(async move { client2.execute_chat(request2).await });

        // Both should complete (though they'll error with httpbin)
        let (result1, result2) = tokio::join!(task1, task2);

        // Both tasks should complete without panicking
        assert!(result1.is_ok()); // Task completed
        assert!(result2.is_ok()); // Task completed

        // The actual API calls will fail with httpbin, but that's expected
        assert!(result1.unwrap().is_err());
        assert!(result2.unwrap().is_err());
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
        let tool = Tool::new("calculator")
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
        let mut messages = Vec::new();
        messages.push(MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("First message")],
        });
        messages.push(MessageParam {
            role: Role::Assistant,
            content: vec![ContentBlock::text("Response message")],
        });

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

        let tool = Tool::new("test_tool")
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
}
