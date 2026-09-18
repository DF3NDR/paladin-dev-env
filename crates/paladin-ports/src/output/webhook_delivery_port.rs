//! Webhook Delivery Repository Port — a Durable Retry Queue (D-40)
//!
//! [`WebhookDeliveryRepositoryPort`] is the whole contract every backend
//! adapter (`InMemoryWebhookDeliveryRepository`,
//! `SqliteWebhookDeliveryRepository`, `PostgresWebhookDeliveryRepository`,
//! all `paladin-storage`) implements.
//!
//! ## `claim_due` is the whole durability story (D-40)
//!
//! A delivery is CLAIMED by a conditional update:
//! `Pending|Retrying -> InFlight` where `next_attempt_at <= now`. Two
//! concurrent `WebhookDeliveryService` drain-loop instances never both claim
//! the same row, and a restart between enqueue and send loses nothing --
//! the row is still `Pending`/`Retrying` on disk, waiting to be claimed by
//! whichever instance polls next.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::webhook::{
    WebhookAttemptResult, WebhookDelivery, WebhookDeliveryId,
};

/// A page of [`WebhookDelivery`]s returned by
/// [`WebhookDeliveryRepositoryPort::list_for_run`], ordered descending by
/// `created_at`.
#[derive(Debug, Clone, Default)]
pub struct WebhookDeliveryPage {
    /// The page's deliveries, in the documented order.
    pub items: Vec<WebhookDelivery>,
    /// Opaque cursor for the next page, `None` on the last page.
    pub next_cursor: Option<WebhookDeliveryId>,
}

/// Errors returned by [`WebhookDeliveryRepositoryPort`] methods (X-06 --
/// structured, never a bare `bool`/`String`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WebhookDeliveryRepositoryError {
    /// No delivery exists with the given id.
    #[error("webhook delivery not found: {delivery_id}")]
    NotFound {
        /// The requested delivery id.
        delivery_id: WebhookDeliveryId,
    },
    /// The underlying storage backend failed.
    #[error("webhook delivery repository backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) delivery could not be (de)serialized.
    #[error("webhook delivery serialization error: {message}")]
    Serialization {
        /// Description of the serialization failure.
        message: String,
    },
    /// A stored delivery carries a schema version this build does not know
    /// how to read.
    #[error("unsupported webhook delivery schema version: found {found}")]
    UnknownSchemaVersion {
        /// The schema version found on the stored data.
        found: String,
    },
}

/// Port trait for persisting and draining a durable webhook delivery queue
/// (D-40).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: deliveries are enqueued and
/// claimed concurrently across `WebhookDeliveryService` instances.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use std::sync::Mutex;
///
/// use async_trait::async_trait;
/// use chrono::{DateTime, Utc};
/// use paladin_core::platform::container::run::{RunEventKind, RunId};
/// use paladin_core::platform::container::waypoint::ThreadId;
/// use paladin_core::platform::container::webhook::{
///     WebhookAttemptResult, WebhookDelivery, WebhookDeliveryId,
/// };
/// use paladin_ports::output::webhook_delivery_port::{
///     WebhookDeliveryPage, WebhookDeliveryRepositoryError, WebhookDeliveryRepositoryPort,
/// };
///
/// struct InMemoryDeliveries {
///     rows: Mutex<HashMap<WebhookDeliveryId, WebhookDelivery>>,
/// }
///
/// #[async_trait]
/// impl WebhookDeliveryRepositoryPort for InMemoryDeliveries {
///     async fn enqueue(
///         &self,
///         delivery: WebhookDelivery,
///     ) -> Result<(), WebhookDeliveryRepositoryError> {
///         self.rows
///             .lock()
///             .unwrap()
///             .insert(delivery.delivery_id.clone(), delivery);
///         Ok(())
///     }
///
///     async fn get(
///         &self,
///         delivery_id: &WebhookDeliveryId,
///     ) -> Result<Option<WebhookDelivery>, WebhookDeliveryRepositoryError> {
///         Ok(self.rows.lock().unwrap().get(delivery_id).cloned())
///     }
///
///     async fn claim_due(
///         &self,
///         _now: DateTime<Utc>,
///         _limit: u32,
///     ) -> Result<Vec<WebhookDelivery>, WebhookDeliveryRepositoryError> {
///         Ok(Vec::new())
///     }
///
///     async fn record_attempt(
///         &self,
///         delivery_id: &WebhookDeliveryId,
///         _result: WebhookAttemptResult,
///     ) -> Result<(), WebhookDeliveryRepositoryError> {
///         if self.rows.lock().unwrap().contains_key(delivery_id) {
///             Ok(())
///         } else {
///             Err(WebhookDeliveryRepositoryError::NotFound {
///                 delivery_id: delivery_id.clone(),
///             })
///         }
///     }
///
///     async fn list_for_run(
///         &self,
///         run_id: &RunId,
///         _limit: u32,
///         _cursor: Option<WebhookDeliveryId>,
///     ) -> Result<WebhookDeliveryPage, WebhookDeliveryRepositoryError> {
///         let items = self
///             .rows
///             .lock()
///             .unwrap()
///             .values()
///             .filter(|d| &d.run_id == run_id)
///             .cloned()
///             .collect();
///         Ok(WebhookDeliveryPage {
///             items,
///             next_cursor: None,
///         })
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let repo = InMemoryDeliveries {
///         rows: Mutex::new(HashMap::new()),
///     };
///     let run_id = RunId::new_v7();
///     let delivery = WebhookDelivery::new(
///         WebhookDeliveryId::new_v7(),
///         run_id.clone(),
///         ThreadId::new("t1").unwrap(),
///         RunEventKind::Completed,
///         "https://example.invalid/hook",
///         "{}",
///         Utc::now(),
///     );
///
///     repo.enqueue(delivery).await.unwrap();
///     let page = repo.list_for_run(&run_id, 10, None).await.unwrap();
///     assert_eq!(page.items.len(), 1, "the enqueued delivery must be listed back");
/// }
/// ```
#[async_trait]
pub trait WebhookDeliveryRepositoryPort: Send + Sync {
    /// Persist a brand-new, `Pending` delivery.
    async fn enqueue(
        &self,
        delivery: WebhookDelivery,
    ) -> Result<(), WebhookDeliveryRepositoryError>;

    /// Load a delivery by id. `Ok(None)` if it does not exist -- never an
    /// error on its own.
    async fn get(
        &self,
        delivery_id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, WebhookDeliveryRepositoryError>;

    /// Claim up to `limit` deliveries that are `Pending` or `Retrying` with
    /// `next_attempt_at <= now`, atomically transitioning each claimed row
    /// to `InFlight` (D-40). Two concurrent callers never both claim the
    /// same row.
    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, WebhookDeliveryRepositoryError>;

    /// Record the result of one delivery attempt: increments the persisted
    /// `attempt` counter by one and applies `result`'s outcome (D-43).
    ///
    /// # Errors
    ///
    /// Returns [`WebhookDeliveryRepositoryError::NotFound`] when
    /// `delivery_id` does not exist.
    async fn record_attempt(
        &self,
        delivery_id: &WebhookDeliveryId,
        result: WebhookAttemptResult,
    ) -> Result<(), WebhookDeliveryRepositoryError>;

    /// Page through a run's deliveries, ordered descending by `created_at`.
    async fn list_for_run(
        &self,
        run_id: &RunId,
        limit: u32,
        cursor: Option<WebhookDeliveryId>,
    ) -> Result<WebhookDeliveryPage, WebhookDeliveryRepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // No full mock `impl WebhookDeliveryRepositoryPort` lives in this file:
    // the real proof of implementability is
    // `InMemoryWebhookDeliveryRepository` (`paladin-storage`), mirroring
    // `run_schedule_repository_port.rs`'s own convention. Object safety
    // alone is checked below.

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn WebhookDeliveryRepositoryPort>> = None;
    }

    #[test]
    fn webhook_delivery_page_default_is_empty() {
        let page = WebhookDeliveryPage::default();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }
}
