// Include the openai_content_analysis_integration module directly
#[path = "integration/openai_content_analysis_integration_test.rs"]
mod openai_content_analysis_integration_test;

// Include the system_log_integration module directly
#[path = "integration/system_log_integration_test.rs"]
mod system_log_integration_test;

// Include the queue_integration module directly
#[path = "integration/redis_queue_integration_test.rs"]
mod redis_queue_integration_test;

// Include the file_storage_integration module directly
#[path = "integration/file_storage_integration.rs"]
mod file_storage_integration;