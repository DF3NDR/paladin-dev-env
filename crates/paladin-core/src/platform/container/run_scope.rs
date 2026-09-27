//! `RunScope` — the host grant a run carries into execution (Doc 05 RT-04,
//! D-21).
//!
//! A `RunScope` is how a host-issued Vault grant travels from wherever a run
//! is started down into `PaladinExecutionService::execute_scoped` and, through
//! the defaulted [`PaladinPort::execute_scoped`](
//! ../../../../paladin_ports/output/paladin_port/trait.PaladinPort.html#method.execute_scoped)
//! method, into a `WarEngine`-dispatched Paladin node. It lives in
//! `paladin-core` beside the Vault's own value types
//! ([`crate::platform::container::vault`]) because both the trait that
//! consumes it (`paladin-ports`) and the engine that constructs it
//! (`paladin-battalion`) need to name the same type, and neither of those
//! crates may depend on the other (ADR-0015: no new `paladin-core`
//! dependency was needed for this type either — it is built entirely from
//! `serde` and this crate's own [`crate::platform::container::vault::Namespace`]).
//!
//! # Forward compatibility (Phase 27, D-21)
//!
//! `RunScope` is `#[non_exhaustive]` with `Default` on purpose: Phase 27
//! (`PLAT-*`) will add fields such as `user_id`/`run_id`, derived from an
//! HTTP run request, and the non-exhaustive attribute is what makes that
//! additive rather than a semver break. Because a non-exhaustive struct
//! cannot be constructed by a literal (nor via `..Default::default()`
//! functional-update syntax) from outside this crate, [`RunScope::default`]
//! plus the [`RunScope::with_vault_namespace`] builder are the only way a
//! downstream crate ever builds one — exactly the shape that keeps working
//! once Phase 27 adds a field.

use serde::{Deserialize, Serialize};

use crate::platform::container::run::RunId;
use crate::platform::container::vault::Namespace;

/// The host-issued grant a single run carries (Doc 05 RT-04, D-21).
///
/// `vault_namespace` is resolved by `PaladinExecutionService::execute_scoped`
/// in a fixed order: the scope's own `vault_namespace` first, else the
/// service's own `with_vault` default, else **no grant at all** — a run
/// that resolves to no grant gets no [`ConfinedVault`](
/// https://docs.rs/paladin-ai) handle, never a handle silently granted the
/// root namespace. See `PaladinExecutionService::confined_vault`'s own
/// rustdoc for the full resolution rule and why "no grant" and "root grant"
/// are never conflated.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::run_scope::RunScope;
/// use paladin_core::platform::container::vault::Namespace;
///
/// let empty = RunScope::default();
/// assert!(empty.vault_namespace.is_none());
///
/// let ns = Namespace::parse("user/alice")?;
/// let scoped = RunScope::default().with_vault_namespace(ns.clone());
/// assert_eq!(scoped.vault_namespace, Some(ns));
/// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunScope {
    /// The Vault namespace this run is granted, if any. `None` means this
    /// scope itself carries no grant — the run may still receive one from
    /// `PaladinExecutionService::with_vault`'s own default, or none at all.
    pub vault_namespace: Option<Namespace>,

    /// The Platform API run this execution belongs to, set only by the run
    /// worker (39-07) when it dispatches an agent-kind run. `None` for
    /// engine nodes (which settle by superstep, never by this scope) and
    /// for plain HTTP agent calls that carry no Platform run at all. When
    /// present, the agent loop's `AgentLoopSettlement::EveryCall`/
    /// `PlatformRunsOnly` settle writer (D-07, 39-05) settles under this
    /// run id rather than the service's own execution id. Omitted from the
    /// serialized form when `None` (D-00f: additive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<RunId>,
}

impl RunScope {
    /// Builds a [`RunScope`] carrying `namespace` as its Vault grant. The
    /// only way to set `vault_namespace` on a `#[non_exhaustive]` struct
    /// from outside this crate.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::run_scope::RunScope;
    /// use paladin_core::platform::container::vault::Namespace;
    ///
    /// let ns = Namespace::parse("user/alice")?;
    /// let scope = RunScope::default().with_vault_namespace(ns.clone());
    /// assert_eq!(scope.vault_namespace, Some(ns));
    /// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
    /// ```
    #[must_use]
    pub fn with_vault_namespace(mut self, namespace: Namespace) -> Self {
        self.vault_namespace = Some(namespace);
        self
    }

    /// Builds a [`RunScope`] carrying `run_id` as the Platform API run this
    /// execution belongs to (D-07). The only way to set `run_id` on a
    /// `#[non_exhaustive]` struct from outside this crate.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::run::RunId;
    /// use paladin_core::platform::container::run_scope::RunScope;
    ///
    /// let run_id = RunId::new_v7();
    /// let scope = RunScope::default().with_run_id(run_id.clone());
    /// assert_eq!(scope.run_id, Some(run_id));
    /// ```
    #[must_use]
    pub fn with_run_id(mut self, run_id: RunId) -> Self {
        self.run_id = Some(run_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 1: `RunScope::default().vault_namespace` is `None`, and the
    /// type is non-exhaustive so a caller cannot construct it by exhaustive
    /// literal outside core (asserted here, in-crate, by grep in the
    /// plan's own acceptance criteria; this test pins the runtime half:
    /// the default value itself).
    #[test]
    fn run_scope_default_is_empty() {
        let scope = RunScope::default();
        assert!(scope.vault_namespace.is_none());
        assert!(scope.run_id.is_none());
    }

    #[test]
    fn run_scope_with_vault_namespace_sets_the_grant() {
        let ns = Namespace::parse("user/alice").unwrap();
        let scope = RunScope::default().with_vault_namespace(ns.clone());
        assert_eq!(scope.vault_namespace, Some(ns));
    }

    #[test]
    fn run_scope_round_trips_through_serde() {
        let ns = Namespace::parse("user/alice").unwrap();
        let scope = RunScope::default().with_vault_namespace(ns);
        let json = serde_json::to_string(&scope).unwrap();
        let back: RunScope = serde_json::from_str(&json).unwrap();
        assert_eq!(scope, back);
    }

    #[test]
    fn run_scope_with_run_id_sets_the_run_id() {
        let run_id = crate::platform::container::run::RunId::new_v7();
        let scope = RunScope::default().with_run_id(run_id.clone());
        assert_eq!(scope.run_id, Some(run_id));
    }

    /// A default scope serializes with no `run_id` key at all (D-00f:
    /// additive, `skip_serializing_if`), and a scope carrying one round-trips.
    #[test]
    fn run_scope_run_id_omitted_when_none_and_round_trips_when_some() {
        let empty = RunScope::default();
        let json = serde_json::to_string(&empty).unwrap();
        assert!(
            !json.contains("run_id"),
            "a None run_id must be omitted from the serialized form: {json}"
        );
        let back: RunScope = serde_json::from_str(&json).unwrap();
        assert_eq!(empty, back);

        let run_id = crate::platform::container::run::RunId::new_v7();
        let scoped = RunScope::default().with_run_id(run_id.clone());
        let json = serde_json::to_string(&scoped).unwrap();
        assert!(json.contains("run_id"));
        let back: RunScope = serde_json::from_str(&json).unwrap();
        assert_eq!(scoped, back);
    }
}
