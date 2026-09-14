/*
In-Memory Webhook Delivery Repository

An `Arc<tokio::sync::RwLock<Store>>`-backed implementation of
`WebhookDeliveryRepositoryPort`, for tests and local development (D-03's
InMemory convention). `claim_due` runs under the single write lock, so the
whole compare-and-set sequence is already serialized in-process -- the SQL
adapters (sqlite.rs, postgres.rs) are where the CAS primitive does real
cross-connection work.
*/

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::webhook::{
    WebhookAttemptOutcome, WebhookAttemptResult, WebhookDelivery, WebhookDeliveryId,
    WebhookDeliveryStatus,
};
use paladin_ports::output::webhook_delivery_port::{
    WebhookDeliveryPage, WebhookDeliveryRepositoryError, WebhookDeliveryRepositoryPort,
};

#[derive(Default)]
struct Store {
    deliveries: HashMap<WebhookDeliveryId, WebhookDelivery>,
}

/// In-memory `WebhookDeliveryRepositoryPort` implementation.
///
/// Cloning is cheap and shares the same underlying store (the inner `Arc`
/// is cloned).
#[derive(Clone, Default)]
pub struct InMemoryWebhookDeliveryRepository {
    store: Arc<RwLock<Store>>,
}

impl InMemoryWebhookDeliveryRepository {
    /// Construct a new, empty repository.
    pub fn new() -> Self {
        Self::default()
    }
}

fn is_claimable(delivery: &WebhookDelivery, now: DateTime<Utc>) -> bool {
    matches!(
        delivery.status,
        WebhookDeliveryStatus::Pending | WebhookDeliveryStatus::Retrying
    ) && delivery.next_attempt_at <= now
}

#[async_trait]
impl WebhookDeliveryRepositoryPort for InMemoryWebhookDeliveryRepository {
    async fn enqueue(
        &self,
        delivery: WebhookDelivery,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        let mut store = self.store.write().await;
        store
            .deliveries
            .insert(delivery.delivery_id.clone(), delivery);
        Ok(())
    }

    async fn get(
        &self,
        delivery_id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, WebhookDeliveryRepositoryError> {
        let store = self.store.read().await;
        Ok(store.deliveries.get(delivery_id).cloned())
    }

    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, WebhookDeliveryRepositoryError> {
        let mut store = self.store.write().await;

        let mut eligible_ids: Vec<WebhookDeliveryId> = store
            .deliveries
            .values()
            .filter(|d| is_claimable(d, now))
            .map(|d| d.delivery_id.clone())
            .collect();
        eligible_ids.sort_by_key(|id| {
            store
                .deliveries
                .get(id)
                .map(|d| d.next_attempt_at)
                .unwrap_or(now)
        });
        if limit > 0 {
            eligible_ids.truncate(limit as usize);
        }

        let mut claimed = Vec::with_capacity(eligible_ids.len());
        for id in eligible_ids {
            if let Some(delivery) = store.deliveries.get_mut(&id) {
                delivery.status = WebhookDeliveryStatus::InFlight;
                delivery.updated_at = Utc::now();
                claimed.push(delivery.clone());
            }
        }
        Ok(claimed)
    }

    async fn record_attempt(
        &self,
        delivery_id: &WebhookDeliveryId,
        result: WebhookAttemptResult,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        let mut store = self.store.write().await;
        let delivery = store.deliveries.get_mut(delivery_id).ok_or_else(|| {
            WebhookDeliveryRepositoryError::NotFound {
                delivery_id: delivery_id.clone(),
            }
        })?;

        delivery.attempt += 1;
        delivery.last_response_status = result.response_status;
        delivery.last_error = result.error;
        delivery.updated_at = Utc::now();
        match result.outcome {
            WebhookAttemptOutcome::Delivered => {
                delivery.status = WebhookDeliveryStatus::Delivered;
            }
            WebhookAttemptOutcome::Retrying { next_attempt_at } => {
                delivery.status = WebhookDeliveryStatus::Retrying;
                delivery.next_attempt_at = next_attempt_at;
            }
            WebhookAttemptOutcome::Dead => {
                delivery.status = WebhookDeliveryStatus::Dead;
            }
        }
        Ok(())
    }

    async fn list_for_run(
        &self,
        run_id: &RunId,
        limit: u32,
        cursor: Option<WebhookDeliveryId>,
    ) -> Result<WebhookDeliveryPage, WebhookDeliveryRepositoryError> {
        let store = self.store.read().await;
        let mut items: Vec<WebhookDelivery> = store
            .deliveries
            .values()
            .filter(|d| &d.run_id == run_id)
            .cloned()
            .collect();
        items.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.delivery_id.cmp(&a.delivery_id))
        });

        if let Some(cursor) = &cursor {
            let cut = items
                .iter()
                .position(|d| &d.delivery_id == cursor)
                .map(|idx| idx + 1)
                .unwrap_or(0);
            items = items.split_off(cut.min(items.len()));
        }

        let effective_limit = if limit == 0 {
            items.len()
        } else {
            limit as usize
        };
        let next_cursor = if items.len() > effective_limit {
            items
                .get(effective_limit.saturating_sub(1))
                .map(|last| last.delivery_id.clone())
        } else {
            None
        };
        items.truncate(effective_limit);
        Ok(WebhookDeliveryPage { items, next_cursor })
    }
}

#[cfg(test)]
mod contract_suite {
    use super::*;
    use crate::webhook::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent, per
    // `crate::run_schedule::in_memory`'s own convention), each against a
    // fresh `InMemoryWebhookDeliveryRepository`, so a failure names the
    // violated contract clause.

    #[tokio::test]
    async fn enqueue_creates_and_is_readable() {
        contract_tests::enqueue_creates_and_is_readable(&InMemoryWebhookDeliveryRepository::new())
            .await;
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown() {
        contract_tests::get_returns_none_for_unknown(&InMemoryWebhookDeliveryRepository::new())
            .await;
    }

    #[tokio::test]
    async fn claim_due_transitions_eligible_rows_to_in_flight() {
        contract_tests::claim_due_transitions_eligible_rows_to_in_flight(
            &InMemoryWebhookDeliveryRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn claim_due_respects_limit() {
        contract_tests::claim_due_respects_limit(&InMemoryWebhookDeliveryRepository::new()).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn claim_due_race_admits_each_row_once() {
        let repo: Arc<dyn WebhookDeliveryRepositoryPort> =
            Arc::new(InMemoryWebhookDeliveryRepository::new());
        contract_tests::claim_due_race_admits_each_row_once(repo).await;
    }

    #[tokio::test]
    async fn record_attempt_delivered_sets_terminal_status() {
        contract_tests::record_attempt_delivered_sets_terminal_status(
            &InMemoryWebhookDeliveryRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn record_attempt_retrying_sets_next_attempt_at() {
        contract_tests::record_attempt_retrying_sets_next_attempt_at(
            &InMemoryWebhookDeliveryRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn record_attempt_dead_sets_terminal_status() {
        contract_tests::record_attempt_dead_sets_terminal_status(
            &InMemoryWebhookDeliveryRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn record_attempt_on_unknown_returns_not_found() {
        contract_tests::record_attempt_on_unknown_returns_not_found(
            &InMemoryWebhookDeliveryRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn list_for_run_orders_descending_by_created_at() {
        contract_tests::list_for_run_orders_descending_by_created_at(
            &InMemoryWebhookDeliveryRepository::new(),
        )
        .await;
    }
}
