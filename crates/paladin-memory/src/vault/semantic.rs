//! `SemanticVault` -- a `VaultPort` composed from a `VaultPort` store, a
//! `SanctumPort` and an `EmbeddingPort`, giving `search` a real
//! implementation (D-24, RT-FR-13...16).
//!
//! # Not feature-gated
//!
//! `SemanticVault` holds only three trait objects (`Arc<dyn VaultPort>`,
//! `Arc<dyn SanctumPort>`, `Arc<dyn EmbeddingPort>`) and needs no
//! `qdrant-client` symbol of its own -- it is therefore **not** behind any
//! cargo feature (ADR-0046's composition rule: a type built entirely from
//! trait objects never needs the feature that gates one possible concrete
//! implementation of one of those traits). PRD 05's "feature `qdrant`" is
//! satisfied by the Qdrant `SanctumPort` adapter (`QdrantSanctumAdapter`,
//! behind the crate's existing `qdrant` feature) -- one possible value of
//! this struct's `sanctum` field, not a property of `SemanticVault` itself.
//!
//! # Namespace carrier
//!
//! Every `SanctumPort` implementation in this crate (`InMemorySanctum`,
//! `QdrantSanctumAdapter`) already treats `Memory::paladin_id` as its one
//! dedicated, always-filterable, exact-match field (`SanctumFilter::paladin_id`).
//! `SemanticVault` deliberately reuses that field to carry the Vault
//! namespace's `/`-joined path, rather than a `metadata_filters` entry,
//! because `paladin_id` is the one field every current and future
//! `SanctumPort` adapter is guaranteed to index and filter on -- a
//! `metadata_filters` key would only be as portable as each adapter's own
//! metadata-filtering support happens to be.
//!
//! # Deterministic entry id
//!
//! `put` writes to `sanctum` under an id derived purely from `(ns, key)`
//! (`deterministic_entry_id`, private to this module), so a re-`put` on the same address updates
//! the existing Sanctum entry in place (both `InMemorySanctum::store` and a
//! Qdrant upsert are keyed by id) rather than leaving a stale duplicate
//! behind.
//!
//! # The confinement invariant is re-established here, not trusted from the backend
//!
//! `search` passes `ns` to the backend as a `SanctumFilter::paladin_id`
//! best-effort narrowing hint, then **unconditionally re-filters every
//! returned hit** with [`Namespace::is_prefix_of`] before returning anything
//! to the caller -- a vector backend's own filter is a performance
//! optimisation, and the confinement invariant must hold even if a backend
//! ignores the filter it was given entirely (D-24, D-41). Each surviving
//! hit's value is then always loaded fresh from `store` -- the vector
//! payload is never trusted as the authoritative record.
//!
//! # Test tier (D-24, D-39)
//!
//! Every test in this module uses `InMemorySanctum` plus a deterministic
//! mock `EmbeddingPort` (Tier 1, always run). The Qdrant-backed
//! configuration (`QdrantSanctumAdapter` behind the `sanctum` field) is not
//! provable in this environment -- it is routed to UAT per D-24/D-39 and the
//! Phase 24 D-28 precedent, and must never be recorded as passing locally.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::sanctum::{MemoryBuilder, MemoryType, SanctumEntry};
use paladin_core::platform::container::vault::{Namespace, ScoredVaultRecord, VaultRecord};
use paladin_ports::output::embedding_port::{EmbeddingError, EmbeddingPort};
use paladin_ports::output::sanctum_port::{SanctumError, SanctumFilter, SanctumPort, SanctumQuery};
use paladin_ports::output::vault_port::{Page, VaultError, VaultPort};
use serde_json::json;
use uuid::Uuid;

use crate::vault::redact::redact_and_bound;

/// The `Memory.metadata` key `put` stores the record's Vault `key` under, so
/// `search` can recover the `(ns, key)` address of a hit and load the
/// authoritative record from `store`.
const KEY_METADATA_FIELD: &str = "vault_key";

/// A `VaultPort` composed from a `VaultPort` store, a `SanctumPort` and an
/// `EmbeddingPort` -- see the module doc for the composition rationale,
/// the namespace carrier, the deterministic entry id, and the re-filter
/// invariant.
pub struct SemanticVault {
    store: Arc<dyn VaultPort>,
    sanctum: Arc<dyn SanctumPort>,
    embedder: Arc<dyn EmbeddingPort>,
}

impl SemanticVault {
    /// Composes a `SemanticVault` from an existing `VaultPort` store, a
    /// `SanctumPort`, and an `EmbeddingPort`.
    pub fn new(
        store: Arc<dyn VaultPort>,
        sanctum: Arc<dyn SanctumPort>,
        embedder: Arc<dyn EmbeddingPort>,
    ) -> Self {
        Self {
            store,
            sanctum,
            embedder,
        }
    }
}

/// Derives a deterministic Sanctum entry id from `(ns, key)`, so a re-`put`
/// on the same address updates the existing Sanctum entry rather than
/// leaving a stale duplicate behind.
///
/// `ns.to_string()` never contains a NUL byte (a [`Namespace`] segment may
/// never contain a control character, and NUL is one), so joining with a
/// NUL separator before `key` (which has no such restriction) is injective
/// on `(ns, key)` pairs: the first NUL in the combined string is always the
/// separator this function inserted, never one contributed by `ns`.
///
/// This uses `std::collections::hash_map::DefaultHasher` (deterministic
/// within one build, not a cryptographic hash and not guaranteed stable
/// across Rust versions) rather than `Uuid::new_v5`, to avoid adding the
/// `uuid` crate's `v5` cargo feature for a single call site. A hash
/// collision would only ever mean two Vault addresses alias to the same
/// Sanctum entry, which never breaks `get`/`list`/`delete` (always
/// authoritative via `store`) and is astronomically unlikely for `search`.
fn deterministic_entry_id(ns: &Namespace, key: &str) -> Uuid {
    use std::hash::{Hash, Hasher};

    let name = format!("{ns}\u{0}{key}");

    let mut first = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut first);
    let high = first.finish();

    let mut second = std::collections::hash_map::DefaultHasher::new();
    0xA5u8.hash(&mut second);
    name.hash(&mut second);
    let low = second.finish();

    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&high.to_be_bytes());
    bytes[8..].copy_from_slice(&low.to_be_bytes());
    Uuid::from_bytes(bytes)
}

fn map_embedding_error(err: EmbeddingError) -> VaultError {
    VaultError::Storage {
        message: redact_and_bound(&err.to_string()),
    }
}

fn map_sanctum_error(err: SanctumError) -> VaultError {
    VaultError::Storage {
        message: redact_and_bound(&err.to_string()),
    }
}

#[async_trait]
impl VaultPort for SemanticVault {
    async fn put(
        &self,
        ns: &Namespace,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), VaultError> {
        // Embed BEFORE writing anything, so a failing embedder leaves both
        // halves untouched rather than a store row with no matching Sanctum
        // entry (Test 6: embedding_failure_surfaces_as_a_typed_error_not_a_panic).
        let text = value.to_string();
        let embedding = self
            .embedder
            .embed_text(&text)
            .await
            .map_err(map_embedding_error)?;

        self.store.put(ns, key, value).await?;

        let id = deterministic_entry_id(ns, key);
        let mut metadata = HashMap::new();
        metadata.insert(KEY_METADATA_FIELD.to_string(), json!(key));

        // `paladin_id` carries the Vault namespace (see the module doc); the
        // ID is overwritten immediately after `build()` because `MemoryBuilder`
        // always assigns a fresh random id.
        let mut memory = MemoryBuilder::new(ns.to_string(), text)
            .memory_type(MemoryType::Semantic)
            .metadata(metadata)
            .build()
            .map_err(|e| VaultError::Storage {
                message: redact_and_bound(&e),
            })?;
        memory.id = id;

        let entry =
            SanctumEntry::new(memory, embedding.vector).map_err(|e| VaultError::Storage {
                message: redact_and_bound(&e),
            })?;

        // `store` upserts by id (both InMemorySanctum's HashMap insert and a
        // Qdrant point upsert are keyed by id), so calling `store` again on
        // the same deterministic id updates in place rather than duplicating.
        self.sanctum.store(entry).await.map_err(map_sanctum_error)?;

        Ok(())
    }

    async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError> {
        self.store.get(ns, key).await
    }

    async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
        let deleted = self.store.delete(ns, key).await?;

        let id = deterministic_entry_id(ns, key);
        match self.sanctum.delete(&id.to_string()).await {
            Ok(_) | Err(SanctumError::NotFound(_)) => {}
            Err(e) => return Err(map_sanctum_error(e)),
        }

        Ok(deleted)
    }

    async fn list(
        &self,
        ns: &Namespace,
        prefix: Option<&str>,
        page: Page,
    ) -> Result<Vec<VaultRecord>, VaultError> {
        self.store.list(ns, prefix, page).await
    }

    async fn search(
        &self,
        ns: &Namespace,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ScoredVaultRecord>, VaultError> {
        let embedding = self
            .embedder
            .embed_text(query)
            .await
            .map_err(map_embedding_error)?;

        // The backend filter is a best-effort performance hint for the
        // common exact-namespace case; it is never trusted for correctness
        // -- every hit is re-checked below regardless of whether the
        // backend honoured this filter at all.
        let sanctum_query = SanctumQuery::new(embedding.vector, limit as usize)
            .filter(SanctumFilter::new().paladin_id(ns.to_string()));
        let hits = self
            .sanctum
            .search(sanctum_query)
            .await
            .map_err(map_sanctum_error)?;

        let mut scored = Vec::with_capacity(hits.len());
        for hit in hits {
            let Ok(hit_ns) = Namespace::parse(&hit.entry.memory.paladin_id) else {
                continue;
            };

            // The confinement invariant is re-established here, in our own
            // code, and never depends on whatever filter (if any) the
            // backend actually honoured (D-24, D-41).
            if !ns.is_prefix_of(&hit_ns) {
                continue;
            }

            let Some(key) = hit
                .entry
                .memory
                .metadata
                .get(KEY_METADATA_FIELD)
                .and_then(|v| v.as_str())
            else {
                continue;
            };

            // The authoritative value always comes from `store`, never
            // reconstructed from the vector payload -- a stale or truncated
            // embedding-side copy must never leak into a caller-facing result.
            if let Some(record) = self.store.get(&hit_ns, key).await? {
                scored.push(ScoredVaultRecord::new(record, hit.score));
            }
        }

        Ok(scored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sanctum::InMemorySanctum;
    use crate::vault::contract_tests;
    use crate::vault::in_memory::InMemoryVault;
    use paladin_ports::output::embedding_port::Embedding;
    use paladin_ports::output::sanctum_port::SanctumSearchResult;
    use std::collections::HashSet;
    use std::sync::Mutex;

    /// Deterministic mock embedder (D-24, D-39 Tier 1): identical text
    /// always embeds identically and different text embeds differently --
    /// sufficient for this suite's assertions without depending on real
    /// semantic understanding.
    struct DeterministicMockEmbedder;

    #[async_trait]
    impl EmbeddingPort for DeterministicMockEmbedder {
        async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
            let dimension = 8;
            let mut vector = vec![0.0f32; dimension];
            for (i, byte) in text.bytes().enumerate() {
                vector[i % dimension] += byte as f32;
            }
            Ok(Embedding {
                vector,
                model: "deterministic-mock".to_string(),
                dimension,
                token_count: Some(text.split_whitespace().count() as u32),
            })
        }

        async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
            let mut out = Vec::with_capacity(texts.len());
            for text in texts {
                out.push(self.embed_text(text).await?);
            }
            Ok(out)
        }

        fn dimension(&self) -> usize {
            8
        }

        fn model_name(&self) -> &str {
            "deterministic-mock"
        }
    }

    /// Always fails -- Test 6's forced-failure double.
    struct AlwaysFailingEmbedder;

    #[async_trait]
    impl EmbeddingPort for AlwaysFailingEmbedder {
        async fn embed_text(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
            Err(EmbeddingError::ProviderError(
                "forced failure for test".to_string(),
            ))
        }

        async fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
            Err(EmbeddingError::ProviderError(
                "forced failure for test".to_string(),
            ))
        }

        fn dimension(&self) -> usize {
            8
        }

        fn model_name(&self) -> &str {
            "always-failing-mock"
        }
    }

    /// Records every id `store`/`update` touches and lets `delete` remove it
    /// -- Test 4/5's double, standing in for a real `SanctumPort` so a test
    /// can assert on Sanctum-side state directly (the trait itself has no
    /// "get by id").
    #[derive(Default)]
    struct RecordingSanctum {
        entries: Mutex<HashSet<String>>,
    }

    impl RecordingSanctum {
        fn new() -> Self {
            Self::default()
        }

        fn contains(&self, id: &str) -> bool {
            self.entries.lock().unwrap().contains(id)
        }

        fn len(&self) -> usize {
            self.entries.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl SanctumPort for RecordingSanctum {
        async fn store(&self, entry: SanctumEntry) -> Result<(), SanctumError> {
            self.entries
                .lock()
                .unwrap()
                .insert(entry.memory.id.to_string());
            Ok(())
        }

        async fn store_batch(&self, entries: Vec<SanctumEntry>) -> Result<(), SanctumError> {
            for entry in entries {
                self.store(entry).await?;
            }
            Ok(())
        }

        async fn search(
            &self,
            _query: SanctumQuery,
        ) -> Result<Vec<SanctumSearchResult>, SanctumError> {
            Ok(vec![])
        }

        async fn delete(&self, id: &str) -> Result<bool, SanctumError> {
            Ok(self.entries.lock().unwrap().remove(id))
        }

        async fn update(&self, entry: SanctumEntry) -> Result<(), SanctumError> {
            self.entries
                .lock()
                .unwrap()
                .insert(entry.memory.id.to_string());
            Ok(())
        }

        async fn count(&self, _filter: Option<SanctumFilter>) -> Result<usize, SanctumError> {
            Ok(self.entries.lock().unwrap().len())
        }
    }

    /// Test 3's deliberately misbehaving backend: always returns exactly one
    /// hit from a namespace the caller never asked about, ignoring the query
    /// and the filter it was given entirely -- simulating a backend that
    /// does not honour (or does not even support) namespace scoping.
    struct ForeignNamespaceSanctum;

    #[async_trait]
    impl SanctumPort for ForeignNamespaceSanctum {
        async fn store(&self, _entry: SanctumEntry) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn store_batch(&self, _entries: Vec<SanctumEntry>) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn search(
            &self,
            _query: SanctumQuery,
        ) -> Result<Vec<SanctumSearchResult>, SanctumError> {
            let memory =
                MemoryBuilder::new("some/other-namespace".to_string(), "foreign".to_string())
                    .memory_type(MemoryType::Semantic)
                    .build()
                    .unwrap();
            let entry = SanctumEntry::new(memory, vec![1.0, 0.0]).unwrap();
            Ok(vec![SanctumSearchResult::new(entry, 0.99)])
        }

        async fn delete(&self, _id: &str) -> Result<bool, SanctumError> {
            Ok(false)
        }

        async fn update(&self, _entry: SanctumEntry) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn count(&self, _filter: Option<SanctumFilter>) -> Result<usize, SanctumError> {
            Ok(0)
        }
    }

    fn semantic_vault_for_tests() -> SemanticVault {
        SemanticVault::new(
            Arc::new(InMemoryVault::new()),
            Arc::new(InMemorySanctum::new(1_000)),
            Arc::new(DeterministicMockEmbedder),
        )
    }

    // One #[tokio::test] per shared contract clause, mirroring
    // `in_memory.rs`'s / `sqlite.rs`'s own test module shape -- so a failure
    // names the exact violated clause directly.

    #[tokio::test]
    async fn put_then_get_returns_the_record() {
        contract_tests::put_then_get_returns_the_record(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn put_overwrites_and_preserves_created_at() {
        contract_tests::put_overwrites_and_preserves_created_at(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn delete_returns_true_then_false() {
        contract_tests::delete_returns_true_then_false(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn list_returns_only_this_namespace_ordered_by_key() {
        contract_tests::list_returns_only_this_namespace_ordered_by_key(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn list_filters_by_key_prefix() {
        contract_tests::list_filters_by_key_prefix(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn list_paginates_by_opaque_after_cursor() {
        contract_tests::list_paginates_by_opaque_after_cursor(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn list_on_an_empty_namespace_is_an_empty_page_not_an_error() {
        contract_tests::list_on_an_empty_namespace_is_an_empty_page_not_an_error(
            &semantic_vault_for_tests(),
        )
        .await;
    }

    #[tokio::test]
    async fn namespaces_are_isolated() {
        contract_tests::namespaces_are_isolated(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn value_larger_than_the_bound_is_rejected() {
        contract_tests::value_larger_than_the_bound_is_rejected(&semantic_vault_for_tests()).await;
    }

    #[tokio::test]
    async fn semantic_vault_passes_the_shared_contract_suite() {
        let vault = semantic_vault_for_tests();
        contract_tests::run_all_shared_clauses(&vault).await;
        contract_tests::search_returns_scored_records_from_the_store(&vault).await;
    }

    #[tokio::test]
    async fn search_returns_scored_records_from_the_store() {
        let vault = semantic_vault_for_tests();
        contract_tests::search_returns_scored_records_from_the_store(&vault).await;
    }

    #[tokio::test]
    async fn search_re_filters_by_namespace_even_when_the_backend_does_not() {
        let vault = SemanticVault::new(
            Arc::new(InMemoryVault::new()),
            Arc::new(ForeignNamespaceSanctum),
            Arc::new(DeterministicMockEmbedder),
        );
        let ns = Namespace::parse("legit/namespace").unwrap();

        let results = vault.search(&ns, "query", 5).await.unwrap();
        assert!(
            results.is_empty(),
            "a hit from a namespace the caller never asked about must be dropped, even though \
             this mock backend ignored the filter and the query entirely"
        );
    }

    #[tokio::test]
    async fn put_is_deterministic_and_updates_rather_than_duplicates() {
        let store = Arc::new(InMemoryVault::new());
        let sanctum = Arc::new(RecordingSanctum::new());
        let vault = SemanticVault::new(store, sanctum.clone(), Arc::new(DeterministicMockEmbedder));
        let ns = Namespace::parse("semantic-put/ns").unwrap();

        vault.put(&ns, "k", json!("first")).await.unwrap();
        vault.put(&ns, "k", json!("second")).await.unwrap();

        assert_eq!(
            sanctum.len(),
            1,
            "a re-put must update, never duplicate, the Sanctum entry"
        );

        let stored = vault.get(&ns, "k").await.unwrap().unwrap();
        assert_eq!(stored.value(), &json!("second"));

        let listed = vault.list(&ns, None, Page::default()).await.unwrap();
        assert_eq!(
            listed.len(),
            1,
            "a re-put must update, never duplicate, the store row"
        );
    }

    #[tokio::test]
    async fn delete_removes_from_both_halves() {
        let store = Arc::new(InMemoryVault::new());
        let sanctum = Arc::new(RecordingSanctum::new());
        let vault = SemanticVault::new(store, sanctum.clone(), Arc::new(DeterministicMockEmbedder));
        let ns = Namespace::parse("semantic-delete/ns").unwrap();

        vault.put(&ns, "k", json!("value")).await.unwrap();
        let id = deterministic_entry_id(&ns, "k").to_string();
        assert!(sanctum.contains(&id));

        assert!(vault.delete(&ns, "k").await.unwrap());
        assert!(vault.get(&ns, "k").await.unwrap().is_none());
        assert!(!sanctum.contains(&id));
    }

    #[tokio::test]
    async fn embedding_failure_surfaces_as_a_typed_error_not_a_panic() {
        let vault = SemanticVault::new(
            Arc::new(InMemoryVault::new()),
            Arc::new(RecordingSanctum::new()),
            Arc::new(AlwaysFailingEmbedder),
        );
        let ns = Namespace::parse("semantic-fail/ns").unwrap();

        let put_err = vault.put(&ns, "k", json!("v")).await.unwrap_err();
        assert!(matches!(put_err, VaultError::Storage { .. }));

        let search_err = vault.search(&ns, "q", 5).await.unwrap_err();
        assert!(matches!(search_err, VaultError::Storage { .. }));
    }

    #[tokio::test]
    async fn semantic_vault_is_constructible_without_the_qdrant_feature() {
        // Compile-level proof: this whole test module (indeed this whole
        // file) builds and runs under default features, i.e. with the
        // `qdrant` cargo feature OFF -- `SemanticVault` needs no
        // `qdrant-client` symbol.
        let vault = semantic_vault_for_tests();
        let ns = Namespace::parse("semantic-ungated/ns").unwrap();
        vault.put(&ns, "k", json!(1)).await.unwrap();
        assert!(vault.get(&ns, "k").await.unwrap().is_some());
    }
}
