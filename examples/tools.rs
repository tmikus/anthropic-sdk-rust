//! Comprehensive tool calling example demonstrating Claude's tool use capabilities
//!
//! This example shows how to:
//! - Define tools with JSON schemas
//! - Handle tool use requests from Claude
//! - Provide tool results back to Claude
//! - Create multi-step tool workflows
//! - Handle tool errors gracefully
//!
//! Run with: cargo run --example tools

use anthropic_rust::{
    Client, Model, ContentBlock, Tool,
};
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Anthropic Rust SDK - Tool Calling Examples ===\n");

    // Create client
    let client = match Client::new(Model::Claude35Sonnet20241022) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  ANTHROPIC_API_KEY not found. Using demo configuration...");
            Client::builder()
                .api_key("demo-key")
                .model(Model::Claude35Sonnet20241022)
                .max_tokens(1500)
                .build()?
        }
    };

    // Example 1: Simple calculator tool
    println!("1. Simple Calculator Tool");
    println!("========================");
    
    let calculator_tool = Tool::new("calculate")
        .description("Perform basic arithmetic calculations")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["operation", "a", "b"]
        }))
        .build();

    let request = client.chat_builder()
        .system("You are a helpful assistant with access to a calculator. Use the calculator tool for any math problems.")
        .user_message(ContentBlock::text("What's 15 * 23 + 7?"))
        .tool(calculator_tool)
        .build();

    match client.execute_chat(request).await {
        Ok(response) => {
            println!("User: What's 15 * 23 + 7?");
            
            for content in &response.content {
                match content {
                    ContentBlock::Text { text, .. } => {
                        println!("Claude: {}", text);
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        println!("🔧 Claude wants to use tool '{}' with input: {}", name, input);
                        
                        // Simulate tool execution
                        let result = execute_calculator_tool(input);
                        println!("📊 Tool result: {}", result);
                        
                        // Continue conversation with tool result
                        let follow_up = client.chat_builder()
                            .system("You are a helpful assistant with access to a calculator.")
                            .user_message(ContentBlock::text("What's 15 * 23 + 7?"))
                            .assistant_message(content.clone())
                            .assistant_message(ContentBlock::tool_result(
                                id.clone(),
                                result.to_string()
                            ))
                            .build();

                        if let Ok(final_response) = client.execute_chat(follow_up).await {
                            if let Some(ContentBlock::Text { text, .. }) = final_response.content.first() {
                                println!("Claude: {}", text);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(e) => println!("❌ Calculator tool example failed: {}", e),
    }

    // Example 2: Weather tool with error handling
    println!("\n2. Weather Tool with Error Handling");
    println!("==================================");
    
    let weather_tool = Tool::new("get_weather")
        .description("Get current weather information for a location")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or location"
                },
                "units": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature units",
                    "default": "celsius"
                }
            },
            "required": ["location"]
        }))
        .build();

    let weather_request = client.chat_builder()
        .system("You are a weather assistant. Use the weather tool to get current conditions.")
        .user_message(ContentBlock::text("What's the weather like in Tokyo and London?"))
        .tool(weather_tool)
        .build();

    match client.execute_chat(weather_request).await {
        Ok(response) => {
            println!("User: What's the weather like in Tokyo and London?");
            
            for content in &response.content {
                match content {
                    ContentBlock::Text { text, .. } => {
                        println!("Claude: {}", text);
                    }
                    ContentBlock::ToolUse { id: _, name, input } => {
                        println!("🌤️  Claude wants to use tool '{}' with input: {}", name, input);
                        
                        // Simulate weather API call
                        let weather_result = get_mock_weather(input);
                        println!("🌡️  Weather result: {}", weather_result);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => println!("❌ Weather tool example failed: {}", e),
    }

    // Example 3: Multiple tools workflow
    println!("\n3. Multiple Tools Workflow");
    println!("=========================");
    
    let search_tool = Tool::new("web_search")
        .description("Search the web for information")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results",
                    "default": 5
                }
            },
            "required": ["query"]
        }))
        .build();

    let summarize_tool = Tool::new("summarize_text")
        .description("Summarize a piece of text")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to summarize"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum length of summary in words",
                    "default": 100
                }
            },
            "required": ["text"]
        }))
        .build();

    let multi_tool_request = client.chat_builder()
        .system("You are a research assistant with access to web search and text summarization tools. Help users find and summarize information.")
        .user_message(ContentBlock::text("Find information about the latest developments in quantum computing and give me a brief summary."))
        .tool(search_tool)
        .tool(summarize_tool)
        .build();

    match client.execute_chat(multi_tool_request).await {
        Ok(response) => {
            println!("User: Find information about quantum computing and summarize it.");
            
            for content in &response.content {
                match content {
                    ContentBlock::Text { text, .. } => {
                        println!("Claude: {}", text);
                    }
                    ContentBlock::ToolUse { id: _, name, input } => {
                        println!("🔍 Claude wants to use tool '{}' with input: {}", name, input);
                        
                        match name.as_str() {
                            "web_search" => {
                                let search_results = mock_web_search(input);
                                println!("📄 Search results: {}", search_results);
                            }
                            "summarize_text" => {
                                let summary = mock_summarize(input);
                                println!("📝 Summary: {}", summary);
                            }
                            _ => {
                                println!("❓ Unknown tool: {}", name);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(e) => println!("❌ Multi-tool workflow failed: {}", e),
    }

    // Example 4: Tool with complex schema (file operations)
    println!("\n4. Complex Tool Schema - File Operations");
    println!("=======================================");
    
    let file_tool = Tool::new("file_operations")
        .description("Perform file system operations")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list", "delete", "create_dir"],
                    "description": "File operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write (for write operations)"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "recursive": {
                            "type": "boolean",
                            "description": "Recursive operation",
                            "default": false
                        },
                        "create_parents": {
                            "type": "boolean",
                            "description": "Create parent directories if they don't exist",
                            "default": false
                        }
                    }
                }
            },
            "required": ["operation", "path"]
        }))
        .build();

    let file_request = client.chat_builder()
        .system("You are a file management assistant. Use the file_operations tool to help with file system tasks. Always be careful with file operations.")
        .user_message(ContentBlock::text("Can you list the files in the current directory?"))
        .tool(file_tool)
        .build();

    match client.execute_chat(file_request).await {
        Ok(response) => {
            println!("User: Can you list the files in the current directory?");
            
            for content in &response.content {
                match content {
                    ContentBlock::Text { text, .. } => {
                        println!("Claude: {}", text);
                    }
                    ContentBlock::ToolUse { id: _, name, input } => {
                        println!("📁 Claude wants to use tool '{}' with input: {}", name, input);
                        
                        let file_result = mock_file_operation(input);
                        println!("📋 File operation result: {}", file_result);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => println!("❌ File operations example failed: {}", e),
    }

    // Example 5: Tool error handling
    println!("\n5. Tool Error Handling");
    println!("=====================");
    
    let api_tool = Tool::new("api_call")
        .description("Make API calls to external services")
        .schema_value(json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "API endpoint URL"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE"],
                    "description": "HTTP method"
                },
                "headers": {
                    "type": "object",
                    "description": "HTTP headers"
                },
                "body": {
                    "type": "string",
                    "description": "Request body"
                }
            },
            "required": ["url", "method"]
        }))
        .build();

    let api_request = client.chat_builder()
        .system("You are an API testing assistant. Use the api_call tool to make HTTP requests. Handle errors gracefully.")
        .user_message(ContentBlock::text("Can you check if the GitHub API is working by calling https://api.github.com/users/octocat?"))
        .tool(api_tool)
        .build();

    match client.execute_chat(api_request).await {
        Ok(response) => {
            println!("User: Check if the GitHub API is working...");
            
            for content in &response.content {
                match content {
                    ContentBlock::Text { text, .. } => {
                        println!("Claude: {}", text);
                    }
                    ContentBlock::ToolUse { id: _, name, input } => {
                        println!("🌐 Claude wants to use tool '{}' with input: {}", name, input);
                        
                        // Simulate API call with potential error
                        let api_result = mock_api_call(input);
                        println!("📡 API result: {}", api_result);
                        
                        // Show how to handle tool errors
                        if api_result.contains("error") {
                            println!("⚠️  Tool returned an error - Claude should handle this gracefully");
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(e) => println!("❌ API tool example failed: {}", e),
    }

    println!("\n=== Tool Calling Examples Complete ===");
    println!("💡 Try running with a valid ANTHROPIC_API_KEY to see real tool interactions!");
    
    Ok(())
}

// Mock tool implementations for demonstration

fn execute_calculator_tool(input: &Value) -> f64 {
    let operation = input["operation"].as_str().unwrap_or("add");
    let a = input["a"].as_f64().unwrap_or(0.0);
    let b = input["b"].as_f64().unwrap_or(0.0);
    
    match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => if b != 0.0 { a / b } else { f64::NAN },
        _ => 0.0,
    }
}

fn get_mock_weather(input: &Value) -> String {
    let location = input["location"].as_str().unwrap_or("Unknown");
    let units = input["units"].as_str().unwrap_or("celsius");
    
    // Mock weather data
    let weather_data = HashMap::from([
        ("Tokyo", ("Sunny", 22)),
        ("London", ("Cloudy", 15)),
        ("New York", ("Rainy", 18)),
        ("Sydney", ("Partly Cloudy", 25)),
    ]);
    
    if let Some((condition, temp)) = weather_data.get(location) {
        let temp_str = if units == "fahrenheit" {
            format!("{}°F", temp * 9 / 5 + 32)
        } else {
            format!("{}°C", temp)
        };
        format!("{}: {} {}", location, condition, temp_str)
    } else {
        format!("Weather data not available for {}", location)
    }
}

fn mock_web_search(input: &Value) -> String {
    let query = input["query"].as_str().unwrap_or("");
    let max_results = input["max_results"].as_i64().unwrap_or(5);
    
    format!(
        "Found {} results for '{}': [Mock results would include recent articles about quantum computing breakthroughs, new quantum processors, and research developments]",
        max_results, query
    )
}

fn mock_summarize(input: &Value) -> String {
    let _text = input["text"].as_str().unwrap_or("");
    let max_length = input["max_length"].as_i64().unwrap_or(100);
    
    format!(
        "Summary (max {} words): Recent quantum computing developments include advances in error correction, new qubit technologies, and increased computational capabilities. Major tech companies are making significant investments in quantum research.",
        max_length
    )
}

fn mock_file_operation(input: &Value) -> String {
    let operation = input["operation"].as_str().unwrap_or("list");
    let path = input["path"].as_str().unwrap_or(".");
    
    match operation {
        "list" => format!("Files in '{}': Cargo.toml, src/, examples/, README.md, .gitignore", path),
        "read" => format!("Content of '{}': [File content would be displayed here]", path),
        "write" => format!("Successfully wrote to '{}'", path),
        "delete" => format!("Successfully deleted '{}'", path),
        "create_dir" => format!("Successfully created directory '{}'", path),
        _ => format!("Unknown operation: {}", operation),
    }
}

fn mock_api_call(input: &Value) -> String {
    let url = input["url"].as_str().unwrap_or("");
    let method = input["method"].as_str().unwrap_or("GET");
    
    if url.contains("github.com") {
        format!(
            "{} {}: {{\"login\": \"octocat\", \"id\": 1, \"name\": \"The Octocat\", \"public_repos\": 8}}",
            method, url
        )
    } else {
        format!("{} {}: {{\"error\": \"Service unavailable\"}}", method, url)
    }
}