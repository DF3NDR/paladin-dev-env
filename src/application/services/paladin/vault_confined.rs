//! `ConfinedVault` — the Vault's enforcement point (Doc 05 RT-04, D-20).
//!
//! Wraps an inner [`VaultPort`] behind a granted [`Namespace`] and denies
//! every call whose target namespace does not have the grant as a
//! [`Namespace::is_prefix_of`] ancestor, **before** the inner port is ever
//! touched. This is the single place a host-issued grant becomes an
//! enforced boundary: everything upstream of this file (a `RunScope`, an
//! Armament call, a graph author's own code) only ever sees the Vault
//! through this decorator once a grant is in play.
//!
//! # Absolute addressing, not grant-relative (D-20)
//!
//! `vault_get`/`vault_put` Armaments (a later plan) take an **absolute**
//! `namespace` argument, not one relative to the grant, and `ConfinedVault`
//! is built the same way: every method receives the caller's own full
//! `Namespace` and compares it against `self.granted` with
//! [`Namespace::is_prefix_of`]. The alternative — namespaces relative to the
//! grant — was considered and rejected: it would be harmless in isolation,
//! but it would make PRD 05 §3.4's attack test unexpressible (there would be
//! no way for a scripted tool call to even *name* a sibling namespace like
//! `["user","bob"]` from inside a `["user","alice"]` grant), and it would
//! permanently hide an agent's own grant from itself — `granted()` exists
//! precisely so an agent (or a test) can inspect what it was given.
//!
//! # Wrapping narrows, never widens
//!
//! A `ConfinedVault` wrapping another `ConfinedVault` composes like any
//! other decorator: the outer grant is checked first, and if it passes, the
//! call reaches the inner `ConfinedVault`, whose own grant is checked again.
//! There is no path by which composition can produce a wider effective
//! grant than the narrowest one in the chain — see
//! `confined_vault_is_a_vault_port_and_composes` below.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_ports::output::vault_port::{
    Namespace, Page, ScoredVaultRecord, VaultError, VaultPort, VaultRecord,
};

/// Decorates an inner [`VaultPort`] with a fixed [`Namespace`] grant,
/// denying every call outside it before the inner port is ever consulted.
///
/// See the module documentation for the absolute-addressing rationale and
/// the composition rule.
#[derive(Clone)]
pub struct ConfinedVault {
    inner: Arc<dyn VaultPort>,
    granted: Namespace,
}

impl ConfinedVault {
    /// Wraps `inner`, confining every call to `granted` and its
    /// descendants.
    pub fn new(inner: Arc<dyn VaultPort>, granted: Namespace) -> Self {
        Self { inner, granted }
    }

    /// The namespace this handle was granted — an agent (or a test) can
    /// always inspect its own grant, one of the two reasons D-20 chose
    /// absolute addressing over grant-relative namespaces.
    pub fn granted(&self) -> &Namespace {
        &self.granted
    }

    /// Returns `Err(VaultError::NamespaceDenied)` unless `self.granted` is a
    /// prefix of `ns` (`ns` is `self.granted` itself or one of its
    /// descendants). Called as the FIRST statement of every `VaultPort`
    /// method below — the check happens before `self.inner` is touched,
    /// which is the security property Test 2's zero-inner-call assertion
    /// exists to prove.
    fn check(&self, _ns: &Namespace) -> Result<(), VaultError> {
        // RED (plan 26-13 Task 1): deliberately wrong -- allows every call
        // regardless of grant, so the denial tests fail before the real
        // segment-wise check below replaces this.
        Ok(())
    }
}

impl fmt::Debug for ConfinedVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfinedVault")
            .field("granted", &self.granted)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ConfinedVault {
    /// Compares two handles by their granted namespace only — `inner` is a
    /// trait object with no `PartialEq` and, semantically, two handles
    /// granted the same namespace ARE the same handle for every purpose a
    /// caller (e.g. `NodeContext`, plan 26-13's Task 3) cares about,
    /// regardless of which `Arc` they happen to wrap.
    fn eq(&self, other: &Self) -> bool {
        self.granted == other.granted
    }
}

#[async_trait]
impl VaultPort for ConfinedVault {
    async fn put(
        &self,
        ns: &Namespace,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), VaultError> {
        self.check(ns)?;
        self.inner.put(ns, key, value).await
    }

    async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError> {
        self.check(ns)?;
        self.inner.get(ns, key).await
    }

    async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
        self.check(ns)?;
        self.inner.delete(ns, key).await
    }

    async fn list(
        &self,
        ns: &Namespace,
        prefix: Option<&str>,
        page: Page,
    ) -> Result<Vec<VaultRecord>, VaultError> {
        self.check(ns)?;
        self.inner.list(ns, prefix, page).await
    }

    async fn search(
        &self,
        ns: &Namespace,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ScoredVaultRecord>, VaultError> {
        self.check(ns)?;
        self.inner.search(ns, query, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A `VaultPort` that counts every call it actually receives, so a
    /// denial can be proven to have never reached the backend (Test 2).
    #[derive(Default)]
    struct CountingVault {
        calls: AtomicUsize,
    }

    impl CountingVault {
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl VaultPort for CountingVault {
        async fn put(
            &self,
            _ns: &Namespace,
            _key: &str,
            _value: serde_json::Value,
        ) -> Result<(), VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn get(
            &self,
            _ns: &Namespace,
            _key: &str,
        ) -> Result<Option<VaultRecord>, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }

        async fn delete(&self, _ns: &Namespace, _key: &str) -> Result<bool, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(false)
        }

        async fn list(
            &self,
            _ns: &Namespace,
            _prefix: Option<&str>,
            _page: Page,
        ) -> Result<Vec<VaultRecord>, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        async fn search(
            &self,
            _ns: &Namespace,
            _query: &str,
            _limit: u32,
        ) -> Result<Vec<ScoredVaultRecord>, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }
    }

    fn alice() -> Namespace {
        Namespace::parse("user/alice").unwrap()
    }

    fn alice_prefs() -> Namespace {
        Namespace::parse("user/alice/prefs").unwrap()
    }

    fn alice2() -> Namespace {
        Namespace::parse("user/alice2").unwrap()
    }

    fn bob() -> Namespace {
        Namespace::parse("user/bob").unwrap()
    }

    fn user_only() -> Namespace {
        Namespace::parse("user").unwrap()
    }

    // --- Test 1: confined_vault_allows_the_grant_and_its_descendants ----

    #[tokio::test]
    async fn confined_vault_allows_the_grant_and_its_descendants() {
        let inner = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(inner.clone(), alice());

        for ns in [alice(), alice_prefs()] {
            confined
                .put(&ns, "k", serde_json::json!(1))
                .await
                .expect("put on the grant or a descendant succeeds");
            confined
                .get(&ns, "k")
                .await
                .expect("get on the grant or a descendant succeeds");
            confined
                .delete(&ns, "k")
                .await
                .expect("delete on the grant or a descendant succeeds");
            confined
                .list(&ns, None, Page::default())
                .await
                .expect("list on the grant or a descendant succeeds");
            confined
                .search(&ns, "q", 5)
                .await
                .expect("search on the grant or a descendant succeeds");
        }

        assert_eq!(inner.calls(), 10, "every call for both namespaces reached the inner port");
    }

    // --- Test 2: confined_vault_denies_a_sibling_namespace --------------

    #[tokio::test]
    async fn confined_vault_denies_a_sibling_namespace() {
        let inner = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(inner.clone(), alice());

        let err = confined
            .get(&alice2(), "k")
            .await
            .expect_err("a sibling namespace must be denied");
        assert!(matches!(
            err,
            VaultError::NamespaceDenied { .. }
        ));
        assert_eq!(
            inner.calls(),
            0,
            "a denied call must never reach the inner port"
        );
    }

    // --- Test 3: confined_vault_denies_an_unrelated_namespace -----------

    #[tokio::test]
    async fn confined_vault_denies_an_unrelated_namespace() {
        let inner = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(inner.clone(), alice());

        let err = confined
            .get(&bob(), "k")
            .await
            .expect_err("an unrelated namespace must be denied");
        assert!(matches!(err, VaultError::NamespaceDenied { .. }));
        assert_eq!(inner.calls(), 0);
    }

    // --- Test 4: confined_vault_denies_a_parent_namespace ---------------

    #[tokio::test]
    async fn confined_vault_denies_a_parent_namespace() {
        let inner = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(inner.clone(), alice());

        let err = confined
            .get(&user_only(), "k")
            .await
            .expect_err("a grant does not confer access upward to its own parent");
        assert!(matches!(err, VaultError::NamespaceDenied { .. }));
        assert_eq!(inner.calls(), 0);
    }

    // --- Test 5: every_port_method_is_gated ------------------------------

    #[tokio::test]
    async fn every_port_method_is_gated() {
        let inner = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(inner.clone(), alice());

        assert!(matches!(
            confined
                .put(&bob(), "k", serde_json::json!(1))
                .await
                .unwrap_err(),
            VaultError::NamespaceDenied { .. }
        ));
        assert!(matches!(
            confined.get(&bob(), "k").await.unwrap_err(),
            VaultError::NamespaceDenied { .. }
        ));
        assert!(matches!(
            confined.delete(&bob(), "k").await.unwrap_err(),
            VaultError::NamespaceDenied { .. }
        ));
        assert!(matches!(
            confined
                .list(&bob(), None, Page::default())
                .await
                .unwrap_err(),
            VaultError::NamespaceDenied { .. }
        ));
        assert!(matches!(
            confined.search(&bob(), "q", 5).await.unwrap_err(),
            VaultError::NamespaceDenied { .. }
        ));

        assert_eq!(
            inner.calls(),
            0,
            "no gated method may ever reach the inner port on a foreign namespace"
        );
    }

    // --- Test 6: denial_names_both_namespaces ----------------------------

    #[tokio::test]
    async fn denial_names_both_namespaces() {
        let inner = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(inner, alice());

        let err = confined.get(&bob(), "k").await.unwrap_err();
        match err {
            VaultError::NamespaceDenied { requested, granted } => {
                assert_eq!(requested, bob());
                assert_eq!(granted, alice());
            }
            other => panic!("expected NamespaceDenied, got {other:?}"),
        }
    }

    // --- Test 7: confined_vault_is_a_vault_port_and_composes ------------

    #[tokio::test]
    async fn confined_vault_is_a_vault_port_and_composes() {
        let inner = Arc::new(CountingVault::default());
        // Outer grant is wider (`user`), inner grant narrows to `user/alice`.
        let outer = Arc::new(ConfinedVault::new(inner.clone(), user_only()));
        let narrowed = ConfinedVault::new(outer, alice());

        // Within the narrowed grant: allowed.
        narrowed
            .get(&alice(), "k")
            .await
            .expect("within the narrowed grant succeeds");
        assert_eq!(inner.calls(), 1);

        // A sibling of the narrowed grant, even though it is still within
        // the OUTER grant (`user`), is denied -- wrapping narrows, it never
        // widens back out.
        let err = narrowed
            .get(&bob(), "k")
            .await
            .expect_err("a sibling of the inner grant must still be denied");
        assert!(matches!(err, VaultError::NamespaceDenied { .. }));
        assert_eq!(
            inner.calls(),
            1,
            "the denied call must not reach the inner port either"
        );
    }

    // --- ConfinedVault: PartialEq/Debug/granted() ------------------------

    #[test]
    fn confined_vault_equality_and_accessors() {
        let inner_a: Arc<dyn VaultPort> = Arc::new(CountingVault::default());
        let inner_b: Arc<dyn VaultPort> = Arc::new(CountingVault::default());
        let inner_c: Arc<dyn VaultPort> = Arc::new(CountingVault::default());
        let a = ConfinedVault::new(inner_a, alice());
        let b = ConfinedVault::new(inner_b, alice());
        let c = ConfinedVault::new(inner_c, bob());

        assert_eq!(a, b, "two handles granted the same namespace are equal");
        assert_ne!(a, c, "two handles granted different namespaces are unequal");
        assert_eq!(a.granted(), &alice());
        assert!(format!("{a:?}").contains("alice"));
    }
}
