//! Client implementation for the Anthropic API

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

/// Main client for interacting with the Anthropic API
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

/// Middleware for request/response logging and debugging
#[derive(Debug, Clone)]
pub struct RequestMiddleware {
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_headers: bool,
    pub log_body: bool,
}

impl Default for RequestMiddleware {
    fn default() -> Self {
        Self {
            log_requests: false,
            log_responses: false,
            log_headers: false,
            log_body: false,
        }
    }
}

impl RequestMiddleware {
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
        let url = self.config.base_url.join(path)
            .map_err(|e| Error::Config(format!("Invalid URL path '{}': {}", path, e)))?;

        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay;

        loop {
            let request_result = self.build_request(method.clone(), &url, body.clone()).await;
            
            match request_result {
                Ok(response) => {
                    match self.handle_response::<T>(response).await {
                        Ok(result) => return Ok(result),
                        Err(error) => {
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
    ) -> Result<Response> {
        let mut request_builder = self.http_client.request(method.clone(), url.clone());

        // Add body if provided
        if let Some(body) = &body {
            request_builder = request_builder.json(body);
        }

        // Log request if middleware is enabled
        if self.middleware.log_requests {
            eprintln!("HTTP Request: {} {}", method, url);
            
            if self.middleware.log_headers {
                if let Some(request) = request_builder.try_clone() {
                    if let Ok(built_request) = request.build() {
                        eprintln!("Request Headers: {:?}", built_request.headers());
                    }
                }
            }
            
            if self.middleware.log_body {
                if let Some(body) = &body {
                    eprintln!("Request Body: {}", serde_json::to_string_pretty(body).unwrap_or_else(|_| "Invalid JSON".to_string()));
                }
            }
        }

        // Execute the request
        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::timeout(self.config.timeout, None)
            } else if e.is_connect() {
                Error::Network(format!("Connection failed: {}", e))
            } else {
                Error::Http(e)
            }
        })?;

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
    pub async fn execute_streaming_request(
        &self,
        path: &str,
        body: Option<Value>,
    ) -> Result<MessageStream> {
        let url = self.config.base_url.join(path)
            .map_err(|e| Error::Config(format!("Invalid URL path '{}': {}", path, e)))?;

        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay;

        loop {
            let request_result = self.build_streaming_request(&url, body.clone()).await;
            
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
    ) -> Result<MessageStream> {


        let mut request_builder = self.http_client.post(url.clone());

        // Add body if provided
        if let Some(body) = &body {
            request_builder = request_builder.json(body);
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
        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::timeout(self.config.timeout, None)
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
    /// Create a new client builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a new client with the specified model using environment variables for configuration
    pub fn new(model: Model) -> Result<Self> {
        Self::builder().model(model).build()
    }

    /// Create a client from ClientInner (internal use)
    pub(crate) fn from_inner(inner: ClientInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Execute a chat request using the client's configured model and max_tokens
    pub async fn execute_chat(&self, request: ChatRequest) -> Result<Message> {
        self.execute_chat_with_model(self.inner.config.model.clone(), request).await
    }

    /// Execute a chat request with a specific model override
    pub async fn execute_chat_with_model(
        &self,
        model: Model,
        request: ChatRequest,
    ) -> Result<Message> {
        // Create the request body with model and max_tokens
        let mut body = serde_json::to_value(&request)?;
        
        // Add model and max_tokens to the request
        body["model"] = serde_json::to_value(&model)?;
        body["max_tokens"] = serde_json::to_value(self.inner.config.max_tokens)?;
        
        // Execute the request
        self.inner.execute_request(
            reqwest::Method::POST,
            "/v1/messages",
            Some(body),
        ).await
    }

    /// Stream a chat request using the client's configured model and max_tokens
    pub async fn stream_chat(&self, request: ChatRequest) -> Result<MessageStream> {
        self.stream_chat_with_model(self.inner.config.model.clone(), request).await
    }

    /// Stream a chat request with a specific model override
    pub async fn stream_chat_with_model(
        &self,
        model: Model,
        request: ChatRequest,
    ) -> Result<MessageStream> {
        // Create the request body with model, max_tokens, and stream=true
        let mut body = serde_json::to_value(&request)?;
        
        // Add model and max_tokens to the request
        body["model"] = serde_json::to_value(&model)?;
        body["max_tokens"] = serde_json::to_value(self.inner.config.max_tokens)?;
        body["stream"] = serde_json::Value::Bool(true);
        
        // Execute the streaming request
        self.inner.execute_streaming_request("/v1/messages", Some(body)).await
    }

    /// Count tokens in a request
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

    /// Create a new chat request builder
    pub fn chat_builder(&self) -> ChatRequestBuilder {
        ChatRequestBuilder::new()
    }

    /// Get the client's default model
    pub fn default_model(&self) -> Model {
        self.inner.config.model.clone()
    }

    /// Get the client's default max_tokens
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