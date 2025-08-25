//! Comprehensive unit tests for configuration module

#[cfg(test)]
mod tests {
    use crate::{Client, config::*, types::Model, Error};
    use std::time::Duration;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        
        assert_eq!(config.api_key, "");
        assert_eq!(config.base_url.as_str(), "https://api.anthropic.com/");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.model, Model::Claude35Sonnet20241022);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_client_builder_basic() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-test-key");
        assert_eq!(client.inner.config.model, Model::Claude35Sonnet20241022);
        assert_eq!(client.inner.config.max_tokens, 4096);
    }

    #[test]
    fn test_client_builder_with_custom_values() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-custom-key")
            .model(Model::Claude3Haiku20240307)
            .max_tokens(2000)
            .timeout(Duration::from_secs(30))
            .max_retries(5)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-custom-key");
        assert_eq!(client.inner.config.model, Model::Claude3Haiku20240307);
        assert_eq!(client.inner.config.max_tokens, 2000);
        assert_eq!(client.inner.config.timeout, Duration::from_secs(30));
        assert_eq!(client.inner.config.max_retries, 5);
    }

    #[test]
    fn test_client_builder_with_base_url() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .base_url("https://custom.api.com")
            .unwrap()
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        
        assert_eq!(client.inner.config.base_url.as_str(), "https://custom.api.com/");
    }

    #[test]
    fn test_client_builder_invalid_base_url() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .base_url("not-a-valid-url");

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("Invalid base URL")),
            _ => panic!("Expected Config error for invalid URL"),
        }
    }

    #[test]
    fn test_client_builder_empty_api_key() {
        let result = ClientBuilder::new()
            .api_key("")
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("API key cannot be empty")),
            _ => panic!("Expected Config error for empty API key"),
        }
    }

    #[test]
    fn test_client_builder_missing_api_key() {
        let result = Client::builder()
            .model(Model::Claude35Sonnet20241022)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("API key not provided")),
            _ => panic!("Expected Config error for missing API key"),
        }
    }

    #[test]
    fn test_client_builder_environment_variable() {
        // Set environment variable
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-env-key");

        let client = ClientBuilder::new()
            .model(Model::Claude35Sonnet20241022)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-env-key");

        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_client_builder_explicit_api_key_overrides_env() {
        // Set environment variable
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-env-key");

        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-explicit-key")
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-explicit-key");

        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_client_builder_custom_http_client() {
        let custom_http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .http_client(custom_http_client)
            .build();

        assert!(client.is_ok());
        // We can't directly verify the HTTP client was used, but we can verify
        // that the client was built successfully
    }

    #[test]
    fn test_client_new_convenience_method() {
        // Set environment variable for the test
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-test-key");

        let client = crate::Client::new(Model::Claude3Haiku20240307);
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.default_model(), Model::Claude3Haiku20240307);
        assert_eq!(client.default_max_tokens(), 4096); // Default value

        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_client_new_missing_env_var() {
        // Ensure environment variable is not set
        std::env::remove_var("ANTHROPIC_API_KEY");

        let result = crate::Client::new(Model::Claude35Sonnet20241022);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("API key")),
            _ => panic!("Expected Config error for missing API key"),
        }
    }

    #[test]
    fn test_client_builder_fluent_api() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Opus20240229)
            .max_tokens(8000)
            .timeout(Duration::from_secs(45))
            .max_retries(2)
            .base_url("https://custom.anthropic.com")
            .unwrap()
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        
        let config = &client.inner.config;
        assert_eq!(config.api_key, "sk-ant-api03-test-key");
        assert_eq!(config.model, Model::Claude3Opus20240229);
        assert_eq!(config.max_tokens, 8000);
        assert_eq!(config.timeout, Duration::from_secs(45));
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.base_url.as_str(), "https://custom.anthropic.com/");
    }

    #[test]
    fn test_client_builder_zero_timeout() {
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .timeout(Duration::from_secs(0))
            .build();

        // Zero timeout should be rejected by validation
        assert!(client.is_err());
    }

    #[test]
    fn test_client_builder_zero_max_retries() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .max_retries(0)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.inner.config.max_retries, 0);
    }

    #[test]
    fn test_client_builder_zero_max_tokens() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .max_tokens(0)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("max_tokens must be greater than zero")),
            _ => panic!("Expected Config error for zero max_tokens"),
        }
    }

    #[test]
    fn test_client_builder_excessive_max_tokens() {
        let result = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Haiku20240307)
            .max_tokens(300_000) // Exceeds model limit
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("exceeds model limit")),
            _ => panic!("Expected Config error for excessive max_tokens"),
        }
    }

    #[test]
    fn test_client_builder_valid_max_tokens_at_limit() {
        let client = ClientBuilder::new()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Haiku20240307)
            .max_tokens(200_000) // At model limit
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.inner.config.max_tokens, 200_000);
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            api_key: "test-key".to_string(),
            base_url: "https://api.anthropic.com/".parse().unwrap(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            model: Model::Claude35Sonnet20241022,
            max_tokens: 1000,
        };

        let cloned = config.clone();
        assert_eq!(config.api_key, cloned.api_key);
        assert_eq!(config.base_url, cloned.base_url);
        assert_eq!(config.timeout, cloned.timeout);
        assert_eq!(config.max_retries, cloned.max_retries);
        assert_eq!(config.model, cloned.model);
        assert_eq!(config.max_tokens, cloned.max_tokens);
    }

    #[test]
    fn test_config_debug() {
        let config = Config {
            api_key: "sk-ant-api03-secret-key".to_string(),
            base_url: "https://api.anthropic.com/".parse().unwrap(),
            timeout: Duration::from_secs(60),
            max_retries: 3,
            model: Model::Claude35Sonnet20241022,
            max_tokens: 4096,
        };

        let debug_str = format!("{:?}", config);
        
        // Check that debug output contains the expected fields
        // The exact format may vary, so we check for key components
        assert!(debug_str.contains("api_key"));
        assert!(debug_str.contains("anthropic.com"));
        assert!(debug_str.contains("60s"));
        assert!(debug_str.contains("max_retries: 3"));
        assert!(debug_str.contains("Claude35Sonnet20241022"));
        assert!(debug_str.contains("max_tokens: 4096"));
    }

    #[test]
    fn test_client_builder_default() {
        let builder = Client::builder();
        
        // Should be equivalent to Client::builder()
        let result = builder.api_key("sk-ant-api03-test-key").build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_builder_multiple_calls_same_field() {
        // Test that later calls override earlier ones
        let client = Client::builder()
            .api_key("sk-ant-api03-first-key")
            .api_key("sk-ant-api03-second-key")
            .model(Model::Claude3Haiku20240307)
            .model(Model::Claude35Sonnet20241022)
            .max_tokens(1000)
            .max_tokens(2000)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        
        assert_eq!(client.inner.config.api_key, "sk-ant-api03-second-key");
        assert_eq!(client.inner.config.model, Model::Claude35Sonnet20241022);
        assert_eq!(client.inner.config.max_tokens, 2000);
    }

    #[test]
    fn test_base_url_normalization() {
        let test_cases = vec![
            ("https://api.anthropic.com", "https://api.anthropic.com/"),
            ("https://api.anthropic.com/", "https://api.anthropic.com/"),
            ("https://custom.api.com/v1", "https://custom.api.com/v1"),
            ("https://custom.api.com/v1/", "https://custom.api.com/v1/"),
        ];

        for (input, expected) in test_cases {
            let client = Client::builder()
                .api_key("sk-ant-api03-test-key") // Use valid API key format
                .base_url(input)
                .unwrap()
                .build()
                .unwrap();

            assert_eq!(client.inner.config.base_url.as_str(), expected);
        }
    }

    #[test]
    fn test_client_default_accessors() {
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude3Opus20240229)
            .max_tokens(8000)
            .build()
            .unwrap();

        assert_eq!(client.default_model(), Model::Claude3Opus20240229);
        assert_eq!(client.default_max_tokens(), 8000);
    }

    #[test]
    fn test_client_clone() {
        let client = Client::builder()
            .api_key("sk-ant-api03-test-key")
            .model(Model::Claude35Sonnet20241022)
            .build()
            .unwrap();

        let cloned = client.clone();
        
        // Both should have the same configuration
        assert_eq!(client.default_model(), cloned.default_model());
        assert_eq!(client.default_max_tokens(), cloned.default_max_tokens());
        
        // They should be independent instances (Arc makes this efficient)
        assert_eq!(
            std::ptr::eq(&*client.inner, &*cloned.inner),
            true // Arc should point to the same data
        );
    }

    #[test]
    fn test_client_send_sync() {
        // Verify that Client implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        
        assert_send::<crate::Client>();
        assert_sync::<crate::Client>();
    }

    #[test]
    fn test_environment_variable_variations() {
        // Test different environment variable names that might be used
        let env_vars = vec![
            "ANTHROPIC_API_KEY",
            // Add other potential env var names if supported
        ];

        for env_var in env_vars {
            std::env::set_var(env_var, "sk-ant-api03-test-key-from-env");
            
            let client = Client::builder().build();
            
            if env_var == "ANTHROPIC_API_KEY" {
                assert!(client.is_ok());
                assert_eq!(client.unwrap().inner.config.api_key, "sk-ant-api03-test-key-from-env");
            }
            
            std::env::remove_var(env_var);
        }
    }

    #[test]
    fn test_client_builder_validation_order() {
        // Test that validation happens in the right order
        
        // Missing API key should be caught first
        let result = ClientBuilder::new()
            .max_tokens(0) // This is also invalid
            .build();
        
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => {
                // Should complain about API key first
                assert!(msg.contains("API key"));
            }
            _ => panic!("Expected Config error"),
        }
    }
}