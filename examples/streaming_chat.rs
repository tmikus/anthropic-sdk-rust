//! Comprehensive streaming example demonstrating real-time response processing
//!
//! This example shows how to:
//! - Stream responses from Claude in real-time
//! - Handle different stream event types
//! - Accumulate streaming content
//! - Display progress and statistics
//! - Handle streaming errors gracefully
//!
//! Run with: cargo run --example streaming_chat

use anthropic::{Client, Model, ContentBlock, StreamEvent, MessageAccumulator};
use futures::StreamExt;
use std::io::{self, Write};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Anthropic Rust SDK - Streaming Examples ===\n");

    // Create client
    let client = match Client::new(Model::Claude35Sonnet20241022) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  ANTHROPIC_API_KEY not found. Using demo configuration...");
            Client::builder()
                .api_key("demo-key")
                .model(Model::Claude35Sonnet20241022)
                .max_tokens(1500)
                .build()?
        }
    };

    // Example 1: Basic streaming with real-time display
    println!("1. Basic Streaming Response");
    println!("==========================");
    
    let request = client.chat_builder()
        .system("You are a creative writer. Write engaging content.")
        .user_message(ContentBlock::text("Write a short story about a robot learning to paint."))
        .build();

    println!("User: Write a short story about a robot learning to paint.");
    println!("Claude: ");
    print!("🤖 ");
    io::stdout().flush()?;

    let start_time = Instant::now();
    let mut accumulated_text = String::new();
    let mut token_count = 0;

    match client.stream_chat(request).await {
        Ok(mut stream) => {
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            StreamEvent::MessageStart { message } => {
                                println!("\n📨 Message started (ID: {})", message.id);
                                print!("🤖 ");
                                io::stdout().flush()?;
                            }
                            StreamEvent::ContentBlockStart { index, .. } => {
                                if index > 0 {
                                    println!("\n📝 Content block {} started", index);
                                    print!("🤖 ");
                                    io::stdout().flush()?;
                                }
                            }
                            StreamEvent::ContentBlockDelta { delta, .. } => {
                                if let anthropic::ContentDelta::TextDelta { text } = delta {
                                    print!("{}", text);
                                    accumulated_text.push_str(&text);
                                    token_count += text.split_whitespace().count();
                                    io::stdout().flush()?;
                                }
                            }
                            StreamEvent::ContentBlockStop { index } => {
                                if index == 0 {
                                    println!("\n✅ Content block {} completed", index);
                                }
                            }
                            StreamEvent::MessageDelta { delta } => {
                                if let Some(stop_reason) = delta.stop_reason {
                                    println!("\n🛑 Stop reason: {:?}", stop_reason);
                                }
                                if let Some(usage) = delta.usage {
                                    println!("📊 Token usage: {} input, {} output", 
                                             usage.input_tokens, usage.output_tokens);
                                }
                            }
                            StreamEvent::MessageStop => {
                                let duration = start_time.elapsed();
                                println!("\n✨ Message completed!");
                                println!("⏱️  Duration: {:.2}s", duration.as_secs_f64());
                                println!("📝 Approximate words: {}", token_count);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        println!("\n❌ Stream error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => println!("❌ Failed to start stream: {}", e),
    }

    // Example 2: Using MessageAccumulator
    println!("\n2. Stream Accumulation");
    println!("=====================");
    
    let accumulator_request = client.chat_builder()
        .user_message(ContentBlock::text("Explain quantum computing in simple terms."))
        .build();

    println!("User: Explain quantum computing in simple terms.");
    println!("Claude: [Accumulating response...]");

    match client.stream_chat(accumulator_request).await {
        Ok(stream) => {
            let accumulator = MessageAccumulator::new(stream);
            match accumulator.accumulate().await {
                Ok(final_message) => {
                    println!("\n✅ Accumulation complete!");
                    println!("📨 Message ID: {}", final_message.id);
                    println!("🎯 Model: {:?}", final_message.model);
                    
                    for (i, content) in final_message.content.iter().enumerate() {
                        if let ContentBlock::Text { text, .. } = content {
                            println!("📝 Content block {}: {} characters", i, text.len());
                            // Show first 200 characters
                            let preview = if text.len() > 200 {
                                format!("{}...", &text[..200])
                            } else {
                                text.clone()
                            };
                            println!("   Preview: {}", preview);
                        }
                    }
                    
                    let usage = final_message.usage;
                    println!("📊 Final usage: {} input + {} output = {} total tokens",
                             usage.input_tokens, usage.output_tokens,
                             usage.input_tokens + usage.output_tokens);
                }
                Err(e) => println!("❌ Accumulation failed: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to start accumulation stream: {}", e),
    }

    // Example 3: Streaming with different models
    println!("\n3. Model Comparison Streaming");
    println!("============================");
    
    let question = "What are the benefits of Rust programming language?";
    println!("Question: {}", question);

    let models = vec![
        (Model::Claude3Haiku20240307, "Haiku (Fast)"),
        (Model::Claude35Sonnet20241022, "Sonnet (Balanced)"),
    ];

    for (model, description) in models {
        println!("\n🎯 Streaming with {}:", description);
        print!("🤖 ");
        io::stdout().flush()?;

        let model_request = client.chat_builder()
            .user_message(ContentBlock::text(question))
            .build();

        let start = Instant::now();
        let mut char_count = 0;

        match client.stream_chat_with_model(model, model_request).await {
            Ok(mut stream) => {
                while let Some(event_result) = stream.next().await {
                    match event_result {
                        Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                            if let anthropic::ContentDelta::TextDelta { text } = delta {
                                print!("{}", text);
                                char_count += text.len();
                                io::stdout().flush()?;
                            }
                        }
                        Ok(StreamEvent::MessageStop) => {
                            let duration = start.elapsed();
                            println!("\n⏱️  {} completed in {:.2}s ({} chars)", 
                                     description, duration.as_secs_f64(), char_count);
                            break;
                        }
                        Ok(_) => {} // Ignore other events for this example
                        Err(e) => {
                            println!("\n❌ Error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => println!("❌ Failed to stream with {}: {}", description, e),
        }
    }

    // Example 4: Streaming with progress indicators
    println!("\n4. Streaming with Progress Tracking");
    println!("==================================");
    
    let progress_request = client.chat_builder()
        .system("Provide a detailed explanation with examples.")
        .user_message(ContentBlock::text("Explain the concept of ownership in Rust with examples."))
        .build();

    println!("User: Explain Rust ownership with examples.");
    println!("Progress: ");

    match client.stream_chat(progress_request).await {
        Ok(mut stream) => {
            let mut progress_chars = 0;
            let mut content_blocks = 0;
            let start = Instant::now();

            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            StreamEvent::ContentBlockStart { .. } => {
                                content_blocks += 1;
                                print!("📝");
                                io::stdout().flush()?;
                            }
                            StreamEvent::ContentBlockDelta { delta, .. } => {
                                if let anthropic::ContentDelta::TextDelta { text } = delta {
                                    progress_chars += text.len();
                                    if progress_chars % 50 == 0 {
                                        print!(".");
                                        io::stdout().flush()?;
                                    }
                                }
                            }
                            StreamEvent::MessageStop => {
                                let duration = start.elapsed();
                                println!("\n✅ Complete! {} content blocks, {} characters in {:.2}s",
                                         content_blocks, progress_chars, duration.as_secs_f64());
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        println!("\n❌ Progress tracking error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => println!("❌ Failed to start progress tracking: {}", e),
    }

    // Example 5: Error handling in streaming
    println!("\n5. Streaming Error Handling");
    println!("===========================");
    
    // This example shows how to handle various streaming errors
    let error_request = client.chat_builder()
        .user_message(ContentBlock::text("Test streaming error handling"))
        .build();

    match client.stream_chat(error_request).await {
        Ok(mut stream) => {
            println!("✅ Stream started successfully");
            
            let mut event_count = 0;
            while let Some(event_result) = stream.next().await {
                event_count += 1;
                
                match event_result {
                    Ok(event) => {
                        match event {
                            StreamEvent::MessageStart { .. } => {
                                println!("📨 Message started (event #{})", event_count);
                            }
                            StreamEvent::MessageStop => {
                                println!("✅ Stream completed successfully after {} events", event_count);
                                break;
                            }
                            _ => {
                                // Handle other events silently for this example
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Stream error at event #{}: {}", event_count, e);
                        println!("🔄 In a real application, you might retry or fallback here");
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to start stream: {}", e);
            println!("💡 This is expected with demo API key");
        }
    }

    println!("\n=== Streaming Examples Complete ===");
    println!("💡 Try running with a valid ANTHROPIC_API_KEY to see real streaming!");
    
    Ok(())
}