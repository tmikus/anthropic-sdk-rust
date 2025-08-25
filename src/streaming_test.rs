//! Tests for streaming functionality

#[cfg(test)]
mod tests {
    use crate::{
        streaming::{ContentDelta, MessageAccumulator, MessageDelta, PartialMessage, StreamEvent},
        types::{ContentBlock, Model, Role, StopReason, Usage},
    };
    use futures::{stream, StreamExt};

    #[test]
    fn test_stream_event_serialization() {
        // Test MessageStart event
        let message_start = StreamEvent::MessageStart {
            message: PartialMessage {
                id: "msg_123".to_string(),
                role: Role::Assistant,
                content: vec![],
                model: Model::Claude35Sonnet20241022,
                stop_reason: None,
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 0,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                },
            },
        };

        let json = serde_json::to_string(&message_start).unwrap();
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            StreamEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg_123");
                assert_eq!(message.role, Role::Assistant);
                assert_eq!(message.model, Model::Claude35Sonnet20241022);
            }
            _ => panic!("Expected MessageStart event"),
        }
    }

    #[test]
    fn test_content_block_start_event() {
        let event = StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::text("Hello"),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                assert_eq!(index, 0);
                match content_block {
                    ContentBlock::Text { text, .. } => assert_eq!(text, "Hello"),
                    _ => panic!("Expected text content block"),
                }
            }
            _ => panic!("Expected ContentBlockStart event"),
        }
    }

    #[test]
    fn test_content_block_delta_event() {
        let event = StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: " world".to_string(),
            },
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            StreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    ContentDelta::TextDelta { text } => assert_eq!(text, " world"),
                }
            }
            _ => panic!("Expected ContentBlockDelta event"),
        }
    }

    #[test]
    fn test_message_delta_event() {
        let event = StreamEvent::MessageDelta {
            delta: MessageDelta {
                stop_reason: Some(StopReason::EndTurn),
                stop_sequence: None,
                usage: Some(Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            },
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            StreamEvent::MessageDelta { delta } => {
                assert_eq!(delta.stop_reason, Some(StopReason::EndTurn));
                assert!(delta.usage.is_some());
                assert_eq!(delta.usage.unwrap().output_tokens, 5);
            }
            _ => panic!("Expected MessageDelta event"),
        }
    }

    #[test]
    fn test_message_stop_event() {
        let event = StreamEvent::MessageStop;
        let json = serde_json::to_string(&event).unwrap();
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();

        matches!(parsed, StreamEvent::MessageStop);
    }

    #[test]
    fn test_content_block_stop_event() {
        let event = StreamEvent::ContentBlockStop { index: 0 };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            StreamEvent::ContentBlockStop { index } => assert_eq!(index, 0),
            _ => panic!("Expected ContentBlockStop event"),
        }
    }

    #[tokio::test]
    async fn test_message_accumulator_basic_flow() {
        // Create a stream of events that represents a complete message
        let events = vec![
            Ok(StreamEvent::MessageStart {
                message: PartialMessage {
                    id: "msg_123".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    model: Model::Claude35Sonnet20241022,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage {
                        input_tokens: 10,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            }),
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta {
                    text: "Hello".to_string(),
                },
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta {
                    text: " world".to_string(),
                },
            }),
            Ok(StreamEvent::ContentBlockStop { index: 0 }),
            Ok(StreamEvent::MessageDelta {
                delta: MessageDelta {
                    stop_reason: Some(StopReason::EndTurn),
                    stop_sequence: None,
                    usage: Some(Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    }),
                },
            }),
            Ok(StreamEvent::MessageStop),
        ];

        let stream = stream::iter(events);
        let message_stream = crate::streaming::MessageStream::new(Box::pin(stream));
        let accumulator = MessageAccumulator::new(message_stream);

        let final_message = accumulator.accumulate().await.unwrap();

        assert_eq!(final_message.id, "msg_123");
        assert_eq!(final_message.role, Role::Assistant);
        assert_eq!(final_message.model, Model::Claude35Sonnet20241022);
        assert_eq!(final_message.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(final_message.usage.input_tokens, 10);
        assert_eq!(final_message.usage.output_tokens, 5);
        assert_eq!(final_message.content.len(), 1);

        match &final_message.content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected text content block"),
        }
    }

    #[tokio::test]
    async fn test_message_accumulator_multiple_content_blocks() {
        let events = vec![
            Ok(StreamEvent::MessageStart {
                message: PartialMessage {
                    id: "msg_456".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    model: Model::Claude35Sonnet20241022,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage {
                        input_tokens: 15,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            }),
            // First content block
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta {
                    text: "First block".to_string(),
                },
            }),
            Ok(StreamEvent::ContentBlockStop { index: 0 }),
            // Second content block
            Ok(StreamEvent::ContentBlockStart {
                index: 1,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 1,
                delta: ContentDelta::TextDelta {
                    text: "Second block".to_string(),
                },
            }),
            Ok(StreamEvent::ContentBlockStop { index: 1 }),
            Ok(StreamEvent::MessageDelta {
                delta: MessageDelta {
                    stop_reason: Some(StopReason::EndTurn),
                    stop_sequence: None,
                    usage: Some(Usage {
                        input_tokens: 15,
                        output_tokens: 8,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    }),
                },
            }),
            Ok(StreamEvent::MessageStop),
        ];

        let stream = stream::iter(events);
        let message_stream = crate::streaming::MessageStream::new(Box::pin(stream));
        let accumulator = MessageAccumulator::new(message_stream);

        let final_message = accumulator.accumulate().await.unwrap();

        assert_eq!(final_message.content.len(), 2);

        match &final_message.content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "First block"),
            _ => panic!("Expected text content block"),
        }

        match &final_message.content[1] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Second block"),
            _ => panic!("Expected text content block"),
        }
    }

    #[tokio::test]
    async fn test_message_accumulator_error_handling() {
        let events = vec![
            Ok(StreamEvent::MessageStart {
                message: PartialMessage {
                    id: "msg_error".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    model: Model::Claude35Sonnet20241022,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage {
                        input_tokens: 5,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            }),
            Err(crate::Error::Stream("Test error".to_string())),
        ];

        let stream = stream::iter(events);
        let message_stream = crate::streaming::MessageStream::new(Box::pin(stream));
        let accumulator = MessageAccumulator::new(message_stream);

        let result = accumulator.accumulate().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            crate::Error::Stream(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Stream error"),
        }
    }

    #[tokio::test]
    async fn test_message_accumulator_incomplete_stream() {
        // Stream that ends without MessageStop
        let events = vec![
            Ok(StreamEvent::MessageStart {
                message: PartialMessage {
                    id: "msg_incomplete".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    model: Model::Claude35Sonnet20241022,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage {
                        input_tokens: 5,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            }),
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            // Stream ends here without ContentBlockStop or MessageStop
        ];

        let stream = stream::iter(events);
        let message_stream = crate::streaming::MessageStream::new(Box::pin(stream));
        let accumulator = MessageAccumulator::new(message_stream);

        let result = accumulator.accumulate().await;

        // Should still return the message even if incomplete
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.id, "msg_incomplete");
    }

    #[test]
    fn test_partial_message_serialization() {
        let partial_message = PartialMessage {
            id: "msg_partial".to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::text("Partial content")],
            model: Model::Claude35Sonnet20241022,
            stop_reason: Some(StopReason::MaxTokens),
            stop_sequence: Some("STOP".to_string()),
            usage: Usage {
                input_tokens: 20,
                output_tokens: 10,
                cache_creation_input_tokens: Some(5),
                cache_read_input_tokens: Some(3),
            },
        };

        let json = serde_json::to_string(&partial_message).unwrap();
        let parsed: PartialMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "msg_partial");
        assert_eq!(parsed.role, Role::Assistant);
        assert_eq!(parsed.model, Model::Claude35Sonnet20241022);
        assert_eq!(parsed.stop_reason, Some(StopReason::MaxTokens));
        assert_eq!(parsed.stop_sequence, Some("STOP".to_string()));
        assert_eq!(parsed.usage.input_tokens, 20);
        assert_eq!(parsed.usage.output_tokens, 10);
        assert_eq!(parsed.usage.cache_creation_input_tokens, Some(5));
        assert_eq!(parsed.usage.cache_read_input_tokens, Some(3));
    }

    #[test]
    fn test_content_delta_serialization() {
        let text_delta = ContentDelta::TextDelta {
            text: "Delta text".to_string(),
        };

        let json = serde_json::to_string(&text_delta).unwrap();
        let parsed: ContentDelta = serde_json::from_str(&json).unwrap();

        match parsed {
            ContentDelta::TextDelta { text } => assert_eq!(text, "Delta text"),
        }
    }

    #[test]
    fn test_message_delta_serialization() {
        let message_delta = MessageDelta {
            stop_reason: Some(StopReason::ToolUse),
            stop_sequence: Some("END".to_string()),
            usage: Some(Usage {
                input_tokens: 25,
                output_tokens: 15,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            }),
        };

        let json = serde_json::to_string(&message_delta).unwrap();
        let parsed: MessageDelta = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.stop_reason, Some(StopReason::ToolUse));
        assert_eq!(parsed.stop_sequence, Some("END".to_string()));
        assert!(parsed.usage.is_some());
        assert_eq!(parsed.usage.unwrap().input_tokens, 25);
    }

    #[tokio::test]
    async fn test_message_stream_as_stream() {
        let events = vec![
            Ok(StreamEvent::MessageStart {
                message: PartialMessage {
                    id: "msg_stream_test".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    model: Model::Claude35Sonnet20241022,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage {
                        input_tokens: 1,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            }),
            Ok(StreamEvent::MessageStop),
        ];

        let stream = stream::iter(events);
        let mut message_stream = crate::streaming::MessageStream::new(Box::pin(stream));

        // Test that we can iterate over the stream
        let mut event_count = 0;
        while let Some(event_result) = message_stream.next().await {
            assert!(event_result.is_ok());
            event_count += 1;
        }

        assert_eq!(event_count, 2);
    }
}
