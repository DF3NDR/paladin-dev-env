use crate::core::domain::services::content_delivery_service::ContentDeliveryService;

pub struct DeliverContentUseCase<T: ContentDeliveryService> {
    service: T,
}

impl<T: ContentDeliveryService> DeliverContentUseCase<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, content: &str) -> Result<(), String> {
        self.service.deliver_content(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::services::content_delivery_service::ContentDeliveryService;

    struct MockContentDeliveryService;

    impl ContentDeliveryService for MockContentDeliveryService {
        fn deliver_content(&self, _content: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_deliver_content() {
        let service = MockContentDeliveryService;
        let use_case = DeliverContentUseCase::new(service);
        let result = use_case.execute("Test Content");
        assert!(result.is_ok());
    }
}

