//! The assertion library (OBS-04, D-29, PRD 07 §3 acceptance 5, OBS-FR-12).
//!
//! Twelve evaluators (this plan lands the first, `node_executed`; the
//! remaining eleven follow in the next task of this same plan) over exactly
//! three inputs -- the captured [`TraceRecord`] stream, the final
//! [`Battlefield`], and the [`RunOutcome`] -- proving the twelve-variant
//! `TraceEvent` is sufficient for the program's own acceptance bar without
//! reaching into engine internals (D-29). [`AssertionContext`] is the sole
//! handle any evaluator receives. A failing evaluation carries an
//! [`AssertionFailure`] whose [`AssertionFailure::render_failure`] produces
//! the four-part actionable message (expected, observed, the `seq` range
//! the evidence came from, and further detail) this crate's `insta`
//! snapshot suite freezes.
//!
//! # Threats mitigated here (28-08-PLAN.md `<threat_model>`)
//!
//! - **T-28-08-02** ([`CustomAssertion`] reachable from a scenario file):
//!   [`CustomAssertion`] carries no `serde` derive and has no variant in the
//!   file-format `Assertion` enum (`crates/paladin-eval/src/scenario.rs`) --
//!   it is a Rust-API-only escape hatch that can never be deserialized.
//! - **T-28-08-03** (rendered failures echoing observed state into CI logs):
//!   accepted -- a failure must show observed values to be actionable, and
//!   scenarios are scripted fixtures, never production data.

use std::collections::BTreeMap;
use std::sync::Arc;

use paladin_battalion::engine::RunOutcome;
use paladin_core::platform::container::battlefield::Battlefield;
use paladin_core::platform::container::trace::{TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::NodeOutcomeKind;

use crate::scenario::Times;

/// The evaluation context every assertion evaluator receives, and the ONLY
/// thing it receives (D-29): the captured [`TraceRecord`] stream, the final
/// [`Battlefield`], and the [`RunOutcome`] the run produced. No evaluator
/// reaches into engine internals beyond these three.
pub struct AssertionContext<'a> {
    records: &'a [TraceRecord],
    battlefield: &'a Battlefield,
    outcome: &'a RunOutcome,
    /// `NodeStarted` records grouped by node id, in the order they occurred.
    node_starts: BTreeMap<String, Vec<&'a TraceRecord>>,
    /// The full `NodeStarted` node-id sequence, in order (`route_taken`'s
    /// evidence, wired up by the next task).
    route: Vec<String>,
}

impl<'a> AssertionContext<'a> {
    /// Build a new evaluation context from the three inputs D-29 permits: the
    /// captured trace record stream (causal, `seq`-ordered), the final
    /// `Battlefield`, and the `RunOutcome` the run produced.
    pub fn new(records: &'a [TraceRecord], battlefield: &'a Battlefield, outcome: &'a RunOutcome) -> Self {
        let mut node_starts: BTreeMap<String, Vec<&'a TraceRecord>> = BTreeMap::new();
        let mut route = Vec::new();
        for record in records {
            if let TraceEvent::NodeStarted { node_id, .. } = &record.event {
                let key = node_id.as_str().to_string();
                node_starts.entry(key.clone()).or_default().push(record);
                route.push(key);
            }
        }
        Self {
            records,
            battlefield,
            outcome,
            node_starts,
            route,
        }
    }

    /// The `RunOutcome` this context was built from.
    pub fn outcome(&self) -> &'a RunOutcome {
        self.outcome
    }

    /// The final `Battlefield` this context was built from.
    pub fn battlefield(&self) -> &'a Battlefield {
        self.battlefield
    }

    /// The captured `TraceRecord` stream this context was built from.
    pub fn records(&self) -> &'a [TraceRecord] {
        self.records
    }

    /// The `NodeStarted` node-id sequence, in order.
    pub fn route(&self) -> &[String] {
        &self.route
    }

    fn node_starts(&self, node: &str) -> &[&'a TraceRecord] {
        self.node_starts.get(node).map(Vec::as_slice).unwrap_or(&[])
    }

    fn finished_outcome(&self, node: &str, superstep: u64, attempt: u32) -> Option<&'a NodeOutcomeKind> {
        self.records.iter().find_map(|r| match &r.event {
            TraceEvent::NodeFinished {
                node_id,
                superstep: s,
                attempt: a,
                outcome,
                ..
            } if node_id.as_str() == node && *s == superstep && *a == attempt => Some(outcome),
            _ => None,
        })
    }
}

/// The result of evaluating one assertion against an [`AssertionContext`].
///
/// A passing evaluation carries no rendered text -- failure rendering is
/// computed only on [`AssertionOutcome::Failed`], never on the happy path.
#[derive(Debug)]
pub enum AssertionOutcome {
    /// The assertion held.
    Passed,
    /// The assertion did not hold; carries the actionable failure.
    Failed(AssertionFailure),
}

/// An actionable assertion failure (D-29, PRD 07 §3 acceptance 5): what was
/// expected, what was observed, the `seq` range the evidence came from (when
/// the evidence is trace records), and further detail (a visit table, the
/// observed route, or a diff) when one helps a reader fix the scenario
/// without opening the trace themselves.
#[derive(Debug, Clone)]
pub struct AssertionFailure {
    /// A rendering of the assertion that failed, including its parameters.
    pub assertion: String,
    /// What the assertion expected.
    pub expected: String,
    /// What was actually observed.
    pub observed: String,
    /// The `seq` range (inclusive) the evidence for this failure spans,
    /// when the evidence comes from trace records.
    pub seq_range: Option<(u64, u64)>,
    /// Further detail -- a visit table, the observed route, or a diff --
    /// when one makes the failure actionable without opening the trace.
    pub detail: Option<String>,
}

impl AssertionFailure {
    /// Render this failure as the four-part actionable message this crate's
    /// assertions promise (PRD 07 §3 acceptance 5, D-29): the assertion
    /// itself, the expected and observed values, the `seq` range the
    /// evidence came from, and further detail when one is available. This
    /// exact wording is frozen by `insta` snapshots in
    /// `tests/assertion_snapshots.rs` -- it is part of the contract, not an
    /// accident of the implementation.
    pub fn render_failure(&self) -> String {
        let mut out = format!(
            "assertion failed: {}\n  expected: {}\n  observed: {}\n",
            self.assertion, self.expected, self.observed
        );
        if let Some((lo, hi)) = self.seq_range {
            out.push_str(&format!("  seq: {lo}..={hi}\n"));
        }
        if let Some(detail) = &self.detail {
            out.push_str("  detail:\n");
            for line in detail.lines() {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }
}

/// A Rust-API-only assertion hook: an arbitrary closure over an
/// [`AssertionContext`]. `CustomAssertion` carries no `serde` derive and has
/// no variant in the file-format `Assertion` enum
/// (`crates/paladin-eval/src/scenario.rs`), so it can never be deserialized
/// from a scenario file (T-28-08-02) -- it exists only for a host test
/// binary composing assertions directly in Rust.
pub type CustomAssertion = Arc<dyn for<'a> Fn(&AssertionContext<'a>) -> AssertionOutcome + Send + Sync>;

// ---------------------------------------------------------------------------
// node_executed
// ---------------------------------------------------------------------------

fn node_executed(ctx: &AssertionContext<'_>, node: &str, times: Times) -> AssertionOutcome {
    let starts = ctx.node_starts(node).to_vec();
    let count = starts.len() as u32;
    let (passed, expected) = match times {
        Times::Exact(n) => (count == n, format!("exactly {n}")),
        Times::Min(n) => (count >= n, format!("at least {n}")),
        Times::Max(n) => (count <= n, format!("at most {n}")),
    };
    if passed {
        return AssertionOutcome::Passed;
    }
    AssertionOutcome::Failed(AssertionFailure {
        assertion: format!("node_executed {{ node: {node:?}, times: {times:?} }}"),
        expected,
        observed: format!("{count} execution(s)"),
        seq_range: seq_range_of(&starts),
        detail: Some(render_visit_table(ctx, node, &starts)),
    })
}

fn seq_range_of(records: &[&TraceRecord]) -> Option<(u64, u64)> {
    let mut iter = records.iter();
    let first = iter.next()?;
    let (mut lo, mut hi) = (first.seq, first.seq);
    for r in iter {
        lo = lo.min(r.seq);
        hi = hi.max(r.seq);
    }
    Some((lo, hi))
}

fn render_visit_table(ctx: &AssertionContext<'_>, node: &str, starts: &[&TraceRecord]) -> String {
    let mut lines = vec!["visits (seq / superstep / attempt / outcome):".to_string()];
    for r in starts {
        if let TraceEvent::NodeStarted { superstep, attempt, .. } = &r.event {
            let outcome = ctx
                .finished_outcome(node, *superstep, *attempt)
                .map(|o| format!("{o:?}"))
                .unwrap_or_else(|| "no matching NodeFinished".to_string());
            lines.push(format!(
                "  seq={} superstep={superstep} attempt={attempt} outcome={outcome}",
                r.seq
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paladin_core::platform::container::battlefield::BattlefieldSchema;
    use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointId};

    fn thread() -> ThreadId {
        ThreadId::new("assertion-tests").expect("valid thread id")
    }

    fn rec(seq: u64, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id: thread(),
            run_id: None,
            seq,
            at: Utc::now(),
            event,
        }
    }

    fn node_started(seq: u64, node: &str, superstep: u64, attempt: u32) -> TraceRecord {
        rec(
            seq,
            TraceEvent::NodeStarted {
                superstep,
                node_id: NodeId::new(node),
                attempt,
                muster_task_key: None,
            },
        )
    }

    fn node_finished(seq: u64, node: &str, superstep: u64, attempt: u32, outcome: NodeOutcomeKind) -> TraceRecord {
        rec(
            seq,
            TraceEvent::NodeFinished {
                superstep,
                node_id: NodeId::new(node),
                attempt,
                outcome,
                duration_ms: 1,
                token_count: 1,
                cache_hit: false,
            },
        )
    }

    fn empty_battlefield() -> Battlefield {
        Battlefield::new(BattlefieldSchema::new(vec![]))
    }

    fn outcome_for(bf: &Battlefield) -> RunOutcome {
        RunOutcome::Completed {
            final_state: bf.clone(),
            waypoint: WaypointId::generate(),
        }
    }

    fn assert_passed(outcome: AssertionOutcome) {
        assert!(matches!(outcome, AssertionOutcome::Passed), "expected Passed, got {outcome:?}");
    }

    fn assert_failed(outcome: AssertionOutcome) -> AssertionFailure {
        match outcome {
            AssertionOutcome::Failed(f) => f,
            AssertionOutcome::Passed => panic!("expected Failed, got Passed"),
        }
    }

    #[test]
    fn node_executed_exact_passes_and_fails() {
        let records = vec![
            node_started(1, "worker", 1, 1),
            node_started(2, "worker", 2, 1),
            node_started(3, "worker", 3, 1),
        ];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(node_executed(&ctx, "worker", Times::Exact(3)));

        let failure = assert_failed(node_executed(&ctx, "worker", Times::Exact(2)));
        assert!(failure.render_failure().contains("3 execution"));
    }

    #[test]
    fn node_executed_counts_attempts_not_nodes() {
        let records = vec![
            node_started(1, "worker", 1, 1),
            node_started(2, "worker", 1, 2),
            node_started(3, "worker", 1, 3),
        ];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(node_executed(&ctx, "worker", Times::Exact(3)));
    }

    #[test]
    fn node_executed_min_and_max() {
        let records = vec![
            node_started(1, "worker", 1, 1),
            node_started(2, "worker", 1, 2),
            node_started(3, "worker", 1, 3),
        ];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(node_executed(&ctx, "worker", Times::Min(2)));
        assert_failed(node_executed(&ctx, "worker", Times::Max(2)));
    }

    #[test]
    fn node_executed_failure_renders_a_visit_table() {
        let records = vec![
            node_started(1, "worker", 1, 1),
            node_finished(2, "worker", 1, 1, NodeOutcomeKind::Succeeded),
        ];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(node_executed(&ctx, "worker", Times::Exact(5)));
        let rendered = failure.render_failure();
        assert!(rendered.contains("superstep=1"));
        assert!(rendered.contains("attempt=1"));
        assert!(rendered.contains("Succeeded"));
        assert!(rendered.contains("seq: 1..=1"));
    }

    #[test]
    fn passing_assertion_renders_nothing() {
        let records = vec![node_started(1, "worker", 1, 1)];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let result = node_executed(&ctx, "worker", Times::Exact(1));
        assert!(matches!(result, AssertionOutcome::Passed));
    }

    #[test]
    fn assertion_context_uses_only_records_battlefield_and_outcome() {
        let records = vec![node_started(1, "worker", 1, 1)];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_eq!(ctx.records().len(), 1);
        assert!(matches!(ctx.outcome(), RunOutcome::Completed { .. }));
        assert_eq!(ctx.battlefield().schema().fields.len(), 0);
        assert_eq!(ctx.route(), &["worker".to_string()]);
    }
}
