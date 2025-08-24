//! Example demonstrating the ChatRequest and ChatRequestBuilder functionality

use anthropic::{
    Client, Model,
    types::{ChatRequestBuilder, ContentBlock, Role, MessageParam},
    tools::Tool,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with default configuration
    let client = Client::builder()
        .api_key("sk-ant-api03-example-key") // In real usage, use environment variables
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1000)
        .build()?;

    println!("Client default model: {:?}", client.default_model());
    println!("Client default max_tokens: {}", client.default_max_tokens());

    // Example 1: Basic chat request using the builder
    let basic_request = client.chat_builder()
        .user_message(ContentBlock::text("Hello, Claude!"))
        .assistant_message(ContentBlock::text("Hello! How can I help you today?"))
        .user_message(ContentBlock::text("What's the weather like?"))
        .build();

    println!("\n=== Basic Chat Request ===");
    println!("Messages: {}", basic_request.messages.len());
    for (i, msg) in basic_request.messages.iter().enumerate() {
        println!("Message {}: {:?} - {:?}", i + 1, msg.role, msg.content[0]);
    }

    // Example 2: Chat request with system message and parameters
    let advanced_request = ChatRequestBuilder::new()
        .system("You are a helpful assistant that responds concisely.")
        .user_message(ContentBlock::text("Explain quantum computing in simple terms."))
        .temperature(0.7)
        .top_p(0.9)
        .stop_sequence("END")
        .build();

    println!("\n=== Advanced Chat Request ===");
    println!("Has system message: {}", advanced_request.system.is_some());
    println!("Temperature: {:?}", advanced_request.temperature);
    println!("Top-p: {:?}", advanced_request.top_p);
    println!("Stop sequences: {:?}", advanced_request.stop_sequences);

    // Example 3: Multimodal content
    let multimodal_request = client.chat_builder()
        .message_with_content(Role::User, vec![
            ContentBlock::text("What do you see in this image?"),
            ContentBlock::image_base64(
                anthropic::types::ImageMediaType::Png,
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
            ),
        ])
        .build();

    println!("\n=== Multimodal Chat Request ===");
    println!("Content blocks in first message: {}", multimodal_request.messages[0].content.len());

    // Example 4: Tool usage
    let calculator_tool = Tool::new("calculator")
        .description("Perform mathematical calculations")
        .schema_value(serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Mathematical expression to evaluate"
                }
            },
            "required": ["expression"]
        }))
        .build();

    let tool_request = client.chat_builder()
        .user_message(ContentBlock::text("What is 15 * 23?"))
        .tool(calculator_tool.clone())
        .build();

    println!("\n=== Tool Usage Request ===");
    println!("Has tools: {}", tool_request.tools.is_some());
    if let Some(tools) = &tool_request.tools {
        println!("Tool count: {}", tools.len());
        println!("First tool name: {}", tools[0].name);
    }

    // Example 5: Bulk message addition
    let conversation_history = vec![
        MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Hello!")],
        },
        MessageParam {
            role: Role::Assistant,
            content: vec![ContentBlock::text("Hi there!")],
        },
        MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("How are you?")],
        },
    ];

    let bulk_request = client.chat_builder()
        .messages(conversation_history)
        .user_message(ContentBlock::text("What can you help me with?"))
        .build();

    println!("\n=== Bulk Messages Request ===");
    println!("Total messages: {}", bulk_request.messages.len());

    // Example 6: Complex request with all features
    let complex_request = client.chat_builder()
        .system("You are an expert assistant.")
        .system("Always provide detailed explanations.")
        .messages(vec![
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Previous context message")],
            }
        ])
        .user_message(ContentBlock::text("Current question"))
        .temperature(0.8)
        .top_p(0.95)
        .stop_sequences(vec!["STOP".to_string(), "END".to_string()])
        .tools(vec![calculator_tool])
        .build();

    println!("\n=== Complex Request ===");
    println!("Messages: {}", complex_request.messages.len());
    println!("System messages: {}", complex_request.system.as_ref().map_or(0, |s| s.len()));
    println!("Tools: {}", complex_request.tools.as_ref().map_or(0, |t| t.len()));
    println!("Stop sequences: {}", complex_request.stop_sequences.as_ref().map_or(0, |s| s.len()));

    // Demonstrate serialization
    let json = serde_json::to_string_pretty(&complex_request)?;
    println!("\n=== Serialized Request (first 500 chars) ===");
    println!("{}...", &json[..std::cmp::min(500, json.len())]);

    Ok(())
}