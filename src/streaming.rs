//! Streaming support for the Anthropic API

use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    types::{ContentBlock, Message, Usage},
};

/// Stream of message events
pub struct MessageStream {
    inner: Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>,
}

impl MessageStream {
    /// Create a message accumulator to reconstruct the full message from stream events
    pub fn accumulate(self) -> MessageAccumulator {
        MessageAccumulator::new()
    }
}

/// Events that can be received in a message stream
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        message: PartialMessage,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDelta,
    },
    MessageStop,
}

/// Partial message for stream start events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialMessage {
    pub id: String,
    pub role: crate::types::Role,
    pub content: Vec<ContentBlock>,
    pub model: crate::types::Model,
    pub stop_reason: Option<crate::types::StopReason>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// Content delta for streaming updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
}

/// Message delta for streaming updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<crate::types::StopReason>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

/// Accumulator for reconstructing messages from stream events
pub struct MessageAccumulator {
    message: Option<Message>,
}

impl MessageAccumulator {
    /// Create a new message accumulator
    pub fn new() -> Self {
        Self { message: None }
    }

    /// Apply a stream event to update the accumulated message
    pub fn apply_event(&mut self, event: StreamEvent) -> Result<(), Error> {
        // Implementation will be added in task 9
        todo!("Implementation will be added in task 9")
    }

    /// Get the final accumulated message
    pub fn into_message(self) -> Option<Message> {
        self.message
    }
}

impl Default for MessageAccumulator {
    fn default() -> Self {
        Self::new()
    }
}