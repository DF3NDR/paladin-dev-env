//! Environment-Gated Vision Integration Tests
//!
//! These tests validate vision capabilities with real API calls to OpenAI and Anthropic.
//! They are gated by environment variables to allow CI to skip them when API keys are not available.
//!
//! To run these tests:
//! ```bash
//! ENABLE_VISION_TESTS=true OPENAI_API_KEY=your_key ANTHROPIC_API_KEY=your_key cargo test --test vision_integration
//! ```

use paladin::core::platform::container::prompt::{PromptItem, PromptRole, PromptType, TextPrompt};
use paladin::core::platform::container::vision::{ImageDetail, VisionContent, VisionRequest};
use paladin::{AnthropicAdapter, AnthropicConfig};
use paladin::{OpenAIAdapter, OpenAIConfig};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use paladin_ports::output::vision_llm_port::VisionCapableLlm;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Helper to check if vision integration tests should run
fn should_run_vision_tests() -> bool {
    env::var("ENABLE_VISION_TESTS").unwrap_or_default() == "true"
}

/// Helper to get test fixture path
fn get_fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(filename);
    path
}

/// Helper to read image as base64
fn read_image_as_base64(path: &PathBuf) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    use base64::{Engine as _, engine::general_purpose};
    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Helper to create an LlmRequest for testing
fn create_llm_request(model: &str, prompt: &str) -> LlmRequest {
    let text_prompt = TextPrompt {
        content: prompt.to_string(),
        role: PromptRole::User,
    };
    LlmRequest::new(
        model,
        PromptItem::new(PromptType::Text(text_prompt)).unwrap(),
    )
}

#[tokio::test]
async fn test_openai_vision_integration() {
    // Skip if not enabled or API key not available
    if !should_run_vision_tests() {
        println!("Skipping OpenAI vision integration test (ENABLE_VISION_TESTS not set)");
        return;
    }

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping OpenAI vision integration test (OPENAI_API_KEY not set)");
            return;
        }
    };

    // Create adapter with real API key
    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Should create OpenAI adapter");

    // Verify capabilities include vision
    let capabilities = adapter.get_capabilities();
    assert!(
        capabilities.supports_vision,
        "OpenAI adapter should support vision"
    );

    // Read test image
    let image_path = get_fixture_path("sample_image.jpg");
    let image_base64 = read_image_as_base64(&image_path).expect("Should read test image");

    // Create vision request
    let vision_content = VisionContent::ImageBase64 {
        data: image_base64,
        media_type: "image/png".to_string(),
        detail: ImageDetail::Auto,
    };

    let vision_request = VisionRequest::new(
        "Describe what you see in this image".to_string(),
        vec![vision_content],
    )
    .expect("Should create valid vision request");

    let llm_request = create_llm_request("gpt-4o-mini", "Describe what you see in this image");

    // Execute real API call
    let result = adapter
        .generate_with_vision(llm_request, vision_request)
        .await;

    // Verify response
    match result {
        Ok(response) => {
            println!("OpenAI Vision Response: {}", response.content);
            assert!(!response.content.is_empty(), "Response should not be empty");
            assert!(
                response.usage.total_tokens > 0,
                "Usage information should be present"
            );
            println!("✓ OpenAI vision integration test passed");
        }
        Err(e) => {
            panic!("OpenAI vision API call failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_anthropic_vision_integration() {
    // Skip if not enabled or API key not available
    if !should_run_vision_tests() {
        println!("Skipping Anthropic vision integration test (ENABLE_VISION_TESTS not set)");
        return;
    }

    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping Anthropic vision integration test (ANTHROPIC_API_KEY not set)");
            return;
        }
    };

    // Create adapter with real API key
    let config = AnthropicConfig::new(
        api_key,
        "https://api.anthropic.com/v1".to_string(),
        "claude-3-5-sonnet-20241022".to_string(),
        4096,
    );
    let adapter = AnthropicAdapter::new(config).expect("Should create Anthropic adapter");

    // Verify capabilities include vision
    let capabilities = adapter.get_capabilities();
    assert!(
        capabilities.supports_vision,
        "Anthropic adapter should support vision"
    );

    // Read test image
    let image_path = get_fixture_path("sample_diagram.png");
    let image_base64 = read_image_as_base64(&image_path).expect("Should read test image");

    // Create vision request
    let vision_content = VisionContent::ImageBase64 {
        data: image_base64,
        media_type: "image/png".to_string(),
        detail: ImageDetail::Auto,
    };

    let vision_request = VisionRequest::new(
        "Describe what you see in this image".to_string(),
        vec![vision_content],
    )
    .expect("Should create valid vision request");

    let llm_request = create_llm_request(
        "claude-3-5-sonnet-20241022",
        "Describe what you see in this image",
    );

    // Execute real API call
    let result = adapter
        .generate_with_vision(llm_request, vision_request)
        .await;

    // Verify response
    match result {
        Ok(response) => {
            println!("Anthropic Vision Response: {}", response.content);
            assert!(!response.content.is_empty(), "Response should not be empty");
            assert!(
                response.usage.total_tokens > 0,
                "Usage information should be present"
            );
            println!("✓ Anthropic vision integration test passed");
        }
        Err(e) => {
            panic!("Anthropic vision API call failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_multiple_images_openai() {
    // Skip if not enabled or API key not available
    if !should_run_vision_tests() {
        println!("Skipping OpenAI multiple images test (ENABLE_VISION_TESTS not set)");
        return;
    }

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping OpenAI multiple images test (OPENAI_API_KEY not set)");
            return;
        }
    };

    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Should create OpenAI adapter");

    // Read multiple test images
    let image1_base64 = read_image_as_base64(&get_fixture_path("sample_image.jpg"))
        .expect("Should read test image 1");
    let image2_base64 = read_image_as_base64(&get_fixture_path("sample_diagram.png"))
        .expect("Should read test image 2");

    // Create vision request with multiple images
    let vision_contents = vec![
        VisionContent::ImageBase64 {
            data: image1_base64,
            media_type: "image/jpeg".to_string(),
            detail: ImageDetail::Auto,
        },
        VisionContent::ImageBase64 {
            data: image2_base64,
            media_type: "image/png".to_string(),
            detail: ImageDetail::Auto,
        },
    ];

    let vision_request = VisionRequest::new(
        "Compare these two images and describe any differences".to_string(),
        vision_contents,
    )
    .expect("Should create valid vision request");

    let llm_request = create_llm_request(
        "gpt-4o-mini",
        "Compare these two images and describe any differences",
    );

    // Execute real API call
    let result = adapter
        .generate_with_vision(llm_request, vision_request)
        .await;

    // Verify response
    match result {
        Ok(response) => {
            println!("OpenAI Multiple Images Response: {}", response.content);
            assert!(!response.content.is_empty(), "Response should not be empty");
            println!("✓ OpenAI multiple images test passed");
        }
        Err(e) => {
            panic!("OpenAI multiple images API call failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_image_url_openai() {
    // Skip if not enabled or API key not available
    if !should_run_vision_tests() {
        println!("Skipping OpenAI image URL test (ENABLE_VISION_TESTS not set)");
        return;
    }

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping OpenAI image URL test (OPENAI_API_KEY not set)");
            return;
        }
    };

    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Should create OpenAI adapter");

    // Create vision request with image URL
    let vision_content = VisionContent::ImageUrl {
        url: "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/320px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg".to_string(),
        detail: ImageDetail::Auto,
    };

    let vision_request = VisionRequest::new(
        "Describe what you see in this image".to_string(),
        vec![vision_content],
    )
    .expect("Should create valid vision request");

    let llm_request = create_llm_request("gpt-4o-mini", "Describe what you see in this image");

    // Execute real API call
    let result = adapter
        .generate_with_vision(llm_request, vision_request)
        .await;

    // Verify response
    match result {
        Ok(response) => {
            println!("OpenAI Image URL Response: {}", response.content);
            assert!(!response.content.is_empty(), "Response should not be empty");
            println!("✓ OpenAI image URL test passed");
        }
        Err(e) => {
            panic!("OpenAI image URL API call failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_high_detail_image_openai() {
    // Skip if not enabled or API key not available
    if !should_run_vision_tests() {
        println!("Skipping OpenAI high detail test (ENABLE_VISION_TESTS not set)");
        return;
    }

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping OpenAI high detail test (OPENAI_API_KEY not set)");
            return;
        }
    };

    let config = OpenAIConfig::new(api_key);
    let adapter = OpenAIAdapter::new(config).expect("Should create OpenAI adapter");

    // Read test image
    let image_base64 = read_image_as_base64(&get_fixture_path("sample_image.jpg"))
        .expect("Should read test image");

    // Create vision request with high detail
    let vision_content = VisionContent::ImageBase64 {
        data: image_base64,
        media_type: "image/jpeg".to_string(),
        detail: ImageDetail::High,
    };

    let vision_request = VisionRequest::new(
        "Analyze this image in detail and describe any patterns or structure".to_string(),
        vec![vision_content],
    )
    .expect("Should create valid vision request");

    let llm_request = create_llm_request(
        "gpt-4o-mini",
        "Analyze this image in detail and describe any patterns or structure",
    );

    // Execute real API call
    let result = adapter
        .generate_with_vision(llm_request, vision_request)
        .await;

    // Verify response
    match result {
        Ok(response) => {
            println!("OpenAI High Detail Response: {}", response.content);
            assert!(!response.content.is_empty(), "Response should not be empty");
            println!("✓ OpenAI high detail test passed");
        }
        Err(e) => {
            panic!("OpenAI high detail API call failed: {:?}", e);
        }
    }
}
