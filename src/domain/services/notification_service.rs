// src/domain/services/notification_service.rs
use crate::domain::entities::notification::Notification;

pub trait NotificationService {
    fn send_notification(&self, notification: Notification) -> Result<(), String>;
}