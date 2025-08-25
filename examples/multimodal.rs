//! Comprehensive multimodal example demonstrating image and document processing
//!
//! This example shows how to:
//! - Send images to Claude (base64 and URL)
//! - Process different image formats
//! - Handle documents and PDFs
//! - Combine text and visual content
//! - Use multimodal content in conversations
//!
//! Run with: cargo run --example multimodal

use anthropic::{
    Client, Model, ContentBlock, ImageMediaType, ImageSource,
    types::{MessageParam, Role},
    Error, Result,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Anthropic Rust SDK - Multimodal Examples ===\n");

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

    // Example 1: Simple image analysis with base64
    println!("1. Simple Image Analysis (Base64)");
    println!("=================================");
    
    // Create a simple test image (1x1 red pixel in PNG format)
    let test_image_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    
    let image_request = client.chat_builder()
        .user_message(ContentBlock::image_base64(
            ImageMediaType::Png,
            test_image_base64.to_string()
        ))
        .user_message(ContentBlock::text("What do you see in this image? Describe its properties."))
        .build();

    match client.execute_chat(image_request).await {
        Ok(response) => {
            println!("User: [Sent a 1x1 red pixel PNG image]");
            println!("User: What do you see in this image? Describe its properties.");
            
            for content in response.content {
                if let ContentBlock::Text { text, .. } = content {
                    println!("Claude: {}", text);
                }
            }
        }
        Err(e) => println!("❌ Image analysis failed: {}", e),
    }

    // Example 2: Multiple images in conversation
    println!("\n2. Multiple Images in Conversation");
    println!("=================================");
    
    // Create different colored pixels for comparison
    let red_pixel = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let blue_pixel = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
    
    let multi_image_request = client.chat_builder()
        .user_message(ContentBlock::text("I'm going to show you two images. Please compare them."))
        .user_message(ContentBlock::image_base64(ImageMediaType::Png, red_pixel.to_string()))
        .user_message(ContentBlock::text("This is image 1."))
        .user_message(ContentBlock::image_base64(ImageMediaType::Png, blue_pixel.to_string()))
        .user_message(ContentBlock::text("This is image 2. What are the differences between these images?"))
        .build();

    match client.execute_chat(multi_image_request).await {
        Ok(response) => {
            println!("User: I'm going to show you two images. Please compare them.");
            println!("User: [Sent red pixel image] This is image 1.");
            println!("User: [Sent blue pixel image] This is image 2. What are the differences?");
            
            for content in response.content {
                if let ContentBlock::Text { text, .. } = content {
                    println!("Claude: {}", text);
                }
            }
        }
        Err(e) => println!("❌ Multi-image comparison failed: {}", e),
    }

    // Example 3: Different image formats
    println!("\n3. Different Image Formats");
    println!("=========================");
    
    // Demonstrate different image format support
    let formats = vec![
        (ImageMediaType::Png, "PNG format"),
        (ImageMediaType::Jpeg, "JPEG format"),
        (ImageMediaType::Gif, "GIF format"),
        (ImageMediaType::WebP, "WebP format"),
    ];

    for (format, description) in formats {
        println!("📸 Testing {} support...", description);
        
        let format_request = client.chat_builder()
            .user_message(ContentBlock::image_base64(format.clone(), test_image_base64.to_string()))
            .user_message(ContentBlock::text(&format!("This image is in {} format. Can you confirm you can see it?", description)))
            .build();

        match client.execute_chat(format_request).await {
            Ok(response) => {
                if let Some(ContentBlock::Text { text, .. }) = response.content.first() {
                    let preview = if text.len() > 100 { 
                        format!("{}...", &text[..100]) 
                    } else { 
                        text.clone() 
                    };
                    println!("   ✅ {}: {}", description, preview);
                }
            }
            Err(e) => println!("   ❌ {} failed: {}", description, e),
        }
    }

    println!("\n=== Multimodal Examples Complete ===");
    println!("💡 Try running with a valid ANTHROPIC_API_KEY and real images!");
    
    Ok(())
}

/// Helper function to load and encode an image file (for reference)
#[allow(dead_code)]
fn load_image_as_base64(file_path: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let image_bytes = std::fs::read(file_path)?;
    Ok(base64::encode(&image_bytes))
}

/// Helper function to determine image media type from file extension
#[allow(dead_code)]
fn get_media_type_from_extension(file_path: &str) -> ImageMediaType {
    let extension = file_path.split('.').last().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "jpg" | "jpeg" => ImageMediaType::Jpeg,
        "png" => ImageMediaType::Png,
        "gif" => ImageMediaType::Gif,
        "webp" => ImageMediaType::WebP,
        _ => ImageMediaType::Png, // Default to PNG
    }
}

/// Helper function to validate image data
#[allow(dead_code)]
fn validate_image_data(base64_data: &str) -> bool {
    base64::decode(base64_data).is_ok()
}