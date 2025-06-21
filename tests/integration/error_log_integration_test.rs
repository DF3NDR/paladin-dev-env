// /*
// Error Log Adapter Integration Tests

// These tests verify that the ErrorLogAdapter correctly handles error logging operations
// with enhanced error tracking, categorization, and correlation capabilities.
// */
// use std::time::Duration;
// use tokio::time::sleep;
// use uuid::Uuid;
// use serde_json::json;

// use in4me::infrastructure::adapters::logs::error_log_adapter::{
//     ErrorLogAdapter, ErrorLogAdapterConfig, ErrorInfo
// };
// use in4me::application::ports::output::log_port::{LogPort, LogFormat};
// use in4me::application::logs::error_log::{ErrorSeverity, ErrorCategory};
// use in4me::core::platform::container::log::{
//     LogLevel, LogDestination, LogEntryBuilder, LogEntryExt
// };
// use in4me::core::base::entity::message::Location;

// /// Test error log adapter creation and basic error logging
// #[tokio::test]
// async fn test_error_adapter_creation_and_logging() {
//     let config = ErrorLogAdapterConfig {
//         log_level: "error".to_string(),
//         format: LogFormat::Json,
//         target: "error_integration_test".to_string(),
//         structured: true,
//         include_stack_traces: true,
//         error_cache_size: 100,
//         enable_correlation: true,
//         min_severity: ErrorSeverity::Low,
//     };

//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Write some error entries
//     let error_entries = vec![
//         LogEntryBuilder::new_entry(
//             Location::system("database_service"),
//             LogDestination::Error,
//             LogLevel::Error,
//             "Database connection failed".to_string(),
//         ),
//         LogEntryBuilder::new_entry(
//             Location::service("user_service"),
//             LogDestination::Error,
//             LogLevel::Fatal,
//             "Critical user authentication error".to_string(),
//         ),
//         LogEntryBuilder::new_entry(
//             Location::system("payment_service"),
//             LogDestination::Error,
//             LogLevel::Warn,
//             "Payment processing warning".to_string(),
//         ),
//     ];

//     // Write entries one by one
//     for entry in &error_entries {
//         adapter.write_entry(entry.clone()).await.expect("Failed to write error entry");
//     }

//     adapter.flush().await.expect("Failed to flush");
//     sleep(Duration::from_millis(10)).await;

//     // Check statistics
//     let stats = adapter.get_stats().await.expect("Failed to get stats");
//     assert_eq!(stats.entries_written, 3, "Should have written 3 error entries");
//     assert_eq!(stats.errors, 3, "Should have tracked 3 errors");
//     assert!(stats.last_write.is_some(), "Should have last_write timestamp");

//     // Check recent errors cache
//     let recent_errors = adapter.get_recent_errors(Some(5)).await;
//     assert_eq!(recent_errors.len(), 3, "Should have 3 recent errors in cache");

//     println!("Successfully wrote {} error entries", error_entries.len());
// }

// /// Test enhanced error logging with detailed context
// #[tokio::test]
// async fn test_enhanced_error_logging() {
//     let config = ErrorLogAdapterConfig {
//         log_level: "error".to_string(),
//         format: LogFormat::Json,
//         target: "enhanced_error_test".to_string(),
//         structured: true,
//         include_stack_traces: true,
//         error_cache_size: 50,
//         enable_correlation: true,
//         min_severity: ErrorSeverity::Low,
//     };

//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Create enhanced error with full context
//     let error_info = ErrorInfo {
//         severity: ErrorSeverity::High,
//         category: ErrorCategory::Database,
//         error_code: Some("DB_CONN_001".to_string()),
//         stack_trace: Some("at DatabaseConnection.connect(db.rs:42)\nat UserService.authenticate(user.rs:156)".to_string()),
//         user_id: Some("user_12345".to_string()),
//         request_id: Some("req_67890".to_string()),
//         session_id: Some("sess_abcdef".to_string()),
//         additional_context: Some(json!({
//             "database_host": "prod-db-01",
//             "connection_pool_size": 20,
//             "retry_attempts": 3,
//             "error_occurred_at": "user_authentication"
//         })),
//     };

//     adapter.log_enhanced_error(
//         error_info,
//         "Failed to authenticate user due to database connection timeout".to_string(),
//         "user_authentication_service"
//     ).await.expect("Failed to log enhanced error");

//     adapter.flush().await.expect("Failed to flush");
//     sleep(Duration::from_millis(10)).await;

//     // Verify the error was logged and cached
//     let stats = adapter.get_stats().await.expect("Failed to get stats");
//     assert_eq!(stats.entries_written, 1);

//     let recent_errors = adapter.get_recent_errors(Some(1)).await;
//     assert_eq!(recent_errors.len(), 1);
    
//     let logged_error = &recent_errors[0];
//     assert!(logged_error.message.message.contains("Failed to authenticate user"));
//     assert_eq!(logged_error.level(), LogLevel::Error);
    
//     // Verify context was preserved
//     if let Some(ref context) = logged_error.message.context {
//         assert_eq!(context["severity"], "High");
//         assert_eq!(context["category"], "database");
//         assert_eq!(context["error_code"], "DB_CONN_001");
//         assert_eq!(context["user_id"], "user_12345");
//         assert!(context.get("stack_trace").is_some());
//         assert!(context.get("additional_context").is_some());
//     } else {
//         panic!("Error context should be present");
//     }

//     println!("Enhanced error logging test passed with full context preservation");
// }

// /// Test error severity filtering
// #[tokio::test]
// async fn test_error_severity_filtering() {
//     let config = ErrorLogAdapterConfig {
//         log_level: "error".to_string(),
//         format: LogFormat::Text,
//         target: "severity_filter_test".to_string(),
//         structured: true,
//         include_stack_traces: false,
//         error_cache_size: 50,
//         enable_correlation: true,
//         min_severity: ErrorSeverity::Medium, // Only log Medium and above
//     };

//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Try to log errors at different severity levels
//     let errors = vec![
//         (ErrorSeverity::Low, "Low severity error"),
//         (ErrorSeverity::Medium, "Medium severity error"),
//         (ErrorSeverity::High, "High severity error"),
//         (ErrorSeverity::Critical, "Critical severity error"),
//     ];

//     for (severity, message) in errors {
//         let error_info = ErrorInfo {
//             severity: severity.clone(),
//             category: ErrorCategory::Application,
//             error_code: Some(format!("ERR_{:?}", severity)),
//             stack_trace: None,
//             user_id: None,
//             request_id: None,
//             session_id: None,
//             additional_context: None,
//         };

//         adapter.log_enhanced_error(
//             error_info,
//             message.to_string(),
//             "severity_test_component"
//         ).await.expect("Failed to log error");
//     }

//     adapter.flush().await.expect("Failed to flush");
//     sleep(Duration::from_millis(10)).await;

//     // Only Medium, High, and Critical should be logged (3 out of 4)
//     let stats = adapter.get_stats().await.expect("Failed to get stats");
//     assert_eq!(stats.entries_written, 3, "Should have filtered out Low severity error");

//     let recent_errors = adapter.get_recent_errors(None).await;
//     assert_eq!(recent_errors.len(), 3);
    
//     // Verify no low severity errors were logged
//     for error in &recent_errors {
//         if let Some(context) = &error.message.context {
//             let severity = context.get("severity").and_then(|s| s.as_str()).unwrap_or("");
//             assert_ne!(severity, "Low", "Low severity error should have been filtered out");
//         }
//     }

//     println!("Error severity filtering test passed: 3/4 errors logged (filtered out Low)");
// }

// /// Test error correlation tracking
// #[tokio::test]
// async fn test_error_correlation_tracking() {
//     let config = ErrorLogAdapterConfig {
//         log_level: "error".to_string(),
//         format: LogFormat::Json,
//         target: "correlation_test".to_string(),
//         structured: true,
//         include_stack_traces: false,
//         error_cache_size: 100,
//         enable_correlation: true,
//         min_severity: ErrorSeverity::Low,
//     };

//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Create errors with the same correlation ID
//     let correlation_id = Uuid::new_v4();
//     let correlated_errors = vec![
//         "First error in sequence",
//         "Second error in sequence", 
//         "Third error in sequence",
//     ];

//     for (i, message) in correlated_errors.iter().enumerate() {
//         let mut entry = LogEntryBuilder::new_entry(
//             Location::service(&format!("component_{}", i)),
//             LogDestination::Error,
//             LogLevel::Error,
//             message.to_string(),
//         );
        
//         entry.correlation_id = Some(correlation_id);
        
//         adapter.write_entry(entry).await.expect("Failed to write correlated error");
//     }

//     // Create an error with a different correlation ID
//     let mut unrelated_entry = LogEntryBuilder::new_entry(
//         Location::service("unrelated_component"),
//         LogDestination::Error,
//         LogLevel::Error,
//         "Unrelated error".to_string(),
//     );
//     unrelated_entry.correlation_id = Some(Uuid::new_v4());
    
//     adapter.write_entry(unrelated_entry).await.expect("Failed to write unrelated error");

//     adapter.flush().await.expect("Failed to flush");
//     sleep(Duration::from_millis(10)).await;

//     // Check correlation tracking
//     let correlated_error_ids = adapter.get_correlated_errors(&correlation_id.to_string()).await;
//     assert_eq!(correlated_error_ids.len(), 3, "Should have 3 correlated errors");

//     let stats = adapter.get_stats().await.expect("Failed to get stats");
//     assert_eq!(stats.entries_written, 4, "Should have written 4 total errors");

//     println!("Error correlation tracking test passed: {} correlated errors tracked", correlated_error_ids.len());
// }

// /// Test error cache size management
// #[tokio::test]
// async fn test_error_cache_management() {
//     let config = ErrorLogAdapterConfig {
//         log_level: "error".to_string(),
//         format: LogFormat::Text,
//         target: "cache_test".to_string(),
//         structured: false,
//         include_stack_traces: false,
//         error_cache_size: 3, // Small cache for testing
//         enable_correlation: false,
//         min_severity: ErrorSeverity::Low,
//     };

//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Write more errors than cache size
//     for i in 0..5 {
//         let entry = LogEntryBuilder::new_entry(
//             Location::system(&format!("cache_test_{}", i)),
//             LogDestination::Error,
//             LogLevel::Error,
//             format!("Cache test error {}", i),
//         );
//         adapter.write_entry(entry).await.expect("Failed to write cache test error");
//     }

//     adapter.flush().await.expect("Failed to flush");
//     sleep(Duration::from_millis(10)).await;

//     // Cache should be limited to configured size
//     let recent_errors = adapter.get_recent_errors(None).await;
//     assert_eq!(recent_errors.len(), 3, "Cache should be limited to configured size");

//     // Should contain the most recent errors (2, 3, 4)
//     let messages: Vec<&str> = recent_errors.iter()
//         .map(|e| e.message.message.as_str())
//         .collect();
    
//     assert!(messages.contains(&"Cache test error 2"));
//     assert!(messages.contains(&"Cache test error 3"));
//     assert!(messages.contains(&"Cache test error 4"));
//     assert!(!messages.contains(&"Cache test error 0")); // Should be evicted
//     assert!(!messages.contains(&"Cache test error 1")); // Should be evicted

//     println!("Error cache management test passed: cache size properly limited");
// }

// /// Test different error categories and JSON formatting
// #[tokio::test]
// async fn test_error_categories_and_json_formatting() {
//     let config = ErrorLogAdapterConfig {
//         log_level: "error".to_string(),
//         format: LogFormat::Json,
//         target: "category_test".to_string(),
//         structured: true,
//         include_stack_traces: true,
//         error_cache_size: 50,
//         enable_correlation: false,
//         min_severity: ErrorSeverity::Low,
//     };

//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Test different error categories
//     let categories = vec![
//         (ErrorCategory::Application, "Application logic error"),
//         (ErrorCategory::Database, "Database connection error"),
//         (ErrorCategory::Network, "Network timeout error"),
//         (ErrorCategory::Security, "Authentication failure"),
//         (ErrorCategory::System, "System resource exhausted"),
//         (ErrorCategory::Validation, "Input validation failed"),
//     ];

//     for (category, message) in categories {
//         let error_info = ErrorInfo {
//             severity: ErrorSeverity::Medium,
//             category: category.clone(),
//             error_code: Some(format!("{}_{:03}", category.to_string().to_uppercase(), 001)),
//             stack_trace: Some("stack trace here".to_string()),
//             user_id: None,
//             request_id: None,
//             session_id: None,
//             additional_context: Some(json!({
//                 "test_category": category.to_string()
//             })),
//         };

//         adapter.log_enhanced_error(
//             error_info,
//             message.to_string(),
//             "category_test_component"
//         ).await.expect("Failed to log categorized error");
//     }

//     adapter.flush().await.expect("Failed to flush");
//     sleep(Duration::from_millis(10)).await;

//     let stats = adapter.get_stats().await.expect("Failed to get stats");
//     assert_eq!(stats.entries_written, 6, "Should have logged all category errors");

//     let recent_errors = adapter.get_recent_errors(None).await;
//     assert_eq!(recent_errors.len(), 6);

//     // Verify each category was properly formatted and logged
//     let mut found_categories = std::collections::HashSet::new();
//     for error in &recent_errors {
//         if let Some(context) = &error.message.context {
//             if let Some(category) = context.get("category").and_then(|c| c.as_str()) {
//                 found_categories.insert(category.to_string());
//             }
//         }
//     }

//     assert_eq!(found_categories.len(), 6, "Should have found all error categories");
//     assert!(found_categories.contains("application"));
//     assert!(found_categories.contains("database"));
//     assert!(found_categories.contains("network"));
//     assert!(found_categories.contains("security"));
//     assert!(found_categories.contains("system"));
//     assert!(found_categories.contains("validation"));

//     println!("Error categories and JSON formatting test passed: all {} categories logged", found_categories.len());
// }

// /// Test error adapter health check and metadata
// #[tokio::test]
// async fn test_error_adapter_metadata_and_health() {
//     let config = ErrorLogAdapterConfig::default();
//     let adapter = ErrorLogAdapter::new(config).expect("Failed to create error adapter");

//     // Test provider name
//     assert_eq!(adapter.get_provider_name(), "ErrorLogAdapter (log + env_logger)");

//     // Test supported formats
//     let formats = adapter.supported_formats();
//     assert!(formats.contains(&LogFormat::Json), "Should support JSON format");
//     assert!(formats.contains(&LogFormat::Text), "Should support Text format");
//     assert!(formats.len() >= 2, "Should support at least 2 formats");

//     // Test health check
//     let health = adapter.health_check().await.expect("Failed to perform health check");
//     assert!(health.is_empty() || health.iter().all(|h| h.healthy), "Adapter should be healthy");

//     // Test specific destination health check
//     let error_health = adapter.health_check_destination(LogDestination::Error).await.expect("Failed to check error destination health");
//     assert!(error_health.healthy);
//     assert_eq!(error_health.destination, LogDestination::Error);

//     // Test connection
//     adapter.test_connection().await.expect("Connection test should succeed");

//     println!("Error adapter metadata and health test passed. Supported formats: {:?}", formats);
// }