//! Configuration for MinIO file storage.

use crate::config::env_utils::EnvOverridable;
use crate::config::redaction::RedactedSecret;
use serde::{Deserialize, Serialize};

/// Configuration for MinIO file storage
///
/// `Debug` prints `minio_access_key` and `minio_secret_key` as `<redacted>` (DF-07).
#[derive(Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    pub minio_endpoint: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub minio_bucket: String,
    pub minio_region: Option<String>,
    pub minio_secure: Option<bool>,
    pub minio_path_style: Option<bool>,
    pub connection_timeout: Option<u64>,
    pub request_timeout: Option<u64>,
    pub max_idle_conns: Option<u32>,
    pub max_file_size: Option<u64>,
    pub allowed_extensions: Option<Vec<String>>,
}

impl std::fmt::Debug for FileStorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileStorageConfig")
            .field("minio_endpoint", &self.minio_endpoint)
            .field("minio_access_key", &RedactedSecret(&self.minio_access_key))
            .field("minio_secret_key", &RedactedSecret(&self.minio_secret_key))
            .field("minio_bucket", &self.minio_bucket)
            .field("minio_region", &self.minio_region)
            .field("minio_secure", &self.minio_secure)
            .field("minio_path_style", &self.minio_path_style)
            .field("connection_timeout", &self.connection_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("max_idle_conns", &self.max_idle_conns)
            .field("max_file_size", &self.max_file_size)
            .field("allowed_extensions", &self.allowed_extensions)
            .finish()
    }
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self {
            minio_endpoint: "localhost:9000".to_string(),
            minio_access_key: "minioadmin".to_string(),
            minio_secret_key: "minioadmin".to_string(),
            minio_bucket: "paladin-files".to_string(),
            minio_region: None,
            minio_secure: Some(false),
            minio_path_style: Some(true),
            connection_timeout: Some(30),
            request_timeout: Some(300),
            max_idle_conns: Some(10),
            max_file_size: Some(100 * 1024 * 1024), // 100MB
            allowed_extensions: Some(vec![
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
                "pdf".to_string(),
                "doc".to_string(),
                "docx".to_string(),
                "jpg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "html".to_string(),
                "css".to_string(),
                "xml".to_string(),
            ]),
        }
    }
}

impl EnvOverridable for FileStorageConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("APP_MINIO_ENDPOINT") {
            self.minio_endpoint = v;
        }
        if let Ok(v) = std::env::var("APP_MINIO_ACCESS_KEY") {
            self.minio_access_key = v;
        }
        if let Ok(v) = std::env::var("APP_MINIO_SECRET_KEY") {
            self.minio_secret_key = v;
        }
        if let Ok(v) = std::env::var("APP_MINIO_BUCKET") {
            self.minio_bucket = v;
        }
        if let Ok(v) = std::env::var("APP_MINIO_REGION") {
            self.minio_region = Some(v);
        }
        if let Ok(v) = std::env::var("APP_MINIO_SECURE")
            && let Ok(b) = v.parse::<bool>()
        {
            self.minio_secure = Some(b);
        }
        if let Ok(v) = std::env::var("APP_MINIO_PATH_STYLE")
            && let Ok(b) = v.parse::<bool>()
        {
            self.minio_path_style = Some(b);
        }
        if let Ok(v) = std::env::var("APP_MINIO_CONNECTION_TIMEOUT")
            && let Ok(t) = v.parse::<u64>()
        {
            self.connection_timeout = Some(t);
        }
        if let Ok(v) = std::env::var("APP_MINIO_REQUEST_TIMEOUT")
            && let Ok(t) = v.parse::<u64>()
        {
            self.request_timeout = Some(t);
        }
        if let Ok(v) = std::env::var("APP_MINIO_MAX_IDLE_CONNS")
            && let Ok(n) = v.parse::<u32>()
        {
            self.max_idle_conns = Some(n);
        }
        if let Ok(v) = std::env::var("APP_MINIO_MAX_FILE_SIZE")
            && let Ok(n) = v.parse::<u64>()
        {
            self.max_file_size = Some(n);
        }
        if let Ok(v) = std::env::var("APP_MINIO_ALLOWED_EXTENSIONS") {
            let exts: Vec<String> = v.split(',').map(|s| s.trim().to_lowercase()).collect();
            if !exts.is_empty() {
                self.allowed_extensions = Some(exts);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_default_file_storage_config() {
        let config = FileStorageConfig::default();
        assert_eq!(config.minio_endpoint, "localhost:9000");
        assert_eq!(config.minio_access_key, "minioadmin");
        assert_eq!(config.minio_secret_key, "minioadmin");
        assert_eq!(config.minio_bucket, "paladin-files");
        assert_eq!(config.minio_secure, Some(false));
        assert_eq!(config.minio_path_style, Some(true));
        assert_eq!(config.connection_timeout, Some(30));
        assert_eq!(config.request_timeout, Some(300));
        assert_eq!(config.max_idle_conns, Some(10));
        assert_eq!(config.max_file_size, Some(100 * 1024 * 1024));
        assert!(config.allowed_extensions.is_some());
    }

    #[test]
    #[serial]
    fn test_file_storage_config_env_override() {
        unsafe {
            env::set_var("APP_MINIO_ENDPOINT", "minio-server:9000");
            env::set_var("APP_MINIO_ACCESS_KEY", "testuser");
            env::set_var("APP_MINIO_SECRET_KEY", "testpass");
            env::set_var("APP_MINIO_BUCKET", "test-bucket");
            env::set_var("APP_MINIO_REGION", "us-east-1");
            env::set_var("APP_MINIO_SECURE", "true");
            env::set_var("APP_MINIO_PATH_STYLE", "false");
            env::set_var("APP_MINIO_CONNECTION_TIMEOUT", "60");
            env::set_var("APP_MINIO_REQUEST_TIMEOUT", "600");
            env::set_var("APP_MINIO_MAX_IDLE_CONNS", "20");
            env::set_var("APP_MINIO_MAX_FILE_SIZE", "209715200");
            env::set_var("APP_MINIO_ALLOWED_EXTENSIONS", "pdf,doc,docx,jpg,png");
        }

        let mut config = FileStorageConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.minio_endpoint, "minio-server:9000");
        assert_eq!(config.minio_access_key, "testuser");
        assert_eq!(config.minio_secret_key, "testpass");
        assert_eq!(config.minio_bucket, "test-bucket");
        assert_eq!(config.minio_region, Some("us-east-1".to_string()));
        assert_eq!(config.minio_secure, Some(true));
        assert_eq!(config.minio_path_style, Some(false));
        assert_eq!(config.connection_timeout, Some(60));
        assert_eq!(config.request_timeout, Some(600));
        assert_eq!(config.max_idle_conns, Some(20));
        assert_eq!(config.max_file_size, Some(209715200));
        assert_eq!(
            config.allowed_extensions,
            Some(vec![
                "pdf".to_string(),
                "doc".to_string(),
                "docx".to_string(),
                "jpg".to_string(),
                "png".to_string()
            ])
        );

        unsafe {
            env::remove_var("APP_MINIO_ENDPOINT");
            env::remove_var("APP_MINIO_ACCESS_KEY");
            env::remove_var("APP_MINIO_SECRET_KEY");
            env::remove_var("APP_MINIO_BUCKET");
            env::remove_var("APP_MINIO_REGION");
            env::remove_var("APP_MINIO_SECURE");
            env::remove_var("APP_MINIO_PATH_STYLE");
            env::remove_var("APP_MINIO_CONNECTION_TIMEOUT");
            env::remove_var("APP_MINIO_REQUEST_TIMEOUT");
            env::remove_var("APP_MINIO_MAX_IDLE_CONNS");
            env::remove_var("APP_MINIO_MAX_FILE_SIZE");
            env::remove_var("APP_MINIO_ALLOWED_EXTENSIONS");
        }
    }

    #[test]
    fn file_storage_config_debug_redacts_both_minio_keys() {
        let access = "test-key-not-a-secret-df07-minio-access-key";
        let secret = "test-key-not-a-secret-df07-minio-secret-key";
        let config = FileStorageConfig {
            minio_access_key: access.to_string(),
            minio_secret_key: secret.to_string(),
            ..FileStorageConfig::default()
        };
        // Non-vacuous: both fields really hold their sentinels.
        assert_eq!(config.minio_access_key, access);
        assert_eq!(config.minio_secret_key, secret);
        let text = format!("{config:?}");
        assert!(
            !text.contains(access),
            "FileStorageConfig Debug leaked the access key"
        );
        assert!(
            !text.contains(secret),
            "FileStorageConfig Debug leaked the secret key"
        );
        assert_eq!(text.matches("<redacted>").count(), 2, "markers in: {text}");
        assert!(text.contains("localhost:9000"), "endpoint missing: {text}");
        assert!(text.contains("paladin-files"), "bucket missing: {text}");
    }
}
