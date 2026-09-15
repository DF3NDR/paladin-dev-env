//! Vision Analysis Example
//!
//! Demonstrates basic single-image analysis using the Sentinel Vision System.
//! This example shows how to:
//! - Create a vision-enabled Paladin
//! - Analyze an image from a file
//! - Process the analysis results
//!
//! Run with: `cargo run --example vision_analysis`

use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::vision::{ImageDetail, VisionContent, VisionError};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::{OpenAIAdapter, OpenAIConfig};
use paladin_ports::output::llm_port::LlmPort;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🎯 Sentinel Vision System - Image Analysis Example\n");

    // Step 1: Configure OpenAI with vision-capable model
    println!("📋 Step 1: Configuring OpenAI with GPT-4o (vision-capable)...");
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let config = OpenAIConfig {
        api_key,
        base_url: "https://api.openai.com/v1".to_string(),
        organization: None,
        timeout_seconds: 300,
        max_retries: 3,
    };

    let llm_adapter: Arc<dyn LlmPort> =
        Arc::new(OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter"));

    // Verify vision support
    let capabilities = llm_adapter.get_capabilities();
    if !capabilities.supports_vision {
        eprintln!("❌ Error: Selected model does not support vision!");
        eprintln!("Please use a vision-capable model like gpt-4o or gpt-4-vision-preview");
        std::process::exit(1);
    }
    println!("✅ Vision support confirmed\n");

    // Step 2: Create circuit breaker for fault tolerance
    println!("📋 Step 2: Setting up circuit breaker...");
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,                       // failure_threshold: 5 failures before opening
        3,                       // success_threshold: 3 successes to close
        Duration::from_secs(60), // timeout: wait 60s before retry
    ));
    println!("✅ Circuit breaker configured\n");

    // Step 3: Build vision-enabled Paladin
    println!("📋 Step 3: Building vision-enabled Paladin...");
    let paladin = PaladinBuilder::new(llm_adapter.clone())
        .name("ImageAnalyzer")
        .system_prompt(
            "You are an expert image analyst. \
             Provide detailed, accurate descriptions of images. \
             Focus on key objects, scenes, actions, colors, and composition.",
        )
        .enable_vision(true) // ⚠️  Critical: Enable vision capabilities
        .model("gpt-4o")
        .temperature(0.7)
        .max_loops(1)
        .timeout_seconds(120)
        .build()
        .await
        .expect("Failed to build Paladin");

    println!(
        "✅ Paladin '{}' created with vision enabled\n",
        paladin.node.name
    );

    // Step 4: Create execution service
    println!("📋 Step 4: Creating execution service...");
    let execution_service = PaladinExecutionService::new(
        llm_adapter,
        circuit_breaker,
        None, // No garrison (memory) for this simple example
        None, // No arsenal (tools) for this example
    );
    println!("✅ Execution service ready\n");

    // Step 5: Prepare image for analysis
    println!("📋 Step 5: Preparing image for analysis...");

    // Option 1: Analyze a local file (recommended for this example)
    // Create a test image or point to an existing one
    let image_path = PathBuf::from("./examples/assets/sample_image.jpg");

    if !image_path.exists() {
        println!("⚠️  Warning: Sample image not found at {:?}", image_path);
        println!("   Creating example with image URL instead...\n");

        // Option 2: Use a publicly accessible image URL
        let vision_content = vec![VisionContent::ImageUrl {
            url: "https://upload.wikimedia.org/wikipedia/commons/thumb/4/47/PNG_transparency_demonstration_1.png/280px-PNG_transparency_demonstration_1.png".to_string(),
            detail: ImageDetail::Auto,
        }];

        analyze_image(
            &execution_service,
            &paladin,
            vision_content,
            "What do you see in this image? Describe it in detail.",
        )
        .await?;
    } else {
        // Use local file
        let vision_content = vec![VisionContent::ImageFile {
            path: image_path.clone(),
            detail: ImageDetail::Auto, // Let the model decide optimal detail level
        }];

        println!("📸 Image: {:?}", image_path);
        println!("🔍 Detail Level: Auto (balanced speed/quality)\n");

        analyze_image(
            &execution_service,
            &paladin,
            vision_content,
            "What do you see in this image? Describe it in detail.",
        )
        .await?;
    }

    // Step 6: Demonstrate different detail levels
    println!();
    println!("{}", "=".repeat(80));
    println!("📊 Demonstrating Different Detail Levels");
    println!("{}", "=".repeat(80));
    println!();

    // Low detail - faster and cheaper
    let low_detail_image = vec![VisionContent::ImageUrl {
        url: "https://upload.wikimedia.org/wikipedia/commons/thumb/4/47/PNG_transparency_demonstration_1.png/280px-PNG_transparency_demonstration_1.png".to_string(),
        detail: ImageDetail::Low,  // Max 512x512, ~85 tokens
    }];

    println!("🔽 LOW DETAIL (Fast & Cheap - ~85 tokens)");
    analyze_image(
        &execution_service,
        &paladin,
        low_detail_image,
        "Quick summary of this image.",
    )
    .await?;

    // High detail - more accurate but slower
    let high_detail_image = vec![VisionContent::ImageUrl {
        url: "https://upload.wikimedia.org/wikipedia/commons/thumb/4/47/PNG_transparency_demonstration_1.png/280px-PNG_transparency_demonstration_1.png".to_string(),
        detail: ImageDetail::High,  // Up to 2048x2048, ~170 tokens per tile
    }];

    println!("\n🔼 HIGH DETAIL (Accurate & Detailed - ~170+ tokens)");
    analyze_image(
        &execution_service,
        &paladin,
        high_detail_image,
        "Provide an extremely detailed analysis of this image.",
    )
    .await?;

    // Step 7: Demonstrate base64-encoded image processing
    println!();
    println!("{}", "=".repeat(80));
    println!("📦 Demonstrating Base64-Encoded Image Processing");
    println!("{}", "=".repeat(80));
    println!();

    demonstrate_base64_image(&execution_service, &paladin).await?;

    // Step 8: Demonstrate multiple images in a single request
    println!();
    println!("{}", "=".repeat(80));
    println!("🖼️  Demonstrating Multiple Images in One Request");
    println!("{}", "=".repeat(80));
    println!();

    demonstrate_multiple_images(&execution_service, &paladin).await?;

    // Step 9: Demonstrate error handling patterns
    println!();
    println!("{}", "=".repeat(80));
    println!("⚠️  Demonstrating Error Handling Patterns");
    println!("{}", "=".repeat(80));
    println!();

    demonstrate_error_handling(&execution_service, &paladin).await;

    println!();
    println!("{}", "=".repeat(80));
    println!("✅ Vision analysis example completed successfully!");
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Helper function to analyze an image and display results
async fn analyze_image(
    service: &PaladinExecutionService,
    paladin: &paladin::core::platform::container::paladin::Paladin,
    images: Vec<VisionContent>,
    task: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💭 Task: {}", task);
    println!("⏳ Analyzing...\n");

    let start = std::time::Instant::now();

    // Execute vision analysis
    let result = service.execute_with_vision(paladin, task, images).await?;

    let duration = start.elapsed();

    // Display results
    println!("{}", "─".repeat(80));
    println!("📊 ANALYSIS RESULTS");
    println!("{}", "─".repeat(80));
    println!("🤖 Paladin: {}", paladin.node.name);
    println!("⏱️  Execution Time: {:.2}s", duration.as_secs_f64());
    println!("🔄 Loops: {}", result.loop_count);
    println!("🎫 Tokens: {}", result.usage.total_tokens);
    println!("🛑 Stop Reason: {:?}", result.stop_reason);
    println!("{}", "─".repeat(80));
    println!("📝 OUTPUT:\n");
    println!("{}", result.output);
    println!("{}", "─".repeat(80));

    Ok(())
}

/// Demonstrate processing a base64-encoded image
///
/// Base64 encoding is useful when:
/// - Images are generated dynamically in memory
/// - You need to embed images in API requests
/// - Working with images from databases or data streams
async fn demonstrate_base64_image(
    service: &PaladinExecutionService,
    paladin: &paladin::core::platform::container::paladin::Paladin,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Base64 encoding is ideal for dynamically generated or in-memory images.");
    println!("This example creates a small image and encodes it as base64.\n");

    // Create a minimal valid PNG image (1x1 red pixel)
    // This is a base64-encoded PNG image
    let base64_data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==".to_string();

    let vision_content = vec![VisionContent::ImageBase64 {
        data: base64_data,
        media_type: "image/png".to_string(),
        detail: ImageDetail::Low, // Small image, so low detail is fine
    }];

    println!("📦 Image Format: Base64-encoded PNG");
    println!("🔍 Detail Level: Low");
    println!("💭 Task: Analyze this base64-encoded image\n");

    analyze_image(
        service,
        paladin,
        vision_content,
        "What color is this image?",
    )
    .await?;

    println!("\n💡 Tip: Base64 encoding is handled automatically by the vision adapter.");
    println!("   Just provide the data and media_type, and the adapter handles the rest.\n");

    Ok(())
}

/// Demonstrate analyzing multiple images in a single request
///
/// Multi-image analysis is useful for:
/// - Comparing multiple images
/// - Analyzing image sequences
/// - Processing related visual content together
async fn demonstrate_multiple_images(
    service: &PaladinExecutionService,
    paladin: &paladin::core::platform::container::paladin::Paladin,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("You can analyze multiple images in a single request for comparison");
    println!("or sequential analysis.\n");

    // Two different images for comparison
    let images = vec![
        VisionContent::ImageUrl {
            url: "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/400px-Cat03.jpg".to_string(),
            detail: ImageDetail::Low,
        },
        VisionContent::ImageUrl {
            url: "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4d/Cat_November_2010-1a.jpg/400px-Cat_November_2010-1a.jpg".to_string(),
            detail: ImageDetail::Low,
        },
    ];

    println!("📸 Number of Images: {}", images.len());
    println!("💭 Task: Compare and describe the differences\n");

    analyze_image(
        service,
        paladin,
        images,
        "Compare these two images. What are the key differences between them?",
    )
    .await?;

    println!("\n💡 Tip: Multiple image support varies by provider.");
    println!("   OpenAI GPT-4o and Anthropic Claude 3 both support multiple images per request.\n");

    Ok(())
}

/// Demonstrate error handling patterns for vision operations
///
/// Common errors include:
/// - Invalid image URLs (404, unreachable)
/// - Unsupported formats
/// - Authentication failures
/// - Rate limits exceeded
/// - Network timeouts
async fn demonstrate_error_handling(
    service: &PaladinExecutionService,
    paladin: &paladin::core::platform::container::paladin::Paladin,
) {
    println!("Vision operations can fail for various reasons. Here's how to handle them:\n");

    // Example 1: Invalid URL (likely to fail)
    println!("1️⃣  Handling Invalid Image URL");
    let invalid_url = vec![VisionContent::ImageUrl {
        url: "https://example.com/nonexistent-image-12345.jpg".to_string(),
        detail: ImageDetail::Auto,
    }];

    match service
        .execute_with_vision(paladin, "Describe this image", invalid_url)
        .await
    {
        Ok(_) => println!("   ✅ Unexpectedly succeeded"),
        Err(e) => {
            println!("   ❌ Error handled gracefully: {}", e);
            println!("   💡 Tip: Always validate URLs before processing");
        }
    }
    println!();

    // Example 2: Validate format before processing
    println!("2️⃣  Format Validation (Unsupported Format)");
    let unsupported = VisionContent::ImageBase64 {
        data: "invalid_base64".to_string(),
        media_type: "image/bmp".to_string(), // BMP not supported
        detail: ImageDetail::Auto,
    };

    match unsupported.validate_format() {
        Ok(_) => println!("   ✅ Format is valid"),
        Err(VisionError::UnsupportedFormat(msg)) => {
            println!("   ❌ Format validation failed: {}", msg);
            println!("   💡 Tip: Supported formats are PNG, JPEG, GIF, and WebP");
        }
        Err(e) => println!("   ❌ Validation error: {:?}", e),
    }
    println!();

    // Example 3: Demonstrate proper error handling pattern
    println!("3️⃣  Recommended Error Handling Pattern");
    println!("   ```rust");
    println!("   match service.execute_with_vision(paladin, task, images).await {{");
    println!("       Ok(result) => {{");
    println!("           // Process successful result");
    println!(r#"           println!("Analysis: {{}}", result.output);"#);
    println!("       }}");
    println!("       Err(e) => {{");
    println!("           // Log and handle error gracefully");
    println!(r#"           eprintln!("Vision error: {{}}", e);"#);
    println!("           // Optionally retry or use fallback strategy");
    println!("       }}");
    println!("   }}");
    println!("   ```\n");

    println!("💡 Best Practices:");
    println!("   • Validate image formats before sending to API");
    println!("   • Use circuit breaker for fault tolerance");
    println!("   • Implement retry logic with exponential backoff");
    println!("   • Set appropriate timeouts for large images");
    println!("   • Monitor token usage to stay within limits");
    println!("   • Handle rate limits gracefully\n");
}
