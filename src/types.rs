//! Core types and data models for the Anthropic API.
//!
//! This module contains all the request and response types used when interacting
//! with the Anthropic API, including message structures, content blocks, and
//! configuration enums.

use serde::{Deserialize, Serialize};
use url::Url;

/// Available Claude models with their capabilities and token limits.
///
/// Each model has different strengths, speeds, and costs. Choose the model that
/// best fits your use case:
///
/// - **Haiku**: Fastest and most cost-effective for simple tasks
/// - **Sonnet**: Balanced performance for most applications  
/// - **Opus**: Most capable for complex reasoning tasks
///
/// # Examples
///
/// ```rust
/// use anthropic::Model;
///
/// // Get the maximum tokens for a model
/// let max_tokens = Model::Claude35Sonnet20241022.max_tokens();
/// println!("Max tokens: {}", max_tokens);
///
/// // Compare models
/// assert_eq!(Model::Claude3Haiku20240307.max_tokens(), 200_000);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Model {
    #[serde(rename = "claude-3-haiku-20240307")]
    Claude3Haiku20240307,
    #[serde(rename = "claude-3-sonnet-20240229")]
    Claude3Sonnet20240229,
    #[serde(rename = "claude-3-opus-20240229")]
    Claude3Opus20240229,
    #[serde(rename = "claude-3-5-sonnet-20241022")]
    Claude35Sonnet20241022,
    #[serde(rename = "claude-3-5-sonnet-20250114")]
    Claude35Sonnet20250114,
    #[serde(rename = "claude-4-sonnet-20250514")]
    Claude4Sonnet20250514,
}

impl Model {
    /// Returns the maximum tokens supported by this model
    pub fn max_tokens(&self) -> u32 {
        match self {
            Model::Claude3Haiku20240307 => 200_000,
            Model::Claude3Sonnet20240229 => 200_000,
            Model::Claude3Opus20240229 => 200_000,
            Model::Claude35Sonnet20241022 => 200_000,
            Model::Claude35Sonnet20250114 => 200_000,
            Model::Claude4Sonnet20250514 => 200_000,
        }
    }
}

/// Message role indicating who sent the message.
///
/// In a conversation, messages alternate between `User` (human) and `Assistant` (Claude).
/// The conversation must always start with a `User` message.
///
/// # Examples
///
/// ```rust
/// use anthropic::{Role, MessageParam, ContentBlock};
///
/// let user_message = MessageParam {
///     role: Role::User,
///     content: vec![ContentBlock::text("Hello!")],
/// };
///
/// let assistant_message = MessageParam {
///     role: Role::Assistant,
///     content: vec![ContentBlock::text("Hi there!")],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Message from the human user
    User,
    /// Message from Claude (the assistant)
    Assistant,
}

/// Reason why Claude stopped generating tokens.
///
/// This indicates how the message generation ended, which can be useful for
/// understanding whether the response was complete or truncated.
///
/// # Examples
///
/// ```rust
/// use anthropic::StopReason;
///
/// // Check if the response was complete
/// let stop_reason = StopReason::EndTurn;
/// match stop_reason {
///     StopReason::EndTurn => println!("Response completed naturally"),
///     StopReason::MaxTokens => println!("Response was truncated due to token limit"),
///     StopReason::StopSequence => println!("Response stopped at a stop sequence"),
///     StopReason::ToolUse => println!("Response ended to use a tool"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Claude finished its response naturally
    EndTurn,
    /// Response was truncated due to max_tokens limit
    MaxTokens,
    /// Response stopped at a configured stop sequence
    StopSequence,
    /// Claude wants to use a tool
    ToolUse,
}

/// Token usage information for a request/response.
///
/// This provides detailed information about token consumption, including
/// input tokens (from your messages) and output tokens (from Claude's response).
/// Cache-related fields are included when using prompt caching features.
///
/// # Examples
///
/// ```rust
/// use anthropic::Usage;
///
/// let usage = Usage {
///     input_tokens: 50,
///     output_tokens: 100,
///     cache_creation_input_tokens: None,
///     cache_read_input_tokens: None,
/// };
///
/// let total_tokens = usage.input_tokens + usage.output_tokens;
/// println!("Total tokens used: {}", total_tokens);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// Number of input tokens (from your messages)
    pub input_tokens: u32,
    /// Number of output tokens (from Claude's response)
    pub output_tokens: u32,
    /// Tokens used for cache creation (when using prompt caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Tokens read from cache (when using prompt caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// Content block types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<Citation>>,
    },
    Image {
        source: ImageSource,
    },
    Document {
        source: DocumentSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ContentBlock>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl ContentBlock {
    /// Create a text content block
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
            citations: None,
        }
    }

    /// Create an image content block from base64 data
    pub fn image_base64(media_type: ImageMediaType, data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                media_type,
                data: data.into(),
            },
        }
    }

    /// Create an image content block from URL
    pub fn image_url(url: impl TryInto<Url>) -> Result<Self, crate::Error> {
        let url = url
            .try_into()
            .map_err(|_| crate::Error::Config("Invalid image URL".to_string()))?;
        Ok(Self::Image {
            source: ImageSource::Url { url },
        })
    }

    /// Create a tool use content block
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: impl Serialize,
    ) -> Result<Self, crate::Error> {
        Ok(Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input: serde_json::to_value(input)?,
        })
    }

    /// Create a document content block from base64 data
    pub fn document_base64(media_type: DocumentMediaType, data: impl Into<String>) -> Self {
        Self::Document {
            source: DocumentSource::Base64 {
                media_type,
                data: data.into(),
            },
        }
    }

    /// Create a document content block from URL
    pub fn document_url(url: impl TryInto<Url>) -> Result<Self, crate::Error> {
        let url = url
            .try_into()
            .map_err(|_| crate::Error::Config("Invalid document URL".to_string()))?;
        Ok(Self::Document {
            source: DocumentSource::Url { url },
        })
    }

    /// Create a tool result content block
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: vec![Self::text(content.into())],
            is_error: None,
        }
    }
}

/// Image source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 {
        media_type: ImageMediaType,
        data: String,
    },
    Url {
        url: Url,
    },
}

/// Document source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    Base64 {
        media_type: DocumentMediaType,
        data: String,
    },
    Url {
        url: Url,
    },
}

/// Supported image media types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageMediaType {
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(rename = "image/webp")]
    WebP,
}

/// Supported document media types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentMediaType {
    #[serde(rename = "application/pdf")]
    Pdf,
    #[serde(rename = "text/plain")]
    Text,
}

/// Citation information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub start_index: u32,
    pub end_index: u32,
    pub source: String,
}

/// Message parameter for requests
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageParam {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

/// Complete message response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: Model,
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// System message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: String,
}

/// Chat request structure
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatRequest {
    pub messages: Vec<MessageParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::tools::Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Builder for chat requests
#[derive(Debug, Default)]
pub struct ChatRequestBuilder {
    messages: Vec<MessageParam>,
    system: Option<Vec<SystemMessage>>,
    tools: Option<Vec<crate::tools::Tool>>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop_sequences: Option<Vec<String>>,
}

impl ChatRequestBuilder {
    /// Create a new chat request builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message with specified role and content
    pub fn message(mut self, role: Role, content: ContentBlock) -> Self {
        self.messages.push(MessageParam {
            role,
            content: vec![content],
        });
        self
    }

    /// Add a message with specified role and multiple content blocks
    pub fn message_with_content(mut self, role: Role, content: Vec<ContentBlock>) -> Self {
        self.messages.push(MessageParam { role, content });
        self
    }

    /// Add multiple messages at once
    pub fn messages(mut self, messages: Vec<MessageParam>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Add a user message
    pub fn user_message(self, content: ContentBlock) -> Self {
        self.message(Role::User, content)
    }

    /// Add an assistant message
    pub fn assistant_message(self, content: ContentBlock) -> Self {
        self.message(Role::Assistant, content)
    }

    /// Add a system message
    pub fn system(mut self, content: impl Into<String>) -> Self {
        let system_msg = SystemMessage {
            message_type: "text".to_string(),
            text: content.into(),
        };
        self.system.get_or_insert_with(Vec::new).push(system_msg);
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: crate::tools::Tool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set top_p
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Add stop sequence
    pub fn stop_sequence(mut self, sequence: impl Into<String>) -> Self {
        self.stop_sequences
            .get_or_insert_with(Vec::new)
            .push(sequence.into());
        self
    }

    /// Add multiple stop sequences
    pub fn stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences
            .get_or_insert_with(Vec::new)
            .extend(sequences);
        self
    }

    /// Add multiple tools
    pub fn tools(mut self, tools: Vec<crate::tools::Tool>) -> Self {
        self.tools.get_or_insert_with(Vec::new).extend(tools);
        self
    }

    /// Build the chat request
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            messages: self.messages,
            system: self.system,
            tools: self.tools,
            temperature: self.temperature,
            top_p: self.top_p,
            stop_sequences: self.stop_sequences,
        }
    }
}

/// Token counting request
#[derive(Debug, Clone, Serialize)]
pub struct CountTokensRequest {
    pub messages: Vec<MessageParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::tools::Tool>>,
}

impl From<ChatRequest> for CountTokensRequest {
    /// Convert a ChatRequest to a CountTokensRequest
    /// This is useful for counting tokens in a message before sending it
    fn from(chat_request: ChatRequest) -> Self {
        Self {
            messages: chat_request.messages,
            system: chat_request.system,
            tools: chat_request.tools,
        }
    }
}

/// Token count response
#[derive(Debug, Clone, Deserialize)]
pub struct TokenCount {
    pub input_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_model_serialization() {
        let model = Model::Claude35Sonnet20241022;
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(json, "\"claude-3-5-sonnet-20241022\"");
    }

    #[test]
    fn test_model_deserialization() {
        let json = "\"claude-3-5-sonnet-20241022\"";
        let model: Model = serde_json::from_str(json).unwrap();
        assert_eq!(model, Model::Claude35Sonnet20241022);
    }

    #[test]
    fn test_model_max_tokens() {
        assert_eq!(Model::Claude3Haiku20240307.max_tokens(), 200_000);
        assert_eq!(Model::Claude3Sonnet20240229.max_tokens(), 200_000);
        assert_eq!(Model::Claude3Opus20240229.max_tokens(), 200_000);
        assert_eq!(Model::Claude35Sonnet20241022.max_tokens(), 200_000);
        assert_eq!(Model::Claude35Sonnet20250114.max_tokens(), 200_000);
        assert_eq!(Model::Claude4Sonnet20250514.max_tokens(), 200_000);
    }

    #[test]
    fn test_role_serialization() {
        let user_role = Role::User;
        let assistant_role = Role::Assistant;
        
        assert_eq!(serde_json::to_string(&user_role).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&assistant_role).unwrap(), "\"assistant\"");
    }

    #[test]
    fn test_role_deserialization() {
        let user_json = "\"user\"";
        let assistant_json = "\"assistant\"";
        
        let user_role: Role = serde_json::from_str(user_json).unwrap();
        let assistant_role: Role = serde_json::from_str(assistant_json).unwrap();
        
        assert_eq!(user_role, Role::User);
        assert_eq!(assistant_role, Role::Assistant);
    }

    #[test]
    fn test_stop_reason_serialization() {
        let reasons = vec![
            (StopReason::EndTurn, "\"end_turn\""),
            (StopReason::MaxTokens, "\"max_tokens\""),
            (StopReason::StopSequence, "\"stop_sequence\""),
            (StopReason::ToolUse, "\"tool_use\""),
        ];

        for (reason, expected_json) in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            assert_eq!(json, expected_json);
        }
    }

    #[test]
    fn test_stop_reason_deserialization() {
        let reasons = vec![
            ("\"end_turn\"", StopReason::EndTurn),
            ("\"max_tokens\"", StopReason::MaxTokens),
            ("\"stop_sequence\"", StopReason::StopSequence),
            ("\"tool_use\"", StopReason::ToolUse),
        ];

        for (json, expected_reason) in reasons {
            let reason: StopReason = serde_json::from_str(json).unwrap();
            assert_eq!(reason, expected_reason);
        }
    }

    #[test]
    fn test_usage_serialization() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: Some(10),
            cache_read_input_tokens: None,
        };

        let json = serde_json::to_string(&usage).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["input_tokens"], 100);
        assert_eq!(parsed["output_tokens"], 50);
        assert_eq!(parsed["cache_creation_input_tokens"], 10);
        assert!(parsed.get("cache_read_input_tokens").is_none());
    }

    #[test]
    fn test_usage_deserialization() {
        let json = r#"{
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 10
        }"#;

        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.cache_creation_input_tokens, Some(10));
        assert_eq!(usage.cache_read_input_tokens, None);
    }

    #[test]
    fn test_content_block_text_serialization() {
        let text_block = ContentBlock::Text {
            text: "Hello, world!".to_string(),
            citations: None,
        };

        let json = serde_json::to_string(&text_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "text");
        assert_eq!(parsed["text"], "Hello, world!");
        assert!(parsed.get("citations").is_none());
    }

    #[test]
    fn test_content_block_text_with_citations() {
        let citation = Citation {
            start_index: 0,
            end_index: 5,
            source: "example.com".to_string(),
        };

        let text_block = ContentBlock::Text {
            text: "Hello, world!".to_string(),
            citations: Some(vec![citation]),
        };

        let json = serde_json::to_string(&text_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "text");
        assert_eq!(parsed["text"], "Hello, world!");
        assert!(parsed["citations"].is_array());
        assert_eq!(parsed["citations"][0]["start_index"], 0);
        assert_eq!(parsed["citations"][0]["end_index"], 5);
        assert_eq!(parsed["citations"][0]["source"], "example.com");
    }

    #[test]
    fn test_content_block_image_serialization() {
        let image_block = ContentBlock::Image {
            source: ImageSource::Base64 {
                media_type: ImageMediaType::Png,
                data: "base64data".to_string(),
            },
        };

        let json = serde_json::to_string(&image_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "image");
        assert_eq!(parsed["source"]["type"], "base64");
        assert_eq!(parsed["source"]["media_type"], "image/png");
        assert_eq!(parsed["source"]["data"], "base64data");
    }

    #[test]
    fn test_content_block_tool_use_serialization() {
        let tool_use_block = ContentBlock::ToolUse {
            id: "tool_123".to_string(),
            name: "calculator".to_string(),
            input: serde_json::json!({"operation": "add", "a": 1, "b": 2}),
        };

        let json = serde_json::to_string(&tool_use_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "tool_use");
        assert_eq!(parsed["id"], "tool_123");
        assert_eq!(parsed["name"], "calculator");
        assert_eq!(parsed["input"]["operation"], "add");
        assert_eq!(parsed["input"]["a"], 1);
        assert_eq!(parsed["input"]["b"], 2);
    }

    #[test]
    fn test_content_block_tool_result_serialization() {
        let tool_result_block = ContentBlock::ToolResult {
            tool_use_id: "tool_123".to_string(),
            content: vec![ContentBlock::text("Result: 3")],
            is_error: Some(false),
        };

        let json = serde_json::to_string(&tool_result_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "tool_result");
        assert_eq!(parsed["tool_use_id"], "tool_123");
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Result: 3");
        assert_eq!(parsed["is_error"], false);
    }

    #[test]
    fn test_content_block_deserialization() {
        let json = r#"{
            "type": "text",
            "text": "Hello, world!"
        }"#;

        let content_block: ContentBlock = serde_json::from_str(json).unwrap();
        match content_block {
            ContentBlock::Text { text, citations } => {
                assert_eq!(text, "Hello, world!");
                assert_eq!(citations, None);
            }
            _ => panic!("Expected text content block"),
        }
    }

    #[test]
    fn test_image_media_type_serialization() {
        let media_types = vec![
            (ImageMediaType::Jpeg, "\"image/jpeg\""),
            (ImageMediaType::Png, "\"image/png\""),
            (ImageMediaType::Gif, "\"image/gif\""),
            (ImageMediaType::WebP, "\"image/webp\""),
        ];

        for (media_type, expected_json) in media_types {
            let json = serde_json::to_string(&media_type).unwrap();
            assert_eq!(json, expected_json);
        }
    }

    #[test]
    fn test_document_media_type_serialization() {
        let media_types = vec![
            (DocumentMediaType::Pdf, "\"application/pdf\""),
            (DocumentMediaType::Text, "\"text/plain\""),
        ];

        for (media_type, expected_json) in media_types {
            let json = serde_json::to_string(&media_type).unwrap();
            assert_eq!(json, expected_json);
        }
    }

    #[test]
    fn test_message_param_serialization() {
        let message_param = MessageParam {
            role: Role::User,
            content: vec![ContentBlock::text("Hello!")],
        };

        let json = serde_json::to_string(&message_param).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["role"], "user");
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Hello!");
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{
            "id": "msg_123",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello there!"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        }"#;

        let message: Message = serde_json::from_str(json).unwrap();
        assert_eq!(message.id, "msg_123");
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.model, Model::Claude35Sonnet20241022);
        assert_eq!(message.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
        
        match &message.content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Hello there!"),
            _ => panic!("Expected text content block"),
        }
    }

    #[test]
    fn test_system_message_serialization() {
        let system_msg = SystemMessage {
            message_type: "text".to_string(),
            text: "You are a helpful assistant.".to_string(),
        };

        let json = serde_json::to_string(&system_msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "text");
        assert_eq!(parsed["text"], "You are a helpful assistant.");
    }

    #[test]
    fn test_chat_request_serialization() {
        let chat_request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Hello!")],
            }],
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "Be helpful.".to_string(),
            }]),
            tools: None,
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: Some(vec!["STOP".to_string()]),
        };

        let json = serde_json::to_string(&chat_request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["messages"][0]["role"], "user");
        assert_eq!(parsed["system"][0]["text"], "Be helpful.");
        assert_eq!(parsed["temperature"], 0.7);
        assert_eq!(parsed["stop_sequences"][0], "STOP");
        assert!(parsed.get("top_p").is_none());
        assert!(parsed.get("tools").is_none());
    }

    #[test]
    fn test_count_tokens_request_serialization() {
        let count_request = CountTokensRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Count my tokens!")],
            }],
            system: None,
            tools: None,
        };

        let json = serde_json::to_string(&count_request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["messages"][0]["role"], "user");
        assert_eq!(parsed["messages"][0]["content"][0]["text"], "Count my tokens!");
        assert!(parsed.get("system").is_none());
        assert!(parsed.get("tools").is_none());
    }

    #[test]
    fn test_count_tokens_request_from_chat_request() {
        let chat_request = ChatRequest {
            messages: vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Convert me!")],
            }],
            system: Some(vec![SystemMessage {
                message_type: "text".to_string(),
                text: "System message".to_string(),
            }]),
            tools: None,
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_sequences: Some(vec!["STOP".to_string()]),
        };

        // Test From trait implementation
        let count_request = CountTokensRequest::from(chat_request.clone());
        
        assert_eq!(count_request.messages.len(), 1);
        assert_eq!(count_request.messages[0].role, Role::User);
        assert!(count_request.system.is_some());
        assert_eq!(count_request.system.as_ref().unwrap()[0].text, "System message");
        assert!(count_request.tools.is_none());

        // Test that temperature, top_p, and stop_sequences are not included
        let serialized = serde_json::to_value(&count_request).unwrap();
        assert!(serialized.get("temperature").is_none());
        assert!(serialized.get("top_p").is_none());
        assert!(serialized.get("stop_sequences").is_none());

        // Test using into() syntax
        let count_request2: CountTokensRequest = chat_request.into();
        assert_eq!(count_request2.messages.len(), 1);
        assert!(count_request2.system.is_some());
    }

    #[test]
    fn test_token_count_deserialization() {
        let json = r#"{
            "input_tokens": 42
        }"#;

        let token_count: TokenCount = serde_json::from_str(json).unwrap();
        assert_eq!(token_count.input_tokens, 42);
    }

    #[test]
    fn test_content_block_convenience_constructors() {
        // Test text constructor
        let text_block = ContentBlock::text("Hello!");
        match text_block {
            ContentBlock::Text { text, citations } => {
                assert_eq!(text, "Hello!");
                assert_eq!(citations, None);
            }
            _ => panic!("Expected text content block"),
        }

        // Test image base64 constructor
        let image_block = ContentBlock::image_base64(ImageMediaType::Png, "data123");
        match image_block {
            ContentBlock::Image { source } => match source {
                ImageSource::Base64 { media_type, data } => {
                    assert_eq!(media_type, ImageMediaType::Png);
                    assert_eq!(data, "data123");
                }
                _ => panic!("Expected base64 image source"),
            },
            _ => panic!("Expected image content block"),
        }

        // Test image URL constructor
        let url = "https://example.com/image.png";
        let image_block = ContentBlock::image_url(url).unwrap();
        match image_block {
            ContentBlock::Image { source } => match source {
                ImageSource::Url { url } => {
                    assert_eq!(url.as_str(), "https://example.com/image.png");
                }
                _ => panic!("Expected URL image source"),
            },
            _ => panic!("Expected image content block"),
        }

        // Test document base64 constructor
        let doc_block = ContentBlock::document_base64(DocumentMediaType::Pdf, "pdf_data123");
        match doc_block {
            ContentBlock::Document { source } => match source {
                DocumentSource::Base64 { media_type, data } => {
                    assert_eq!(media_type, DocumentMediaType::Pdf);
                    assert_eq!(data, "pdf_data123");
                }
                _ => panic!("Expected base64 document source"),
            },
            _ => panic!("Expected document content block"),
        }

        // Test document URL constructor
        let doc_url = "https://example.com/document.pdf";
        let doc_block = ContentBlock::document_url(doc_url).unwrap();
        match doc_block {
            ContentBlock::Document { source } => match source {
                DocumentSource::Url { url } => {
                    assert_eq!(url.as_str(), "https://example.com/document.pdf");
                }
                _ => panic!("Expected URL document source"),
            },
            _ => panic!("Expected document content block"),
        }

        // Test tool use constructor
        let tool_block = ContentBlock::tool_use("id123", "calculator", serde_json::json!({"a": 1})).unwrap();
        match tool_block {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "id123");
                assert_eq!(name, "calculator");
                assert_eq!(input["a"], 1);
            }
            _ => panic!("Expected tool use content block"),
        }

        // Test tool result constructor
        let result_block = ContentBlock::tool_result("id123", "Result: 42");
        match result_block {
            ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                assert_eq!(tool_use_id, "id123");
                assert_eq!(is_error, None);
                match &content[0] {
                    ContentBlock::Text { text, .. } => assert_eq!(text, "Result: 42"),
                    _ => panic!("Expected text content in tool result"),
                }
            }
            _ => panic!("Expected tool result content block"),
        }
    }

    #[test]
    fn test_chat_request_builder_basic() {
        let request = ChatRequestBuilder::new()
            .user_message(ContentBlock::text("Hello!"))
            .assistant_message(ContentBlock::text("Hi there!"))
            .system("Be helpful")
            .temperature(0.8)
            .top_p(0.9)
            .stop_sequence("STOP")
            .build();

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[1].role, Role::Assistant);
        assert!(request.system.is_some());
        assert_eq!(request.system.as_ref().unwrap()[0].text, "Be helpful");
        assert_eq!(request.temperature, Some(0.8));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.stop_sequences.as_ref().unwrap()[0], "STOP");
    }

    #[test]
    fn test_chat_request_builder_empty() {
        let request = ChatRequestBuilder::new().build();

        assert_eq!(request.messages.len(), 0);
        assert!(request.system.is_none());
        assert!(request.tools.is_none());
        assert!(request.temperature.is_none());
        assert!(request.top_p.is_none());
        assert!(request.stop_sequences.is_none());
    }

    #[test]
    fn test_chat_request_builder_message_with_content() {
        let content_blocks = vec![
            ContentBlock::text("Hello!"),
            ContentBlock::image_base64(ImageMediaType::Png, "base64data"),
        ];

        let request = ChatRequestBuilder::new()
            .message_with_content(Role::User, content_blocks.clone())
            .build();

        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[0].content.len(), 2);
        
        match &request.messages[0].content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Hello!"),
            _ => panic!("Expected text content block"),
        }
        
        match &request.messages[0].content[1] {
            ContentBlock::Image { .. } => {},
            _ => panic!("Expected image content block"),
        }
    }

    #[test]
    fn test_chat_request_builder_multiple_messages() {
        let messages = vec![
            MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("First message")],
            },
            MessageParam {
                role: Role::Assistant,
                content: vec![ContentBlock::text("Second message")],
            },
        ];

        let request = ChatRequestBuilder::new()
            .messages(messages.clone())
            .user_message(ContentBlock::text("Third message"))
            .build();

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[1].role, Role::Assistant);
        assert_eq!(request.messages[2].role, Role::User);
        
        match &request.messages[0].content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "First message"),
            _ => panic!("Expected text content block"),
        }
        
        match &request.messages[2].content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Third message"),
            _ => panic!("Expected text content block"),
        }
    }

    #[test]
    fn test_chat_request_builder_multiple_system_messages() {
        let request = ChatRequestBuilder::new()
            .system("First system message")
            .system("Second system message")
            .user_message(ContentBlock::text("Hello!"))
            .build();

        assert_eq!(request.messages.len(), 1);
        assert!(request.system.is_some());
        assert_eq!(request.system.as_ref().unwrap().len(), 2);
        assert_eq!(request.system.as_ref().unwrap()[0].text, "First system message");
        assert_eq!(request.system.as_ref().unwrap()[1].text, "Second system message");
    }

    #[test]
    fn test_chat_request_builder_multiple_tools() {
        use crate::tools::Tool;
        
        let tool1 = Tool::new("calculator").description("A calculator tool").build();
        let tool2 = Tool::new("weather").description("A weather tool").build();
        let tool3 = Tool::new("search").build();

        let request = ChatRequestBuilder::new()
            .tool(tool1.clone())
            .tool(tool2.clone())
            .tools(vec![tool3.clone()])
            .user_message(ContentBlock::text("Hello!"))
            .build();

        assert_eq!(request.messages.len(), 1);
        assert!(request.tools.is_some());
        assert_eq!(request.tools.as_ref().unwrap().len(), 3);
        assert_eq!(request.tools.as_ref().unwrap()[0].name, "calculator");
        assert_eq!(request.tools.as_ref().unwrap()[1].name, "weather");
        assert_eq!(request.tools.as_ref().unwrap()[2].name, "search");
    }

    #[test]
    fn test_chat_request_builder_multiple_stop_sequences() {
        let request = ChatRequestBuilder::new()
            .stop_sequence("STOP")
            .stop_sequence("END")
            .stop_sequences(vec!["HALT".to_string(), "QUIT".to_string()])
            .user_message(ContentBlock::text("Hello!"))
            .build();

        assert_eq!(request.messages.len(), 1);
        assert!(request.stop_sequences.is_some());
        assert_eq!(request.stop_sequences.as_ref().unwrap().len(), 4);
        assert_eq!(request.stop_sequences.as_ref().unwrap()[0], "STOP");
        assert_eq!(request.stop_sequences.as_ref().unwrap()[1], "END");
        assert_eq!(request.stop_sequences.as_ref().unwrap()[2], "HALT");
        assert_eq!(request.stop_sequences.as_ref().unwrap()[3], "QUIT");
    }

    #[test]
    fn test_chat_request_builder_parameter_validation() {
        // Test temperature bounds (should be between 0.0 and 1.0 in practice, but we don't enforce this in the builder)
        let request = ChatRequestBuilder::new()
            .temperature(0.0)
            .user_message(ContentBlock::text("Hello!"))
            .build();
        assert_eq!(request.temperature, Some(0.0));

        let request = ChatRequestBuilder::new()
            .temperature(1.0)
            .user_message(ContentBlock::text("Hello!"))
            .build();
        assert_eq!(request.temperature, Some(1.0));

        // Test top_p bounds (should be between 0.0 and 1.0 in practice, but we don't enforce this in the builder)
        let request = ChatRequestBuilder::new()
            .top_p(0.0)
            .user_message(ContentBlock::text("Hello!"))
            .build();
        assert_eq!(request.top_p, Some(0.0));

        let request = ChatRequestBuilder::new()
            .top_p(1.0)
            .user_message(ContentBlock::text("Hello!"))
            .build();
        assert_eq!(request.top_p, Some(1.0));
    }

    #[test]
    fn test_chat_request_builder_fluent_chaining() {
        // Test that all methods return Self for fluent chaining
        let request = ChatRequestBuilder::new()
            .user_message(ContentBlock::text("Hello!"))
            .assistant_message(ContentBlock::text("Hi!"))
            .message(Role::User, ContentBlock::text("How are you?"))
            .message_with_content(Role::Assistant, vec![ContentBlock::text("I'm good!")])
            .messages(vec![MessageParam {
                role: Role::User,
                content: vec![ContentBlock::text("Great!")],
            }])
            .system("Be helpful")
            .temperature(0.7)
            .top_p(0.9)
            .stop_sequence("STOP")
            .stop_sequences(vec!["END".to_string()])
            .build();

        assert_eq!(request.messages.len(), 5);
        assert!(request.system.is_some());
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.stop_sequences.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_chat_request_builder_with_multimodal_content() {
        let request = ChatRequestBuilder::new()
            .user_message(ContentBlock::text("What's in this image?"))
            .message_with_content(Role::User, vec![
                ContentBlock::text("Please analyze this document:"),
                ContentBlock::document_base64(DocumentMediaType::Pdf, "pdf_data"),
                ContentBlock::image_url("https://example.com/image.jpg").unwrap(),
            ])
            .build();

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[1].content.len(), 3);
        
        match &request.messages[1].content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Please analyze this document:"),
            _ => panic!("Expected text content block"),
        }
        
        match &request.messages[1].content[1] {
            ContentBlock::Document { .. } => {},
            _ => panic!("Expected document content block"),
        }
        
        match &request.messages[1].content[2] {
            ContentBlock::Image { .. } => {},
            _ => panic!("Expected image content block"),
        }
    }

    #[test]
    fn test_chat_request_builder_with_tool_use() {
        use crate::tools::Tool;
        
        let calculator_tool = Tool::new("calculator")
            .description("Perform mathematical calculations")
            .schema_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }))
            .build();

        let tool_use_content = ContentBlock::tool_use(
            "tool_123",
            "calculator",
            serde_json::json!({"expression": "2 + 2"})
        ).unwrap();

        let tool_result_content = ContentBlock::tool_result("tool_123", "4");

        let request = ChatRequestBuilder::new()
            .user_message(ContentBlock::text("What is 2 + 2?"))
            .tool(calculator_tool)
            .assistant_message(tool_use_content)
            .user_message(tool_result_content)
            .build();

        assert_eq!(request.messages.len(), 3);
        assert!(request.tools.is_some());
        assert_eq!(request.tools.as_ref().unwrap().len(), 1);
        assert_eq!(request.tools.as_ref().unwrap()[0].name, "calculator");
        
        // Check tool use message
        match &request.messages[1].content[0] {
            ContentBlock::ToolUse { id, name, .. } => {
                assert_eq!(id, "tool_123");
                assert_eq!(name, "calculator");
            },
            _ => panic!("Expected tool use content block"),
        }
        
        // Check tool result message
        match &request.messages[2].content[0] {
            ContentBlock::ToolResult { tool_use_id, .. } => {
                assert_eq!(tool_use_id, "tool_123");
            },
            _ => panic!("Expected tool result content block"),
        }
    }

    #[test]
    fn test_invalid_image_url() {
        let result = ContentBlock::image_url("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_citation_serialization() {
        let citation = Citation {
            start_index: 10,
            end_index: 20,
            source: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&citation).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["start_index"], 10);
        assert_eq!(parsed["end_index"], 20);
        assert_eq!(parsed["source"], "https://example.com");
    }

    #[test]
    fn test_document_source_serialization() {
        let doc_source = DocumentSource::Base64 {
            media_type: DocumentMediaType::Pdf,
            data: "pdf_data".to_string(),
        };

        let json = serde_json::to_string(&doc_source).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "base64");
        assert_eq!(parsed["media_type"], "application/pdf");
        assert_eq!(parsed["data"], "pdf_data");
    }

    #[test]
    fn test_image_source_url_serialization() {
        let image_source = ImageSource::Url {
            url: "https://example.com/image.jpg".parse().unwrap(),
        };

        let json = serde_json::to_string(&image_source).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "url");
        assert_eq!(parsed["url"], "https://example.com/image.jpg");
    }

    #[test]
    fn test_content_block_document_serialization() {
        let doc_block = ContentBlock::Document {
            source: DocumentSource::Base64 {
                media_type: DocumentMediaType::Pdf,
                data: "pdf_base64_data".to_string(),
            },
        };

        let json = serde_json::to_string(&doc_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "document");
        assert_eq!(parsed["source"]["type"], "base64");
        assert_eq!(parsed["source"]["media_type"], "application/pdf");
        assert_eq!(parsed["source"]["data"], "pdf_base64_data");
    }

    #[test]
    fn test_content_block_document_url_serialization() {
        let doc_block = ContentBlock::Document {
            source: DocumentSource::Url {
                url: "https://example.com/doc.pdf".parse().unwrap(),
            },
        };

        let json = serde_json::to_string(&doc_block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "document");
        assert_eq!(parsed["source"]["type"], "url");
        assert_eq!(parsed["source"]["url"], "https://example.com/doc.pdf");
    }

    #[test]
    fn test_document_source_deserialization() {
        let json = r#"{
            "type": "base64",
            "media_type": "text/plain",
            "data": "text_content"
        }"#;

        let doc_source: DocumentSource = serde_json::from_str(json).unwrap();
        match doc_source {
            DocumentSource::Base64 { media_type, data } => {
                assert_eq!(media_type, DocumentMediaType::Text);
                assert_eq!(data, "text_content");
            }
            _ => panic!("Expected base64 document source"),
        }
    }

    #[test]
    fn test_image_source_deserialization() {
        let json = r#"{
            "type": "base64",
            "media_type": "image/jpeg",
            "data": "jpeg_data"
        }"#;

        let image_source: ImageSource = serde_json::from_str(json).unwrap();
        match image_source {
            ImageSource::Base64 { media_type, data } => {
                assert_eq!(media_type, ImageMediaType::Jpeg);
                assert_eq!(data, "jpeg_data");
            }
            _ => panic!("Expected base64 image source"),
        }
    }

    #[test]
    fn test_content_block_document_deserialization() {
        let json = r#"{
            "type": "document",
            "source": {
                "type": "url",
                "url": "https://example.com/document.pdf"
            }
        }"#;

        let content_block: ContentBlock = serde_json::from_str(json).unwrap();
        match content_block {
            ContentBlock::Document { source } => match source {
                DocumentSource::Url { url } => {
                    assert_eq!(url.as_str(), "https://example.com/document.pdf");
                }
                _ => panic!("Expected URL document source"),
            },
            _ => panic!("Expected document content block"),
        }
    }

    #[test]
    fn test_invalid_document_url() {
        let result = ContentBlock::document_url("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_image_media_types() {
        let media_types = vec![
            ImageMediaType::Jpeg,
            ImageMediaType::Png,
            ImageMediaType::Gif,
            ImageMediaType::WebP,
        ];

        for media_type in media_types {
            let block = ContentBlock::image_base64(media_type.clone(), "test_data");
            match block {
                ContentBlock::Image { source } => match source {
                    ImageSource::Base64 { media_type: mt, .. } => {
                        assert_eq!(mt, media_type);
                    }
                    _ => panic!("Expected base64 image source"),
                },
                _ => panic!("Expected image content block"),
            }
        }
    }

    #[test]
    fn test_all_document_media_types() {
        let media_types = vec![
            DocumentMediaType::Pdf,
            DocumentMediaType::Text,
        ];

        for media_type in media_types {
            let block = ContentBlock::document_base64(media_type.clone(), "test_data");
            match block {
                ContentBlock::Document { source } => match source {
                    DocumentSource::Base64 { media_type: mt, .. } => {
                        assert_eq!(mt, media_type);
                    }
                    _ => panic!("Expected base64 document source"),
                },
                _ => panic!("Expected document content block"),
            }
        }
    }

    #[test]
    fn test_tool_result_with_error() {
        let tool_result = ContentBlock::ToolResult {
            tool_use_id: "tool_123".to_string(),
            content: vec![ContentBlock::text("Error occurred")],
            is_error: Some(true),
        };

        let json = serde_json::to_string(&tool_result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["type"], "tool_result");
        assert_eq!(parsed["tool_use_id"], "tool_123");
        assert_eq!(parsed["is_error"], true);
        assert_eq!(parsed["content"][0]["text"], "Error occurred");
    }

    #[test]
    fn test_complex_content_block_combinations() {
        // Test a message with multiple content block types
        let message_param = MessageParam {
            role: Role::User,
            content: vec![
                ContentBlock::text("Here's an image and a document:"),
                ContentBlock::image_base64(ImageMediaType::Png, "image_data"),
                ContentBlock::document_base64(DocumentMediaType::Pdf, "pdf_data"),
            ],
        };

        let json = serde_json::to_string(&message_param).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["role"], "user");
        assert_eq!(parsed["content"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][1]["type"], "image");
        assert_eq!(parsed["content"][2]["type"], "document");
    }
}