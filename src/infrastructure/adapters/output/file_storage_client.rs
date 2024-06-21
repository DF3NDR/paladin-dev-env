// src/adapters/primary/file_content_deliverer.rs
use std::fs::File;
use std::io::Write;
use crate::domain::services::content_delivery_service::ContentDeliveryService;

pub struct FileContentDeliverer;

impl ContentDeliveryService for FileContentDeliverer {
    fn deliver_content(&self, content: &str) -> Result<(), String> {
        let mut file = File::create("output.txt").map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_deliver_content() {
        let deliverer = FileContentDeliverer;
        let content = "Hello, world!";

        deliverer.deliver_content(content).unwrap();

        let delivered_content = fs::read_to_string("output.txt").unwrap();
        assert_eq!(delivered_content, content);
    }
}
