//! Shared `TreasuryLedgerPort` contract suite (D-11, LEDGR-01..04).
//!
//! One generic async function per contract clause, each taking `&dyn TreasuryLedgerPort` (or,
//! for the two concurrency clauses, `Arc<dyn TreasuryLedgerPort>`) and asserting inside. Every
//! backend (`InMemoryTreasuryLedger`, `SqliteTreasuryLedger`, `PostgresTreasuryLedger`) invokes
//! these functions unchanged from its own `#[tokio::test]`s, so "identical suite across
//! backends" is enforced by construction rather than by convention, mirroring
//! `crate::run::contract_tests`'s house pattern. Named per clause (not a declarative macro) so
//! a failure names the violated contract clause rather than a line number.
//!
//! Every clause isolates itself with a unique tenant id (via [`contract_scope`]) or unique run
//! ids, and every `spend` call inside a clause filters by that clause's own tenant id or run
//! ids — a shared database (the Postgres suite, 39-03) never leaks rows between clauses.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use paladin_core::platform::container::cost::{Cost, CurrencyCode};
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::treasury_ledger::{
    LedgerScope, ReservationId, ReserveRequest, SettleOutcome, SettleRequest, SettlementKey,
    SpendGroupBy, SpendQuery,
};
use paladin_ports::output::treasury_ledger_port::{TreasuryLedgerError, TreasuryLedgerPort};

/// The USD currency code, used by every clause that does not specifically exercise a
/// currency-mismatch path.
pub fn usd() -> CurrencyCode {
    CurrencyCode::new("USD").expect("USD is a valid currency code")
}

/// A second currency, used only by clauses proving currencies are never combined.
pub fn eur() -> CurrencyCode {
    CurrencyCode::new("EUR").expect("EUR is a valid currency code")
}

/// A fresh, clause-unique [`LedgerScope`] (`contract-<clause>-<uuid v4>` tenant,
/// `key-<clause>` api key) so concurrent clauses (and a shared Postgres database across
/// clauses, 39-03) never see each other's rows.
pub fn contract_scope(clause: &str) -> LedgerScope {
    LedgerScope::new(
        format!("contract-{clause}-{}", Uuid::new_v4()),
        format!("key-{clause}"),
    )
}

/// Build an unreserved [`SettleRequest`] for `nanos` of `currency`, with a single-model
/// breakdown `{model: nanos}`.
pub fn settle_request(
    scope: LedgerScope,
    key: SettlementKey,
    nanos: i64,
    currency: CurrencyCode,
    model: &str,
) -> SettleRequest {
    let amount = Cost::new(nanos, currency);
    SettleRequest::unreserved(
        scope,
        key,
        amount,
        BTreeMap::from([(model.to_string(), nanos)]),
    )
}

/// Read a scope+window's current balance without writing anything: reserves a hold of
/// `i64::MAX` against a ceiling of `0`, which must always be refused (D-03 admits only when
/// `balance + hold <= ceiling`, and no finite balance clears that bar against `i64::MAX`), and
/// returns the `balance` the `Refused` error names.
///
/// # Panics
///
/// Panics if `port.reserve` returns anything other than
/// [`TreasuryLedgerError::Refused`] (a currency mismatch, most likely, if the caller passed a
/// currency other than what is already recorded in this scope+window).
pub async fn observed_balance(
    port: &dyn TreasuryLedgerPort,
    scope: &LedgerScope,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    currency: &CurrencyCode,
) -> i64 {
    let request = ReserveRequest {
        scope: scope.clone(),
        hold: Cost::new(i64::MAX, currency.clone()),
        ceiling: Cost::new(0, currency.clone()),
        window_start,
        window_end,
        key: None,
    };
    match port.reserve(request).await {
        Err(TreasuryLedgerError::Refused { balance, .. }) => balance.nanos(),
        other => panic!("observed_balance: expected Refused, got {other:?}"),
    }
}

// ── Race (LEDGR-02, the phase's first red test) ───────────────────────────

/// Sixteen concurrent `reserve` calls of hold `1` nano USD against ceiling `15` on a fresh scope
/// yield exactly 15 `Ok` and one `Refused { balance: 15, hold: 1, ceiling: 15 }`.
pub async fn reserve_race_admits_exactly_n_minus_one(port: Arc<dyn TreasuryLedgerPort>) {
    let scope = contract_scope("reserve-race");
    let currency = usd();
    let now = port.store_now().await.expect("store_now must succeed");
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);

    let mut handles = Vec::new();
    for _ in 0..16 {
        let port = Arc::clone(&port);
        let scope = scope.clone();
        let currency = currency.clone();
        let request = ReserveRequest {
            scope,
            hold: Cost::new(1, currency.clone()),
            ceiling: Cost::new(15, currency),
            window_start,
            window_end,
            key: None,
        };
        handles.push(tokio::spawn(async move { port.reserve(request).await }));
    }

    let mut ok_count = 0;
    let mut refused = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        for handle in handles {
            match handle.await.expect("reserve task must not panic") {
                Ok(_) => ok_count += 1,
                Err(err) => refused.push(err),
            }
        }
    })
    .await
    .expect("16 concurrent reserves must not hang");

    assert_eq!(ok_count, 15, "exactly 15 of 16 reserves must be admitted");
    assert_eq!(refused.len(), 1, "exactly one reserve must be refused");
    match &refused[0] {
        TreasuryLedgerError::Refused {
            balance,
            hold,
            ceiling,
        } => {
            assert_eq!(balance.nanos(), 15);
            assert_eq!(hold.nanos(), 1);
            assert_eq!(ceiling.nanos(), 15);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

// ── Adjacency and overflow (LEDGR-02 edge) ────────────────────────────────

/// A hold that brings the balance exactly to the ceiling is admitted; a further hold of 1 nano
/// is refused with the exact figures; a hold of `i64::MAX` against a ceiling of `0` is refused
/// because the overflowing add does not fit.
pub async fn reserve_admits_at_the_ceiling_and_refuses_one_past_it(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("reserve-adjacency");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);

    let first = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(5, currency.clone()),
            ceiling: Cost::new(5, currency.clone()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(
        first.is_ok(),
        "hold == ceiling must be admitted, got {first:?}"
    );

    let second = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(1, currency.clone()),
            ceiling: Cost::new(5, currency.clone()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    match second {
        Err(TreasuryLedgerError::Refused {
            balance,
            hold,
            ceiling,
        }) => {
            assert_eq!(balance.nanos(), 5);
            assert_eq!(hold.nanos(), 1);
            assert_eq!(ceiling.nanos(), 5);
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    // A real `balance + hold` overflow (not just a hold exceeding the ceiling): put a small
    // balance in a fresh scope, then attempt a hold of `i64::MAX` against a ceiling of
    // `i64::MAX` -- the ceiling would admit the hold alone, but `balance.checked_add(hold)`
    // overflows, and an overflowing add must count as not fitting (never wrap or panic).
    let overflow_scope = contract_scope("reserve-overflow");
    port.reserve(ReserveRequest {
        scope: overflow_scope.clone(),
        hold: Cost::new(1, currency.clone()),
        ceiling: Cost::new(1, currency.clone()),
        window_start,
        window_end,
        key: None,
    })
    .await
    .expect("the small priming reserve must be admitted");

    let overflow = port
        .reserve(ReserveRequest {
            scope: overflow_scope,
            hold: Cost::new(i64::MAX, currency.clone()),
            ceiling: Cost::new(i64::MAX, currency),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(
        matches!(overflow, Err(TreasuryLedgerError::Refused { .. })),
        "an overflowing balance + hold must refuse (never wrap or panic), got {overflow:?}"
    );
}

// ── Balance math (ADR-0053 §2) ────────────────────────────────────────────

/// `reserve` 10, then `settle` actual 7 referencing it: observed balance is 7, and `spend` by
/// run shows 7 with 1 settlement.
pub async fn reserve_then_settle_contributes_actual_minus_hold(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("reserve-settle-actual-minus-hold");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);
    let run_id = RunId::new_v7();
    let key = SettlementKey::new(run_id.clone(), 1, 1);

    let reservation = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(10, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start,
            window_end,
            key: Some(key.clone()),
        })
        .await
        .expect("reserve must be admitted");

    let outcome = port
        .settle(SettleRequest {
            scope: scope.clone(),
            key,
            amount: Cost::new(7, currency.clone()),
            model_breakdown: BTreeMap::from([("gpt-4".to_string(), 7)]),
            reservation: Some(reservation),
        })
        .await
        .expect("settle must succeed");
    assert_eq!(outcome, SettleOutcome::Settled);

    let balance = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(
        balance, 7,
        "reserve 10 then settle actual 7 must net to balance 7"
    );

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_id.clone()],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].group, run_id.as_str());
    assert_eq!(rows[0].amount.nanos(), 7);
    assert_eq!(rows[0].settlements, 1);
}

/// An unreserved settle of 4 contributes exactly 4 to the observed balance.
pub async fn unreserved_settle_contributes_actual(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("unreserved-settle-actual");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);
    let key = SettlementKey::new(RunId::new_v7(), 1, 1);

    let outcome = port
        .settle(settle_request(
            scope.clone(),
            key,
            4,
            currency.clone(),
            "gpt-4",
        ))
        .await
        .unwrap();
    assert_eq!(outcome, SettleOutcome::Settled);

    let balance = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance, 4);
}

/// `release` returns the hold to zero, is idempotent (a second `release` is `Ok(())` and
/// changes nothing), a `release` after a `settle` referencing the same reservation is also a
/// no-op, and releasing a never-issued [`ReservationId`] is `UnknownReservation`.
pub async fn release_returns_the_hold_and_is_idempotent(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("release-idempotent");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);

    let reservation = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(10, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start,
            window_end,
            key: None,
        })
        .await
        .unwrap();

    let balance_after_reserve =
        observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance_after_reserve, 10);

    port.release(reservation.clone()).await.unwrap();
    let balance_after_release =
        observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance_after_release, 0);

    // Idempotent: releasing again is a no-op, still `Ok`.
    port.release(reservation.clone()).await.unwrap();
    let balance_after_second_release =
        observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance_after_second_release, 0);

    // reserve 6, settle 6, release -> Ok, balance stays 6 (release after settle is a no-op).
    let key = SettlementKey::new(RunId::new_v7(), 1, 1);
    let reservation2 = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(6, currency.clone()),
            ceiling: Cost::new(20, currency.clone()),
            window_start,
            window_end,
            key: Some(key.clone()),
        })
        .await
        .unwrap();
    port.settle(SettleRequest {
        scope: scope.clone(),
        key,
        amount: Cost::new(6, currency.clone()),
        model_breakdown: BTreeMap::from([("gpt-4".to_string(), 6)]),
        reservation: Some(reservation2.clone()),
    })
    .await
    .unwrap();
    port.release(reservation2).await.unwrap();
    let balance_after_settled_release =
        observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(
        balance_after_settled_release, 6,
        "release after settle must be a no-op"
    );

    let unknown = ReservationId::new_v7();
    let err = port.release(unknown).await.unwrap_err();
    assert!(matches!(
        err,
        TreasuryLedgerError::UnknownReservation { .. }
    ));
}

/// `reserve` 10, `release`, then `settle` 3 referencing the (now closed) reservation: the
/// settle charges actual only (outstanding hold is 0 once closed), so the observed balance is
/// 3.
pub async fn settle_after_release_charges_actual_only(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("settle-after-release");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);
    let key = SettlementKey::new(RunId::new_v7(), 1, 1);

    let reservation = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(10, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start,
            window_end,
            key: Some(key.clone()),
        })
        .await
        .unwrap();

    port.release(reservation.clone()).await.unwrap();

    let outcome = port
        .settle(SettleRequest {
            scope: scope.clone(),
            key,
            amount: Cost::new(3, currency.clone()),
            model_breakdown: BTreeMap::from([("gpt-4".to_string(), 3)]),
            reservation: Some(reservation),
        })
        .await
        .unwrap();
    assert_eq!(outcome, SettleOutcome::Settled);

    let balance = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(
        balance, 3,
        "settle after release must charge actual only (reservation already closed)"
    );
}

/// A reservation's window `W1 = [t0-1h, t0+300ms)`; the reservation admits inside `W1`; after a
/// 500 ms sleep (well past `W1`'s end), a settle referencing it is still attributed to `W1` --
/// `W1`'s observed balance is 7 (10 reserved, settled at actual 7) and `W2 = [t0+300ms, t0+2h)`
/// sees no contribution at all.
pub async fn settle_is_attributed_to_its_reservation_window(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("settle-window-attribution");
    let currency = usd();
    let t0 = port.store_now().await.unwrap();
    let w1_start = t0 - chrono::Duration::hours(1);
    let w1_end = t0 + chrono::Duration::milliseconds(300);
    let w2_start = w1_end;
    let w2_end = t0 + chrono::Duration::hours(2);

    let key = SettlementKey::new(RunId::new_v7(), 1, 1);
    let reservation = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(10, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start: w1_start,
            window_end: w1_end,
            key: Some(key.clone()),
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    port.settle(SettleRequest {
        scope: scope.clone(),
        key,
        amount: Cost::new(7, currency.clone()),
        model_breakdown: BTreeMap::from([("gpt-4".to_string(), 7)]),
        reservation: Some(reservation),
    })
    .await
    .unwrap();

    let w1_balance = observed_balance(port, &scope, w1_start, w1_end, &currency).await;
    let w2_balance = observed_balance(port, &scope, w2_start, w2_end, &currency).await;
    assert_eq!(
        w1_balance, 7,
        "a settle after its reservation window closes must still attribute to W1"
    );
    assert_eq!(
        w2_balance, 0,
        "W2 (after the reservation window) must see no contribution"
    );
}

// ── Currency mismatch (ADR-0053 §2, D-00a) ────────────────────────────────

/// A reserve that finds a row of another currency in its scope and window fails with
/// `CurrencyMismatch`, as does a hold/ceiling currency mismatch and a settle whose currency
/// differs from its reservation's.
pub async fn reserve_refuses_a_second_currency_in_scope_and_window(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("currency-mismatch");
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);

    let eur_key = SettlementKey::new(RunId::new_v7(), 1, 1);
    port.settle(settle_request(scope.clone(), eur_key, 5, eur(), "gpt-4"))
        .await
        .unwrap();

    let mismatch = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(1, usd()),
            ceiling: Cost::new(100, usd()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(
        matches!(mismatch, Err(TreasuryLedgerError::CurrencyMismatch { .. })),
        "a foreign currency already in scope+window must refuse, got {mismatch:?}"
    );

    let hold_ceiling_scope = contract_scope("currency-mismatch-hold-ceiling");
    let hold_ceiling = port
        .reserve(ReserveRequest {
            scope: hold_ceiling_scope,
            hold: Cost::new(1, usd()),
            ceiling: Cost::new(100, eur()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(
        matches!(
            hold_ceiling,
            Err(TreasuryLedgerError::CurrencyMismatch { .. })
        ),
        "hold/ceiling currency mismatch must refuse, got {hold_ceiling:?}"
    );

    let reservation_scope = contract_scope("currency-mismatch-reservation");
    let reservation_key = SettlementKey::new(RunId::new_v7(), 1, 1);
    let reservation = port
        .reserve(ReserveRequest {
            scope: reservation_scope.clone(),
            hold: Cost::new(10, usd()),
            ceiling: Cost::new(10, usd()),
            window_start,
            window_end,
            key: Some(reservation_key.clone()),
        })
        .await
        .unwrap();
    let settle_mismatch = port
        .settle(SettleRequest {
            scope: reservation_scope,
            key: reservation_key,
            amount: Cost::new(5, eur()),
            model_breakdown: BTreeMap::from([("gpt-4".to_string(), 5)]),
            reservation: Some(reservation),
        })
        .await;
    assert!(
        matches!(
            settle_mismatch,
            Err(TreasuryLedgerError::CurrencyMismatch { .. })
        ),
        "a settle currency differing from its reservation must refuse, got {settle_mismatch:?}"
    );
}

// ── Validation (InvalidRequest, no I/O on rejection) ──────────────────────

/// A negative hold, negative ceiling, `window_start >= window_end`, or an empty tenant id is
/// `InvalidRequest`, and none of them write anything.
pub async fn reserve_rejects_invalid_requests(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("reserve-invalid");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);

    let balance_before = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance_before, 0);

    let negative_hold = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(-1, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(matches!(
        negative_hold,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let negative_ceiling = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(1, currency.clone()),
            ceiling: Cost::new(-1, currency.clone()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(matches!(
        negative_ceiling,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let bad_window = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(1, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start: window_end,
            window_end: window_start,
            key: None,
        })
        .await;
    assert!(matches!(
        bad_window,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let empty_tenant = port
        .reserve(ReserveRequest {
            scope: LedgerScope::new("", "key-1"),
            hold: Cost::new(1, currency.clone()),
            ceiling: Cost::new(10, currency.clone()),
            window_start,
            window_end,
            key: None,
        })
        .await;
    assert!(matches!(
        empty_tenant,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let balance_after = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance_after, 0, "invalid requests must not write anything");
}

// ── Two reservations, one superstep attempt (D-06) ────────────────────────

/// Two reserves carrying the same `SettlementKey` are both admitted with distinct
/// [`ReservationId`]s (only `settle` rows are keyed, D-06), and the observed balance is the sum
/// of both holds.
pub async fn two_reservations_for_one_superstep_attempt_are_legal(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("two-reservations-one-attempt");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);
    let key = SettlementKey::new(RunId::new_v7(), 4, 1);

    let first = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(4, currency.clone()),
            ceiling: Cost::new(20, currency.clone()),
            window_start,
            window_end,
            key: Some(key.clone()),
        })
        .await
        .unwrap();
    let second = port
        .reserve(ReserveRequest {
            scope: scope.clone(),
            hold: Cost::new(1, currency.clone()),
            ceiling: Cost::new(20, currency.clone()),
            window_start,
            window_end,
            key: Some(key),
        })
        .await
        .unwrap();

    assert_ne!(
        first, second,
        "two reservations for one superstep attempt must have distinct ids"
    );

    let balance = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance, 5, "observed balance must be the sum of both holds");
}

// ── Settlement idempotency (LEDGR-03) ─────────────────────────────────────

/// Settling the same `SettlementKey` twice returns `Settled` then `AlreadySettled` with the
/// balance and spend view unchanged (the first amount is kept even though the duplicate carries
/// a different amount).
pub async fn duplicate_settle_is_already_settled_and_charges_once(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("duplicate-settle");
    let currency = usd();
    let now = port.store_now().await.unwrap();
    let window_start = now - chrono::Duration::hours(1);
    let window_end = now + chrono::Duration::hours(1);
    let run_id = RunId::new_v7();
    let key = SettlementKey::new(run_id.clone(), 1, 1);

    let first = port
        .settle(settle_request(
            scope.clone(),
            key.clone(),
            5,
            currency.clone(),
            "gpt-4",
        ))
        .await
        .unwrap();
    let second = port
        .settle(settle_request(
            scope.clone(),
            key,
            9,
            currency.clone(),
            "gpt-4",
        ))
        .await
        .unwrap();
    assert_eq!(first, SettleOutcome::Settled);
    assert_eq!(second, SettleOutcome::AlreadySettled);

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_id],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].amount.nanos(),
        5,
        "the first amount is kept, never the duplicate's"
    );
    assert_eq!(rows[0].settlements, 1);

    let balance = observed_balance(port, &scope, window_start, window_end, &currency).await;
    assert_eq!(balance, 5);
}

/// The same `(run_id, superstep)` at a bumped `attempt` is a second, distinct settlement.
pub async fn bumped_attempt_is_a_distinct_settlement(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("bumped-attempt");
    let currency = usd();
    let run_id = RunId::new_v7();

    let first = port
        .settle(settle_request(
            scope.clone(),
            SettlementKey::new(run_id.clone(), 3, 1),
            5,
            currency.clone(),
            "gpt-4",
        ))
        .await
        .unwrap();
    let second = port
        .settle(settle_request(
            scope,
            SettlementKey::new(run_id.clone(), 3, 2),
            6,
            currency.clone(),
            "gpt-4",
        ))
        .await
        .unwrap();
    assert_eq!(first, SettleOutcome::Settled);
    assert_eq!(second, SettleOutcome::Settled);

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_id],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].amount.nanos(), 11);
    assert_eq!(rows[0].settlements, 2);
}

/// Ten concurrent settles of one `SettlementKey` produce exactly one `Settled` and nine
/// `AlreadySettled` with `settlements == 1` in `spend`; five concurrent settles of five distinct
/// keys all appear.
pub async fn concurrent_duplicate_settles_charge_once(port: Arc<dyn TreasuryLedgerPort>) {
    let scope = contract_scope("concurrent-duplicate-settles");
    let currency = usd();
    let run_id = RunId::new_v7();
    let key = SettlementKey::new(run_id.clone(), 1, 1);

    let mut handles = Vec::new();
    for _ in 0..10 {
        let port = Arc::clone(&port);
        let request = settle_request(scope.clone(), key.clone(), 5, currency.clone(), "gpt-4");
        handles.push(tokio::spawn(async move { port.settle(request).await }));
    }

    let mut settled = 0;
    let mut already = 0;
    tokio::time::timeout(Duration::from_secs(10), async {
        for handle in handles {
            match handle
                .await
                .expect("settle task must not panic")
                .expect("settle must succeed")
            {
                SettleOutcome::Settled => settled += 1,
                SettleOutcome::AlreadySettled => already += 1,
            }
        }
    })
    .await
    .expect("10 concurrent settles must not hang");

    assert_eq!(
        settled, 1,
        "exactly one concurrent settle of one key must be Settled"
    );
    assert_eq!(already, 9);

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_id.clone()],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].settlements, 1);

    let mut distinct_handles = Vec::new();
    for i in 0..5u64 {
        let port = Arc::clone(&port);
        let request = settle_request(
            scope.clone(),
            SettlementKey::new(run_id.clone(), 2 + i, 1),
            1,
            currency.clone(),
            "gpt-4",
        );
        distinct_handles.push(tokio::spawn(async move { port.settle(request).await }));
    }
    let mut distinct_settled = 0;
    tokio::time::timeout(Duration::from_secs(10), async {
        for handle in distinct_handles {
            match handle
                .await
                .expect("settle task must not panic")
                .expect("settle must succeed")
            {
                SettleOutcome::Settled => distinct_settled += 1,
                SettleOutcome::AlreadySettled => panic!("distinct keys must never collide"),
            }
        }
    })
    .await
    .expect("5 concurrent settles must not hang");
    assert_eq!(distinct_settled, 5);

    let rows_after = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_id],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rows_after.len(), 1);
    assert_eq!(
        rows_after[0].settlements, 6,
        "1 (from the duplicate group) + 5 distinct settlements"
    );
}

// ── Spend grouping, ordering, windows (LEDGR-04) ──────────────────────────

/// Rows across 2 tenants, 2 API keys, 3 runs, and models `gpt-4` / `gpt-4o-mini` /
/// `claude-sonnet` (one row with a two-model breakdown): grouping by tenant, API key, run, and
/// model each returns the expected sums and settlement counts, and `tenant_id`/`api_key_id`/
/// `run_ids` filters narrow the result.
pub async fn spend_groups_by_every_dimension_over_a_window(port: &dyn TreasuryLedgerPort) {
    let currency = usd();
    let suffix = Uuid::new_v4();
    let tenant_a = format!("spend-groups-tenant-a-{suffix}");
    let tenant_b = format!("spend-groups-tenant-b-{suffix}");
    let key_a = format!("spend-groups-key-a-{suffix}");
    let key_b = format!("spend-groups-key-b-{suffix}");

    let scope_1 = LedgerScope::new(tenant_a.clone(), key_a.clone());
    let scope_2 = LedgerScope::new(tenant_a.clone(), key_b.clone());
    let scope_3 = LedgerScope::new(tenant_b.clone(), key_a.clone());

    let run_1 = RunId::new_v7();
    let run_2 = RunId::new_v7();
    let run_3 = RunId::new_v7();

    port.settle(settle_request(
        scope_1,
        SettlementKey::new(run_1.clone(), 1, 1),
        10,
        currency.clone(),
        "gpt-4",
    ))
    .await
    .unwrap();
    port.settle(settle_request(
        scope_2,
        SettlementKey::new(run_2.clone(), 1, 1),
        20,
        currency.clone(),
        "gpt-4o-mini",
    ))
    .await
    .unwrap();
    port.settle(SettleRequest {
        scope: scope_3,
        key: SettlementKey::new(run_3.clone(), 1, 1),
        amount: Cost::new(12, currency.clone()),
        model_breakdown: BTreeMap::from([
            ("gpt-4".to_string(), 5),
            ("claude-sonnet".to_string(), 7),
        ]),
        reservation: None,
    })
    .await
    .unwrap();

    let run_ids = vec![run_1.clone(), run_2.clone(), run_3.clone()];

    let by_tenant = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Tenant,
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    let tenant_a_row = by_tenant
        .iter()
        .find(|r| r.group == tenant_a)
        .expect("tenant A row present");
    assert_eq!(tenant_a_row.amount.nanos(), 30);
    assert_eq!(tenant_a_row.settlements, 2);
    let tenant_b_row = by_tenant
        .iter()
        .find(|r| r.group == tenant_b)
        .expect("tenant B row present");
    assert_eq!(tenant_b_row.amount.nanos(), 12);
    assert_eq!(tenant_b_row.settlements, 1);

    let by_api_key = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::ApiKey,
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    let key_a_row = by_api_key
        .iter()
        .find(|r| r.group == key_a)
        .expect("key A row present");
    assert_eq!(key_a_row.amount.nanos(), 22);
    assert_eq!(key_a_row.settlements, 2);
    let key_b_row = by_api_key
        .iter()
        .find(|r| r.group == key_b)
        .expect("key B row present");
    assert_eq!(key_b_row.amount.nanos(), 20);
    assert_eq!(key_b_row.settlements, 1);

    let by_run = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_run.len(), 3);
    assert_eq!(
        by_run
            .iter()
            .find(|r| r.group == run_1.as_str())
            .unwrap()
            .amount
            .nanos(),
        10
    );
    assert_eq!(
        by_run
            .iter()
            .find(|r| r.group == run_2.as_str())
            .unwrap()
            .amount
            .nanos(),
        20
    );
    assert_eq!(
        by_run
            .iter()
            .find(|r| r.group == run_3.as_str())
            .unwrap()
            .amount
            .nanos(),
        12
    );

    let by_model = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Model,
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    let gpt4_row = by_model.iter().find(|r| r.group == "gpt-4").unwrap();
    assert_eq!(gpt4_row.amount.nanos(), 15);
    assert_eq!(gpt4_row.settlements, 2);
    let mini_row = by_model.iter().find(|r| r.group == "gpt-4o-mini").unwrap();
    assert_eq!(mini_row.amount.nanos(), 20);
    assert_eq!(mini_row.settlements, 1);
    let claude_row = by_model
        .iter()
        .find(|r| r.group == "claude-sonnet")
        .unwrap();
    assert_eq!(claude_row.amount.nanos(), 7);
    assert_eq!(claude_row.settlements, 1);

    let tenant_filtered = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            tenant_id: Some(tenant_a.clone()),
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(tenant_filtered.len(), 2);
    assert!(
        tenant_filtered
            .iter()
            .all(|r| r.group == run_1.as_str() || r.group == run_2.as_str())
    );

    let api_key_filtered = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            api_key_id: Some(key_a.clone()),
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(api_key_filtered.len(), 2);
    assert!(
        api_key_filtered
            .iter()
            .all(|r| r.group == run_1.as_str() || r.group == run_3.as_str())
    );

    let run_filtered = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_1.clone(), run_3.clone()],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(run_filtered.len(), 2);
    assert!(
        run_filtered
            .iter()
            .all(|r| r.group == run_1.as_str() || r.group == run_3.as_str())
    );
}

/// Groups `b`, `a`, `c` and currencies USD/EUR inserted in shuffled order come back ordered by
/// group value ascending, then currency code ascending: `(a EUR, a USD, b USD, c USD)`.
pub async fn spend_orders_groups_then_currencies_ascending(port: &dyn TreasuryLedgerPort) {
    let suffix = Uuid::new_v4();
    let tenant_a = format!("spend-order-a-{suffix}");
    let tenant_b = format!("spend-order-b-{suffix}");
    let tenant_c = format!("spend-order-c-{suffix}");

    let run_ids: Vec<RunId> = (0..4).map(|_| RunId::new_v7()).collect();

    // Deliberately shuffled insertion order.
    port.settle(settle_request(
        LedgerScope::new(tenant_b.clone(), "key"),
        SettlementKey::new(run_ids[0].clone(), 1, 1),
        1,
        usd(),
        "gpt-4",
    ))
    .await
    .unwrap();
    port.settle(settle_request(
        LedgerScope::new(tenant_a.clone(), "key"),
        SettlementKey::new(run_ids[1].clone(), 1, 1),
        3,
        eur(),
        "gpt-4",
    ))
    .await
    .unwrap();
    port.settle(settle_request(
        LedgerScope::new(tenant_c.clone(), "key"),
        SettlementKey::new(run_ids[2].clone(), 1, 1),
        4,
        usd(),
        "gpt-4",
    ))
    .await
    .unwrap();
    port.settle(settle_request(
        LedgerScope::new(tenant_a.clone(), "key"),
        SettlementKey::new(run_ids[3].clone(), 1, 1),
        2,
        usd(),
        "gpt-4",
    ))
    .await
    .unwrap();

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Tenant,
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();

    let groups_and_currencies: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.group.clone(), r.amount.currency().as_str().to_string()))
        .collect();
    assert_eq!(
        groups_and_currencies,
        vec![
            (tenant_a.clone(), "EUR".to_string()),
            (tenant_a, "USD".to_string()),
            (tenant_b, "USD".to_string()),
            (tenant_c, "USD".to_string()),
        ],
        "spend rows must be ordered by group value ascending, then currency code ascending"
    );
}

/// A window in 2020 (long before any test fixture) returns `Ok(vec![])`, never an error.
pub async fn spend_over_an_empty_window_is_empty(port: &dyn TreasuryLedgerPort) {
    let since = "2020-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let until = "2020-01-02T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Tenant,
            since: Some(since),
            until: Some(until),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        rows.is_empty(),
        "a window in 2020 must return no rows, got {rows:?}"
    );
}

/// `spend`'s window is half-open: settle A; sleep; `since = store_now()`; sleep; settle B;
/// sleep; `until = store_now()`; sleep; settle C. `spend(since, until)` contains only B;
/// `spend(since, None)` contains B and C; `spend(None, until)` contains A and B.
pub async fn spend_window_is_half_open(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("spend-half-open-window");
    let currency = usd();
    let run_a = RunId::new_v7();
    let run_b = RunId::new_v7();
    let run_c = RunId::new_v7();

    port.settle(settle_request(
        scope.clone(),
        SettlementKey::new(run_a.clone(), 1, 1),
        1,
        currency.clone(),
        "gpt-4",
    ))
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let since = port.store_now().await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    port.settle(settle_request(
        scope.clone(),
        SettlementKey::new(run_b.clone(), 1, 1),
        1,
        currency.clone(),
        "gpt-4",
    ))
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let until = port.store_now().await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    port.settle(settle_request(
        scope,
        SettlementKey::new(run_c.clone(), 1, 1),
        1,
        currency,
        "gpt-4",
    ))
    .await
    .unwrap();

    let run_ids = vec![run_a.clone(), run_b.clone(), run_c.clone()];

    let bounded = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            since: Some(since),
            until: Some(until),
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    let bounded_groups: Vec<&str> = bounded.iter().map(|r| r.group.as_str()).collect();
    assert_eq!(
        bounded_groups,
        vec![run_b.as_str()],
        "the half-open window must contain only B"
    );

    let since_only_rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            since: Some(since),
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut since_only_groups: Vec<&str> =
        since_only_rows.iter().map(|r| r.group.as_str()).collect();
    since_only_groups.sort_unstable();
    let mut expected_since = vec![run_b.as_str(), run_c.as_str()];
    expected_since.sort_unstable();
    assert_eq!(since_only_groups, expected_since);

    let until_only_rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            until: Some(until),
            run_ids: run_ids.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut until_only_groups: Vec<&str> =
        until_only_rows.iter().map(|r| r.group.as_str()).collect();
    until_only_groups.sort_unstable();
    let mut expected_until = vec![run_a.as_str(), run_b.as_str()];
    expected_until.sort_unstable();
    assert_eq!(until_only_groups, expected_until);
}

/// One USD and one EUR unreserved settle for a single run yield two `SpendRow`s, never a
/// combined figure.
pub async fn spend_splits_currencies_into_separate_rows(port: &dyn TreasuryLedgerPort) {
    let scope = contract_scope("spend-split-currencies");
    let run_id = RunId::new_v7();

    port.settle(settle_request(
        scope.clone(),
        SettlementKey::new(run_id.clone(), 1, 1),
        5,
        usd(),
        "gpt-4",
    ))
    .await
    .unwrap();
    port.settle(settle_request(
        scope,
        SettlementKey::new(run_id.clone(), 2, 1),
        3,
        eur(),
        "gpt-4",
    ))
    .await
    .unwrap();

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Run,
            run_ids: vec![run_id],
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(
        rows.len(),
        2,
        "one run with two currencies must yield two rows, never combined"
    );
    let usd_row = rows
        .iter()
        .find(|r| r.amount.currency().as_str() == "USD")
        .unwrap();
    assert_eq!(usd_row.amount.nanos(), 5);
    let eur_row = rows
        .iter()
        .find(|r| r.amount.currency().as_str() == "EUR")
        .unwrap();
    assert_eq!(eur_row.amount.nanos(), 3);
}

// ── Settle validation and store clock ─────────────────────────────────────

/// A breakdown that does not sum to the amount, a negative amount, or an empty breakdown with a
/// non-zero amount is `InvalidRequest`; a zero amount with an empty breakdown is `Settled`.
pub async fn settle_rejects_a_breakdown_that_does_not_sum_to_the_amount(
    port: &dyn TreasuryLedgerPort,
) {
    let scope = contract_scope("settle-validation");
    let currency = usd();

    let mismatched = port
        .settle(SettleRequest {
            scope: scope.clone(),
            key: SettlementKey::new(RunId::new_v7(), 1, 1),
            amount: Cost::new(10, currency.clone()),
            model_breakdown: BTreeMap::from([("gpt-4".to_string(), 1)]),
            reservation: None,
        })
        .await;
    assert!(matches!(
        mismatched,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let negative = port
        .settle(SettleRequest {
            scope: scope.clone(),
            key: SettlementKey::new(RunId::new_v7(), 1, 1),
            amount: Cost::new(-1, currency.clone()),
            model_breakdown: BTreeMap::new(),
            reservation: None,
        })
        .await;
    assert!(matches!(
        negative,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let empty_breakdown_nonzero = port
        .settle(SettleRequest {
            scope: scope.clone(),
            key: SettlementKey::new(RunId::new_v7(), 1, 1),
            amount: Cost::new(5, currency.clone()),
            model_breakdown: BTreeMap::new(),
            reservation: None,
        })
        .await;
    assert!(matches!(
        empty_breakdown_nonzero,
        Err(TreasuryLedgerError::InvalidRequest { .. })
    ));

    let zero_amount = port
        .settle(SettleRequest {
            scope,
            key: SettlementKey::new(RunId::new_v7(), 1, 1),
            amount: Cost::new(0, currency),
            model_breakdown: BTreeMap::new(),
            reservation: None,
        })
        .await
        .unwrap();
    assert_eq!(
        zero_amount,
        SettleOutcome::Settled,
        "a zero amount with an empty breakdown must be accepted"
    );
}

/// Two successive `store_now` calls: the second is `>=` the first.
pub async fn store_now_is_non_decreasing(port: &dyn TreasuryLedgerPort) {
    let first = port.store_now().await.unwrap();
    let second = port.store_now().await.unwrap();
    assert!(
        second >= first,
        "store_now must be non-decreasing within a test"
    );
}

/// A settle with [`LedgerScope::unattributed`] for a unique run is grouped under the literal
/// `"unattributed"` sentinel when queried by tenant.
pub async fn unattributed_scope_is_grouped_under_the_sentinel(port: &dyn TreasuryLedgerPort) {
    let run_id = RunId::new_v7();
    port.settle(settle_request(
        LedgerScope::unattributed(),
        SettlementKey::new(run_id.clone(), 1, 1),
        5,
        usd(),
        "gpt-4",
    ))
    .await
    .unwrap();

    let rows = port
        .spend(SpendQuery {
            group_by: SpendGroupBy::Tenant,
            run_ids: vec![run_id],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].group, LedgerScope::UNATTRIBUTED);
}
