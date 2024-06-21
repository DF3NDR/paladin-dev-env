use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Protocol {
    Http,
    Https,
    Ftp,
    Sftp,
    File,
    MySql,
    Ssh,
    Smb,
    S3,
    Gcs,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentSource {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub protocol: Protocol,
    pub prompt: String,
    pub tags: Vec<String>,
}
