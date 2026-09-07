//! Shared `VaultPort` contract suite (D-18/D-24, mirroring
//! `paladin-storage`'s `node_cache::contract_tests` shape).
//!
//! One `pub async fn` per contract clause, each taking `&dyn VaultPort` and
//! asserting inside. `InMemoryVault` is the first adapter instantiated
//! against this suite (this plan); plan 26-09 instantiates `SqliteVault` and
//! `SemanticVault` against the SAME list -- a new Vault adapter is not
//! "done" until it appears here. Named per-clause (not a declarative macro)
//! so a failing case names the violated contract clause directly, rather
//! than a line number inside a shared macro expansion.
//!
//! Case 10 (`assert_search_unsupported`) is deliberately NOT part of the
//! shared put/get/delete/list contract -- it is an adapter-specific
//! capability assertion. `InMemoryVault` and `SqliteVault` call it (neither
//! has embeddings); `SemanticVault` does not (it asserts `search` actually
//! works instead, via case 11, `search_returns_scored_records_from_the_store`).
//!
//! This module is plain (not `#[cfg(test)]`) so it is reachable from every
//! adapter's own `#[cfg(test)]` module regardless of which crate that
//! adapter eventually lives in.

use std::sync::Arc;

use paladin_core::platform::container::vault::{DEFAULT_MAX_VALUE_BYTES, Namespace, Page};
use paladin_ports::output::vault_port::{VaultError, VaultPort};
use serde_json::json;

/// Contract case 1: a `put` followed by a `get` of the same key returns a
/// record whose value round-trips and whose `created_at == updated_at`
/// (nothing has overwritten it yet).
pub async fn put_then_get_returns_the_record(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract/put-then-get").unwrap();
    port.put(&ns, "k", json!(1)).await.unwrap();

    let record = port.get(&ns, "k").await.unwrap().expect("expected a hit");
    assert_eq!(record.value(), &json!(1));
    assert_eq!(record.created_at(), record.updated_at());
}

/// Contract case 2: a second `put` on the same `(ns, key)` changes the
/// value, bumps `updated_at`, and leaves `created_at` unchanged.
pub async fn put_overwrites_and_preserves_created_at(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract/overwrite").unwrap();
    port.put(&ns, "k", json!(1)).await.unwrap();
    let first = port
        .get(&ns, "k")
        .await
        .unwrap()
        .expect("first put must hit");

    port.put(&ns, "k", json!(2)).await.unwrap();
    let second = port
        .get(&ns, "k")
        .await
        .unwrap()
        .expect("second put must hit");

    assert_eq!(second.value(), &json!(2));
    assert_eq!(second.created_at(), first.created_at());
    assert!(second.updated_at() >= first.updated_at());
}

/// Contract case 3: the first `delete` of a key returns `Ok(true)`, the
/// second returns `Ok(false)`, and a subsequent `get` returns `Ok(None)`.
pub async fn delete_returns_true_then_false(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract/delete").unwrap();
    port.put(&ns, "k", json!(1)).await.unwrap();

    assert!(port.delete(&ns, "k").await.unwrap());
    assert!(!port.delete(&ns, "k").await.unwrap());
    assert!(port.get(&ns, "k").await.unwrap().is_none());
}

/// Contract case 4: with records under `["a"]`, `["a","b"]` and `["c"]`,
/// `list(["a"], None, Page::default())` returns only the `["a"]` records --
/// NOT the `["a","b"]` descendant -- ordered by key ascending.
pub async fn list_returns_only_this_namespace_ordered_by_key(port: &dyn VaultPort) {
    let ns_a = Namespace::parse("contract-scope/a").unwrap();
    let ns_a_b = Namespace::parse("contract-scope/a/b").unwrap();
    let ns_c = Namespace::parse("contract-scope/c").unwrap();

    port.put(&ns_a, "z", json!("a-z")).await.unwrap();
    port.put(&ns_a, "m", json!("a-m")).await.unwrap();
    port.put(&ns_a_b, "x", json!("ab-x")).await.unwrap();
    port.put(&ns_c, "y", json!("c-y")).await.unwrap();

    let records = port.list(&ns_a, None, Page::default()).await.unwrap();
    let keys: Vec<&str> = records.iter().map(|r| r.key()).collect();

    assert_eq!(
        keys,
        vec!["m", "z"],
        "must contain only ns_a's own records, ordered by key ascending, \
         excluding the ns_a/b descendant"
    );
}

/// Contract case 5: `list(ns, Some("cfg."), ..)` returns only keys starting
/// with `cfg.`.
pub async fn list_filters_by_key_prefix(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract-prefix/ns").unwrap();
    port.put(&ns, "cfg.a", json!(1)).await.unwrap();
    port.put(&ns, "cfg.b", json!(2)).await.unwrap();
    port.put(&ns, "other", json!(3)).await.unwrap();

    let records = port.list(&ns, Some("cfg."), Page::default()).await.unwrap();
    let keys: Vec<&str> = records.iter().map(|r| r.key()).collect();

    assert_eq!(keys, vec!["cfg.a", "cfg.b"]);
}

/// Contract case 6: with 5 records and `Page { limit: 2, after: None }`
/// then two follow-up pages using the last returned key, every record is
/// returned exactly once, in key order, with no overlap and no gap.
pub async fn list_paginates_by_opaque_after_cursor(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract-page/ns").unwrap();
    let all_keys = ["k1", "k2", "k3", "k4", "k5"];
    for key in all_keys {
        port.put(&ns, key, json!(key)).await.unwrap();
    }

    let mut collected: Vec<String> = Vec::new();
    let mut after: Option<String> = None;

    loop {
        let page = Page::new(2, after.clone()).unwrap();
        let records = port.list(&ns, None, page).await.unwrap();
        if records.is_empty() {
            break;
        }
        for record in &records {
            collected.push(record.key().to_string());
        }
        after = Some(records.last().unwrap().key().to_string());
        if collected.len() >= all_keys.len() {
            break;
        }
    }

    assert_eq!(
        collected,
        all_keys.to_vec(),
        "every record must be returned exactly once, in key order, with no overlap or gap"
    );
}

/// Contract case 7: `list` on a namespace with no records returns
/// `Ok(vec![])`, never an error.
pub async fn list_on_an_empty_namespace_is_an_empty_page_not_an_error(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract-empty/ns").unwrap();
    let records = port.list(&ns, None, Page::default()).await.unwrap();
    assert!(records.is_empty());
}

/// Contract case 8: a record under `["user","alice"]` is invisible to
/// `get`/`list` under `["user","bob"]` and under `["user","alice2"]` (the
/// sibling-namespace case).
pub async fn namespaces_are_isolated(port: &dyn VaultPort) {
    let alice = Namespace::parse("contract-iso/user/alice").unwrap();
    let bob = Namespace::parse("contract-iso/user/bob").unwrap();
    let alice2 = Namespace::parse("contract-iso/user/alice2").unwrap();

    port.put(&alice, "secret", json!("alice's value"))
        .await
        .unwrap();

    assert!(port.get(&bob, "secret").await.unwrap().is_none());
    assert!(port.get(&alice2, "secret").await.unwrap().is_none());
    assert!(
        port.list(&bob, None, Page::default())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        port.list(&alice2, None, Page::default())
            .await
            .unwrap()
            .is_empty()
    );

    // Sanity: alice's own namespace still has it.
    assert!(port.get(&alice, "secret").await.unwrap().is_some());
}

/// Contract case 9: a value serializing above `DEFAULT_MAX_VALUE_BYTES`
/// fails with `ValueTooLarge { bytes, max }`, and nothing is stored.
pub async fn value_larger_than_the_bound_is_rejected(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract-bound/ns").unwrap();
    let huge_value = json!("x".repeat(DEFAULT_MAX_VALUE_BYTES + 1));

    let err = port.put(&ns, "k", huge_value).await.unwrap_err();
    match err {
        VaultError::ValueTooLarge { bytes, max } => {
            assert!(bytes > max);
        }
        other => panic!("expected ValueTooLarge, got {other:?}"),
    }

    assert!(
        port.get(&ns, "k").await.unwrap().is_none(),
        "a rejected put must not store anything"
    );
}

/// Contract case 10 (adapter-specific capability assertion, NOT part of the
/// shared put/get/delete/list contract): `search` returns
/// `Err(VaultError::Unsupported { operation: "search" })` on an adapter
/// with no search capability. Called by `InMemoryVault` and `SqliteVault`;
/// NOT called by `SemanticVault`, which asserts `search` actually works
/// instead.
pub async fn assert_search_unsupported(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract-search/ns").unwrap();
    let err = port.search(&ns, "query", 5).await.unwrap_err();
    assert!(matches!(
        err,
        VaultError::Unsupported {
            operation: "search"
        }
    ));
}

/// Smoke aggregate: runs every shared contract clause (1-9) in sequence
/// against a single fresh port, mirroring `node_cache::contract_tests::run_all`.
/// Individual adapters should still invoke each function from its own named
/// test (so a failure names the violated clause) -- this exists as a
/// single-call convenience, not a replacement.
pub async fn run_all_shared_clauses(port: &dyn VaultPort) {
    put_then_get_returns_the_record(port).await;
    put_overwrites_and_preserves_created_at(port).await;
    delete_returns_true_then_false(port).await;
    list_returns_only_this_namespace_ordered_by_key(port).await;
    list_filters_by_key_prefix(port).await;
    list_paginates_by_opaque_after_cursor(port).await;
    list_on_an_empty_namespace_is_an_empty_page_not_an_error(port).await;
    namespaces_are_isolated(port).await;
    value_larger_than_the_bound_is_rejected(port).await;
}

/// Convenience: build an `Arc<dyn VaultPort>` from any concrete adapter, for
/// call sites that want the trait-object form.
pub fn as_dyn(port: impl VaultPort + 'static) -> Arc<dyn VaultPort> {
    Arc::new(port)
}

/// Contract case 11 (search-capable adapters only, plan 26-09): called by
/// `SemanticVault`, NOT by `InMemoryVault`/`SqliteVault`, which report
/// `Unsupported` instead -- see [`assert_search_unsupported`].
///
/// After putting three records under one namespace, `search` returns
/// `ScoredVaultRecord`s in descending-score order, each carrying the SAME
/// value `get` would return for that key -- proving the returned record is
/// the store's authoritative copy, never a value reconstructed from the
/// vector backend's own payload (D-24).
pub async fn search_returns_scored_records_from_the_store(port: &dyn VaultPort) {
    let ns = Namespace::parse("contract-search-real/ns").unwrap();
    port.put(&ns, "a", json!("first")).await.unwrap();
    port.put(&ns, "b", json!("second")).await.unwrap();
    port.put(&ns, "c", json!("third")).await.unwrap();

    let results = port.search(&ns, "anything", 10).await.unwrap();
    assert!(!results.is_empty(), "expected at least one scored result");

    for pair in results.windows(2) {
        assert!(
            pair[0].score() >= pair[1].score(),
            "scores must be in descending order"
        );
    }

    for scored in &results {
        let fetched = port
            .get(&ns, scored.record().key())
            .await
            .unwrap()
            .expect("every scored record's key must still exist in the store");
        assert_eq!(
            scored.record(),
            &fetched,
            "a scored record's value must be the store's authoritative copy, not a value \
             reconstructed from the vector backend's own payload"
        );
    }
}
