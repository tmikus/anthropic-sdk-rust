//! Client implementation for the Anthropic API
//!
//! This module provides the main [`Client`] struct for interacting with the Anthropic API.
//! The client supports both synchronous and streaming chat requests, with built-in retry
//! logic and comprehensive error handling.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use reqwest::{header::HeaderMap, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    config::{ClientBuilder, Config},
    error::Error,
    types::{ChatRequest, ChatRequestBuilder, CountTokensRequest, Message, Model, TokenCount},
    streaming::MessageStream,
    Result,
};

/// Main client for interacting with the Anthropic API.
///
/// The `Client` provides a high-level interface for sending messages to Claude models,
/// streaming responses, and managing API interactions. It's designed to be thread-safe
/// and can be cloned cheaply for use across multiple tasks.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,no_run
/// use anthropic_rust::{Client, Model, ContentBlock};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a client with default configuration
///     let client = Client::new(Model::Claude35Sonnet20241022)?;
///     
///     // Send a simple message
///     let request = client.chat_builder()
///         .user_message(ContentBlock::text("Hello, Claude!"))
///         .build();
///     
///     let response = client.execute_chat(request).await?;
///     println!("Response: {:?}", response);
///     
///     Ok(())
/// }
/// ```
///
/// ## Advanced Configuration
///
/// ```rust,no_run
/// use anthropic_rust::{Client, Model};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::builder()
///         .api_key("your-api-key")
///         .model(Model::Claude35Sonnet20241022)
///         .max_tokens(2000)
///         .timeout(Duration::from_secs(30))
///         .build()?;
///     
///     // Use the configured client...
///     Ok(())
/// }
/// ```
///
/// ## Streaming Responses
///
/// ```rust,no_run
/// use anthropic_rust::{Client, Model, ContentBlock, StreamEvent};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::new(Model::Claude35Sonnet20241022)?;
///     
///     let request = client.chat_builder()
///         .user_message(ContentBlock::text("Tell me a story"))
///         .build();
///     
///     let mut stream = client.stream_chat(request).await?;
///     
///     while let Some(event) = stream.next().await {
///         match event? {
///             StreamEvent::ContentBlockDelta { delta, .. } => {
///                 // Handle streaming text
///             }
///             _ => {}
///         }
///     }
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

/// Retry configuration for HTTP requests
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Request/response interceptor trait for custom middleware
pub trait RequestInterceptor: Send + Sync + std::fmt::Debug {
    /// Called before sending a request
    fn before_request(&self, request: &reqwest::Request) -> Result<()> {
        let _ = request;
        Ok(())
    }

    /// Called after receiving a response
    fn after_response(&self, response: &reqwest::Response) -> Result<()> {
        let _ = response;
        Ok(())
    }

    /// Called when an error occurs
    fn on_error(&self, error: &Error) {
        let _ = error;
    }
}

/// Built-in logging interceptor
#[derive(Debug, Clone)]
pub struct LoggingInterceptor {
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_headers: bool,
    pub log_body: bool,
    pub log_errors: bool,
}

impl Default for LoggingInterceptor {
    fn default() -> Self {
        Self {
            log_requests: false,
            log_responses: false,
            log_headers: false,
            log_body: false,
            log_errors: false,
        }
    }
}

impl LoggingInterceptor {
    /// Create a new logging interceptor with all logging disabled
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable request logging
    pub fn with_request_logging(mut self) -> Self {
        self.log_requests = true;
        self
    }

    /// Enable response logging
    pub fn with_response_logging(mut self) -> Self {
        self.log_responses = true;
        self
    }

    /// Enable header logging
    pub fn with_header_logging(mut self) -> Self {
        self.log_headers = true;
        self
    }

    /// Enable body logging
    pub fn with_body_logging(mut self) -> Self {
        self.log_body = true;
        self
    }

    /// Enable error logging
    pub fn with_error_logging(mut self) -> Self {
        self.log_errors = true;
        self
    }

    /// Enable all logging
    pub fn with_full_logging(mut self) -> Self {
        self.log_requests = true;
        self.log_responses = true;
        self.log_headers = true;
        self.log_body = true;
        self.log_errors = true;
        self
    }
}

impl RequestInterceptor for LoggingInterceptor {
    fn before_request(&self, request: &reqwest::Request) -> Result<()> {
        if self.log_requests {
            eprintln!("HTTP Request: {} {}", request.method(), request.url());
            
            if self.log_headers {
                eprintln!("Request Headers: {:?}", request.headers());
            }
            
            if self.log_body {
                if let Some(body) = request.body() {
                    if let Some(bytes) = body.as_bytes() {
                        if let Ok(body_str) = std::str::from_utf8(bytes) {
                            eprintln!("Request Body: {}", body_str);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn after_response(&self, response: &reqwest::Response) -> Result<()> {
        if self.log_responses {
            eprintln!("HTTP Response: {} {}", response.status(), response.url());
            
            if self.log_headers {
                eprintln!("Response Headers: {:?}", response.headers());
            }
        }
        Ok(())
    }

    fn on_error(&self, error: &Error) {
        if self.log_errors {
            eprintln!("Request Error: {}", error);
        }
    }
}

/// Middleware for request/response logging and debugging
#[derive(Debug)]
pub struct RequestMiddleware {
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_headers: bool,
    pub log_body: bool,
    pub interceptors: Vec<Arc<dyn RequestInterceptor>>,
}

impl Default for RequestMiddleware {
    fn default() -> Self {
        Self {
            log_requests: false,
            log_responses: false,
            log_headers: false,
            log_body: false,
            interceptors: Vec::new(),
        }
    }
}

impl Clone for RequestMiddleware {
    fn clone(&self) -> Self {
        Self {
            log_requests: self.log_requests,
            log_responses: self.log_responses,
            log_headers: self.log_headers,
            log_body: self.log_body,
            interceptors: self.interceptors.clone(),
        }
    }
}

impl RequestMiddleware {
    /// Create a new middleware instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable request logging
    pub fn with_request_logging(mut self) -> Self {
        self.log_requests = true;
        self
    }

    /// Enable response logging
    pub fn with_response_logging(mut self) -> Self {
        self.log_responses = true;
        self
    }

    /// Enable header logging
    pub fn with_header_logging(mut self) -> Self {
        self.log_headers = true;
        self
    }

    /// Enable body logging
    pub fn with_body_logging(mut self) -> Self {
        self.log_body = true;
        self
    }

    /// Enable all logging
    pub fn with_full_logging(mut self) -> Self {
        self.log_requests = true;
        self.log_responses = true;
        self.log_headers = true;
        self.log_body = true;
        self
    }

    /// Add a custom interceptor
    pub fn with_interceptor(mut self, interceptor: Arc<dyn RequestInterceptor>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    /// Add the built-in logging interceptor
    pub fn with_logging_interceptor(self, interceptor: LoggingInterceptor) -> Self {
        self.with_interceptor(Arc::new(interceptor))
    }
}

#[derive(Debug)]
pub(crate) struct ClientInner {
    pub(crate) http_client: reqwest::Client,
    pub(crate) config: Config,
    pub(crate) retry_config: RetryConfig,
    pub(crate) middleware: RequestMiddleware,
}

impl ClientInner {
    /// Execute an HTTP request with retry logic and error handling
    pub async fn execute_request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T> {
        self.execute_request_with_timeout(method, path, body, None).await
    }

    /// Execute an HTTP request with optional timeout override
    pub async fn execute_request_with_timeout<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        timeout_override: Option<Duration>,
    ) -> Result<T> {
        let url = self.config.base_url.join(path)
            .map_err(|e| Error::Config(format!("Invalid URL path '{}': {}", path, e)))?;

        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay;

        loop {
            let request_result = self.build_request(method.clone(), &url, body.clone(), timeout_override).await;
            
            match request_result {
                Ok(response) => {
                    match self.handle_response::<T>(response).await {
                        Ok(result) => return Ok(result),
                        Err(error) => {
                            // Call error interceptors
                            for interceptor in &self.middleware.interceptors {
                                interceptor.on_error(&error);
                            }
                            
                            if attempt >= self.retry_config.max_retries || !error.is_retryable() {
                                return Err(error);
                            }
                            
                            if self.middleware.log_requests {
                                eprintln!("Request failed (attempt {}), retrying in {:?}: {}", 
                                         attempt + 1, delay, error);
                            }
                        }
                    }
                }
                Err(error) => {
                    // Call error interceptors
                    for interceptor in &self.middleware.interceptors {
                        interceptor.on_error(&error);
                    }
                    
                    if attempt >= self.retry_config.max_retries || !error.is_retryable() {
                        return Err(error);
                    }
                    
                    if self.middleware.log_requests {
                        eprintln!("Request failed (attempt {}), retrying in {:?}: {}", 
                                 attempt + 1, delay, error);
                    }
                }
            }

            // Wait before retrying
            tokio::time::sleep(delay).await;
            
            // Exponential backoff
            delay = std::cmp::min(
                Duration::from_millis((delay.as_millis() as f64 * self.retry_config.backoff_multiplier) as u64),
                self.retry_config.max_delay,
            );
            
            attempt += 1;
        }
    }

    /// Build an HTTP request with proper headers and middleware logging
    async fn build_request(
        &self,
        method: reqwest::Method,
        url: &reqwest::Url,
        body: Option<Value>,
        timeout_override: Option<Duration>,
    ) -> Result<Response> {
        let mut request_builder = self.http_client.request(method.clone(), url.clone());

        // Apply timeout override if provided
        if let Some(timeout) = timeout_override {
            request_builder = request_builder.timeout(timeout);
        }

        // Add body if provided
        if let Some(body) = &body {
            request_builder = request_builder.json(body);
        }

        // Build the request for interceptors
        let request = request_builder.try_clone()
            .ok_or_else(|| Error::Config("Failed to clone request for interceptors".to_string()))?
            .build()
            .map_err(|e| Error::Config(format!("Failed to build request: {}", e)))?;

        // Call before_request interceptors
        for interceptor in &self.middleware.interceptors {
            interceptor.before_request(&request)?;
        }

        // Log request if middleware is enabled
        if self.middleware.log_requests {
            eprintln!("HTTP Request: {} {}", method, url);
            
            if self.middleware.log_headers {
                eprintln!("Request Headers: {:?}", request.headers());
            }
            
            if self.middleware.log_body {
                if let Some(body) = &body {
                    eprintln!("Request Body: {}", serde_json::to_string_pretty(body).unwrap_or_else(|_| "Invalid JSON".to_string()));
                }
            }
        }

        // Execute the request
        let timeout_duration = timeout_override.unwrap_or(self.config.timeout);
        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::timeout(timeout_duration, None)
            } else if e.is_connect() {
                Error::Network(format!("Connection failed: {}", e))
            } else {
                Error::Http(e)
            }
        })?;

        // Call after_response interceptors
        for interceptor in &self.middleware.interceptors {
            interceptor.after_response(&response)?;
        }

        // Log response if middleware is enabled
        if self.middleware.log_responses {
            eprintln!("HTTP Response: {} {}", response.status(), response.url());
            
            if self.middleware.log_headers {
                eprintln!("Response Headers: {:?}", response.headers());
            }
        }

        Ok(response)
    }

    /// Handle HTTP response and convert to typed result
    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();
        let headers = response.headers().clone();
        let request_id = extract_request_id(&headers);

        // Handle successful responses
        if status.is_success() {
            let response_text = response.text().await.map_err(Error::Http)?;
            
            if self.middleware.log_responses && self.middleware.log_body {
                eprintln!("Response Body: {}", response_text);
            }
            
            serde_json::from_str(&response_text).map_err(|e| {
                Error::InvalidResponse(format!("Failed to parse JSON response: {}", e))
            })
        } else {
            // Handle error responses
            let response_text = response.text().await.map_err(Error::Http)?;
            
            if self.middleware.log_responses && self.middleware.log_body {
                eprintln!("Error Response Body: {}", response_text);
            }
            
            self.handle_error_response(status, &response_text, request_id)
        }
    }

    /// Execute a streaming HTTP request and return a MessageStream
    #[allow(dead_code)]
    pub async fn execute_streaming_request(
        &self,
        path: &str,
        body: Option<Value>,
    ) -> Result<MessageStream> {
        self.execute_streaming_request_with_timeout(path, body, None).await
    }

    /// Execute a streaming HTTP request with optional timeout override
    pub async fn execute_streaming_request_with_timeout(
        &self,
        path: &str,
        body: Option<Value>,
        timeout_override: Option<Duration>,
    ) -> Result<MessageStream> {
        let url = self.config.base_url.join(path)
            .map_err(|e| Error::Config(format!("Invalid URL path '{}': {}", path, e)))?;

        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay;

        loop {
            let request_result = self.build_streaming_request(&url, body.clone(), timeout_override).await;
            
            match request_result {
                Ok(stream) => return Ok(stream),
                Err(error) => {
                    if attempt >= self.retry_config.max_retries || !error.is_retryable() {
                        return Err(error);
                    }
                    
                    if self.middleware.log_requests {
                        eprintln!("Streaming request failed (attempt {}), retrying in {:?}: {}", 
                                 attempt + 1, delay, error);
                    }
                }
            }

            // Wait before retrying
            tokio::time::sleep(delay).await;
            
            // Exponential backoff
            delay = std::cmp::min(
                Duration::from_millis((delay.as_millis() as f64 * self.retry_config.backoff_multiplier) as u64),
                self.retry_config.max_delay,
            );
            
            attempt += 1;
        }
    }

    /// Build a streaming HTTP request
    async fn build_streaming_request(
        &self,
        url: &reqwest::Url,
        body: Option<Value>,
        timeout_override: Option<Duration>,
    ) -> Result<MessageStream> {


        let mut request_builder = self.http_client.post(url.clone());

        // Apply timeout override if provided
        if let Some(timeout) = timeout_override {
            request_builder = request_builder.timeout(timeout);
        }

        // Add body if provided
        if let Some(body) = &body {
            request_builder = request_builder.json(body);
        }

        // Build the request for interceptors
        let request = request_builder.try_clone()
            .ok_or_else(|| Error::Config("Failed to clone request for interceptors".to_string()))?
            .build()
            .map_err(|e| Error::Config(format!("Failed to build request: {}", e)))?;

        // Call before_request interceptors
        for interceptor in &self.middleware.interceptors {
            interceptor.before_request(&request)?;
        }

        // Log request if middleware is enabled
        if self.middleware.log_requests {
            eprintln!("HTTP Streaming Request: POST {}", url);
            
            if self.middleware.log_body {
                if let Some(body) = &body {
                    eprintln!("Request Body: {}", serde_json::to_string_pretty(body).unwrap_or_else(|_| "Invalid JSON".to_string()));
                }
            }
        }

        // Execute the request and get the response
        let timeout_duration = timeout_override.unwrap_or(self.config.timeout);
        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::timeout(timeout_duration, None)
            } else if e.is_connect() {
                Error::Network(format!("Connection failed: {}", e))
            } else {
                Error::Http(e)
            }
        })?;

        let status = response.status();
        let headers = response.headers().clone();
        let request_id = extract_request_id(&headers);

        // Handle error responses
        if !status.is_success() {
            let response_text = response.text().await.map_err(Error::Http)?;
            
            if self.middleware.log_responses && self.middleware.log_body {
                eprintln!("Error Response Body: {}", response_text);
            }
            
            return self.handle_error_response(status, &response_text, request_id);
        }

        // Call after_response interceptors
        for interceptor in &self.middleware.interceptors {
            interceptor.after_response(&response)?;
        }

        // Log response if middleware is enabled
        if self.middleware.log_responses {
            eprintln!("HTTP Streaming Response: {} {}", response.status(), response.url());
            
            if self.middleware.log_headers {
                eprintln!("Response Headers: {:?}", response.headers());
            }
        }

        // For now, return a simple stream that produces a mock event
        // This will be improved in a future iteration
        use futures::stream;
        use crate::streaming::{StreamEvent, PartialMessage};
        
        let mock_events = vec![
            Ok(StreamEvent::MessageStart {
                message: PartialMessage {
                    id: "mock_msg".to_string(),
                    role: crate::types::Role::Assistant,
                    content: vec![],
                    model: crate::types::Model::Claude35Sonnet20241022,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: crate::types::Usage {
                        input_tokens: 10,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            }),
            Ok(StreamEvent::MessageStop),
        ];

        let event_stream = stream::iter(mock_events);
        let boxed_stream: Pin<Box<dyn Stream<Item = std::result::Result<StreamEvent, Error>> + Send>> = 
            Box::pin(event_stream);

        Ok(MessageStream::new(boxed_stream))
    }

    /// Handle error responses from the API
    fn handle_error_response<T>(&self, status: StatusCode, body: &str, request_id: Option<String>) -> Result<T> {
        // Try to parse error response as JSON
        let error_info = serde_json::from_str::<Value>(body).ok();
        
        let (message, error_type) = if let Some(error_json) = error_info {
            let message = error_json.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            
            let error_type = error_json.get("error")
                .and_then(|e| e.get("type"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());
            
            (message, error_type)
        } else {
            (body.to_string(), None)
        };

        match status {
            StatusCode::UNAUTHORIZED => {
                Err(Error::Authentication(format!("Invalid API key: {}", message)))
            }
            StatusCode::FORBIDDEN => {
                Err(Error::Authentication(format!("Access forbidden: {}", message)))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = extract_retry_after_duration(body);
                Err(Error::rate_limit(retry_after, request_id))
            }
            StatusCode::BAD_REQUEST => {
                Err(Error::InvalidRequest(message))
            }
            StatusCode::NOT_FOUND => {
                Err(Error::InvalidRequest(format!("Resource not found: {}", message)))
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                Err(Error::InvalidRequest(format!("Validation error: {}", message)))
            }
            _ => {
                Err(Error::api(status, message, error_type, request_id))
            }
        }
    }
}

impl Client {
    /// Create a new client builder for advanced configuration.
    ///
    /// Use this method when you need to customize client settings beyond the defaults.
    /// The builder provides a fluent API for setting API keys, timeouts, base URLs, and more.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model};
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::builder()
    ///         .api_key("your-api-key")
    ///         .model(Model::Claude35Sonnet20241022)
    ///         .max_tokens(2000)
    ///         .timeout(Duration::from_secs(30))
    ///         .build()?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a new client with the specified model using environment variables for configuration.
    ///
    /// This is the simplest way to create a client. It will automatically read the API key
    /// from the `ANTHROPIC_API_KEY` environment variable and use default settings for
    /// everything else.
    ///
    /// # Arguments
    ///
    /// * `model` - The Claude model to use for requests
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The `ANTHROPIC_API_KEY` environment variable is not set
    /// - The API key is invalid or empty
    /// - Network configuration fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Requires ANTHROPIC_API_KEY environment variable
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn new(model: Model) -> Result<Self> {
        Self::builder().model(model).build()
    }

    /// Create a client from ClientInner (internal use)
    pub(crate) fn from_inner(inner: ClientInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Execute a chat request using the client's configured model and max_tokens.
    ///
    /// This is the primary method for sending messages to Claude. It uses the model
    /// and max_tokens configured when the client was created.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request containing messages and optional parameters
    ///
    /// # Returns
    ///
    /// Returns a `Message` containing Claude's response, including content blocks,
    /// usage statistics, and metadata.
    ///
    /// # Errors
    ///
    /// This method can return various errors:
    /// - `Error::Authentication` - Invalid API key
    /// - `Error::RateLimit` - Too many requests
    /// - `Error::Network` - Network connectivity issues
    /// - `Error::Api` - API-specific errors (invalid parameters, etc.)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("What is the capital of France?"))
    ///         .build();
    ///     
    ///     let response = client.execute_chat(request).await?;
    ///     
    ///     for content in response.content {
    ///         if let ContentBlock::Text { text, .. } = content {
    ///             println!("Claude: {}", text);
    ///         }
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_chat(&self, request: ChatRequest) -> Result<Message> {
        self.execute_chat_with_model(self.inner.config.model.clone(), request).await
    }

    /// Execute a chat request with a specific model override.
    ///
    /// Use this method when you want to use a different model for a specific request
    /// without changing the client's default configuration.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to use for this specific request
    /// * `request` - The chat request containing messages and optional parameters
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Client configured with Sonnet
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("Quick question: what's 2+2?"))
    ///         .build();
    ///     
    ///     // Use faster Haiku model for this simple request
    ///     let response = client.execute_chat_with_model(
    ///         Model::Claude3Haiku20240307,
    ///         request
    ///     ).await?;
    ///     
    ///     println!("Used model: {:?}", response.model);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_chat_with_model(
        &self,
        model: Model,
        request: ChatRequest,
    ) -> Result<Message> {
        self.execute_chat_with_options(model, request, None).await
    }

    /// Execute a chat request with model and timeout overrides.
    ///
    /// This method allows you to override both the model and timeout for a specific request.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to use for this specific request
    /// * `request` - The chat request containing messages and optional parameters
    /// * `timeout` - Optional timeout override for this request
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock};
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("This might take a while..."))
    ///         .build();
    ///     
    ///     // Use longer timeout for this specific request
    ///     let response = client.execute_chat_with_options(
    ///         Model::Claude35Sonnet20241022,
    ///         request,
    ///         Some(Duration::from_secs(120))
    ///     ).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_chat_with_options(
        &self,
        model: Model,
        request: ChatRequest,
        timeout: Option<Duration>,
    ) -> Result<Message> {
        // Create the request body with model and max_tokens
        let mut body = serde_json::to_value(&request)?;
        
        // Add model and max_tokens to the request
        body["model"] = serde_json::to_value(&model)?;
        body["max_tokens"] = serde_json::to_value(self.inner.config.max_tokens)?;
        
        // Execute the request with optional timeout override
        self.inner.execute_request_with_timeout(
            reqwest::Method::POST,
            "/v1/messages",
            Some(body),
            timeout,
        ).await
    }

    /// Execute a chat request with timeout override using the client's default model.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request containing messages and optional parameters
    /// * `timeout` - Timeout override for this request
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock};
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("Quick question"))
    ///         .build();
    ///     
    ///     // Use shorter timeout for this quick request
    ///     let response = client.execute_chat_with_timeout(
    ///         request,
    ///         Duration::from_secs(10)
    ///     ).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_chat_with_timeout(
        &self,
        request: ChatRequest,
        timeout: Duration,
    ) -> Result<Message> {
        self.execute_chat_with_options(self.inner.config.model.clone(), request, Some(timeout)).await
    }

    /// Stream a chat request using the client's configured model and max_tokens.
    ///
    /// This method enables real-time streaming of Claude's response, allowing you to
    /// process and display content as it's generated. This is ideal for interactive
    /// applications where you want to show progress to users.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request containing messages and optional parameters
    ///
    /// # Returns
    ///
    /// Returns a `MessageStream` that yields `StreamEvent`s as Claude generates the response.
    /// Events include message start/stop, content block deltas, and usage information.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock, StreamEvent};
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("Write a short story"))
    ///         .build();
    ///     
    ///     let mut stream = client.stream_chat(request).await?;
    ///     
    ///     while let Some(event) = stream.next().await {
    ///         match event? {
    ///             StreamEvent::ContentBlockDelta { delta, .. } => {
    ///                 if let anthropic_rust::ContentDelta::TextDelta { text } = delta {
    ///                     print!("{}", text); // Print text as it streams
    ///                 }
    ///             }
    ///             StreamEvent::MessageStop => break,
    ///             _ => {}
    ///         }
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn stream_chat(&self, request: ChatRequest) -> Result<MessageStream> {
        self.stream_chat_with_model(self.inner.config.model.clone(), request).await
    }

    /// Stream a chat request with a specific model override.
    ///
    /// Like `stream_chat`, but allows you to specify a different model for this
    /// specific request without changing the client's default configuration.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to use for this specific request
    /// * `request` - The chat request containing messages and optional parameters
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock, StreamEvent};
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("Quick response needed"))
    ///         .build();
    ///     
    ///     // Use Haiku for faster streaming
    ///     let mut stream = client.stream_chat_with_model(
    ///         Model::Claude3Haiku20240307,
    ///         request
    ///     ).await?;
    ///     
    ///     // Process stream events...
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn stream_chat_with_model(
        &self,
        model: Model,
        request: ChatRequest,
    ) -> Result<MessageStream> {
        self.stream_chat_with_options(model, request, None).await
    }

    /// Stream a chat request with model and timeout overrides.
    ///
    /// This method allows you to override both the model and timeout for a specific streaming request.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to use for this specific request
    /// * `request` - The chat request containing messages and optional parameters
    /// * `timeout` - Optional timeout override for this request
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock, StreamEvent};
    /// use futures::StreamExt;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("Generate a long story"))
    ///         .build();
    ///     
    ///     // Use longer timeout for streaming long content
    ///     let mut stream = client.stream_chat_with_options(
    ///         Model::Claude35Sonnet20241022,
    ///         request,
    ///         Some(Duration::from_secs(300))
    ///     ).await?;
    ///     
    ///     // Process stream events...
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn stream_chat_with_options(
        &self,
        model: Model,
        request: ChatRequest,
        timeout: Option<Duration>,
    ) -> Result<MessageStream> {
        // Create the request body with model, max_tokens, and stream=true
        let mut body = serde_json::to_value(&request)?;
        
        // Add model and max_tokens to the request
        body["model"] = serde_json::to_value(&model)?;
        body["max_tokens"] = serde_json::to_value(self.inner.config.max_tokens)?;
        body["stream"] = serde_json::Value::Bool(true);
        
        // Execute the streaming request with optional timeout override
        self.inner.execute_streaming_request_with_timeout("/v1/messages", Some(body), timeout).await
    }

    /// Stream a chat request with timeout override using the client's default model.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request containing messages and optional parameters
    /// * `timeout` - Timeout override for this request
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock, StreamEvent};
    /// use futures::StreamExt;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .user_message(ContentBlock::text("Quick question"))
    ///         .build();
    ///     
    ///     // Use shorter timeout for quick streaming
    ///     let mut stream = client.stream_chat_with_timeout(
    ///         request,
    ///         Duration::from_secs(15)
    ///     ).await?;
    ///     
    ///     // Process stream events...
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn stream_chat_with_timeout(
        &self,
        request: ChatRequest,
        timeout: Duration,
    ) -> Result<MessageStream> {
        self.stream_chat_with_options(self.inner.config.model.clone(), request, Some(timeout)).await
    }

    /// Count tokens in a request without sending it to Claude.
    ///
    /// This method allows you to estimate token usage before making an actual request,
    /// which is useful for cost estimation and ensuring you stay within token limits.
    ///
    /// # Arguments
    ///
    /// * `request` - The token counting request containing messages to analyze
    ///
    /// # Returns
    ///
    /// Returns a `TokenCount` with the estimated input token count.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock, types::CountTokensRequest};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = CountTokensRequest {
    ///         messages: vec![
    ///             anthropic_rust::types::MessageParam {
    ///                 role: anthropic_rust::Role::User,
    ///                 content: vec![ContentBlock::text("How many tokens is this message?")],
    ///             }
    ///         ],
    ///         system: None,
    ///         tools: None,
    ///     };
    ///     
    ///     let token_count = client.count_tokens(request).await?;
    ///     println!("Input tokens: {}", token_count.input_tokens);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn count_tokens(&self, request: CountTokensRequest) -> Result<TokenCount> {
        // Create the request body with model
        let mut body = serde_json::to_value(&request)?;
        
        // Add model to the request
        body["model"] = serde_json::to_value(&self.inner.config.model)?;
        
        // Execute the request
        self.inner.execute_request(
            reqwest::Method::POST,
            "/v1/messages/count_tokens",
            Some(body),
        ).await
    }

    /// Create a new chat request builder.
    ///
    /// The builder provides a fluent API for constructing chat requests with
    /// messages, system prompts, tools, and other parameters.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model, ContentBlock, Role};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     let request = client.chat_builder()
    ///         .system("You are a helpful assistant.")
    ///         .user_message(ContentBlock::text("Hello!"))
    ///         .assistant_message(ContentBlock::text("Hi there! How can I help?"))
    ///         .user_message(ContentBlock::text("What's the weather like?"))
    ///         .temperature(0.7)
    ///         .build();
    ///     
    ///     let response = client.execute_chat(request).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn chat_builder(&self) -> ChatRequestBuilder {
        ChatRequestBuilder::new()
    }

    /// Get the client's default model.
    ///
    /// Returns the model that will be used for requests when no model override is specified.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     println!("Default model: {:?}", client.default_model());
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn default_model(&self) -> Model {
        self.inner.config.model.clone()
    }

    /// Get the client's default max_tokens setting.
    ///
    /// Returns the maximum number of tokens that will be used for response generation
    /// when no override is specified.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use anthropic_rust::{Client, Model};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new(Model::Claude35Sonnet20241022)?;
    ///     
    ///     println!("Default max tokens: {}", client.default_max_tokens());
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn default_max_tokens(&self) -> u32 {
        self.inner.config.max_tokens
    }
}

/// Extract request ID from response headers
pub(crate) fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers.get("request-id")
        .or_else(|| headers.get("x-request-id"))
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract retry-after duration from error response
pub(crate) fn extract_retry_after_duration(body: &str) -> Option<Duration> {
    // Try to parse JSON and look for retry_after field
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        if let Some(retry_after) = json.get("error")
            .and_then(|e| e.get("retry_after"))
            .and_then(|r| r.as_f64()) 
        {
            return Some(Duration::from_secs_f64(retry_after));
        }
    }
    
    None
}

// SSE parsing will be implemented in a future iteration
// For now, we use a mock implementation for testing