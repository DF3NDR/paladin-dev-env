//! Identity scope types (Phase 40, TENANT-01/TENANT-02, ADR-0054).
//!
//! [`TenantId`], [`PrincipalRef`], [`RunAttribution`] and [`RunReadScope`] are pure value
//! types with no I/O. The tenant a caller belongs to is always **server-derived**: it is
//! looked up from `AgentAuthConfig` inside `paladin-web`'s `authenticate()` and nowhere
//! else (D-02) -- no request header, query parameter or body field may name, influence or
//! override it.
//!
//! [`RunReadScope`] is the **one shared read-scope rule** (D-12), applied by exactly two
//! mechanisms and nowhere else: the list path (`RunQuery.scope`, applied as a SQL
//! `WHERE tenant_id = ?` by every `RunRepositoryPort::list` adapter) and the single-run
//! path (`paladin-web`'s `load_visible_run` helper, which calls [`RunReadScope::permits`]).
//!
//! [`TenantId::OPEN_ACCESS`] and `LedgerScope::UNATTRIBUTED`
//! (`crate::platform::container::treasury_ledger::LedgerScope`) are deliberately different
//! literals: `"open-access"` names a principal that exists when auth is disabled (an Admin,
//! by the D-11 rule, sees every run); `"unattributed"` names the absence of any principal on
//! a ledger row. Never conflate the two.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::platform::container::run::Run;
use crate::platform::container::user::UserRole;

/// Maximum length, in bytes, of a [`TenantId`].
///
/// This is a **byte** limit (`str::len()`, the UTF-8 encoded length). `ThreadId` allows 256;
/// `TenantId` is bounded to 128 to keep the new `idx_runs_tenant_submitted` index key short.
pub const TENANT_ID_MAX_LEN: usize = 128;

/// Identity of a tenant: a plain identifier (D-00e -- not a Medieval-military officer word)
/// naming the organization or deployment scope an authenticated principal belongs to.
///
/// Validated non-empty, at most [`TENANT_ID_MAX_LEN`] bytes, printable ASCII only (no
/// whitespace -- an untrimmed value is rejected, never silently trimmed), so it is safe to
/// use as a storage key (SQL parameter, HTTP header) without further sanitization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TenantId(String);

/// Error returned by [`TenantId::new`] when the supplied string is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TenantIdError {
    /// The supplied tenant id was empty.
    #[error("tenant id must not be empty")]
    Empty,
    /// The supplied tenant id exceeded [`TENANT_ID_MAX_LEN`] bytes.
    #[error("tenant id must be at most {TENANT_ID_MAX_LEN} bytes, got {len}")]
    TooLong {
        /// The length of the rejected tenant id, in bytes.
        len: usize,
    },
    /// The supplied tenant id contained whitespace.
    #[error("tenant id must not contain whitespace")]
    ContainsWhitespace,
    /// The supplied tenant id contained a character outside printable ASCII.
    #[error("tenant id must be printable ASCII")]
    NotPrintableAscii,
}

impl TenantId {
    /// The sentinel tenant a deployment's open-access (auth disabled) principal carries.
    ///
    /// Deliberately distinct from `LedgerScope::UNATTRIBUTED`: this names a principal that
    /// exists (with `role = Admin`), not the absence of one.
    pub const OPEN_ACCESS: &'static str = "open-access";

    /// Construct a `TenantId`, validating non-empty, length, whitespace and printable ASCII,
    /// in that order.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::principal::TenantId;
    ///
    /// let tenant = TenantId::new("acme")?;
    /// assert_eq!(tenant.as_str(), "acme");
    /// assert!(TenantId::new("").is_err());
    /// assert!(TenantId::new("a b").is_err());
    /// # Ok::<(), paladin_core::platform::container::principal::TenantIdError>(())
    /// ```
    pub fn new(id: impl Into<String>) -> Result<Self, TenantIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(TenantIdError::Empty);
        }
        if id.len() > TENANT_ID_MAX_LEN {
            return Err(TenantIdError::TooLong { len: id.len() });
        }
        if id.chars().any(char::is_whitespace) {
            return Err(TenantIdError::ContainsWhitespace);
        }
        if id.chars().any(|c| !('!'..='~').contains(&c)) {
            return Err(TenantIdError::NotPrintableAscii);
        }
        Ok(Self(id))
    }

    /// Borrow the tenant id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct the documented open-access sentinel tenant ([`TenantId::OPEN_ACCESS`]).
    ///
    /// Built directly from the literal -- it is always valid, so this is infallible.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::principal::TenantId;
    ///
    /// assert_eq!(TenantId::open_access().as_str(), TenantId::OPEN_ACCESS);
    /// ```
    pub fn open_access() -> Self {
        Self(Self::OPEN_ACCESS.to_string())
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for TenantId {
    type Error = TenantIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TenantId> for String {
    fn from(value: TenantId) -> Self {
        value.0
    }
}

/// The submitting principal recorded on a [`Run`] (D-08).
///
/// `api_key_id` is the API key's configured name (or a bearer principal's id) -- never the
/// secret key value. Role is deliberately absent: roles are config, not data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunAttribution {
    /// The tenant this run is attributed to.
    pub tenant_id: TenantId,
    /// The submitting principal's API key id (or bearer principal id).
    pub api_key_id: String,
}

impl RunAttribution {
    /// Construct a `RunAttribution` from a tenant and an API key id.
    pub fn new(tenant_id: TenantId, api_key_id: impl Into<String>) -> Self {
        Self {
            tenant_id,
            api_key_id: api_key_id.into(),
        }
    }
}

/// A reference to the principal that submitted or is acting on a run (D-04).
///
/// Replaces the former `(String, UserRole)` tuple carried by `SubmitRun.requested_by`,
/// `ForkRun.requested_by` and `RunSubmissionPort::cancel`. `Option<PrincipalRef>` keeps the
/// existing "`None` = internal caller, skip the role check" semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalRef {
    /// The submitting principal's API key id (or bearer principal id).
    pub api_key_id: String,
    /// The tenant this principal belongs to.
    pub tenant_id: TenantId,
    /// The principal's role.
    pub role: UserRole,
}

impl PrincipalRef {
    /// Construct a `PrincipalRef`.
    pub fn new(api_key_id: impl Into<String>, tenant_id: TenantId, role: UserRole) -> Self {
        Self {
            api_key_id: api_key_id.into(),
            tenant_id,
            role,
        }
    }

    /// Derive the [`RunAttribution`] this principal stamps onto a run it submits, dropping
    /// the role (roles are config, not data, D-08).
    pub fn attribution(&self) -> RunAttribution {
        RunAttribution::new(self.tenant_id.clone(), self.api_key_id.clone())
    }
}

/// The one shared run-read authorization rule (D-12): who may see a given [`Run`].
///
/// Deliberately exhaustive (not `#[non_exhaustive]`): a future variant must fail every
/// adapter's compile rather than be silently ignored by a `_ => ...` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunReadScope {
    /// Every run is visible -- the Admin arm of [`RunReadScope::for_principal`] (D-11), and
    /// the default for internal callers that carry no principal at all.
    All,
    /// Only runs attributed to this tenant are visible.
    Tenant(TenantId),
}

impl Default for RunReadScope {
    /// `All` -- internal callers (workers, services, tests with no principal) are unchanged
    /// by this phase (D-12).
    fn default() -> Self {
        Self::All
    }
}

impl RunReadScope {
    /// Derive the read scope for a principal with the given role and tenant (D-11): Admin
    /// sees every run; a non-Admin principal is scoped to its own tenant.
    pub fn for_principal(role: UserRole, tenant_id: &TenantId) -> Self {
        match role {
            UserRole::Admin => Self::All,
            UserRole::User => Self::Tenant(tenant_id.clone()),
        }
    }

    /// Whether this scope permits reading the given run.
    ///
    /// A `Tenant` scope never permits an unattributed run (`run.submitted_by == None`) --
    /// only an `All` (Admin) scope can see those (D-10, D-11).
    pub fn permits(&self, run: &Run) -> bool {
        match self {
            Self::All => true,
            Self::Tenant(t) => run.submitted_by.as_ref().is_some_and(|a| &a.tenant_id == t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::container::run::{AssistantRef, RunId};
    use crate::platform::container::waypoint::ThreadId;

    fn tenant(s: &str) -> TenantId {
        TenantId::new(s).unwrap()
    }

    fn bare_run() -> Run {
        Run::new(
            RunId::new_v7(),
            ThreadId::new("t-1").unwrap(),
            AssistantRef {
                assistant_id: "wf".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        )
    }

    #[test]
    fn tenant_id_rejects_empty_whitespace_non_printable_and_overlong() {
        assert_eq!(TenantId::new(""), Err(TenantIdError::Empty));
        assert_eq!(
            TenantId::new(" acme"),
            Err(TenantIdError::ContainsWhitespace)
        );
        assert_eq!(TenantId::new("a b"), Err(TenantIdError::ContainsWhitespace));
        assert_eq!(TenantId::new("acmé"), Err(TenantIdError::NotPrintableAscii));
        let overlong = "a".repeat(TENANT_ID_MAX_LEN + 1);
        assert_eq!(
            TenantId::new(overlong.clone()),
            Err(TenantIdError::TooLong {
                len: overlong.len()
            })
        );
    }

    #[test]
    fn tenant_id_accepts_exactly_the_max_length() {
        let exact = "a".repeat(TENANT_ID_MAX_LEN);
        let id = TenantId::new(exact.clone()).unwrap();
        assert_eq!(id.as_str(), exact);
    }

    #[test]
    fn tenant_id_deserialize_rejects_an_invalid_value() {
        assert!(serde_json::from_str::<TenantId>("\"a b\"").is_err());
        let round_tripped: TenantId = serde_json::from_str("\"acme\"").unwrap();
        assert_eq!(round_tripped.as_str(), "acme");
    }

    #[test]
    fn open_access_tenant_is_the_documented_literal() {
        assert_eq!(TenantId::open_access().as_str(), "open-access");
        assert_eq!(TenantId::OPEN_ACCESS, "open-access");
    }

    #[test]
    fn principal_ref_attribution_drops_the_role() {
        let principal_ref = PrincipalRef::new("svc-a", tenant("acme"), UserRole::Admin);
        let attribution = principal_ref.attribution();
        assert_eq!(attribution.tenant_id, tenant("acme"));
        assert_eq!(attribution.api_key_id, "svc-a");
    }

    #[test]
    fn run_read_scope_default_is_all() {
        assert_eq!(RunReadScope::default(), RunReadScope::All);
    }

    #[test]
    fn run_read_scope_for_principal_admin_is_all_and_user_is_tenant() {
        assert_eq!(
            RunReadScope::for_principal(UserRole::Admin, &tenant("acme")),
            RunReadScope::All
        );
        assert_eq!(
            RunReadScope::for_principal(UserRole::User, &tenant("acme")),
            RunReadScope::Tenant(tenant("acme"))
        );
    }

    #[test]
    fn run_read_scope_tenant_permits_only_matching_attribution() {
        let mut run = bare_run();
        run.submitted_by = Some(RunAttribution::new(tenant("acme"), "svc-a"));
        let scope = RunReadScope::Tenant(tenant("acme"));
        assert!(scope.permits(&run));
        let other_scope = RunReadScope::Tenant(tenant("globex"));
        assert!(!other_scope.permits(&run));
        assert!(RunReadScope::All.permits(&run));
    }

    #[test]
    fn run_read_scope_tenant_never_permits_an_unattributed_run() {
        let run = bare_run();
        assert!(run.submitted_by.is_none());
        let scope = RunReadScope::Tenant(tenant("acme"));
        assert!(!scope.permits(&run));
        assert!(RunReadScope::All.permits(&run));
    }
}
