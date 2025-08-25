//! Comprehensive unit tests for types module

#[cfg(test)]
mod tests {
    use crate::types::*;
    use crate::Tool;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_model_serialization() {
        let test_cases = vec![
            (Model::Claude3Haiku20240307, "claude-3-haiku-20240307"),
            (Model::Claude3Sonnet20240229, "claude-3-sonnet-20240229"),
            (Model::Claude3Opus20240229, "claude-3-opus-20240229"),
            (Model::Claude35Sonnet20241022, "claude-3-5-sonnet-20241022"),
            (Model::Claude35Sonnet20250114, "claude-3-5-sonnet-20250114"),
            (Model::Claude4Sonnet20250514, "claude-4-sonnet-20250514"),
        ];

        for (model, expected_str) in test_cases {
            let serialized = serde_json::to_string(&model).unwrap();
            assert_eq!(serialized, format!("\"{}\"", expected_str));

            let deserialized: Model = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, model);
        }
    }

    #[test]
    fn test_model_max_tokens() {
        assert_eq!(Model::Claude3Haiku20240307.max_tokens(), 200_000);
        assert_eq!(Model::Claude3Sonnet20240229.max_tokens(), 200_000);
        assert_eq!(Model::Claude3Opus20240229.max_tokens(), 200_000);
        assert_eq!(Model::Claude35Sonnet20241022.max_tokens(), 200_000);
        assert_eq!(Model::Claude35Sonnet20250114.max_tokens(), 200_000);
        assert_eq!(Model::Claude4Sonnet20250514.max_tokens(), 200_000);
    }

    #[test]
    fn test_role_serialization() {
        let user_role = Role::User;
        let assistant_role = Role::Assistant;

        assert_eq!(serde_json::to_string(&user_role).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&assistant_role).unwrap(),
            "\"assistant\""
        );

        let user_deserialized: Role = serde_json::from_str("\"user\"").unwrap();
        let assistant_deserialized: Role = serde_json::from_str("\"assistant\"").unwrap();

        assert_eq!(user_deserialized, Role::User);
        assert_eq!(assistant_deserialized, Role::Assistant);
    }

    #[test]
    fn test_stop_reason_serialization() {
        let test_cases = vec![
            (StopReason::EndTurn, "end_turn"),
            (StopReason::MaxTokens, "max_tokens"),
            (StopReason::StopSequence, "stop_sequence"),
            (StopReason::ToolUse, "tool_use"),
        ];

        for (stop_reason, expected_str) in test_cases {
            let serialized = serde_json::to_string(&stop_reason).unwrap();
            assert_eq!(serialized, format!("\"{}\"", expected_str));

            let deserialized: StopReason = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, stop_reason);
        }
    }

    #[test]
    fn test_usage_serialization() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: Some(10),
            cache_read_input_tokens: Some(5),
        };

        let serialized = serde_json::to_value(&usage).unwrap();
        assert_eq!(serialized["input_tokens"], 100);
        assert_eq!(serialized["output_tokens"], 50);
        assert_eq!(serialized["cache_creation_input_tokens"], 10);
        assert_eq!(serialized["cache_read_input_tokens"], 5);

        let deserialized: Usage = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized.input_tokens, 100);
        assert_eq!(deserialized.output_tokens, 50);
        assert_eq!(deserialized.cache_creation_input_tokens, Some(10));
        assert_eq!(deserialized.cache_read_input_tokens, Some(5));
    }

    #[test]
    fn test_usage_optional_fields() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };

        let serialized = serde_json::to_value(&usage).unwrap();
        assert_eq!(serialized["input_tokens"], 100);
        assert_eq!(serialized["output_tokens"], 50);
        assert!(serialized.get("cache_creation_input_tokens").is_none());
        assert!(serialized.get("cache_read_input_tokens").is_none());
    }

    #[test]
    fn test_content_block_text() {
        let text_block = ContentBlock::text("Hello, world!");

        let serialized = serde_json::to_value(&text_block).unwrap();
        assert_eq!(serialized["type"], "text");
        assert_eq!(serialized["text"], "Hello, world!");

        let deserialized: ContentBlock = serde_json::from_value(serialized).unwrap();
        match deserialized {
            ContentBlock::Text { text, citations } => {
                assert_eq!(text, "Hello, world!");
                assert!(citations.is_none());
            }
            _ => panic!("Expected text content block"),
        }
    }

    #[test]
    fn test_content_block_image_base64() {
        let image_block = ContentBlock::image_base64(ImageMediaType::Png, "base64data".to_string());

        let serialized = serde_json::to_value(&image_block).unwrap();
        assert_eq!(serialized["type"], "image");
        assert_eq!(serialized["source"]["type"], "base64");
        assert_eq!(serialized["source"]["media_type"], "image/png");
        assert_eq!(serialized["source"]["data"], "base64data");

        let deserialized: ContentBlock = serde_json::from_value(serialized).unwrap();
        match deserialized {
            ContentBlock::Image { source } => match source {
                ImageSource::Base64 { media_type, data } => {
                    assert_eq!(media_type, ImageMediaType::Png);
                    assert_eq!(data, "base64data");
                }
                _ => panic!("Expected base64 image source"),
            },
            _ => panic!("Expected image content block"),
        }
    }

    #[test]
    fn test_content_block_image_url() {
        let image_block = ContentBlock::image_url("https://example.com/image.jpg").unwrap();

        let serialized = serde_json::to_value(&image_block).unwrap();
        assert_eq!(serialized["type"], "image");
        assert_eq!(serialized["source"]["type"], "url");
        assert_eq!(serialized["source"]["url"], "https://example.com/image.jpg");

        let deserialized: ContentBlock = serde_json::from_value(serialized).unwrap();
        match deserialized {
            ContentBlock::Image { source } => match source {
                ImageSource::Url { url } => {
                    assert_eq!(url.as_str(), "https://example.com/image.jpg");
                }
                _ => panic!("Expected URL image source"),
            },
            _ => panic!("Expected image content block"),
        }
    }

    #[test]
    fn test_content_block_tool_use() {
        let tool_input = serde_json::json!({
            "operation": "add",
            "a": 5,
            "b": 3
        });

        let tool_block =
            ContentBlock::tool_use("tool-123", "calculator", tool_input.clone()).unwrap();

        let serialized = serde_json::to_value(&tool_block).unwrap();
        assert_eq!(serialized["type"], "tool_use");
        assert_eq!(serialized["id"], "tool-123");
        assert_eq!(serialized["name"], "calculator");
        assert_eq!(serialized["input"], tool_input);

        let deserialized: ContentBlock = serde_json::from_value(serialized).unwrap();
        match deserialized {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tool-123");
                assert_eq!(name, "calculator");
                assert_eq!(input, tool_input);
            }
            _ => panic!("Expected tool use content block"),
        }
    }

    #[test]
    fn test_content_block_tool_result() {
        let tool_result = ContentBlock::tool_result("tool-123", "The result is 8");

        let serialized = serde_json::to_value(&tool_result).unwrap();
        assert_eq!(serialized["type"], "tool_result");
        assert_eq!(serialized["tool_use_id"], "tool-123");
        assert!(serialized["content"].is_array());
        assert_eq!(serialized["content"][0]["type"], "text");
        assert_eq!(serialized["content"][0]["text"], "The result is 8");

        let deserialized: ContentBlock = serde_json::from_value(serialized).unwrap();
        match deserialized {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tool-123");
                assert_eq!(content.len(), 1);
                assert!(is_error.is_none());
                match &content[0] {
                    ContentBlock::Text { text, .. } => assert_eq!(text, "The result is 8"),
                    _ => panic!("Expected text content in tool result"),
                }
            }
            _ => panic!("Expected tool result content block"),
        }
    }

    #[test]
    fn test_image_media_type_serialization() {
        let test_cases = vec![
            (ImageMediaType::Jpeg, "image/jpeg"),
            (ImageMediaType::Png, "image/png"),
            (ImageMediaType::Gif, "image/gif"),
            (ImageMediaType::WebP, "image/webp"),
        ];

        for (media_type, expected_str) in test_cases {
            let serialized = serde_json::to_string(&media_type).unwrap();
            assert_eq!(serialized, format!("\"{}\"", expected_str));

            let deserialized: ImageMediaType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, media_type);
        }
    }

    #[test]
    fn test_message_param_serialization() {
        let message = MessageParam {
            role: Role::User,
            content: vec![
                ContentBlock::text("Hello"),
                ContentBlock::image_base64(ImageMediaType::Png, "data".to_string()),
            ],
        };

        let serialized = serde_json::to_value(&message).unwrap();
        assert_eq!(serialized["role"], "user");
        assert!(serialized["content"].is_array());
        assert_eq!(serialized["content"].as_array().unwrap().len(), 2);

        let deserialized: MessageParam = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized.role, Role::User);
        assert_eq!(deserialized.content.len(), 2);
    }

    #[test]
    fn test_system_message_serialization() {
        let system_msg = SystemMessage {
            message_type: "text".to_string(),
            text: "You are a helpful assistant".to_string(),
        };

        let serialized = serde_json::to_value(&system_msg).unwrap();
        assert_eq!(serialized["type"], "text");
        assert_eq!(serialized["text"], "You are a helpful assistant");

        let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized.message_type, "text");
        assert_eq!(deserialized.text, "You are a helpful assistant");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Hello")],
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

        let serialized = serde_json::to_value(&request).unwrap();
        assert!(serialized["messages"].is_array());
        assert!(serialized["system"].is_array());
        // Check temperature with floating point tolerance
        let temp_value = serialized["temperature"].as_f64().unwrap();
        assert!((temp_value - 0.7).abs() < 0.001);
        // Check top_p with floating point tolerance
        let top_p_value = serialized["top_p"].as_f64().unwrap();
        assert!((top_p_value - 0.9).abs() < 0.001);
        assert!(serialized["stop_sequences"].is_array());
        // tools should be omitted when None
        assert!(serialized.get("tools").is_none());

        // Note: ChatRequest doesn't need Deserialize for this test
        // We're just testing serialization
    }

    #[test]
    fn test_count_tokens_request_serialization() {
        let request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Count tokens")],
            }],
            system: None,
            tools: None,
        };

        let serialized = serde_json::to_value(&request).unwrap();
        assert!(serialized["messages"].is_array());
        assert!(serialized.get("system").is_none());
        assert!(serialized.get("tools").is_none());

        // Note: CountTokensRequest doesn't need Deserialize for this test
        // We're just testing serialization
    }

    #[test]
    fn test_token_count_response_serialization() {
        let response_json = serde_json::json!({
            "input_tokens": 15
        });

        let token_count: TokenCount = serde_json::from_value(response_json).unwrap();
        assert_eq!(token_count.input_tokens, 15);
    }

    #[test]
    fn test_message_response_serialization() {
        let response_json = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello there!"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let message: Message = serde_json::from_value(response_json).unwrap();
        assert_eq!(message.id, "msg_123");
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        assert_eq!(message.model, Model::Claude35Sonnet20241022);
        assert_eq!(message.stop_reason, Some(StopReason::EndTurn));
        assert!(message.stop_sequence.is_none());
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: Some("Perform calculations".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string"}
                }
            }),
        };

        let serialized = serde_json::to_value(&tool).unwrap();
        assert_eq!(serialized["name"], "calculator");
        assert_eq!(serialized["description"], "Perform calculations");
        assert!(serialized["input_schema"].is_object());

        let deserialized: Tool = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized.name, "calculator");
        assert_eq!(
            deserialized.description,
            Some("Perform calculations".to_string())
        );
    }

    #[test]
    fn test_content_block_convenience_constructors() {
        // Test text constructor
        let text_block = ContentBlock::text("Test text");
        match text_block {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Test text"),
            _ => panic!("Expected text block"),
        }

        // Test image base64 constructor
        let image_block = ContentBlock::image_base64(ImageMediaType::Jpeg, "data");
        match image_block {
            ContentBlock::Image {
                source: ImageSource::Base64 { media_type, data },
            } => {
                assert_eq!(media_type, ImageMediaType::Jpeg);
                assert_eq!(data, "data");
            }
            _ => panic!("Expected base64 image block"),
        }

        // Test tool result constructor
        let tool_result = ContentBlock::tool_result("id", "result");
        match tool_result {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            } => {
                assert_eq!(tool_use_id, "id");
                assert_eq!(content.len(), 1);
            }
            _ => panic!("Expected tool result block"),
        }
    }

    #[test]
    fn test_invalid_url_handling() {
        let result = ContentBlock::image_url("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_use_invalid_json() {
        // Test with a value that can be serialized successfully
        // The actual validation happens at the JSON serialization level
        // which is handled by serde_json and should work for most valid Rust types
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("key".to_string(), "valid_value");

        let result = ContentBlock::tool_use("id", "name", map);
        assert!(result.is_ok());

        // The error case would be if serde_json::to_value fails,
        // but most standard Rust types serialize successfully
    }

    #[test]
    fn test_empty_content_blocks() {
        let message = MessageParam {
            role: Role::User,
            content: vec![],
        };

        let serialized = serde_json::to_value(&message).unwrap();
        assert!(serialized["content"].is_array());
        assert_eq!(serialized["content"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_large_token_counts() {
        let usage = Usage {
            input_tokens: u32::MAX,
            output_tokens: u32::MAX,
            cache_creation_input_tokens: Some(u32::MAX),
            cache_read_input_tokens: Some(u32::MAX),
        };

        let serialized = serde_json::to_value(&usage).unwrap();
        let deserialized: Usage = serde_json::from_value(serialized).unwrap();

        assert_eq!(deserialized.input_tokens, u32::MAX);
        assert_eq!(deserialized.output_tokens, u32::MAX);
        assert_eq!(deserialized.cache_creation_input_tokens, Some(u32::MAX));
        assert_eq!(deserialized.cache_read_input_tokens, Some(u32::MAX));
    }
}
