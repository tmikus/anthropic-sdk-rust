//! Tool calling functionality for the Anthropic API
//!
//! This module uses the `serde_json::json!` macro which requires serde_json >= 1.0.39.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;

/// Tool definition for function calling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool builder
    pub fn builder(name: impl Into<String>) -> ToolBuilder {
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

    /// Set the JSON schema from a type that implements JsonSchema (requires schemars feature)
    #[cfg(feature = "schemars")]
    pub fn schema<T: JsonSchema>(mut self) -> Self {
        let schema = schemars::schema_for!(T);
        self.schema = serde_json::to_value(schema).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        });
        self
    }

    /// Add a property to the schema
    pub fn property(
        mut self,
        name: impl Into<String>,
        property_type: impl Into<String>,
        description: Option<impl Into<String>>,
        required: bool,
    ) -> Self {
        let name = name.into();
        let property_type = property_type.into();

        // Ensure we have a proper object schema
        if !self.schema.is_object()
            || self.schema.get("type") != Some(&serde_json::Value::String("object".to_string()))
        {
            self.schema = serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            });
        }

        // Add the property
        let mut property_def = serde_json::json!({
            "type": property_type
        });

        if let Some(desc) = description {
            property_def["description"] = serde_json::Value::String(desc.into());
        }

        self.schema["properties"][&name] = property_def;

        // Add to required array if needed
        if required {
            if let Some(required_array) = self.schema["required"].as_array_mut() {
                if !required_array.iter().any(|v| v.as_str() == Some(&name)) {
                    required_array.push(serde_json::Value::String(name));
                }
            }
        }

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
///
/// # Examples
///
/// ```rust
/// use anthropic_rust::tool;
///
/// // Simple tool with just a name
/// let tool1 = tool!("calculator");
///
/// // Tool with name and description
/// let tool2 = tool!("calculator", "A simple calculator tool");
///
/// // Tool with schema using schemars (requires schemars feature)
/// #[cfg(feature = "schemars")]
/// {
///     use anthropic_rust::tool_with_schema;
///     use serde::{Deserialize, Serialize};
///     use schemars::JsonSchema;
///     
///     #[derive(Serialize, Deserialize, JsonSchema)]
///     struct CalculatorInput {
///         operation: String,
///         a: f64,
///         b: f64,
///     }
///     
///     let tool3 = tool_with_schema!("calculator", "A calculator tool", CalculatorInput);
/// }
/// ```
#[macro_export]
macro_rules! tool {
    ($name:expr) => {
        $crate::tools::Tool::builder($name).build()
    };
    ($name:expr, $desc:expr) => {
        $crate::tools::Tool::builder($name)
            .description($desc)
            .build()
    };
}

/// Convenience macro for tool definition with schemars support
///
/// This macro is only available when the `schemars` feature is enabled.
#[cfg(feature = "schemars")]
#[macro_export]
macro_rules! tool_with_schema {
    ($name:expr, $desc:expr, $schema_type:ty) => {
        $crate::tools::Tool::builder($name)
            .description($desc)
            .schema::<$schema_type>()
            .build()
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_creation_basic() {
        let tool = Tool::builder("calculator").build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, None);
        assert_eq!(tool.input_schema["type"], "object");
        assert_eq!(tool.input_schema["properties"], json!({}));
        assert_eq!(tool.input_schema["required"], json!([]));
    }

    #[test]
    fn test_tool_creation_with_description() {
        let tool = Tool::builder("calculator")
            .description("A simple calculator tool")
            .build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(
            tool.description,
            Some("A simple calculator tool".to_string())
        );
    }

    #[test]
    fn test_tool_builder_with_custom_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "The operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First operand"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand"
                }
            },
            "required": ["operation", "a", "b"]
        });

        let tool = Tool::builder("calculator")
            .description("A calculator tool")
            .schema_value(schema.clone())
            .build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, Some("A calculator tool".to_string()));
        assert_eq!(tool.input_schema, schema);
    }

    #[test]
    fn test_tool_builder_with_properties() {
        let tool = Tool::builder("calculator")
            .description("A calculator tool")
            .property(
                "operation",
                "string",
                Some("The operation to perform"),
                true,
            )
            .property("a", "number", Some("First operand"), true)
            .property("b", "number", Some("Second operand"), true)
            .property("precision", "integer", Some("Decimal precision"), false)
            .build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, Some("A calculator tool".to_string()));

        let schema = &tool.input_schema;
        assert_eq!(schema["type"], "object");

        // Check properties
        let properties = &schema["properties"];
        assert_eq!(properties["operation"]["type"], "string");
        assert_eq!(
            properties["operation"]["description"],
            "The operation to perform"
        );
        assert_eq!(properties["a"]["type"], "number");
        assert_eq!(properties["a"]["description"], "First operand");
        assert_eq!(properties["b"]["type"], "number");
        assert_eq!(properties["b"]["description"], "Second operand");
        assert_eq!(properties["precision"]["type"], "integer");
        assert_eq!(properties["precision"]["description"], "Decimal precision");

        // Check required fields
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("operation")));
        assert!(required.contains(&json!("a")));
        assert!(required.contains(&json!("b")));
        assert!(!required.contains(&json!("precision")));
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool::builder("calculator")
            .description("A calculator tool")
            .property(
                "operation",
                "string",
                Some("The operation to perform"),
                true,
            )
            .property("a", "number", None::<String>, true)
            .build();

        let json = serde_json::to_string(&tool).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["name"], "calculator");
        assert_eq!(parsed["description"], "A calculator tool");
        assert_eq!(parsed["input_schema"]["type"], "object");
        assert_eq!(
            parsed["input_schema"]["properties"]["operation"]["type"],
            "string"
        );
        assert_eq!(parsed["input_schema"]["properties"]["a"]["type"], "number");
    }

    #[test]
    fn test_tool_deserialization() {
        let json = json!({
            "name": "weather",
            "description": "Get weather information",
            "input_schema": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The location to get weather for"
                    }
                },
                "required": ["location"]
            }
        });

        let tool: Tool = serde_json::from_value(json).unwrap();

        assert_eq!(tool.name, "weather");
        assert_eq!(
            tool.description,
            Some("Get weather information".to_string())
        );
        assert_eq!(
            tool.input_schema["properties"]["location"]["type"],
            "string"
        );
    }

    #[test]
    fn test_tool_macro_basic() {
        let tool = tool!("test_tool");
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, None);
    }

    #[test]
    fn test_tool_macro_with_description() {
        let tool = tool!("test_tool", "A test tool");
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, Some("A test tool".to_string()));
    }

    #[cfg(feature = "schemars")]
    #[test]
    fn test_tool_with_schemars() {
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, JsonSchema)]
        struct CalculatorInput {
            /// The mathematical operation to perform
            operation: String,
            /// First operand
            a: f64,
            /// Second operand  
            b: f64,
            /// Optional precision for the result
            #[serde(skip_serializing_if = "Option::is_none")]
            precision: Option<u32>,
        }

        let tool = Tool::builder("calculator")
            .description("A calculator tool")
            .schema::<CalculatorInput>()
            .build();

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, Some("A calculator tool".to_string()));

        let schema = &tool.input_schema;
        assert_eq!(schema["type"], "object");

        // The exact structure depends on schemars version, but we can check basic properties
        let properties = &schema["properties"];
        assert!(properties.get("operation").is_some());
        assert!(properties.get("a").is_some());
        assert!(properties.get("b").is_some());
        assert!(properties.get("precision").is_some());
    }

    #[cfg(feature = "schemars")]
    #[test]
    fn test_tool_macro_with_schemars() {
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, JsonSchema)]
        struct WeatherInput {
            location: String,
            units: Option<String>,
        }

        let tool = tool_with_schema!("weather", "Get weather information", WeatherInput);

        assert_eq!(tool.name, "weather");
        assert_eq!(
            tool.description,
            Some("Get weather information".to_string())
        );

        let schema = &tool.input_schema;
        assert_eq!(schema["type"], "object");

        let properties = &schema["properties"];
        assert!(properties.get("location").is_some());
        assert!(properties.get("units").is_some());
    }

    #[test]
    fn test_tool_builder_property_without_description() {
        let tool = Tool::builder("simple_tool")
            .property("param1", "string", None::<String>, true)
            .property("param2", "number", None::<String>, false)
            .build();

        let schema = &tool.input_schema;
        let properties = &schema["properties"];

        // Properties should exist but without description
        assert_eq!(properties["param1"]["type"], "string");
        assert!(properties["param1"].get("description").is_none());
        assert_eq!(properties["param2"]["type"], "number");
        assert!(properties["param2"].get("description").is_none());

        // Check required array
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("param1")));
        assert!(!required.contains(&json!("param2")));
    }

    #[test]
    fn test_tool_builder_overwrite_schema() {
        let tool = Tool::builder("test_tool")
            .property("initial", "string", None::<String>, true)
            .schema_value(json!({
                "type": "object",
                "properties": {
                    "new_prop": {
                        "type": "boolean"
                    }
                },
                "required": ["new_prop"]
            }))
            .build();

        let schema = &tool.input_schema;
        let properties = &schema["properties"];

        // Should have the new schema, not the initial property
        assert!(properties.get("initial").is_none());
        assert_eq!(properties["new_prop"]["type"], "boolean");

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("new_prop")));
        assert!(!required.contains(&json!("initial")));
    }

    #[test]
    fn test_tool_builder_add_property_to_custom_schema() {
        let tool = Tool::builder("test_tool")
            .schema_value(json!({
                "type": "object",
                "properties": {
                    "existing": {
                        "type": "string"
                    }
                },
                "required": ["existing"]
            }))
            .property("new_prop", "number", Some("A new property"), true)
            .build();

        let schema = &tool.input_schema;
        let properties = &schema["properties"];

        // Should have both properties
        assert_eq!(properties["existing"]["type"], "string");
        assert_eq!(properties["new_prop"]["type"], "number");
        assert_eq!(properties["new_prop"]["description"], "A new property");

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("existing")));
        assert!(required.contains(&json!("new_prop")));
    }

    #[test]
    fn test_tool_builder_duplicate_required_property() {
        let tool = Tool::builder("test_tool")
            .property("param", "string", None::<String>, true)
            .property("param", "number", Some("Updated param"), true) // Same name, should update
            .build();

        let schema = &tool.input_schema;
        let properties = &schema["properties"];

        // Should have updated property
        assert_eq!(properties["param"]["type"], "number");
        assert_eq!(properties["param"]["description"], "Updated param");

        // Should only appear once in required array
        let required = schema["required"].as_array().unwrap();
        let param_count = required
            .iter()
            .filter(|v| v.as_str() == Some("param"))
            .count();
        assert_eq!(param_count, 1);
    }
}
