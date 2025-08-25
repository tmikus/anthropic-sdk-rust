//! Example demonstrating advanced configuration features
//!
//! This example shows how to use:
//! - Custom HTTP client injection
//! - Request/response logging and debugging
//! - Per-request timeout overrides
//! - Custom middleware and interceptors

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anthropic_rust::{
    ClientBuilder, ContentBlock, LoggingInterceptor, Model, RequestInterceptor, Result, RetryConfig,
};

/// Custom interceptor that tracks request metrics
#[derive(Debug)]
struct MetricsInterceptor {
    request_count: Arc<Mutex<u32>>,
    response_count: Arc<Mutex<u32>>,
    error_count: Arc<Mutex<u32>>,
}

impl MetricsInterceptor {
    fn new() -> Self {
        Self {
            request_count: Arc::new(Mutex::new(0)),
            response_count: Arc::new(Mutex::new(0)),
            error_count: Arc::new(Mutex::new(0)),
        }
    }

    fn get_metrics(&self) -> (u32, u32, u32) {
        let requests = *self.request_count.lock().unwrap();
        let responses = *self.response_count.lock().unwrap();
        let errors = *self.error_count.lock().unwrap();
        (requests, responses, errors)
    }
}

impl RequestInterceptor for MetricsInterceptor {
    fn before_request(&self, request: &reqwest::Request) -> Result<()> {
        let mut count = self.request_count.lock().unwrap();
        *count += 1;
        println!(
            "📊 Request #{}: {} {}",
            *count,
            request.method(),
            request.url()
        );
        Ok(())
    }

    fn after_response(&self, response: &reqwest::Response) -> Result<()> {
        let mut count = self.response_count.lock().unwrap();
        *count += 1;
        println!(
            "📊 Response #{}: {} {}",
            *count,
            response.status(),
            response.url()
        );
        Ok(())
    }

    fn on_error(&self, error: &anthropic_rust::Error) {
        let mut count = self.error_count.lock().unwrap();
        *count += 1;
        println!("📊 Error #{}: {}", *count, error);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Advanced Configuration Example");
    println!("================================\n");

    // Example 1: Custom HTTP Client
    println!("1️⃣ Custom HTTP Client Configuration");
    let custom_http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("anthropic-rust-sdk-example/1.0")
        .build()
        .map_err(|e| {
            anthropic_rust::Error::Config(format!("Failed to build HTTP client: {}", e))
        })?;

    // Example 2: Custom Retry Configuration
    println!("2️⃣ Custom Retry Configuration");
    let retry_config = RetryConfig {
        max_retries: 2,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        backoff_multiplier: 1.5,
    };

    // Example 3: Custom Interceptors
    println!("3️⃣ Custom Interceptors Setup");
    let metrics_interceptor = Arc::new(MetricsInterceptor::new());
    let logging_interceptor = LoggingInterceptor::new()
        .with_request_logging()
        .with_response_logging()
        .with_error_logging();

    // Example 4: Advanced Client Configuration
    println!("4️⃣ Building Advanced Client");
    let client = ClientBuilder::new()
        .api_key("sk-ant-api03-example-key-not-real") // This will fail, but demonstrates config
        .model(Model::Claude35Sonnet20241022)
        .max_tokens(1000)
        .timeout(Duration::from_secs(60))
        .http_client(custom_http_client)
        .retry_config(retry_config)
        .with_interceptor(metrics_interceptor.clone())
        .with_logging_interceptor(logging_interceptor)
        .with_logging() // Enable built-in logging as well
        .build();

    match client {
        Ok(client) => {
            println!("✅ Client built successfully with advanced configuration");

            // Example 5: Per-request timeout overrides
            println!("\n5️⃣ Per-request Timeout Overrides");

            let request = client
                .chat_builder()
                .user_message(ContentBlock::text("Hello, Claude!"))
                .build();

            // This would normally make a request, but will fail due to invalid API key
            // The example demonstrates the API structure
            println!("📝 Request with default timeout:");
            match client.execute_chat(request.clone()).await {
                Ok(_) => println!("✅ Request succeeded"),
                Err(e) => println!("❌ Request failed (expected): {}", e),
            }

            println!("\n📝 Request with custom timeout (10 seconds):");
            match client
                .execute_chat_with_timeout(request.clone(), Duration::from_secs(10))
                .await
            {
                Ok(_) => println!("✅ Request succeeded"),
                Err(e) => println!("❌ Request failed (expected): {}", e),
            }

            println!("\n📝 Request with model and timeout override:");
            match client
                .execute_chat_with_options(
                    Model::Claude3Haiku20240307,
                    request,
                    Some(Duration::from_secs(5)),
                )
                .await
            {
                Ok(_) => println!("✅ Request succeeded"),
                Err(e) => println!("❌ Request failed (expected): {}", e),
            }

            // Show metrics
            let (requests, responses, errors) = metrics_interceptor.get_metrics();
            println!("\n📊 Final Metrics:");
            println!("   Requests: {}", requests);
            println!("   Responses: {}", responses);
            println!("   Errors: {}", errors);
        }
        Err(e) => {
            println!("❌ Failed to build client: {}", e);
        }
    }

    println!("\n🎯 Advanced Configuration Features Demonstrated:");
    println!("   ✓ Custom HTTP client injection");
    println!("   ✓ Custom retry configuration");
    println!("   ✓ Request/response interceptors");
    println!("   ✓ Built-in logging interceptor");
    println!("   ✓ Per-request timeout overrides");
    println!("   ✓ Model overrides with timeout");
    println!("   ✓ Metrics collection via interceptors");

    Ok(())
}
