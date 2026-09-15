//! Vision Battalion Example
//!
//! Demonstrates multi-agent vision processing using Battalion orchestration patterns.
//! This example shows:
//! - Formation: Sequential vision analysis pipeline
//! - Phalanx: Parallel vision processing across multiple images
//!
//! Run with: `cargo run --example vision_battalion`

use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::vision::{ImageDetail, VisionContent};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::{OpenAIAdapter, OpenAIConfig};
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🏰 Sentinel Vision Battalion Example\n");

    // Step 1: Configure OpenAI
    println!("📋 Step 1: Configuring OpenAI with GPT-4o...");
    let openai_api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let config = OpenAIConfig {
        api_key: openai_api_key,
        base_url: "https://api.openai.com/v1".to_string(),
        organization: None,
        timeout_seconds: 300,
        max_retries: 3,
    };
    let llm_port: Arc<dyn LlmPort> =
        Arc::new(OpenAIAdapter::new(config).expect("Failed to create OpenAI adapter"));
    println!("✅ OpenAI configured\n");

    // Step 2: Setup circuit breaker
    println!("📋 Step 2: Setting up circuit breaker...");
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        5,
        2,
        std::time::Duration::from_secs(60),
    ));
    println!("✅ Circuit breaker ready\n");

    println!();
    println!("{}", "=".repeat(80));
    println!("🔗 FORMATION PATTERN: Sequential Vision Pipeline");
    println!("{}", "=".repeat(80));
    println!();

    // Demonstrate Formation pattern
    demonstrate_formation(&llm_port, &circuit_breaker).await?;

    println!();
    println!("{}", "=".repeat(80));
    println!("⚔️  PHALANX PATTERN: Parallel Vision Processing");
    println!("{}", "=".repeat(80));
    println!();

    // Demonstrate Phalanx pattern
    demonstrate_phalanx(&llm_port, &circuit_breaker).await?;

    println!();
    println!("{}", "=".repeat(80));
    println!("✅ Battalion vision processing completed successfully!");
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Demonstrate Formation pattern: Sequential vision pipeline
async fn demonstrate_formation(
    llm_port: &Arc<dyn LlmPort>,
    circuit_breaker: &Arc<CircuitBreaker>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Formation creates a sequential pipeline where each Paladin's output");
    println!("becomes the input for the next Paladin.\n");
    println!("Pipeline: Image Analyzer → Detail Extractor → Insight Generator\n");

    // Paladin 1: Image Analyzer (Initial analysis)
    println!("🛡️  Creating Paladin 1: Image Analyzer");
    let analyzer = PaladinBuilder::new(llm_port.clone())
        .name("Image Analyzer")
        .system_prompt(
            "You are a vision analyst. Analyze images and provide a brief description \
             of what you see. Focus on the main subjects, composition, and overall scene. \
             Keep your response concise (2-3 sentences).",
        )
        .max_loops(1)
        .enable_vision(true)
        .build()
        .await?;

    // Paladin 2: Detail Extractor (Detailed analysis based on initial)
    println!("🛡️  Creating Paladin 2: Detail Extractor");
    let extractor = PaladinBuilder::new(llm_port.clone())
        .name("Detail Extractor")
        .system_prompt(
            "You receive an initial image analysis. Your task is to identify and list \
             specific details mentioned: objects, colors, textures, spatial relationships. \
             Format as a bulleted list.",
        )
        .max_loops(1)
        .build()
        .await?;

    // Paladin 3: Insight Generator (Generate insights from details)
    println!("🛡️  Creating Paladin 3: Insight Generator");
    let generator = PaladinBuilder::new(llm_port.clone())
        .name("Insight Generator")
        .system_prompt(
            "You receive a detailed analysis. Generate 3 key insights or interesting \
             observations based on the details provided. Be thoughtful and analytical.",
        )
        .max_loops(1)
        .build()
        .await?;

    println!("✅ Formation battalion assembled: 3 Paladins\n");

    // Execute Formation pipeline
    println!("🚀 Executing Formation pipeline...\n");
    let start = Instant::now();

    // Stage 1: Analyze image
    println!("┌─ Stage 1: Image Analysis ─┐");
    let service_1 =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    // Use a sample image
    let image = VisionContent::ImageUrl {
        url: "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/1200px-Cat03.jpg"
            .to_string(),
        detail: ImageDetail::Low,
    };

    let stage_1_result = service_1
        .execute_with_vision(&analyzer, "Analyze this image.", vec![image])
        .await?;

    println!("│ Output: {}", stage_1_result.output.trim());
    println!("│ Tokens: {}", stage_1_result.usage.total_tokens);
    println!("└{}", "─".repeat(79));
    println!();

    // Stage 2: Extract details
    println!("┌─ Stage 2: Detail Extraction ─┐");
    let service_2 =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    let stage_2_result = service_2
        .execute(&extractor, &stage_1_result.output)
        .await?;

    println!(
        "│ Output:\n│ {}",
        stage_2_result.output.replace('\n', "\n│ ")
    );
    println!("│ Tokens: {}", stage_2_result.usage.total_tokens);
    println!("└{}", "─".repeat(79));
    println!();

    // Stage 3: Generate insights
    println!("┌─ Stage 3: Insight Generation ─┐");
    let service_3 =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    let stage_3_result = service_3
        .execute(&generator, &stage_2_result.output)
        .await?;

    println!(
        "│ Output:\n│ {}",
        stage_3_result.output.replace('\n', "\n│ ")
    );
    println!("│ Tokens: {}", stage_3_result.usage.total_tokens);
    println!("└{}", "─".repeat(79));
    println!();

    let duration = start.elapsed();

    // Summary
    println!("📊 FORMATION SUMMARY");
    println!("{}", "─".repeat(80));
    println!("Total Execution Time: {:.2}s", duration.as_secs_f64());
    println!(
        "Total Tokens: {}",
        stage_1_result.usage.total_tokens
            + stage_2_result.usage.total_tokens
            + stage_3_result.usage.total_tokens
    );
    println!("Pipeline Stages: 3 (sequential)");
    println!("Final Output Length: {} chars", stage_3_result.output.len());
    println!("{}", "─".repeat(80));

    Ok(())
}

/// Demonstrate Phalanx pattern: Parallel vision processing
async fn demonstrate_phalanx(
    llm_port: &Arc<dyn LlmPort>,
    circuit_breaker: &Arc<CircuitBreaker>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Phalanx executes multiple Paladins in parallel, each processing");
    println!("different images simultaneously for maximum throughput.\n");
    println!("Parallel Tasks: 3 images analyzed concurrently\n");

    // Create a single vision-enabled Paladin for parallel execution
    println!("🛡️  Creating Vision Paladin for parallel execution");
    let vision_paladin = PaladinBuilder::new(llm_port.clone())
        .name("Parallel Vision Analyst")
        .system_prompt(
            "You are a professional image analyst. Describe the image concisely, \
             identifying key elements, mood, and any notable features. Keep it under 100 words.",
        )
        .max_loops(1)
        .enable_vision(true)
        .build()
        .await?;

    println!("✅ Phalanx battalion ready\n");

    // Prepare multiple images for parallel processing
    let images = vec![
        (
            "Image 1: Cat",
            "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/1200px-Cat03.jpg",
        ),
        (
            "Image 2: Landscape",
            "https://upload.wikimedia.org/wikipedia/commons/thumb/e/e7/Everest_North_Face_toward_Base_Camp_Tibet_Luca_Galuzzi_2006.jpg/1200px-Everest_North_Face_toward_Base_Camp_Tibet_Luca_Galuzzi_2006.jpg",
        ),
        (
            "Image 3: Architecture",
            "https://upload.wikimedia.org/wikipedia/commons/thumb/6/6e/Taj_Mahal%2C_Agra%2C_India.jpg/1200px-Taj_Mahal%2C_Agra%2C_India.jpg",
        ),
    ];

    println!(
        "🚀 Executing Phalanx: {} images in parallel...\n",
        images.len()
    );
    let start = Instant::now();

    // Execute all analyses in parallel using tokio::spawn
    let mut tasks = vec![];

    for (name, url) in images {
        let paladin = vision_paladin.clone();
        let llm = llm_port.clone();
        let cb = circuit_breaker.clone();
        let url = url.to_string();
        let name = name.to_string();

        let task = tokio::spawn(async move {
            let service = PaladinExecutionService::new(llm, cb, None, None);

            let image = VisionContent::ImageUrl {
                url,
                detail: ImageDetail::Low,
            };

            let result = service
                .execute_with_vision(&paladin, "Analyze this image.", vec![image])
                .await;

            (name, result)
        });

        tasks.push(task);
    }

    // Collect results
    let mut total_tokens = 0;
    let mut success_count = 0;

    for (idx, task) in tasks.into_iter().enumerate() {
        match task.await {
            Ok((name, Ok(result))) => {
                success_count += 1;
                total_tokens += result.usage.total_tokens;

                println!("┌─ {} ─┐", name);
                println!("│ Status: ✅ Success");
                println!("│ Tokens: {}", result.usage.total_tokens);
                println!("│ Time: {:.2}s", result.execution_time_ms as f64 / 1000.0);
                println!("│");
                println!("│ Analysis:");

                // Format output with indentation
                for line in result.output.lines() {
                    println!("│   {}", line);
                }

                println!("└{}", "─".repeat(79));
                println!();
            }
            Ok((name, Err(e))) => {
                println!("┌─ {} ─┐", name);
                println!("│ Status: ❌ Failed");
                println!("│ Error: {}", e);
                println!("└{}", "─".repeat(79));
                println!();
            }
            Err(e) => {
                println!("┌─ Task {} ─┐", idx + 1);
                println!("│ Status: ❌ Task Error");
                println!("│ Error: {}", e);
                println!("└{}", "─".repeat(79));
                println!();
            }
        }
    }

    let duration = start.elapsed();

    // Summary
    println!("📊 PHALANX SUMMARY");
    println!("{}", "─".repeat(80));
    println!(
        "Total Execution Time: {:.2}s (parallel)",
        duration.as_secs_f64()
    );
    println!("Total Tokens: {}", total_tokens);
    println!("Successful Tasks: {}/3", success_count);
    println!(
        "Average Time per Image: {:.2}s",
        duration.as_secs_f64() / 3.0
    );
    println!(
        "Throughput: {:.2} images/second",
        3.0 / duration.as_secs_f64()
    );
    println!("{}", "─".repeat(80));

    println!("\n💡 Key Insight: Phalanx achieves ~3x speedup vs sequential processing!");

    Ok(())
}
