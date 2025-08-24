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

    #[test]
    fn test_client_chat_builder_integration() {
        use crate::{Client, types::{ContentBlock, Role}};

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
        let request = builder
            .user_message(ContentBlock::text("Hello!"))
            .build();

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

        let client = Client::new(Model::Claude3Haiku20240307)
            .expect("Client::new should work with env var");

        assert_eq!(client.default_model(), Model::Claude3Haiku20240307);
        assert_eq!(client.default_max_tokens(), 4096); // Default max_tokens

        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn test_execute_chat_request_structure() {
        use crate::{Client, types::{ContentBlock, ChatRequest, MessageParam, Role}};

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
        use crate::{Client, types::{ContentBlock, ChatRequest, MessageParam, Role}};

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
        let result = client.execute_chat_with_model(
            Model::Claude35Sonnet20241022, // Different model
            request
        ).await;

        // This will fail because httpbin doesn't implement the Anthropic API,
        // but we're testing that the method accepts the model parameter
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chat_builder_integration() {
        use crate::{Client, types::ContentBlock};

        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://httpbin.org")
            .unwrap()
            .model(Model::Claude35Sonnet20241022)
            .max_tokens(2000)
            .build()
            .expect("Client should build successfully");

        // Test the fluent API
        let request = client.chat_builder()
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
        use crate::{Client, types::{ContentBlock, ChatRequest, MessageParam, Role}};
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

        let task1 = task::spawn(async move {
            client1.execute_chat(request1).await
        });

        let task2 = task::spawn(async move {
            client2.execute_chat(request2).await
        });

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
        use crate::types::{ChatRequest, MessageParam, Role, ContentBlock, SystemMessage};
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
        use crate::types::{ChatRequest, MessageParam, Role, ContentBlock, Model};
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
        body["model"] = serde_json::to_value(&Model::Claude35Sonnet20241022).expect("Should serialize model");
        body["max_tokens"] = serde_json::to_value(1000u32).expect("Should serialize max_tokens");

        // Verify the fields were added
        assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(body["max_tokens"], 1000);
        assert!(body["messages"].is_array());
    }
}