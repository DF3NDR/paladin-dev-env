/* 

ML Port

A port that defines how the application interacts with the ML (Machine Learning) model.

This port is responsible for translating high-level application use cases into interactions
with the ML model. It provides an abstraction layer that allows the application to interact with
the ML model without being tightly coupled to its implementation details.

Typical implementations of this port would be for the adapter to translate the requirements
of the high-level use cases into calls to the ML model, and to translate the results of those calls
back into a format that the application can use.

An ML Api usually requires a few standard fields to be present in the request and response
like 

A typical adapter for this type of port would be TensorFlow or PyTorch.

https://github.com/tensorflow/rust

*/
use crate::core::platform::container::content::ContentItem;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MlPredictionRequest {
    pub model_name: String,
    pub input_data: MlInputData,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MlInputData {
    Text(String),
    Image(Vec<u8>),
    Audio(Vec<u8>),
    Video(Vec<u8>),
    Structured(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MlPredictionResponse {
    pub model_name: String,
    pub predictions: Vec<MlPrediction>,
    pub confidence_scores: Option<Vec<f64>>,
    pub processing_time_ms: u64,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MlPrediction {
    Classification { class: String, confidence: f64 },
    Regression { value: f64 },
    TextGeneration { text: String },
    ObjectDetection { objects: Vec<DetectedObject> },
    Sentiment { sentiment: String, confidence: f64 },
    Embedding { vector: Vec<f64> },
    Custom(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectedObject {
    pub class: String,
    pub confidence: f64,
    pub bounding_box: Option<BoundingBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Error)]
pub enum MlPortError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Invalid input format: {0}")]
    InvalidInput(String),
    #[error("Prediction failed: {0}")]
    PredictionFailed(String),
    #[error("Model loading error: {0}")]
    ModelLoadingError(String),
    #[error("Unsupported content type for ML processing")]
    UnsupportedContentType,
    #[error("ML service unavailable")]
    ServiceUnavailable,
}

/// ML Port
/// A port that defines how the application interacts with the ML (Machine Learning) model.
/// This port is responsible for translating high-level application use cases into interactions
/// with the ML model. It provides an abstraction layer that allows the application to interact with
/// the ML model without being tightly coupled to its implementation details.
pub trait MlPort {
    /// Make a prediction using the ML model
    fn predict(&self, request: MlPredictionRequest) -> Result<MlPredictionResponse, MlPortError>;
    
    /// Analyze content and return enriched content item with ML predictions
    fn analyze_content(&self, content: ContentItem, model_name: &str) -> Result<ContentItem, MlPortError>;
    
    /// List available models
    fn list_models(&self) -> Result<Vec<String>, MlPortError>;
    
    /// Check if a model is available
    fn is_model_available(&self, model_name: &str) -> Result<bool, MlPortError>;
    
    /// Get model information
    fn get_model_info(&self, model_name: &str) -> Result<MlModelInfo, MlPortError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MlModelInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Helper functions to convert ContentItem to ML input data
impl MlInputData {
    pub fn from_content_item(content_item: &ContentItem) -> Result<Self, MlPortError> {
        use crate::core::platform::container::content::ContentType;
        
        match &content_item.content {
            ContentType::Text(text_content) => {
                if let Some(ref content) = text_content.content {
                    Ok(MlInputData::Text(content.clone()))
                } else if let Some(ref path) = text_content.path {
                    // Read file content
                    let content = std::fs::read_to_string(path)
                        .map_err(|e| MlPortError::InvalidInput(format!("Failed to read text file: {}", e)))?;
                    Ok(MlInputData::Text(content))
                } else {
                    Err(MlPortError::InvalidInput("No text content available".to_string()))
                }
            },
            ContentType::Image(image_content) => {
                if let Some(ref path) = image_content.path {
                    let bytes = std::fs::read(path)
                        .map_err(|e| MlPortError::InvalidInput(format!("Failed to read image file: {}", e)))?;
                    Ok(MlInputData::Image(bytes))
                } else {
                    Err(MlPortError::InvalidInput("No image file path available".to_string()))
                }
            },
            ContentType::Audio(audio_content) => {
                if let Some(ref path) = audio_content.path {
                    let bytes = std::fs::read(path)
                        .map_err(|e| MlPortError::InvalidInput(format!("Failed to read audio file: {}", e)))?;
                    Ok(MlInputData::Audio(bytes))
                } else {
                    Err(MlPortError::InvalidInput("No audio file path available".to_string()))
                }
            },
            ContentType::Video(video_content) => {
                if let Some(ref path) = video_content.path {
                    let bytes = std::fs::read(path)
                        .map_err(|e| MlPortError::InvalidInput(format!("Failed to read video file: {}", e)))?;
                    Ok(MlInputData::Video(bytes))
                } else {
                    Err(MlPortError::InvalidInput("No video file path available".to_string()))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentType, TextContent};

    #[test]
    fn test_ml_input_data_from_text_content() {
        let text_content = TextContent::new(None, Some("Hello world".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();
        
        let ml_input = MlInputData::from_content_item(&content_item).unwrap();
        
        match ml_input {
            MlInputData::Text(text) => assert_eq!(text, "Hello world"),
            _ => panic!("Expected text input data"),
        }
    }

    #[test]
    fn test_ml_prediction_serialization() {
        let prediction = MlPrediction::Classification {
            class: "positive".to_string(),
            confidence: 0.95,
        };
        
        let json = serde_json::to_string(&prediction).unwrap();
        let deserialized: MlPrediction = serde_json::from_str(&json).unwrap();
        
        assert_eq!(prediction, deserialized);
    }
}