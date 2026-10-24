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
    /// The principal has a configured ceiling but the agent's model has no `treasurer.pricing`
    /// row, so no token budget can be derived and the call cannot be metered (Phase 42 D-10).
    ///
    /// A configuration incoherence rather than quota exhaustion: callers map it to a `422`
    /// without `Retry-After`, never to the pacing-shaped `429`.
    #[error("model {model} has no treasurer.pricing row, so an allowance cannot meter it")]
    ModelUnpriced {
        /// The model name that has no price row.
        model: String,
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

    /// Check `subject`'s allowances for an agent call against `model`, deriving the per-run token
    /// budget the agent loop enforces (ALLOW-05, Phase 42 D-09, D-10).
    ///
    /// The default body calls [`AllowanceAdmissionPort::admit`] and ignores `model`, so no
    /// existing implementor breaks and a port that does not meter derives no budget. An
    /// implementor that meters (the Treasurer) should override it: for a principal with a
    /// configured ceiling it derives the budget from the remaining allowance at the model's
    /// dearest price and returns it on the [`Admission`] ([`Admission::derived_budget`]).
    ///
    /// # Errors
    ///
    /// Everything [`AllowanceAdmissionPort::admit`] returns, plus
    /// [`AdmissionError::ModelUnpriced`] when a ceiling applies and `model` has no price row,
    /// and [`AdmissionError::Refused`] when the derived budget is zero tokens.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use paladin_core::platform::container::allowance::Admission;
    /// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    /// use paladin_core::platform::container::run::RunId;
    /// use paladin_ports::input::allowance_admission_port::{AdmissionError, AllowanceAdmissionPort};
    ///
    /// // An implementor of only the required methods: it inherits the default `admit_for_model`.
    /// struct AlwaysAdmits;
    ///
    /// #[async_trait]
    /// impl AllowanceAdmissionPort for AlwaysAdmits {
    ///     async fn admit(
    ///         &self,
    ///         _subject: &RunAttribution,
    ///         _run_id: Option<&RunId>,
    ///     ) -> Result<Admission, AdmissionError> {
    ///         Ok(Admission::none())
    ///     }
    ///
    ///     async fn confirm(&self, _admission: &Admission) {}
    ///
    ///     async fn abandon(&self, _admission: &Admission) {}
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let port = AlwaysAdmits;
    ///     let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
    ///     let admission = port.admit_for_model(&subject, None, "gpt-4").await?;
    ///     // The default delegates to `admit` and derives no budget.
    ///     assert!(admission.derived_budget().is_none());
    ///     Ok(())
    /// }
    /// ```
    async fn admit_for_model(
        &self,
        subject: &RunAttribution,
        run_id: Option<&RunId>,
        model: &str,
    ) -> Result<Admission, AdmissionError> {
        let _ = model;
        self.admit(subject, run_id).await
    }

    /// Called after the admitted run (or agent call) is durably accepted.
    async fn confirm(&self, admission: &Admission);

    /// Called when an admitted run (or agent call) was never accepted, so any notice this
    /// admission won can be released.
    async fn abandon(&self, admission: &Admission);
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::principal::TenantId;

    /// Implements only the required methods and counts `admit` calls.
    struct CountingPort(std::sync::atomic::AtomicUsize);

    #[async_trait]
    impl AllowanceAdmissionPort for CountingPort {
        async fn admit(
            &self,
            _subject: &RunAttribution,
            _run_id: Option<&RunId>,
        ) -> Result<Admission, AdmissionError> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Admission::none())
        }
        async fn confirm(&self, _admission: &Admission) {}
        async fn abandon(&self, _admission: &Admission) {}
    }

    #[tokio::test]
    async fn the_default_admit_for_model_delegates_to_admit_with_no_budget() {
        let port = CountingPort(std::sync::atomic::AtomicUsize::new(0));
        let subject = RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a");
        let admission = port
            .admit_for_model(&subject, None, "any-model")
            .await
            .unwrap();
        assert_eq!(port.0.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(admission.derived_budget().is_none());
    }

    #[test]
    fn model_unpriced_names_the_model_in_its_message() {
        let error = AdmissionError::ModelUnpriced {
            model: "gpt-9".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "model gpt-9 has no treasurer.pricing row, so an allowance cannot meter it"
        );
    }
}
