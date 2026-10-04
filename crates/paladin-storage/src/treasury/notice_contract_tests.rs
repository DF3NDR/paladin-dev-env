//! Shared `TreasuryNoticePort` contract suite (ALLOW-04, D-16, RESEARCH C5).
//!
//! One generic async function per contract clause, each taking `&dyn TreasuryNoticePort` (or,
//! for the race clause, `Arc<dyn TreasuryNoticePort>`) and asserting inside. Every backend
//! (`InMemoryTreasuryLedger`, `SqliteTreasuryLedger`, `PostgresTreasuryLedger`) invokes these
//! functions unchanged from its own `#[tokio::test]`s, mirroring [`super::contract_tests`]'s
//! house pattern, so "identical suite across backends" holds by construction.
//!
//! Every clause isolates itself with a unique tenant id (see [`notice_tenant`]) so a shared
//! database never leaks rows between clauses. Every instant is a whole second, so a notice reads
//! back byte-identical on a backend that stores microseconds.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

use paladin_core::platform::container::allowance::{
    AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning, NoticeOutcome, NoticeRecord,
};
use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::run::RunId;
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;

/// The API key name every API-key-scope notice in this suite is held against.
const NOTICE_KEY: &str = "key-notice";

/// A fresh, clause-unique tenant id so concurrent clauses (and a shared PostgreSQL database)
/// never see each other's rows.
pub fn notice_tenant(clause: &str) -> String {
    format!("notice-{clause}-{}", Uuid::new_v4())
}

/// A whole-second UTC instant on 2026-10-03 (`hour`:00:00) -- a window boundary the notice
/// clauses share.
pub fn notice_instant(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 3, hour, 0, 0)
        .single()
        .expect("a valid UTC instant")
}

/// The hour-long window `[hour:00, hour+1:00)` on 2026-10-03.
pub fn notice_window(hour: u32) -> (DateTime<Utc>, DateTime<Utc>) {
    (notice_instant(hour), notice_instant(hour + 1))
}

fn usd() -> CurrencyCode {
    CurrencyCode::new("USD").expect("USD is a valid currency code")
}

/// An API-key-scope window notice for `tenant` over `window`, with the given ceiling, a fresh
/// `notice_id`, no run and `recorded_at` at the window's start.
pub fn key_window_notice(
    tenant: &str,
    window: (DateTime<Utc>, DateTime<Utc>),
    ceiling_nanos: i64,
) -> NoticeRecord {
    NoticeRecord {
        notice_id: Uuid::now_v7().to_string(),
        tenant_id: tenant.to_string(),
        api_key_id: Some(NOTICE_KEY.to_string()),
        warning: AllowanceWarning {
            scope_kind: AllowanceScopeKind::ApiKey,
            limit_kind: AllowanceLimitKind::Window,
            balance: Cost::new(ceiling_nanos * 8 / 10, usd()),
            ceiling: Cost::new(ceiling_nanos, usd()),
            window_start: Some(window.0),
            window_end: Some(window.1),
            warn_at: 80,
        },
        run_id: None,
        recorded_at: window.0,
    }
}

/// A tenant-scope window notice (`api_key_id` `None`).
pub fn tenant_window_notice(
    tenant: &str,
    window: (DateTime<Utc>, DateTime<Utc>),
    ceiling_nanos: i64,
) -> NoticeRecord {
    let mut notice = key_window_notice(tenant, window, ceiling_nanos);
    notice.api_key_id = None;
    notice.warning.scope_kind = AllowanceScopeKind::Tenant;
    notice
}

/// A tenant-scope lifetime notice (`api_key_id` `None`, both window bounds `None`).
pub fn tenant_lifetime_notice(tenant: &str, ceiling_nanos: i64) -> NoticeRecord {
    let mut notice = tenant_window_notice(tenant, notice_window(0), ceiling_nanos);
    notice.warning.limit_kind = AllowanceLimitKind::Lifetime;
    notice.warning.window_start = None;
    notice.warning.window_end = None;
    notice
}

/// `notice` re-keyed with a fresh `notice_id` -- the same identity, a different claim attempt.
fn reclaimed(notice: &NoticeRecord) -> NoticeRecord {
    NoticeRecord {
        notice_id: Uuid::now_v7().to_string(),
        ..notice.clone()
    }
}

async fn record_ok(port: &dyn TreasuryNoticePort, notice: &NoticeRecord) -> NoticeOutcome {
    port.record(notice)
        .await
        .unwrap_or_else(|e| panic!("record must succeed, got {e:?}"))
}

// ── Dedup ─────────────────────────────────────────────────────────────────

/// The first claim of an identity is `Recorded`; a second claim with a different `notice_id`
/// and the same identity is `AlreadyRecorded`, and only the first row exists.
pub async fn first_claim_wins_duplicate_is_already_recorded(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("first-claim");
    let run = RunId::new_v7();
    let mut first = key_window_notice(&tenant, notice_window(1), 100);
    first.run_id = Some(run.clone());
    let mut second = reclaimed(&first);
    // A different balance, warn_at and recorded_at on the second claim: none is identity.
    second.warning.balance = Cost::new(95, usd());
    second.recorded_at = notice_instant(2);

    assert_eq!(record_ok(port, &first).await, NoticeOutcome::Recorded);
    assert_eq!(
        record_ok(port, &second).await,
        NoticeOutcome::AlreadyRecorded
    );

    let rows = port.notices_for_run(&run).await.expect("read back");
    assert_eq!(rows, vec![first], "only the winning claim's row exists");
}

/// Two tenant-scope claims (`api_key_id` `None`, stored as `''`) for one window yield exactly
/// one `Recorded` -- the guard for RESEARCH C5 (a NULL in a unique key never conflicts).
pub async fn tenant_scope_duplicate_dedups(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("tenant-scope");
    let first = tenant_window_notice(&tenant, notice_window(1), 100);
    let second = reclaimed(&first);

    assert_eq!(record_ok(port, &first).await, NoticeOutcome::Recorded);
    assert_eq!(
        record_ok(port, &second).await,
        NoticeOutcome::AlreadyRecorded
    );

    // A tenant-scope and an API-key-scope notice of the same tenant and window are distinct
    // identities: the scope kind is part of the key.
    let keyed = key_window_notice(&tenant, notice_window(1), 100);
    assert_eq!(record_ok(port, &keyed).await, NoticeOutcome::Recorded);
}

/// A lifetime notice (stored with the epoch as its `window_start`) dedups per ceiling: the
/// same ceiling is `AlreadyRecorded`, a raised ceiling is `Recorded` again.
pub async fn lifetime_notice_dedups_per_ceiling(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("lifetime");
    let first = tenant_lifetime_notice(&tenant, 1_000);
    let again = reclaimed(&first);
    let raised = tenant_lifetime_notice(&tenant, 2_000);

    assert_eq!(record_ok(port, &first).await, NoticeOutcome::Recorded);
    assert_eq!(
        record_ok(port, &again).await,
        NoticeOutcome::AlreadyRecorded
    );
    assert_eq!(record_ok(port, &raised).await, NoticeOutcome::Recorded);
}

/// Raising a ceiling re-arms the notice for the same window (the ceiling is part of the
/// identity); changing `warn_at` alone does not.
pub async fn raised_ceiling_rearms_the_same_window(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("raised-ceiling");
    let window = notice_window(3);
    let first = key_window_notice(&tenant, window, 100);
    let raised = key_window_notice(&tenant, window, 200);
    let mut other_threshold = reclaimed(&first);
    other_threshold.warning.warn_at = 50;

    assert_eq!(record_ok(port, &first).await, NoticeOutcome::Recorded);
    assert_eq!(record_ok(port, &raised).await, NoticeOutcome::Recorded);
    assert_eq!(
        record_ok(port, &other_threshold).await,
        NoticeOutcome::AlreadyRecorded,
        "warn_at alone must not re-arm the notice"
    );
}

/// The same scope and ceiling in a different window is a new notice.
pub async fn distinct_window_start_is_a_distinct_notice(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("distinct-window");
    let first = key_window_notice(&tenant, notice_window(4), 100);
    let next = key_window_notice(&tenant, notice_window(5), 100);

    assert_eq!(record_ok(port, &first).await, NoticeOutcome::Recorded);
    assert_eq!(record_ok(port, &next).await, NoticeOutcome::Recorded);
    assert_eq!(
        record_ok(port, &reclaimed(&first)).await,
        NoticeOutcome::AlreadyRecorded
    );
}

// ── Race (D-16) ───────────────────────────────────────────────────────────

/// Sixteen concurrent claims of one identity yield exactly one `Recorded` and fifteen
/// `AlreadyRecorded`, and exactly one row exists afterwards.
pub async fn sixteen_concurrent_claims_yield_exactly_one_recorded(
    port: Arc<dyn TreasuryNoticePort>,
) {
    let tenant = notice_tenant("race");
    let run = RunId::new_v7();
    let base = {
        let mut notice = tenant_window_notice(&tenant, notice_window(6), 100);
        notice.run_id = Some(run.clone());
        notice
    };

    let mut handles = Vec::new();
    for _ in 0..16 {
        let port = Arc::clone(&port);
        let claim = reclaimed(&base);
        handles.push(tokio::spawn(async move { port.record(&claim).await }));
    }

    let mut recorded = 0;
    let mut already = 0;
    tokio::time::timeout(Duration::from_secs(10), async {
        for handle in handles {
            match handle.await.expect("claim task must not panic") {
                Ok(NoticeOutcome::Recorded) => recorded += 1,
                Ok(NoticeOutcome::AlreadyRecorded) => already += 1,
                Err(e) => panic!("a concurrent claim must never error, got {e:?}"),
            }
        }
    })
    .await
    .expect("16 concurrent claims must not hang");

    assert_eq!(recorded, 1, "exactly one claim wins");
    assert_eq!(already, 15, "every other claim is AlreadyRecorded");
    let rows = port.notices_for_run(&run).await.expect("read back");
    assert_eq!(rows.len(), 1, "exactly one row exists for the identity");
}

// ── Read back and discard ─────────────────────────────────────────────────

/// `notices_for_run` returns only the rows recorded with that run id, oldest first.
pub async fn notices_for_run_returns_only_that_runs_rows(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("for-run");
    let run_a = RunId::new_v7();
    let run_b = RunId::new_v7();

    let mut older = key_window_notice(&tenant, notice_window(7), 100);
    older.run_id = Some(run_a.clone());
    older.recorded_at = notice_instant(7);
    let mut newer = tenant_window_notice(&tenant, notice_window(7), 100);
    newer.run_id = Some(run_a.clone());
    newer.recorded_at = notice_instant(8);
    let mut other = key_window_notice(&tenant, notice_window(9), 100);
    other.run_id = Some(run_b.clone());
    let unowned = key_window_notice(&tenant, notice_window(10), 100);

    // Record out of chronological order: the read must still be oldest first.
    for notice in [&newer, &other, &older, &unowned] {
        assert_eq!(record_ok(port, notice).await, NoticeOutcome::Recorded);
    }

    let rows = port.notices_for_run(&run_a).await.expect("read back");
    assert_eq!(rows, vec![older, newer], "only run A's rows, oldest first");
    let rows = port.notices_for_run(&run_b).await.expect("read back");
    assert_eq!(rows, vec![other]);
    let rows = port
        .notices_for_run(&RunId::new_v7())
        .await
        .expect("read back");
    assert!(rows.is_empty(), "an unknown run has no notices");
}

/// `discard` removes exactly the named rows; an unknown id is `Ok`; a discarded identity can be
/// claimed again.
pub async fn discard_removes_only_the_named_rows(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("discard");
    let run = RunId::new_v7();
    let mut keep = key_window_notice(&tenant, notice_window(11), 100);
    keep.run_id = Some(run.clone());
    let mut drop = tenant_window_notice(&tenant, notice_window(11), 100);
    drop.run_id = Some(run.clone());

    assert_eq!(record_ok(port, &keep).await, NoticeOutcome::Recorded);
    assert_eq!(record_ok(port, &drop).await, NoticeOutcome::Recorded);

    port.discard(&[drop.notice_id.clone(), "no-such-notice".to_string()])
        .await
        .expect("discarding named and unknown ids is Ok");
    let rows = port.notices_for_run(&run).await.expect("read back");
    assert_eq!(rows, vec![keep], "only the named row is removed");

    // An empty discard is a no-op.
    port.discard(&[]).await.expect("an empty discard is Ok");

    // The discarded identity is free again: the next claim wins.
    assert_eq!(
        record_ok(port, &reclaimed(&drop)).await,
        NoticeOutcome::Recorded
    );
}

/// Every field of a window notice and of a lifetime notice reads back exactly as recorded --
/// lifetime rows with both window bounds `None`, tenant rows with `api_key_id` `None`.
pub async fn notice_round_trips_every_field(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("round-trip");
    let run = RunId::new_v7();

    let mut window = key_window_notice(&tenant, notice_window(12), 2_500_000_000);
    window.run_id = Some(run.clone());
    window.warning.balance = Cost::new(2_000_000_000, usd());
    window.recorded_at = notice_instant(12);

    let mut lifetime = tenant_lifetime_notice(&tenant, 9_000_000_000);
    lifetime.run_id = Some(run.clone());
    lifetime.warning.warn_at = 75;
    lifetime.recorded_at = notice_instant(13);

    assert_eq!(record_ok(port, &window).await, NoticeOutcome::Recorded);
    assert_eq!(record_ok(port, &lifetime).await, NoticeOutcome::Recorded);

    let rows = port.notices_for_run(&run).await.expect("read back");
    assert_eq!(rows, vec![window, lifetime]);
    assert_eq!(rows[1].warning.window_start, None);
    assert_eq!(rows[1].warning.window_end, None);
    assert_eq!(rows[1].api_key_id, None);
}

// ── Validation ────────────────────────────────────────────────────────────

/// A malformed notice is `InvalidRequest` and writes nothing: an empty tenant, an API-key scope
/// without a key, a tenant scope with a key, `warn_at` 101, a window notice without its bounds
/// and a lifetime notice with a window.
pub async fn invalid_notice_is_rejected_before_io(port: &dyn TreasuryNoticePort) {
    let tenant = notice_tenant("invalid");
    let run = RunId::new_v7();
    let good = {
        let mut notice = key_window_notice(&tenant, notice_window(14), 100);
        notice.run_id = Some(run.clone());
        notice
    };

    let mut empty_tenant = reclaimed(&good);
    empty_tenant.tenant_id = String::new();
    let mut empty_id = reclaimed(&good);
    empty_id.notice_id = String::new();
    let mut key_scope_without_key = reclaimed(&good);
    key_scope_without_key.api_key_id = None;
    let mut tenant_scope_with_key = reclaimed(&good);
    tenant_scope_with_key.warning.scope_kind = AllowanceScopeKind::Tenant;
    let mut warn_101 = reclaimed(&good);
    warn_101.warning.warn_at = 101;
    let mut window_without_bounds = reclaimed(&good);
    window_without_bounds.warning.window_start = None;
    window_without_bounds.warning.window_end = None;
    let mut lifetime_with_window = reclaimed(&good);
    lifetime_with_window.warning.limit_kind = AllowanceLimitKind::Lifetime;

    for (label, bad) in [
        ("empty tenant", empty_tenant),
        ("empty notice id", empty_id),
        ("api_key scope without a key", key_scope_without_key),
        ("tenant scope with a key", tenant_scope_with_key),
        ("warn_at 101", warn_101),
        ("window notice without bounds", window_without_bounds),
        ("lifetime notice with a window", lifetime_with_window),
    ] {
        match port.record(&bad).await {
            Err(TreasuryLedgerError::InvalidRequest { .. }) => {}
            other => panic!("{label}: expected InvalidRequest, got {other:?}"),
        }
    }

    // Nothing was written: the good identity is still unclaimed.
    assert!(
        port.notices_for_run(&run)
            .await
            .expect("read back")
            .is_empty(),
        "an invalid notice must write nothing"
    );
    assert_eq!(record_ok(port, &good).await, NoticeOutcome::Recorded);
}
