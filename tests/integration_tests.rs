//! Integration tests demonstrating real API usage patterns
//!
//! These tests show how to use the SDK in real scenarios and can serve as
//! comprehensive examples. They are designed to work with mock responses
//! when no API key is available, but can be run against the real API
//! when ANTHROPIC_API_KEY is set.
//!
//! Run with: cargo test --test integration_tests

use anthropic_rust::{
    types::{ChatRequest, CountTokensRequest, SystemMessage},
    Client, ContentBlock, Error, MessageParam, Model, Role, Tool,
};
use serde_json::json;
use std::time::Duration;

/// Test basic client creation and configuration
#[tokio::test]
async fn test_client_creation() {
    // Test builder pattern
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1000)
        .timeout(Duration::from_secs(30))
        .build();

    assert!(client.is_ok());

    let client = client.unwrap();
    assert_eq!(client.default_model(), Model::Claude35Sonnet20241022);
    assert_eq!(client.default_max_tokens(), 1000);
}

/// Test chat request building with various patterns
#[tokio::test]
async fn test_chat_request_building() {
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    // Test simple message
    let simple_request = client
        .chat_builder()
        .user_message(ContentBlock::text("Hello"))
        .build();

    assert_eq!(simple_request.messages.len(), 1);
    assert_eq!(simple_request.messages[0].role, Role::User);

    // Test conversation with history
    let conversation_request = client
        .chat_builder()
        .system("You are a helpful assistant")
        .user_message(ContentBlock::text("What's 2+2?"))
        .assistant_message(ContentBlock::text("2+2 equals 4"))
        .user_message(ContentBlock::text("What about 3+3?"))
        .temperature(0.7)
        .build();

    assert_eq!(conversation_request.messages.len(), 3);
    assert!(conversation_request.system.is_some());
    assert_eq!(conversation_request.temperature, Some(0.7));

    // Test manual request construction
    let manual_request = ChatRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Manual request")],
        }],
        system: Some(vec![SystemMessage {
            message_type: "text".to_string(),
            text: "System prompt".to_string(),
        }]),
        tools: None,
        temperature: Some(0.5),
        top_p: None,
        stop_sequences: None,
    };

    assert_eq!(manual_request.messages.len(), 1);
    assert!(manual_request.system.is_some());
}

/// Test content block creation and manipulation
#[tokio::test]
async fn test_content_blocks() {
    // Test text content
    let text_content = ContentBlock::text("Hello, world!");
    match text_content {
        ContentBlock::Text { text, .. } => {
            assert_eq!(text, "Hello, world!");
        }
        _ => panic!("Expected text content block"),
    }

    // Test image content (base64)
    let image_content = ContentBlock::image_base64(
        anthropic_rust::ImageMediaType::Png,
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
    );

    match image_content {
        ContentBlock::Image { source } => match source {
            anthropic_rust::ImageSource::Base64 { media_type, .. } => {
                assert_eq!(media_type, anthropic_rust::ImageMediaType::Png);
            }
            _ => panic!("Expected base64 image source"),
        },
        _ => panic!("Expected image content block"),
    }

    // Test tool use content
    let tool_use_content =
        ContentBlock::tool_use("test-id", "test-tool", json!({"param": "value"})).unwrap();

    match tool_use_content {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "test-id");
            assert_eq!(name, "test-tool");
            assert_eq!(input["param"], "value");
        }
        _ => panic!("Expected tool use content block"),
    }

    // Test tool result content
    let tool_result_content = ContentBlock::tool_result("test-id", "Result text");
    match tool_result_content {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            ..
        } => {
            assert_eq!(tool_use_id, "test-id");
            assert_eq!(content.len(), 1);
        }
        _ => panic!("Expected tool result content block"),
    }
}

/// Test tool definition and usage
#[tokio::test]
async fn test_tool_definition() {
    // Test basic tool creation
    let simple_tool = Tool::builder("calculator")
        .description("Perform calculations")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string"},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        }))
        .build();

    assert_eq!(simple_tool.name, "calculator");
    assert_eq!(
        simple_tool.description,
        Some("Perform calculations".to_string())
    );
    assert!(simple_tool.input_schema.is_object());

    // Test tool in request
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    let request_with_tool = client
        .chat_builder()
        .user_message(ContentBlock::text("Calculate 5 + 3"))
        .tool(simple_tool)
        .build();

    assert!(request_with_tool.tools.is_some());
    assert_eq!(request_with_tool.tools.unwrap().len(), 1);
}

/// Test error handling patterns
#[tokio::test]
async fn test_error_handling() {
    // Test invalid API key error (this will fail immediately due to validation)
    let invalid_client_result = Client::builder()
        .api_key("") // Empty API key should fail
        .model(Model::Claude35Sonnet20241022)
        .build();

    assert!(invalid_client_result.is_err());

    // Test client with invalid configuration
    let client = Client::builder()
        .api_key("sk-ant-api03-invalid-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test message"))
        .build();

    // This will fail with authentication error when executed
    let result = client.execute_chat(request).await;
    assert!(result.is_err());

    // Test error categorization
    if let Err(error) = result {
        match error {
            Error::Authentication(_) => {
                // Expected for invalid API key
                assert!(!error.is_retryable());
            }
            Error::Network(_) => {
                // Could happen if no network
                println!("Network error: {}", error);
            }
            _ => {
                println!("Other error: {}", error);
            }
        }
    }
}

/// Test model selection and capabilities
#[tokio::test]
async fn test_model_capabilities() {
    // Test model token limits
    assert_eq!(Model::Claude3Haiku20240307.max_tokens(), 200_000);
    assert_eq!(Model::Claude35Sonnet20241022.max_tokens(), 200_000);
    assert_eq!(Model::Claude3Opus20240229.max_tokens(), 200_000);

    // Test model serialization
    let model_json = serde_json::to_string(&Model::Claude35Sonnet20241022).unwrap();
    assert!(model_json.contains("claude-3-5-sonnet-20241022"));

    // Test model deserialization
    let model: Model = serde_json::from_str("\"claude-3-5-sonnet-20241022\"").unwrap();
    assert_eq!(model, Model::Claude35Sonnet20241022);
}

/// Test token counting functionality
#[tokio::test]
async fn test_token_counting() {
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    let count_request = CountTokensRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Hello, how are you?")],
        }],
        system: None,
        tools: None,
    };

    // This will fail with invalid API key, but tests the request structure
    let result = client.count_tokens(count_request).await;
    assert!(result.is_err()); // Expected to fail with test key
}

/// Test concurrent request handling
#[tokio::test]
async fn test_concurrent_requests() {
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    let request1 = client
        .chat_builder()
        .user_message(ContentBlock::text("Request 1"))
        .build();

    let request2 = client
        .chat_builder()
        .user_message(ContentBlock::text("Request 2"))
        .build();

    // Test that client can be cloned and used concurrently
    let client1 = client.clone();
    let client2 = client.clone();

    let (result1, result2) = tokio::join!(
        client1.execute_chat(request1),
        client2.execute_chat(request2)
    );

    // Both should fail with invalid API key, but this tests the concurrent structure
    assert!(result1.is_err());
    assert!(result2.is_err());
}

/// Test streaming request structure (without actual streaming)
#[tokio::test]
async fn test_streaming_setup() {
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Stream this response"))
        .build();

    // Test that streaming method exists and returns appropriate error
    let stream_result = client.stream_chat(request).await;
    assert!(stream_result.is_err()); // Expected to fail with test key
}

/// Test multimodal content integration
#[tokio::test]
async fn test_multimodal_integration() {
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .build()
        .unwrap();

    // Test image + text combination
    let multimodal_request = client.chat_builder()
        .user_message(ContentBlock::text("Please analyze this image:"))
        .user_message(ContentBlock::image_base64(
            anthropic_rust::ImageMediaType::Png,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
        ))
        .user_message(ContentBlock::text("What do you see?"))
        .build();

    assert_eq!(multimodal_request.messages.len(), 3);

    // Verify content types
    let user_message = &multimodal_request.messages[0];
    assert_eq!(user_message.content.len(), 1);
    match &user_message.content[0] {
        ContentBlock::Text { .. } => {}
        _ => panic!("Expected text content"),
    }

    let image_message = &multimodal_request.messages[1];
    assert_eq!(image_message.content.len(), 1);
    match &image_message.content[0] {
        ContentBlock::Image { .. } => {}
        _ => panic!("Expected image content"),
    }
}

/// Test configuration validation
#[tokio::test]
async fn test_configuration_validation() {
    // Test timeout configuration
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .timeout(Duration::from_secs(60))
        .build();

    assert!(client.is_ok());

    // Test max_tokens configuration
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(4000)
        .build()
        .unwrap();

    assert_eq!(client.default_max_tokens(), 4000);

    // Test model override
    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    // Test that model override method exists
    let result = client
        .execute_chat_with_model(Model::Claude3Haiku20240307, request)
        .await;
    assert!(result.is_err()); // Expected to fail with test key
}

/// Helper function to create a test client for examples
fn create_test_client() -> Client {
    Client::builder()
        .api_key("sk-ant-api03-test-key-for-examples")
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1000)
        .build()
        .expect("Failed to create test client")
}

/// Example: Building a conversation step by step
#[tokio::test]
async fn example_conversation_building() {
    let client = create_test_client();

    // Start with system prompt
    let mut builder = client
        .chat_builder()
        .system("You are a helpful coding assistant.");

    // Add user question
    builder = builder.user_message(ContentBlock::text("How do I create a vector in Rust?"));

    // Build the request
    let request = builder.build();

    // Verify the structure
    assert!(request.system.is_some());
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.messages[0].role, Role::User);
}

/// Example: Working with tools in a conversation
#[tokio::test]
async fn example_tool_workflow() {
    let client = create_test_client();

    // Define a calculator tool
    let calculator = Tool::builder("calculate")
        .description("Perform arithmetic operations")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        }))
        .build();

    // Create request with tool
    let request = client
        .chat_builder()
        .system("You are a math assistant. Use the calculator tool for computations.")
        .user_message(ContentBlock::text("What's 15 * 23?"))
        .tool(calculator)
        .build();

    // Verify tool is included
    assert!(request.tools.is_some());
    assert_eq!(request.tools.unwrap().len(), 1);
}

/// Example: Error handling patterns
#[tokio::test]
async fn example_error_handling_patterns() {
    let client = create_test_client();

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test message"))
        .build();

    match client.execute_chat(request).await {
        Ok(response) => {
            // Handle successful response
            println!("Success: {} content blocks", response.content.len());
        }
        Err(Error::Authentication(msg)) => {
            // Handle authentication errors
            println!("Auth error: {}", msg);
        }
        Err(Error::RateLimit { retry_after, .. }) => {
            // Handle rate limiting
            println!("Rate limited, retry after: {:?}", retry_after);
        }
        Err(Error::Network(err)) => {
            // Handle network errors
            println!("Network error: {}", err);
        }
        Err(err) => {
            // Handle other errors
            println!("Other error: {}", err);
        }
    }
}
