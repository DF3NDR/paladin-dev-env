/*
User Notification Service

This service provides an interface for sending notifications to users.
*/
use crate::core::domain::entities::user::User;
use crate::core::domain::entities::notification::Notification;

pub trait UserNotificationService {
    // Sends a notification to a user.
    // return might be better as a result with a custom nofication error type
    fn notify_user(&self, user: User, notification: Notification) -> bool;
}