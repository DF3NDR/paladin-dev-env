// Include the mysql_content_repository_test module directly
#[path = "functional/content_lifecycle.rs"]
mod mysql_content_repository_test;

// Include the content_fetching_pipeline module directly
#[path = "functional/content_fetching_pipeline.rs"]
mod content_fetching_pipeline;

// Include the content_lifecycle module directly
#[path = "functional/content_lifecycle.rs"]
mod content_lifecycle;

// Include the content_llm_analysis_pipeline module directly
#[path = "functional/content_llm_analysis_pipeline.rs"]
mod content_llm_analysis_pipeline;