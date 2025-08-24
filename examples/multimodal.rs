//! Multimodal content example
//! 
//! This example demonstrates how to work with images and documents
//! using the Anthropic SDK's multimodal capabilities.

use anthropic::{
    multimodal::{DocumentUtils, ImageUtils},
    types::{ContentBlock, DocumentMediaType, ImageMediaType, Role},
    Client, Model, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the client (will use dummy API key for demonstration)
    let client = Client::builder()
        .api_key("sk-ant-dummy-key-for-demo-purposes-only") // In real usage, use environment variable
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1024)
        .build()?;

    println!("=== Multimodal Content Examples ===\n");

    // Example 1: Image from base64 data
    println!("1. Creating image content from base64 data...");
    let image_data = create_sample_png_data();
    let image_content = ImageUtils::from_bytes(&image_data, ImageMediaType::Png)?;
    println!("   ✓ Created PNG image content block");

    // Example 2: Image from URL
    println!("2. Creating image content from URL...");
    let _image_url_content = ImageUtils::from_url("https://example.com/sample.jpg")?;
    println!("   ✓ Created image content block from URL");

    // Example 3: Document from base64 data
    println!("3. Creating document content from base64 data...");
    let doc_data = create_sample_pdf_data();
    let doc_content = DocumentUtils::from_bytes(&doc_data, DocumentMediaType::Pdf)?;
    println!("   ✓ Created PDF document content block");

    // Example 4: Document from URL
    println!("4. Creating document content from URL...");
    let _doc_url_content = DocumentUtils::from_url("https://example.com/sample.pdf")?;
    println!("   ✓ Created document content block from URL");

    // Example 5: Text document
    println!("5. Creating text document from string...");
    let text_data = "This is a sample text document with some content.";
    let _text_content = DocumentUtils::from_bytes(text_data.as_bytes(), DocumentMediaType::Text)?;
    println!("   ✓ Created text document content block");

    // Example 6: Building a multimodal chat request
    println!("6. Building multimodal chat request...");
    let chat_request = client
        .chat_builder()
        .message_with_content(
            Role::User,
            vec![
                ContentBlock::text("Please analyze this image and document:"),
                image_content,
                doc_content,
            ],
        )
        .temperature(0.7)
        .build();

    println!("   ✓ Built chat request with {} content blocks", 
             chat_request.messages[0].content.len());

    // Example 7: Demonstrate MIME type utilities
    println!("7. MIME type utilities...");
    demonstrate_mime_utilities();

    // Example 8: Demonstrate validation
    println!("8. Content validation...");
    demonstrate_validation()?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

/// Create sample PNG data with valid magic bytes
fn create_sample_png_data() -> Vec<u8> {
    // PNG signature + minimal IHDR chunk
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // Width: 1
        0x00, 0x00, 0x00, 0x01, // Height: 1
        0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, etc.
        0x90, 0x77, 0x53, 0xDE, // CRC
    ]
}

/// Create sample PDF data with valid magic bytes
fn create_sample_pdf_data() -> Vec<u8> {
    b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000010 00000 n \n0000000079 00000 n \n0000000173 00000 n \ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n253\n%%EOF".to_vec()
}

/// Demonstrate MIME type utilities
fn demonstrate_mime_utilities() {
    use anthropic::multimodal::MimeUtils;

    println!("   Image MIME types:");
    println!("     JPEG: {}", MimeUtils::image_media_type_to_string(ImageMediaType::Jpeg));
    println!("     PNG:  {}", MimeUtils::image_media_type_to_string(ImageMediaType::Png));
    println!("     GIF:  {}", MimeUtils::image_media_type_to_string(ImageMediaType::Gif));
    println!("     WebP: {}", MimeUtils::image_media_type_to_string(ImageMediaType::WebP));

    println!("   Document MIME types:");
    println!("     PDF:  {}", MimeUtils::document_media_type_to_string(DocumentMediaType::Pdf));
    println!("     Text: {}", MimeUtils::document_media_type_to_string(DocumentMediaType::Text));

    println!("   MIME type support:");
    println!("     image/jpeg supported: {}", MimeUtils::is_supported_image_mime("image/jpeg"));
    println!("     image/bmp supported:  {}", MimeUtils::is_supported_image_mime("image/bmp"));
    println!("     application/pdf supported: {}", MimeUtils::is_supported_document_mime("application/pdf"));
    println!("     application/msword supported: {}", MimeUtils::is_supported_document_mime("application/msword"));
}

/// Demonstrate content validation
fn demonstrate_validation() -> Result<()> {
    use anthropic::multimodal::{validate_url, Base64Utils};

    println!("   URL validation:");
    
    // Valid URLs
    let valid_urls = [
        "https://example.com/image.jpg",
        "http://cdn.example.com/doc.pdf",
    ];
    
    for url in &valid_urls {
        match validate_url(url) {
            Ok(_) => println!("     ✓ Valid: {}", url),
            Err(e) => println!("     ✗ Invalid: {} ({})", url, e),
        }
    }

    // Invalid URLs
    let invalid_urls = [
        "ftp://example.com/file.jpg",
        "https://localhost/file.jpg",
        "not-a-url",
    ];
    
    for url in &invalid_urls {
        match validate_url(url) {
            Ok(_) => println!("     ✗ Should be invalid: {}", url),
            Err(_) => println!("     ✓ Correctly rejected: {}", url),
        }
    }

    println!("   Base64 validation:");
    let valid_base64 = "SGVsbG8sIFdvcmxkIQ=="; // "Hello, World!"
    let invalid_base64 = "This is not base64!@#";
    
    match Base64Utils::validate(valid_base64) {
        Ok(_) => println!("     ✓ Valid base64: {}", valid_base64),
        Err(e) => println!("     ✗ Should be valid: {} ({})", valid_base64, e),
    }
    
    match Base64Utils::validate(invalid_base64) {
        Ok(_) => println!("     ✗ Should be invalid: {}", invalid_base64),
        Err(_) => println!("     ✓ Correctly rejected: {}", invalid_base64),
    }

    Ok(())
}