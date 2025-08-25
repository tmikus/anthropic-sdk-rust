//! Example demonstrating token counting functionality
//!
//! This example shows how to count tokens in messages before sending them to the API.
//! Token counting is useful for:
//! - Estimating costs before making API calls
//! - Ensuring messages fit within model token limits
//! - Optimizing prompt length for better performance

use anthropic_rust::{Client, Model, Result};
use anthropic_rust::types::{ContentBlock, CountTokensRequest, MessageParam, Role, SystemMessage};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the client with your API key
    // The API key can be set via the ANTHROPIC_API_KEY environment variable
    let client = Client::new(Model::Claude35Sonnet20241022)?;

    // Example 1: Count tokens in a simple message
    println!("=== Example 1: Simple Message ===");
    let simple_request = CountTokensRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Hello, Claude! How are you today?")],
        }],
        system: None,
        tools: None,
    };

    match client.count_tokens(simple_request).await {
        Ok(token_count) => {
            println!("Simple message token count: {}", token_count.input_tokens);
        }
        Err(e) => {
            println!("Error counting tokens: {}", e);
        }
    }

    // Example 2: Count tokens in a conversation with system message
    println!("\n=== Example 2: Conversation with System Message ===");
    let conversation_request = CountTokensRequest {
        messages: vec![
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("What's the capital of France?")],
            },
            MessageParam {
                role: Role::Assistant,
                content: vec![ContentBlock::text("The capital of France is Paris.")],
            },
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("What's the population of that city?")],
            },
        ],
        system: Some(vec![SystemMessage {
            message_type: "text".to_string(),
            text: "You are a helpful geography assistant. Provide accurate and concise information about world geography.".to_string(),
        }]),
        tools: None,
    };

    match client.count_tokens(conversation_request).await {
        Ok(token_count) => {
            println!("Conversation token count: {}", token_count.input_tokens);
        }
        Err(e) => {
            println!("Error counting tokens: {}", e);
        }
    }

    // Example 3: Count tokens in a multimodal message
    println!("\n=== Example 3: Multimodal Message ===");
    let multimodal_request = CountTokensRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![
                ContentBlock::text("What do you see in this image?"),
                ContentBlock::image_base64(
                    anthropic_rust::types::ImageMediaType::Png,
                    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
                ),
            ],
        }],
        system: None,
        tools: None,
    };

    match client.count_tokens(multimodal_request).await {
        Ok(token_count) => {
            println!("Multimodal message token count: {}", token_count.input_tokens);
        }
        Err(e) => {
            println!("Error counting tokens: {}", e);
        }
    }

    // Example 4: Count tokens with tools
    println!("\n=== Example 4: Message with Tools ===");
    
    // Create a simple calculator tool
    let calculator_tool = anthropic_rust::tools::Tool::new("calculator")
        .description("A simple calculator that can perform basic arithmetic operations")
        .schema_value(serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "The first number"
                },
                "b": {
                    "type": "number", 
                    "description": "The second number"
                }
            },
            "required": ["operation", "a", "b"]
        }))
        .build();

    let tools_request = CountTokensRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Please calculate 15 * 7 for me.")],
        }],
        system: Some(vec![SystemMessage {
            message_type: "text".to_string(),
            text: "You are a helpful assistant with access to a calculator tool. Use the calculator for any arithmetic operations.".to_string(),
        }]),
        tools: Some(vec![calculator_tool]),
    };

    match client.count_tokens(tools_request).await {
        Ok(token_count) => {
            println!("Message with tools token count: {}", token_count.input_tokens);
        }
        Err(e) => {
            println!("Error counting tokens: {}", e);
        }
    }

    // Example 5: Compare token counts across different models
    println!("\n=== Example 5: Token Counting with Different Models ===");
    let test_message = CountTokensRequest {
        messages: vec![MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("This is a test message to compare token counts across different Claude models.")],
        }],
        system: None,
        tools: None,
    };

    let models = vec![
        Model::Claude3Haiku20240307,
        Model::Claude3Sonnet20240229,
        Model::Claude35Sonnet20241022,
    ];

    for model in models {
        let model_client = Client::builder()
            .api_key(std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "your-api-key".to_string()))
            .model(model.clone())
            .build()?;

        match model_client.count_tokens(test_message.clone()).await {
            Ok(token_count) => {
                println!("{:?}: {} tokens", model, token_count.input_tokens);
            }
            Err(e) => {
                println!("{:?}: Error - {}", model, e);
            }
        }
    }

    // Example 6: Convert ChatRequest to CountTokensRequest
    println!("\n=== Example 6: Converting ChatRequest to CountTokensRequest ===");
    
    // Build a chat request using the builder
    let chat_request = client.chat_builder()
        .user_message(ContentBlock::text("What's the weather like today?"))
        .system("You are a helpful weather assistant.")
        .temperature(0.7)
        .build();

    // Convert to CountTokensRequest using the From trait
    let count_request: CountTokensRequest = chat_request.into();

    match client.count_tokens(count_request).await {
        Ok(token_count) => {
            println!("Converted chat request token count: {}", token_count.input_tokens);
        }
        Err(e) => {
            println!("Error counting tokens: {}", e);
        }
    }

    println!("\n=== Token Counting Examples Complete ===");
    println!("Note: Token counting helps you:");
    println!("- Estimate API costs before making requests");
    println!("- Ensure messages fit within model limits");
    println!("- Optimize prompt length for better performance");
    println!("- Debug issues with message size");
    println!("- Convert ChatRequest to CountTokensRequest using .into()");

    Ok(())
}