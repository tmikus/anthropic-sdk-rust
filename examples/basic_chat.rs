//! Basic chat example demonstrating the main Client functionality
//!
//! This example shows how to:
//! - Create a client with configuration
//! - Execute chat requests using execute_chat
//! - Override models with execute_chat_with_model
//! - Handle errors appropriately
//!
//! Note: This example requires a valid ANTHROPIC_API_KEY environment variable.
//! For testing purposes, you can set it to a dummy value to see the request structure.

use anthropic::{
    Client, Model,
    types::{ContentBlock, ChatRequest, MessageParam, Role},
    Error,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see request/response details (optional)
    env_logger::init();

    println!("=== Anthropic Rust SDK - Basic Chat Example ===\n");

    // Example 1: Create client with environment variable
    println!("1. Creating client with default configuration...");
    let client = match Client::new(Model::Claude35Sonnet20241022) {
        Ok(client) => {
            println!("✓ Client created successfully");
            println!("  Default model: {:?}", client.default_model());
            println!("  Default max_tokens: {}", client.default_max_tokens());
            client
        }
        Err(e) => {
            println!("✗ Failed to create client: {}", e);
            println!("  Make sure ANTHROPIC_API_KEY environment variable is set");
            
            // Create client with explicit API key for demonstration
            println!("  Creating client with explicit configuration...");
            Client::builder()
                .api_key("sk-ant-api03-demo-key") // This will fail, but shows the structure
                .model(Model::Claude35Sonnet20241022)
                .max_tokens(1000)
                .build()?
        }
    };

    // Example 2: Simple chat request
    println!("\n2. Creating a simple chat request...");
    let simple_request = client.chat_builder()
        .user_message(ContentBlock::text("Hello, Claude! Can you introduce yourself?"))
        .build();

    println!("✓ Request created with {} message(s)", simple_request.messages.len());

    // Example 3: Execute the chat request
    println!("\n3. Executing chat request...");
    match client.execute_chat(simple_request).await {
        Ok(response) => {
            println!("✓ Chat request successful!");
            println!("  Response ID: {}", response.id);
            println!("  Model used: {:?}", response.model);
            println!("  Stop reason: {:?}", response.stop_reason);
            println!("  Token usage: {} input, {} output", 
                     response.usage.input_tokens, response.usage.output_tokens);
            
            // Print the response content
            println!("  Response content:");
            for (i, content_block) in response.content.iter().enumerate() {
                match content_block {
                    ContentBlock::Text { text, .. } => {
                        println!("    [{}] Text: {}", i + 1, text);
                    }
                    ContentBlock::ToolUse { name, .. } => {
                        println!("    [{}] Tool use: {}", i + 1, name);
                    }
                    _ => {
                        println!("    [{}] Other content type", i + 1);
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Chat request failed: {}", e);
            handle_error(&e);
        }
    }

    // Example 4: Chat with model override
    println!("\n4. Testing model override...");
    let conversation_request = ChatRequest {
        messages: vec![
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("What's the capital of France?")],
            },
        ],
        system: Some(vec![anthropic::types::SystemMessage {
            message_type: "text".to_string(),
            text: "Be concise and direct in your responses.".to_string(),
        }]),
        tools: None,
        temperature: Some(0.3), // Lower temperature for factual questions
        top_p: None,
        stop_sequences: None,
    };

    match client.execute_chat_with_model(Model::Claude3Haiku20240307, conversation_request).await {
        Ok(response) => {
            println!("✓ Model override successful!");
            println!("  Used model: {:?} (overrode default {:?})", 
                     response.model, client.default_model());
            
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                println!("  Response: {}", text);
            }
        }
        Err(e) => {
            println!("✗ Model override request failed: {}", e);
            handle_error(&e);
        }
    }

    // Example 5: Conversation with multiple messages
    println!("\n5. Testing conversation with history...");
    let conversation = client.chat_builder()
        .user_message(ContentBlock::text("I'm learning Rust programming."))
        .assistant_message(ContentBlock::text("That's great! Rust is a powerful systems programming language. What would you like to know about it?"))
        .user_message(ContentBlock::text("What makes Rust's ownership system special?"))
        .temperature(0.7)
        .build();

    match client.execute_chat(conversation).await {
        Ok(response) => {
            println!("✓ Conversation successful!");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                // Print first 200 characters of response
                let preview = if text.len() > 200 {
                    format!("{}...", &text[..200])
                } else {
                    text.clone()
                };
                println!("  Response preview: {}", preview);
            }
        }
        Err(e) => {
            println!("✗ Conversation failed: {}", e);
            handle_error(&e);
        }
    }

    // Example 6: Demonstrate concurrent requests
    println!("\n6. Testing concurrent requests...");
    let client1 = client.clone();
    let client2 = client.clone();

    let request1 = client1.chat_builder()
        .user_message(ContentBlock::text("Count from 1 to 3"))
        .build();

    let request2 = client2.chat_builder()
        .user_message(ContentBlock::text("Name three colors"))
        .build();

    let (result1, result2) = tokio::join!(
        client1.execute_chat(request1),
        client2.execute_chat(request2)
    );

    match (result1, result2) {
        (Ok(_), Ok(_)) => {
            println!("✓ Both concurrent requests succeeded!");
        }
        (Err(e1), Ok(_)) => {
            println!("✗ First request failed: {}", e1);
        }
        (Ok(_), Err(e2)) => {
            println!("✗ Second request failed: {}", e2);
        }
        (Err(e1), Err(e2)) => {
            println!("✗ Both requests failed: {} | {}", e1, e2);
        }
    }

    println!("\n=== Example completed ===");
    Ok(())
}

/// Helper function to provide detailed error information
fn handle_error(error: &Error) {
    println!("  Error details:");
    println!("    Category: {:?}", error.category());
    println!("    Is retryable: {}", error.is_retryable());
    
    if let Some(request_id) = error.request_id() {
        println!("    Request ID: {}", request_id);
    }
    
    if let Some(retry_delay) = error.retry_delay() {
        println!("    Suggested retry delay: {:?}", retry_delay);
    }

    match error {
        Error::Authentication(_) => {
            println!("    💡 Tip: Check your ANTHROPIC_API_KEY environment variable");
        }
        Error::RateLimit { .. } => {
            println!("    💡 Tip: Wait before retrying or reduce request frequency");
        }
        Error::Network(_) => {
            println!("    💡 Tip: Check your internet connection");
        }
        Error::Config(_) => {
            println!("    💡 Tip: Verify your client configuration");
        }
        _ => {}
    }
}