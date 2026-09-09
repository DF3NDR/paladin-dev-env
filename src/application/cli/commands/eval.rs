//! `paladin-cli eval run` -- drive [`paladin_eval::ScenarioRunner`] over a glob of
//! `.eval.yaml` scenario files (D-33, OBS-FR-13).
//!
//! The SAME [`paladin_eval::ScenarioRunner`] the `evals` `[[test]] harness = false`
//! target (`tests/evals.rs`) drives is used here -- one runner, two front doors
//! (D-32/D-33). This module adds `--repeat`'s flakiness detection and `--bless`'s
//! snapshot regeneration on top of it; `--live`'s three-way gate (D-35) is
//! enforced by [`paladin_eval::check_live_mode`] itself -- checked here, UP
//! FRONT, before any case runs (so a whole invocation refuses cleanly rather
//! than failing case-by-case), and checked AGAIN inside
//! `ScenarioRunner::run_case` for every caller, not only this one.

use std::path::{Path, PathBuf};

use paladin_core::platform::container::trace::TraceRecord;
use paladin_eval::{Case, RunOptions, Scenario, ScenarioRunner, check_live_mode};

use crate::application::cli::error::CliError;

/// `paladin-cli eval` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum EvalCommands {
    /// Run every scenario file matching a glob pattern.
    Run(EvalRunArgs),
}

/// Arguments for `paladin-cli eval run`.
#[derive(Debug, clap::Args)]
pub struct EvalRunArgs {
    /// Glob pattern selecting scenario files, e.g. `"evals/**/*.eval.yaml"`.
    pub glob: String,
    /// Run every matched case this many times, exiting non-zero on ANY
    /// divergence across repeats (not merely a failure) -- nondeterminism
    /// under scripted mocks is a bug to surface, never averaged away.
    #[arg(long)]
    pub repeat: Option<u32>,
    /// (Re)write each `final_state_snapshot` case's blessed file from its
    /// final `Battlefield` before evaluating assertions.
    #[arg(long)]
    pub bless: bool,
    /// Run against a real provider instead of scripted mocks. Requires the
    /// `PALADIN_EVAL_LIVE` environment variable AND a configured provider
    /// credential (D-35) -- refused otherwise, never silently falling back
    /// to mocks.
    #[arg(long)]
    pub live: bool,
    /// The name of a host-registered `EngineRegistries` a `graph_doc` target
    /// should compile through, overriding the scenario file's own
    /// `registries` field when set.
    #[arg(long)]
    pub registries: Option<String>,
}

/// One line of [`EvalRunReport`]'s output, plus the summary counters
/// `paladin-cli eval run`'s exit code is computed from.
pub struct EvalRunReport {
    /// Every output line, in the order they should be printed.
    pub lines: Vec<String>,
    /// How many of `total_cases` passed (a `--repeat` case counts as passed
    /// only when every repeat passed AND no divergence was detected).
    pub passed_cases: usize,
    /// How many `(file, case)` pairs were matched and run.
    pub total_cases: usize,
    /// Whether the whole command should exit `0`.
    pub exit_ok: bool,
}

impl EvalRunReport {
    /// Join every line with `\n` -- the exact text `run_eval` prints to
    /// stdout.
    pub fn render(&self) -> String {
        self.lines.join("\n")
    }
}

/// `paladin-cli eval run <glob> [--repeat N] [--bless] [--live] [--registries <name>]`
/// (D-33): prints one line per case (or per `--repeat` sweep) plus a summary
/// count, exiting non-zero if any case failed, diverged under `--repeat`, or
/// the glob itself was unreadable.
pub async fn run_eval(
    glob: String,
    repeat: Option<u32>,
    bless: bool,
    live: bool,
    registries: Option<String>,
) -> Result<(), CliError> {
    let report = run_eval_report(glob, repeat, bless, live, registries).await?;
    println!("{}", report.render());
    if report.exit_ok {
        Ok(())
    } else {
        Err(CliError::execution(format!(
            "{}/{} eval cases passed",
            report.passed_cases, report.total_cases
        )))
    }
}

/// The testable core behind [`run_eval`]: builds an [`EvalRunReport`]
/// without printing or translating it into a process exit code, so a test
/// can assert on `EvalRunReport::render()`/`passed_cases`/`exit_ok`
/// directly.
pub async fn run_eval_report(
    glob: String,
    repeat: Option<u32>,
    bless: bool,
    live: bool,
    _registries: Option<String>,
) -> Result<EvalRunReport, CliError> {
    // D-35: refuse the WHOLE invocation up front when --live's three-way
    // gate is not satisfied -- never run some cases scripted and others
    // live, and never silently fall back to scripted mocks. ScenarioRunner
    // ALSO enforces this per case (defence in depth for callers other than
    // this command), so this is belt-and-braces, not the only check.
    if live {
        check_live_mode(true).map_err(|source| CliError::execution(source.to_string()))?;
    }

    let mut paths: Vec<PathBuf> = glob::glob(&glob)
        .map_err(|source| CliError::InvalidFilePath {
            path: glob.clone(),
            message: source.to_string(),
        })?
        .filter_map(Result::ok)
        .collect();
    paths.sort();

    // No graph constructor or registries is pre-registered here: the
    // facade's own eval registry (the E2E-1/2/3 constructors) lands in plan
    // 28-16. A `registered` target is therefore always unresolved today --
    // `ScenarioRunner::run_case` reports that as a clear,
    // named-target-and-registered-list `CaseOutcome::Errored`, not a panic
    // or a silent skip.
    let runner = ScenarioRunner::new();

    let mut lines = Vec::new();
    let mut total_cases = 0usize;
    let mut passed_cases = 0usize;
    let mut any_failed = false;

    for path in &paths {
        let scenario = match Scenario::from_path(path) {
            Ok(scenario) => scenario,
            Err(err) => {
                any_failed = true;
                lines.push(format!("{}: FAILED TO PARSE: {err}", path.display()));
                continue;
            }
        };

        for case in &scenario.cases {
            total_cases += 1;
            if let Some(n) = repeat {
                let (line, ok) =
                    run_repeat_sweep(&runner, path, &scenario, case, n, bless, live).await;
                if ok {
                    passed_cases += 1;
                } else {
                    any_failed = true;
                }
                lines.push(line);
            } else {
                let options = RunOptions { bless, live };
                let report = runner.run_case(path, &scenario, case, options).await;
                let label = format!("{}::{}", path.display(), case.name);
                if report.passed() {
                    passed_cases += 1;
                    lines.push(format!("PASS {label}"));
                } else {
                    any_failed = true;
                    lines.push(format!("FAIL {label}\n{}", report.render_failures()));
                }
            }
        }
    }

    lines.push(format!("{passed_cases}/{total_cases} cases passed"));

    Ok(EvalRunReport {
        lines,
        passed_cases,
        total_cases,
        exit_ok: !any_failed,
    })
}

/// Run one case `repeat` times, treating ANY divergence across the repeats
/// -- not merely a failure -- as a non-zero-exit condition (D-33,
/// OBS-FR-13): reports the per-case pass rate and, when the repeats
/// diverge, the `seq` range where the first divergence was observed and
/// which run it first appeared in.
async fn run_repeat_sweep(
    runner: &ScenarioRunner,
    path: &Path,
    scenario: &Scenario,
    case: &Case,
    repeat: u32,
    bless: bool,
    live: bool,
) -> (String, bool) {
    let label = format!("{}::{}", path.display(), case.name);
    let mut passed_count = 0u32;
    let mut all_records: Vec<Vec<TraceRecord>> = Vec::with_capacity(repeat.max(1) as usize);

    for _ in 0..repeat {
        // Never bless mid-sweep: blessing on repeat K would overwrite the
        // file against a run that is not the one `--bless`'s own final,
        // single write (below) blesses against.
        let options = RunOptions { bless: false, live };
        let report = runner.run_case(path, scenario, case, options).await;
        if report.passed() {
            passed_count += 1;
        }
        all_records.push(report.records().to_vec());
    }

    if bless {
        let options = RunOptions { bless: true, live };
        let _ = runner.run_case(path, scenario, case, options).await;
    }

    let divergence = first_divergence(&all_records);
    let ok = passed_count == repeat && divergence.is_none();

    let mut line = format!("{label}: {passed_count}/{repeat} passed");
    if let Some((run_index, seq_from, seq_to)) = divergence {
        line.push_str(&format!(
            "\n  DIVERGENCE: run {run_index} differs from run 0, first divergence in \
             seq range {seq_from}..={seq_to} -- nondeterminism under scripted mocks is a \
             bug to surface, never averaged away"
        ));
    }
    (line, ok)
}

/// The first `(run_index, seq_from, seq_to)` at which some run's captured
/// record stream diverges from run 0's, comparing a normalized
/// [`record_fingerprint`] (never the raw record: `TraceRecord::at` and
/// several per-attempt `duration_ms`/generated-id fields are wall-clock or
/// UUID noise that legitimately differs run-to-run even under a fully
/// deterministic scripted LLM). `None` when every run's fingerprint
/// sequence is identical.
///
/// `pub` (not `pub(crate)`): `tests/cli/eval_run_test.rs` exercises this
/// pure function directly with synthetically "injected" divergent record
/// streams -- see that test's own module doc for why a deliberately
/// nondeterministic ENGINE run is neither necessary nor achievable through
/// this crate's own scripted-mock `ScenarioLlm`.
pub fn first_divergence(all_records: &[Vec<TraceRecord>]) -> Option<(usize, u64, u64)> {
    let baseline = all_records.first()?;
    let baseline_fingerprints: Vec<String> = baseline.iter().map(record_fingerprint).collect();
    let baseline_max_seq = baseline.last().map(|r| r.seq).unwrap_or(0);

    for (run_index, records) in all_records.iter().enumerate().skip(1) {
        let candidate_fingerprints: Vec<String> = records.iter().map(record_fingerprint).collect();
        if candidate_fingerprints == baseline_fingerprints {
            continue;
        }
        let max_len = baseline_fingerprints
            .len()
            .max(candidate_fingerprints.len());
        for i in 0..max_len {
            if baseline_fingerprints.get(i) != candidate_fingerprints.get(i) {
                let seq_from = baseline
                    .get(i)
                    .or_else(|| records.get(i))
                    .map(|r| r.seq)
                    .unwrap_or(0);
                let candidate_max_seq = records.last().map(|r| r.seq).unwrap_or(seq_from);
                let seq_to = baseline_max_seq.max(candidate_max_seq);
                return Some((run_index, seq_from, seq_to));
            }
        }
    }
    None
}

/// Render one [`TraceRecord`]'s content-identity, deliberately EXCLUDING
/// wall-clock (`at`, every `duration_ms`) and randomly-generated-per-run
/// fields (`waypoint_id`, `parley_id`, `thread_id`) so two runs of a fully
/// deterministic scripted scenario fingerprint identically -- only a real
/// behavioral divergence (a different node executing, a different edge
/// firing, a different route, a different outcome) changes this string.
fn record_fingerprint(record: &TraceRecord) -> String {
    use paladin_core::platform::container::trace::TraceEvent;
    match &record.event {
        TraceEvent::RunStarted {
            graph_fingerprint, ..
        } => format!("run_started:{graph_fingerprint}"),
        TraceEvent::SuperstepStarted {
            superstep,
            vanguard,
        } => {
            format!("superstep_started:{superstep}:{vanguard:?}")
        }
        TraceEvent::NodeStarted {
            superstep,
            node_id,
            attempt,
            ..
        } => format!("node_started:{superstep}:{node_id}:{attempt}"),
        TraceEvent::NodeProgress { node_id, progress } => {
            format!("node_progress:{node_id}:{progress:?}")
        }
        TraceEvent::NodeFinished {
            superstep,
            node_id,
            attempt,
            outcome,
            ..
        } => format!("node_finished:{superstep}:{node_id}:{attempt}:{outcome:?}"),
        TraceEvent::EdgeEvaluated {
            from,
            to,
            condition_kind,
            fired,
        } => format!("edge_evaluated:{from}:{to}:{condition_kind}:{fired}"),
        TraceEvent::DeltaMerged {
            superstep,
            field_changes,
        } => format!("delta_merged:{superstep}:{field_changes:?}"),
        TraceEvent::WaypointSaved {
            superstep, status, ..
        } => format!("waypoint_saved:{superstep}:{status}"),
        TraceEvent::ParleyRaised {
            node_id,
            parley_kind,
            ..
        } => format!("parley_raised:{node_id}:{parley_kind:?}"),
        TraceEvent::RunFinished {
            status,
            total_supersteps,
            total_tokens,
            ..
        } => format!("run_finished:{status:?}:{total_supersteps}:{total_tokens}"),
        TraceEvent::FallbackHop {
            node_id,
            from_provider,
            to_provider,
        } => format!("fallback_hop:{node_id:?}:{from_provider}:{to_provider}"),
        TraceEvent::MiddlewareEvent { name, action } => {
            format!("middleware_event:{name}:{action:?}")
        }
        // TraceEvent is #[non_exhaustive]: a future variant this match has
        // not been updated for still fingerprints deterministically via its
        // own Debug rendering, never panics.
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_run_args_parse_every_flag() {
        use clap::Parser;

        #[derive(clap::Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: EvalRunArgs,
        }

        let parsed = Wrapper::parse_from([
            "paladin-cli",
            "evals/**/*.eval.yaml",
            "--repeat",
            "20",
            "--bless",
            "--live",
            "--registries",
            "e2e",
        ]);
        assert_eq!(parsed.args.glob, "evals/**/*.eval.yaml");
        assert_eq!(parsed.args.repeat, Some(20));
        assert!(parsed.args.bless);
        assert!(parsed.args.live);
        assert_eq!(parsed.args.registries.as_deref(), Some("e2e"));
    }
}
