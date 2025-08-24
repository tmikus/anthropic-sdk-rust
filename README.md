# Anthropic Rust SDK

[![Crates.io](https://img.shields.io/crates/v/anthropic.svg)](https://crates.io/crates/anthropic)
[![Documentation](https://docs.rs/anthropic/badge.svg)](https://docs.rs/anthropic)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A modern, idiomatic Rust SDK for the [Anthropic API](https://docs.anthropic.com/), providing type-safe, async-first access to Claude models.

## Features

- 🦀 **Idiomatic Rust**: Built with Rust best practices, leveraging the type system for safety
- ⚡ **Async/Await**: Full async support with tokio compatibility
- 🔄 **Streaming**: Real-time streaming of Claude's responses
- 🛠️ **Tool Calling**: Complete support for Claude's tool use capabilities
- 🖼️ **Multimodal**: Send images and documents to Claude
- 🔧 **Builder Pattern**: Fluent APIs for constructing requests
- 📊 **Token Counting**: Estimate costs before making requests
- 🔄 **Retry Logic**: Built-in exponential backoff for transient errors
- 📝 **Comprehensive Docs**: Extensive documentation and examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
anthropic = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

```rust
use anthropic::{Client, Model, ContentBlock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client (reads API key from ANTHROPIC_API_KEY env var)
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    // Send a message
    let request = client.chat_builder()
        .user_message(ContentBlock::text("Hello, Claude!"))
        .build();
    
    let response = client.execute_chat(request).await?;
    
    // Print Claude's response
    for content in response.content {
        if let ContentBlock::Text { text, .. } = content {
            println!("Claude: {}", text);
        }
    }
    
    Ok(())
}
```

## Authentication

Set your API key as an environment variable:

```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

Or configure it explicitly:

```rust
let client = Client::builder()
    .api_key("your-api-key")
    .model(Model::Claude35Sonnet20241022)
    .build()?;
```

## Examples

### Basic Conversation

```rust
use anthropic::{Client, Model, ContentBlock, Role, MessageParam};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    // Build a conversation with history
    let request = client.chat_builder()
        .system("You are a helpful assistant.")
        .user_message(ContentBlock::text("What's the capital of France?"))
        .assistant_message(ContentBlock::text("The capital of France is Paris."))
        .user_message(ContentBlock::text("What's its population?"))
        .build();
    
    let response = client.execute_chat(request).await?;
    
    if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
        println!("Claude: {}", text);
    }
    
    Ok(())
}
```

### Streaming Responses

```rust
use anthropic::{Client, Model, ContentBlock, StreamEvent};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    let request = client.chat_builder()
        .user_message(ContentBlock::text("Write a short poem about Rust"))
        .build();
    
    let mut stream = client.stream_chat(request).await?;
    
    print!("Claude: ");
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let anthropic::ContentDelta::TextDelta { text } = delta {
                    print!("{}", text);
                }
            }
            StreamEvent::MessageStop => break,
            _ => {}
        }
    }
    println!();
    
    Ok(())
}
```

### Tool Calling

```rust
use anthropic::{Client, Model, ContentBlock, Tool};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    // Define a tool
    let weather_tool = Tool::new("get_weather")
        .description("Get current weather for a location")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }))
        .build();
    
    let request = client.chat_builder()
        .user_message(ContentBlock::text("What's the weather in San Francisco?"))
        .tool(weather_tool)
        .build();
    
    let response = client.execute_chat(request).await?;
    
    // Handle tool use in response
    for content in response.content {
        match content {
            ContentBlock::Text { text, .. } => {
                println!("Claude: {}", text);
            }
            ContentBlock::ToolUse { name, input, .. } => {
                println!("Claude wants to use tool: {} with input: {}", name, input);
                // Implement your tool logic here
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

### Multimodal (Images)

```rust
use anthropic::{Client, Model, ContentBlock, ImageMediaType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    // Load and encode an image
    let image_data = std::fs::read("path/to/image.jpg")?;
    let base64_image = base64::encode(&image_data);
    
    let request = client.chat_builder()
        .user_message(ContentBlock::image_base64(
            ImageMediaType::Jpeg,
            base64_image
        ))
        .user_message(ContentBlock::text("What do you see in this image?"))
        .build();
    
    let response = client.execute_chat(request).await?;
    
    if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
        println!("Claude: {}", text);
    }
    
    Ok(())
}
```

### Advanced Configuration

```rust
use anthropic::{Client, Model, RetryConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key("your-api-key")
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(2000)
        .timeout(Duration::from_secs(30))
        .retry_config(RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        })
        .build()?;
    
    // Use the configured client...
    Ok(())
}
```

### Token Counting

```rust
use anthropic::{Client, Model, ContentBlock, types::{CountTokensRequest, MessageParam, Role}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    let request = CountTokensRequest {
        messages: vec![
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("How many tokens is this message?")],
            }
        ],
        system: None,
        tools: None,
    };
    
    let token_count = client.count_tokens(request).await?;
    println!("This message uses {} input tokens", token_count.input_tokens);
    
    Ok(())
}
```

## Error Handling

The SDK provides comprehensive error handling with detailed error types:

```rust
use anthropic::{Client, Model, Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Model::Claude35Sonnet20241022)?;
    
    // ... create request ...
    
    match client.execute_chat(request).await {
        Ok(response) => {
            println!("Success: {:?}", response);
        }
        Err(Error::Authentication(msg)) => {
            eprintln!("Authentication failed: {}", msg);
        }
        Err(Error::RateLimit { retry_after }) => {
            eprintln!("Rate limited. Retry after: {:?}", retry_after);
        }
        Err(Error::Network(err)) => {
            eprintln!("Network error: {}", err);
        }
        Err(err) => {
            eprintln!("Other error: {}", err);
        }
    }
    
    Ok(())
}
```

## Model Selection

Choose the right model for your use case:

| Model | Best For | Speed | Cost |
|-------|----------|-------|------|
| `Claude3Haiku20240307` | Simple tasks, fast responses | Fastest | Lowest |
| `Claude35Sonnet20241022` | Balanced performance | Medium | Medium |
| `Claude3Opus20240229` | Complex reasoning, analysis | Slower | Higher |

```rust
// Use different models for different tasks
let client = Client::new(Model::Claude35Sonnet20241022)?;

// Use Haiku for simple, fast responses
let simple_response = client.execute_chat_with_model(
    Model::Claude3Haiku20240307,
    simple_request
).await?;

// Use Opus for complex analysis
let complex_response = client.execute_chat_with_model(
    Model::Claude3Opus20240229,
    complex_request
).await?;
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Links

- [Anthropic API Documentation](https://docs.anthropic.com/)
- [Claude Models Overview](https://docs.anthropic.com/claude/docs/models-overview)
- [API Reference](https://docs.anthropic.com/claude/reference/)