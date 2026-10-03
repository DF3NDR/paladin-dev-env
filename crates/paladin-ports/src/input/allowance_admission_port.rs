//! Allowance Admission Port -- the admission-time check the Treasurer answers (ALLOW-02, C1)
//!
//! [`AllowanceAdmissionPort`] is the input port `paladin-web` and `RunSubmissionService` call
//! before a run (or an agent call) is accepted. It is core-typed only: `paladin-web` cannot
//! name the facade's `Treasurer` (that would be a crate cycle), so the facade implements this
//! port and the callers hold an `Arc<dyn AllowanceAdmissionPort>`.
//!
//! The Treasurer's identity input is a [`RunAttribution`] -- tenant and API key name only, never
//! a role -- so no role can bypass an allowance (D-09).
//!
//! ## Contract
//!
//! - **Admission is a check only** (D-05): no hold is placed and no ledger row is written. Holds
//!   and mid-run halts are a later phase's concern.
//! - **A refusal precedes every write** (ALLOW-02): [`AdmissionError::Refused`] must be answered
//!   before any run row, queue entry or ledger row exists.
//! - **Fail closed** (D-10): when a principal has a configured ceiling and the ledger cannot be
//!   read, the answer is [`AdmissionError::Backend`], never an admission.
//! - [`AllowanceAdmissionPort::confirm`] is called once the admitted run (or agent call) is
//!   durably accepted; [`AllowanceAdmissionPort::abandon`] when it never was.
//!
//! # Examples
//!
//! ```
//! use async_trait::async_trait;
//! use paladin_core::platform::container::allowance::Admission;
//! use paladin_core::platform::container::principal::{RunAttribution, TenantId};
//! use paladin_core::platform::container::run::RunId;
//! use paladin_ports::input::allowance_admission_port::{AdmissionError, AllowanceAdmissionPort};
//!
//! struct AlwaysAdmits;
//!
//! #[async_trait]
//! impl AllowanceAdmissionPort for AlwaysAdmits {
//!     async fn admit(
//!         &self,
//!         _subject: &RunAttribution,
//!         _run_id: Option<&RunId>,
//!     ) -> Result<Admission, AdmissionError> {
//!         Ok(Admission::none())
//!     }
//!
//!     async fn confirm(&self, _admission: &Admission) {}
//!
//!     async fn abandon(&self, _admission: &Admission) {}
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let port = AlwaysAdmits;
//!     let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
//!     let admission = port.admit(&subject, None).await?;
//!     assert!(admission.is_empty());
//!     port.confirm(&admission).await;
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::allowance::{Admission, AllowanceRefusal};
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::run::RunId;

/// Every way [`AllowanceAdmissionPort::admit`] can decline to admit.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AdmissionError {
    /// A balance has reached a configured ceiling; nothing may be persisted for this request.
    #[error("{0}")]
    Refused(AllowanceRefusal),
    /// The ledger could not be read for a principal that has a configured ceiling. Fail closed
    /// (D-10): the request is not admitted.
    #[error("allowance admission backend error: {message}")]
    Backend {
        /// Description of the backend failure.
        message: String,
    },
}

/// The admission-time allowance check (ALLOW-02, C1).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: admission runs concurrently across HTTP handlers.
#[async_trait]
pub trait AllowanceAdmissionPort: Send + Sync {
    /// Check `subject`'s allowances. `run_id` is the run being admitted, when one exists (the
    /// HTTP agent path has none).
    ///
    /// A principal with no configured allowance is admitted without any ledger read (D-03).
    /// Admission writes nothing (D-05).
    ///
    /// # Errors
    ///
    /// [`AdmissionError::Refused`] when a balance is at or above a ceiling, and
    /// [`AdmissionError::Backend`] when the ledger cannot be read (fail closed, D-10).
    async fn admit(
        &self,
        subject: &RunAttribution,
        run_id: Option<&RunId>,
    ) -> Result<Admission, AdmissionError>;

    /// Called after the admitted run (or agent call) is durably accepted.
    async fn confirm(&self, admission: &Admission);

    /// Called when an admitted run (or agent call) was never accepted, so any notice this
    /// admission won can be released.
    async fn abandon(&self, admission: &Admission);
}
