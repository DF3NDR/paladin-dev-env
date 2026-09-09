//! The assertion library (OBS-04, D-29, PRD 07 §3 acceptance 5, OBS-FR-12).
//!
//! Twelve evaluators over exactly three inputs -- the captured
//! [`TraceRecord`] stream, the final [`Battlefield`], and the [`RunOutcome`]
//! -- proving the twelve-variant [`TraceEvent`] is sufficient for the
//! program's own acceptance bar without reaching into engine internals
//! (D-29). [`AssertionContext`] is the sole handle any evaluator receives;
//! [`evaluate`] dispatches [`Assertion`] over it with no wildcard arm, so a
//! future assertion kind is a compile error here, never a silently-ignored
//! no-op (T-28-08-04). A failing evaluation carries an [`AssertionFailure`]
//! whose [`AssertionFailure::render_failure`] produces the four-part
//! actionable message (expected, observed, the `seq` range the evidence
//! came from, and further detail) this crate's `insta` snapshot suite
//! freezes.
//!
//! # Threats mitigated here (28-08-PLAN.md `<threat_model>`)
//!
//! - **T-28-08-01** (a pathological regex in `final_state_field_matches`):
//!   regexes are compiled through the `regex` crate (linear-time by
//!   construction, no backtracking); a compile failure is a typed
//!   [`AssertionOutcome::Failed`], never a panic.
//! - **T-28-08-02** ([`CustomAssertion`] reachable from a scenario file):
//!   [`CustomAssertion`] carries no `serde` derive and has no variant in the
//!   file-format [`Assertion`] enum (`crates/paladin-eval/src/scenario.rs`)
//!   -- it is a Rust-API-only escape hatch that can never be deserialized.
//! - **T-28-08-03** (rendered failures echoing observed state into CI logs):
//!   accepted -- a failure must show observed values to be actionable, and
//!   scenarios are scripted fixtures, never production data.
//! - **T-28-08-04** (a silently-passing assertion whose evidence never
//!   existed): every evaluator that depends on a `RunFinished` record fails
//!   explicitly when the stream has none; [`evaluate`] has no wildcard arm;
//!   an absent `final_state_snapshot` file is a failure naming `--bless`,
//!   never an implicit pass.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use regex::Regex;
use serde_json::Value;

use paladin_battalion::engine::RunOutcome;
use paladin_core::platform::container::battlefield::{Battlefield, FieldName};
use paladin_core::platform::container::parley::ParleyKind;
use paladin_core::platform::container::trace::{RunFinishStatus, TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::NodeOutcomeKind;

use crate::scenario::{Assertion, RunStatusValue, Times};

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
    /// evidence).
    route: Vec<String>,
    /// The blessed snapshot file `final_state_snapshot` compares against,
    /// when one is configured (see [`AssertionContext::with_snapshot_path`]).
    snapshot_path: Option<PathBuf>,
}

impl<'a> AssertionContext<'a> {
    /// Build a new evaluation context from the three inputs D-29 permits: the
    /// captured trace record stream (causal, `seq`-ordered), the final
    /// `Battlefield`, and the `RunOutcome` the run produced.
    pub fn new(
        records: &'a [TraceRecord],
        battlefield: &'a Battlefield,
        outcome: &'a RunOutcome,
    ) -> Self {
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
            snapshot_path: None,
        }
    }

    /// Configure the blessed snapshot file [`Assertion::FinalStateSnapshot`]
    /// compares the serialized final `Battlefield` against (D-33). Left
    /// unset, `final_state_snapshot` fails naming `--bless`, rather than
    /// passing implicitly (T-28-08-04).
    pub fn with_snapshot_path(mut self, path: PathBuf) -> Self {
        self.snapshot_path = Some(path);
        self
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

    fn finished_outcome(
        &self,
        node: &str,
        superstep: u64,
        attempt: u32,
    ) -> Option<&'a NodeOutcomeKind> {
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

    fn run_finished(&self) -> Option<RunFinishedInfo<'a>> {
        self.records.iter().find_map(|r| match &r.event {
            TraceEvent::RunFinished {
                status,
                total_supersteps,
                total_tokens,
                ..
            } => Some(RunFinishedInfo {
                seq: r.seq,
                status,
                total_tokens: *total_tokens,
                total_supersteps: *total_supersteps,
            }),
            _ => None,
        })
    }
}

/// The subset of a `RunFinished` record the token/superstep/status
/// evaluators need, extracted once so `run_status`/`total_tokens_max`/
/// `supersteps_max` never re-destructure the same match arm.
struct RunFinishedInfo<'a> {
    seq: u64,
    status: &'a RunFinishStatus,
    total_tokens: u64,
    total_supersteps: u64,
}

/// The result of evaluating one [`Assertion`] against an [`AssertionContext`].
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
/// no variant in the file-format [`Assertion`] enum
/// (`crates/paladin-eval/src/scenario.rs`), so it can never be deserialized
/// from a scenario file (T-28-08-02) -- it exists only for a host test
/// binary composing assertions directly in Rust.
pub type CustomAssertion =
    Arc<dyn for<'a> Fn(&AssertionContext<'a>) -> AssertionOutcome + Send + Sync>;

/// Evaluate one [`Assertion`] against `ctx`.
///
/// Dispatches over the closed [`Assertion`] enum with no wildcard arm: a
/// future assertion kind added to the file format is a compile error here,
/// never a silently-ignored no-op (T-28-08-04).
pub fn evaluate(assertion: &Assertion, ctx: &AssertionContext<'_>) -> AssertionOutcome {
    match assertion {
        Assertion::FinalStateFieldEquals { field, value } => {
            final_state_field_equals(ctx, field, value)
        }
        Assertion::FinalStateFieldMatches { field, pattern } => {
            final_state_field_matches(ctx, field, pattern)
        }
        Assertion::FieldJsonPathEquals { path, value } => field_json_path_equals(ctx, path, value),
        Assertion::NodeExecuted { node, times } => node_executed(ctx, node, *times),
        Assertion::NodeNotExecuted { node } => node_not_executed(ctx, node),
        Assertion::EdgeFired { from, to } => edge_fired(ctx, from, to),
        Assertion::RouteTaken(nodes) => route_taken(ctx, nodes),
        Assertion::RunStatus(status) => run_status(ctx, *status),
        Assertion::TotalTokensMax(max) => total_tokens_max(ctx, *max),
        Assertion::SuperstepsMax(max) => supersteps_max(ctx, *max),
        Assertion::ParleyRaised { kind, node } => parley_raised(ctx, kind, node),
        Assertion::FinalStateSnapshot => final_state_snapshot(ctx),
    }
}

// ---------------------------------------------------------------------------
// node_executed / node_not_executed
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

fn node_not_executed(ctx: &AssertionContext<'_>, node: &str) -> AssertionOutcome {
    let starts = ctx.node_starts(node).to_vec();
    if starts.is_empty() {
        return AssertionOutcome::Passed;
    }
    AssertionOutcome::Failed(AssertionFailure {
        assertion: format!("node_not_executed {{ node: {node:?} }}"),
        expected: "zero executions".to_string(),
        observed: format!("{} execution(s)", starts.len()),
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
        if let TraceEvent::NodeStarted {
            superstep, attempt, ..
        } = &r.event
        {
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

// ---------------------------------------------------------------------------
// edge_fired / route_taken
// ---------------------------------------------------------------------------

fn edge_fired(ctx: &AssertionContext<'_>, from: &str, to: &str) -> AssertionOutcome {
    let assertion = format!("edge_fired {{ from: {from:?}, to: {to:?} }}");
    let mut evaluated_not_fired: Option<&TraceRecord> = None;
    for r in ctx.records {
        if let TraceEvent::EdgeEvaluated {
            from: f,
            to: t,
            fired,
            ..
        } = &r.event
            && f.as_str() == from
            && t.as_str() == to
        {
            if *fired {
                return AssertionOutcome::Passed;
            }
            evaluated_not_fired = Some(r);
        }
    }
    let (observed, seq_range) = match evaluated_not_fired {
        Some(r) => (
            format!("edge {from} -> {to} was evaluated but did not fire"),
            Some((r.seq, r.seq)),
        ),
        None => (format!("edge {from} -> {to} was never evaluated"), None),
    };
    AssertionOutcome::Failed(AssertionFailure {
        assertion,
        expected: format!("edge {from} -> {to} to fire at least once"),
        observed,
        seq_range,
        detail: None,
    })
}

fn route_taken(ctx: &AssertionContext<'_>, expected_route: &[String]) -> AssertionOutcome {
    let assertion = format!("route_taken {expected_route:?}");
    if is_subsequence(expected_route, &ctx.route) {
        return AssertionOutcome::Passed;
    }
    let all_starts: Vec<&TraceRecord> = ctx
        .records
        .iter()
        .filter(|r| matches!(r.event, TraceEvent::NodeStarted { .. }))
        .collect();
    AssertionOutcome::Failed(AssertionFailure {
        assertion,
        expected: format!("{expected_route:?} as a subsequence of the executed route"),
        observed: format!("{:?}", ctx.route),
        seq_range: seq_range_of(&all_starts),
        detail: None,
    })
}

fn is_subsequence(needle: &[String], haystack: &[String]) -> bool {
    let mut hay_iter = haystack.iter();
    needle.iter().all(|n| hay_iter.by_ref().any(|h| h == n))
}

// ---------------------------------------------------------------------------
// run_status / total_tokens_max / supersteps_max
// ---------------------------------------------------------------------------

fn run_status_matches(actual: &RunFinishStatus, expected: RunStatusValue) -> bool {
    matches!(
        (actual, expected),
        (RunFinishStatus::Completed, RunStatusValue::Completed)
            | (RunFinishStatus::Failed, RunStatusValue::Failed)
            | (RunFinishStatus::Halted, RunStatusValue::Halted)
            | (
                RunFinishStatus::AwaitingInput,
                RunStatusValue::AwaitingInput
            )
    )
}

fn run_status(ctx: &AssertionContext<'_>, expected: RunStatusValue) -> AssertionOutcome {
    let assertion = format!("run_status {expected:?}");
    match ctx.run_finished() {
        None => run_never_finished(assertion),
        Some(info) => {
            if run_status_matches(info.status, expected) {
                AssertionOutcome::Passed
            } else {
                AssertionOutcome::Failed(AssertionFailure {
                    assertion,
                    expected: format!("{expected:?}"),
                    observed: format!("{:?}", info.status),
                    seq_range: Some((info.seq, info.seq)),
                    detail: None,
                })
            }
        }
    }
}

fn total_tokens_max(ctx: &AssertionContext<'_>, max: u64) -> AssertionOutcome {
    let assertion = format!("total_tokens_max {max}");
    match ctx.run_finished() {
        None => run_never_finished(assertion),
        Some(info) => {
            if info.total_tokens <= max {
                AssertionOutcome::Passed
            } else {
                AssertionOutcome::Failed(AssertionFailure {
                    assertion,
                    expected: format!("total_tokens <= {max}"),
                    observed: format!("total_tokens = {}", info.total_tokens),
                    seq_range: Some((info.seq, info.seq)),
                    detail: None,
                })
            }
        }
    }
}

fn supersteps_max(ctx: &AssertionContext<'_>, max: u64) -> AssertionOutcome {
    let assertion = format!("supersteps_max {max}");
    match ctx.run_finished() {
        None => run_never_finished(assertion),
        Some(info) => {
            if info.total_supersteps <= max {
                AssertionOutcome::Passed
            } else {
                AssertionOutcome::Failed(AssertionFailure {
                    assertion,
                    expected: format!("total_supersteps <= {max}"),
                    observed: format!("total_supersteps = {}", info.total_supersteps),
                    seq_range: Some((info.seq, info.seq)),
                    detail: None,
                })
            }
        }
    }
}

fn run_never_finished(assertion: String) -> AssertionOutcome {
    AssertionOutcome::Failed(AssertionFailure {
        assertion,
        expected: "a RunFinished record".to_string(),
        observed: "the run never finished (no RunFinished record in this trace)".to_string(),
        seq_range: None,
        detail: None,
    })
}

// ---------------------------------------------------------------------------
// parley_raised
// ---------------------------------------------------------------------------

fn parley_kind_wire_name(kind: &ParleyKind) -> &'static str {
    match kind {
        ParleyKind::Approval => "approval",
        ParleyKind::Choice => "choice",
        ParleyKind::FreeText => "free_text",
        ParleyKind::StateEdit => "state_edit",
        _ => "unknown",
    }
}

fn parley_raised(ctx: &AssertionContext<'_>, kind: &str, node: &str) -> AssertionOutcome {
    let assertion = format!("parley_raised {{ kind: {kind:?}, node: {node:?} }}");
    for r in ctx.records {
        if let TraceEvent::ParleyRaised {
            node_id,
            parley_kind,
            ..
        } = &r.event
            && node_id.as_str() == node
            && parley_kind_wire_name(parley_kind) == kind
        {
            return AssertionOutcome::Passed;
        }
    }
    AssertionOutcome::Failed(AssertionFailure {
        assertion,
        expected: format!("a ParleyRaised record with kind={kind:?} node={node:?}"),
        observed: render_observed_parleys(ctx),
        seq_range: None,
        detail: None,
    })
}

fn render_observed_parleys(ctx: &AssertionContext<'_>) -> String {
    let mut lines = Vec::new();
    for r in ctx.records {
        if let TraceEvent::ParleyRaised {
            node_id,
            parley_kind,
            ..
        } = &r.event
        {
            lines.push(format!(
                "kind={} node={}",
                parley_kind_wire_name(parley_kind),
                node_id
            ));
        }
    }
    if lines.is_empty() {
        "no ParleyRaised records in this run".to_string()
    } else {
        lines.join(", ")
    }
}

// ---------------------------------------------------------------------------
// final_state_field_equals / final_state_field_matches / field_json_path_equals
// ---------------------------------------------------------------------------

fn known_fields(bf: &Battlefield) -> Vec<&str> {
    bf.schema().fields.iter().map(|f| f.name.as_str()).collect()
}

fn missing_field_failure(assertion: String, field: &str, bf: &Battlefield) -> AssertionOutcome {
    AssertionOutcome::Failed(AssertionFailure {
        assertion,
        expected: format!("field {field:?} to be present"),
        observed: format!(
            "field {field:?} is absent; known fields: {:?}",
            known_fields(bf)
        ),
        seq_range: None,
        detail: None,
    })
}

fn invalid_field_name_failure(assertion: String, field: &str) -> AssertionOutcome {
    AssertionOutcome::Failed(AssertionFailure {
        assertion,
        expected: "a non-empty field name".to_string(),
        observed: format!("field name {field:?} is invalid"),
        seq_range: None,
        detail: None,
    })
}

fn final_state_field_equals(
    ctx: &AssertionContext<'_>,
    field: &str,
    value: &Value,
) -> AssertionOutcome {
    let assertion = format!("final_state_field_equals {{ field: {field:?}, value: {value} }}");
    let field_name = match FieldName::new(field) {
        Ok(f) => f,
        Err(_) => return invalid_field_name_failure(assertion, field),
    };
    match ctx.battlefield.get_raw(&field_name) {
        None => missing_field_failure(assertion, field, ctx.battlefield),
        Some(actual) if actual == value => AssertionOutcome::Passed,
        Some(actual) => AssertionOutcome::Failed(AssertionFailure {
            assertion,
            expected: value.to_string(),
            observed: actual.to_string(),
            seq_range: None,
            detail: None,
        }),
    }
}

fn render_value_as_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn final_state_field_matches(
    ctx: &AssertionContext<'_>,
    field: &str,
    pattern: &str,
) -> AssertionOutcome {
    let assertion =
        format!("final_state_field_matches {{ field: {field:?}, pattern: {pattern:?} }}");
    let field_name = match FieldName::new(field) {
        Ok(f) => f,
        Err(_) => return invalid_field_name_failure(assertion, field),
    };
    let Some(actual) = ctx.battlefield.get_raw(&field_name) else {
        return missing_field_failure(assertion, field, ctx.battlefield);
    };
    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(err) => {
            return AssertionOutcome::Failed(AssertionFailure {
                assertion,
                expected: format!("a valid regex pattern ({pattern:?})"),
                observed: format!("pattern failed to compile: {err}"),
                seq_range: None,
                detail: None,
            });
        }
    };
    let rendered = render_value_as_string(actual);
    if regex.is_match(&rendered) {
        AssertionOutcome::Passed
    } else {
        AssertionOutcome::Failed(AssertionFailure {
            assertion,
            expected: format!("value matching /{pattern}/"),
            observed: rendered,
            seq_range: None,
            detail: None,
        })
    }
}

fn battlefield_to_value(bf: &Battlefield) -> Value {
    let mut obj = serde_json::Map::new();
    for field in &bf.schema().fields {
        if let Some(v) = bf.get_raw(&field.name) {
            obj.insert(field.name.as_str().to_string(), v.clone());
        }
    }
    Value::Object(obj)
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn field_json_path_equals(
    ctx: &AssertionContext<'_>,
    path: &str,
    value: &Value,
) -> AssertionOutcome {
    let assertion = format!("field_json_path_equals {{ path: {path:?}, value: {value} }}");
    let root = battlefield_to_value(ctx.battlefield);
    match root.pointer(path) {
        None => AssertionOutcome::Failed(AssertionFailure {
            assertion,
            expected: format!("{path:?} to resolve to {value}"),
            observed: format!("{path:?} does not resolve in the final state"),
            seq_range: None,
            detail: Some(format!("final state:\n{}", pretty(&root))),
        }),
        Some(actual) if actual == value => AssertionOutcome::Passed,
        Some(actual) => AssertionOutcome::Failed(AssertionFailure {
            assertion,
            expected: value.to_string(),
            observed: actual.to_string(),
            seq_range: None,
            detail: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// final_state_snapshot
// ---------------------------------------------------------------------------

fn first_differing_path(expected: &Value, actual: &Value, path: &str) -> Option<String> {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            let mut keys: Vec<&String> = e.keys().chain(a.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let child_path = format!("{path}/{k}");
                match (e.get(k), a.get(k)) {
                    (Some(ev), Some(av)) => {
                        if let Some(p) = first_differing_path(ev, av, &child_path) {
                            return Some(p);
                        }
                    }
                    _ => return Some(child_path),
                }
            }
            None
        }
        _ => {
            if expected == actual {
                None
            } else if path.is_empty() {
                Some("(root)".to_string())
            } else {
                Some(path.to_string())
            }
        }
    }
}

fn final_state_snapshot(ctx: &AssertionContext<'_>) -> AssertionOutcome {
    let current = battlefield_to_value(ctx.battlefield);
    let Some(path) = &ctx.snapshot_path else {
        return AssertionOutcome::Failed(AssertionFailure {
            assertion: "final_state_snapshot".to_string(),
            expected: "a blessed snapshot file (configure one, or run with --bless to create it)"
                .to_string(),
            observed: "no snapshot file is configured for this case".to_string(),
            seq_range: None,
            detail: None,
        });
    };
    if !path.exists() {
        return AssertionOutcome::Failed(AssertionFailure {
            assertion: "final_state_snapshot".to_string(),
            expected: "an existing blessed snapshot file".to_string(),
            observed: "the snapshot file does not exist yet".to_string(),
            seq_range: None,
            detail: Some("run with --bless to create it".to_string()),
        });
    }
    let blessed_text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            return AssertionOutcome::Failed(AssertionFailure {
                assertion: "final_state_snapshot".to_string(),
                expected: "a readable blessed snapshot file".to_string(),
                observed: format!("failed to read the snapshot file: {err}"),
                seq_range: None,
                detail: None,
            });
        }
    };
    let blessed: Value = match serde_json::from_str(&blessed_text) {
        Ok(v) => v,
        Err(err) => {
            return AssertionOutcome::Failed(AssertionFailure {
                assertion: "final_state_snapshot".to_string(),
                expected: "a valid JSON blessed snapshot file".to_string(),
                observed: format!("failed to parse the snapshot file: {err}"),
                seq_range: None,
                detail: None,
            });
        }
    };
    if blessed == current {
        return AssertionOutcome::Passed;
    }
    let diff_path =
        first_differing_path(&blessed, &current, "").unwrap_or_else(|| "(root)".to_string());
    AssertionOutcome::Failed(AssertionFailure {
        assertion: "final_state_snapshot".to_string(),
        expected: "the final state to match the blessed snapshot".to_string(),
        observed: format!("first differing path: {diff_path}"),
        seq_range: None,
        detail: Some("run with --bless to update the snapshot".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldSpec, StateDelta,
    };
    use paladin_core::platform::container::parley::ParleyId;
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

    fn node_finished(
        seq: u64,
        node: &str,
        superstep: u64,
        attempt: u32,
        outcome: NodeOutcomeKind,
    ) -> TraceRecord {
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
        assert!(
            matches!(outcome, AssertionOutcome::Passed),
            "expected Passed, got {outcome:?}"
        );
    }

    fn assert_failed(outcome: AssertionOutcome) -> AssertionFailure {
        match outcome {
            AssertionOutcome::Failed(f) => f,
            AssertionOutcome::Passed => panic!("expected Failed, got Passed"),
        }
    }

    // -- Task 1: AssertionContext + node_executed --------------------------

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

        assert_passed(evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Exact(3),
            },
            &ctx,
        ));

        let failure = assert_failed(evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Exact(2),
            },
            &ctx,
        ));
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

        assert_passed(evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Exact(3),
            },
            &ctx,
        ));
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

        assert_passed(evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Min(2),
            },
            &ctx,
        ));
        assert_failed(evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Max(2),
            },
            &ctx,
        ));
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

        let failure = assert_failed(evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Exact(5),
            },
            &ctx,
        ));
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

        let result = evaluate(
            &Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Exact(1),
            },
            &ctx,
        );
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

    // -- Task 2: the remaining eleven kinds ---------------------------------

    fn field_battlefield() -> Battlefield {
        let status = FieldName::new("status").expect("valid field name");
        let count = FieldName::new("count").expect("valid field name");
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(status.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(count.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut bf = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(status, "done").expect("sets status");
        delta.set(count, 3_i64).expect("sets count");
        bf.merge(
            vec![(NodeId::new("writer"), delta)],
            0,
            &CustomDispatchResolver::new(),
        )
        .expect("merge succeeds");
        bf
    }

    #[test]
    fn final_state_field_equals_passes() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::FinalStateFieldEquals {
                field: "status".to_string(),
                value: serde_json::json!("done"),
            },
            &ctx,
        ));
    }

    #[test]
    fn final_state_field_equals_fails() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(
            &Assertion::FinalStateFieldEquals {
                field: "status".to_string(),
                value: serde_json::json!("nope"),
            },
            &ctx,
        ));
        assert!(failure.render_failure().contains("done"));
    }

    #[test]
    fn final_state_field_equals_fails_on_missing_field() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let missing = assert_failed(evaluate(
            &Assertion::FinalStateFieldEquals {
                field: "missing".to_string(),
                value: serde_json::json!("x"),
            },
            &ctx,
        ));
        assert!(missing.render_failure().contains("known fields"));
    }

    #[test]
    fn final_state_field_matches_passes() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::FinalStateFieldMatches {
                field: "status".to_string(),
                pattern: "^do".to_string(),
            },
            &ctx,
        ));
    }

    #[test]
    fn final_state_field_matches_fails() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_failed(evaluate(
            &Assertion::FinalStateFieldMatches {
                field: "status".to_string(),
                pattern: "^zz".to_string(),
            },
            &ctx,
        ));
    }

    #[test]
    fn final_state_field_matches_fails_on_invalid_regex() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let bad_regex = assert_failed(evaluate(
            &Assertion::FinalStateFieldMatches {
                field: "status".to_string(),
                pattern: "(".to_string(),
            },
            &ctx,
        ));
        assert!(bad_regex.render_failure().contains("compile"));
    }

    #[test]
    fn field_json_path_equals_passes() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::FieldJsonPathEquals {
                path: "/status".to_string(),
                value: serde_json::json!("done"),
            },
            &ctx,
        ));
    }

    #[test]
    fn field_json_path_equals_fails() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_failed(evaluate(
            &Assertion::FieldJsonPathEquals {
                path: "/count".to_string(),
                value: serde_json::json!(99),
            },
            &ctx,
        ));
    }

    #[test]
    fn node_not_executed_passes() {
        let records = vec![node_started(1, "worker", 1, 1)];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::NodeNotExecuted {
                node: "idle".to_string(),
            },
            &ctx,
        ));
    }

    #[test]
    fn node_not_executed_fails() {
        let records = vec![node_started(1, "worker", 1, 1)];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(
            &Assertion::NodeNotExecuted {
                node: "worker".to_string(),
            },
            &ctx,
        ));
        assert!(failure.render_failure().contains("1 execution"));
    }

    fn edge_records() -> Vec<TraceRecord> {
        vec![
            rec(
                1,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("a"),
                    to: NodeId::new("b"),
                    condition_kind: "always".to_string(),
                    fired: true,
                },
            ),
            rec(
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("a"),
                    to: NodeId::new("c"),
                    condition_kind: "contains".to_string(),
                    fired: false,
                },
            ),
        ]
    }

    #[test]
    fn edge_fired_passes() {
        let records = edge_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::EdgeFired {
                from: "a".to_string(),
                to: "b".to_string(),
            },
            &ctx,
        ));
    }

    #[test]
    fn edge_fired_fails_when_evaluated_but_not_fired() {
        let records = edge_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(
            &Assertion::EdgeFired {
                from: "a".to_string(),
                to: "c".to_string(),
            },
            &ctx,
        ));
        assert!(
            failure
                .render_failure()
                .contains("evaluated but did not fire")
        );
    }

    #[test]
    fn edge_fired_fails_when_never_evaluated() {
        let records = edge_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let never = assert_failed(evaluate(
            &Assertion::EdgeFired {
                from: "x".to_string(),
                to: "y".to_string(),
            },
            &ctx,
        ));
        assert!(never.render_failure().contains("never evaluated"));
    }

    fn route_records() -> Vec<TraceRecord> {
        vec![
            node_started(1, "a", 1, 1),
            node_started(2, "b", 2, 1),
            node_started(3, "c", 3, 1),
        ]
    }

    #[test]
    fn route_taken_passes() {
        let records = route_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::RouteTaken(vec!["a".to_string(), "c".to_string()]),
            &ctx,
        ));
    }

    #[test]
    fn route_taken_fails() {
        let records = route_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(
            &Assertion::RouteTaken(vec!["c".to_string(), "a".to_string()]),
            &ctx,
        ));
        assert!(failure.render_failure().contains("[\"a\", \"b\", \"c\"]"));
    }

    fn run_finished_records() -> Vec<TraceRecord> {
        vec![rec(
            1,
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 4,
                total_tokens: 5,
                duration_ms: 1,
                trace_dropped_total: 0,
            },
        )]
    }

    #[test]
    fn run_status_passes() {
        let records = run_finished_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::RunStatus(RunStatusValue::Completed),
            &ctx,
        ));
    }

    #[test]
    fn run_status_fails() {
        let records = run_finished_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_failed(evaluate(
            &Assertion::RunStatus(RunStatusValue::Failed),
            &ctx,
        ));
    }

    #[test]
    fn run_status_fails_when_run_never_finished() {
        let records: Vec<TraceRecord> = vec![];
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(
            &Assertion::RunStatus(RunStatusValue::Completed),
            &ctx,
        ));
        assert!(failure.render_failure().contains("never finished"));
    }

    #[test]
    fn total_tokens_max_passes() {
        let records = run_finished_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(&Assertion::TotalTokensMax(10), &ctx));
    }

    #[test]
    fn total_tokens_max_fails() {
        let records = run_finished_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_failed(evaluate(&Assertion::TotalTokensMax(1), &ctx));
    }

    #[test]
    fn supersteps_max_passes() {
        let records = run_finished_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(&Assertion::SuperstepsMax(10), &ctx));
    }

    #[test]
    fn supersteps_max_fails() {
        let records = run_finished_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_failed(evaluate(&Assertion::SuperstepsMax(1), &ctx));
    }

    fn parley_records() -> Vec<TraceRecord> {
        vec![rec(
            1,
            TraceEvent::ParleyRaised {
                parley_id: ParleyId::new(),
                node_id: NodeId::new("worker"),
                parley_kind: ParleyKind::Approval,
            },
        )]
    }

    #[test]
    fn parley_raised_passes() {
        let records = parley_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        assert_passed(evaluate(
            &Assertion::ParleyRaised {
                kind: "approval".to_string(),
                node: "worker".to_string(),
            },
            &ctx,
        ));
    }

    #[test]
    fn parley_raised_fails() {
        let records = parley_records();
        let bf = empty_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(
            &Assertion::ParleyRaised {
                kind: "choice".to_string(),
                node: "worker".to_string(),
            },
            &ctx,
        ));
        assert!(failure.render_failure().contains("kind=approval"));
    }

    #[test]
    fn final_state_snapshot_fails_when_absent_naming_bless() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);
        let ctx = AssertionContext::new(&records, &bf, &outcome);

        let failure = assert_failed(evaluate(&Assertion::FinalStateSnapshot, &ctx));
        assert!(failure.render_failure().contains("--bless"));
    }

    #[test]
    fn final_state_snapshot_matching_file_passes() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);

        let path = std::env::temp_dir().join(format!(
            "paladin-eval-assertion-test-{}-pass.json",
            std::process::id()
        ));
        std::fs::write(&path, pretty(&battlefield_to_value(&bf))).expect("writes blessed snapshot");

        let ctx = AssertionContext::new(&records, &bf, &outcome).with_snapshot_path(path.clone());
        assert_passed(evaluate(&Assertion::FinalStateSnapshot, &ctx));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn final_state_snapshot_diff_reports_first_differing_path() {
        let records: Vec<TraceRecord> = vec![];
        let bf = field_battlefield();
        let outcome = outcome_for(&bf);

        let path = std::env::temp_dir().join(format!(
            "paladin-eval-assertion-test-{}-diff.json",
            std::process::id()
        ));
        std::fs::write(&path, r#"{"count":3,"status":"pending"}"#)
            .expect("writes blessed snapshot");

        let ctx = AssertionContext::new(&records, &bf, &outcome).with_snapshot_path(path.clone());
        let failure = assert_failed(evaluate(&Assertion::FinalStateSnapshot, &ctx));
        assert!(failure.render_failure().contains("/status"));

        let _ = std::fs::remove_file(&path);
    }
}
