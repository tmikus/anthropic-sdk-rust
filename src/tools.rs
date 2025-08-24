//! Tool calling functionality for the Anthropic API

use serde::{Deserialize, Serialize};

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool builder
    pub fn new(name: impl Into<String>) -> ToolBuilder {
        ToolBuilder::new(name)
    }
}

/// Builder for creating tools
#[derive(Debug)]
pub struct ToolBuilder {
    name: String,
    description: Option<String>,
    schema: serde_json::Value,
}

impl ToolBuilder {
    /// Create a new tool builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    /// Set the tool description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the JSON schema for the tool input
    pub fn schema_value(mut self, schema: serde_json::Value) -> Self {
        self.schema = schema;
        self
    }

    /// Build the tool
    pub fn build(self) -> Tool {
        Tool {
            name: self.name,
            description: self.description,
            input_schema: self.schema,
        }
    }
}

/// Convenience macro for tool definition
#[macro_export]
macro_rules! tool {
    ($name:expr) => {
        $crate::tools::Tool::new($name).build()
    };
    ($name:expr, $desc:expr) => {
        $crate::tools::Tool::new($name).description($desc).build()
    };
}