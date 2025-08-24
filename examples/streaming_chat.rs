//! Example demonstrating streaming chat functionality

use anthropic::{Client, Model, ContentBlock, StreamEvent};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client
    let client = Client::builder()
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1024)
        .build()?;

    // Create a chat request
    let request = client.chat_builder()
        .user_message(ContentBlock::text("Write a short poem about streaming data."))
        .build();

    println!("Streaming response from Claude:");
    println!("================================");

    // Stream the response
    let mut stream = client.stream_chat(request).await?;
    let mut accumulated_text = String::new();

    while let Some(event_result) = stream.next().await {
        match event_result? {
            StreamEvent::MessageStart { message } => {
                println!("Message started: ID = {}", message.id);
            }
            StreamEvent::ContentBlockStart { index, .. } => {
                println!("Content block {} started", index);
            }
            StreamEvent::ContentBlockDelta { index: _, delta } => {
                match delta {
                    anthropic::ContentDelta::TextDelta { text } => {
                        print!("{}", text);
                        accumulated_text.push_str(&text);
                        // Flush stdout to show text as it streams
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();
                    }
                }
            }
            StreamEvent::ContentBlockStop { index } => {
                println!("\nContent block {} completed", index);
            }
            StreamEvent::MessageDelta { delta } => {
                if let Some(stop_reason) = delta.stop_reason {
                    println!("Stop reason: {:?}", stop_reason);
                }
                if let Some(usage) = delta.usage {
                    println!("Token usage: {} input, {} output", 
                             usage.input_tokens, usage.output_tokens);
                }
            }
            StreamEvent::MessageStop => {
                println!("Message completed!");
                break;
            }
        }
    }

    println!("\n================================");
    println!("Final accumulated text:");
    println!("{}", accumulated_text);

    Ok(())
}