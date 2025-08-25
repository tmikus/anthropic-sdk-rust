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

// Test modules
#[cfg(test)]
mod types_test;
#[cfg(test)]
mod error_test;
#[cfg(test)]
mod config_test;
#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod client_test;
#[cfg(test)]
mod streaming_test;
#[cfg(test)]
mod advanced_config_test;

// Re-export commonly used types for convenience
pub use client::{Client, RetryConfig, RequestMiddleware, RequestInterceptor, LoggingInterceptor};
pub use config::{ClientBuilder, Config};
pub use error::Error;
pub use streaming::{MessageStream, MessageAccumulator, StreamEvent, ContentDelta, MessageDelta, PartialMessage};
pub use tools::{Tool, ToolBuilder};
pub use types::{
    ChatRequest, ChatRequestBuilder, ContentBlock, ImageMediaType, ImageSource, DocumentSource, DocumentMediaType,
    Message, MessageParam, Model, Role, StopReason, Usage, SystemMessage, CountTokensRequest, TokenCount,
};

// Re-export multimodal utilities for convenience
pub use multimodal::{ImageUtils, DocumentUtils, Base64Utils, MimeUtils, validate_url};

/// Result type alias for the SDK
pub type Result<T> = std::result::Result<T, Error>;
