//! Streaming support for the Anthropic API

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
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
    /// Create a new MessageStream from a stream of events
    pub fn new(stream: Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>) -> Self {
        Self { inner: stream }
    }

    /// Create a message accumulator to reconstruct the full message from stream events
    pub fn accumulate(self) -> MessageAccumulator {
        MessageAccumulator::new(self)
    }
}

impl Stream for MessageStream {
    type Item = Result<StreamEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
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
    stream: MessageStream,
    message: Option<Message>,
    content_blocks: Vec<ContentBlock>,
}

impl MessageAccumulator {
    /// Create a new message accumulator from a stream
    pub fn new(stream: MessageStream) -> Self {
        Self {
            stream,
            message: None,
            content_blocks: Vec::new(),
        }
    }

    /// Process the stream and accumulate the final message
    pub async fn accumulate(mut self) -> Result<Message, Error> {
        while let Some(event_result) = self.stream.next().await {
            let event = event_result?;
            self.apply_event(event)?;
        }

        self.message.ok_or_else(|| {
            Error::Stream("Stream ended without producing a complete message".to_string())
        })
    }

    /// Apply a stream event to update the accumulated message
    pub fn apply_event(&mut self, event: StreamEvent) -> Result<(), Error> {
        match event {
            StreamEvent::MessageStart { message } => {
                self.message = Some(Message {
                    id: message.id,
                    role: message.role,
                    content: Vec::new(), // Will be populated from content block events
                    model: message.model,
                    stop_reason: message.stop_reason,
                    stop_sequence: message.stop_sequence,
                    usage: message.usage,
                });
                self.content_blocks.clear();
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                // Ensure we have enough space in the content blocks vector
                while self.content_blocks.len() <= index {
                    self.content_blocks.push(ContentBlock::text(""));
                }
                self.content_blocks[index] = content_block;
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                // Ensure we have enough space in the content blocks vector
                while self.content_blocks.len() <= index {
                    self.content_blocks.push(ContentBlock::text(""));
                }

                // Apply the delta to the content block
                match delta {
                    ContentDelta::TextDelta { text } => {
                        if let ContentBlock::Text {
                            text: existing_text,
                            citations: _,
                        } = &mut self.content_blocks[index]
                        {
                            existing_text.push_str(&text);
                        } else {
                            // If it's not a text block, replace it with a text block
                            self.content_blocks[index] = ContentBlock::Text {
                                text,
                                citations: None,
                            };
                        }
                    }
                }
            }
            StreamEvent::ContentBlockStop { .. } => {
                // Content block is complete, no action needed
            }
            StreamEvent::MessageDelta { delta } => {
                if let Some(ref mut message) = self.message {
                    if let Some(stop_reason) = delta.stop_reason {
                        message.stop_reason = Some(stop_reason);
                    }
                    if let Some(stop_sequence) = delta.stop_sequence {
                        message.stop_sequence = Some(stop_sequence);
                    }
                    if let Some(usage) = delta.usage {
                        message.usage = usage;
                    }
                }
            }
            StreamEvent::MessageStop => {
                // Finalize the message by setting the content blocks
                if let Some(ref mut message) = self.message {
                    message.content = self.content_blocks.clone();
                }
            }
        }

        Ok(())
    }

    /// Get the current accumulated message (may be incomplete)
    pub fn current_message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    /// Get the current content blocks (may be incomplete)
    pub fn current_content_blocks(&self) -> &[ContentBlock] {
        &self.content_blocks
    }
}
