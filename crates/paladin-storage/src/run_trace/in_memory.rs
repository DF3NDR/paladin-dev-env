/*
In-Memory Run Trace Store

An `Arc<tokio::sync::RwLock<HashMap<ThreadId, BTreeMap<u64, StoredRecord>>>>`
-backed implementation of `RunTracePort`, for tests and local development
(mirrors `InMemoryWaypointStore`'s D-01 precedent: always available, no
feature gate). `append` is idempotent per `(thread_id, seq)` -- a `seq`
already present is left untouched, never overwritten, mirroring the SQL
backends' `ON CONFLICT ... DO NOTHING`.
*/

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use paladin_core::platform::container::trace::{TRACE_SCHEMA_VERSION, TraceRecord};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_trace_port::{RunTraceError, RunTracePort};

use crate::run_trace::contract_tests::RawSchemaVersionWriter;
use crate::run_trace::superstep_of;

#[derive(Clone)]
struct StoredRecord {
    record: TraceRecord,
    superstep: u64,
    schema_version: String,
}

/// In-memory `RunTracePort` implementation.
///
/// Cloning an `InMemoryRunTraceStore` is cheap and shares the same
/// underlying store (the inner `Arc` is cloned), mirroring
/// `InMemoryWaypointStore`'s own convention.
#[derive(Clone, Default)]
pub struct InMemoryRunTraceStore {
    threads: Arc<RwLock<HashMap<ThreadId, BTreeMap<u64, StoredRecord>>>>,
}

impl InMemoryRunTraceStore {
    /// Construct a new, empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RunTracePort for InMemoryRunTraceStore {
    async fn append(&self, records: &[TraceRecord]) -> Result<(), RunTraceError> {
        let mut threads = self.threads.write().await;
        for record in records {
            let entry = threads.entry(record.thread_id.clone()).or_default();
            entry.entry(record.seq).or_insert_with(|| StoredRecord {
                superstep: superstep_of(&record.event),
                schema_version: TRACE_SCHEMA_VERSION.to_string(),
                record: record.clone(),
            });
        }
        Ok(())
    }

    async fn read(
        &self,
        thread: &ThreadId,
        after_seq: u64,
        limit: u32,
    ) -> Result<Vec<TraceRecord>, RunTraceError> {
        let threads = self.threads.read().await;
        let Some(rows) = threads.get(thread) else {
            return Ok(vec![]);
        };

        let mut out = Vec::new();
        for (_, stored) in rows.range((
            std::ops::Bound::Excluded(after_seq),
            std::ops::Bound::Unbounded,
        )) {
            if stored.schema_version != TRACE_SCHEMA_VERSION {
                return Err(RunTraceError::UnsupportedSchemaVersion {
                    found: stored.schema_version.clone(),
                });
            }
            out.push(stored.record.clone());
            if out.len() as u32 >= limit {
                break;
            }
        }
        Ok(out)
    }

    async fn prune_thread(
        &self,
        thread: &ThreadId,
        before_superstep: u64,
    ) -> Result<u64, RunTraceError> {
        let mut threads = self.threads.write().await;
        let Some(rows) = threads.get_mut(thread) else {
            return Ok(0);
        };
        let before_len = rows.len();
        rows.retain(|_, stored| stored.superstep >= before_superstep);
        Ok((before_len - rows.len()) as u64)
    }
}

#[async_trait]
impl RawSchemaVersionWriter for InMemoryRunTraceStore {
    async fn write_with_schema_version(&self, record: &TraceRecord, schema_version: &str) {
        let mut threads = self.threads.write().await;
        let entry = threads.entry(record.thread_id.clone()).or_default();
        entry.insert(
            record.seq,
            StoredRecord {
                superstep: superstep_of(&record.event),
                schema_version: schema_version.to_string(),
                record: record.clone(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_trace::contract_tests;

    // One #[tokio::test] per shared contract function, each against a fresh
    // store, so a failure names the violated contract clause. See
    // `contract_tests` for the assertions themselves -- this file only
    // wires InMemoryRunTraceStore into them.

    #[tokio::test]
    async fn append_then_read_round_trips() {
        contract_tests::append_then_read_round_trips(&InMemoryRunTraceStore::new()).await;
    }

    #[tokio::test]
    async fn read_paginates_by_after_seq() {
        contract_tests::read_paginates_by_after_seq(&InMemoryRunTraceStore::new()).await;
    }

    #[tokio::test]
    async fn read_of_unknown_thread_is_empty_not_error() {
        contract_tests::read_of_unknown_thread_is_empty_not_error(&InMemoryRunTraceStore::new())
            .await;
    }

    #[tokio::test]
    async fn append_is_idempotent_on_same_seq() {
        contract_tests::append_is_idempotent_on_same_seq(&InMemoryRunTraceStore::new()).await;
    }

    #[tokio::test]
    async fn records_are_scoped_by_thread() {
        contract_tests::records_are_scoped_by_thread(&InMemoryRunTraceStore::new()).await;
    }

    #[tokio::test]
    async fn prune_thread_removes_only_older_supersteps() {
        contract_tests::prune_thread_removes_only_older_supersteps(&InMemoryRunTraceStore::new())
            .await;
    }

    #[tokio::test]
    async fn unsupported_schema_version_is_typed() {
        contract_tests::unsupported_schema_version_is_typed(&InMemoryRunTraceStore::new()).await;
    }

    #[tokio::test]
    async fn run_all_contract_functions_smoke_aggregate() {
        contract_tests::run_all(Arc::new(InMemoryRunTraceStore::new())).await;
    }
}
