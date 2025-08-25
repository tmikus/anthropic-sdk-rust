//! Property-based tests for serialization/deserialization

#[cfg(test)]
mod tests {
    use crate::types::*;
    use crate::Tool;
    use proptest::prelude::*;
    use serde_json;

    // Property test strategies for generating test data
    prop_compose! {
        fn arb_usage()(
            input_tokens in 0u32..1_000_000,
            output_tokens in 0u32..1_000_000,
            cache_creation in prop::option::of(0u32..100_000),
            cache_read in prop::option::of(0u32..100_000),
        ) -> Usage {
            Usage {
                input_tokens,
                output_tokens,
                cache_creation_input_tokens: cache_creation,
                cache_read_input_tokens: cache_read,
            }
        }
    }

    prop_compose! {
        fn arb_model()(
            model_idx in 0..6usize
        ) -> Model {
            match model_idx {
                0 => Model::Claude3Haiku20240307,
                1 => Model::Claude3Sonnet20240229,
                2 => Model::Claude3Opus20240229,
                3 => Model::Claude35Sonnet20241022,
                4 => Model::Claude35Sonnet20250114,
                5 => Model::Claude4Sonnet20250514,
                _ => Model::Claude35Sonnet20241022,
            }
        }
    }

    prop_compose! {
        fn arb_role()(
            is_user in any::<bool>()
        ) -> Role {
            if is_user { Role::User } else { Role::Assistant }
        }
    }

    prop_compose! {
        fn arb_stop_reason()(
            reason_idx in 0..4usize
        ) -> StopReason {
            match reason_idx {
                0 => StopReason::EndTurn,
                1 => StopReason::MaxTokens,
                2 => StopReason::StopSequence,
                3 => StopReason::ToolUse,
                _ => StopReason::EndTurn,
            }
        }
    }

    prop_compose! {
        fn arb_image_media_type()(
            media_idx in 0..4usize
        ) -> ImageMediaType {
            match media_idx {
                0 => ImageMediaType::Jpeg,
                1 => ImageMediaType::Png,
                2 => ImageMediaType::Gif,
                3 => ImageMediaType::WebP,
                _ => ImageMediaType::Png,
            }
        }
    }

    prop_compose! {
        fn arb_image_source()(
            media_type in arb_image_media_type(),
            data in "[a-zA-Z0-9+/]{10,100}",
            url in "https://example\\.com/[a-zA-Z0-9._-]+\\.(jpg|png|gif|webp)",
            use_base64 in any::<bool>(),
        ) -> ImageSource {
            if use_base64 {
                ImageSource::Base64 { media_type, data }
            } else {
                ImageSource::Url { url: url.parse().unwrap() }
            }
        }
    }

    prop_compose! {
        fn arb_content_block()(
            text in "[a-zA-Z0-9 .,!?]{1,1000}",
            image_source in arb_image_source(),
            tool_id in "[a-zA-Z0-9-]{5,20}",
            tool_name in "[a-zA-Z_][a-zA-Z0-9_]{2,20}",
            block_type in 0..3usize,
        ) -> ContentBlock {
            match block_type {
                0 => ContentBlock::Text { text, citations: None },
                1 => ContentBlock::Image { source: image_source },
                2 => ContentBlock::ToolUse {
                    id: tool_id,
                    name: tool_name,
                    input: serde_json::json!({"test": "value"}),
                },
                _ => ContentBlock::Text { text, citations: None },
            }
        }
    }

    prop_compose! {
        fn arb_message_param()(
            role in arb_role(),
            content in prop::collection::vec(arb_content_block(), 1..5),
        ) -> MessageParam {
            MessageParam { role, content }
        }
    }

    prop_compose! {
        fn arb_system_message()(
            text in "[a-zA-Z0-9 .,!?]{10,500}",
        ) -> SystemMessage {
            SystemMessage {
                message_type: "text".to_string(),
                text,
            }
        }
    }

    prop_compose! {
        fn arb_tool()(
            name in "[a-zA-Z_][a-zA-Z0-9_]{2,30}",
            description in prop::option::of("[a-zA-Z0-9 .,!?]{10,200}"),
        ) -> Tool {
            Tool {
                name,
                description,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "param": {"type": "string"}
                    }
                }),
            }
        }
    }

    prop_compose! {
        fn arb_chat_request()(
            messages in prop::collection::vec(arb_message_param(), 1..10),
            system in prop::option::of(prop::collection::vec(arb_system_message(), 1..3)),
            tools in prop::option::of(prop::collection::vec(arb_tool(), 1..5)),
            temperature in prop::option::of(0.0f32..2.0f32),
            top_p in prop::option::of(0.0f32..1.0f32),
            stop_sequences in prop::option::of(prop::collection::vec("[A-Z]{2,10}", 1..3)),
        ) -> ChatRequest {
            ChatRequest {
                messages,
                system,
                tools,
                temperature,
                top_p,
                stop_sequences,
            }
        }
    }

    prop_compose! {
        fn arb_message()(
            id in "[a-zA-Z0-9-]{10,30}",
            role in arb_role(),
            content in prop::collection::vec(arb_content_block(), 1..5),
            model in arb_model(),
            stop_reason in prop::option::of(arb_stop_reason()),
            stop_sequence in prop::option::of("[A-Z]{2,10}"),
            usage in arb_usage(),
        ) -> Message {
            Message {
                id,
                role,
                content,
                model,
                stop_reason,
                stop_sequence,
                usage,
            }
        }
    }

    // Property tests
    proptest! {
        #[test]
        fn test_usage_roundtrip(usage in arb_usage()) {
            let json = serde_json::to_value(&usage).unwrap();
            let deserialized: Usage = serde_json::from_value(json).unwrap();
            prop_assert_eq!(usage, deserialized);
        }

        #[test]
        fn test_model_roundtrip(model in arb_model()) {
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(model, deserialized);
        }

        #[test]
        fn test_role_roundtrip(role in arb_role()) {
            let json = serde_json::to_string(&role).unwrap();
            let deserialized: Role = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(role, deserialized);
        }

        #[test]
        fn test_stop_reason_roundtrip(stop_reason in arb_stop_reason()) {
            let json = serde_json::to_string(&stop_reason).unwrap();
            let deserialized: StopReason = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(stop_reason, deserialized);
        }

        #[test]
        fn test_image_media_type_roundtrip(media_type in arb_image_media_type()) {
            let json = serde_json::to_string(&media_type).unwrap();
            let deserialized: ImageMediaType = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(media_type, deserialized);
        }

        #[test]
        fn test_image_source_roundtrip(source in arb_image_source()) {
            let json = serde_json::to_value(&source).unwrap();
            let deserialized: ImageSource = serde_json::from_value(json).unwrap();
            prop_assert_eq!(source, deserialized);
        }

        #[test]
        fn test_content_block_roundtrip(block in arb_content_block()) {
            let json = serde_json::to_value(&block).unwrap();
            let deserialized: ContentBlock = serde_json::from_value(json).unwrap();
            prop_assert_eq!(block, deserialized);
        }

        #[test]
        fn test_message_param_roundtrip(message in arb_message_param()) {
            let json = serde_json::to_value(&message).unwrap();
            let deserialized: MessageParam = serde_json::from_value(json).unwrap();
            prop_assert_eq!(message, deserialized);
        }

        #[test]
        fn test_system_message_roundtrip(system_msg in arb_system_message()) {
            let json = serde_json::to_value(&system_msg).unwrap();
            let deserialized: SystemMessage = serde_json::from_value(json).unwrap();
            prop_assert_eq!(system_msg, deserialized);
        }

        #[test]
        fn test_tool_roundtrip(tool in arb_tool()) {
            let json = serde_json::to_value(&tool).unwrap();
            let deserialized: Tool = serde_json::from_value(json).unwrap();
            prop_assert_eq!(tool, deserialized);
        }

        #[test]
        fn test_chat_request_serialization(request in arb_chat_request()) {
            // Only test serialization since ChatRequest doesn't implement Deserialize
            let json = serde_json::to_value(&request).unwrap();
            prop_assert!(json.is_object());
            prop_assert!(json.get("messages").is_some());
        }

        #[test]
        fn test_message_roundtrip(message in arb_message()) {
            let json = serde_json::to_value(&message).unwrap();
            let deserialized: Message = serde_json::from_value(json).unwrap();
            prop_assert_eq!(message, deserialized);
        }

        #[test]
        fn test_json_stability(request in arb_chat_request()) {
            // Test that serializing twice produces the same result
            let json1 = serde_json::to_string(&request).unwrap();
            let json2 = serde_json::to_string(&request).unwrap();
            prop_assert_eq!(json1, json2);
        }

        #[test]
        fn test_optional_fields_omitted(
            messages in prop::collection::vec(arb_message_param(), 1..3),
        ) {
            let request = ChatRequest {
                messages,
                system: None,
                tools: None,
                temperature: None,
                top_p: None,
                stop_sequences: None,
            };

            let json = serde_json::to_value(&request).unwrap();

            // None fields should be omitted from JSON
            prop_assert!(json.get("system").is_none());
            prop_assert!(json.get("tools").is_none());
            prop_assert!(json.get("temperature").is_none());
            prop_assert!(json.get("top_p").is_none());
            prop_assert!(json.get("stop_sequences").is_none());

            // Required fields should be present
            prop_assert!(json.get("messages").is_some());
        }

        #[test]
        fn test_content_block_type_field(block in arb_content_block()) {
            let json = serde_json::to_value(&block).unwrap();
            let type_field = json.get("type").unwrap().as_str().unwrap();

            match block {
                ContentBlock::Text { .. } => prop_assert_eq!(type_field, "text"),
                ContentBlock::Image { .. } => prop_assert_eq!(type_field, "image"),
                ContentBlock::ToolUse { .. } => prop_assert_eq!(type_field, "tool_use"),
                ContentBlock::ToolResult { .. } => prop_assert_eq!(type_field, "tool_result"),
                ContentBlock::Document { .. } => prop_assert_eq!(type_field, "document"),
            }
        }

        #[test]
        fn test_image_source_type_field(source in arb_image_source()) {
            let json = serde_json::to_value(&source).unwrap();
            let type_field = json.get("type").unwrap().as_str().unwrap();

            match source {
                ImageSource::Base64 { .. } => prop_assert_eq!(type_field, "base64"),
                ImageSource::Url { .. } => prop_assert_eq!(type_field, "url"),
            }
        }

        #[test]
        fn test_large_token_counts(
            input_tokens in 0u32..u32::MAX,
            output_tokens in 0u32..u32::MAX,
        ) {
            let usage = Usage {
                input_tokens,
                output_tokens,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            };

            let json = serde_json::to_value(&usage).unwrap();
            let deserialized: Usage = serde_json::from_value(json).unwrap();

            prop_assert_eq!(usage.input_tokens, deserialized.input_tokens);
            prop_assert_eq!(usage.output_tokens, deserialized.output_tokens);
        }

        #[test]
        fn test_empty_collections(
            messages in prop::collection::vec(arb_message_param(), 1..3),
        ) {
            let request = ChatRequest {
                messages,
                system: Some(vec![]), // Empty system messages
                tools: Some(vec![]),  // Empty tools
                temperature: None,
                top_p: None,
                stop_sequences: Some(vec![]), // Empty stop sequences
            };

            let json = serde_json::to_value(&request).unwrap();

            // Test that empty collections are serialized
            prop_assert!(json.get("system").is_some());
            prop_assert!(json.get("tools").is_some());
            prop_assert!(json.get("stop_sequences").is_some());
        }

        #[test]
        fn test_unicode_text_handling(
            base_text in "[a-zA-Z0-9 ]{10,50}",
            emoji in "[\u{1F600}-\u{1F64F}]{1,5}",
            unicode_text in "[\u{0100}-\u{017F}]{5,20}",
        ) {
            let text = format!("{} {} {}", base_text, emoji, unicode_text);
            let content_block = ContentBlock::text(text.clone());

            let json = serde_json::to_value(&content_block).unwrap();
            let deserialized: ContentBlock = serde_json::from_value(json).unwrap();

            match deserialized {
                ContentBlock::Text { text: deserialized_text, .. } => {
                    prop_assert_eq!(text, deserialized_text);
                }
                _ => prop_assert!(false, "Expected text content block"),
            }
        }
    }

    // Additional focused property tests for edge cases
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_extreme_float_values(
            temperature in -1000.0f32..1000.0f32,
            top_p in -1000.0f32..1000.0f32,
        ) {
            let request = ChatRequest {
                messages: vec![MessageParam {
                    role: Role::User,
                    content: vec![ContentBlock::text("test")],
                }],
                system: None,
                tools: None,
                temperature: Some(temperature),
                top_p: Some(top_p),
                stop_sequences: None,
            };

            // Should be able to serialize any float values
            let json = serde_json::to_value(&request).unwrap();

            // Test that the JSON contains the expected fields
            prop_assert!(json.get("messages").is_some());
            if request.temperature.is_some() {
                prop_assert!(json.get("temperature").is_some());
            }
            if request.top_p.is_some() {
                prop_assert!(json.get("top_p").is_some());
            }
        }

        #[test]
        fn test_very_long_strings(
            text_len in 1000usize..10000,
        ) {
            let long_text = "a".repeat(text_len);
            let content_block = ContentBlock::text(long_text.clone());

            let json = serde_json::to_value(&content_block).unwrap();
            let deserialized: ContentBlock = serde_json::from_value(json).unwrap();

            match deserialized {
                ContentBlock::Text { text: deserialized_text, .. } => {
                    prop_assert_eq!(long_text.len(), deserialized_text.len());
                    prop_assert_eq!(long_text, deserialized_text);
                }
                _ => prop_assert!(false, "Expected text content block"),
            }
        }
    }
}
