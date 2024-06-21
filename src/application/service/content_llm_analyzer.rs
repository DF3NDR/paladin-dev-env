/* !
Content Analyzer Port for LLM

This module contains the implementation of the Content Analysis Service for LLM.
It provides the functionality to analyze the content of a given ContentItem by
a LLM using a defined prompt and return the analysis results.

An LLM API has the following requirements:
- The URL of the enpoint
- The API Key
- The prompt to be used for the analysis
- The maximum number of tokens to be generated
- The data to be analyzed

The LlmContentAnalyzer struct implements the ContentAnalysisService trait for LLM.
The port defines the contract for the LLM content analysis service.
An adapter will implement this port to provide the actual implementation of the service.

*/
use serde_json::Value;
use crate::domain::entities::content_item::ContentItem;
use crate::domain::services::content_analysis_service::ContentAnalysisService;

pub struct LlmContentAnalyzer;

impl ContentAnalysisService for LlmContentAnalyzer {
    fn analyze_content(&self, content: &ContentItem) -> Result<Value, String> {
        // ToDO - Implement the actual logic to analyze the content
        Ok(Value::Null)
    }
}
