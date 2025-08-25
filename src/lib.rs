//! # Anthropic Rust SDK
//!
//! A modern Rust SDK for the Anthropic API, providing type-safe, async-first access to Claude models.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use anthropic_rust::{Client, Model, ContentBlock};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new(Model::Claude35Sonnet20241022)?;
//!     
//!     let request = client.chat_builder()
//!         .user_message(ContentBlock::text("Hello, Claude!"))
//!         .build();
//!     
//!     let response = client.execute_chat(request).await?;
//!     println!("Response: {:?}", response);
//!     
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod multimodal;
pub mod streaming;
pub mod tools;
pub mod types;

// Mock infrastructure for unit tests
#[cfg(test)]
pub mod mock;

// Test modules
#[cfg(test)]
mod config_test;
#[cfg(test)]
mod error_test;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
mod types_test;

#[cfg(test)]
mod advanced_config_test;
#[cfg(test)]
mod client_test;
#[cfg(test)]
mod streaming_test;

// Re-export commonly used types for convenience
pub use client::{Client, LoggingInterceptor, RequestInterceptor, RequestMiddleware, RetryConfig};
pub use config::{ClientBuilder, Config};
pub use error::Error;
pub use streaming::{
    ContentDelta, MessageAccumulator, MessageDelta, MessageStream, PartialMessage, StreamEvent,
};
pub use tools::{Tool, ToolBuilder};
pub use types::{
    ChatRequest, ChatRequestBuilder, ContentBlock, CountTokensRequest, DocumentMediaType,
    DocumentSource, ImageMediaType, ImageSource, Message, MessageParam, Model, Role, StopReason,
    SystemMessage, TokenCount, Usage,
};

// Re-export multimodal utilities for convenience
pub use multimodal::{validate_url, Base64Utils, DocumentUtils, ImageUtils, MimeUtils};

/// Result type alias for the SDK
pub type Result<T> = std::result::Result<T, Error>;
