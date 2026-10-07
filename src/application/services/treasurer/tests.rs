//! Proofs of the Treasurer's admission rule (ALLOW-01, ALLOW-02, Phase 41).
//!
//! Most tests drive [`Treasurer::admit`] over [`FakeLedger`], a scripted
//! [`TreasuryLedgerPort`] whose store clock the test sets directly -- so a window boundary, a
//! `Retry-After` and a clock decades from the real one are all exact, never sleep-bracketed. The
//! idempotency and concurrency proofs run over a real [`InMemoryTreasuryLedger`] so the ledger's
//! own row counts witness that admission never writes.
//!
//! Test code may import `paladin_storage`; the module's production files may not (hexagonal,
//! D-06).
//!
//! # Accepted race (not testable here)
//!
//! Two admissions for one principal at the same instant may both be admitted even when together
//! they over-spend an allowance: admission is check-only (D-05, no hold and no reserve row), so
//! nothing here can serialize them. Phase 42 does not close the race: its superstep-boundary
//! check is check-only too (ADR-0057 supersedes ADR-0056's earlier plan to reserve at the
//! boundary), so runs admitted in the same instant can each overshoot by at most one superstep.
//! ADR-0056 (plan 41-09) and ADR-0057 record the race and `.planning/WINDOWS.md` row 65 holds
//! it open -- those records are the backstop; no mechanical check in this file can prevent it, by
//! design. A per-superstep reservation hold is the deferred mitigation.

use super::*;
use std::sync::Mutex;

use paladin_core::platform::container::allowance::{
    AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind,
};
use paladin_core::platform::container::cost::CurrencyCode;
use paladin_core::platform::container::principal::TenantId;
use paladin_core::platform::container::run::RunEventKind;
use paladin_core::platform::container::treasury_ledger::{
    BalanceQuery, LedgerScope, ReservationId, ReserveRequest, SettleOutcome, SettleRequest,
    SettlementKey, SpendGroupBy, SpendQuery, SpendRow,
};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;
use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

// -- the scripted ledger ---------------------------------------------------

/// How a [`FakeLedger`] is told to fail. Each call builds a fresh `TreasuryLedgerError` (the
/// error type is not `Clone`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FakeFailure {
    /// `balance` fails with a backend error.
    Backend,
    /// `balance` reports a foreign-currency row (D-00h).
    CurrencyMismatch,
    /// `store_now` fails.
    StoreNow,
}

/// One scripted ledger row: the fields `balance` filters and sums over.
#[derive(Debug, Clone)]
struct FakeRow {
    tenant_id: String,
    api_key_id: String,
    attributed_at: DateTime<Utc>,
    nanos: i64,
    currency: CurrencyCode,
}

#[derive(Debug)]
struct FakeState {
    now: DateTime<Utc>,
    rows: Vec<FakeRow>,
    balance_calls: Vec<BalanceQuery>,
    store_now_calls: usize,
    fail: Option<FakeFailure>,
}

/// A [`TreasuryLedgerPort`] scripting `store_now` and `balance` over an in-process row list and
/// recording every call. The four write methods are never reached by admission and fail loudly
/// if they are.
#[derive(Debug)]
struct FakeLedger {
    state: Mutex<FakeState>,
}

impl FakeLedger {
    fn new(now: DateTime<Utc>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(FakeState {
                now,
                rows: Vec::new(),
                balance_calls: Vec::new(),
                store_now_calls: 0,
                fail: None,
            }),
        })
    }

    fn state(&self) -> std::sync::MutexGuard<'_, FakeState> {
        self.state
            .lock()
            .expect("fake ledger state is never poisoned")
    }

    /// Add a USD row of `nanos` for (`tenant`, `key`) attributed at `attributed_at`.
    fn with_row(
        self: Arc<Self>,
        tenant: &str,
        key: &str,
        attributed_at: DateTime<Utc>,
        nanos: i64,
    ) -> Arc<Self> {
        self.state().rows.push(FakeRow {
            tenant_id: tenant.to_string(),
            api_key_id: key.to_string(),
            attributed_at,
            nanos,
            currency: usd(),
        });
        self
    }

    fn set_now(&self, now: DateTime<Utc>) {
        self.state().now = now;
    }

    fn set_failure(&self, fail: Option<FakeFailure>) {
        self.state().fail = fail;
    }

    fn balance_calls(&self) -> Vec<BalanceQuery> {
        self.state().balance_calls.clone()
    }

    fn store_now_calls(&self) -> usize {
        self.state().store_now_calls
    }
}

fn not_scripted() -> TreasuryLedgerError {
    TreasuryLedgerError::InvalidRequest {
        message: "FakeLedger: write methods are never reached by admission".to_string(),
    }
}

#[async_trait]
impl TreasuryLedgerPort for FakeLedger {
    async fn reserve(&self, _r: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError> {
        Err(not_scripted())
    }
    async fn release(&self, _r: ReservationId) -> Result<(), TreasuryLedgerError> {
        Err(not_scripted())
    }
    async fn settle(&self, _r: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError> {
        Err(not_scripted())
    }
    async fn spend(&self, _q: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
        Err(not_scripted())
    }

    async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
        let mut state = self.state();
        state.store_now_calls += 1;
        if state.fail == Some(FakeFailure::StoreNow) {
            return Err(TreasuryLedgerError::Backend {
                source: "scripted store clock failure".into(),
            });
        }
        Ok(state.now)
    }

    async fn balance(&self, query: BalanceQuery) -> Result<Cost, TreasuryLedgerError> {
        let mut state = self.state();
        state.balance_calls.push(query.clone());
        match state.fail {
            Some(FakeFailure::Backend) => {
                return Err(TreasuryLedgerError::Backend {
                    source: "scripted balance failure".into(),
                });
            }
            Some(FakeFailure::CurrencyMismatch) => {
                return Err(TreasuryLedgerError::CurrencyMismatch {
                    expected: query.currency,
                    found: CurrencyCode::new("EUR").expect("EUR is valid"),
                });
            }
            Some(FakeFailure::StoreNow) | None => {}
        }
        let sum = state
            .rows
            .iter()
            .filter(|row| {
                row.tenant_id == query.tenant_id
                    && query
                        .api_key_id
                        .as_ref()
                        .is_none_or(|key| &row.api_key_id == key)
                    && row.currency == query.currency
                    && query.since.is_none_or(|since| row.attributed_at >= since)
                    && query.until.is_none_or(|until| row.attributed_at < until)
            })
            .fold(0_i64, |acc, row| acc.saturating_add(row.nanos));
        Ok(Cost::new(sum, query.currency))
    }
}

// -- helpers ---------------------------------------------------------------

/// A whole-second instant, `secs` after the Unix epoch.
fn at(secs: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(secs, 0).expect("representable instant")
}

/// A fractional-second instant (900 ms past `secs`).
fn at_900ms(secs: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(secs, 900_000_000).expect("representable instant")
}

fn usd() -> CurrencyCode {
    CurrencyCode::new("USD").expect("USD is valid")
}

fn subject(tenant: &str, key: &str) -> RunAttribution {
    RunAttribution::new(TenantId::new(tenant).expect("valid tenant"), key)
}

/// 2001-09-09T01:46:40Z -- a store instant decades away from the real clock.
const NOW: i64 = 1_000_000_000;
/// The hourly window containing [`NOW`]: `floor(NOW / 3600) * 3600`.
const WS: i64 = 999_997_200;
/// That window's exclusive end.
const WE: i64 = WS + 3_600;
/// The hourly period every window ceiling below uses.
const P: u64 = 3_600;
/// The window ceiling every single-ceiling policy below uses.
const C: i64 = 100;

/// One API-key window ceiling of [`C`] per [`P`] seconds for `svc-a`.
fn key_policy() -> AllowancePolicy {
    AllowancePolicy::new(usd(), 80).with_api_key("svc-a", ScopeAllowance::new(P, C))
}

/// All four ceilings for `acme` / `svc-a`: key window 100 per hour, key lifetime 1000, tenant
/// window 500 per day, tenant lifetime 800.
fn four_ceiling_policy() -> AllowancePolicy {
    AllowancePolicy::new(usd(), 80)
        .with_api_key("svc-a", ScopeAllowance::new(P, 100).with_lifetime(1_000))
        .with_tenant("acme", ScopeAllowance::new(86_400, 500).with_lifetime(800))
}

fn treasurer(policy: AllowancePolicy, ledger: &Arc<FakeLedger>) -> Treasurer {
    Treasurer::new(policy, ledger.clone())
}

/// Run one admission for `acme` / `svc-a`.
async fn admit_svc_a(treasurer: &Treasurer) -> Result<Admission, AdmissionError> {
    treasurer.admit(&subject("acme", "svc-a"), None).await
}

/// The refusal an admission returned, panicking with the actual outcome otherwise.
fn refusal_of(result: Result<Admission, AdmissionError>) -> AllowanceRefusal {
    match result {
        Err(AdmissionError::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A comparable, clock-independent digest of a decision: `None` when admitted, otherwise the
/// refusal's figures without `evaluated_at` (which legitimately moves with the real clock).
fn decision(result: &Result<Admission, AdmissionError>) -> String {
    match result {
        Ok(admission) => format!("admitted:{admission:?}"),
        Err(AdmissionError::Refused(r)) => format!(
            "refused:{:?}:{:?}:{:?}:{:?}:{:?}",
            r.scope_kind, r.limit_kind, r.balance, r.ceiling, r.window
        ),
        Err(other) => format!("error:{other}"),
    }
}

// -- the 41-01 tests, moved here over the scripted ledger -----------------

#[tokio::test]
async fn principal_without_an_entry_is_admitted_without_a_ledger_read() {
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = treasurer(key_policy(), &ledger);

    let admission = treasurer
        .admit(&subject("acme", "svc-b"), None)
        .await
        .expect("no entry is admitted");

    assert!(admission.is_empty());
    assert_eq!(ledger.store_now_calls(), 0);
    assert!(ledger.balance_calls().is_empty());
}

#[tokio::test]
async fn balance_at_the_ceiling_is_refused_with_the_window_figures() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C);
    let treasurer = treasurer(key_policy(), &ledger);

    let refusal = refusal_of(admit_svc_a(&treasurer).await);

    assert_eq!(refusal.scope_kind, AllowanceScopeKind::ApiKey);
    assert_eq!(refusal.limit_kind, AllowanceLimitKind::Window);
    assert_eq!(refusal.balance.nanos(), C);
    assert_eq!(refusal.ceiling.nanos(), C);
    assert_eq!(refusal.window, Some((at(WS), at(WE))));
    assert_eq!(refusal.evaluated_at, at(NOW));

    let queries = ledger.balance_calls();
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].tenant_id, "acme");
    assert_eq!(queries[0].api_key_id.as_deref(), Some("svc-a"));
    assert_eq!(queries[0].since, Some(at(WS)));
    assert_eq!(queries[0].until, Some(at(WE)));
}

#[tokio::test]
async fn balance_below_the_ceiling_is_admitted() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C - 1);
    let treasurer = treasurer(key_policy(), &ledger);

    let admission = admit_svc_a(&treasurer).await.expect("below the ceiling");

    assert!(admission.is_empty());
    assert_eq!(ledger.store_now_calls(), 1);
}

#[tokio::test]
async fn the_store_clock_is_truncated_to_whole_seconds() {
    let ledger = FakeLedger::new(at_900ms(NOW)).with_row("acme", "svc-a", at(WS), C);
    let treasurer = treasurer(key_policy(), &ledger);

    let refusal = refusal_of(admit_svc_a(&treasurer).await);

    assert_eq!(refusal.evaluated_at, at(NOW));
}

// -- window selection and boundaries (D-01, D-05) --------------------------

#[tokio::test]
async fn store_clock_selects_the_window_not_the_local_clock() {
    // A scripted store clock reading 2001-09-09T01:46:40Z. Were the process clock consulted the
    // window would sit decades away from this instant.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C);
    let treasurer = treasurer(key_policy(), &ledger);

    let refusal = refusal_of(admit_svc_a(&treasurer).await);

    let (start, end) = refusal.window.expect("a window refusal carries its window");
    assert_eq!(start, at(NOW / 3_600 * 3_600), "floor(now / P) * P");
    assert_eq!(start, at(WS));
    assert_eq!(end, at(WS + 3_600));
    assert_eq!(refusal.evaluated_at, at(NOW));
    assert_eq!(
        start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "2001-09-09T01:00:00Z"
    );
    // The real clock is years past the scripted one: it cannot have chosen this window.
    assert!(Utc::now() > refusal.evaluated_at + chrono::Duration::days(365));
}

#[tokio::test]
async fn instant_at_window_end_starts_a_new_window() {
    // A row one second before the window's end counts toward that window...
    let ledger = FakeLedger::new(at(WE - 1)).with_row("acme", "svc-a", at(WE - 1), C);
    let treasurer = treasurer(key_policy(), &ledger);

    let refusal = refusal_of(admit_svc_a(&treasurer).await);
    assert_eq!(refusal.window, Some((at(WS), at(WE))));
    assert_eq!(refusal.balance.nanos(), C);

    // ...but the store instant `window_end` itself already belongs to the next window, whose
    // balance starts at zero.
    ledger.set_now(at(WE));
    let admission = admit_svc_a(&treasurer)
        .await
        .expect("the next window starts empty");
    assert!(admission.is_empty());

    let queries = ledger.balance_calls();
    let last = queries.last().expect("the second admission read a balance");
    assert_eq!(last.since, Some(at(WE)));
    assert_eq!(last.until, Some(at(WE + 3_600)));
}

#[tokio::test]
async fn balance_exactly_at_the_ceiling_is_refused_and_one_nano_below_is_admitted() {
    let at_ceiling = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C);
    let refusal = refusal_of(admit_svc_a(&treasurer(key_policy(), &at_ceiling)).await);
    assert_eq!(refusal.balance.nanos(), C);
    assert_eq!(refusal.ceiling.nanos(), C);

    let below = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C - 1);
    admit_svc_a(&treasurer(key_policy(), &below))
        .await
        .expect("one nano-unit below the ceiling is admitted");
}

#[tokio::test]
async fn retry_after_counts_whole_seconds_to_window_end_from_the_store_clock() {
    let ledger = FakeLedger::new(at(WE - 1)).with_row("acme", "svc-a", at(WS), C);
    let treasurer = treasurer(key_policy(), &ledger);

    // One second before the window ends: retry in one second.
    assert_eq!(
        refusal_of(admit_svc_a(&treasurer).await).retry_after_secs(),
        Some(1)
    );

    // At the window's start: the full period.
    ledger.set_now(at(WS));
    assert_eq!(
        refusal_of(admit_svc_a(&treasurer).await).retry_after_secs(),
        Some(P)
    );

    // 0.9 s past the start truncates to the start, so the full period still remains.
    ledger.set_now(at_900ms(WS));
    let refusal = refusal_of(admit_svc_a(&treasurer).await);
    assert_eq!(refusal.evaluated_at, at(WS));
    assert_eq!(refusal.retry_after_secs(), Some(P));
}

#[tokio::test]
async fn lifetime_ceiling_ignores_time_and_has_no_retry_after() {
    // The window ceiling is huge, so only the lifetime cap can refuse; the spend sits far
    // outside any current window.
    let policy = AllowancePolicy::new(usd(), 80)
        .with_api_key("svc-a", ScopeAllowance::new(P, 1_000_000).with_lifetime(C));
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(0), C);
    let treasurer = treasurer(policy, &ledger);

    let first = refusal_of(admit_svc_a(&treasurer).await);
    assert_eq!(first.scope_kind, AllowanceScopeKind::ApiKey);
    assert_eq!(first.limit_kind, AllowanceLimitKind::Lifetime);
    assert_eq!(first.window, None);
    assert_eq!(first.retry_after_secs(), None);

    let lifetime_query = ledger
        .balance_calls()
        .into_iter()
        .find(|q| q.since.is_none())
        .expect("the lifetime ceiling issued a query");
    assert_eq!((lifetime_query.since, lifetime_query.until), (None, None));

    // Years later the decision is the same.
    ledger.set_now(at(2_000_000_000));
    let later = refusal_of(admit_svc_a(&treasurer).await);
    assert_eq!(later.limit_kind, AllowanceLimitKind::Lifetime);
    assert_eq!(later.balance, first.balance);
    assert_eq!(later.ceiling, first.ceiling);
    assert_eq!(later.window, None);
    assert_eq!(later.retry_after_secs(), None);
}

// -- ordering and short-circuiting (D-02) -----------------------------------

#[tokio::test]
async fn ceilings_are_evaluated_key_window_key_lifetime_tenant_window_tenant_lifetime() {
    // Different periods so each window query shows which scope's period produced it.
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = treasurer(four_ceiling_policy(), &ledger);

    admit_svc_a(&treasurer).await.expect("nothing is spent");

    let shape: Vec<_> = ledger
        .balance_calls()
        .iter()
        .map(|q| {
            (
                q.api_key_id.clone(),
                q.since.zip(q.until).map(|(s, u)| (u - s).num_seconds()),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (Some("svc-a".to_string()), Some(3_600)),
            (Some("svc-a".to_string()), None),
            (None, Some(86_400)),
            (None, None),
        ]
    );
    assert_eq!(ledger.store_now_calls(), 1, "one clock read per admission");
}

#[tokio::test]
async fn first_exhausted_ceiling_in_order_names_the_refusal() {
    let policy = four_ceiling_policy();
    let old = at(0);

    // Every ceiling exhausted at once: the API-key window names the refusal.
    let all = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 5_000);
    let refusal = refusal_of(admit_svc_a(&treasurer(policy.clone(), &all)).await);
    assert_eq!(
        (refusal.scope_kind, refusal.limit_kind),
        (AllowanceScopeKind::ApiKey, AllowanceLimitKind::Window)
    );

    // Only the key's lifetime cap: spend outside any window, key total at its lifetime ceiling.
    let key_life = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", old, 1_000);
    let refusal = refusal_of(admit_svc_a(&treasurer(policy.clone(), &key_life)).await);
    assert_eq!(
        (refusal.scope_kind, refusal.limit_kind),
        (AllowanceScopeKind::ApiKey, AllowanceLimitKind::Lifetime)
    );

    // Only the tenant window: a sibling key's spend inside the tenant's day, nothing for svc-a.
    let tenant_window = FakeLedger::new(at(NOW)).with_row("acme", "svc-b", at(WS), 500);
    let refusal = refusal_of(admit_svc_a(&treasurer(policy.clone(), &tenant_window)).await);
    assert_eq!(
        (refusal.scope_kind, refusal.limit_kind),
        (AllowanceScopeKind::Tenant, AllowanceLimitKind::Window)
    );

    // Only the tenant lifetime: a sibling key's old spend at the tenant's lifetime ceiling.
    let tenant_life = FakeLedger::new(at(NOW)).with_row("acme", "svc-b", old, 800);
    let refusal = refusal_of(admit_svc_a(&treasurer(policy, &tenant_life)).await);
    assert_eq!(
        (refusal.scope_kind, refusal.limit_kind),
        (AllowanceScopeKind::Tenant, AllowanceLimitKind::Lifetime)
    );
    assert_eq!(refusal.window, None);
}

#[tokio::test]
async fn refusal_short_circuits_remaining_balance_reads() {
    // API-key window exhausted: exactly one balance read.
    let first = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 5_000);
    refusal_of(admit_svc_a(&treasurer(four_ceiling_policy(), &first)).await);
    assert_eq!(first.balance_calls().len(), 1);

    // Only the key lifetime exhausted: two reads, none for the tenant's ceilings.
    let second = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(0), 1_000);
    refusal_of(admit_svc_a(&treasurer(four_ceiling_policy(), &second)).await);
    assert_eq!(second.balance_calls().len(), 2);
}

// -- no entry, scope sums, fail closed (D-03, D-09, D-10) -------------------

#[tokio::test]
async fn principal_without_an_entry_never_touches_the_ledger() {
    // Entries exist only for another key and another tenant.
    let policy = AllowancePolicy::new(usd(), 80)
        .with_api_key("svc-z", ScopeAllowance::new(P, C))
        .with_tenant("globex", ScopeAllowance::new(P, C));

    for failure in [
        FakeFailure::Backend,
        FakeFailure::CurrencyMismatch,
        FakeFailure::StoreNow,
    ] {
        let ledger = FakeLedger::new(at(NOW));
        ledger.set_failure(Some(failure));
        let treasurer = treasurer(policy.clone(), &ledger);

        let admission = admit_svc_a(&treasurer)
            .await
            .unwrap_or_else(|e| panic!("{failure:?}: a principal with no entry is admitted: {e}"));

        assert!(admission.is_empty(), "{failure:?}");
        assert_eq!(ledger.store_now_calls(), 0, "{failure:?}");
        assert!(ledger.balance_calls().is_empty(), "{failure:?}");
    }
}

#[tokio::test]
async fn tenant_ceiling_sums_every_key_of_the_tenant() {
    let policy = AllowancePolicy::new(usd(), 80).with_tenant("acme", ScopeAllowance::new(P, C));

    // k1 at 60 and k2 at 50 sum past the tenant's 100; another tenant's spend never counts.
    let ledger = FakeLedger::new(at(NOW))
        .with_row("acme", "k1", at(WS), 60)
        .with_row("acme", "k2", at(WS), 50)
        .with_row("globex", "k1", at(WS), 10_000);
    let refusal = refusal_of(
        treasurer(policy.clone(), &ledger)
            .admit(&subject("acme", "k1"), None)
            .await,
    );
    assert_eq!(refusal.scope_kind, AllowanceScopeKind::Tenant);
    assert_eq!(refusal.balance.nanos(), 110);

    // Without k2's spend the tenant is under its ceiling.
    let under = FakeLedger::new(at(NOW))
        .with_row("acme", "k1", at(WS), 60)
        .with_row("globex", "k1", at(WS), 10_000);
    treasurer(policy, &under)
        .admit(&subject("acme", "k1"), None)
        .await
        .expect("60 of 100 is admitted");
}

#[tokio::test]
async fn ledger_failure_fails_closed_for_an_allowanced_principal() {
    for failure in [
        FakeFailure::Backend,
        FakeFailure::CurrencyMismatch,
        FakeFailure::StoreNow,
    ] {
        let ledger = FakeLedger::new(at(NOW));
        ledger.set_failure(Some(failure));
        let treasurer = treasurer(key_policy(), &ledger);

        let result = admit_svc_a(&treasurer).await;

        assert!(
            matches!(result, Err(AdmissionError::Backend { .. })),
            "{failure:?} must fail closed with a backend error, got {result:?}"
        );
    }
}

// -- the one balance function -----------------------------------------------

#[tokio::test]
async fn every_ceiling_kind_round_trips_through_the_one_balance_function() {
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = treasurer(four_ceiling_policy(), &ledger);

    admit_svc_a(&treasurer)
        .await
        .expect("all four ceilings fit");

    let shape: Vec<_> = ledger
        .balance_calls()
        .into_iter()
        .map(|q| {
            (
                q.tenant_id,
                q.api_key_id,
                q.since.is_some(),
                q.until.is_some(),
                q.currency,
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (
                "acme".to_string(),
                Some("svc-a".to_string()),
                true,
                true,
                usd()
            ),
            (
                "acme".to_string(),
                Some("svc-a".to_string()),
                false,
                false,
                usd()
            ),
            ("acme".to_string(), None, true, true, usd()),
            ("acme".to_string(), None, false, false, usd()),
        ],
        "key-window, key-lifetime, tenant-window, tenant-lifetime, all in the policy currency"
    );
}

// -- read-only admission over a real ledger (D-05) ---------------------------

/// A real in-memory ledger holding one 100-nano settlement for `acme` / `svc-a`.
async fn seeded_ledger() -> Arc<InMemoryTreasuryLedger> {
    let ledger = Arc::new(InMemoryTreasuryLedger::new());
    ledger
        .settle(SettleRequest::unreserved(
            LedgerScope::new("acme", "svc-a"),
            SettlementKey::new(RunId::new_v7(), 1, 1),
            Cost::new(100, usd()),
            std::collections::BTreeMap::from([("gpt-4".to_string(), 100)]),
        ))
        .await
        .expect("the seed settlement is accepted");
    ledger
}

/// The ledger's observable `spend` view for `acme`: (group, nanos, settlements) per run.
async fn spend_view(ledger: &InMemoryTreasuryLedger) -> Vec<(String, i64, u64)> {
    ledger
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            tenant_id: Some("acme".to_string()),
            ..Default::default()
        })
        .await
        .expect("spend is readable")
        .into_iter()
        .map(|row| (row.group, row.amount.nanos(), row.settlements))
        .collect()
}

/// A lifetime-only policy for `svc-a` so the decision never depends on which window the real
/// in-memory store clock lands in.
fn lifetime_policy(cap: i64) -> AllowancePolicy {
    AllowancePolicy::new(usd(), 80).with_api_key(
        "svc-a",
        ScopeAllowance::new(86_400, i64::MAX).with_lifetime(cap),
    )
}

#[tokio::test]
async fn repeated_admission_is_read_only_and_identical() {
    let ledger = seeded_ledger().await;
    let before = spend_view(&ledger).await;
    assert_eq!(before.len(), 1);

    // Refused case: two identical admissions, identical refusals, nothing written.
    let refusing = Treasurer::new(lifetime_policy(100), ledger.clone());
    let first = admit_svc_a(&refusing).await;
    let second = admit_svc_a(&refusing).await;
    assert!(
        matches!(first, Err(AdmissionError::Refused(_))),
        "{first:?}"
    );
    assert_eq!(decision(&first), decision(&second));

    // Admitted case: identical, empty admissions.
    let admitting = Treasurer::new(lifetime_policy(1_000), ledger.clone());
    let first = admit_svc_a(&admitting).await;
    let second = admit_svc_a(&admitting).await;
    assert!(matches!(first, Ok(ref a) if a.is_empty()), "{first:?}");
    assert_eq!(decision(&first), decision(&second));

    assert_eq!(
        spend_view(&ledger).await,
        before,
        "admission must leave the ledger's rows and settlement counts unchanged"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_admissions_write_no_ledger_rows() {
    let ledger = seeded_ledger().await;
    let before = spend_view(&ledger).await;
    let treasurer = Arc::new(Treasurer::new(lifetime_policy(100), ledger.clone()));

    let mut handles = Vec::new();
    for _ in 0..16 {
        let treasurer = treasurer.clone();
        handles.push(tokio::spawn(async move {
            treasurer.admit(&subject("acme", "svc-a"), None).await
        }));
    }
    let mut decisions = Vec::new();
    for handle in handles {
        let result = handle.await.expect("the admission task did not panic");
        decisions.push(decision(&result));
    }

    assert_eq!(decisions.len(), 16);
    assert!(
        decisions.windows(2).all(|pair| pair[0] == pair[1]),
        "every concurrent admission must reach the same decision: {decisions:?}"
    );
    assert!(decisions[0].starts_with("refused:"), "{}", decisions[0]);
    assert_eq!(
        spend_view(&ledger).await,
        before,
        "admission never writes, so concurrent admissions cannot corrupt the ledger"
    );
}

// -- the warn leg: once-per-window notices (ALLOW-04, D-15, D-16, 41-06) -----------------------

use std::sync::atomic::{AtomicBool, Ordering};

use paladin_core::platform::container::allowance::{NoticeOutcome, NoticeRecord};
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;

/// A [`TreasuryNoticePort`] over a real [`InMemoryTreasuryLedger`] (so deduplication is the
/// adapter's own) that also records every `record` attempt and every `discard`, and can be told
/// to fail either.
#[derive(Default)]
struct RecordingNotices {
    inner: InMemoryTreasuryLedger,
    attempts: Mutex<Vec<NoticeRecord>>,
    discards: Mutex<Vec<Vec<String>>>,
    fail_record: AtomicBool,
    fail_discard: AtomicBool,
}

impl RecordingNotices {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn attempts(&self) -> Vec<NoticeRecord> {
        self.attempts.lock().expect("not poisoned").clone()
    }

    fn discards(&self) -> Vec<Vec<String>> {
        self.discards.lock().expect("not poisoned").clone()
    }
}

#[async_trait]
impl TreasuryNoticePort for RecordingNotices {
    async fn record(&self, notice: &NoticeRecord) -> Result<NoticeOutcome, TreasuryLedgerError> {
        self.attempts
            .lock()
            .expect("not poisoned")
            .push(notice.clone());
        if self.fail_record.load(Ordering::SeqCst) {
            return Err(TreasuryLedgerError::Backend {
                source: "scripted notice store failure".into(),
            });
        }
        self.inner.record(notice).await
    }

    async fn notices_for_run(
        &self,
        run_id: &RunId,
    ) -> Result<Vec<NoticeRecord>, TreasuryLedgerError> {
        self.inner.notices_for_run(run_id).await
    }

    async fn discard(&self, notice_ids: &[String]) -> Result<(), TreasuryLedgerError> {
        self.discards
            .lock()
            .expect("not poisoned")
            .push(notice_ids.to_vec());
        if self.fail_discard.load(Ordering::SeqCst) {
            return Err(TreasuryLedgerError::Backend {
                source: "scripted notice discard failure".into(),
            });
        }
        self.inner.discard(notice_ids).await
    }
}

fn treasurer_with_notices(
    policy: AllowancePolicy,
    ledger: &Arc<FakeLedger>,
    notices: &Arc<RecordingNotices>,
) -> Treasurer {
    treasurer(policy, ledger).with_notices(notices.clone())
}

#[tokio::test]
async fn crossing_at_the_threshold_records_one_notice_with_the_run_id() {
    // 80 of 100 is exactly 80%: the inclusive boundary.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 80);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);
    let run = RunId::new_v7();

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), Some(&run))
        .await
        .expect("a crossing is still admitted");

    assert_eq!(admission.notices().len(), 1);
    let notice = &admission.notices()[0];
    assert_eq!(notice.tenant_id, "acme");
    assert_eq!(notice.api_key_id.as_deref(), Some("svc-a"));
    assert_eq!(notice.run_id.as_ref(), Some(&run));
    assert_eq!(notice.recorded_at, at(NOW), "the truncated store instant");
    let warning = &notice.warning;
    assert_eq!(warning.scope_kind, AllowanceScopeKind::ApiKey);
    assert_eq!(warning.limit_kind, AllowanceLimitKind::Window);
    assert_eq!(warning.balance.nanos(), 80, "the PRE-admission balance");
    assert_eq!(warning.ceiling.nanos(), C);
    assert_eq!(warning.window_start, Some(at(WS)));
    assert_eq!(warning.window_end, Some(at(WE)));
    assert_eq!(warning.warn_at, 80);

    // The durable row exists and names the admitting run.
    let rows = notices.notices_for_run(&run).await.expect("read back");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].notice_id, notice.notice_id);
}

#[tokio::test]
async fn one_nano_below_the_threshold_records_nothing() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 79);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);

    let admission = admit_svc_a(&treasurer).await.expect("admitted");

    assert!(admission.is_empty());
    assert!(
        notices.attempts().is_empty(),
        "a balance below the threshold must not even attempt a claim"
    );
}

#[tokio::test]
async fn second_admission_in_the_window_records_nothing() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);

    let first = admit_svc_a(&treasurer).await.expect("admitted");
    let second = admit_svc_a(&treasurer).await.expect("admitted");

    assert_eq!(first.notices().len(), 1);
    assert!(second.is_empty(), "the window's notice was already won");
    assert_eq!(
        notices.attempts().len(),
        2,
        "the second admission still attempted (and lost) the claim -- dedup is the store's"
    );
}

#[tokio::test]
async fn every_applicable_ceiling_is_checked_for_a_crossing() {
    // Key window 85/100 (85%), tenant window 450/500 (90%) -- the tenant balance includes a
    // sibling key's 365. Key lifetime 85/1000 and tenant lifetime 450/800 (56%) stay below 80%.
    let ledger = FakeLedger::new(at(NOW))
        .with_row("acme", "svc-a", at(WS), 85)
        .with_row("acme", "svc-b", at(WS), 365);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(four_ceiling_policy(), &ledger, &notices);

    let admission = admit_svc_a(&treasurer).await.expect("admitted");

    let shape: Vec<_> = admission
        .warnings()
        .map(|w| {
            (
                w.scope_kind,
                w.limit_kind,
                w.balance.nanos(),
                w.ceiling.nanos(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (
                AllowanceScopeKind::ApiKey,
                AllowanceLimitKind::Window,
                85,
                100
            ),
            (
                AllowanceScopeKind::Tenant,
                AllowanceLimitKind::Window,
                450,
                500
            ),
        ]
    );
    assert_eq!(
        admission.notices()[1].api_key_id,
        None,
        "tenant scope has no key"
    );
}

#[tokio::test]
async fn a_refused_admission_claims_no_notice() {
    // The key window is exhausted (refused) while the tenant window would cross: nothing is
    // claimed, because the claim happens only on the admitted path.
    let ledger = FakeLedger::new(at(NOW))
        .with_row("acme", "svc-a", at(WS), 100)
        .with_row("acme", "svc-b", at(WS), 350);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(four_ceiling_policy(), &ledger, &notices);

    refusal_of(admit_svc_a(&treasurer).await);

    assert!(notices.attempts().is_empty());
}

#[tokio::test]
async fn warn_at_zero_and_one_hundred_never_notify() {
    for warn_at in [0_u8, 100] {
        let policy = AllowancePolicy::new(usd(), 80)
            .with_api_key("svc-a", ScopeAllowance::new(P, C).with_warn_at(warn_at));
        // 99 of 100: as close to the ceiling as an admitted request can be.
        let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 99);
        let notices = RecordingNotices::new();
        let treasurer = treasurer_with_notices(policy, &ledger, &notices);

        let admission = admit_svc_a(&treasurer).await.expect("admitted");

        assert!(admission.is_empty(), "warn_at {warn_at} must never notify");
        assert!(notices.attempts().is_empty(), "warn_at {warn_at}");
    }
}

#[tokio::test]
async fn crossing_math_is_exact_at_a_huge_ceiling() {
    // 80% of i64::MAX is 7_378_697_629_483_820_645.6: the smallest crossing balance is ...646.
    let ceiling = i64::MAX;
    let boundary = 7_378_697_629_483_820_646_i64;
    for (balance, expected) in [(boundary - 1, 0_usize), (boundary, 1)] {
        let policy =
            AllowancePolicy::new(usd(), 80).with_api_key("svc-a", ScopeAllowance::new(P, ceiling));
        let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), balance);
        let notices = RecordingNotices::new();
        let treasurer = treasurer_with_notices(policy, &ledger, &notices);

        let admission = admit_svc_a(&treasurer).await.expect("admitted");

        assert_eq!(admission.notices().len(), expected, "balance {balance}");
    }
}

#[tokio::test]
async fn notices_store_failure_never_blocks_admission() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 90);
    let notices = RecordingNotices::new();
    notices.fail_record.store(true, Ordering::SeqCst);
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);

    let admission = admit_svc_a(&treasurer)
        .await
        .expect("a notice-store failure must never fail an admitted run");

    assert!(admission.is_empty(), "no notice was won");
    assert_eq!(notices.attempts().len(), 1, "the claim was attempted");
}

#[tokio::test]
async fn without_a_notice_store_the_warn_leg_is_off() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 90);
    let treasurer = treasurer(key_policy(), &ledger);

    let admission = admit_svc_a(&treasurer).await.expect("admitted");

    assert!(admission.is_empty());
    // abandon with nothing attached is a no-op, not a panic.
    treasurer.abandon(&admission).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sixteen_concurrent_crossings_yield_exactly_one_notice() {
    // One real in-memory store is BOTH the ledger and the notice store: the seeded balance of
    // 100 is 80% of a 125 lifetime cap, a window-independent crossing.
    let store = seeded_ledger().await;
    let treasurer =
        Arc::new(Treasurer::new(lifetime_policy(125), store.clone()).with_notices(store.clone()));

    let mut handles = Vec::new();
    for _ in 0..16 {
        let treasurer = treasurer.clone();
        let run = RunId::new_v7();
        handles.push(tokio::spawn(async move {
            treasurer.admit(&subject("acme", "svc-a"), Some(&run)).await
        }));
    }
    let mut won = 0;
    for handle in handles {
        let admission = handle
            .await
            .expect("the admission task did not panic")
            .expect("every concurrent admission is admitted");
        won += admission.notices().len();
    }

    assert_eq!(won, 1, "exactly one of the sixteen carries the notice");
}

#[tokio::test]
async fn abandon_discards_only_this_admissions_notices() {
    let policy = AllowancePolicy::new(usd(), 80)
        .with_api_key("svc-a", ScopeAllowance::new(P, C))
        .with_api_key("svc-b", ScopeAllowance::new(P, C));
    let ledger = FakeLedger::new(at(NOW))
        .with_row("acme", "svc-a", at(WS), 85)
        .with_row("acme", "svc-b", at(WS), 85);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(policy, &ledger, &notices);
    let (run_a, run_b, run_retry) = (RunId::new_v7(), RunId::new_v7(), RunId::new_v7());

    let admission_a = treasurer
        .admit(&subject("acme", "svc-a"), Some(&run_a))
        .await
        .expect("admitted");
    let admission_b = treasurer
        .admit(&subject("acme", "svc-b"), Some(&run_b))
        .await
        .expect("admitted");
    assert_eq!(admission_a.notices().len(), 1);
    assert_eq!(admission_b.notices().len(), 1);

    // A's run never persisted: give its notice back.
    treasurer.abandon(&admission_a).await;

    assert_eq!(
        notices.discards(),
        vec![vec![admission_a.notices()[0].notice_id.clone()]],
        "exactly A's own notice id is discarded"
    );
    assert!(
        notices
            .notices_for_run(&run_a)
            .await
            .expect("read")
            .is_empty()
    );
    assert_eq!(
        notices.notices_for_run(&run_b).await.expect("read").len(),
        1,
        "B's notice is untouched"
    );

    // The next admission in the window wins the notice again, with its own run id.
    let retried = treasurer
        .admit(&subject("acme", "svc-a"), Some(&run_retry))
        .await
        .expect("admitted");
    assert_eq!(retried.notices().len(), 1);
    assert_eq!(retried.notices()[0].run_id.as_ref(), Some(&run_retry));
}

#[tokio::test]
async fn abandon_of_an_empty_admission_touches_nothing_and_a_discard_error_is_swallowed() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);

    treasurer.abandon(&Admission::none()).await;
    assert!(notices.discards().is_empty(), "nothing to discard, no call");

    let admission = admit_svc_a(&treasurer).await.expect("admitted");
    notices.fail_discard.store(true, Ordering::SeqCst);
    // A failing discard is logged, never propagated: abandon has no error to return.
    treasurer.abandon(&admission).await;
    assert_eq!(notices.discards().len(), 1);
}

#[tokio::test]
async fn agent_path_admission_records_a_notice_without_a_run_id() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), None)
        .await
        .expect("admitted");

    assert_eq!(admission.notices().len(), 1);
    assert_eq!(admission.notices()[0].run_id, None);
    let attempts = notices.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].run_id, None, "the HTTP agent path has no run");
}

// -- the operator webhook leg: confirm enqueues one delivery per won notice (D-17, 41-08) ------

use paladin_core::platform::container::webhook::{
    WebhookAttemptResult, WebhookDelivery as OperatorDelivery, WebhookDeliveryId,
};
use paladin_ports::output::webhook_delivery_port::{
    WebhookDeliveryPage, WebhookDeliveryRepositoryError, WebhookDeliveryRepositoryPort,
};
use paladin_storage::webhook::in_memory::InMemoryWebhookDeliveryRepository;

const OPERATOR_URL: &str = "https://ops.example.com/allowance";

/// A delivery queue whose `enqueue` always fails and counts its attempts; every other method
/// is unreachable from `confirm`.
#[derive(Default)]
struct FailingDeliveries {
    enqueue_attempts: std::sync::atomic::AtomicU32,
}

#[async_trait]
impl WebhookDeliveryRepositoryPort for FailingDeliveries {
    async fn enqueue(
        &self,
        _delivery: OperatorDelivery,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        self.enqueue_attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(WebhookDeliveryRepositoryError::Backend {
            source: "scripted enqueue failure".into(),
        })
    }

    async fn get(
        &self,
        _delivery_id: &WebhookDeliveryId,
    ) -> Result<Option<OperatorDelivery>, WebhookDeliveryRepositoryError> {
        Ok(None)
    }

    async fn claim_due(
        &self,
        _now: DateTime<Utc>,
        _limit: u32,
    ) -> Result<Vec<OperatorDelivery>, WebhookDeliveryRepositoryError> {
        Ok(Vec::new())
    }

    async fn record_attempt(
        &self,
        _delivery_id: &WebhookDeliveryId,
        _result: WebhookAttemptResult,
    ) -> Result<(), WebhookDeliveryRepositoryError> {
        Ok(())
    }

    async fn list_for_run(
        &self,
        _run_id: &RunId,
        _limit: u32,
        _cursor: Option<WebhookDeliveryId>,
    ) -> Result<WebhookDeliveryPage, WebhookDeliveryRepositoryError> {
        Ok(WebhookDeliveryPage::default())
    }
}

fn treasurer_with_operator_webhook(
    policy: AllowancePolicy,
    ledger: &Arc<FakeLedger>,
    notices: &Arc<RecordingNotices>,
    deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
) -> Treasurer {
    treasurer_with_notices(policy, ledger, notices)
        .with_operator_webhook(OperatorNoticeTarget::new(OPERATOR_URL, deliveries))
}

/// A ledger whose key window (85 of 100) and tenant window (450 of 500) both cross 80%.
fn two_crossing_ledger() -> Arc<FakeLedger> {
    FakeLedger::new(at(NOW))
        .with_row("acme", "svc-a", at(WS), 85)
        .with_row("acme", "svc-b", at(WS), 365)
}

#[tokio::test]
async fn confirm_enqueues_one_operator_delivery_per_won_notice() {
    let ledger = two_crossing_ledger();
    let notices = RecordingNotices::new();
    let deliveries = Arc::new(InMemoryWebhookDeliveryRepository::new());
    let treasurer = treasurer_with_operator_webhook(
        four_ceiling_policy(),
        &ledger,
        &notices,
        deliveries.clone(),
    );
    let run = RunId::new_v7();

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), Some(&run))
        .await
        .expect("admitted");
    assert_eq!(admission.notices().len(), 2);
    // `admit` alone enqueues nothing: the webhook leg is `confirm`'s.
    assert!(
        deliveries
            .claim_due(at(NOW), 10)
            .await
            .expect("claim")
            .is_empty()
    );

    treasurer.confirm(&admission).await;

    let claimed = deliveries.claim_due(at(NOW), 10).await.expect("claim");
    assert_eq!(claimed.len(), 2, "exactly one delivery per won notice");
    for delivery in &claimed {
        assert_eq!(delivery.event, RunEventKind::AllowanceWarning);
        assert_eq!(delivery.url, OPERATOR_URL);
        assert_eq!(delivery.thread_id.as_str(), OPERATOR_NOTICE_THREAD_ID);
        assert_ne!(delivery.run_id, run, "a correlation id no run owns");
    }
    let mut stored: Vec<String> = claimed.iter().map(|d| d.payload.clone()).collect();
    let mut expected: Vec<String> = admission
        .notices()
        .iter()
        .map(|n| serde_json::to_string(&AllowanceWarningPayload::from_notice(n)).expect("json"))
        .collect();
    stored.sort();
    expected.sort();
    assert_eq!(stored, expected, "payload serialized once, stored verbatim");
    // The real admitting run travels in the payload.
    let value: serde_json::Value = serde_json::from_str(&claimed[0].payload).expect("json");
    assert_eq!(value["run_id"], serde_json::json!(run));
    assert_eq!(value["tenant_id"], "acme");
}

#[tokio::test]
async fn operator_delivery_is_not_listed_for_the_admitting_run() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    let deliveries = Arc::new(InMemoryWebhookDeliveryRepository::new());
    let treasurer =
        treasurer_with_operator_webhook(key_policy(), &ledger, &notices, deliveries.clone());
    let run = RunId::new_v7();

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), Some(&run))
        .await
        .expect("admitted");
    treasurer.confirm(&admission).await;

    let listed = deliveries
        .list_for_run(&run, 100, None)
        .await
        .expect("list");
    assert!(
        listed.items.is_empty(),
        "the admitting run's own delivery list never shows an operator notice"
    );
    assert_eq!(
        deliveries
            .claim_due(at(NOW), 10)
            .await
            .expect("claim")
            .len(),
        1,
        "the notice is queued, just not under the run"
    );
}

#[tokio::test]
async fn confirm_without_an_operator_target_enqueues_nothing() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    // No `with_operator_webhook`: the queue below is reachable by nothing the Treasurer holds,
    // and the notice leg alone still works.
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices);
    let run = RunId::new_v7();

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), Some(&run))
        .await
        .expect("admitted");
    treasurer.confirm(&admission).await;

    assert_eq!(admission.notices().len(), 1, "the notice row still occurs");
    assert_eq!(notices.notices_for_run(&run).await.expect("read").len(), 1);
    assert!(format!("{treasurer:?}").contains("operator_webhook: None"));
}

#[tokio::test]
async fn abandon_never_enqueues() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    let deliveries = Arc::new(InMemoryWebhookDeliveryRepository::new());
    let treasurer =
        treasurer_with_operator_webhook(key_policy(), &ledger, &notices, deliveries.clone());

    let admission = admit_svc_a(&treasurer).await.expect("admitted");
    treasurer.abandon(&admission).await;

    assert!(
        deliveries
            .claim_due(at(NOW), 10)
            .await
            .expect("claim")
            .is_empty(),
        "an abandoned admission gives its notice back and sends nothing"
    );
}

#[tokio::test]
async fn enqueue_failure_is_logged_and_never_blocks() {
    let ledger = two_crossing_ledger();
    let notices = RecordingNotices::new();
    let deliveries = Arc::new(FailingDeliveries::default());
    let treasurer = treasurer_with_operator_webhook(
        four_ceiling_policy(),
        &ledger,
        &notices,
        deliveries.clone(),
    );

    let admission = admit_svc_a(&treasurer).await.expect("admitted");
    // Returns normally: `confirm` has no error to surface and the run is unaffected.
    treasurer.confirm(&admission).await;

    assert_eq!(
        deliveries
            .enqueue_attempts
            .load(std::sync::atomic::Ordering::SeqCst),
        2,
        "one attempt per won notice, never retried"
    );
}

#[tokio::test]
async fn agent_path_operator_payload_has_a_null_run_id() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 85);
    let notices = RecordingNotices::new();
    let deliveries = Arc::new(InMemoryWebhookDeliveryRepository::new());
    let treasurer =
        treasurer_with_operator_webhook(key_policy(), &ledger, &notices, deliveries.clone());

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), None)
        .await
        .expect("admitted");
    treasurer.confirm(&admission).await;

    let claimed = deliveries.claim_due(at(NOW), 10).await.expect("claim");
    assert_eq!(claimed.len(), 1);
    let value: serde_json::Value = serde_json::from_str(&claimed[0].payload).expect("json");
    assert!(value["run_id"].is_null(), "no run row on the agent path");
    assert_eq!(
        value["api_key_id"], "svc-a",
        "the key NAME identifies the scope"
    );
}

// -- the mid-run boundary guard (ALLOW-03, Phase 42 D-01..D-03, G11, 42-04) --------------------

use paladin_core::platform::container::allowance::HaltReason;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::spend_guard::{SpendDecision, SpendGuard};

/// The ceiling (in nano-units) the boundary-edge guard tests use.
const GUARD_CEILING: i64 = 1_000;

impl FakeLedger {
    /// Drop every scripted row, so every later balance reads zero.
    fn clear_rows(&self) {
        self.state().rows.clear();
    }

    /// How many `balance` reads have been made.
    fn balance_call_count(&self) -> usize {
        self.state().balance_calls.len()
    }
}

/// One API-key window ceiling of [`GUARD_CEILING`] nano-units per [`P`] seconds for `svc-a`.
fn guard_policy() -> AllowancePolicy {
    AllowancePolicy::new(usd(), 80).with_api_key("svc-a", ScopeAllowance::new(P, GUARD_CEILING))
}

/// A per-run guard for `acme` / `svc-a` over `ledger`.
fn guard_for(policy: AllowancePolicy, ledger: &Arc<FakeLedger>) -> Arc<dyn SpendGuard> {
    Arc::new(treasurer(policy, ledger)).spend_guard(subject("acme", "svc-a"), RunId::new_v7())
}

fn guard_thread() -> ThreadId {
    ThreadId::new("11111111-1111-7111-8111-111111111111").expect("a valid thread id")
}

/// The refusal inside a `Halt(AllowanceExhausted(..))` answer, panicking with the actual one.
fn exhausted_of(decision: SpendDecision) -> AllowanceRefusal {
    match decision {
        SpendDecision::Halt(HaltReason::AllowanceExhausted(refusal)) => refusal,
        other => panic!("expected Halt(AllowanceExhausted), got {other:?}"),
    }
}

#[tokio::test]
async fn guard_halts_at_exactly_the_ceiling_and_continues_one_nano_below() {
    // One nano below: Continue.
    let below = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), GUARD_CEILING - 1);
    assert_eq!(
        guard_for(guard_policy(), &below)
            .check(&guard_thread())
            .await,
        SpendDecision::Continue,
        "999 of 1000 has headroom"
    );

    // Exactly at the ceiling: Halt, carrying the exact figures (D-01: `>=` exhausts).
    let exact = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), GUARD_CEILING);
    let refusal = exhausted_of(
        guard_for(guard_policy(), &exact)
            .check(&guard_thread())
            .await,
    );
    assert_eq!(refusal.balance.nanos(), GUARD_CEILING);
    assert_eq!(refusal.ceiling.nanos(), GUARD_CEILING);

    // One nano above: Halt.
    let above = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), GUARD_CEILING + 1);
    let refusal = exhausted_of(
        guard_for(guard_policy(), &above)
            .check(&guard_thread())
            .await,
    );
    assert_eq!(refusal.balance.nanos(), GUARD_CEILING + 1);
}

#[tokio::test]
async fn guard_reads_every_applicable_ceiling_on_every_check_without_caching() {
    // Two ceilings (key window + key lifetime), both with headroom.
    let policy = AllowancePolicy::new(usd(), 80)
        .with_api_key("svc-a", ScopeAllowance::new(P, 100).with_lifetime(1_000));
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 10);
    let guard = guard_for(policy, &ledger);

    for _ in 0..3 {
        assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    }

    assert_eq!(
        ledger.store_now_calls(),
        3,
        "one store-clock read per boundary, none served from a cache"
    );
    assert_eq!(
        ledger.balance_call_count(),
        6,
        "both ceilings are read at every boundary"
    );
}

#[tokio::test]
async fn guard_for_a_principal_without_a_ceiling_never_reads_the_ledger_even_when_the_ledger_fails()
{
    let ledger = FakeLedger::new(at(NOW));
    ledger.set_failure(Some(FakeFailure::Backend));
    // The policy only names `svc-a`; `svc-b` has no entry.
    let guard = Arc::new(treasurer(key_policy(), &ledger))
        .spend_guard(subject("acme", "svc-b"), RunId::new_v7());

    for _ in 0..3 {
        assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    }

    assert_eq!(ledger.store_now_calls(), 0);
    assert_eq!(ledger.balance_call_count(), 0);
}

#[tokio::test]
async fn guard_fails_closed_with_ledger_unavailable_when_balance_errs() {
    let ledger = FakeLedger::new(at(NOW));
    ledger.set_failure(Some(FakeFailure::Backend));

    assert_eq!(
        guard_for(guard_policy(), &ledger)
            .check(&guard_thread())
            .await,
        SpendDecision::Halt(HaltReason::LedgerUnavailable)
    );
}

#[tokio::test]
async fn guard_fails_closed_with_ledger_unavailable_when_the_store_clock_errs() {
    let ledger = FakeLedger::new(at(NOW));
    ledger.set_failure(Some(FakeFailure::StoreNow));

    assert_eq!(
        guard_for(guard_policy(), &ledger)
            .check(&guard_thread())
            .await,
        SpendDecision::Halt(HaltReason::LedgerUnavailable)
    );
    assert_eq!(
        ledger.balance_call_count(),
        0,
        "no balance is read once the clock failed"
    );
}

#[tokio::test]
async fn guard_fails_closed_with_ledger_unavailable_on_a_currency_mismatch() {
    let ledger = FakeLedger::new(at(NOW));
    ledger.set_failure(Some(FakeFailure::CurrencyMismatch));

    assert_eq!(
        guard_for(guard_policy(), &ledger)
            .check(&guard_thread())
            .await,
        SpendDecision::Halt(HaltReason::LedgerUnavailable)
    );
}

#[tokio::test]
async fn guard_memoises_its_first_halt() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), GUARD_CEILING);
    let guard = guard_for(guard_policy(), &ledger);

    let first = guard.check(&guard_thread()).await;
    exhausted_of(first.clone());
    let (clock_reads, balance_reads) = (ledger.store_now_calls(), ledger.balance_call_count());

    // The balance drops to zero and the window rolls: a fresh read would now Continue.
    ledger.clear_rows();
    ledger.set_now(at(WE + 1));

    for _ in 0..3 {
        assert_eq!(
            guard.check(&guard_thread()).await,
            first,
            "a halted run's guard answers the identical reason"
        );
    }
    assert_eq!(
        ledger.store_now_calls(),
        clock_reads,
        "no further clock read"
    );
    assert_eq!(
        ledger.balance_call_count(),
        balance_reads,
        "no further balance read"
    );
}

#[tokio::test]
async fn guard_memoises_a_ledger_unavailable_halt_too() {
    let ledger = FakeLedger::new(at(NOW));
    ledger.set_failure(Some(FakeFailure::Backend));
    let guard = guard_for(guard_policy(), &ledger);
    assert_eq!(
        guard.check(&guard_thread()).await,
        SpendDecision::Halt(HaltReason::LedgerUnavailable)
    );

    // The ledger recovers, but this run's halt is final (G11); a fork re-runs admission.
    ledger.set_failure(None);
    assert_eq!(
        guard.check(&guard_thread()).await,
        SpendDecision::Halt(HaltReason::LedgerUnavailable)
    );
}

#[tokio::test]
async fn two_guards_on_one_scope_both_halt_once_the_shared_balance_crosses() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), GUARD_CEILING - 1);
    let treasurer = Arc::new(treasurer(guard_policy(), &ledger));
    let run_a = treasurer.spend_guard(subject("acme", "svc-a"), RunId::new_v7());
    let run_b = treasurer.spend_guard(subject("acme", "svc-a"), RunId::new_v7());

    assert_eq!(run_a.check(&guard_thread()).await, SpendDecision::Continue);
    assert_eq!(run_b.check(&guard_thread()).await, SpendDecision::Continue);

    // One more nano of spend, from either run, crosses the shared ceiling.
    let _ = ledger.clone().with_row("acme", "svc-a", at(WS), 1);

    exhausted_of(run_a.check(&guard_thread()).await);
    exhausted_of(run_b.check(&guard_thread()).await);
}

#[test]
fn fail_closed_log_line_names_run_scope_tenant_and_error_only() {
    let run_id = RunId::new_v7();
    let tenant = TenantId::new("acme").expect("valid tenant");
    let line = super::guard::fail_closed_message(
        &run_id,
        ["api_key", "api_key", "tenant"],
        &tenant,
        &"balance unavailable: scripted balance failure",
    );

    assert!(line.contains(&format!("run={run_id}")), "{line}");
    assert!(line.contains("scope=api_key,tenant"), "{line}");
    assert!(line.contains("tenant=acme"), "{line}");
    assert!(line.contains("scripted balance failure"), "{line}");
}

// -- the derived agent budget (ALLOW-05, Phase 42 D-09, D-10) -------------------

use paladin_core::platform::container::allowance::DerivedTokenBudget;
use paladin_core::platform::container::cost::{PriceRow, PriceTable};

/// 10 USD per 1M tokens on its dearest axis (completion).
const DEAR: i64 = 10_000_000_000;

fn prices() -> Arc<PriceTable> {
    Arc::new(
        PriceTable::new(usd())
            .with_row(
                "dear",
                PriceRow::new(1_000_000_000, DEAR).expect("valid row"),
            )
            .with_row(
                "reasoner",
                PriceRow::new(1_000_000_000, 2_000_000_000)
                    .expect("valid row")
                    .with_reasoning(40_000_000_000)
                    .expect("valid price"),
            )
            .with_row("free", PriceRow::new(0, 0).expect("valid row")),
    )
}

/// One key-window ceiling of `ceiling` nanos per hour for `svc-a`.
fn derive_policy(ceiling: i64) -> AllowancePolicy {
    AllowancePolicy::new(usd(), 80).with_api_key("svc-a", ScopeAllowance::new(P, ceiling))
}

fn priced(policy: AllowancePolicy, ledger: &Arc<FakeLedger>) -> Treasurer {
    treasurer(policy, ledger).with_pricing(prices())
}

async fn admit_model(treasurer: &Treasurer, model: &str) -> Result<Admission, AdmissionError> {
    treasurer
        .admit_for_model(&subject("acme", "svc-a"), None, model)
        .await
}

#[tokio::test]
async fn admit_for_model_without_a_ceiling_derives_nothing_and_reads_no_ledger() {
    let ledger = FakeLedger::new(at(NOW));
    // svc-b has no entry; the model is unpriced, which must not matter without a ceiling.
    let treasurer = priced(key_policy(), &ledger);

    let admission = treasurer
        .admit_for_model(&subject("acme", "svc-b"), None, "no-such-model")
        .await
        .expect("no ceiling is admitted");

    assert!(admission.derived_budget().is_none());
    assert!(admission.is_empty());
    assert_eq!(ledger.store_now_calls(), 0);
    assert!(ledger.balance_calls().is_empty());
}

#[tokio::test]
async fn admit_for_model_derives_from_the_remaining_allowance_at_the_dearest_price() {
    // 1 USD ceiling, nothing spent: 1e9 * 1e6 / 1e10 = 100_000 tokens.
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = priced(derive_policy(1_000_000_000), &ledger);

    let admission = admit_model(&treasurer, "dear").await.expect("admitted");

    let budget = admission.derived_budget().expect("a budget is derived");
    assert_eq!(budget.max_tokens, 100_000);
    // A5: the halt figures report the ceiling as the balance.
    assert_eq!(budget.halt_figures.ceiling.nanos(), 1_000_000_000);
    assert_eq!(budget.halt_figures.balance.nanos(), 1_000_000_000);
    assert_eq!(budget.halt_figures.scope_kind, AllowanceScopeKind::ApiKey);
    assert_eq!(budget.halt_figures.limit_kind, AllowanceLimitKind::Window);
    assert_eq!(budget.halt_figures.window, Some((at(WS), at(WE))));
    assert_eq!(budget.halt_figures.evaluated_at, at(NOW));
}

#[tokio::test]
async fn the_dearest_axis_includes_reasoning_above_completion() {
    // reasoning at 40 USD per 1M beats completion at 2: 1e9 * 1e6 / 4e10 = 25_000 tokens.
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = priced(derive_policy(1_000_000_000), &ledger);

    let admission = admit_model(&treasurer, "reasoner").await.expect("admitted");

    assert_eq!(
        admission.derived_budget().map(|b| b.max_tokens),
        Some(25_000)
    );
}

#[tokio::test]
async fn a_derived_figure_of_one_is_admitted_and_zero_is_refused_with_the_real_figures() {
    // remaining 10_000 nanos at 1e10 per 1M -> exactly 1 token: admitted.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 1_000_000 - 10_000);
    let one = priced(derive_policy(1_000_000), &ledger);
    let admission = admit_model(&one, "dear")
        .await
        .expect("one token is admitted");
    assert_eq!(admission.derived_budget().map(|b| b.max_tokens), Some(1));

    // remaining 9_999 nanos -> floor(0.9999) = 0 tokens: refused with the binding ceiling's
    // REAL balance, not the ceiling.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 1_000_000 - 9_999);
    let zero = priced(derive_policy(1_000_000), &ledger);
    let refusal = refusal_of(admit_model(&zero, "dear").await);
    assert_eq!(refusal.ceiling.nanos(), 1_000_000);
    assert_eq!(refusal.balance.nanos(), 1_000_000 - 9_999);
    assert_eq!(refusal.window, Some((at(WS), at(WE))));
    assert_eq!(refusal.retry_after_secs(), Some((WE - NOW) as u64));
}

#[tokio::test]
async fn a_free_model_gets_no_derived_budget() {
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = priced(derive_policy(1_000_000_000), &ledger);

    let admission = admit_model(&treasurer, "free").await.expect("admitted");

    assert!(admission.derived_budget().is_none());
}

#[tokio::test]
async fn an_unpriced_model_under_a_ceiling_is_refused_before_any_notice_claim() {
    // 90 of 100 is past the 80% warn threshold, so a claim WOULD be made if admission got that far.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 90);
    let notices = RecordingNotices::new();
    let treasurer = treasurer_with_notices(key_policy(), &ledger, &notices).with_pricing(prices());

    let result = admit_model(&treasurer, "gpt-unlisted").await;

    match result {
        Err(AdmissionError::ModelUnpriced { model }) => assert_eq!(model, "gpt-unlisted"),
        other => panic!("expected ModelUnpriced, got {other:?}"),
    }
    assert!(
        notices.attempts().is_empty(),
        "a refused derivation claims no notice"
    );
}

#[tokio::test]
async fn no_price_table_at_all_refuses_a_ceilinged_principal_as_unpriced() {
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = treasurer(key_policy(), &ledger);

    let result = admit_model(&treasurer, "dear").await;

    assert!(
        matches!(result, Err(AdmissionError::ModelUnpriced { .. })),
        "{result:?}"
    );
}

#[tokio::test]
async fn an_exhausted_ceiling_is_refused_by_admit_for_model_with_the_exhausted_figures() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C);
    let treasurer = priced(key_policy(), &ledger);

    let refusal = refusal_of(admit_model(&treasurer, "dear").await);

    assert_eq!(refusal.balance.nanos(), C);
    assert_eq!(refusal.ceiling.nanos(), C);
}

#[tokio::test]
async fn the_tightest_ceiling_binds_the_derived_budget() {
    // key window 1e9 (balance 0): headroom 1e9.
    // key lifetime 8e8 (balance 7e8): headroom 1e8 <- binding.
    // tenant window 5e9 (balance 7e8): headroom 4.3e9.
    // tenant lifetime 9e9 (balance 7e8): headroom 8.3e9.
    let policy = AllowancePolicy::new(usd(), 100)
        .with_api_key(
            "svc-a",
            ScopeAllowance::new(P, 1_000_000_000).with_lifetime(800_000_000),
        )
        .with_tenant(
            "acme",
            ScopeAllowance::new(86_400, 5_000_000_000).with_lifetime(9_000_000_000),
        );
    // The spend is attributed two hours before the store instant: outside the key's hourly
    // window, so only the lifetime scopes (and the day-wide tenant window) see it.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(NOW - 7_200), 700_000_000);
    let treasurer = priced(policy, &ledger);

    let admission = admit_model(&treasurer, "dear").await.expect("admitted");

    let budget = admission.derived_budget().expect("derived");
    // The key lifetime has the least headroom (1e8): 1e8 * 1e6 / 1e10 = 10_000 tokens.
    assert_eq!(budget.max_tokens, 10_000);
    assert_eq!(budget.halt_figures.scope_kind, AllowanceScopeKind::ApiKey);
    assert_eq!(budget.halt_figures.limit_kind, AllowanceLimitKind::Lifetime);
    assert_eq!(budget.halt_figures.ceiling.nanos(), 800_000_000);
    assert_eq!(budget.halt_figures.balance.nanos(), 800_000_000);
    assert_eq!(budget.halt_figures.window, None);
}

#[tokio::test]
async fn a_tie_between_ceilings_goes_to_the_first_in_policy_order() {
    // The key window and the tenant window have identical headroom; the key window is first in
    // policy order, so it is the binding one.
    let policy = AllowancePolicy::new(usd(), 100)
        .with_api_key("svc-a", ScopeAllowance::new(P, 1_000_000_000))
        .with_tenant("acme", ScopeAllowance::new(P, 1_000_000_000));
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = priced(policy, &ledger);

    let admission = admit_model(&treasurer, "dear").await.expect("admitted");

    let budget = admission.derived_budget().expect("derived");
    assert_eq!(budget.halt_figures.scope_kind, AllowanceScopeKind::ApiKey);
}

#[tokio::test]
async fn a_price_table_in_another_currency_is_a_backend_error_naming_both_codes() {
    let ledger = FakeLedger::new(at(NOW));
    let eur = CurrencyCode::new("EUR").expect("EUR is valid");
    let table = PriceTable::new(eur).with_row("dear", PriceRow::new(1, DEAR).expect("valid row"));
    let treasurer = treasurer(derive_policy(1_000_000_000), &ledger).with_pricing(Arc::new(table));

    let result = admit_model(&treasurer, "dear").await;

    match result {
        Err(AdmissionError::Backend { message }) => {
            assert!(
                message.contains("EUR") && message.contains("USD"),
                "{message}"
            );
        }
        other => panic!("expected a backend error, got {other:?}"),
    }
}

#[tokio::test]
async fn the_plain_admit_never_derives_a_budget() {
    let ledger = FakeLedger::new(at(NOW));
    let treasurer = priced(derive_policy(1_000_000_000), &ledger);

    let admission = treasurer
        .admit(&subject("acme", "svc-a"), None)
        .await
        .expect("admitted");

    assert!(admission.derived_budget().is_none());
}

#[tokio::test]
async fn derive_budget_is_idempotent_and_writes_nothing() {
    let ledger = seeded_ledger().await;
    let before = spend_view(&ledger).await;
    let treasurer =
        Treasurer::new(lifetime_policy(1_000_000_000), ledger.clone()).with_pricing(prices());
    let who = subject("acme", "svc-a");

    let first = treasurer
        .derive_budget(&who, "dear")
        .await
        .expect("derives");
    let second = treasurer
        .derive_budget(&who, "dear")
        .await
        .expect("derives");

    let first: Option<DerivedTokenBudget> = first;
    assert!(first.is_some());
    // The store clock may tick between the calls; the figures that matter must not.
    assert_eq!(
        first.as_ref().map(|b| b.max_tokens),
        second.as_ref().map(|b| b.max_tokens)
    );
    assert_eq!(
        first
            .as_ref()
            .map(|b| (&b.halt_figures.ceiling, &b.halt_figures.balance)),
        second
            .as_ref()
            .map(|b| (&b.halt_figures.ceiling, &b.halt_figures.balance)),
    );
    assert_eq!(
        spend_view(&ledger).await,
        before,
        "derivation writes nothing"
    );
}

#[tokio::test]
async fn deriving_twice_over_an_unchanged_ledger_and_clock_yields_equal_budgets() {
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), 1_234_567);
    let treasurer = priced(derive_policy(1_000_000_000), &ledger);
    let who = subject("acme", "svc-a");

    let first = treasurer
        .derive_budget(&who, "dear")
        .await
        .expect("derives");
    let second = treasurer
        .derive_budget(&who, "dear")
        .await
        .expect("derives");

    assert!(first.is_some());
    assert_eq!(first, second);
}

// -- the guard's notice legs (ALLOW-03, Phase 42 D-17, D-18, Pitfall 9, 42-11) --------------------

use paladin_core::platform::container::allowance::NoticeKind;
use paladin_core::platform::container::trace::TraceEvent;
use paladin_ports::output::trace_sink_port::TraceEmitter;

/// A [`TraceEmitter`] that records every event it is handed.
#[derive(Default)]
struct RecordingEmitter {
    events: Mutex<Vec<TraceEvent>>,
}

impl RecordingEmitter {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn events(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("not poisoned").clone()
    }
}

impl TraceEmitter for RecordingEmitter {
    fn emit(&self, event: TraceEvent) {
        self.events.lock().expect("not poisoned").push(event);
    }
}

/// The whole notice harness a guard test needs: a scripted ledger, a recording notice store, an
/// in-memory operator queue and a Treasurer wired to all three over [`key_policy`] (ceiling
/// [`C`], warn at 80).
struct NoticeHarness {
    ledger: Arc<FakeLedger>,
    notices: Arc<RecordingNotices>,
    deliveries: Arc<InMemoryWebhookDeliveryRepository>,
    treasurer: Arc<Treasurer>,
}

impl NoticeHarness {
    /// A harness whose key window starts at `spent` nano-units.
    fn new(spent: i64) -> Self {
        let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), spent);
        let notices = RecordingNotices::new();
        let deliveries = Arc::new(InMemoryWebhookDeliveryRepository::new());
        let treasurer = Arc::new(treasurer_with_operator_webhook(
            key_policy(),
            &ledger,
            &notices,
            deliveries.clone(),
        ));
        Self {
            ledger,
            notices,
            deliveries,
            treasurer,
        }
    }

    /// Spend `more` further nano-units in the current window.
    fn spend(&self, more: i64) {
        let _ = self.ledger.clone().with_row("acme", "svc-a", at(WS), more);
    }

    /// A guard for a fresh run, with `emitter` attached.
    fn guard(&self, emitter: Option<Arc<RecordingEmitter>>) -> (Arc<dyn SpendGuard>, RunId) {
        let run = RunId::new_v7();
        let emitter = emitter.map(|e| e as Arc<dyn TraceEmitter>);
        let guard =
            self.treasurer
                .spend_guard_with_emitter(subject("acme", "svc-a"), run.clone(), emitter);
        (guard, run)
    }

    /// Every operator delivery queued so far.
    async fn queued(&self) -> Vec<OperatorDelivery> {
        self.deliveries
            .claim_due(at(NOW), 100)
            .await
            .expect("claim")
    }
}

fn warnings_in(events: &[TraceEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, TraceEvent::AllowanceWarning { .. }))
        .count()
}

#[tokio::test]
async fn guard_claims_a_mid_run_warning_once() {
    // Admitted below warn_at (50 of 100, threshold 80).
    let harness = NoticeHarness::new(50);
    let emitter = RecordingEmitter::new();
    let (guard, run) = harness.guard(Some(emitter.clone()));

    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    assert!(harness.notices.attempts().is_empty(), "no crossing yet");

    // The run spends past the threshold (85 of 100): the first boundary claims the warning.
    harness.spend(35);
    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);

    let attempts = harness.notices.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].kind, NoticeKind::Warning);
    assert_eq!(attempts[0].run_id.as_ref(), Some(&run));
    assert_eq!(attempts[0].warning.balance.nanos(), 85);
    assert_eq!(attempts[0].warning.warn_at, 80);
    assert_eq!(attempts[0].recorded_at, at(NOW));
    let events = emitter.events();
    assert_eq!(warnings_in(&events), 1, "exactly one AllowanceWarning");
    assert_eq!(events.len(), 1);
    let queued = harness.queued().await;
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].event, RunEventKind::AllowanceWarning);

    // Later boundaries make no further notice write and emit nothing (Pitfall 9).
    for _ in 0..3 {
        assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    }
    assert_eq!(
        harness.notices.attempts().len(),
        1,
        "the memo skips repeats"
    );
    assert_eq!(warnings_in(&emitter.events()), 1);
    assert!(harness.queued().await.is_empty(), "nothing further queued");
}

/// WR-3 (42-REVIEW): a notice-store write that FAILS must not be memoised as "tried". The memo
/// only skips writes the store would answer `AlreadyRecorded` to, so a transient failure leaves
/// the claim retryable at the next boundary, and the operator is still warned once the store
/// recovers.
#[tokio::test]
async fn a_failed_mid_run_warning_claim_is_retried_at_the_next_boundary() {
    let harness = NoticeHarness::new(50);
    let emitter = RecordingEmitter::new();
    let (guard, _run) = harness.guard(Some(emitter.clone()));
    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);

    // The run crosses warn_at (85 of 100) while the notice store is down.
    harness.spend(35);
    harness.notices.fail_record.store(true, Ordering::SeqCst);
    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    assert_eq!(harness.notices.attempts().len(), 1, "the write was tried");
    assert!(
        emitter.events().is_empty(),
        "nothing was won, nothing emitted"
    );
    assert!(harness.queued().await.is_empty());

    // The store recovers: the very next boundary retries and wins the claim.
    harness.notices.fail_record.store(false, Ordering::SeqCst);
    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    assert_eq!(
        harness.notices.attempts().len(),
        2,
        "a failed write is not memoised, so it is retried"
    );
    assert_eq!(
        warnings_in(&emitter.events()),
        1,
        "the warning is delivered"
    );
    let queued = harness.queued().await;
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].event, RunEventKind::AllowanceWarning);

    // Once recorded, the memo applies again: no further write.
    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    assert_eq!(
        harness.notices.attempts().len(),
        2,
        "the memo skips repeats"
    );
}

#[tokio::test]
async fn a_lost_warning_claim_emits_and_enqueues_nothing() {
    let harness = NoticeHarness::new(85);
    let first_emitter = RecordingEmitter::new();
    let second_emitter = RecordingEmitter::new();
    let (first, _) = harness.guard(Some(first_emitter.clone()));
    let (second, _) = harness.guard(Some(second_emitter.clone()));

    assert_eq!(first.check(&guard_thread()).await, SpendDecision::Continue);
    assert_eq!(second.check(&guard_thread()).await, SpendDecision::Continue);

    assert_eq!(harness.notices.attempts().len(), 2, "each run tried once");
    assert_eq!(warnings_in(&first_emitter.events()), 1, "the winner emits");
    assert!(
        second_emitter.events().is_empty(),
        "a lost claim emits nothing"
    );
    assert_eq!(harness.queued().await.len(), 1, "one delivery per window");
}

#[tokio::test]
async fn a_warning_the_admission_already_claimed_is_not_claimed_again_mid_run() {
    let harness = NoticeHarness::new(85);
    let admission = harness
        .treasurer
        .admit(&subject("acme", "svc-a"), Some(&RunId::new_v7()))
        .await
        .expect("admitted");
    assert_eq!(admission.notices().len(), 1, "admission won the window");
    let emitter = RecordingEmitter::new();
    let (guard, _) = harness.guard(Some(emitter.clone()));

    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);

    assert!(
        emitter.events().is_empty(),
        "the store says already claimed"
    );
    assert!(harness.queued().await.is_empty());
}

#[tokio::test]
async fn guard_halt_claims_one_halt_notice_and_one_operator_delivery() {
    let harness = NoticeHarness::new(C);
    let emitter = RecordingEmitter::new();
    let (guard, run) = harness.guard(Some(emitter.clone()));

    exhausted_of(guard.check(&guard_thread()).await);

    let attempts = harness.notices.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].kind, NoticeKind::Halt);
    assert_eq!(attempts[0].run_id.as_ref(), Some(&run));
    assert_eq!(attempts[0].warning.balance.nanos(), C);
    assert_eq!(attempts[0].warning.ceiling.nanos(), C);
    assert_eq!(attempts[0].warning.warn_at, 80, "the configured threshold");
    assert_eq!(attempts[0].warning.window_start, Some(at(WS)));
    assert_eq!(attempts[0].warning.window_end, Some(at(WE)));
    assert_eq!(attempts[0].recorded_at, at(NOW));

    let queued = harness.queued().await;
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].event, RunEventKind::AllowanceHalted);
    assert_eq!(queued[0].thread_id.as_str(), OPERATOR_NOTICE_THREAD_ID);
    let payload: serde_json::Value = serde_json::from_str(&queued[0].payload).expect("json");
    assert_eq!(payload["event"], "allowance_halted");
    assert_eq!(payload["run_id"], serde_json::json!(run));
    assert_eq!(payload["warn_at"], 80);
    assert_eq!(
        payload.as_object().expect("an object").len(),
        12,
        "the warning's twelve keys, no thirteenth"
    );
    assert_eq!(
        warnings_in(&emitter.events()),
        0,
        "a halt notice is not a trace warning"
    );

    // The halt is sticky: later boundaries re-claim nothing.
    for _ in 0..3 {
        exhausted_of(guard.check(&guard_thread()).await);
    }
    assert_eq!(harness.notices.attempts().len(), 1);
    assert!(harness.queued().await.is_empty());
}

#[tokio::test]
async fn three_guards_halting_in_one_window_enqueue_one_allowance_halted_delivery() {
    let harness = NoticeHarness::new(C);
    let guards: Vec<_> = (0..3).map(|_| harness.guard(None).0).collect();

    for guard in &guards {
        exhausted_of(guard.check(&guard_thread()).await);
    }

    let halts = harness
        .notices
        .attempts()
        .into_iter()
        .filter(|record| record.kind == NoticeKind::Halt)
        .count();
    assert_eq!(halts, 3, "every run tries once; the store decides");
    let queued = harness.queued().await;
    assert_eq!(
        queued.len(),
        1,
        "one operator delivery for the window (D-18)"
    );
    assert_eq!(queued[0].event, RunEventKind::AllowanceHalted);
}

#[tokio::test]
async fn ledger_unavailable_halt_claims_no_notice_and_enqueues_nothing() {
    let harness = NoticeHarness::new(0);
    harness.ledger.set_failure(Some(FakeFailure::Backend));
    let (guard, _) = harness.guard(Some(RecordingEmitter::new()));

    assert_eq!(
        guard.check(&guard_thread()).await,
        SpendDecision::Halt(HaltReason::LedgerUnavailable)
    );

    assert!(harness.notices.attempts().is_empty());
    assert!(harness.queued().await.is_empty());
}

#[tokio::test]
async fn a_guard_built_without_an_emitter_still_claims_and_notifies_but_emits_nothing() {
    let harness = NoticeHarness::new(85);
    // The published constructor: no emitter.
    let run = RunId::new_v7();
    let guard = harness
        .treasurer
        .spend_guard(subject("acme", "svc-a"), run.clone());

    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);

    let attempts = harness.notices.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].run_id.as_ref(), Some(&run));
    let queued = harness.queued().await;
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].event, RunEventKind::AllowanceWarning);
}

#[tokio::test]
async fn a_notice_store_failure_never_changes_the_guard_decision() {
    // Warn leg: a failing store still lets the run continue.
    let warn = NoticeHarness::new(85);
    warn.notices.fail_record.store(true, Ordering::SeqCst);
    let emitter = RecordingEmitter::new();
    let (guard, _) = warn.guard(Some(emitter.clone()));
    assert_eq!(guard.check(&guard_thread()).await, SpendDecision::Continue);
    assert!(emitter.events().is_empty(), "no claim won, nothing emitted");
    assert!(warn.queued().await.is_empty());

    // Halt leg: a failing store still halts the run, with the exhausted figures.
    let halt = NoticeHarness::new(C);
    halt.notices.fail_record.store(true, Ordering::SeqCst);
    let (guard, _) = halt.guard(None);
    let refusal = exhausted_of(guard.check(&guard_thread()).await);
    assert_eq!(refusal.balance.nanos(), C);
    assert!(halt.queued().await.is_empty());
}

#[tokio::test]
async fn a_guard_without_a_notice_store_neither_claims_nor_notifies() {
    // The ledger and policy only: the warn and halt legs are both off.
    let ledger = FakeLedger::new(at(NOW)).with_row("acme", "svc-a", at(WS), C);
    let guard = guard_for(key_policy(), &ledger);
    exhausted_of(guard.check(&guard_thread()).await);
}
