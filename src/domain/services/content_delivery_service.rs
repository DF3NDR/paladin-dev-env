pub trait ContentDeliveryService {
    fn deliver_content(&self, content: &str) -> Result<(), String>;
}
