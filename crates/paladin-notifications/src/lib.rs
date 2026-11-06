//! # paladin-notifications
//!
//! Notification adapter implementations for the Paladin multi-agent framework.
//!
//! ## Feature flags
//!
//! | Flag     | Enables |
//! |----------|---------|
//! | `email`  | `email_notification_adapter` — requires `lettre` + `handlebars` |
//! | `push`   | `push_notification_adapter` (stub, no extra deps) |
//! | `system` | `system_notification_adapter` — in-memory system notifications |

#![warn(missing_docs)]

/// Email notification adapter (SMTP via `lettre`, templating via `handlebars`).
#[cfg(feature = "email")]
pub mod email_notification_adapter;

/// Push notification adapter (stub, no runtime dependencies).
#[cfg(feature = "push")]
pub mod push_notification_adapter;

/// System (in-app) notification adapter backed by in-memory storage.
#[cfg(feature = "system")]
pub mod system_notification_adapter;
