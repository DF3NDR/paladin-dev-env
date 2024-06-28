/*

Notification Manager

This is the Notification Manager, it is responsible for managing the delivery of 
notifications that are sent to a user. This module contains the Notification Manager 
Service, its related traits and implementations.

This service is meant to be a base on which multiple notification services can be built in
the application layer as use cases to send notifications to the user. Other services will
call this service to send notifications to the user and the Notification Manager 
will handle the details of how the notification are sent.

The user should be able to set preferences in the Application to determine how they receive
Notifications. The Notification Manager will use these preferences to determine how to send
the notification.

*/

use crate::core::platform::container::notification::Notification;


