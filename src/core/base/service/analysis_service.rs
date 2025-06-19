/*
Analysis Service

This is the generalized base service for performing analysis of containers in the platform. It provides a types and traits on top of which use case level services that perform analysis can build can build.  
*/
use std::fmt::Debug;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum AnalysisError {
    #[error("Analysis failed: {0}")]
    ProcessingError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Analysis service unavailable")]
    ServiceUnavailable,
    #[error("Timeout during analysis")]
    Timeout,
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Generic analysis result that can be specialized for different analysis types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult<T> {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub analysis_type: String,
    pub input_hash: Option<String>,
    pub result: T,
    pub confidence: Option<f64>,
    pub metadata: std::collections::HashMap<String, String>,
    pub processing_time_ms: u64,
}

/// Configuration trait for analysis services
pub trait AnalysisConfig: Debug + Clone {
    fn validate(&self) -> Result<(), AnalysisError>;
}

/// Base trait for all analysis services
pub trait AnalysisService<TInput, TOutput, TConfig> 
where 
    TInput: Debug + Clone,
    TOutput: Debug + Clone + Serialize,
    TConfig: AnalysisConfig,
{
    fn analyze(&self, input: &TInput, config: &TConfig) -> Result<AnalysisResult<TOutput>, AnalysisError>;
    
    fn get_analysis_type(&self) -> &'static str;
    
    fn validate_input(&self, input: &TInput) -> Result<(), AnalysisError>;
}

/// Repository trait for storing analysis results
pub trait AnalysisRepository<T> {
    fn store_result(&self, result: &AnalysisResult<T>) -> Result<(), AnalysisError>;
    fn get_result(&self, id: Uuid) -> Result<Option<AnalysisResult<T>>, AnalysisError>;
    fn get_results_by_hash(&self, hash: &str) -> Result<Vec<AnalysisResult<T>>, AnalysisError>;
    fn get_results_by_type(&self, analysis_type: &str) -> Result<Vec<AnalysisResult<T>>, AnalysisError>;
}