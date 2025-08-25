//! Performance benchmarks for critical code paths

use anthropic_rust::{
    Client, Model, ContentBlock, Role, MessageParam, Tool,
    types::{ChatRequest, SystemMessage, Usage, Message, StopReason},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use serde_json;
use std::time::Duration;

// Benchmark data generation helpers
fn create_simple_message() -> Message {
    Message {
        id: "msg_bench_123".to_string(),
        role: Role::Assistant,
        content: vec![ContentBlock::text("This is a benchmark message for performance testing.")],
        model: Model::Claude35Sonnet20241022,
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 15,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        },
    }
}

fn create_complex_message() -> Message {
    Message {
        id: "msg_complex_456".to_string(),
        role: Role::Assistant,
        content: vec![
            ContentBlock::text("This is a complex message with multiple content blocks."),
            ContentBlock::image_base64(
                anthropic_rust::ImageMediaType::Png,
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
            ),
            ContentBlock::tool_use(
                "tool_123",
                "calculator",
                serde_json::json!({"operation": "add", "a": 5, "b": 3})
            ).unwrap(),
            ContentBlock::tool_result("tool_123", "The result is 8"),
        ],
        model: Model::Claude35Sonnet20241022,
        stop_reason: Some(StopReason::ToolUse),
        stop_sequence: Some("STOP".to_string()),
        usage: Usage {
            input_tokens: 50,
            output_tokens: 25,
            cache_creation_input_tokens: Some(10),
            cache_read_input_tokens: Some(5),
        },
    }
}

fn create_large_message(content_blocks: usize) -> Message {
    let content = (0..content_blocks)
        .map(|i| ContentBlock::text(format!("Content block number {} with some text content.", i)))
        .collect();

    Message {
        id: format!("msg_large_{}", content_blocks),
        role: Role::Assistant,
        content,
        model: Model::Claude35Sonnet20241022,
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: Usage {
            input_tokens: content_blocks as u32 * 10,
            output_tokens: content_blocks as u32 * 5,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        },
    }
}

fn create_chat_request(messages: usize) -> ChatRequest {
    let messages = (0..messages)
        .map(|i| MessageParam {
            role: if i % 2 == 0 { Role::User } else { Role::Assistant },
            content: vec![ContentBlock::text(format!("Message number {}", i))],
        })
        .collect();

    ChatRequest {
        messages,
        system: Some(vec![SystemMessage {
            message_type: "text".to_string(),
            text: "You are a helpful assistant for benchmarking.".to_string(),
        }]),
        tools: Some(vec![
            Tool::new("calculator")
                .description("Perform calculations")
                .schema_value(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    }
                }))
                .build(),
        ]),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_sequences: Some(vec!["STOP".to_string(), "END".to_string()]),
    }
}

// Serialization benchmarks
fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");
    
    let simple_message = create_simple_message();
    let complex_message = create_complex_message();
    
    group.bench_function("simple_message", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&simple_message)).unwrap();
            black_box(json);
        })
    });
    
    group.bench_function("complex_message", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&complex_message)).unwrap();
            black_box(json);
        })
    });

    // Benchmark different message sizes
    for size in [1, 5, 10, 50, 100].iter() {
        let large_message = create_large_message(*size);
        group.bench_with_input(
            BenchmarkId::new("large_message", size),
            size,
            |b, _| {
                b.iter(|| {
                    let json = serde_json::to_string(black_box(&large_message)).unwrap();
                    black_box(json);
                })
            },
        );
    }
    
    group.finish();
}

fn bench_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_deserialization");
    
    let simple_message = create_simple_message();
    let complex_message = create_complex_message();
    
    let simple_json = serde_json::to_string(&simple_message).unwrap();
    let complex_json = serde_json::to_string(&complex_message).unwrap();
    
    group.bench_function("simple_message", |b| {
        b.iter(|| {
            let message: Message = serde_json::from_str(black_box(&simple_json)).unwrap();
            black_box(message);
        })
    });
    
    group.bench_function("complex_message", |b| {
        b.iter(|| {
            let message: Message = serde_json::from_str(black_box(&complex_json)).unwrap();
            black_box(message);
        })
    });

    // Benchmark different message sizes
    for size in [1, 5, 10, 50, 100].iter() {
        let large_message = create_large_message(*size);
        let large_json = serde_json::to_string(&large_message).unwrap();
        group.bench_with_input(
            BenchmarkId::new("large_message", size),
            size,
            |b, _| {
                b.iter(|| {
                    let message: Message = serde_json::from_str(black_box(&large_json)).unwrap();
                    black_box(message);
                })
            },
        );
    }
    
    group.finish();
}

fn bench_chat_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("chat_request_serialization");
    
    // Benchmark different request sizes
    for size in [1, 5, 10, 25, 50].iter() {
        let request = create_chat_request(*size);
        group.bench_with_input(
            BenchmarkId::new("messages", size),
            size,
            |b, _| {
                b.iter(|| {
                    let json = serde_json::to_string(black_box(&request)).unwrap();
                    black_box(json);
                })
            },
        );
    }
    
    group.finish();
}

fn bench_content_block_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_block_creation");
    
    group.bench_function("text_block", |b| {
        b.iter(|| {
            let block = ContentBlock::text(black_box("This is a test message"));
            black_box(block);
        })
    });
    
    group.bench_function("image_base64_block", |b| {
        b.iter(|| {
            let block = ContentBlock::image_base64(
                black_box(anthropic_rust::ImageMediaType::Png),
                black_box("base64encodeddata".to_string())
            );
            black_box(block);
        })
    });
    
    group.bench_function("tool_use_block", |b| {
        b.iter(|| {
            let block = ContentBlock::tool_use(
                black_box("tool_id"),
                black_box("tool_name"),
                black_box(serde_json::json!({"param": "value"}))
            ).unwrap();
            black_box(block);
        })
    });
    
    group.bench_function("tool_result_block", |b| {
        b.iter(|| {
            let block = ContentBlock::tool_result(
                black_box("tool_id"),
                black_box("Result text")
            );
            black_box(block);
        })
    });
    
    group.finish();
}

fn bench_client_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_builder");
    
    group.bench_function("basic_client", |b| {
        b.iter(|| {
            let client = Client::builder()
                .api_key(black_box("sk-ant-api03-test-key"))
                .model(black_box(Model::Claude35Sonnet20241022))
                .build()
                .unwrap();
            black_box(client);
        })
    });
    
    group.bench_function("full_config_client", |b| {
        b.iter(|| {
            let client = Client::builder()
                .api_key(black_box("sk-ant-api03-test-key"))
                .model(black_box(Model::Claude35Sonnet20241022))
                .max_tokens(black_box(4000))
                .timeout(black_box(Duration::from_secs(30)))
                .max_retries(black_box(3))
                .base_url(black_box("https://api.anthropic.com"))
                .unwrap()
                .build()
                .unwrap();
            black_box(client);
        })
    });
    
    group.finish();
}

fn bench_chat_request_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("chat_request_builder");
    
    let client = Client::builder()
        .api_key("sk-ant-api03-test-key")
        .build()
        .unwrap();
    
    group.bench_function("simple_request", |b| {
        b.iter(|| {
            let request = client.chat_builder()
                .user_message(black_box(ContentBlock::text("Hello")))
                .build();
            black_box(request);
        })
    });
    
    group.bench_function("complex_request", |b| {
        b.iter(|| {
            let request = client.chat_builder()
                .system(black_box("You are helpful"))
                .user_message(black_box(ContentBlock::text("Hello")))
                .assistant_message(black_box(ContentBlock::text("Hi there!")))
                .user_message(black_box(ContentBlock::text("How are you?")))
                .temperature(black_box(0.7))
                .build();
            black_box(request);
        })
    });
    
    // Benchmark building requests with many messages
    for size in [5, 10, 25, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("many_messages", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut builder = client.chat_builder();
                    for i in 0..size {
                        builder = builder.user_message(
                            black_box(ContentBlock::text(format!("Message {}", i)))
                        );
                    }
                    let request = builder.build();
                    black_box(request);
                })
            },
        );
    }
    
    group.finish();
}

fn bench_tool_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_creation");
    
    group.bench_function("simple_tool", |b| {
        b.iter(|| {
            let tool = Tool::new(black_box("calculator"))
                .build();
            black_box(tool);
        })
    });
    
    group.bench_function("complex_tool", |b| {
        b.iter(|| {
            let tool = Tool::new(black_box("calculator"))
                .description(black_box("Perform arithmetic operations"))
                .schema_value(black_box(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["operation", "a", "b"]
                })))
                .build();
            black_box(tool);
        })
    });
    
    group.finish();
}

fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    
    group.bench_function("error_creation", |b| {
        b.iter(|| {
            let error = anthropic_rust::Error::Api {
                status: black_box(reqwest::StatusCode::BAD_REQUEST),
                message: black_box("Test error".to_string()),
                error_type: black_box(Some("test_error".to_string())),
                request_id: black_box(Some("req-123".to_string())),
            };
            black_box(error);
        })
    });
    
    group.bench_function("error_categorization", |b| {
        let error = anthropic_rust::Error::Api {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            message: "Server error".to_string(),
            error_type: None,
            request_id: None,
        };
        
        b.iter(|| {
            let is_retryable = black_box(&error).is_retryable();
            let is_server_error = black_box(&error).is_server_error();
            let is_auth_error = black_box(&error).is_auth_error();
            black_box((is_retryable, is_server_error, is_auth_error));
        })
    });
    
    group.finish();
}

fn bench_model_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_operations");
    
    let models = [
        Model::Claude3Haiku20240307,
        Model::Claude3Sonnet20240229,
        Model::Claude3Opus20240229,
        Model::Claude35Sonnet20241022,
        Model::Claude35Sonnet20250114,
        Model::Claude4Sonnet20250514,
    ];
    
    group.bench_function("max_tokens_lookup", |b| {
        b.iter(|| {
            for model in &models {
                let max_tokens = black_box(model).max_tokens();
                black_box(max_tokens);
            }
        })
    });
    
    group.bench_function("model_serialization", |b| {
        b.iter(|| {
            for model in &models {
                let json = serde_json::to_string(black_box(model)).unwrap();
                black_box(json);
            }
        })
    });
    
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    // Benchmark memory allocation patterns
    group.bench_function("large_message_creation", |b| {
        b.iter(|| {
            let message = create_large_message(black_box(100));
            black_box(message);
        })
    });
    
    group.bench_function("message_cloning", |b| {
        let message = create_complex_message();
        b.iter(|| {
            let cloned = black_box(&message).clone();
            black_box(cloned);
        })
    });
    
    group.bench_function("client_cloning", |b| {
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .build()
            .unwrap();
        
        b.iter(|| {
            let cloned = black_box(&client).clone();
            black_box(cloned);
        })
    });
    
    group.finish();
}

// Benchmark streaming-related operations
fn bench_streaming_operations(c: &mut Criterion) {
    use anthropic_rust::streaming::{StreamEvent, ContentDelta, MessageDelta, PartialMessage};
    
    let mut group = c.benchmark_group("streaming_operations");
    
    let stream_events = vec![
        StreamEvent::MessageStart {
            message: PartialMessage {
                id: "msg_stream".to_string(),
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
        },
        StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::text(""),
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: "Hello".to_string(),
            },
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: " world".to_string(),
            },
        },
        StreamEvent::ContentBlockStop { index: 0 },
        StreamEvent::MessageDelta {
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
        },
        StreamEvent::MessageStop,
    ];
    
    group.bench_function("stream_event_serialization", |b| {
        b.iter(|| {
            for event in &stream_events {
                let json = serde_json::to_string(black_box(event)).unwrap();
                black_box(json);
            }
        })
    });
    
    let stream_events_json: Vec<String> = stream_events
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    
    group.bench_function("stream_event_deserialization", |b| {
        b.iter(|| {
            for json in &stream_events_json {
                let event: StreamEvent = serde_json::from_str(black_box(json)).unwrap();
                black_box(event);
            }
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_message_serialization,
    bench_message_deserialization,
    bench_chat_request_serialization,
    bench_content_block_creation,
    bench_client_builder,
    bench_chat_request_builder,
    bench_tool_creation,
    bench_error_handling,
    bench_model_operations,
    bench_memory_usage,
    bench_streaming_operations
);

criterion_main!(benches);