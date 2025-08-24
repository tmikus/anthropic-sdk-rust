//! Example demonstrating the MessageAccumulator for reconstructing complete messages from streams

use anthropic::{Client, Model, ContentBlock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client
    let client = Client::builder()
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(512)
        .build()?;

    // Create a chat request
    let request = client.chat_builder()
        .user_message(ContentBlock::text("Explain what streaming is in 2-3 sentences."))
        .build();

    println!("Using MessageAccumulator to get complete message:");
    println!("================================================");

    // Stream the response and accumulate it
    let stream = client.stream_chat(request).await?;
    let accumulator = stream.accumulate();
    
    // Get the final complete message
    let final_message = accumulator.accumulate().await?;

    println!("Complete message received:");
    println!("ID: {}", final_message.id);
    println!("Role: {:?}", final_message.role);
    println!("Model: {:?}", final_message.model);
    println!("Stop reason: {:?}", final_message.stop_reason);
    println!("Usage: {} input tokens, {} output tokens", 
             final_message.usage.input_tokens, 
             final_message.usage.output_tokens);
    
    println!("\nContent:");
    for (i, content_block) in final_message.content.iter().enumerate() {
        match content_block {
            ContentBlock::Text { text, .. } => {
                println!("Block {}: {}", i, text);
            }
            _ => {
                println!("Block {}: {:?}", i, content_block);
            }
        }
    }

    Ok(())
}