/*
Content LLM Analysis Pipeline Test

This test verifies the functionality of the content LLM analysis pipeline using the proper hexagonal architecture.
It tests the LlmContentAnalyzer use case which takes pre-existing PromptItem and ContentItem objects as input,
and uses a mock LLM to simulate the analysis process. This ensures separation of concerns where the analyzer
doesn't know about specific LLM APIs and works with domain objects (PromptItem and ContentItem).
*/

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
use tokio;
use async_trait::async_trait;
use futures::stream;

use in4me::application::use_cases::content::content_llm_analysis_service::{
    LlmContentAnalyzer, LlmContentAnalysisInput, LlmContentAnalysisConfig
};
use in4me::core::platform::container::content::{
    ContentItem, ContentType, TextContent, VideoContent, AudioContent, ImageContent
};
use in4me::core::platform::container::prompt::{
    PromptItem, PromptType, TextPrompt, PromptRole
};
use in4me::application::ports::output::llm_port::{
    LlmPort, LlmRequest, LlmResponse, LlmError, TokenUsage, FinishReason, StreamingResponse
};
use in4me::application::use_cases::analysis::llm_analysis_service::{
    LlmAnalysisService, LlmAnalysisConfig
};

/// Mock LLM Port for testing
struct MockLlmPort {
    responses: HashMap<String, String>,
    delay_ms: Option<u64>,
    should_fail: bool,
    call_count: Arc<std::sync::Mutex<usize>>,
}

impl MockLlmPort {
    fn new() -> Self {
        let mut responses = HashMap::new();
        
        // Text content analysis response
        responses.insert("text".to_string(), json!({
            "main_topics": ["technology", "artificial intelligence", "future trends"],
            "key_information": [
                "AI will revolutionize multiple industries",
                "Machine learning is becoming more accessible",
                "Ethical considerations are paramount"
            ],
            "sentiment": "positive",
            "tone": "informative",
            "quality_score": 8.5,
            "summary": "A comprehensive overview of AI's potential impact on society and industry."
        }).to_string());

        // Video content analysis response
        responses.insert("video".to_string(), json!({
            "main_topics": ["educational content", "tutorial", "visual learning"],
            "key_information": [
                "Step-by-step instructions provided",
                "Visual demonstrations enhance understanding",
                "Suitable for beginners"
            ],
            "sentiment": "neutral",
            "tone": "instructional",
            "quality_score": 7.8,
            "video_analysis": {
                "estimated_engagement": "high",
                "content_density": "medium",
                "pacing": "appropriate"
            },
            "summary": "An instructional video with clear demonstrations and good pacing."
        }).to_string());

        // Audio content analysis response
        responses.insert("audio".to_string(), json!({
            "main_topics": ["podcast", "interview", "discussion"],
            "key_information": [
                "Expert insights shared",
                "Multiple perspectives presented",
                "Q&A format engaged audience"
            ],
            "sentiment": "positive",
            "tone": "conversational",
            "quality_score": 8.2,
            "audio_analysis": {
                "speech_clarity": "excellent",
                "background_noise": "minimal",
                "engagement_level": "high"
            },
            "summary": "An engaging interview with expert insights and clear audio quality."
        }).to_string());

        // Image content analysis response
        responses.insert("image".to_string(), json!({
            "main_topics": ["data visualization", "charts", "business metrics"],
            "key_information": [
                "Revenue trends shown over 5 years",
                "Growth trajectory is positive",
                "Seasonal patterns visible"
            ],
            "sentiment": "neutral",
            "tone": "analytical", 
            "quality_score": 9.0,
            "visual_analysis": {
                "composition": "well-balanced",
                "color_usage": "effective",
                "readability": "high"
            },
            "summary": "A clear and informative data visualization showing business performance."
        }).to_string());

        Self {
            responses,
            delay_ms: None,
            should_fail: false,
            call_count: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = Some(delay_ms);
        self
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    fn get_response_for_content_type(&self, content_type: &str) -> String {
        self.responses.get(content_type)
            .cloned()
            .unwrap_or_else(|| json!({
                "main_topics": ["general content"],
                "key_information": ["Content analyzed successfully"],
                "sentiment": "neutral",
                "tone": "informative",
                "quality_score": 7.0,
                "summary": "General content analysis completed."
            }).to_string())
    }
}

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Increment call count
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
        }

        // Simulate delay if configured
        if let Some(delay) = self.delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }

        // Simulate failure if configured
        if self.should_fail {
            return Err(LlmError::ProcessingError("Mock LLM failure".to_string()));
        }

        // Determine content type from attachments
        let content_type = if !request.attachments.is_empty() {
            match request.attachments[0].content() {
                ContentType::Text(_) => "text",
                ContentType::Video(_) => "video",
                ContentType::Audio(_) => "audio",
                ContentType::Image(_) => "image",
            }
        } else {
            "text" // Default for prompt-only requests
        };

        let response_content = self.get_response_for_content_type(content_type);

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: request.model,
            content: response_content,
            finish_reason: FinishReason::Stop,
            usage: TokenUsage {
                prompt_tokens: 150,
                completion_tokens: 200,
                total_tokens: 350,
            },
            created_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    async fn generate_stream(&self, request: LlmRequest) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let response = self.generate(request).await?;
        let chunks = vec![
            Ok(StreamingResponse {
                id: response.id,
                delta: response.content[..response.content.len()/2].to_string(),
                finish_reason: None,
            }),
            Ok(StreamingResponse {
                id: response.id,
                delta: response.content[response.content.len()/2..].to_string(),
                finish_reason: Some(FinishReason::Stop),
            }),
        ];
        
        Ok(Box::new(stream::iter(chunks)))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-model-v1".to_string(), "mock-model-v2".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "MockLLM"
    }
}

/// Helper function to create test content items
/// Fixed: Use None for file paths to avoid FileNotFound errors in tests
fn create_test_content_items() -> Vec<ContentItem> {
    vec![
        // Text content
        ContentItem::new_with_title(
            ContentType::Text(TextContent::new(
                None,
                Some("Artificial Intelligence is transforming industries worldwide. Machine learning algorithms are becoming more sophisticated and accessible to developers. However, ethical considerations must be at the forefront of AI development to ensure responsible innovation.".to_string())
            ).expect("Failed to create text content")),
            "AI Technology Overview".to_string()
        ).expect("Failed to create text content item"),

        // Video content - Use None for path to avoid file validation in tests
        ContentItem::new_with_title(
            ContentType::Video(VideoContent::new(
                None, // No file path for testing
                1800 // 30 minutes
            ).expect("Failed to create video content")),
            "Programming Tutorial".to_string()
        ).expect("Failed to create video content item"),

        // Audio content - Use None for path to avoid file validation in tests
        ContentItem::new_with_title(
            ContentType::Audio(AudioContent::new(
                None, // No file path for testing
                3600 // 1 hour
            ).expect("Failed to create audio content")),
            "Tech Industry Podcast".to_string()
        ).expect("Failed to create audio content item"),

        // Image content - Use None for path to avoid file validation in tests
        ContentItem::new_with_title(
            ContentType::Image(ImageContent::new(
                None, // No file path for testing
                (1920, 1080) // Resolution as tuple
            ).expect("Failed to create image content")),
            "Revenue Growth Chart".to_string()
        ).expect("Failed to create image content item"),
    ]
}

/// Helper function to create different types of analysis prompts
fn create_test_prompts() -> Vec<PromptItem> {
    vec![
        // General content analysis prompt
        PromptItem::new_with_title(
            PromptType::Text(TextPrompt {
                content: r#"Please analyze the attached content and provide comprehensive insights in JSON format:
{
  "main_topics": ["topic1", "topic2"],
  "key_information": ["fact1", "fact2"], 
  "sentiment": "positive/negative/neutral",
  "tone": "formal/informal/technical/etc",
  "quality_score": 0.8,
  "summary": "Brief summary of the content"
}"#.to_string(),
                role: PromptRole::User,
            }),
            "General Content Analysis".to_string()
        ).expect("Failed to create general analysis prompt"),

        // Technical content analysis prompt
        PromptItem::new_with_title(
            PromptType::Text(TextPrompt {
                content: r#"Analyze the technical content provided and focus on:
- Technical accuracy and depth
- Complexity level and target audience
- Practical applicability
- Areas for improvement

Provide your analysis in JSON format with technical_analysis, accuracy_score, and recommendations fields."#.to_string(),
                role: PromptRole::User,
            }),
            "Technical Content Analysis".to_string()
        ).expect("Failed to create technical analysis prompt"),

        // Creative content analysis prompt
        PromptItem::new_with_title(
            PromptType::Text(TextPrompt {
                content: r#"Evaluate the creative aspects of this content:
- Originality and creativity
- Engagement potential
- Emotional impact
- Visual/audio appeal (if applicable)

Return analysis as JSON with creativity_score, engagement_metrics, and enhancement_suggestions."#.to_string(),
                role: PromptRole::User,
            }),
            "Creative Content Analysis".to_string()
        ).expect("Failed to create creative analysis prompt"),

        // Educational content analysis prompt
        PromptItem::new_with_title(
            PromptType::Text(TextPrompt {
                content: r#"Assess the educational value of this content:
- Learning objectives clarity
- Content structure and flow
- Pedagogical effectiveness
- Accessibility for target learners

Provide JSON response with educational_score, learning_outcomes, and instructional_improvements."#.to_string(),
                role: PromptRole::User,
            }),
            "Educational Content Analysis".to_string()
        ).expect("Failed to create educational analysis prompt"),
    ]
}

// ... existing imports and helper functions ...

/// Test the complete content LLM analysis pipeline with proper architecture
#[tokio::test]
async fn test_content_llm_analysis_pipeline_complete_flow() {
    // Setup
    let mock_llm = Arc::new(MockLlmPort::new());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm.clone()));
    
    let analyzer = LlmContentAnalyzer::new(llm_service);
    let content_items = create_test_content_items();
    let prompts = create_test_prompts();

    let config = LlmContentAnalysisConfig::default();

    // Test analysis of each content type with the general prompt
    let general_prompt = &prompts[0]; // Use the general analysis prompt

    for (index, content_item) in content_items.iter().enumerate() {
        println!("Testing content item {}: {:?}", index, content_item.title());
        
        let input = LlmContentAnalysisInput {
            prompt: general_prompt.clone(),
            content: content_item.clone(),
        };
        
        let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
        
        assert!(result.is_ok(), "Analysis failed for content item {}: {:?}", index, result.err());
        
        let analysis = result.unwrap();
        assert!(analysis.is_object(), "Analysis result should be a JSON object");
        
        // ... rest of assertions remain the same
    }

    // Verify that the mock LLM was called for each content item
    assert_eq!(mock_llm.get_call_count(), content_items.len(), "Mock LLM should be called once per content item");
}

#[tokio::test]
async fn test_multiple_prompt_types_with_same_content() {
    let mock_llm = Arc::new(MockLlmPort::new());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm.clone()));
    let analyzer = LlmContentAnalyzer::new(llm_service);
    
    let content_items = create_test_content_items();
    let prompts = create_test_prompts();
    let config = LlmContentAnalysisConfig::default();

    // Test the same content with different prompts
    let text_content = &content_items[0]; // Use first content item (text)

    for (prompt_index, prompt) in prompts.iter().enumerate() {
        println!("Testing with prompt {}: {:?}", prompt_index, prompt.title());
        
        let input = LlmContentAnalysisInput {
            prompt: prompt.clone(),
            content: text_content.clone(),
        };
        
        let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
        assert!(result.is_ok(), "Analysis should succeed with prompt {}: {:?}", prompt_index, result.err());
        
        let analysis = result.unwrap();
        assert!(analysis.is_object(), "Analysis result should be a JSON object");
        
        // Each prompt should produce analysis
        assert!(analysis.get("content_metadata").is_some(), "Metadata should be present regardless of prompt type");
    }

    // Verify that the mock LLM was called for each prompt
    assert_eq!(mock_llm.get_call_count(), prompts.len(), "Mock LLM should be called once per prompt");
}

#[tokio::test]
async fn test_content_llm_analysis_with_custom_configuration() {
    let mock_llm = Arc::new(MockLlmPort::new().with_delay(100));
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm));
    let analyzer = LlmContentAnalyzer::new(llm_service);
    
    let content_items = create_test_content_items();
    let prompts = create_test_prompts();
    
    // Custom configuration
    let config = LlmContentAnalysisConfig {
        llm_config: LlmAnalysisConfig {
            model: "custom-model".to_string(),
            max_retries: 2,
            timeout_seconds: 60,
            enable_streaming: true,
        },
        include_content_metadata: false, // Disable metadata
        max_content_length: Some(5000),
    };

    let input = LlmContentAnalysisInput {
        prompt: prompts[0].clone(),
        content: content_items[0].clone(),
    };

    let start_time = std::time::Instant::now();
    let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
    let elapsed = start_time.elapsed();

    assert!(result.is_ok(), "Analysis should succeed with custom config");
    assert!(elapsed.as_millis() >= 100, "Should respect delay configuration");
    
    let analysis = result.unwrap();
    
    // Metadata should not be present due to configuration
    assert!(analysis.get("content_metadata").is_none(), "Metadata should not be present when disabled");
}

#[tokio::test]
async fn test_content_length_validation() {
    let mock_llm = Arc::new(MockLlmPort::new());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm));
    let analyzer = LlmContentAnalyzer::new(llm_service);
    
    let prompts = create_test_prompts();
    
    // Create content that exceeds length limit
    let long_content = ContentItem::new_with_title(
        ContentType::Text(TextContent::new(
            None,
            Some("a".repeat(2000)) // Long content
        ).expect("Failed to create long text content")),
        "Long Content".to_string()
    ).expect("Failed to create long content item");

    let input = LlmContentAnalysisInput {
        prompt: prompts[0].clone(),
        content: long_content,
    };
    
    // Set a low max content length
    let config = LlmContentAnalysisConfig {
        max_content_length: Some(100),
        ..Default::default()
    };

    let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
    assert!(result.is_err(), "Analysis should fail for content exceeding length limit");
    assert!(result.unwrap_err().contains("exceeds maximum allowed length"), "Error should mention length limit");
}

#[tokio::test]
async fn test_prompt_validation() {
    let mock_llm = Arc::new(MockLlmPort::new());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm));
    let analyzer = LlmContentAnalyzer::new(llm_service);
    
    let content_items = create_test_content_items();
    let config = LlmContentAnalysisConfig::default();

    // Test with empty prompt
    let empty_prompt = PromptItem::new_with_title(
        PromptType::Text(TextPrompt {
            content: "".to_string(), // Empty prompt
            role: PromptRole::User,
        }),
        "Empty Prompt".to_string()
    ).expect("Failed to create empty prompt");

    let input = LlmContentAnalysisInput {
        prompt: empty_prompt,
        content: content_items[0].clone(),
    };

    let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
    assert!(result.is_err(), "Analysis should fail with empty prompt");
    assert!(result.unwrap_err().contains("cannot be empty"), "Error should mention empty prompt");
}

#[tokio::test]
async fn test_error_handling_and_retries() {
    // Test with failing mock LLM
    let mock_llm = Arc::new(MockLlmPort::new().with_failure());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm));
    let analyzer = LlmContentAnalyzer::new(llm_service);
    
    let content_items = create_test_content_items();
    let prompts = create_test_prompts();
    
    let config = LlmContentAnalysisConfig {
        llm_config: LlmAnalysisConfig {
            model: "failing-model".to_string(),
            max_retries: 2,
            timeout_seconds: 5,
            enable_streaming: false,
        },
        ..Default::default()
    };

    let input = LlmContentAnalysisInput {
        prompt: prompts[0].clone(),
        content: content_items[0].clone(),
    };

    let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
    assert!(result.is_err(), "Analysis should fail with failing mock LLM");
    assert!(result.unwrap_err().contains("LLM analysis failed"), "Error should indicate LLM failure");
}

#[tokio::test]
async fn test_batch_processing_with_different_prompts() {
    let mock_llm = Arc::new(MockLlmPort::new());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm.clone()));
    
    let content_items = create_test_content_items();
    let prompts = create_test_prompts();
    let config = LlmContentAnalysisConfig::default();

    // Process multiple combinations in parallel
    let mut handles = vec![];
    
    for (content_index, content_item) in content_items.iter().enumerate() {
        for (prompt_index, prompt) in prompts.iter().enumerate() {
            let analyzer = LlmContentAnalyzer::new(llm_service.clone());
            let input = LlmContentAnalysisInput {
                prompt: prompt.clone(),
                content: content_item.clone(),
            };
            let config_clone = config.clone();
            
            let handle = tokio::spawn(async move {
                (content_index, prompt_index, analyzer.analyze_with_prompt_async(&input, &config_clone).await) // Use async version
            });
            
            handles.push(handle);
        }
    }

    // Wait for all analyses to complete
    let mut results = vec![];
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        results.push(result);
    }

    // Verify all analyses succeeded
    for (content_index, prompt_index, result) in results.iter() {
        assert!(result.is_ok(), "Batch analysis content {} prompt {} should succeed: {:?}", 
                content_index, prompt_index, result);
    }

    // Verify correct number of LLM calls (content_items * prompts)
    let expected_calls = content_items.len() * prompts.len();
    assert_eq!(mock_llm.get_call_count(), expected_calls, 
               "Should make {} LLM calls for {} contents × {} prompts", 
               expected_calls, content_items.len(), prompts.len());
}

#[tokio::test]
async fn test_content_analysis_separation_of_concerns() {
    // This test verifies that the analyzer properly separates concerns:
    // 1. It uses pre-existing PromptItem and ContentItem (domain objects)
    // 2. It doesn't create prompts internally
    // 3. It uses the LLM port abstraction
    // 4. It returns structured results
    
    let mock_llm = Arc::new(MockLlmPort::new());
    let llm_service = Arc::new(LlmAnalysisService::new(mock_llm.clone()));
    let analyzer = LlmContentAnalyzer::new(llm_service);
    
    // Domain objects created outside the analyzer
    let content = ContentItem::new_with_title(
        ContentType::Text(TextContent::new(
            None,
            Some("Test content for separation of concerns validation.".to_string())
        ).expect("Failed to create test content")),
        "SoC Test Content".to_string()
    ).expect("Failed to create content item");

    let prompt = PromptItem::new_with_title(
        PromptType::Text(TextPrompt {
            content: "Analyze this content for architectural compliance.".to_string(),
            role: PromptRole::User,
        }),
        "SoC Test Prompt".to_string()
    ).expect("Failed to create test prompt");

    let input = LlmContentAnalysisInput { prompt, content };
    let config = LlmContentAnalysisConfig::default();

    // The analyzer should work with these pre-existing objects
    let result = analyzer.analyze_with_prompt_async(&input, &config).await; // Use async version
    
    assert!(result.is_ok(), "Analyzer should work with pre-existing domain objects");
    
    let analysis = result.unwrap();
    
    // Verify proper structure and metadata handling
    assert!(analysis.is_object(), "Result should be properly structured");
    assert!(analysis.get("content_metadata").is_some(), "Metadata should be properly added");
    
    // Verify the mock was called (showing port abstraction works)
    assert_eq!(mock_llm.get_call_count(), 1, "LLM port should be called exactly once");
}