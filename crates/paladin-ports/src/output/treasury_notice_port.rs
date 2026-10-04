//! Treasury Notice Port -- the Treasurer's once-per-window allowance notices (ALLOW-04, D-16)
//!
//! [`TreasuryNoticePort`] records, reads back and releases the durable notice a warn-threshold
//! crossing produces. It is a sibling of
//! [`TreasuryLedgerPort`](crate::output::treasury_ledger_port::TreasuryLedgerPort) -- the ledger
//! port is unchanged -- and reuses its
//! [`TreasuryLedgerError`](crate::output::treasury_ledger_port::TreasuryLedgerError).
//!
//! ## Store-enforced deduplication (D-16)
//!
//! [`TreasuryNoticePort::record`] must compile to a store-enforced constraint -- a unique index
//! over `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling)` with
//! `INSERT ... ON CONFLICT ... DO NOTHING` -- never an application-level "have I already sent
//! this?" check (the Phase 39 `settle` precedent, D-06). Zero rows written is
//! [`NoticeOutcome::AlreadyRecorded`], never an error: losing a claim is the normal outcome for
//! every admission after the first in a window, on any replica. A tenant-scope notice is stored
//! with an empty `api_key_id` and a lifetime notice with the Unix epoch as its `window_start`,
//! so no key column is ever NULL (a NULL in a unique key never conflicts).
//!
//! ## Policy-free
//!
//! This port knows no allowance policy and no warn threshold: the Treasurer decides *whether* a
//! crossing happened and hands over a complete [`NoticeRecord`]; the store only guarantees the
//! identity is recorded at most once.
//!
//! # Examples
//!
//! A mock that records into a `Mutex<Vec<_>>`:
//!
//! ```
//! use std::sync::Mutex;
//!
//! use async_trait::async_trait;
//! use chrono::{TimeZone, Utc};
//! use paladin_core::platform::container::allowance::{
//!     AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning, NoticeOutcome, NoticeRecord,
//! };
//! use paladin_core::platform::container::cost::{Cost, CurrencyCode};
//! use paladin_core::platform::container::run::RunId;
//! use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
//! use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
//!
//! #[derive(Default)]
//! struct MockNotices {
//!     rows: Mutex<Vec<NoticeRecord>>,
//! }
//!
//! #[async_trait]
//! impl TreasuryNoticePort for MockNotices {
//!     async fn record(&self, notice: &NoticeRecord) -> Result<NoticeOutcome, TreasuryLedgerError> {
//!         let mut rows = self.rows.lock().map_err(|e| TreasuryLedgerError::InvalidRequest {
//!             message: e.to_string(),
//!         })?;
//!         let same = |r: &NoticeRecord| {
//!             r.tenant_id == notice.tenant_id
//!                 && r.api_key_id == notice.api_key_id
//!                 && r.warning.scope_kind == notice.warning.scope_kind
//!                 && r.warning.limit_kind == notice.warning.limit_kind
//!                 && r.warning.window_start == notice.warning.window_start
//!                 && r.warning.ceiling == notice.warning.ceiling
//!         };
//!         if rows.iter().any(same) {
//!             return Ok(NoticeOutcome::AlreadyRecorded);
//!         }
//!         rows.push(notice.clone());
//!         Ok(NoticeOutcome::Recorded)
//!     }
//!
//!     async fn notices_for_run(
//!         &self,
//!         run_id: &RunId,
//!     ) -> Result<Vec<NoticeRecord>, TreasuryLedgerError> {
//!         let rows = self.rows.lock().map_err(|e| TreasuryLedgerError::InvalidRequest {
//!             message: e.to_string(),
//!         })?;
//!         Ok(rows
//!             .iter()
//!             .filter(|r| r.run_id.as_ref() == Some(run_id))
//!             .cloned()
//!             .collect())
//!     }
//!
//!     async fn discard(&self, notice_ids: &[String]) -> Result<(), TreasuryLedgerError> {
//!         let mut rows = self.rows.lock().map_err(|e| TreasuryLedgerError::InvalidRequest {
//!             message: e.to_string(),
//!         })?;
//!         rows.retain(|r| !notice_ids.contains(&r.notice_id));
//!         Ok(())
//!     }
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let usd = CurrencyCode::new("USD")?;
//! let record = NoticeRecord {
//!     notice_id: "n-1".to_string(),
//!     tenant_id: "acme".to_string(),
//!     api_key_id: None,
//!     warning: AllowanceWarning {
//!         scope_kind: AllowanceScopeKind::Tenant,
//!         limit_kind: AllowanceLimitKind::Lifetime,
//!         balance: Cost::new(80, usd.clone()),
//!         ceiling: Cost::new(100, usd),
//!         window_start: None,
//!         window_end: None,
//!         warn_at: 80,
//!     },
//!     run_id: Some(RunId::new_v7()),
//!     recorded_at: Utc.timestamp_opt(0, 0).single().ok_or("bad instant")?,
//! };
//!
//! let notices = MockNotices::default();
//! assert_eq!(notices.record(&record).await?, NoticeOutcome::Recorded);
//! assert_eq!(notices.record(&record).await?, NoticeOutcome::AlreadyRecorded);
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;

use paladin_core::platform::container::allowance::{NoticeOutcome, NoticeRecord};
use paladin_core::platform::container::run::RunId;

use crate::output::treasury_ledger_port::TreasuryLedgerError;

/// Port trait for the Treasurer's durable once-per-window allowance notices (ALLOW-04, D-16).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: admissions on every replica claim notices
/// concurrently, and the worker reads them back by run id.
#[async_trait]
pub trait TreasuryNoticePort: Send + Sync {
    /// Claim `notice`'s identity: the first claim writes the row and returns
    /// [`NoticeOutcome::Recorded`]; every later claim of the same identity -- the same scope,
    /// tenant, API key, limit kind, window start and ceiling -- writes nothing and returns
    /// [`NoticeOutcome::AlreadyRecorded`].
    ///
    /// Deduplication is enforced by the store (a unique index plus `ON CONFLICT DO NOTHING`),
    /// so it holds across every replica sharing the store. `AlreadyRecorded` is never an error.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::InvalidRequest`] (before any I/O) for an empty
    /// `notice_id` or `tenant_id`, an API-key-scope notice without a key, a tenant-scope notice
    /// with one, a `warn_at` above 100, a window notice without both window bounds, a lifetime
    /// notice carrying one, or a ceiling and balance in different currencies; and
    /// [`TreasuryLedgerError::Backend`] when the store fails.
    async fn record(&self, notice: &NoticeRecord) -> Result<NoticeOutcome, TreasuryLedgerError>;

    /// Every notice recorded with `run_id` as its admitting run, oldest first.
    ///
    /// A lifetime notice reads back with both window bounds `None`, and a tenant-scope notice
    /// with `api_key_id` `None`, exactly as it was recorded.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::Backend`] when the store fails, or
    /// [`TreasuryLedgerError::Serialization`] when a stored row cannot be decoded.
    async fn notices_for_run(
        &self,
        run_id: &RunId,
    ) -> Result<Vec<NoticeRecord>, TreasuryLedgerError>;

    /// Delete exactly the notices named by `notice_ids` -- the notices one abandoned admission
    /// won -- so the next admission in the same window wins them again. An id that names no row
    /// is not an error.
    ///
    /// # Errors
    ///
    /// Returns [`TreasuryLedgerError::Backend`] when the store fails.
    async fn discard(&self, notice_ids: &[String]) -> Result<(), TreasuryLedgerError>;
}
