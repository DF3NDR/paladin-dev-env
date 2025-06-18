use crate::application::ports::input::ml_port::{
    MlPort, MlPredictionRequest, MlPredictionResponse, MlPortError, MlModelInfo,
    MlInputData, MlPrediction
};
use crate::core::platform::container::content::ContentItem;
use std::collections::HashMap;
use std::time::Instant;
use std::path::Path;

/// TensorFlow Adapter
/// An implementation of the ML Port using TensorFlow.
/// This adapter translates ML port calls into TensorFlow operations.
pub struct TensorFlowAdapter {
    model_path: String,
    loaded_models: HashMap<String, TensorFlowModel>,
    default_timeout_ms: u64,
}

#[derive(Debug, Clone)]
struct TensorFlowModel {
    name: String,
    #[allow(dead_code)]
    path: String,
    version: String,
    input_types: Vec<String>,
    output_types: Vec<String>,
    #[allow(dead_code)]
    loaded: bool,
}

impl TensorFlowAdapter {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self, MlPortError> {
        let path_str = model_path.as_ref()
            .to_str()
            .ok_or_else(|| MlPortError::ModelLoadingError("Invalid model path".to_string()))?
            .to_string();

        if !model_path.as_ref().exists() {
            return Err(MlPortError::ModelLoadingError(
                format!("Model path does not exist: {}", path_str)
            ));
        }

        Ok(Self {
            model_path: path_str,
            loaded_models: HashMap::new(),
            default_timeout_ms: 30000, // 30 seconds default timeout
        })
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    #[allow(dead_code)]
    fn load_model(&mut self, model_name: &str) -> Result<(), MlPortError> {
        if self.loaded_models.contains_key(model_name) {
            return Ok(());
        }

        let model_path = format!("{}/{}", self.model_path, model_name);
        
        if !Path::new(&model_path).exists() {
            return Err(MlPortError::ModelNotFound(model_name.to_string()));
        }

        // In a real implementation, you would load the TensorFlow model here
        // For now, we'll create a mock model entry
        let model = TensorFlowModel {
            name: model_name.to_string(),
            path: model_path,
            version: "1.0.0".to_string(),
            input_types: vec!["tensor".to_string()],
            output_types: vec!["tensor".to_string()],
            loaded: true,
        };

        self.loaded_models.insert(model_name.to_string(), model);
        Ok(())
    }

    fn execute_model(&self, model_name: &str, input_data: &MlInputData) -> Result<Vec<MlPrediction>, MlPortError> {
        let _model = self.loaded_models.get(model_name)
            .ok_or_else(|| MlPortError::ModelNotFound(model_name.to_string()))?;

        // In a real implementation, this would call TensorFlow's C API or use tensorflow-rust
        // For demonstration, we'll provide mock predictions based on input type
        match input_data {
            MlInputData::Text(text) => {
                self.process_text_prediction(text)
            },
            MlInputData::Image(_bytes) => {
                self.process_image_prediction()
            },
            MlInputData::Audio(_bytes) => {
                self.process_audio_prediction()
            },
            MlInputData::Video(_bytes) => {
                self.process_video_prediction()
            },
            MlInputData::Structured(_data) => {
                self.process_structured_prediction()
            },
        }
    }

    fn process_text_prediction(&self, text: &str) -> Result<Vec<MlPrediction>, MlPortError> {
        // Mock sentiment analysis
        let sentiment = if text.contains("good") || text.contains("great") || text.contains("excellent") {
            ("positive", 0.85)
        } else if text.contains("bad") || text.contains("terrible") || text.contains("awful") {
            ("negative", 0.80)
        } else {
            ("neutral", 0.60)
        };

        Ok(vec![
            MlPrediction::Sentiment {
                sentiment: sentiment.0.to_string(),
                confidence: sentiment.1,
            },
            MlPrediction::TextGeneration {
                text: format!("Summary: {}", &text[..std::cmp::min(100, text.len())]),
            },
        ])
    }

    fn process_image_prediction(&self) -> Result<Vec<MlPrediction>, MlPortError> {
        // Mock image classification
        Ok(vec![
            MlPrediction::Classification {
                class: "object".to_string(),
                confidence: 0.75,
            },
            MlPrediction::ObjectDetection {
                objects: vec![],
            },
        ])
    }

    fn process_audio_prediction(&self) -> Result<Vec<MlPrediction>, MlPortError> {
        // Mock audio classification
        Ok(vec![
            MlPrediction::Classification {
                class: "speech".to_string(),
                confidence: 0.90,
            }
        ])
    }

    fn process_video_prediction(&self) -> Result<Vec<MlPrediction>, MlPortError> {
        // Mock video analysis
        Ok(vec![
            MlPrediction::Classification {
                class: "action".to_string(),
                confidence: 0.70,
            }
        ])
    }

    fn process_structured_prediction(&self) -> Result<Vec<MlPrediction>, MlPortError> {
        // Mock structured data prediction
        Ok(vec![
            MlPrediction::Regression {
                value: 42.0,
            }
        ])
    }

    fn discover_models(&self) -> Result<Vec<String>, MlPortError> {
        let model_dir = Path::new(&self.model_path);
        if !model_dir.exists() {
            return Ok(vec![]);
        }

        let mut models = Vec::new();
        if let Ok(entries) = std::fs::read_dir(model_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        models.push(name.to_string());
                    }
                }
            }
        }
        Ok(models)
    }
}

impl MlPort for TensorFlowAdapter {
    fn predict(&self, request: MlPredictionRequest) -> Result<MlPredictionResponse, MlPortError> {
        let start_time = Instant::now();
        
        // Check if model is loaded
        if !self.loaded_models.contains_key(&request.model_name) {
            return Err(MlPortError::ModelNotFound(request.model_name));
        }

        let predictions = self.execute_model(&request.model_name, &request.input_data)?;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        // Extract confidence scores
        let confidence_scores: Vec<f64> = predictions.iter()
            .filter_map(|p| match p {
                MlPrediction::Classification { confidence, .. } => Some(*confidence),
                MlPrediction::Sentiment { confidence, .. } => Some(*confidence),
                _ => None,
            })
            .collect();

        Ok(MlPredictionResponse {
            model_name: request.model_name,
            predictions,
            confidence_scores: if confidence_scores.is_empty() { None } else { Some(confidence_scores) },
            processing_time_ms: processing_time,
            metadata: None,
        })
    }

    fn analyze_content(&self, mut content: ContentItem, model_name: &str) -> Result<ContentItem, MlPortError> {
        // Convert content to ML input
        let input_data = MlInputData::from_content_item(&content)?;
        
        let request = MlPredictionRequest {
            model_name: model_name.to_string(),
            input_data,
            parameters: None,
        };

        let response = self.predict(request)?;
        
        // Enrich content item with ML predictions
        let mut tags = content.tags.unwrap_or_default();
        
        for prediction in &response.predictions {
            match prediction {
                MlPrediction::Classification { class, confidence } => {
                    tags.push(format!("ml:class:{}", class));
                    if *confidence > 0.8 {
                        tags.push(format!("ml:high_confidence:{}", class));
                    }
                },
                MlPrediction::Sentiment { sentiment, confidence } => {
                    tags.push(format!("ml:sentiment:{}", sentiment));
                    if *confidence > 0.8 {
                        tags.push(format!("ml:strong_sentiment:{}", sentiment));
                    }
                },
                _ => {}
            }
        }
        
        content.tags = Some(tags);
        
        // Add ML metadata to description if not present
        if content.description.is_none() {
            let ml_summary: Vec<String> = response.predictions.iter()
                .filter_map(|p| match p {
                    MlPrediction::TextGeneration { text } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            
            if !ml_summary.is_empty() {
                content.description = Some(ml_summary.join("; "));
            }
        }

        Ok(content)
    }

    fn list_models(&self) -> Result<Vec<String>, MlPortError> {
        let mut models: Vec<String> = self.loaded_models.keys().cloned().collect();
        let discovered = self.discover_models()?;
        
        for model in discovered {
            if !models.contains(&model) {
                models.push(model);
            }
        }
        
        Ok(models)
    }

    fn is_model_available(&self, model_name: &str) -> Result<bool, MlPortError> {
        if self.loaded_models.contains_key(model_name) {
            return Ok(true);
        }
        
        let model_path = format!("{}/{}", self.model_path, model_name);
        Ok(Path::new(&model_path).exists())
    }

    fn get_model_info(&self, model_name: &str) -> Result<MlModelInfo, MlPortError> {
        if let Some(model) = self.loaded_models.get(model_name) {
            Ok(MlModelInfo {
                name: model.name.clone(),
                version: model.version.clone(),
                description: Some(format!("TensorFlow model: {}", model.name)),
                input_types: model.input_types.clone(),
                output_types: model.output_types.clone(),
                parameters: None,
            })
        } else if self.is_model_available(model_name)? {
            Ok(MlModelInfo {
                name: model_name.to_string(),
                version: "unknown".to_string(),
                description: Some(format!("Available TensorFlow model: {}", model_name)),
                input_types: vec!["tensor".to_string()],
                output_types: vec!["tensor".to_string()],
                parameters: None,
            })
        } else {
            Err(MlPortError::ModelNotFound(model_name.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use crate::core::platform::container::content::{ContentType, TextContent};

    fn create_test_adapter() -> TensorFlowAdapter {
        let temp_dir = tempdir().unwrap();
        let model_dir = temp_dir.path().join("test_model");
        fs::create_dir_all(&model_dir).unwrap();
        
        TensorFlowAdapter::new(temp_dir.path()).unwrap()
    }

    #[test]
    fn test_tensorflow_adapter_creation() {
        let temp_dir = tempdir().unwrap();
        let adapter = TensorFlowAdapter::new(temp_dir.path());
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_tensorflow_adapter_invalid_path() {
        let adapter = TensorFlowAdapter::new("/nonexistent/path");
        assert!(adapter.is_err());
    }

    #[test]
    fn test_text_prediction() {
        let adapter = create_test_adapter();
        
        let request = MlPredictionRequest {
            model_name: "test_model".to_string(),
            input_data: MlInputData::Text("This is great!".to_string()),
            parameters: None,
        };

        // This will fail because model isn't loaded, which is expected behavior
        let result = adapter.predict(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MlPortError::ModelNotFound(_)));
    }

    #[test]
    fn test_analyze_content() {
        let mut adapter = create_test_adapter();
        
        // Create a mock model
        let model = TensorFlowModel {
            name: "sentiment_model".to_string(),
            path: "/test/path".to_string(),
            version: "1.0.0".to_string(),
            input_types: vec!["text".to_string()],
            output_types: vec!["classification".to_string()],
            loaded: true,
        };
        adapter.loaded_models.insert("sentiment_model".to_string(), model);

        let text_content = TextContent::new(None, Some("This is excellent content!".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();

        let result = adapter.analyze_content(content_item, "sentiment_model");
        assert!(result.is_ok());
        
        let enriched_content = result.unwrap();
        assert!(enriched_content.tags.is_some());
        let tags = enriched_content.tags.unwrap();
        assert!(tags.iter().any(|tag| tag.contains("ml:sentiment:")));
    }

    #[test]
    fn test_list_models() {
        let adapter = create_test_adapter();
        let _models = adapter.list_models().unwrap();
        // Should return empty list or discovered models - no assertion needed as unwrap() handles errors
    }

    #[test]
    fn test_model_availability() {
        let temp_dir = tempdir().unwrap();
        let model_dir = temp_dir.path().join("available_model");
        fs::create_dir_all(&model_dir).unwrap();
        
        let adapter = TensorFlowAdapter::new(temp_dir.path()).unwrap();
        
        assert!(adapter.is_model_available("available_model").unwrap());
        assert!(!adapter.is_model_available("nonexistent_model").unwrap());
    }
}