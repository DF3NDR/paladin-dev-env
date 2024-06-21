// src/application/service/notifier.rs
use crate::domain::entities::notification::Notification;
use crate::domain::services::notification_service::NotificationService;

pub struct Notifier {
    notification_service: Box<dyn NotificationService>,
}

impl Notifier {
    pub fn new(notification_service: Box<dyn NotificationService>) -> Notifier {
        Notifier { notification_service }
    }

    pub fn send_notification(&self, notification: Notification) -> Result<(), String> {
        self.notification_service.send_notification(notification)
    }
}