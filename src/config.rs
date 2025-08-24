//! Configuration and builder patterns for the Anthropic client

use std::time::Duration;
use url::Url;

use std::sync::Arc;

use crate::{
    client::{Client, ClientInner, RetryConfig, RequestMiddleware, RequestInterceptor, LoggingInterceptor},
    error::Error,
    types::Model,
    Result,
};

/// Configuration for the Anthropic client
#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: Url,
    pub timeout: Duration,
    pub max_retries: u32,
    pub model: Model,
    pub max_tokens: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.anthropic.com"
                .parse()
                .expect("Default base URL should be valid"),
            timeout: Duration::from_secs(60),
            max_retries: 3,
            model: Model::Claude35Sonnet20241022,
            max_tokens: 4096,
        }
    }
}

impl Config {
    /// Validate the configuration parameters
    pub fn validate(&self) -> Result<()> {
        // Validate API key
        if self.api_key.is_empty() {
            return Err(Error::Config("API key cannot be empty".to_string()));
        }

        // Validate API key format (should start with 'sk-ant-')
        if !self.api_key.starts_with("sk-ant-") {
            return Err(Error::Config(
                "API key must start with 'sk-ant-'".to_string(),
            ));
        }

        // Validate timeout
        if self.timeout.is_zero() {
            return Err(Error::Config("Timeout must be greater than zero".to_string()));
        }

        // Validate max_tokens against model limits
        let model_max_tokens = self.model.max_tokens();
        if self.max_tokens > model_max_tokens {
            return Err(Error::Config(format!(
                "max_tokens ({}) exceeds model limit ({}) for {:?}",
                self.max_tokens, model_max_tokens, self.model
            )));
        }

        if self.max_tokens == 0 {
            return Err(Error::Config("max_tokens must be greater than zero".to_string()));
        }

        // Validate base URL scheme
        if self.base_url.scheme() != "https" && self.base_url.scheme() != "http" {
            return Err(Error::Config(format!(
                "Base URL must use http or https scheme, got: {}",
                self.base_url.scheme()
            )));
        }

        Ok(())
    }
}

/// Builder for creating Anthropic clients
#[derive(Debug, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<Url>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    http_client: Option<reqwest::Client>,
    model: Option<Model>,
    max_tokens: Option<u32>,
    retry_config: Option<RetryConfig>,
    middleware: Option<RequestMiddleware>,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl TryInto<Url>) -> Result<Self> {
        self.base_url = Some(url.try_into().map_err(|_| Error::Config("Invalid base URL".to_string()))?);
        Ok(self)
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set a custom HTTP client
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Set the default model
    pub fn model(mut self, model: Model) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the default max tokens
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set custom retry configuration
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }

    /// Set request middleware for logging and debugging
    pub fn middleware(mut self, middleware: RequestMiddleware) -> Self {
        self.middleware = Some(middleware);
        self
    }

    /// Enable request logging
    pub fn with_logging(mut self) -> Self {
        let middleware = self.middleware.take().unwrap_or_default();
        self.middleware = Some(middleware.with_full_logging());
        self
    }

    /// Add a custom request interceptor
    pub fn with_interceptor(mut self, interceptor: Arc<dyn RequestInterceptor>) -> Self {
        let middleware = self.middleware.take().unwrap_or_default();
        self.middleware = Some(middleware.with_interceptor(interceptor));
        self
    }

    /// Add the built-in logging interceptor with custom configuration
    pub fn with_logging_interceptor(mut self, interceptor: LoggingInterceptor) -> Self {
        let middleware = self.middleware.take().unwrap_or_default();
        self.middleware = Some(middleware.with_logging_interceptor(interceptor));
        self
    }

    /// Build the client
    pub fn build(self) -> Result<Client> {
        let mut config = Config::default();

        // Set API key from builder or environment variables
        config.api_key = self
            .api_key
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .or_else(|| std::env::var("CLAUDE_API_KEY").ok()) // Alternative env var
            .ok_or_else(|| {
                Error::Config(
                    "API key not provided. Set via builder.api_key() or environment variables ANTHROPIC_API_KEY or CLAUDE_API_KEY".to_string(),
                )
            })?;

        // Set base URL from builder or environment variables
        if let Some(base_url) = self.base_url {
            config.base_url = base_url;
        } else if let Ok(env_url) = std::env::var("ANTHROPIC_BASE_URL") {
            config.base_url = env_url.parse().map_err(|_| {
                Error::Config(format!(
                    "Invalid base URL in ANTHROPIC_BASE_URL environment variable: {}",
                    env_url
                ))
            })?;
        }

        // Set other configuration values
        if let Some(timeout) = self.timeout {
            config.timeout = timeout;
        }
        if let Some(max_retries) = self.max_retries {
            config.max_retries = max_retries;
        }
        if let Some(model) = self.model {
            config.model = model;
        }
        if let Some(max_tokens) = self.max_tokens {
            config.max_tokens = max_tokens;
        }

        // Validate the configuration
        config.validate()?;

        // Create HTTP client with proper configuration
        let http_client = self.http_client.unwrap_or_else(|| {
            let mut builder = reqwest::Client::builder()
                .timeout(config.timeout)
                .user_agent(format!("anthropic-rust-sdk/{}", env!("CARGO_PKG_VERSION")));

            // Add default headers
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "x-api-key",
                reqwest::header::HeaderValue::from_str(&config.api_key)
                    .expect("API key should be valid header value"),
            );
            headers.insert(
                "anthropic-version",
                reqwest::header::HeaderValue::from_static("2023-06-01"),
            );
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );

            builder = builder.default_headers(headers);

            builder.build().expect("Failed to create HTTP client")
        });

        // Handle retry configuration - if retry_config is explicitly set, use it
        // Otherwise, create one from max_retries if set
        let retry_config = if let Some(retry_config) = self.retry_config {
            retry_config
        } else {
            let mut default_retry = RetryConfig::default();
            if let Some(max_retries) = self.max_retries {
                default_retry.max_retries = max_retries;
            }
            default_retry
        };

        let inner = ClientInner {
            http_client,
            config,
            retry_config,
            middleware: self.middleware.unwrap_or_default(),
        };

        Ok(Client::from_inner(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.api_key, "");
        assert_eq!(config.base_url.as_str(), "https://api.anthropic.com/");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.model, Model::Claude35Sonnet20241022);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_config_validation_empty_api_key() {
        let config = Config {
            api_key: String::new(),
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key cannot be empty"));
    }

    #[test]
    fn test_config_validation_invalid_api_key_format() {
        let config = Config {
            api_key: "invalid-key".to_string(),
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key must start with 'sk-ant-'"));
    }

    #[test]
    fn test_config_validation_valid_api_key() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_zero_timeout() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            timeout: Duration::from_secs(0),
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Timeout must be greater than zero"));
    }

    #[test]
    fn test_config_validation_zero_max_tokens() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            max_tokens: 0,
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_tokens must be greater than zero"));
    }

    #[test]
    fn test_config_validation_max_tokens_exceeds_model_limit() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            model: Model::Claude3Haiku20240307,
            max_tokens: 300_000, // Exceeds model limit of 200_000
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds model limit"));
    }

    #[test]
    fn test_config_validation_invalid_url_scheme() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            base_url: "ftp://invalid.com".parse().unwrap(),
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Base URL must use http or https scheme"));
    }

    #[test]
    fn test_config_validation_valid_http_scheme() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            base_url: "http://localhost:8080".parse().unwrap(),
            ..Config::default()
        };
        
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_builder_new() {
        let builder = ClientBuilder::new();
        assert!(builder.api_key.is_none());
        assert!(builder.base_url.is_none());
        assert!(builder.timeout.is_none());
        assert!(builder.max_retries.is_none());
        assert!(builder.http_client.is_none());
        assert!(builder.model.is_none());
        assert!(builder.max_tokens.is_none());
    }

    #[test]
    fn test_client_builder_fluent_api() {
        let builder = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .timeout(Duration::from_secs(30))
            .max_retries(5)
            .model(Model::Claude3Haiku20240307)
            .max_tokens(1000);

        assert_eq!(builder.api_key.as_ref().unwrap(), "sk-ant-api03-test-key");
        assert_eq!(builder.timeout.unwrap(), Duration::from_secs(30));
        assert_eq!(builder.max_retries.unwrap(), 5);
        assert_eq!(builder.model.unwrap(), Model::Claude3Haiku20240307);
        assert_eq!(builder.max_tokens.unwrap(), 1000);
    }

    #[test]
    fn test_client_builder_base_url_valid() {
        let builder = ClientBuilder::new()
            .base_url("https://custom.api.com")
            .unwrap();

        assert_eq!(
            builder.base_url.as_ref().unwrap().as_str(),
            "https://custom.api.com/"
        );
    }

    #[test]
    fn test_client_builder_base_url_invalid() {
        let result = ClientBuilder::new().base_url("not-a-url");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid base URL"));
    }

    #[test]
    fn test_client_builder_build_with_api_key() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .build();

        assert!(result.is_ok());
    }

    // Test environment variable functionality by temporarily setting and unsetting them
    #[test]
    fn test_client_builder_env_vars() {
        // Save original values
        let original_anthropic = env::var("ANTHROPIC_API_KEY").ok();
        let original_claude = env::var("CLAUDE_API_KEY").ok();
        let original_base_url = env::var("ANTHROPIC_BASE_URL").ok();

        // Clear all env vars first
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("CLAUDE_API_KEY");
        env::remove_var("ANTHROPIC_BASE_URL");

        // Test missing API key
        let result = ClientBuilder::new().build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key not provided"));

        // Test ANTHROPIC_API_KEY
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-env-key");
        let result = ClientBuilder::new().build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-env-key");

        // Test CLAUDE_API_KEY (when ANTHROPIC_API_KEY is not set)
        env::remove_var("ANTHROPIC_API_KEY");
        env::set_var("CLAUDE_API_KEY", "sk-ant-api03-claude-key");
        let result = ClientBuilder::new().build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-claude-key");

        // Test ANTHROPIC_BASE_URL
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-test-key");
        env::set_var("ANTHROPIC_BASE_URL", "https://custom.api.com");
        let result = ClientBuilder::new().build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.inner.config.base_url.as_str(), "https://custom.api.com/");

        // Test invalid base URL in env var
        env::set_var("ANTHROPIC_BASE_URL", "not-a-valid-url");
        let result = ClientBuilder::new().build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid base URL in ANTHROPIC_BASE_URL"));

        // Test builder precedence over env vars
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-env-key");
        env::set_var("ANTHROPIC_BASE_URL", "https://env.api.com");
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-builder-key")
            .base_url("https://builder.api.com")
            .unwrap()
            .build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-builder-key");
        assert_eq!(client.inner.config.base_url.as_str(), "https://builder.api.com/");

        // Test ANTHROPIC_API_KEY priority over CLAUDE_API_KEY
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-anthropic-key");
        env::set_var("CLAUDE_API_KEY", "sk-ant-api03-claude-key");
        let result = ClientBuilder::new().build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-anthropic-key");

        // Restore original values
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("CLAUDE_API_KEY");
        env::remove_var("ANTHROPIC_BASE_URL");
        
        if let Some(val) = original_anthropic {
            env::set_var("ANTHROPIC_API_KEY", val);
        }
        if let Some(val) = original_claude {
            env::set_var("CLAUDE_API_KEY", val);
        }
        if let Some(val) = original_base_url {
            env::set_var("ANTHROPIC_BASE_URL", val);
        }
    }

    #[test]
    fn test_client_builder_build_with_custom_http_client() {
        let custom_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .http_client(custom_client)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_client_builder_build_validation_failure() {
        let result = ClientBuilder::new()
            .api_key("invalid-key") // Invalid format
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key must start with 'sk-ant-'"));
    }

    #[test]
    fn test_client_builder_build_max_tokens_validation() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Haiku20240307)
            .max_tokens(300_000) // Exceeds model limit
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds model limit"));
    }

    #[test]
    fn test_client_builder_build_with_all_options() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://custom.api.com")
            .unwrap()
            .timeout(Duration::from_secs(30))
            .max_retries(5)
            .model(Model::Claude3Haiku20240307)
            .max_tokens(1000)
            .build();

        assert!(result.is_ok());

        let client = result.unwrap();
        let config = &client.inner.config;
        
        assert_eq!(config.api_key, "sk-ant-api03-test-key");
        assert_eq!(config.base_url.as_str(), "https://custom.api.com/");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.model, Model::Claude3Haiku20240307);
        assert_eq!(config.max_tokens, 1000);
    }

    #[test]
    fn test_config_validation_all_models() {
        let models = vec![
            Model::Claude3Haiku20240307,
            Model::Claude3Sonnet20240229,
            Model::Claude3Opus20240229,
            Model::Claude35Sonnet20241022,
            Model::Claude35Sonnet20250114,
            Model::Claude4Sonnet20250514,
        ];

        for model in models {
            let config = Config {
                api_key: "sk-ant-api03-test-key".to_string(),
                model: model.clone(),
                max_tokens: model.max_tokens(), // Use model's max tokens
                ..Config::default()
            };

            let result = config.validate();
            assert!(result.is_ok(), "Validation failed for model: {:?}", model);
        }
    }

    #[test]
    fn test_config_validation_edge_case_max_tokens() {
        let config = Config {
            api_key: "sk-ant-api03-test-key".to_string(),
            model: Model::Claude3Haiku20240307,
            max_tokens: 1, // Minimum valid value
            ..Config::default()
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_builder_default_values_applied() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .build()
            .unwrap();

        let config = &client.inner.config;
        
        // Check that default values are applied when not explicitly set
        assert_eq!(config.base_url.as_str(), "https://api.anthropic.com/");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.model, Model::Claude35Sonnet20241022);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_client_builder_with_retry_config() {
        use crate::client::RetryConfig;
        
        let retry_config = RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 1.5,
        };

        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .retry_config(retry_config.clone())
            .build()
            .unwrap();

        let client_retry_config = &client.inner.retry_config;
        assert_eq!(client_retry_config.max_retries, 5);
        assert_eq!(client_retry_config.initial_delay, Duration::from_millis(100));
        assert_eq!(client_retry_config.max_delay, Duration::from_secs(10));
        assert_eq!(client_retry_config.backoff_multiplier, 1.5);
    }

    #[test]
    fn test_client_builder_with_middleware() {
        use crate::client::RequestMiddleware;
        
        let middleware = RequestMiddleware::default()
            .with_request_logging()
            .with_response_logging();

        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .middleware(middleware)
            .build()
            .unwrap();

        let client_middleware = &client.inner.middleware;
        assert!(client_middleware.log_requests);
        assert!(client_middleware.log_responses);
        assert!(!client_middleware.log_headers);
        assert!(!client_middleware.log_body);
    }

    #[test]
    fn test_client_builder_with_logging() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .with_logging()
            .build()
            .unwrap();

        let middleware = &client.inner.middleware;
        assert!(middleware.log_requests);
        assert!(middleware.log_responses);
        assert!(middleware.log_headers);
        assert!(middleware.log_body);
    }

    #[test]
    fn test_client_builder_default_retry_and_middleware() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .build()
            .unwrap();

        // Check default retry config
        let retry_config = &client.inner.retry_config;
        assert_eq!(retry_config.max_retries, 3);
        assert_eq!(retry_config.initial_delay, Duration::from_millis(500));
        assert_eq!(retry_config.max_delay, Duration::from_secs(30));
        assert_eq!(retry_config.backoff_multiplier, 2.0);

        // Check default middleware
        let middleware = &client.inner.middleware;
        assert!(!middleware.log_requests);
        assert!(!middleware.log_responses);
        assert!(!middleware.log_headers);
        assert!(!middleware.log_body);
    }


}