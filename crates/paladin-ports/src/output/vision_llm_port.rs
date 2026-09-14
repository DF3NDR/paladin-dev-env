//! Vision-capable LLM Port
//!
//! This port extends the base LLM port with vision capabilities, enabling
//! Paladin agents to process and analyze images alongside text prompts.
//!
//! Vision-capable models can accept image inputs in various formats (URLs, base64,
//! or file paths) and generate text responses based on both the image content
//! and accompanying text prompts.
//!
//! # Example
//!
//! ```rust,no_run
//! use paladin_ports::output::vision_llm_port::VisionCapableLlm;
//! use paladin_ports::output::llm_port::{LlmRequest, LlmPort};
//! use paladin_core::platform::container::vision::VisionRequest;
//!
//! async fn analyze_image(llm: &dyn VisionCapableLlm, request: LlmRequest, vision: VisionRequest) {
//!     if llm.supports_vision() {
//!         let response = llm.generate_with_vision(request, vision).await.unwrap();
//!         println!("Analysis: {}", response.content);
//!     }
//! }
//! ```

use async_trait::async_trait;

use super::llm_port::{LlmError, LlmPort, LlmRequest, LlmResponse};
use paladin_core::platform::container::vision::VisionRequest;

/// Trait for LLM providers that support vision/image inputs.
///
/// This trait extends the base `LlmPort` with methods for processing
/// multimodal requests containing both text and images. Implementers
/// must handle image format conversion, validation, and proper API
/// integration for their specific provider.
///
/// # Requirements
///
/// Implementations must:
/// - Support at least one image format (URL, base64, or file)
/// - Validate image formats and sizes before sending
/// - Handle provider-specific image encoding requirements
/// - Provide accurate capability reporting via `supports_vision()`
///
/// # Error Handling
///
/// Vision-specific errors should be wrapped in appropriate `LlmError` variants:
/// - `LlmError::InvalidPrompt` for invalid image formats or sizes
/// - `LlmError::ModelNotAvailable` for non-vision models
/// - `LlmError::ProcessingError` for image processing failures
///
/// # Choosing a vision surface
///
/// `VisionCapableLlm` is the **adapter-author surface**: implement this trait when adding a
/// vision-capable LLM provider. It is reached via `PaladinBuilder::enable_vision`.
///
/// Application code should generally call the sibling surface, `VisionPort`, via
/// `PaladinExecutionService::execute_with_vision`, instead of calling `VisionCapableLlm` methods
/// directly.
///
/// Both traits ship deliberately, at different layers of the framework. Neither is legacy, and
/// no migration between them is planned or recommended for either audience. See the recorded
/// decision at `.planning/decisions/0011-vision-port-surfaces.md`.
#[async_trait]
pub trait VisionCapableLlm: LlmPort + Send + Sync {
    /// Generate a completion with both text and vision inputs.
    ///
    /// This method processes requests containing images alongside text prompts,
    /// allowing the model to analyze visual content and generate text responses.
    ///
    /// # Arguments
    ///
    /// * `request` - The standard LLM request with text prompt and metadata
    /// * `vision` - The vision request containing one or more images
    ///
    /// # Returns
    ///
    /// * `Ok(LlmResponse)` - The generated response analyzing the images
    /// * `Err(LlmError)` - Error during processing, including:
    ///   - `InvalidPrompt` if image format is unsupported or invalid
    ///   - `ModelNotAvailable` if the model doesn't support vision
    ///   - `ProcessingError` if image processing fails
    ///   - `NetworkError` if image download fails (for URLs)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin_ports::output::vision_llm_port::VisionCapableLlm;
    /// # use paladin_ports::output::llm_port::{LlmRequest, LlmError};
    /// # use paladin_core::platform::container::vision::{VisionRequest, VisionContent, ImageDetail};
    /// # use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    /// # async fn example(llm: &dyn VisionCapableLlm) -> Result<(), Box<dyn std::error::Error>> {
    /// let user_prompt = UserPrompt {
    ///     query: "What is in this image?".to_string(),
    ///     context: None,
    /// };
    ///
    /// let request = LlmRequest::new("gpt-4o", PromptItem::new(PromptType::User(user_prompt))?);
    ///
    /// let vision = VisionRequest::new(
    ///     "Describe the contents of this image in detail.".to_string(),
    ///     vec![VisionContent::ImageUrl {
    ///         url: "https://example.com/image.png".to_string(),
    ///         detail: ImageDetail::High,
    ///     }],
    /// )?;
    ///
    /// let response = llm.generate_with_vision(request, vision).await?;
    /// println!("Analysis: {}", response.content);
    /// # Ok(())
    /// # }
    /// ```
    async fn generate_with_vision(
        &self,
        request: LlmRequest,
        vision: VisionRequest,
    ) -> Result<LlmResponse, LlmError>;

    /// Check if this provider supports vision capabilities.
    ///
    /// This method should return `true` if the provider can process
    /// vision requests, even if only certain models support it. For
    /// model-specific checks, validate the model separately.
    ///
    /// # Returns
    ///
    /// * `true` - Provider supports vision for at least some models
    /// * `false` - Provider does not support vision
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin_ports::output::vision_llm_port::VisionCapableLlm;
    /// # fn example(llm: &dyn VisionCapableLlm) {
    /// if llm.supports_vision() {
    ///     println!("This provider supports vision inputs");
    /// } else {
    ///     println!("Vision not supported, falling back to text-only");
    /// }
    /// # }
    /// ```
    fn supports_vision(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Test that VisionCapableLlm requires Send + Sync
    #[test]
    fn test_vision_capable_llm_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        // This will only compile if VisionCapableLlm is Send + Sync
        assert_send::<Arc<dyn VisionCapableLlm>>();
        assert_sync::<Arc<dyn VisionCapableLlm>>();
    }

    // Test that trait is object-safe (can be used as dyn trait)
    #[test]
    fn test_vision_capable_llm_is_object_safe() {
        #[allow(dead_code)]
        fn assert_object_safe(_: &dyn VisionCapableLlm) {}
        // If this compiles, the trait is object-safe
    }

    // Mock implementation for testing trait bounds
    struct MockVisionLlm;

    #[async_trait]
    impl LlmPort for MockVisionLlm {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            unimplemented!()
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<super::super::llm_port::StreamingResponse, LlmError>,
                    > + Send,
            >,
            LlmError,
        > {
            unimplemented!()
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["gpt-4o".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> super::super::llm_port::ProviderCapabilities {
            super::super::llm_port::ProviderCapabilities {
                supports_vision: true,
                ..Default::default()
            }
        }
    }

    #[async_trait]
    impl VisionCapableLlm for MockVisionLlm {
        async fn generate_with_vision(
            &self,
            _request: LlmRequest,
            _vision: VisionRequest,
        ) -> Result<LlmResponse, LlmError> {
            unimplemented!()
        }

        fn supports_vision(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_mock_vision_llm_implements_trait() {
        let _llm = MockVisionLlm;
        // If this compiles, our mock correctly implements the trait
    }

    #[test]
    fn test_mock_supports_vision() {
        let llm = MockVisionLlm;
        assert!(llm.supports_vision());
    }

    #[test]
    fn test_trait_can_be_boxed() {
        let llm: Box<dyn VisionCapableLlm> = Box::new(MockVisionLlm);
        assert!(llm.supports_vision());
    }

    #[test]
    fn test_trait_can_be_arc() {
        let llm: Arc<dyn VisionCapableLlm> = Arc::new(MockVisionLlm);
        assert!(llm.supports_vision());
    }
}
