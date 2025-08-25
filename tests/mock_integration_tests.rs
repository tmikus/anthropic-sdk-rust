//! Integration tests with mock HTTP responses using wiremock

use anthropic_rust::{
    types::CountTokensRequest, Client, ContentBlock, Error, MessageParam, Model, Role, StopReason,
    Tool,
};
use serde_json::json;
use std::time::Duration;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Helper to create a test client pointing to the mock server
async fn create_mock_client(mock_server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-ant-api03-test-key")
        .base_url(mock_server.uri().as_str())
        .unwrap()
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1000)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap()
}

#[tokio::test]
async fn test_successful_chat_request() {
    let mock_server = MockServer::start().await;

    // Mock successful response
    let response_body = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "Hello! How can I help you today?"
            }
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 8
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-api03-test-key"))
        .and(header("content-type", "application/json"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Hello"))
        .build();

    let response = client.execute_chat(request).await.unwrap();

    assert_eq!(response.id, "msg_123");
    assert_eq!(response.role, Role::Assistant);
    assert_eq!(response.content.len(), 1);
    match &response.content[0] {
        ContentBlock::Text { text, .. } => {
            assert_eq!(text, "Hello! How can I help you today?");
        }
        _ => panic!("Expected text content block"),
    }
    assert_eq!(response.model, Model::Claude35Sonnet20241022);
    assert_eq!(response.stop_reason, Some(StopReason::EndTurn));
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 8);
}

#[tokio::test]
async fn test_chat_request_with_system_and_tools() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "id": "msg_456",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "tool_use",
                "id": "toolu_123",
                "name": "calculator",
                "input": {
                    "operation": "add",
                    "a": 15,
                    "b": 27
                }
            }
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 25,
            "output_tokens": 15
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let calculator_tool = Tool::new("calculator")
        .description("Perform arithmetic operations")
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

    let request = client
        .chat_builder()
        .system("You are a helpful calculator assistant.")
        .user_message(ContentBlock::text("Calculate 15 + 27"))
        .tool(calculator_tool)
        .temperature(0.1)
        .build();

    let response = client.execute_chat(request).await.unwrap();

    assert_eq!(response.id, "msg_456");
    assert_eq!(response.stop_reason, Some(StopReason::ToolUse));
    assert_eq!(response.content.len(), 1);

    match &response.content[0] {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "toolu_123");
            assert_eq!(name, "calculator");
            assert_eq!(input["operation"], "add");
            assert_eq!(input["a"], 15);
            assert_eq!(input["b"], 27);
        }
        _ => panic!("Expected tool use content block"),
    }
}

#[tokio::test]
async fn test_multimodal_request() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "id": "msg_789",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "I can see a small 1x1 pixel transparent image."
            }
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 20,
            "output_tokens": 12
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client.chat_builder()
        .user_message(ContentBlock::text("What do you see in this image?"))
        .user_message(ContentBlock::image_base64(
            anthropic_rust::ImageMediaType::Png,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
        ))
        .build();

    let response = client.execute_chat(request).await.unwrap();

    assert_eq!(response.id, "msg_789");
    match &response.content[0] {
        ContentBlock::Text { text, .. } => {
            assert!(text.contains("1x1 pixel"));
        }
        _ => panic!("Expected text content block"),
    }
}

#[tokio::test]
async fn test_count_tokens_request() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "input_tokens": 8,
        "output_tokens": 0
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages/count_tokens"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = CountTokensRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Count the tokens in this message")],
        }],
        system: None,
        tools: None,
    };

    let response = client.count_tokens(request).await.unwrap();

    assert_eq!(response.input_tokens, 8);
    // TokenCount only has input_tokens field
    assert_eq!(response.input_tokens, 8);
}

#[tokio::test]
async fn test_api_error_handling() {
    let mock_server = MockServer::start().await;

    let error_response = json!({
        "type": "error",
        "error": {
            "type": "invalid_request_error",
            "message": "Invalid request: missing required field 'messages'"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(&error_response)
                .insert_header("request-id", "req-123"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    let result = client.execute_chat(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    // The error could be either API or InvalidRequest depending on parsing
    match error {
        Error::Api {
            status,
            message,
            error_type,
            request_id,
        } => {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            assert!(message.contains("missing required field"));
            assert_eq!(error_type, Some("invalid_request_error".to_string()));
            assert_eq!(request_id, Some("req-123".to_string()));
        }
        Error::InvalidRequest(msg) => {
            // This is also acceptable for a 400 error
            assert!(msg.contains("missing required field") || msg.contains("Invalid request"));
        }
        _ => panic!("Expected API or InvalidRequest error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_authentication_error() {
    let mock_server = MockServer::start().await;

    let error_response = json!({
        "type": "error",
        "error": {
            "type": "authentication_error",
            "message": "Invalid API key"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_response))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    let result = client.execute_chat(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.is_auth_error());
    assert!(!error.is_retryable());
}

#[tokio::test]
async fn test_rate_limit_error() {
    let mock_server = MockServer::start().await;

    let error_response = json!({
        "type": "error",
        "error": {
            "type": "rate_limit_error",
            "message": "Rate limit exceeded",
            "retry_after": 60.5
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(&error_response)
                .insert_header("request-id", "req-rate-limit"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    let result = client.execute_chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::RateLimit {
            retry_after,
            request_id,
        } => {
            assert_eq!(retry_after, Some(Duration::from_secs_f64(60.5)));
            assert_eq!(request_id, Some("req-rate-limit".to_string()));
        }
        _ => panic!("Expected rate limit error"),
    }
}

#[tokio::test]
async fn test_server_error_retryable() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    let result = client.execute_chat(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.is_server_error());
    assert!(error.is_retryable());
}

#[tokio::test]
async fn test_timeout_handling() {
    let mock_server = MockServer::start().await;

    // Mock a slow response that will timeout
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&json!({"id": "msg_slow"}))
                .set_delay(Duration::from_secs(2)), // Longer than client timeout
        )
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .base_url(mock_server.uri().as_str())
        .unwrap()
        .timeout(Duration::from_millis(100)) // Very short timeout
        .build()
        .unwrap();

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    let result = client.execute_chat(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    // Should be either a timeout or network error
    assert!(error.is_retryable());
}

#[tokio::test]
async fn test_model_override() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "id": "msg_override",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "Response from Haiku model"
            }
        ],
        "model": "claude-3-haiku-20240307",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 5,
            "output_tokens": 4
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await; // Default is Claude35Sonnet

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test with different model"))
        .build();

    // Use model override
    let response = client
        .execute_chat_with_model(Model::Claude3Haiku20240307, request)
        .await
        .unwrap();

    assert_eq!(response.model, Model::Claude3Haiku20240307);
    match &response.content[0] {
        ContentBlock::Text { text, .. } => {
            assert_eq!(text, "Response from Haiku model");
        }
        _ => panic!("Expected text content block"),
    }
}

#[tokio::test]
async fn test_conversation_with_history() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "id": "msg_conversation",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "3+3 equals 6."
            }
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 15,
            "output_tokens": 6
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("What's 2+2?"))
        .assistant_message(ContentBlock::text("2+2 equals 4."))
        .user_message(ContentBlock::text("What about 3+3?"))
        .build();

    let response = client.execute_chat(request).await.unwrap();

    assert_eq!(response.id, "msg_conversation");
    match &response.content[0] {
        ContentBlock::Text { text, .. } => {
            assert_eq!(text, "3+3 equals 6.");
        }
        _ => panic!("Expected text content block"),
    }
}

#[tokio::test]
async fn test_invalid_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request = client
        .chat_builder()
        .user_message(ContentBlock::text("Test"))
        .build();

    let result = client.execute_chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidResponse(_) => {
            // Expected for invalid JSON
        }
        _ => panic!("Expected InvalidResponse error"),
    }
}

#[tokio::test]
async fn test_concurrent_requests() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "id": "msg_concurrent",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "Concurrent response"}],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {"input_tokens": 5, "output_tokens": 3}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .expect(2) // Expect exactly 2 requests
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server).await;

    let request1 = client
        .chat_builder()
        .user_message(ContentBlock::text("Request 1"))
        .build();

    let request2 = client
        .chat_builder()
        .user_message(ContentBlock::text("Request 2"))
        .build();

    // Make concurrent requests
    let (result1, result2) =
        tokio::join!(client.execute_chat(request1), client.execute_chat(request2));

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let response1 = result1.unwrap();
    let response2 = result2.unwrap();

    assert_eq!(response1.id, "msg_concurrent");
    assert_eq!(response2.id, "msg_concurrent");
}
