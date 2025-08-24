//! Multimodal content handling utilities

use std::path::Path;

use base64::{engine::general_purpose, Engine as _};

use crate::{
    error::Error,
    types::{ContentBlock, DocumentMediaType, ImageMediaType},
    Result,
};

/// Utilities for handling image content
pub struct ImageUtils;

impl ImageUtils {
    /// Create an image content block from a file path
    pub async fn from_file(path: impl AsRef<Path>) -> Result<ContentBlock> {
        let path = path.as_ref();
        let media_type = Self::detect_media_type(path)?;
        let data = tokio::fs::read(path).await.map_err(|e| {
            Error::Config(format!("Failed to read image file: {}", e))
        })?;
        let encoded = general_purpose::STANDARD.encode(&data);
        
        Ok(ContentBlock::image_base64(media_type, encoded))
    }

    /// Create an image content block from raw bytes
    pub fn from_bytes(data: &[u8], media_type: ImageMediaType) -> ContentBlock {
        let encoded = general_purpose::STANDARD.encode(data);
        ContentBlock::image_base64(media_type, encoded)
    }

    /// Detect media type from file extension
    fn detect_media_type(path: &Path) -> Result<ImageMediaType> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| Error::Config("Unable to determine file extension".to_string()))?
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" => Ok(ImageMediaType::Jpeg),
            "png" => Ok(ImageMediaType::Png),
            "gif" => Ok(ImageMediaType::Gif),
            "webp" => Ok(ImageMediaType::WebP),
            _ => Err(Error::Config(format!(
                "Unsupported image format: {}",
                extension
            ))),
        }
    }
}

/// Utilities for handling document content
pub struct DocumentUtils;

impl DocumentUtils {
    /// Create a document content block from a file path
    pub async fn from_file(path: impl AsRef<Path>) -> Result<ContentBlock> {
        let path = path.as_ref();
        let media_type = Self::detect_media_type(path)?;
        let data = tokio::fs::read(path).await.map_err(|e| {
            Error::Config(format!("Failed to read document file: {}", e))
        })?;
        let encoded = general_purpose::STANDARD.encode(&data);
        
        Ok(ContentBlock::Document {
            source: crate::types::DocumentSource::Base64 {
                media_type,
                data: encoded,
            },
        })
    }

    /// Create a document content block from raw bytes
    pub fn from_bytes(data: &[u8], media_type: DocumentMediaType) -> ContentBlock {
        let encoded = general_purpose::STANDARD.encode(data);
        ContentBlock::Document {
            source: crate::types::DocumentSource::Base64 {
                media_type,
                data: encoded,
            },
        }
    }

    /// Detect media type from file extension
    fn detect_media_type(path: &Path) -> Result<DocumentMediaType> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| Error::Config("Unable to determine file extension".to_string()))?
            .to_lowercase();

        match extension.as_str() {
            "pdf" => Ok(DocumentMediaType::Pdf),
            "txt" => Ok(DocumentMediaType::Text),
            _ => Err(Error::Config(format!(
                "Unsupported document format: {}",
                extension
            ))),
        }
    }
}

/// Validate URL for remote content
pub fn validate_url(url: &str) -> Result<url::Url> {
    let parsed = url::Url::parse(url)?;
    
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(Error::Config(
            "URL must use HTTP or HTTPS scheme".to_string(),
        ));
    }
    
    Ok(parsed)
}