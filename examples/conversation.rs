//! Comprehensive conversation example demonstrating various conversation patterns
//!
//! This example shows how to:
//! - Build conversations with message history
//! - Use system prompts effectively
//! - Handle different content types in conversations
//! - Manage conversation state and context
//! - Implement conversation loops
//!
//! Run with: cargo run --example conversation

use anthropic::{
    Client, Model, ContentBlock, Role, MessageParam,
    types::{ChatRequest, SystemMessage},
    Error,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Anthropic Rust SDK - Conversation Examples ===\n");

    // Create client
    let client = match Client::new(Model::Claude35Sonnet20241022) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  ANTHROPIC_API_KEY not found. Using demo configuration...");
            Client::builder()
                .api_key("demo-key")
                .model(Model::Claude35Sonnet20241022)
                .max_tokens(1000)
                .build()?
        }
    };

    // Example 1: Simple back-and-forth conversation
    println!("1. Simple Conversation");
    println!("=====================");
    
    let conversation = client.chat_builder()
        .system("You are a helpful assistant. Be concise but friendly.")
        .user_message(ContentBlock::text("Hi! What's your name?"))
        .build();

    match client.execute_chat(conversation).await {
        Ok(response) => {
            println!("User: Hi! What's your name?");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                println!("Claude: {}", text);
            }
            
            // Continue the conversation
            let follow_up = client.chat_builder()
                .system("You are a helpful assistant. Be concise but friendly.")
                .user_message(ContentBlock::text("Hi! What's your name?"))
                .assistant_message(response.content[0].clone())
                .user_message(ContentBlock::text("Can you help me with some math?"))
                .build();

            if let Ok(response2) = client.execute_chat(follow_up).await {
                println!("User: Can you help me with some math?");
                if let Some(ContentBlock::Text { text, .. }) = response2.content.first() {
                    println!("Claude: {}", text);
                }
            }
        }
        Err(e) => println!("❌ Conversation failed: {}", e),
    }

    // Example 2: Multi-turn conversation with context
    println!("\n2. Multi-turn Conversation with Context");
    println!("======================================");
    
    let mut conversation_history = vec![
        MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("I'm planning a trip to Japan. What should I know?")],
        }
    ];

    // First exchange
    let request = ChatRequest {
        messages: conversation_history.clone(),
        system: Some(vec![SystemMessage {
            message_type: "text".to_string(),
            text: "You are a knowledgeable travel advisor. Provide helpful, practical advice.".to_string(),
        }]),
        tools: None,
        temperature: Some(0.7),
        top_p: None,
        stop_sequences: None,
    };

    match client.execute_chat(request).await {
        Ok(response) => {
            println!("User: I'm planning a trip to Japan. What should I know?");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                println!("Claude: {}", text);
                
                // Add to conversation history
                conversation_history.push(MessageParam {
                    role: Role::Assistant,
                    content: response.content.clone(),
                });
            }
            
            // Follow-up question
            conversation_history.push(MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("What about the best time to visit for cherry blossoms?")],
            });

            let follow_up_request = ChatRequest {
                messages: conversation_history.clone(),
                system: Some(vec![SystemMessage {
                    message_type: "text".to_string(),
                    text: "You are a knowledgeable travel advisor. Provide helpful, practical advice.".to_string(),
                }]),
                tools: None,
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
            };

            if let Ok(response2) = client.execute_chat(follow_up_request).await {
                println!("User: What about the best time to visit for cherry blossoms?");
                if let Some(ContentBlock::Text { text, .. }) = response2.content.first() {
                    println!("Claude: {}", text);
                }
            }
        }
        Err(e) => println!("❌ Multi-turn conversation failed: {}", e),
    }

    // Example 3: Different conversation styles
    println!("\n3. Different Conversation Styles");
    println!("===============================");

    // Formal style
    let formal_request = client.chat_builder()
        .system("You are a professional business consultant. Use formal language and provide structured advice.")
        .user_message(ContentBlock::text("How can I improve my team's productivity?"))
        .temperature(0.3) // Lower temperature for more consistent, formal responses
        .build();

    match client.execute_chat(formal_request).await {
        Ok(response) => {
            println!("📊 Formal Business Style:");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                let preview = if text.len() > 200 { 
                    format!("{}...", &text[..200]) 
                } else { 
                    text.clone() 
                };
                println!("   {}", preview);
            }
        }
        Err(e) => println!("❌ Formal conversation failed: {}", e),
    }

    // Casual style
    let casual_request = client.chat_builder()
        .system("You are a friendly, casual helper. Use informal language and be conversational.")
        .user_message(ContentBlock::text("How can I improve my team's productivity?"))
        .temperature(0.8) // Higher temperature for more creative, varied responses
        .build();

    match client.execute_chat(casual_request).await {
        Ok(response) => {
            println!("\n😊 Casual Friendly Style:");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                let preview = if text.len() > 200 { 
                    format!("{}...", &text[..200]) 
                } else { 
                    text.clone() 
                };
                println!("   {}", preview);
            }
        }
        Err(e) => println!("❌ Casual conversation failed: {}", e),
    }

    // Example 4: Interactive conversation loop (commented out for automated testing)
    println!("\n4. Interactive Conversation Loop");
    println!("===============================");
    println!("💡 This would start an interactive chat session.");
    println!("   Uncomment the code below to try it!");
    
    /*
    // Uncomment this section for interactive mode
    println!("Starting interactive conversation with Claude...");
    println!("Type 'quit' to exit, 'clear' to start fresh conversation\n");
    
    let mut interactive_history: Vec<MessageParam> = Vec::new();
    
    loop {
        print!("You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        if input == "quit" {
            println!("Goodbye!");
            break;
        }
        
        if input == "clear" {
            interactive_history.clear();
            println!("Conversation cleared!");
            continue;
        }
        
        // Add user message to history
        interactive_history.push(MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text(input.to_string())],
        });
        
        // Create request with full history
        let interactive_request = ChatRequest {
            messages: interactive_history.clone(),
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "You are Claude, a helpful AI assistant. Be conversational and engaging.".to_string(),
            }]),
            tools: None,
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: None,
        };
        
        match client.execute_chat(interactive_request).await {
            Ok(response) => {
                if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                    println!("Claude: {}", text);
                    
                    // Add Claude's response to history
                    interactive_history.push(MessageParam {
                        role: Role::Assistant,
                        content: response.content,
                    });
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                // Remove the failed user message from history
                interactive_history.pop();
            }
        }
        
        println!();
    }
    */

    // Example 5: Conversation with different models
    println!("\n5. Model Comparison in Conversation");
    println!("==================================");
    
    let question = "Explain quantum computing in simple terms.";
    println!("Question: {}", question);
    
    let request = client.chat_builder()
        .user_message(ContentBlock::text(question))
        .build();

    // Try with Haiku (fast, concise)
    match client.execute_chat_with_model(Model::Claude3Haiku20240307, request.clone()).await {
        Ok(response) => {
            println!("\n🏃 Haiku Response (fast & concise):");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                let preview = if text.len() > 150 { 
                    format!("{}...", &text[..150]) 
                } else { 
                    text.clone() 
                };
                println!("   {}", preview);
            }
        }
        Err(e) => println!("❌ Haiku response failed: {}", e),
    }

    // Try with Sonnet (balanced)
    match client.execute_chat_with_model(Model::Claude35Sonnet20241022, request.clone()).await {
        Ok(response) => {
            println!("\n🎵 Sonnet Response (balanced):");
            if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                let preview = if text.len() > 150 { 
                    format!("{}...", &text[..150]) 
                } else { 
                    text.clone() 
                };
                println!("   {}", preview);
            }
        }
        Err(e) => println!("❌ Sonnet response failed: {}", e),
    }

    println!("\n=== Conversation Examples Complete ===");
    println!("💡 Try running with a valid ANTHROPIC_API_KEY to see real responses!");
    
    Ok(())
}

/// Helper function to truncate text for display
fn _truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length])
    }
}

/// Helper function to print conversation history
fn _print_conversation_history(history: &[MessageParam]) {
    println!("📜 Conversation History:");
    for (i, message) in history.iter().enumerate() {
        let role_emoji = match message.role {
            Role::User => "👤",
            Role::Assistant => "🤖",
        };
        
        if let Some(ContentBlock::Text { text, .. }) = message.content.first() {
            let preview = if text.len() > 100 { 
                format!("{}...", &text[..100]) 
            } else { 
                text.clone() 
            };
            println!("   {}. {} {:?}: {}", i + 1, role_emoji, message.role, preview);
        }
    }
    println!();
}