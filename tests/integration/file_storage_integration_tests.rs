use super::{TestContext, TestEnvironment};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::{ minio::MinIO };

use paladin::infrastructure::adapters::file_storage::minio::{MinioAdapter, MinioConfig};
use paladin::infrastructure::adapters::logs::system_log_adapter::SystemLogAdapter;
use paladin::application::ports::output::file_storage_port::{
    FileStoragePort, BatchFileStoragePort, AdvancedFileStoragePort,
    UploadOptions, ListOptions
};
use paladin::application::ports::output::log_port::LogPort;

pub struct FileStorageTestContext {
    pub adapter: MinioAdapter,
    pub env: TestEnvironment,
    #[allow(dead_code)]
    container: Option<ContainerAsync<MinIO>>,
}

#[async_trait::async_trait]
impl TestContext for FileStorageTestContext {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let env = TestEnvironment::new().await;
        
        if env.use_external_services {
            Self::new_external(env).await
        } else {
            Self::new_local(env).await
        }
    }

    async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Cleanup test files
        if let Ok(files) = self.adapter.list_files(None).await {
            for file in files.files {
                let _ = self.adapter.delete_file(&file.path).await;
            }
        }
        Ok(())
    }
}

impl FileStorageTestContext {
    /// Create context using external services (CI/CD)
    async fn new_external(env: TestEnvironment) -> Result<Self, Box<dyn std::error::Error>> {
        let log_adapter = SystemLogAdapter::new(Default::default())?;
        let log_adapter = Arc::new(log_adapter) as Arc<dyn LogPort>;
        
        let minio_config = MinioConfig {
            endpoint: env.minio_endpoint.clone(),
            access_key: env.minio_access_key.clone(),
            secret_key: env.minio_secret_key.clone(),
            bucket: env.test_bucket(),
            region: Some("us-east-1".to_string()),
            secure: false,
            path_style: true,
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
            max_retries: 3,
            max_idle_conns: 10,
        };

        let adapter = MinioAdapter::new(minio_config, Some(log_adapter))
            .await?;

        Ok(Self {
            adapter,
            env,
            container: None,
        })
    }

    /// Create context using local containers (development)
    async fn new_local(mut env: TestEnvironment) -> Result<Self, Box<dyn std::error::Error>> {
        let container = MinIO::default().start().await?;
        let port = container.get_host_port_ipv4(9000).await?;

        env.minio_endpoint = format!("localhost:{}", port);

        // Wait for MinIO to start
        tokio::time::sleep(Duration::from_secs(3)).await;

        let log_adapter = SystemLogAdapter::new(Default::default())?;
        let log_adapter = Arc::new(log_adapter) as Arc<dyn LogPort>;
        
        let minio_config = MinioConfig {
            endpoint: env.minio_endpoint.clone(),
            access_key: env.minio_access_key.clone(),
            secret_key: env.minio_secret_key.clone(),
            bucket: env.test_bucket(),
            region: Some("us-east-1".to_string()),
            secure: false,
            path_style: true,
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
            max_retries: 3,
            max_idle_conns: 10,
        };

        let adapter = MinioAdapter::new(minio_config, Some(log_adapter))
            .await?;

        Ok(Self {
            adapter,
            env,
            container: Some(container),
        })
    }

    pub fn create_test_file(&self, name: &str, content: &str) -> (PathBuf, Vec<u8>) {
        let path = PathBuf::from(format!("test/{}", name));
        let data = content.as_bytes().to_vec();
        (path, data)
    }

    pub fn create_upload_options(&self, tags: Vec<String>) -> UploadOptions {
        let mut metadata = HashMap::new();
        metadata.insert("test".to_string(), "true".to_string());
        metadata.insert("test_mode".to_string(), 
                        if self.env.use_external_services { "external".to_string() } else { "local".to_string() });
        metadata.insert("created_by".to_string(), "integration_test".to_string());

        UploadOptions {
            metadata,
            tags,
            overwrite: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    pub async fn test_file_storage_health_check() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;
        
        let health = ctx.adapter.health_check().await?;
        assert!(health.is_available);
        assert!(health.response_time_ms.is_some());
        assert!(health.error.is_none());

        println!("✅ MinIO health check passed: {}", ctx.env.minio_endpoint);
        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_file_upload_download_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;

        // Test file upload
        let (file_path, file_content) = ctx.create_test_file("hello.txt", "Hello, integration test!");
        let upload_options = ctx.create_upload_options(vec!["test".to_string(), "hello".to_string()]);

        let file_item = ctx.adapter.upload_file(&file_path, &file_content, Some(upload_options)).await?;
        
        assert_eq!(file_item.path, file_path);
        assert_eq!(file_item.size, file_content.len() as u64);
        assert!(file_item.content_type.is_some());

        // Test file download
        let downloaded_content = ctx.adapter.download_file(&file_path, None).await?;
        assert_eq!(downloaded_content, file_content);

        // Test file existence
        let exists = ctx.adapter.file_exists(&file_path).await?;
        assert!(exists);

        // Test file info
        let file_info = ctx.adapter.get_file_info(&file_path).await?;
        assert_eq!(file_info.path, file_path);
        assert_eq!(file_info.size, file_content.len() as u64);

        println!("✅ Upload/Download lifecycle test passed");
        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_file_operations_full_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;

        // Create and upload multiple test files
        let files = vec![
            ("analysis/code.rs", "fn main() { println!(\"Integration test\"); }"),
            ("reports/audit.md", "# Integration Test Report\n\nAll systems working."),
            ("data/config.json", r#"{"integration": {"test": true}}"#),
        ];

        for (path_str, content) in &files {
            let path = PathBuf::from(path_str);
            let content_bytes = content.as_bytes().to_vec();
            let options = ctx.create_upload_options(vec!["integration".to_string()]);

            ctx.adapter.upload_file(&path, &content_bytes, Some(options)).await?;
        }

        // Test file listing
        let file_list = ctx.adapter.list_files(Some(ListOptions {
            prefix: Some("analysis/".to_string()),
            ..Default::default()
        })).await?;
        assert_eq!(file_list.files.len(), 1);

        // Test copy operation
        let source_path = PathBuf::from("analysis/code.rs");
        let dest_path = PathBuf::from("backup/code_backup.rs");
        
        ctx.adapter.copy_file(&source_path, &dest_path).await?;

        // Verify both files exist
        assert!(ctx.adapter.file_exists(&source_path).await?);
        assert!(ctx.adapter.file_exists(&dest_path).await?);

        println!("✅ File operations full cycle test passed");
        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_batch_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;

        // Prepare batch upload data
        let batch_files = vec![
            (PathBuf::from("batch/file1.txt"), "Content 1".as_bytes().to_vec(), 
             Some(ctx.create_upload_options(vec!["batch".to_string()]))),
            (PathBuf::from("batch/file2.txt"), "Content 2".as_bytes().to_vec(),
             Some(ctx.create_upload_options(vec!["batch".to_string()]))),
            (PathBuf::from("batch/file3.txt"), "Content 3".as_bytes().to_vec(),
             Some(ctx.create_upload_options(vec!["batch".to_string()]))),
        ];

        // Test batch upload
        let uploaded_items = ctx.adapter.upload_files(batch_files.clone()).await?;
        assert_eq!(uploaded_items.len(), 3);

        // Test batch download
        let download_paths = batch_files.iter().map(|(path, _, _)| path.clone()).collect();
        let downloaded_files = ctx.adapter.download_files(download_paths, None).await?;
        assert_eq!(downloaded_files.len(), 3);

        // Test batch deletion
        let delete_paths = batch_files.iter().map(|(path, _, _)| path.clone()).collect();
        let deleted_paths = ctx.adapter.delete_files(delete_paths).await?;
        assert_eq!(deleted_paths.len(), 3);

        println!("✅ Batch operations test passed");
        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_presigned_urls() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;

        let file_path = PathBuf::from("presigned/test.txt");
        let content = b"Test content for presigned URLs";

        // Upload file first
        ctx.adapter.upload_file(&file_path, content, None).await?;

        // Test presigned download URL generation
        let download_url = ctx.adapter.generate_download_url(
            &file_path,
            Duration::from_secs(3600),
            None
        ).await?;

        assert!(download_url.contains(&ctx.env.minio_endpoint.split(':').next().unwrap()));
        println!("✅ Generated presigned download URL: {}", download_url);

        // Test presigned upload URL generation
        let upload_path = PathBuf::from("presigned/upload.txt");
        let upload_url = ctx.adapter.generate_upload_url(
            &upload_path,
            Duration::from_secs(3600),
            None
        ).await?;

        assert!(upload_url.contains(&ctx.env.minio_endpoint.split(':').next().unwrap()));
        println!("✅ Generated presigned upload URL: {}", upload_url);

        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_storage_statistics() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;
        
        let image_file = &"x".repeat(1000);
        
        // Upload files of different types
        let test_files = vec![
            ("stats/doc1.pdf", "PDF content 1"),
            ("stats/doc2.pdf", "PDF content 2"),
            ("stats/image1.jpg", image_file),
            ("stats/code1.rs", "fn test() {}"),
        ];

        for (path_str, content) in &test_files {
            let path = PathBuf::from(path_str);
            let content_bytes = content.as_bytes().to_vec();
            
            ctx.adapter.upload_file(&path, &content_bytes, None).await?;
        }

        // Get storage statistics
        let stats = ctx.adapter.get_storage_stats().await?;
        
        assert!(stats.total_files >= 4);
        assert!(stats.total_size > 0);

        println!("✅ Storage statistics: {} files, {} bytes", stats.total_files, stats.total_size);
        
        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;

        // Test download non-existent file
        let non_existent = PathBuf::from("does/not/exist.txt");
        let result = ctx.adapter.download_file(&non_existent, None).await;
        assert!(result.is_err());

        // Test upload with invalid path
        let invalid_path = PathBuf::from("../../../etc/passwd");
        let result = ctx.adapter.upload_file(&invalid_path, b"malicious", None).await;
        assert!(result.is_err());

        // Test file exists for non-existent file
        let exists = ctx.adapter.file_exists(&non_existent).await?;
        assert!(!exists);

        println!("✅ Error handling test passed");
        ctx.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    pub async fn test_end_to_end_security_audit_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = FileStorageTestContext::new().await?;

        // Simulate complete security auditing workflow
        
        // 1. Upload source files
        let source_files = vec![
            ("audit/src/main.rs", "fn main() { println!(\"Security audit\"); }", vec!["source", "rust"]),
            ("audit/src/lib.rs", "pub mod security;", vec!["source", "rust"]),
            ("audit/config/app.toml", "[security]\nenabled = true", vec!["config", "toml"]),
        ];

        let mut uploaded_files = Vec::new();
        for (path_str, content, tags) in &source_files {
            let path = PathBuf::from(path_str);
            let options = ctx.create_upload_options(tags.iter().map(|s| s.to_string()).collect());
            let file_item = ctx.adapter.upload_file(&path, content.as_bytes(), Some(options)).await?;
            uploaded_files.push(file_item);
        }

        // 2. Generate and upload report
        let report_content = format!(
            "# Security Audit Report\n\nAnalyzed {} files.\n\n## Summary\nAll tests passed in {} mode.",
            uploaded_files.len(),
            if ctx.env.use_external_services { "CI/CD" } else { "local" }
        );
        
        let report_path = PathBuf::from("audit/reports/security_report.md");
        let report_options = ctx.create_upload_options(vec!["report".to_string()]);
        ctx.adapter.upload_file(&report_path, report_content.as_bytes(), Some(report_options)).await?;

        // 3. Verify workflow completion
        let audit_files = ctx.adapter.list_files(Some(ListOptions {
            prefix: Some("audit/".to_string()),
            ..Default::default()
        })).await?;

        assert!(audit_files.files.len() >= 4);
        
        println!("✅ End-to-end security audit workflow completed successfully");
        println!("   Processed {} files in {} mode", 
                 audit_files.files.len(),
                 if ctx.env.use_external_services { "CI/CD" } else { "local" });
        
        ctx.cleanup().await?;
        Ok(())
    }
}

// Helper functions for running all tests
pub async fn run_all_file_storage_tests() -> Result<(), Box<dyn std::error::Error>> {
    use crate::integration::report_test_result;

    println!("🧪 Running File Storage Integration Tests");
    
    let tests = vec![
        ("health_check", tests::test_file_storage_health_check()),
        ("upload_download", tests::test_file_upload_download_lifecycle()),
        ("file_operations", tests::test_file_operations_full_cycle()),
        ("batch_operations", tests::test_batch_operations()),
        ("presigned_urls", tests::test_presigned_urls()),
        ("storage_stats", tests::test_storage_statistics()),
        ("error_handling", tests::test_error_handling()),
        ("end_to_end", tests::test_end_to_end_security_audit_workflow()),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (name, test) in tests {
        let result = test;
        report_test_result(&format!("file_storage_{}", name), &result);
        
        match result {
            Ok(_) => passed += 1,
            Err(_) => failed += 1,
        }
    }

    println!("\n📊 File Storage Test Results: {} passed, {} failed", passed, failed);
    
    if failed > 0 {
        Err(format!("{} tests failed", failed).into())
    } else {
        Ok(())
    }
}