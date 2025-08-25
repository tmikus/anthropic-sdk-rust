//! Multimodal content handling utilities

use std::path::Path;

use base64::{engine::general_purpose, Engine as _};
use mime::Mime;

use crate::{
    error::Error,
    types::{ContentBlock, DocumentMediaType, DocumentSource, ImageMediaType, ImageSource},
    Result,
};

/// Utilities for handling image content
pub struct ImageUtils;

impl ImageUtils {
    /// Create an image content block from a file path
    pub async fn from_file(path: impl AsRef<Path>) -> Result<ContentBlock> {
        let path = path.as_ref();

        // Validate file exists and is readable
        if !path.exists() {
            return Err(Error::Config(format!(
                "Image file does not exist: {}",
                path.display()
            )));
        }

        if !path.is_file() {
            return Err(Error::Config(format!(
                "Path is not a file: {}",
                path.display()
            )));
        }

        let media_type = Self::detect_media_type(path)?;
        let data = tokio::fs::read(path).await.map_err(|e| {
            Error::Config(format!(
                "Failed to read image file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Validate file size (max 20MB for images)
        const MAX_IMAGE_SIZE: usize = 20 * 1024 * 1024;
        if data.len() > MAX_IMAGE_SIZE {
            return Err(Error::Config(format!(
                "Image file too large: {} bytes (max: {} bytes)",
                data.len(),
                MAX_IMAGE_SIZE
            )));
        }

        let encoded = general_purpose::STANDARD.encode(&data);
        Ok(ContentBlock::image_base64(media_type, encoded))
    }

    /// Create an image content block from raw bytes
    pub fn from_bytes(data: &[u8], media_type: ImageMediaType) -> Result<ContentBlock> {
        // Validate file size
        const MAX_IMAGE_SIZE: usize = 20 * 1024 * 1024;
        if data.len() > MAX_IMAGE_SIZE {
            return Err(Error::Config(format!(
                "Image data too large: {} bytes (max: {} bytes)",
                data.len(),
                MAX_IMAGE_SIZE
            )));
        }

        let encoded = general_purpose::STANDARD.encode(data);
        Ok(ContentBlock::image_base64(media_type, encoded))
    }

    /// Create an image content block from a URL
    pub fn from_url(url: impl AsRef<str>) -> Result<ContentBlock> {
        let validated_url = validate_url(url.as_ref())?;
        Ok(ContentBlock::Image {
            source: ImageSource::Url { url: validated_url },
        })
    }

    /// Detect media type from file extension
    pub fn detect_media_type(path: &Path) -> Result<ImageMediaType> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                Error::Config(format!(
                    "Unable to determine file extension for: {}",
                    path.display()
                ))
            })?
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" => Ok(ImageMediaType::Jpeg),
            "png" => Ok(ImageMediaType::Png),
            "gif" => Ok(ImageMediaType::Gif),
            "webp" => Ok(ImageMediaType::WebP),
            _ => Err(Error::Config(format!(
                "Unsupported image format '{}' for file: {}",
                extension,
                path.display()
            ))),
        }
    }

    /// Detect media type from MIME type string
    pub fn detect_media_type_from_mime(mime_str: &str) -> Result<ImageMediaType> {
        let mime: Mime = mime_str
            .parse()
            .map_err(|_| Error::Config(format!("Invalid MIME type: {}", mime_str)))?;

        match (mime.type_(), mime.subtype()) {
            (mime::IMAGE, mime::JPEG) => Ok(ImageMediaType::Jpeg),
            (mime::IMAGE, mime::PNG) => Ok(ImageMediaType::Png),
            (mime::IMAGE, mime::GIF) => Ok(ImageMediaType::Gif),
            (mime::IMAGE, subtype) if subtype == "webp" => Ok(ImageMediaType::WebP),
            _ => Err(Error::Config(format!(
                "Unsupported image MIME type: {}",
                mime_str
            ))),
        }
    }

    /// Validate image data format by checking magic bytes
    pub fn validate_image_format(data: &[u8], expected_type: ImageMediaType) -> Result<()> {
        if data.is_empty() {
            return Err(Error::Config("Image data is empty".to_string()));
        }

        let is_valid = match expected_type {
            ImageMediaType::Jpeg => data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8,
            ImageMediaType::Png => {
                data.len() >= 8 && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
            }
            ImageMediaType::Gif => {
                data.len() >= 6
                    && (
                        data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x37, 0x61] || // GIF87a
                    data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x39, 0x61]
                        // GIF89a
                    )
            }
            ImageMediaType::WebP => {
                data.len() >= 12 &&
                data[0..4] == [0x52, 0x49, 0x46, 0x46] && // RIFF
                data[8..12] == [0x57, 0x45, 0x42, 0x50] // WEBP
            }
        };

        if !is_valid {
            return Err(Error::Config(format!(
                "Image data does not match expected format: {:?}",
                expected_type
            )));
        }

        Ok(())
    }
}

/// Utilities for handling document content
pub struct DocumentUtils;

impl DocumentUtils {
    /// Create a document content block from a file path
    pub async fn from_file(path: impl AsRef<Path>) -> Result<ContentBlock> {
        let path = path.as_ref();

        // Validate file exists and is readable
        if !path.exists() {
            return Err(Error::Config(format!(
                "Document file does not exist: {}",
                path.display()
            )));
        }

        if !path.is_file() {
            return Err(Error::Config(format!(
                "Path is not a file: {}",
                path.display()
            )));
        }

        let media_type = Self::detect_media_type(path)?;
        let data = tokio::fs::read(path).await.map_err(|e| {
            Error::Config(format!(
                "Failed to read document file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Validate file size (max 32MB for documents)
        const MAX_DOCUMENT_SIZE: usize = 32 * 1024 * 1024;
        if data.len() > MAX_DOCUMENT_SIZE {
            return Err(Error::Config(format!(
                "Document file too large: {} bytes (max: {} bytes)",
                data.len(),
                MAX_DOCUMENT_SIZE
            )));
        }

        // Validate document format
        Self::validate_document_format(&data, &media_type)?;

        let encoded = general_purpose::STANDARD.encode(&data);
        Ok(ContentBlock::Document {
            source: DocumentSource::Base64 {
                media_type,
                data: encoded,
            },
        })
    }

    /// Create a document content block from raw bytes
    pub fn from_bytes(data: &[u8], media_type: DocumentMediaType) -> Result<ContentBlock> {
        // Validate file size
        const MAX_DOCUMENT_SIZE: usize = 32 * 1024 * 1024;
        if data.len() > MAX_DOCUMENT_SIZE {
            return Err(Error::Config(format!(
                "Document data too large: {} bytes (max: {} bytes)",
                data.len(),
                MAX_DOCUMENT_SIZE
            )));
        }

        // Validate document format
        Self::validate_document_format(data, &media_type)?;

        let encoded = general_purpose::STANDARD.encode(data);
        Ok(ContentBlock::Document {
            source: DocumentSource::Base64 {
                media_type,
                data: encoded,
            },
        })
    }

    /// Create a document content block from a URL
    pub fn from_url(url: impl AsRef<str>) -> Result<ContentBlock> {
        let validated_url = validate_url(url.as_ref())?;
        Ok(ContentBlock::Document {
            source: DocumentSource::Url { url: validated_url },
        })
    }

    /// Detect media type from file extension
    pub fn detect_media_type(path: &Path) -> Result<DocumentMediaType> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                Error::Config(format!(
                    "Unable to determine file extension for: {}",
                    path.display()
                ))
            })?
            .to_lowercase();

        match extension.as_str() {
            "pdf" => Ok(DocumentMediaType::Pdf),
            "txt" => Ok(DocumentMediaType::Text),
            _ => Err(Error::Config(format!(
                "Unsupported document format '{}' for file: {}",
                extension,
                path.display()
            ))),
        }
    }

    /// Detect media type from MIME type string
    pub fn detect_media_type_from_mime(mime_str: &str) -> Result<DocumentMediaType> {
        let mime: Mime = mime_str
            .parse()
            .map_err(|_| Error::Config(format!("Invalid MIME type: {}", mime_str)))?;

        match (mime.type_(), mime.subtype()) {
            (mime::APPLICATION, subtype) if subtype == "pdf" => Ok(DocumentMediaType::Pdf),
            (mime::TEXT, mime::PLAIN) => Ok(DocumentMediaType::Text),
            _ => Err(Error::Config(format!(
                "Unsupported document MIME type: {}",
                mime_str
            ))),
        }
    }

    /// Validate document data format by checking magic bytes
    pub fn validate_document_format(data: &[u8], expected_type: &DocumentMediaType) -> Result<()> {
        if data.is_empty() {
            return Err(Error::Config("Document data is empty".to_string()));
        }

        let is_valid = match expected_type {
            DocumentMediaType::Pdf => {
                data.len() >= 4 && data[0..4] == [0x25, 0x50, 0x44, 0x46] // %PDF
            }
            DocumentMediaType::Text => {
                // For text files, check if it's valid UTF-8
                std::str::from_utf8(data).is_ok()
            }
        };

        if !is_valid {
            return Err(Error::Config(format!(
                "Document data does not match expected format: {:?}",
                expected_type
            )));
        }

        Ok(())
    }
}

/// Validate URL for remote content
pub fn validate_url(url: &str) -> Result<url::Url> {
    if url.is_empty() {
        return Err(Error::Config("URL cannot be empty".to_string()));
    }

    let parsed =
        url::Url::parse(url).map_err(|e| Error::Config(format!("Invalid URL '{}': {}", url, e)))?;

    // Validate scheme
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(Error::Config(format!(
            "URL must use HTTP or HTTPS scheme, got: {}",
            parsed.scheme()
        )));
    }

    // Validate host
    if parsed.host().is_none() {
        return Err(Error::Config(format!(
            "URL must have a valid host: {}",
            url
        )));
    }

    // Check for suspicious patterns
    let host_str = parsed.host_str().unwrap_or("");
    if host_str == "localhost"
        || host_str.starts_with("127.")
        || host_str.starts_with("192.168.")
        || host_str.starts_with("10.")
    {
        return Err(Error::Config(format!(
            "URLs pointing to local/private networks are not allowed: {}",
            url
        )));
    }

    Ok(parsed)
}

/// Base64 encoding utilities
pub struct Base64Utils;

impl Base64Utils {
    /// Encode bytes to base64 string
    pub fn encode(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(data)
    }

    /// Decode base64 string to bytes
    pub fn decode(encoded: &str) -> Result<Vec<u8>> {
        general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| Error::Config(format!("Invalid base64 data: {}", e)))
    }

    /// Validate base64 string format
    pub fn validate(encoded: &str) -> Result<()> {
        Self::decode(encoded).map(|_| ())
    }
}

/// MIME type utilities
pub struct MimeUtils;

impl MimeUtils {
    /// Get MIME type string from ImageMediaType
    pub fn image_media_type_to_string(media_type: ImageMediaType) -> &'static str {
        match media_type {
            ImageMediaType::Jpeg => "image/jpeg",
            ImageMediaType::Png => "image/png",
            ImageMediaType::Gif => "image/gif",
            ImageMediaType::WebP => "image/webp",
        }
    }

    /// Get MIME type string from DocumentMediaType
    pub fn document_media_type_to_string(media_type: DocumentMediaType) -> &'static str {
        match media_type {
            DocumentMediaType::Pdf => "application/pdf",
            DocumentMediaType::Text => "text/plain",
        }
    }

    /// Parse MIME type and determine if it's a supported image type
    pub fn is_supported_image_mime(mime_str: &str) -> bool {
        ImageUtils::detect_media_type_from_mime(mime_str).is_ok()
    }

    /// Parse MIME type and determine if it's a supported document type
    pub fn is_supported_document_mime(mime_str: &str) -> bool {
        DocumentUtils::detect_media_type_from_mime(mime_str).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid() {
        let valid_urls = [
            "https://example.com/image.jpg",
            "http://example.com/document.pdf",
            "https://cdn.example.com/path/to/file.png",
        ];

        for url in &valid_urls {
            assert!(validate_url(url).is_ok(), "URL should be valid: {}", url);
        }
    }

    #[test]
    fn test_validate_url_invalid() {
        let invalid_urls = [
            "",
            "ftp://example.com/file.jpg",
            "file:///local/file.png",
            "https://localhost/file.jpg",
            "https://127.0.0.1/file.jpg",
            "https://192.168.1.1/file.jpg",
            "https://10.0.0.1/file.jpg",
            "not-a-url",
            "https://",
        ];

        for url in &invalid_urls {
            assert!(validate_url(url).is_err(), "URL should be invalid: {}", url);
        }
    }

    #[test]
    fn test_base64_utils() {
        let data = b"Hello, World!";
        let encoded = Base64Utils::encode(data);
        let decoded = Base64Utils::decode(&encoded).unwrap();
        assert_eq!(data, decoded.as_slice());

        // Test validation
        assert!(Base64Utils::validate(&encoded).is_ok());
        assert!(Base64Utils::validate("invalid-base64!@#").is_err());
    }

    #[test]
    fn test_image_media_type_detection() {
        // Test file extension detection
        let test_cases = [
            ("test.jpg", ImageMediaType::Jpeg),
            ("test.jpeg", ImageMediaType::Jpeg),
            ("test.png", ImageMediaType::Png),
            ("test.gif", ImageMediaType::Gif),
            ("test.webp", ImageMediaType::WebP),
        ];

        for (filename, expected) in &test_cases {
            let path = Path::new(filename);
            let detected = ImageUtils::detect_media_type(path).unwrap();
            assert_eq!(detected, *expected);
        }

        // Test MIME type detection
        let mime_cases = [
            ("image/jpeg", ImageMediaType::Jpeg),
            ("image/png", ImageMediaType::Png),
            ("image/gif", ImageMediaType::Gif),
            ("image/webp", ImageMediaType::WebP),
        ];

        for (mime_str, expected) in &mime_cases {
            let detected = ImageUtils::detect_media_type_from_mime(mime_str).unwrap();
            assert_eq!(detected, *expected);
        }
    }

    #[test]
    fn test_document_media_type_detection() {
        // Test file extension detection
        let test_cases = [
            ("test.pdf", DocumentMediaType::Pdf),
            ("test.txt", DocumentMediaType::Text),
        ];

        for (filename, expected) in &test_cases {
            let path = Path::new(filename);
            let detected = DocumentUtils::detect_media_type(path).unwrap();
            assert_eq!(detected, *expected);
        }

        // Test MIME type detection
        let mime_cases = [
            ("application/pdf", DocumentMediaType::Pdf),
            ("text/plain", DocumentMediaType::Text),
        ];

        for (mime_str, expected) in &mime_cases {
            let detected = DocumentUtils::detect_media_type_from_mime(mime_str).unwrap();
            assert_eq!(detected, *expected);
        }
    }

    #[test]
    fn test_image_format_validation() {
        // JPEG magic bytes
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0];
        assert!(ImageUtils::validate_image_format(&jpeg_data, ImageMediaType::Jpeg).is_ok());
        assert!(ImageUtils::validate_image_format(&jpeg_data, ImageMediaType::Png).is_err());

        // PNG magic bytes
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(ImageUtils::validate_image_format(&png_data, ImageMediaType::Png).is_ok());
        assert!(ImageUtils::validate_image_format(&png_data, ImageMediaType::Jpeg).is_err());

        // GIF87a magic bytes
        let gif_data = [0x47, 0x49, 0x46, 0x38, 0x37, 0x61];
        assert!(ImageUtils::validate_image_format(&gif_data, ImageMediaType::Gif).is_ok());

        // WebP magic bytes
        let webp_data = [
            0x52, 0x49, 0x46, 0x46, // RIFF
            0x00, 0x00, 0x00, 0x00, // file size (placeholder)
            0x57, 0x45, 0x42, 0x50, // WEBP
        ];
        assert!(ImageUtils::validate_image_format(&webp_data, ImageMediaType::WebP).is_ok());

        // Empty data should fail
        assert!(ImageUtils::validate_image_format(&[], ImageMediaType::Jpeg).is_err());
    }

    #[test]
    fn test_document_format_validation() {
        // PDF magic bytes
        let pdf_data = b"%PDF-1.4";
        assert!(DocumentUtils::validate_document_format(pdf_data, &DocumentMediaType::Pdf).is_ok());

        // PDF data should not be validated as text when we expect text format
        // Note: PDF data is valid UTF-8, but we validate based on magic bytes for PDF
        let non_pdf_data = b"This is just text, not a PDF";
        assert!(
            DocumentUtils::validate_document_format(non_pdf_data, &DocumentMediaType::Pdf).is_err()
        );

        // Valid UTF-8 text
        let text_data = b"Hello, World!";
        assert!(
            DocumentUtils::validate_document_format(text_data, &DocumentMediaType::Text).is_ok()
        );

        // Invalid UTF-8 should fail for text
        let invalid_utf8 = [0xFF, 0xFE, 0xFD];
        assert!(
            DocumentUtils::validate_document_format(&invalid_utf8, &DocumentMediaType::Text)
                .is_err()
        );

        // Empty data should fail
        assert!(DocumentUtils::validate_document_format(&[], &DocumentMediaType::Pdf).is_err());
    }

    #[test]
    fn test_image_from_bytes() {
        // Create valid JPEG data
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]; // Minimal JPEG header
        let content_block = ImageUtils::from_bytes(&jpeg_data, ImageMediaType::Jpeg).unwrap();

        match content_block {
            ContentBlock::Image {
                source: ImageSource::Base64 { media_type, data },
            } => {
                assert_eq!(media_type, ImageMediaType::Jpeg);
                assert!(!data.is_empty());
            }
            _ => panic!("Expected Image content block with Base64 source"),
        }
    }

    #[test]
    fn test_document_from_bytes() {
        // Create valid PDF data
        let pdf_data = b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n>>\nendobj";
        let content_block = DocumentUtils::from_bytes(pdf_data, DocumentMediaType::Pdf).unwrap();

        match content_block {
            ContentBlock::Document {
                source: DocumentSource::Base64 { media_type, data },
            } => {
                assert_eq!(media_type, DocumentMediaType::Pdf);
                assert!(!data.is_empty());
            }
            _ => panic!("Expected Document content block with Base64 source"),
        }
    }

    #[test]
    fn test_image_from_url() {
        let url = "https://example.com/image.jpg";
        let content_block = ImageUtils::from_url(url).unwrap();

        match content_block {
            ContentBlock::Image {
                source: ImageSource::Url { url: parsed_url },
            } => {
                assert_eq!(parsed_url.as_str(), url);
            }
            _ => panic!("Expected Image content block with URL source"),
        }
    }

    #[test]
    fn test_document_from_url() {
        let url = "https://example.com/document.pdf";
        let content_block = DocumentUtils::from_url(url).unwrap();

        match content_block {
            ContentBlock::Document {
                source: DocumentSource::Url { url: parsed_url },
            } => {
                assert_eq!(parsed_url.as_str(), url);
            }
            _ => panic!("Expected Document content block with URL source"),
        }
    }

    #[test]
    fn test_mime_utils() {
        // Test image MIME type conversion
        assert_eq!(
            MimeUtils::image_media_type_to_string(ImageMediaType::Jpeg),
            "image/jpeg"
        );
        assert_eq!(
            MimeUtils::image_media_type_to_string(ImageMediaType::Png),
            "image/png"
        );
        assert_eq!(
            MimeUtils::image_media_type_to_string(ImageMediaType::Gif),
            "image/gif"
        );
        assert_eq!(
            MimeUtils::image_media_type_to_string(ImageMediaType::WebP),
            "image/webp"
        );

        // Test document MIME type conversion
        assert_eq!(
            MimeUtils::document_media_type_to_string(DocumentMediaType::Pdf),
            "application/pdf"
        );
        assert_eq!(
            MimeUtils::document_media_type_to_string(DocumentMediaType::Text),
            "text/plain"
        );

        // Test MIME type support detection
        assert!(MimeUtils::is_supported_image_mime("image/jpeg"));
        assert!(MimeUtils::is_supported_image_mime("image/png"));
        assert!(!MimeUtils::is_supported_image_mime("image/bmp"));

        assert!(MimeUtils::is_supported_document_mime("application/pdf"));
        assert!(MimeUtils::is_supported_document_mime("text/plain"));
        assert!(!MimeUtils::is_supported_document_mime("application/msword"));
    }

    #[test]
    fn test_size_limits() {
        // Test image size limit
        let large_data = vec![0u8; 25 * 1024 * 1024]; // 25MB
        assert!(ImageUtils::from_bytes(&large_data, ImageMediaType::Jpeg).is_err());

        // Test document size limit
        let large_doc_data = vec![0u8; 35 * 1024 * 1024]; // 35MB
        assert!(DocumentUtils::from_bytes(&large_doc_data, DocumentMediaType::Pdf).is_err());
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_file_operations() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Test image file operations
        let mut img_file = NamedTempFile::new().unwrap();
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG signature
        img_file.write_all(&png_data).unwrap();
        img_file.flush().unwrap();

        let img_path = img_file.path().with_extension("png");
        std::fs::copy(img_file.path(), &img_path).unwrap();

        let content_block = ImageUtils::from_file(&img_path).await.unwrap();
        match content_block {
            ContentBlock::Image {
                source: ImageSource::Base64 { media_type, .. },
            } => {
                assert_eq!(media_type, ImageMediaType::Png);
            }
            _ => panic!("Expected Image content block"),
        }

        // Clean up
        let _ = std::fs::remove_file(&img_path);

        // Test document file operations
        let mut doc_file = NamedTempFile::new().unwrap();
        let pdf_data = b"%PDF-1.4\nSample PDF content";
        doc_file.write_all(pdf_data).unwrap();
        doc_file.flush().unwrap();

        let doc_path = doc_file.path().with_extension("pdf");
        std::fs::copy(doc_file.path(), &doc_path).unwrap();

        let doc_content_block = DocumentUtils::from_file(&doc_path).await.unwrap();
        match doc_content_block {
            ContentBlock::Document {
                source: DocumentSource::Base64 { media_type, .. },
            } => {
                assert_eq!(media_type, DocumentMediaType::Pdf);
            }
            _ => panic!("Expected Document content block"),
        }

        // Clean up
        let _ = std::fs::remove_file(&doc_path);
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_file_error_conditions() {
        // Test non-existent file
        let result = ImageUtils::from_file("non_existent_file.jpg").await;
        assert!(result.is_err());

        // Test directory instead of file
        let result = ImageUtils::from_file(".").await;
        assert!(result.is_err());

        // Test unsupported extension
        let result = ImageUtils::from_file("test.bmp").await;
        assert!(result.is_err());
    }
}
