#[cfg(test)]
mod file_storage_integration_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use testcontainers::{
        core::{IntoContainerPort, WaitFor}, 
        runners::AsyncRunner, 
        GenericImage
    };

    use futures::future::try_join_all;

    use in4me::infrastructure::adapters::file_storage::minio::{MinioAdapter, MinioConfig};
    use in4me::infrastructure::adapters::logs::system_log_adapter::SystemLogAdapter;
    use in4me::application::ports::output::file_storage_port::{
        FileStoragePort, BatchFileStoragePort, AdvancedFileStoragePort, FileVersioningPort,
        UploadOptions,ListOptions, FileStorageUtils
    };
    use in4me::application::ports::output::log_port::LogPort;

    struct TestContext {
        adapter: Arc<MinioAdapter>,
        #[allow(dead_code)]
        container: testcontainers::ContainerAsync<GenericImage>,
        #[allow(dead_code)]
        port: u16,
    }

    impl TestContext {
        async fn new() -> Result<Self, Box<dyn std::error::Error>> {
            // Fix: Use the correct MinIO image name
            let container = GenericImage::new("minio/minio", "latest")
                .with_exposed_port(9000.tcp())
                .with_wait_for(WaitFor::message_on_stdout("Status:         1 Online, 0 Offline"))
                .start()
                .await
                .expect("Failed to start MinIO");

            let port = container.get_host_port_ipv4(9000).await?;

            // Wait longer for MinIO to fully start
            sleep(Duration::from_secs(5)).await;

            let log_adapter = Arc::new(SystemLogAdapter::new(Default::default()).unwrap()) as Arc<dyn LogPort>;
            
            let minio_config = MinioConfig {
                endpoint: format!("127.0.0.1:{}", port),
                access_key: "minioadmin".to_string(),
                secret_key: "minioadmin".to_string(),
                bucket: "test-bucket".to_string(),
                region: Some("us-east-1".to_string()),
                secure: false,
                path_style: true,
                connection_timeout: Duration::from_secs(30),
                request_timeout: Duration::from_secs(60),
                max_idle_conns: 5,
                max_retries: 5,
            };

            let adapter = MinioAdapter::new(minio_config, Some(log_adapter))
                .await?;

            Ok(TestContext {
                adapter: Arc::new(adapter),
                container,
                port,
            })
        }

        fn create_test_file(&self, name: &str, content: &str) -> (PathBuf, Vec<u8>) {
            let path = PathBuf::from(format!("test/{}", name));
            let data = content.as_bytes().to_vec();
            (path, data)
        }

        fn create_upload_options(&self, tags: Vec<String>) -> UploadOptions {
            let mut metadata = HashMap::new();
            metadata.insert("test".to_string(), "true".to_string());
            metadata.insert("created_by".to_string(), "integration_test".to_string());

            UploadOptions {
                metadata,
                tags,
                overwrite: true,
                ..Default::default()
            }
        }
    }

    #[tokio::test]
    async fn test_minio_connection_and_health_check() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;
        
        // Test health check
        let health = ctx.adapter.health_check().await?;
        assert!(health.is_available);
        assert!(health.response_time_ms.is_some());
        assert!(health.error.is_none());

        println!("MinIO connection successful: {}", ctx.port);
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Test concurrent uploads
        let upload_tasks: Vec<_> = (0..5).map(|i| {
            let adapter = ctx.adapter.clone();
            let path = PathBuf::from(format!("concurrent/file_{}.txt", i));
            let content = format!("Content of file {} with rust-s3", i);
            
            async move {
                adapter.upload_file(&path, content.as_bytes(), None).await
            }
        }).collect();

        let upload_results = try_join_all(upload_tasks).await?;
        assert_eq!(upload_results.len(), 5);

        // Test concurrent downloads
        let download_tasks: Vec<_> = (0..5).map(|i| {
            let adapter = ctx.adapter.clone();
            let path = PathBuf::from(format!("concurrent/file_{}.txt", i));
            
            async move {
                adapter.download_file(&path, None).await
            }
        }).collect();

        let download_results = try_join_all(download_tasks).await?;
        assert_eq!(download_results.len(), 5);

        // Verify content
        for (i, content) in download_results.iter().enumerate() {
            let expected = format!("Content of file {} with rust-s3", i);
            assert_eq!(content, expected.as_bytes());
        }

        println!("Concurrent operations test completed successfully");
        Ok(())
    }
    
    #[tokio::test]
    async fn test_upload_and_download_file() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Test file upload
        let (file_path, file_content) = ctx.create_test_file("hello.txt", "Hello, rust-s3 with MinIO!");
        let upload_options = ctx.create_upload_options(vec!["test".to_string(), "hello".to_string()]);

        let file_item = ctx.adapter.upload_file(&file_path, &file_content, Some(upload_options)).await?;
        
        assert_eq!(file_item.path, file_path);
        assert_eq!(file_item.size, file_content.len() as u64);
        assert!(file_item.content_type.is_some());
        assert!(file_item.md5_hash.is_some());
        assert!(!file_item.metadata.is_empty());

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

        println!("Upload/Download test passed for: {}", file_path.display());
        Ok(())
    }

    #[tokio::test]
    async fn test_file_operations_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Create and upload multiple test files
        let files = vec![
            ("analysis/code.rs", "fn main() { println!(\"Security analysis with rust-s3\"); }"),
            ("reports/audit.md", "# Security Audit Report\n\nAll systems secure with rust-s3."),
            ("data/config.json", r#"{"security": {"enabled": true, "backend": "rust-s3"}}"#),
        ];

        let mut uploaded_files = Vec::new();

        for (path_str, content) in &files {
            let path = PathBuf::from(path_str);
            let content_bytes = content.as_bytes().to_vec();
            let options = ctx.create_upload_options(vec!["lifecycle".to_string()]);

            let file_item = ctx.adapter.upload_file(&path, &content_bytes, Some(options)).await?;
            uploaded_files.push((path, file_item));
        }

        // Test file listing
        let list_options = ListOptions {
            prefix: Some("analysis/".to_string()),
            include_metadata: true,
            ..Default::default()
        };

        let file_list = ctx.adapter.list_files(Some(list_options)).await?;
        assert_eq!(file_list.files.len(), 1);
        assert_eq!(file_list.files[0].path, PathBuf::from("analysis/code.rs"));

        // Test copy operation
        let source_path = PathBuf::from("analysis/code.rs");
        let dest_path = PathBuf::from("backup/code_backup.rs");
        
        let copied_file = ctx.adapter.copy_file(&source_path, &dest_path).await?;
        assert_eq!(copied_file.path, dest_path);

        // Verify both files exist
        assert!(ctx.adapter.file_exists(&source_path).await?);
        assert!(ctx.adapter.file_exists(&dest_path).await?);

        // Test move operation
        let move_source = PathBuf::from("backup/code_backup.rs");
        let move_dest = PathBuf::from("archive/code_archived.rs");
        
        let moved_file = ctx.adapter.move_file(&move_source, &move_dest).await?;
        assert_eq!(moved_file.path, move_dest);

        // Verify source no longer exists, destination does
        assert!(!ctx.adapter.file_exists(&move_source).await?);
        assert!(ctx.adapter.file_exists(&move_dest).await?);

        // Test deletion
        ctx.adapter.delete_file(&move_dest).await?;
        assert!(!ctx.adapter.file_exists(&move_dest).await?);

        println!("File operations lifecycle test completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Prepare batch upload data
        let batch_files = vec![
            (PathBuf::from("batch/file1.txt"), "Content of file 1 with rust-s3".as_bytes().to_vec(), 
             Some(ctx.create_upload_options(vec!["batch".to_string(), "file1".to_string()]))),
            (PathBuf::from("batch/file2.txt"), "Content of file 2 with rust-s3".as_bytes().to_vec(),
             Some(ctx.create_upload_options(vec!["batch".to_string(), "file2".to_string()]))),
            (PathBuf::from("batch/file3.txt"), "Content of file 3 with rust-s3".as_bytes().to_vec(),
             Some(ctx.create_upload_options(vec!["batch".to_string(), "file3".to_string()]))),
        ];

        // Test batch upload
        let uploaded_items = ctx.adapter.upload_files(batch_files.clone()).await?;
        assert_eq!(uploaded_items.len(), 3);

        // Test batch download
        let download_paths = batch_files.iter().map(|(path, _, _)| path.clone()).collect();
        let downloaded_files = ctx.adapter.download_files(download_paths, None).await?;
        assert_eq!(downloaded_files.len(), 3);

        // Verify content matches
        for ((original_path, original_content, _), (downloaded_path, downloaded_content)) in 
            batch_files.iter().zip(downloaded_files.iter()) {
            assert_eq!(original_path, downloaded_path);
            assert_eq!(original_content, downloaded_content);
        }

        // Test batch file info
        let info_paths = batch_files.iter().map(|(path, _, _)| path.clone()).collect();
        let file_infos = ctx.adapter.get_files_info(info_paths).await?;
        assert_eq!(file_infos.len(), 3);

        // Test batch deletion
        let delete_paths = batch_files.iter().map(|(path, _, _)| path.clone()).collect();
        let deleted_paths = ctx.adapter.delete_files(delete_paths).await?;
        assert_eq!(deleted_paths.len(), 3);

        // Verify files are deleted
        for (path, _, _) in &batch_files {
            assert!(!ctx.adapter.file_exists(path).await?);
        }

        println!("Batch operations test completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_presigned_urls() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

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

        assert!(download_url.contains(&ctx.port.to_string()));
        assert!(download_url.contains("test.txt"));
        println!("Generated presigned download URL: {}", download_url);

        // Test presigned upload URL generation
        let upload_path = PathBuf::from("presigned/upload.txt");
        let upload_url = ctx.adapter.generate_upload_url(
            &upload_path,
            Duration::from_secs(3600),
            None
        ).await?;

        assert!(upload_url.contains(&ctx.port.to_string()));
        assert!(upload_url.contains("upload.txt"));
        println!("Generated presigned upload URL: {}", upload_url);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_filtering_and_search() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // create a temporary large json data with a bunch of "x"
        let large_json_data = &"x".repeat(1000); 
        // Upload files with different types and metadata
        let test_files = vec![
            ("documents/report.pdf", "PDF report content", vec!["document", "report"]),
            ("images/diagram.png", "PNG image data", vec!["image", "diagram"]),
            ("code/main.rs", "fn main() {}", vec!["code", "rust"]),
            ("code/script.py", "print('hello')", vec!["code", "python"]),
            ("data/small.json", "{}", vec!["data", "json"]),
            ("data/large.json", large_json_data, vec!["data", "json", "large"]),
        ];

        for (path_str, content, tags) in &test_files {
            let path = PathBuf::from(path_str);
            let content_bytes = content.as_bytes().to_vec();
            let options = ctx.create_upload_options(tags.iter().map(|s| s.to_string()).collect());
            
            ctx.adapter.upload_file(&path, &content_bytes, Some(options)).await?;
        }

        // Test filtering by prefix
        let code_files = ctx.adapter.list_files(Some(ListOptions {
            prefix: Some("code/".to_string()),
            ..Default::default()
        })).await?;
        assert_eq!(code_files.files.len(), 2);

        // Test filtering by extension
        let json_files = ctx.adapter.list_files(Some(ListOptions {
            extensions: vec!["json".to_string()],
            ..Default::default()
        })).await?;
        assert_eq!(json_files.files.len(), 2);

        // Test filtering by size
        let large_files = ctx.adapter.list_files(Some(ListOptions {
            min_size: Some(500),
            ..Default::default()
        })).await?;
        assert_eq!(large_files.files.len(), 1);
        assert!(large_files.files[0].path.to_string_lossy().contains("large.json"));

        println!("File filtering and search test completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_file_versioning() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        let file_path = PathBuf::from("versioned/document.txt");
        
        // Upload initial version
        let v1_content = "Version 1 content";
        let v1_file = ctx.adapter.upload_file_version(
            &file_path, 
            v1_content.as_bytes(), 
            Some(ctx.create_upload_options(vec!["version".to_string(), "v1".to_string()]))
        ).await?;

        // Upload second version
        let v2_content = "Version 2 content - updated";
        let v2_file = ctx.adapter.upload_file_version(
            &file_path,
            v2_content.as_bytes(),
            Some(ctx.create_upload_options(vec!["version".to_string(), "v2".to_string()]))
        ).await?;

        // List all versions
        let versions = ctx.adapter.list_file_versions(&file_path).await?;
        assert_eq!(versions.len(), 2);

        // Verify versions are different
        assert_ne!(v1_file.path, v2_file.path);
        assert!(v1_file.path.to_string_lossy().contains(&file_path.file_stem().unwrap().to_string_lossy().to_string()));
        assert!(v2_file.path.to_string_lossy().contains(&file_path.file_stem().unwrap().to_string_lossy().to_string()));

        println!("File versioning test completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_storage_statistics() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        let jpg_image_data = &"x".repeat(1000); 
        // Upload files of different types
        let test_files = vec![
            ("stats/doc1.pdf", "PDF content 1"),
            ("stats/doc2.pdf", "PDF content 2"),
            ("stats/image1.jpg", jpg_image_data),
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
        assert!(stats.files_by_type.contains_key("pdf"));
        assert!(stats.files_by_type.contains_key("jpg"));
        assert!(stats.files_by_type.contains_key("rs"));
        assert!(stats.size_by_type.contains_key("pdf"));

        println!("Storage statistics: {} files, {} bytes", stats.total_files, stats.total_size);
        println!("Files by type: {:?}", stats.files_by_type);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Test download non-existent file
        let non_existent = PathBuf::from("does/not/exist.txt");
        let result = ctx.adapter.download_file(&non_existent, None).await;
        assert!(result.is_err());

        // Test get info for non-existent file
        let result = ctx.adapter.get_file_info(&non_existent).await;
        assert!(result.is_err());

        // Test delete non-existent file (should succeed in S3/MinIO)
        let result = ctx.adapter.delete_file(&non_existent).await;
        assert!(result.is_ok()); // S3 delete is idempotent

        // Test upload with invalid path
        let invalid_path = PathBuf::from("../../../etc/passwd");
        let result = ctx.adapter.upload_file(&invalid_path, b"malicious", None).await;
        assert!(result.is_err());

        // Test file exists for non-existent file
        let exists = ctx.adapter.file_exists(&non_existent).await?;
        assert!(!exists);

        println!("Error handling test completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_end_to_end_security_audit_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Simulate security auditing workflow with rust-s3
        
        // 1. Upload code files for analysis
        let source_files = vec![
            ("audit/src/main.rs", "fn main() { println!(\"Security audit with rust-s3\"); }", vec!["source", "rust"]),
            ("audit/src/lib.rs", "pub mod security;", vec!["source", "rust"]),
            ("audit/config/app.toml", "[security]\nenabled = true\nbackend = \"rust-s3\"", vec!["config", "toml"]),
        ];

        let mut uploaded_files = Vec::new();
        for (path_str, content, tags) in &source_files {
            let path = PathBuf::from(path_str);
            let options = ctx.create_upload_options(tags.iter().map(|s| s.to_string()).collect());
            let file_item = ctx.adapter.upload_file(&path, content.as_bytes(), Some(options)).await?;
            uploaded_files.push(file_item);
        }

        // 2. Generate analysis report
        let report_content = format!(
            "# Security Audit Report (rust-s3 backend)\n\nAnalyzed {} files using rust-s3 with MinIO.\n\n## Summary\nNo critical issues found.",
            uploaded_files.len()
        );
        
        let report_path = PathBuf::from("audit/reports/security_report.md");
        let report_options = ctx.create_upload_options(vec!["report".to_string(), "security".to_string()]);
        let report_file = ctx.adapter.upload_file(&report_path, report_content.as_bytes(), Some(report_options)).await?;

        // 3. Create archive of analysis
        let archive_files: Vec<_> = uploaded_files.iter().map(|f| f.path.clone()).collect();
        let file_infos = ctx.adapter.get_files_info(archive_files).await?;
        
        let archive_summary = format!(
            "# Archive Summary (rust-s3)\n\nTotal files: {}\nTotal size: {} bytes\nBackend: rust-s3 with MinIO\n",
            file_infos.len(),
            file_infos.iter().map(|f| f.size).sum::<u64>()
        );
        
        let summary_path = PathBuf::from("audit/archive_summary.md");
        ctx.adapter.upload_file(&summary_path, archive_summary.as_bytes(), None).await?;

        // 4. Verify all files are accessible
        let audit_files = ctx.adapter.list_files(Some(ListOptions {
            prefix: Some("audit/".to_string()),
            include_metadata: true,
            ..Default::default()
        })).await?;

        assert!(audit_files.files.len() >= 5); // source files + report + summary
        
        // 5. Get storage stats for the audit
        let stats = ctx.adapter.get_storage_stats().await?;
        assert!(stats.total_files > 0);
        assert!(stats.total_size > 0);

        println!("End-to-end security audit workflow completed successfully with rust-s3");
        println!("Processed {} files, generated report: {}", 
                 audit_files.files.len(), report_file.path.display());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_rust_s3_specific_features() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = TestContext::new().await?;

        // Test rust-s3 specific functionality
        
        // 1. Test content type detection and setting
        let test_files = vec![
            ("types/document.json", r#"{"test": "json content type"}"#, "application/json"),
            ("types/stylesheet.css", "body { color: blue; }", "text/css"),
            ("types/script.js", "console.log('test');", "application/javascript"),
            ("types/readme.md", "# Markdown Test", "text/plain"),
        ];

        for (path_str, content, expected_type) in test_files {
            let path = PathBuf::from(path_str);
            let mut options = ctx.create_upload_options(vec!["content-type-test".to_string()]);
            options.content_type = Some(expected_type.to_string());
            
            let file_item = ctx.adapter.upload_file(&path, content.as_bytes(), Some(options)).await?;
            assert_eq!(file_item.content_type, Some(expected_type.to_string()));
        }

        // 2. Test metadata handling
        let metadata_path = PathBuf::from("metadata/test.txt");
        let mut metadata_options = ctx.create_upload_options(vec!["metadata-test".to_string()]);
        metadata_options.metadata.insert("custom-field".to_string(), "custom-value".to_string());
        metadata_options.metadata.insert("project".to_string(), "rust-s3-integration".to_string());

        let metadata_file = ctx.adapter.upload_file(
            &metadata_path, 
            b"Test content with metadata", 
            Some(metadata_options)
        ).await?;

        // Verify metadata was stored and retrieved
        let retrieved_info = ctx.adapter.get_file_info(&metadata_path).await?;
        assert!(retrieved_info.metadata.contains_key("custom-field"));
        assert_eq!(retrieved_info.metadata.get("custom-field"), Some(&"custom-value".to_string()));

        println!("rust-s3 specific features test completed successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_file_utility_functions() -> Result<(), Box<dyn std::error::Error>> {
        // Test content type detection
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type(&PathBuf::from("test.txt")),
            Some("text/plain".to_string())
        );
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type(&PathBuf::from("image.jpg")),
            Some("image/jpeg".to_string())
        );
        assert_eq!(
            <() as FileStorageUtils>::detect_content_type(&PathBuf::from("data.json")),
            Some("application/json".to_string())
        );

        // Test MD5 calculation
        let content = b"Hello, rust-s3!";
        let md5_hash = <() as FileStorageUtils>::calculate_md5(content);
        assert_eq!(md5_hash.len(), 32); // MD5 hash is 32 hex characters

        // Test path validation
        assert!( <() as FileStorageUtils>::validate_path(&PathBuf::from("valid/path.txt")).is_ok());
        assert!( <() as FileStorageUtils>::validate_path(&PathBuf::from("../invalid")).is_err());
        assert!( <() as FileStorageUtils>::validate_path(&PathBuf::from("/absolute")).is_err());
        assert!( <() as FileStorageUtils>::validate_path(&PathBuf::from("")).is_err());

        // Test filename sanitization
        assert_eq!(
             <() as FileStorageUtils>::sanitize_filename("file<name>.txt"),
            "file_name_.txt"
        );
        assert_eq!(
            <() as FileStorageUtils>::sanitize_filename("normal_file.txt"),
            "normal_file.txt"
        );

        println!("File utility functions test completed successfully");
        Ok(())
    }
}