// src/domain/services/content_delivery_service.rs

pub trait ContentDeliveryService {
    fn deliver_content(&self, content: &str) -> Result<(), String>;
}
