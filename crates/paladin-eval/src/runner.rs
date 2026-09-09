//! `ScenarioRunner` -- drives a [`crate::scenario::Scenario`] case through a real
//! [`WarEngine`], captures its trace record stream, and evaluates its assertions
//! (D-29, D-31, D-32, D-34).
//!
//! # What a case run touches (D-29)
//!
//! A case run compiles or resolves the scenario's [`ScenarioTarget`] into a real
//! [`WarGraph`], substitutes a [`ScenarioLlm`] for every Paladin node's `LlmPort`
//! slot, attaches a capturing [`TraceSink`] through [`WarEngine::with_trace_sink`],
//! drives the run to a terminal [`RunOutcome`] (answering any raised Parley from
//! [`crate::scenario::Case::parley_responses`] in order), and evaluates every
//! [`crate::scenario::Assertion`] through [`crate::assertion::evaluate`] against
//! exactly the captured records, the final `Battlefield`, and the `RunOutcome` --
//! nothing else.
//!
//! # Two targets, one substitution point (D-31)
//!
//! A [`ScenarioTarget::GraphDoc`] compiles through the [`EngineRegistries`] the
//! scenario names (or an empty default); a [`ScenarioTarget::Registered`] resolves
//! against a host-registered [`GraphConstructor`] closure, receiving a
//! [`ScriptedPorts`] handle so code-built graphs with `Function` nodes, worker
//! templates, custom edge evaluators and Aegis handlers can reach a per-node
//! [`ScenarioLlm`] directly -- the same substitution a `graph_doc` target's Paladin
//! nodes receive automatically by walking the compiled [`WarGraph`]'s own node
//! table.
//!
//! # Simulated crash-and-resume (D-28, D-34)
//!
//! A case with `interrupt_after_superstep` set reuses the EXACT technique
//! `tests/integration/e2e_crash_resume_test.rs` documents: a throwaway "control"
//! run drives the scenario to completion over its own temp-file SQLite store, the
//! first N Waypoints are copied into a fresh "resumed" store, and a brand new
//! engine resumes from there. This is deterministic (no `CancellationToken` race
//! against an async trace sink) at the cost of one documented limitation: a
//! scenario's SHARED `llm.global` sequence is consumed in call order across every
//! node, so the resumed run's fresh cursor does not "fast-forward" past the calls
//! the already-completed (seeded) nodes made pre-crash. Author an
//! `interrupt_after_superstep` case's LLM script with PER-NODE sequences
//! (`llm.per_node`) for deterministic replay -- each node's own cursor is
//! independent of which OTHER nodes already ran, so a fresh root reproduces the
//! same per-node sequence regardless of the crash point.
//!
//! # The `evals` harness target (D-32)
//!
//! [`crate::eval_scenarios!`] expands to a `fn main()` for a `harness = false` `[[test]]`
//! target: it globs the given pattern at RUNTIME (so `evals/` growing between
//! builds needs no manifest change) and hands one `libtest_mimic::Trial` per
//! `(file, case)` pair to the custom harness, named `<file-stem>::<case>` -- so
//! `cargo test --test evals <filter>` filters exactly like any other Rust test.
//! Every trial runs its case on a FRESH single-threaded Tokio runtime (never a
//! shared one), so one case's runtime state can never leak into another's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use libtest_mimic::{Arguments, Failed, Trial};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use paladin_battalion::engine::graph_doc::WarGraphDoc;
use paladin_battalion::engine::{
    EngineError, EngineRegistries, NodeSpec, RunOutcome, WarEngine, WarGraph,
};
use paladin_core::platform::container::battlefield::{Battlefield, FieldName, StateDelta};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::trace::TraceRecord;
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use paladin_ports::output::paladin_port::{
    PaladinPort, PaladinResult, PaladinStream, PaladinStreamChunk, StopReason,
};
use paladin_ports::output::trace_sink_port::{TraceSink, TraceSinkError};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

use crate::assertion::{self, AssertionContext, AssertionFailure, AssertionOutcome};
use crate::scenario::{Assertion, Case, ParleyScript, Scenario, ScenarioTarget, StoreKind};
use crate::scripted_llm::ScenarioLlm;

// ---------------------------------------------------------------------------
// ScriptedPorts / GraphConstructor / RegistriesFactory (D-31)
// ---------------------------------------------------------------------------

/// The per-node scripted-port handle a [`GraphConstructor`] receives (D-31).
///
/// Wraps ONE scenario's resolved [`crate::scenario::LlmScript`] as a
/// [`ScenarioLlm`] "root": [`ScriptedPorts::for_node`] hands out a
/// [`ScenarioLlm::for_node`] clone sharing that root's cursors, call counters
/// and captured-request log, so a code-built `Function` node can call
/// `.generate()` directly with the exact same per-node routing a compiled
/// `graph_doc`'s Paladin nodes receive automatically.
pub struct ScriptedPorts {
    root: ScenarioLlm,
}

impl ScriptedPorts {
    /// Return a [`ScenarioLlm`] attributed to `node_id`, sharing this handle's
    /// underlying script, cursors and captured-request log.
    pub fn for_node(&self, node_id: impl Into<String>) -> ScenarioLlm {
        self.root.for_node(node_id)
    }
}

/// A host-registered Rust closure building a [`WarGraph`] from a resolved
/// [`ScriptedPorts`] handle (D-31), registered via
/// [`ScenarioRunner::register_graph`].
pub type GraphConstructor = Arc<dyn Fn(&ScriptedPorts) -> WarGraph + Send + Sync>;

/// A host-registered Rust closure building an [`EngineRegistries`] bundle,
/// registered via [`ScenarioRunner::register_registries`] and resolved by a
/// `graph_doc` target naming it in [`crate::scenario::Scenario::registries`].
pub type RegistriesFactory = Arc<dyn Fn() -> EngineRegistries + Send + Sync>;

// ---------------------------------------------------------------------------
// RunOptions
// ---------------------------------------------------------------------------

/// Per-run options a caller of [`ScenarioRunner::run_case`] controls (D-32,
/// D-33). The `evals` harness (`eval_scenarios!`) always uses
/// [`RunOptions::default`] -- `bless: false`, `live: false` -- so a plain
/// `cargo test --test evals` never blesses a snapshot and never enters live
/// mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct RunOptions {
    /// When `true` and the case carries a `final_state_snapshot` assertion,
    /// (re)write the blessed snapshot file from the final `Battlefield`
    /// before evaluating assertions -- so the freshly-blessed file always
    /// passes its own comparison.
    pub bless: bool,
    /// Reserved for the gated live-model mode; inert until the runner's own
    /// `LiveMode` gate exists. `paladin-cli eval run`'s `--live` flag threads
    /// its value through here.
    pub live: bool,
}

// ---------------------------------------------------------------------------
// CaseReport / Verdict
// ---------------------------------------------------------------------------

/// One evaluated [`Assertion`]'s result within a [`CaseReport`].
#[derive(Debug)]
pub struct AssertionResult {
    /// The verdict this assertion reached.
    pub verdict: Verdict,
}

/// The outcome of evaluating one [`Assertion`] (D-29, D-35).
#[derive(Debug)]
pub enum Verdict {
    /// The assertion held.
    Passed,
    /// The assertion did not hold; carries the actionable failure.
    Failed(AssertionFailure),
    /// A content-bearing assertion (D-35) was skipped because the run was
    /// live and the scenario did not opt in (`live.allow_content_assertions`)
    /// -- carries the skip reason. Never folded into
    /// [`CaseReport::passed`]'s pass count: a skip is neither a pass nor a
    /// failure.
    Skipped(String),
}

/// What a [`ScenarioRunner::run_case`] call produced for one
/// [`crate::scenario::Case`] (D-29, D-32).
pub struct CaseReport {
    /// This case's name (`Case::name`), used in trial naming and CLI output.
    pub case_name: String,
    /// Whether the case ran at all, or errored before assertions could be
    /// evaluated (an unresolvable target, a compile failure, an engine
    /// error, ...).
    pub outcome: CaseOutcome,
}

/// The two shapes a [`CaseReport::outcome`] can take.
pub enum CaseOutcome {
    /// The case's engine run reached a terminal [`RunOutcome`] and every
    /// assertion was evaluated against it.
    Ran {
        /// The captured `TraceRecord` stream, in `seq` order.
        records: Vec<TraceRecord>,
        /// The final `Battlefield` assertions were evaluated against.
        final_state: Battlefield,
        /// One [`AssertionResult`] per [`Case::assertions`] entry, in order.
        results: Vec<AssertionResult>,
    },
    /// The case never reached a point where assertions could be evaluated
    /// (a resolution, compile, or engine-level failure -- never itself an
    /// assertion failure).
    Errored(String),
}

impl CaseReport {
    /// Whether every assertion in this report passed. A case that
    /// [`CaseOutcome::Errored`] never passes.
    pub fn passed(&self) -> bool {
        match &self.outcome {
            CaseOutcome::Ran { results, .. } => !results
                .iter()
                .any(|r| matches!(r.verdict, Verdict::Failed(_))),
            CaseOutcome::Errored(_) => false,
        }
    }

    /// The captured trace record stream, or an empty slice for an errored
    /// case.
    pub fn records(&self) -> &[TraceRecord] {
        match &self.outcome {
            CaseOutcome::Ran { records, .. } => records,
            CaseOutcome::Errored(_) => &[],
        }
    }

    /// Render every failing assertion's [`AssertionFailure::render_failure`]
    /// text, concatenated, or a one-line description of the runner-level
    /// error for an errored case.
    pub fn render_failures(&self) -> String {
        match &self.outcome {
            CaseOutcome::Errored(message) => {
                format!(
                    "case {:?} errored before assertions could run: {message}",
                    self.case_name
                )
            }
            CaseOutcome::Ran { results, .. } => {
                let mut out = String::new();
                for result in results {
                    if let Verdict::Failed(failure) = &result.verdict {
                        out.push_str(&failure.render_failure());
                    }
                }
                if out.is_empty() {
                    "(no failing assertions)".to_string()
                } else {
                    out
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ScenarioRunner
// ---------------------------------------------------------------------------

/// Drives [`Scenario`] cases through a real [`WarEngine`] (D-29, D-31, D-32).
///
/// Holds two host-registered name tables -- [`GraphConstructor`]s
/// ([`ScenarioRunner::register_graph`]) and [`RegistriesFactory`]s
/// ([`ScenarioRunner::register_registries`]) -- consulted by
/// [`ScenarioRunner::run_case`] when a scenario's target or `registries` field
/// names one. `Clone`: every registration is `Arc`-backed, so cloning is cheap
/// and each [`ScenarioRunner::trials`] `Trial` closure can own its own copy.
#[derive(Clone, Default)]
pub struct ScenarioRunner {
    graphs: HashMap<String, GraphConstructor>,
    registries: HashMap<String, RegistriesFactory>,
}

impl ScenarioRunner {
    /// Construct an empty runner: no graphs, no registries registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a [`GraphConstructor`] under `name`, resolved by a
    /// [`ScenarioTarget::Registered { registered: name }`](ScenarioTarget::Registered)
    /// target.
    pub fn register_graph(&mut self, name: impl Into<String>, ctor: GraphConstructor) -> &mut Self {
        self.graphs.insert(name.into(), ctor);
        self
    }

    /// Register a [`RegistriesFactory`] under `name`, resolved by a
    /// `graph_doc` target naming it in [`Scenario::registries`].
    pub fn register_registries(
        &mut self,
        name: impl Into<String>,
        factory: RegistriesFactory,
    ) -> &mut Self {
        self.registries.insert(name.into(), factory);
        self
    }

    fn resolve_registries(&self, name: Option<&str>) -> Result<EngineRegistries, RunnerError> {
        match name {
            None => Ok(EngineRegistries::new()),
            Some(name) => {
                let factory =
                    self.registries
                        .get(name)
                        .ok_or_else(|| RunnerError::UnknownRegistries {
                            name: name.to_string(),
                            registered: self.registries.keys().cloned().collect(),
                        })?;
                Ok((factory)())
            }
        }
    }

    /// Resolve `scenario.target` into a real [`WarGraph`] plus the
    /// [`ScenarioLlm`] router keyed by each Paladin node's OWN `PaladinData`
    /// name (D-31) -- the identity [`PaladinPort::execute`] receives, mapped
    /// to a [`ScenarioLlm::for_node`] clone tagged with that node's GRAPH id
    /// (the identity a scenario's `llm.per_node` map is keyed by).
    fn resolve_target(
        &self,
        scenario_path: &Path,
        scenario: &Scenario,
    ) -> Result<(WarGraph, HashMap<String, ScenarioLlm>), RunnerError> {
        match &scenario.target {
            ScenarioTarget::GraphDoc { graph_doc } => {
                let path = resolve_relative(scenario_path, graph_doc);
                let doc = load_graph_doc(&path)?;
                let registries = self.resolve_registries(scenario.registries.as_deref())?;
                let graph = doc
                    .compile(&registries)
                    .map_err(|source| RunnerError::Compile {
                        path: path.clone(),
                        source: Box::new(source),
                    })?;
                let root = ScenarioLlm::new(scenario.llm.clone());
                let router = build_llm_router(&graph, &root);
                Ok((graph, router))
            }
            ScenarioTarget::Registered { registered } => {
                let ctor =
                    self.graphs
                        .get(registered)
                        .ok_or_else(|| RunnerError::UnknownGraph {
                            name: registered.clone(),
                            registered: self.graphs.keys().cloned().collect(),
                        })?;
                let root = ScenarioLlm::new(scenario.llm.clone());
                let scripted_ports = ScriptedPorts { root: root.clone() };
                let graph = (ctor)(&scripted_ports);
                let router = build_llm_router(&graph, &root);
                Ok((graph, router))
            }
        }
    }

    /// Run one [`Case`] of `scenario` (parsed from `scenario_path`) and
    /// evaluate its assertions. Never panics on a resolution/engine failure
    /// -- reported as [`CaseOutcome::Errored`] instead.
    pub async fn run_case(
        &self,
        scenario_path: &Path,
        scenario: &Scenario,
        case: &Case,
        options: RunOptions,
    ) -> CaseReport {
        match self
            .run_case_inner(scenario_path, scenario, case, options)
            .await
        {
            Ok(report) => report,
            Err(err) => CaseReport {
                case_name: case.name.clone(),
                outcome: CaseOutcome::Errored(err.to_string()),
            },
        }
    }

    async fn run_case_inner(
        &self,
        scenario_path: &Path,
        scenario: &Scenario,
        case: &Case,
        options: RunOptions,
    ) -> Result<CaseReport, RunnerError> {
        // D-35: the runner ITSELF enforces the three-way live gate, not
        // only a caller that remembers to check first -- `options.live` is
        // `false` on every path `eval_scenarios!`'s own harness takes, so
        // this is unreachable for a plain `cargo test --test evals`.
        if options.live {
            check_live_mode(true)?;
        }

        let mut initial = StateDelta::new();
        for (key, value) in &case.input {
            let field = FieldName::new(key).map_err(|source| RunnerError::InvalidField {
                field: key.clone(),
                message: source.to_string(),
            })?;
            initial.set_raw(field, value.clone());
        }

        let thread = ThreadId::new(format!("scenario-{}", Uuid::new_v4()))
            .map_err(|source| RunnerError::InvalidThreadId(source.to_string()))?;

        let (records, outcome, final_state) =
            if let Some(superstep) = case.interrupt_after_superstep {
                if scenario.store != StoreKind::SqliteTemp {
                    return Err(RunnerError::InterruptRequiresSqliteTemp {
                        case: case.name.clone(),
                    });
                }
                self.run_interrupted(
                    scenario_path,
                    scenario,
                    case,
                    &thread,
                    &initial,
                    superstep,
                    options,
                )
                .await?
            } else {
                let (graph, router) = self.resolve_target(scenario_path, scenario)?;
                let port = build_port(router, options)?;
                match scenario.store {
                    StoreKind::InMemory => {
                        let store = Arc::new(InMemoryWaypointStore::new());
                        run_plain(
                            store,
                            &graph,
                            port,
                            thread.clone(),
                            initial,
                            &case.parley_responses,
                        )
                        .await?
                    }
                    StoreKind::SqliteTemp => {
                        let store = Arc::new(SqliteWaypointStore::new(&temp_db_url("case")).await?);
                        run_plain(
                            store,
                            &graph,
                            port,
                            thread.clone(),
                            initial,
                            &case.parley_responses,
                        )
                        .await?
                    }
                }
            };

        let snapshot_path = snapshot_file_path(scenario_path, &case.name);
        if options.bless
            && case
                .assertions
                .iter()
                .any(|a| matches!(a, Assertion::FinalStateSnapshot))
        {
            let value = battlefield_snapshot_value(&final_state);
            // Trailing newline: keeps blessed snapshots hook-clean (`end-of-file-fixer`
            // runs over all files in CI); comparison is JSON-based, so it is inert.
            let mut text =
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
            text.push('\n');
            if let Some(parent) = snapshot_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&snapshot_path, text).map_err(|source| RunnerError::Io {
                path: snapshot_path.clone(),
                source,
            })?;
        }

        let ctx = AssertionContext::new(&records, &final_state, &outcome)
            .with_snapshot_path(snapshot_path);

        let mut results = Vec::with_capacity(case.assertions.len());
        for assertion in &case.assertions {
            let verdict = if should_skip_for_live(
                assertion,
                options.live,
                scenario.live.allow_content_assertions,
            ) {
                Verdict::Skipped("content assertion, live mode".to_string())
            } else {
                match assertion::evaluate(assertion, &ctx) {
                    AssertionOutcome::Passed => Verdict::Passed,
                    AssertionOutcome::Failed(failure) => Verdict::Failed(failure),
                }
            };
            results.push(AssertionResult { verdict });
        }

        Ok(CaseReport {
            case_name: case.name.clone(),
            outcome: CaseOutcome::Ran {
                records,
                final_state,
                results,
            },
        })
    }

    /// The simulated-crash driver (D-28, D-34): a throwaway "control" run
    /// drives `scenario`'s case to completion over its own temp-file SQLite
    /// store; every Waypoint up to and including `interrupt_superstep` is
    /// copied into a FRESH "resumed" store; a brand new engine resumes from
    /// there. See this module's own doc comment for the per-node-vs-global
    /// LLM script caveat.
    #[allow(clippy::too_many_arguments)]
    async fn run_interrupted(
        &self,
        scenario_path: &Path,
        scenario: &Scenario,
        case: &Case,
        thread: &ThreadId,
        initial: &StateDelta,
        interrupt_superstep: u64,
        options: RunOptions,
    ) -> Result<(Vec<TraceRecord>, RunOutcome, Battlefield), RunnerError> {
        // Control run: throwaway store and sink, driven to completion so its
        // Waypoint history is ground truth for the seed below.
        let (control_graph, control_router) = self.resolve_target(scenario_path, scenario)?;
        let control_port = build_port(control_router, options)?;
        let control_store = Arc::new(SqliteWaypointStore::new(&temp_db_url("control")).await?);
        let control_sink: Arc<CapturingSink> = Arc::new(CapturingSink::new());
        let control_engine = WarEngine::new(control_port, Arc::clone(&control_store))
            .with_trace_sink(control_sink.clone());
        let control_outcome = control_engine
            .start(&control_graph, thread.clone(), initial.clone())
            .await?;
        drive_parleys(
            &control_engine,
            &control_graph,
            thread,
            control_outcome,
            &case.parley_responses,
        )
        .await?;

        // Seed a fresh store with every Waypoint up to and including the
        // interrupt superstep -- "the process died right after superstep N".
        let resumed_store = Arc::new(SqliteWaypointStore::new(&temp_db_url("resumed")).await?);
        let mut history = full_history(control_store.as_ref(), thread).await?;
        history.sort_by_key(|wp| wp.superstep);
        for waypoint in history
            .iter()
            .filter(|wp| wp.superstep <= interrupt_superstep)
        {
            resumed_store.save(waypoint).await?;
        }

        // Resumed run: a FRESH graph/router (fresh ScenarioLlm cursors) and
        // the REAL capturing sink whose records feed this case's assertions.
        let (resumed_graph, resumed_router) = self.resolve_target(scenario_path, scenario)?;
        let resumed_port = build_port(resumed_router, options)?;
        let resumed_sink: Arc<CapturingSink> = Arc::new(CapturingSink::new());
        let resumed_engine = WarEngine::new(resumed_port, Arc::clone(&resumed_store))
            .with_trace_sink(resumed_sink.clone());
        let resumed_outcome = resumed_engine
            .resume(&resumed_graph, thread.clone())
            .await?;
        let resumed_outcome = drive_parleys(
            &resumed_engine,
            &resumed_graph,
            thread,
            resumed_outcome,
            &case.parley_responses,
        )
        .await?;

        // The dispatcher's consumer task drains asynchronously -- give it a
        // moment before reading the captured records back out (established
        // house pattern, see `engine::hooks`'s own tests).
        tokio::time::sleep(Duration::from_millis(50)).await;
        let records = resumed_sink.records();
        let final_state =
            resolve_battlefield(resumed_store.as_ref(), thread, &resumed_outcome).await?;
        Ok((records, resumed_outcome, final_state))
    }

    /// Glob `pattern` at RUNTIME and build one [`Trial`] per `(file, case)`
    /// pair, named `<file-stem>::<case>` (D-32). A scenario file that fails
    /// to parse becomes a single failing `<file-stem>::parse_error` trial
    /// rather than aborting discovery for every other file. Each trial's
    /// runner drives its case on a FRESH single-threaded Tokio runtime.
    pub fn trials(&self, pattern: &str) -> Vec<Trial> {
        let mut paths: Vec<PathBuf> = match glob::glob(pattern) {
            Ok(entries) => entries.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        };
        paths.sort();

        let mut trials = Vec::new();
        for path in paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("scenario")
                .to_string();
            match Scenario::from_path(&path) {
                Ok(scenario) => {
                    let scenario = Arc::new(scenario);
                    for case in &scenario.cases {
                        let trial_name = format!("{stem}::{}", case.name);
                        let runner = self.clone();
                        let scenario = Arc::clone(&scenario);
                        let case = case.clone();
                        let path = path.clone();
                        trials.push(Trial::test(trial_name, move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|e| Failed::from(e.to_string()))?;
                            let report = rt.block_on(runner.run_case(
                                &path,
                                &scenario,
                                &case,
                                RunOptions::default(),
                            ));
                            if report.passed() {
                                Ok(())
                            } else {
                                Err(Failed::from(report.render_failures()))
                            }
                        }));
                    }
                }
                Err(err) => {
                    trials.push(Trial::test(format!("{stem}::parse_error"), move || {
                        Err(Failed::from(err.to_string()))
                    }));
                }
            }
        }
        trials
    }
}

/// The `fn main()` body [`crate::eval_scenarios!`] expands to (D-32): parses
/// `libtest_mimic::Arguments` from the process's own CLI args, builds a
/// default (nothing registered) [`ScenarioRunner`]'s trials over `pattern`,
/// and exits with the harness's own exit code.
pub fn run_eval_harness_main(pattern: &str) -> ! {
    let args = Arguments::from_args();
    let runner = ScenarioRunner::new();
    let trials = runner.trials(pattern);
    libtest_mimic::run(&args, trials).exit()
}

/// Like [`run_eval_harness_main`], but calls `configure` on a fresh
/// [`ScenarioRunner`] before building trials (D-31, D-34, plan 28-16): lets a
/// host `[[test]] harness = false` binary register its own
/// [`ScenarioRunner::register_graph`]/[`ScenarioRunner::register_registries`]
/// constructors before the pattern is globbed, so a `registered` target
/// resolves instead of reporting [`RunnerError::UnknownGraph`].
pub fn run_eval_harness_main_with(pattern: &str, configure: impl FnOnce(&mut ScenarioRunner)) -> ! {
    let args = Arguments::from_args();
    let mut runner = ScenarioRunner::new();
    configure(&mut runner);
    let trials = runner.trials(pattern);
    libtest_mimic::run(&args, trials).exit()
}

/// Expands to a `fn main()` for a `harness = false` `[[test]]` target (D-32):
/// globs `$pattern` at runtime and hands one `libtest_mimic::Trial` per
/// `(file, case)` pair to the custom harness, so `cargo test --test <name>
/// <filter>` filters exactly like any other Rust test.
///
/// A second, optional argument registers graph constructors and registries
/// factories before the pattern is globbed (D-31, D-34) -- needed by any
/// `registered` (as opposed to `graph_doc`) scenario target.
///
/// # Examples
///
/// ```ignore
/// // tests/evals.rs -- no registered targets
/// paladin_eval::eval_scenarios!("evals/**/*.eval.yaml");
/// ```
///
/// ```ignore
/// // tests/evals.rs -- with registered targets
/// paladin_eval::eval_scenarios!("evals/**/*.eval.yaml", |runner: &mut paladin_eval::ScenarioRunner| {
///     runner.register_graph("my-graph", std::sync::Arc::new(|_ports| my_graph()));
/// });
/// ```
#[macro_export]
macro_rules! eval_scenarios {
    ($pattern:expr) => {
        fn main() {
            $crate::runner::run_eval_harness_main($pattern)
        }
    };
    ($pattern:expr, $configure:expr) => {
        fn main() {
            $crate::runner::run_eval_harness_main_with($pattern, $configure)
        }
    };
}

// ---------------------------------------------------------------------------
// ScenarioPaladinPort -- the PaladinPort bridging every Paladin node's
// LlmPort slot to its substituted ScenarioLlm (D-31).
// ---------------------------------------------------------------------------

/// Where a [`ScenarioPaladinPort`] reaches for its `LlmPort` (D-31, D-35).
enum LlmSource {
    /// Per-node scripted routing (the default, mock mode).
    Scripted(HashMap<String, ScenarioLlm>),
    /// A single real provider, resolved by [`resolve_live_provider`] once
    /// [`check_live_mode`] has already passed (D-35): live mode uses ONE
    /// provider for the whole run (routing by each node's own `model` is
    /// the provider's own job, exactly as `LlmRequest::model` already
    /// carries), never a per-node substitution the way scripted mode does.
    Live(Arc<dyn LlmPort>),
}

/// A [`PaladinPort`] implementation routing each call to the [`ScenarioLlm`]
/// substituted for that Paladin's own node (D-31): looked up by the
/// `Paladin`'s own `PaladinData.name` -- the only identity
/// [`PaladinPort::execute`] receives.
struct ScenarioPaladinPort {
    llm: LlmSource,
}

impl ScenarioPaladinPort {
    fn scripted(router: HashMap<String, ScenarioLlm>) -> Self {
        Self {
            llm: LlmSource::Scripted(router),
        }
    }

    fn live(provider: Arc<dyn LlmPort>) -> Self {
        Self {
            llm: LlmSource::Live(provider),
        }
    }
}

#[async_trait]
impl PaladinPort for ScenarioPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: input.to_string(),
            context: None,
        }))
        .map_err(|source| PaladinError::ConfigurationError(source.to_string()))?;
        let request = LlmRequest::new(paladin.node.model.clone(), prompt);

        let response = match &self.llm {
            LlmSource::Scripted(router) => {
                let llm = router.get(&paladin.node.name).ok_or_else(|| {
                    PaladinError::ConfigurationError(format!(
                        "ScenarioPaladinPort: no ScenarioLlm registered for paladin {:?} \
                         (known: {:?})",
                        paladin.node.name,
                        router.keys().collect::<Vec<_>>()
                    ))
                })?;
                llm.generate(request).await
            }
            LlmSource::Live(provider) => provider.generate(request).await,
        }
        // `paladin_battalion::llm_failure::to_paladin_error` (not the legacy
        // `PaladinError::LlmError(source.to_string())` string erasure): a
        // scripted `error:` entry's `LlmErrorKind::Transient` must survive as
        // `Transience::Transient` so a scenario's Aegis retry policy under
        // the DEFAULT `TransientOnly` predicate can actually retry it (D-34's
        // E2E-3 dogfood scenario depends on this) -- the legacy erasure
        // classifies EVERY `PaladinError::LlmError(_)` as `Transience::
        // Unknown` (`paladin_core::platform::container::paladin_error::
        // PaladinError::transience`), which `TransientOnly` never retries.
        .map_err(|source| paladin_battalion::llm_failure::to_paladin_error(&source))?;

        Ok(PaladinResult {
            output: response.content,
            token_count: response.usage.total_tokens as u32,
            execution_time_ms: 0,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        let result = self.execute(paladin, input).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(Ok(PaladinStreamChunk {
                text: result.output,
                is_final: true,
                metadata: None,
            }))
            .await;
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn build_port(
    router: HashMap<String, ScenarioLlm>,
    options: RunOptions,
) -> Result<Arc<dyn PaladinPort>, RunnerError> {
    if options.live {
        let provider = resolve_live_provider()?;
        Ok(Arc::new(ScenarioPaladinPort::live(provider)))
    } else {
        Ok(Arc::new(ScenarioPaladinPort::scripted(router)))
    }
}

/// Build the [`ScenarioLlm`] router keyed by each Paladin node's OWN
/// `PaladinData.name` (D-31): the identity [`PaladinPort::execute`]
/// receives, mapped to a [`ScenarioLlm::for_node`] clone tagged with that
/// node's GRAPH id -- the identity a scenario's `llm.per_node` map is keyed
/// by (see e.g. `crates/paladin-battalion/tests/fixtures/graph_docs/linear.json`,
/// whose Paladin nodes are authored `name: "Start"` under graph node id
/// `"start"`, proving the two identities are not interchangeable).
fn build_llm_router(graph: &WarGraph, root: &ScenarioLlm) -> HashMap<String, ScenarioLlm> {
    let mut router = HashMap::new();
    for node_id in graph.node_order() {
        if let Some(NodeSpec::Paladin { paladin, .. }) = graph.node(node_id) {
            router.insert(paladin.node.name.clone(), root.for_node(node_id.as_str()));
        }
    }
    router
}

// ---------------------------------------------------------------------------
// CapturingSink -- the TraceSink whose records feed AssertionContext (D-29).
// ---------------------------------------------------------------------------

/// A [`TraceSink`] that captures every [`TraceRecord`] it receives, in
/// arrival order, for later evaluation against a case's assertions (D-29).
struct CapturingSink {
    records: Mutex<Vec<TraceRecord>>,
}

impl CapturingSink {
    fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    fn records(&self) -> Vec<TraceRecord> {
        self.records
            .lock()
            .expect("CapturingSink records lock poisoned")
            .clone()
    }
}

#[async_trait]
impl TraceSink for CapturingSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        self.records
            .lock()
            .expect("CapturingSink records lock poisoned")
            .push(record);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Engine driving helpers
// ---------------------------------------------------------------------------

async fn run_plain<W: WaypointPort + 'static>(
    store: Arc<W>,
    graph: &WarGraph,
    port: Arc<dyn PaladinPort>,
    thread: ThreadId,
    initial: StateDelta,
    parley_responses: &[ParleyScript],
) -> Result<(Vec<TraceRecord>, RunOutcome, Battlefield), RunnerError> {
    let sink: Arc<CapturingSink> = Arc::new(CapturingSink::new());
    let engine = WarEngine::new(port, Arc::clone(&store)).with_trace_sink(sink.clone());
    let outcome = engine.start(graph, thread.clone(), initial).await?;
    let outcome = drive_parleys(&engine, graph, &thread, outcome, parley_responses).await?;

    tokio::time::sleep(Duration::from_millis(50)).await;
    let records = sink.records();
    let final_state = resolve_battlefield(store.as_ref(), &thread, &outcome).await?;
    Ok((records, outcome, final_state))
}

/// Answer every Parley a run raises, in order, from `parley_responses`
/// (D-34) -- stopping WITHOUT error if fewer responses remain than the
/// current `AwaitingInput` has outstanding requests, so a scenario that
/// deliberately wants to assert `run_status: awaiting_input` (by supplying
/// no further responses) is a supported, non-error terminal state.
async fn drive_parleys<W: WaypointPort + 'static>(
    engine: &WarEngine<W>,
    graph: &WarGraph,
    thread: &ThreadId,
    mut outcome: RunOutcome,
    parley_responses: &[ParleyScript],
) -> Result<RunOutcome, RunnerError> {
    let mut idx = 0usize;
    while let RunOutcome::AwaitingInput { parleys, .. } = &outcome {
        if idx + parleys.len() > parley_responses.len() {
            break;
        }
        let mut responses = Vec::with_capacity(parleys.len());
        for request in parleys {
            let script = &parley_responses[idx];
            idx += 1;
            responses.push(ParleyResponse {
                parley_id: request.parley_id,
                kind: request.kind.clone(),
                prompt: request.prompt.clone(),
                value: script.value.clone(),
                responded_by: Some("paladin-eval".to_string()),
                responded_at: chrono::Utc::now(),
                defaulted: false,
            });
        }
        outcome = engine.resume_with(graph, thread.clone(), responses).await?;
    }
    Ok(outcome)
}

/// Resolve the [`Battlefield`] a terminal [`RunOutcome`] leaves behind:
/// `Completed` carries it inline, every other terminal outcome's Battlefield
/// is read back from its own persisted `Waypoint`.
async fn resolve_battlefield<W: WaypointPort>(
    store: &W,
    thread: &ThreadId,
    outcome: &RunOutcome,
) -> Result<Battlefield, RunnerError> {
    match outcome {
        RunOutcome::Completed { final_state, .. } => Ok(final_state.clone()),
        RunOutcome::Halted { waypoint } | RunOutcome::AwaitingInput { waypoint, .. } => {
            let loaded = store
                .get(thread, waypoint)
                .await?
                .ok_or(RunnerError::NoFinalState)?;
            Ok(loaded.battlefield)
        }
        RunOutcome::Failed {
            waypoint: Some(waypoint),
            ..
        } => {
            let loaded = store
                .get(thread, waypoint)
                .await?
                .ok_or(RunnerError::NoFinalState)?;
            Ok(loaded.battlefield)
        }
        RunOutcome::Failed { waypoint: None, .. } => Err(RunnerError::NoFinalState),
    }
}

async fn full_history<W: WaypointPort>(
    store: &W,
    thread: &ThreadId,
) -> Result<Vec<Waypoint>, RunnerError> {
    let summaries = store.history(thread, None, None).await?;
    let mut waypoints = Vec::with_capacity(summaries.len());
    for summary in summaries {
        if let Some(waypoint) = store.get(thread, &summary.waypoint_id).await? {
            waypoints.push(waypoint);
        }
    }
    Ok(waypoints)
}

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!("paladin-eval-{label}-{}.sqlite", Uuid::new_v4()));
    format!("sqlite://{}", path.display())
}

fn resolve_relative(scenario_path: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        scenario_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    }
}

fn load_graph_doc(path: &Path) -> Result<WarGraphDoc, RunnerError> {
    let contents = std::fs::read_to_string(path).map_err(|source| RunnerError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let is_yaml = matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml") | Some("yml")
    );
    let doc: WarGraphDoc = if is_yaml {
        serde_yaml::from_str(&contents).map_err(|source| RunnerError::ParseGraphDoc {
            path: path.to_path_buf(),
            message: source.to_string(),
        })?
    } else {
        serde_json::from_str(&contents).map_err(|source| RunnerError::ParseGraphDoc {
            path: path.to_path_buf(),
            message: source.to_string(),
        })?
    };
    Ok(doc)
}

fn snapshot_file_path(scenario_path: &Path, case_name: &str) -> PathBuf {
    let dir = scenario_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = scenario_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scenario");
    dir.join(format!("{stem}.{case_name}.snap.json"))
}

/// Render a final `Battlefield` the same way `--bless` blesses it: every
/// declared field's raw JSON value, by name. Deliberately duplicated from
/// `crate::assertion`'s private, identically-shaped helper rather than
/// reaching into that module (this plan's own `files_modified` does not
/// touch `assertion.rs`) -- both read exactly `Battlefield::schema().fields`
/// and `Battlefield::get_raw`, the same two public accessors, so the two
/// stay trivially in agreement.
fn battlefield_snapshot_value(battlefield: &Battlefield) -> Value {
    let mut obj = serde_json::Map::new();
    for field in &battlefield.schema().fields {
        if let Some(value) = battlefield.get_raw(&field.name) {
            obj.insert(field.name.as_str().to_string(), value.clone());
        }
    }
    Value::Object(obj)
}

// ---------------------------------------------------------------------------
// LiveMode (D-35)
// ---------------------------------------------------------------------------
//
// Promotion path: a scenario runs script-mocked, deterministic, and free
// (`ScenarioLlm`, the default) in CI on every `cargo test --test evals` and
// every plain `paladin-cli eval run`. Live mode is a DELIBERATE, three-times
// opt-in pre-release smoke check against a real provider -- never the
// default, never entered by accident, and never silently substituted when
// any one of the three gates below is missing.

/// The environment variable [`check_live_mode`] requires to be set (to any
/// non-blank value) alongside `--live` before live mode is entered (D-35).
/// No workflow in this repository ever sets it -- default CI never spends a
/// provider credit.
pub const PALADIN_EVAL_LIVE_ENV: &str = "PALADIN_EVAL_LIVE";

/// Why [`check_live_mode`] refused to enter live mode, or why
/// `resolve_live_provider` (private: the runner's own internal resolution
/// step) could not build a real provider once the gate passed (D-35). Never
/// converted into an implicit fallback to scripted mocks -- every variant
/// here is surfaced to the caller as a refusal.
#[derive(Debug, Error)]
pub enum LiveModeError {
    /// `--live` (or the equivalent `RunOptions::live`) was not set.
    #[error("live mode requires --live")]
    FlagNotSet,
    /// [`PALADIN_EVAL_LIVE_ENV`] is unset or blank.
    #[error(
        "live mode requires the {PALADIN_EVAL_LIVE_ENV} environment variable to be set to a \
         non-blank value (deliberately never set by any CI workflow in this repository)"
    )]
    EnvNotSet,
    /// No provider compiled into this build of `paladin-eval` has a
    /// configured credential (ADR-0012): `paladin_llm::provider_factory::
    /// LlmProviderFactory::get_default_provider()` returned `None`.
    #[error(
        "live mode requires a configured provider credential; none of paladin-llm's compiled-in \
         providers has one set (see ADR-0012)"
    )]
    NoProviderKey,
    /// The gate passed, but this build of `paladin-eval` was not compiled
    /// with its own `live` feature (which enables real `paladin-llm`
    /// provider adapters) -- refuses rather than silently falling back to a
    /// scripted mock.
    #[error(
        "live mode's gate passed, but this build of paladin-eval was not compiled with its own \
         `live` feature (enables real paladin-llm provider adapters); rebuild with \
         `--features live`"
    )]
    FeatureNotCompiled,
    /// The gate passed and the `live` feature is compiled in, but
    /// constructing the resolved provider adapter itself failed.
    #[error("failed to construct the live provider: {0}")]
    ProviderConstruction(String),
}

/// The three-way live-mode gate (D-35): `--live` AND
/// [`PALADIN_EVAL_LIVE_ENV`] AND a configured provider credential, ALL
/// three, or [`LiveModeError`] naming exactly which is missing. Called by
/// [`ScenarioRunner::run_case`] itself whenever `options.live` is `true` --
/// never only by a caller that remembers to check first.
pub fn check_live_mode(live_flag: bool) -> Result<(), LiveModeError> {
    if !live_flag {
        return Err(LiveModeError::FlagNotSet);
    }
    let env_set = std::env::var(PALADIN_EVAL_LIVE_ENV)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !env_set {
        return Err(LiveModeError::EnvNotSet);
    }
    if paladin_llm::provider_factory::LlmProviderFactory::get_default_provider().is_none() {
        return Err(LiveModeError::NoProviderKey);
    }
    Ok(())
}

/// Whether `assertion` is content-bearing (D-35): compares rendered LLM
/// output rather than structural execution facts. Skipped in live mode
/// unless the scenario's own `live.allow_content_assertions` opts in --
/// every other assertion kind (route, status, node counts, edges, limits,
/// Parley) is structural and always runs in live mode regardless.
fn is_content_assertion(assertion: &Assertion) -> bool {
    matches!(
        assertion,
        Assertion::FinalStateFieldEquals { .. }
            | Assertion::FinalStateFieldMatches { .. }
            | Assertion::FieldJsonPathEquals { .. }
            | Assertion::FinalStateSnapshot
    )
}

/// The pure skip-decision [`ScenarioRunner::run_case`]'s assertion loop
/// applies (D-35): `true` only when the run is live, `assertion` is
/// content-bearing, and the scenario did not opt in. A structural assertion
/// (`is_content_assertion` false) is never skipped regardless of `live` --
/// it always evaluates, live or scripted.
fn should_skip_for_live(assertion: &Assertion, live: bool, allow_content_assertions: bool) -> bool {
    live && is_content_assertion(assertion) && !allow_content_assertions
}

/// Resolve a real provider once [`check_live_mode`] has already passed
/// (D-35): live mode uses `paladin_llm::provider_factory`'s first available
/// (configured-credential) provider for the whole run. Compiled only when
/// this crate's own `live` feature is enabled -- see
/// [`LiveModeError::FeatureNotCompiled`] for the refusal a build without it
/// gives instead.
#[cfg(feature = "live")]
fn resolve_live_provider() -> Result<Arc<dyn LlmPort>, LiveModeError> {
    let provider_name = paladin_llm::provider_factory::LlmProviderFactory::get_default_provider()
        .ok_or(LiveModeError::NoProviderKey)?;
    paladin_llm::provider_factory::LlmProviderFactory::new()
        .create(&provider_name)
        .map_err(|source| LiveModeError::ProviderConstruction(source.to_string()))
}

/// See the `#[cfg(feature = "live")]` sibling above -- this build was not
/// compiled with `paladin-eval`'s own `live` feature, so live mode's gate
/// may pass (a credential IS configured) while this crate still cannot
/// build the real adapter itself.
#[cfg(not(feature = "live"))]
fn resolve_live_provider() -> Result<Arc<dyn LlmPort>, LiveModeError> {
    Err(LiveModeError::FeatureNotCompiled)
}

// ---------------------------------------------------------------------------
// RunnerError
// ---------------------------------------------------------------------------

/// Errors [`ScenarioRunner::run_case`] can encounter resolving, compiling or
/// running a case -- always folded into [`CaseOutcome::Errored`] by
/// [`ScenarioRunner::run_case`] itself, never propagated as a panic.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// A file (a `graph_doc` target, or a snapshot write) could not be read
    /// or written.
    #[error("failed to access {path}: {source}")]
    Io {
        /// The file that could not be accessed.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A `graph_doc` target's file did not deserialize as a `WarGraphDoc`.
    #[error("failed to parse WarGraphDoc {path}: {message}")]
    ParseGraphDoc {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying parse error's message.
        message: String,
    },
    /// A `graph_doc` target's document failed `WarGraphDoc::compile`.
    #[error("failed to compile WarGraphDoc {path}: {source}")]
    Compile {
        /// The file that failed to compile.
        path: PathBuf,
        /// The underlying compile error. Boxed (`clippy::result_large_err`):
        /// `CompileError`'s largest variant is well over the lint's
        /// threshold, and every `Result<_, RunnerError>` in this module pays
        /// that size on its happy path too if left unboxed.
        #[source]
        source: Box<paladin_battalion::engine::graph_doc::CompileError>,
    },
    /// A `registered` target names a [`GraphConstructor`] no
    /// [`ScenarioRunner::register_graph`] call registered.
    #[error("scenario names unregistered graph constructor {name:?}; registered: {registered:?}")]
    UnknownGraph {
        /// The unresolved name.
        name: String,
        /// Every name this runner does have registered.
        registered: Vec<String>,
    },
    /// A scenario's `registries` field names a [`RegistriesFactory`] no
    /// [`ScenarioRunner::register_registries`] call registered.
    #[error("scenario names unregistered registries {name:?}; registered: {registered:?}")]
    UnknownRegistries {
        /// The unresolved name.
        name: String,
        /// Every name this runner does have registered.
        registered: Vec<String>,
    },
    /// `interrupt_after_superstep` was set on a case whose scenario does not
    /// use `store: sqlite_temp`.
    #[error("case {case:?}: interrupt_after_superstep requires store: sqlite_temp")]
    InterruptRequiresSqliteTemp {
        /// The offending case's name.
        case: String,
    },
    /// The `WaypointPort` backing this run reported an error.
    #[error("waypoint store error: {0}")]
    Waypoint(#[from] WaypointError),
    /// The engine itself reported an error (never a `RunOutcome::Failed`,
    /// which is a normal terminal outcome an assertion can check).
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),
    /// A case's `input` map named a field whose name is not a valid
    /// `FieldName`.
    #[error("invalid field name {field:?}: {message}")]
    InvalidField {
        /// The offending field name.
        field: String,
        /// The underlying validation error's message.
        message: String,
    },
    /// A terminal `RunOutcome` left no persisted Waypoint to read the final
    /// `Battlefield` back from.
    #[error("run finished with no persisted final state to evaluate assertions against")]
    NoFinalState,
    /// The generated `ThreadId` for this run failed validation.
    #[error("invalid thread id: {0}")]
    InvalidThreadId(String),
    /// `options.live` was set but the three-way live-mode gate refused
    /// (D-35) -- never a silent fallback to scripted mocks.
    #[error("live mode: {0}")]
    Live(#[from] LiveModeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{
        Case as ScenarioCase, LlmScript, RunStatusValue, Scenario as ScenarioDoc, ScenarioTarget,
        ScriptEntry, StoreKind as ScenarioStoreKind, Times,
    };
    use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits};
    use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
    use paladin_battalion::engine::{InputMapping, NodeSpec as EngineNodeSpec};
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldSpec,
    };
    use paladin_core::platform::container::directive::Directive;
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
    use paladin_core::platform::container::waypoint::NodeId;
    use std::collections::BTreeMap;

    fn field(name: &str) -> FieldName {
        FieldName::new(name).expect("valid field name")
    }

    fn make_paladin(name: &str, model: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("{name} prompt"),
            name: name.to_string(),
            user_name: String::new(),
            model: model.to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };
        Node::new(data, Some(name.to_string()))
    }

    fn minimal_scenario(
        target: ScenarioTarget,
        llm: LlmScript,
        assertions: Vec<Assertion>,
    ) -> ScenarioDoc {
        ScenarioDoc {
            schema_version: crate::scenario::EVAL_SCHEMA_VERSION.to_string(),
            target,
            store: ScenarioStoreKind::InMemory,
            llm,
            cases: vec![ScenarioCase {
                name: "happy_path".to_string(),
                input: BTreeMap::from([("topic".to_string(), serde_json::json!("widgets"))]),
                interrupt_after_superstep: None,
                parley_responses: vec![],
                llm: None,
                assertions,
            }],
            live: Default::default(),
            registries: None,
        }
    }

    fn linear_fixture_path() -> PathBuf {
        // Cross-crate fixture reuse (D-31 note in the module doc): the exact
        // document `crates/paladin-battalion`'s own tests compile, proving a
        // `graph_doc` target's relative-path resolution works against a
        // fixture this crate does not own a copy of.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../paladin-battalion/tests/fixtures/graph_docs/linear.json")
    }

    #[tokio::test]
    async fn runner_executes_a_graph_doc_case() {
        let scenario = minimal_scenario(
            ScenarioTarget::GraphDoc {
                graph_doc: PathBuf::from(
                    "../paladin-battalion/tests/fixtures/graph_docs/linear.json",
                ),
            },
            LlmScript {
                global: vec![
                    ScriptEntry::Text("s1".to_string()),
                    ScriptEntry::Text("s2".to_string()),
                    ScriptEntry::Text("s3".to_string()),
                ],
                ..Default::default()
            },
            vec![
                Assertion::NodeExecuted {
                    node: "start".to_string(),
                    times: Times::Exact(1),
                },
                Assertion::RunStatus(RunStatusValue::Completed),
            ],
        );

        let runner = ScenarioRunner::new();
        // The scenario's own `target.graph_doc` path is authored relative to
        // this crate's own manifest dir (a stand-in "scenario file" location)
        // -- `linear_fixture_path()` below proves the SAME resolved path.
        let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenario.eval.yaml");
        let report = runner
            .run_case(
                &scenario_path,
                &scenario,
                &scenario.cases[0],
                RunOptions::default(),
            )
            .await;

        assert!(
            report.passed(),
            "expected the case to pass: {}",
            report.render_failures()
        );
        assert!(linear_fixture_path().exists(), "sanity: fixture must exist");
    }

    #[tokio::test]
    async fn runner_reports_a_failing_assertion_with_its_rendering() {
        let scenario = minimal_scenario(
            ScenarioTarget::GraphDoc {
                graph_doc: PathBuf::from(
                    "../paladin-battalion/tests/fixtures/graph_docs/linear.json",
                ),
            },
            LlmScript {
                global: vec![
                    ScriptEntry::Text("s1".to_string()),
                    ScriptEntry::Text("s2".to_string()),
                    ScriptEntry::Text("s3".to_string()),
                ],
                ..Default::default()
            },
            vec![Assertion::NodeExecuted {
                node: "start".to_string(),
                times: Times::Exact(5),
            }],
        );

        let runner = ScenarioRunner::new();
        let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenario.eval.yaml");
        let report = runner
            .run_case(
                &scenario_path,
                &scenario,
                &scenario.cases[0],
                RunOptions::default(),
            )
            .await;

        assert!(!report.passed());
        let rendered = report.render_failures();
        assert!(
            rendered.contains("expected: exactly 5"),
            "rendering must be the assertion's own render_failure() output: {rendered:?}"
        );
    }

    #[tokio::test]
    async fn runner_executes_a_registered_target() {
        let mut runner = ScenarioRunner::new();
        runner.register_graph(
            "solo",
            Arc::new(|ports: &ScriptedPorts| {
                let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                    field("out"),
                    DispatchRule::LastWrite,
                    None,
                    false,
                )]);
                let mut graph = WarGraph::new(schema, EngineLimits::default());
                let node = NodeId::new("solo");
                let llm = ports.for_node("solo");
                graph.add_node(
                    node.clone(),
                    EngineNodeSpec::Function(Arc::new(RegisteredEchoNode {
                        llm,
                        output_field: field("out"),
                    })),
                );
                graph.add_entry(node);
                graph
            }),
        );

        let mut scenario = minimal_scenario(
            ScenarioTarget::Registered {
                registered: "solo".to_string(),
            },
            LlmScript {
                per_node: BTreeMap::from([(
                    "solo".to_string(),
                    vec![ScriptEntry::Text("registered-output".to_string())],
                )]),
                ..Default::default()
            },
            vec![Assertion::FinalStateFieldEquals {
                field: "out".to_string(),
                value: serde_json::json!("registered-output"),
            }],
        );
        // The "solo" graph's own schema declares only `out` -- unlike
        // `minimal_scenario`'s default `{"topic": "widgets"}` input, which
        // targets the `linear.json` fixture's schema instead.
        scenario.cases[0].input = BTreeMap::new();

        let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenario.eval.yaml");
        let report = runner
            .run_case(
                &scenario_path,
                &scenario,
                &scenario.cases[0],
                RunOptions::default(),
            )
            .await;
        assert!(
            report.passed(),
            "expected the registered target's Function node to receive the scripted port: {}",
            report.render_failures()
        );
    }

    struct RegisteredEchoNode {
        llm: ScenarioLlm,
        output_field: FieldName,
    }

    #[async_trait::async_trait]
    impl StateNode for RegisteredEchoNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, StateNodeError> {
            let request = LlmRequest::new(
                "scenario-model",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "go".to_string(),
                    context: None,
                }))
                .expect("valid prompt"),
            );
            let response = self
                .llm
                .generate(request)
                .await
                .map_err(|e| StateNodeError(e.to_string()))?;
            let mut delta = StateDelta::new();
            delta
                .set(self.output_field.clone(), response.content)
                .map_err(|e| StateNodeError(e.to_string()))?;
            Ok(delta.into())
        }
    }

    #[tokio::test]
    async fn interrupt_and_resume_reproduces_the_control_run() {
        // A two-Paladin-node chain with PER-NODE scripts (see this module's
        // own doc comment on the global-vs-per-node caveat).
        fn build_graph() -> WarGraph {
            let schema = BattlefieldSchema::new(vec![
                FieldSpec::new(
                    field("topic"),
                    DispatchRule::LastWrite,
                    Some(serde_json::json!("widgets")),
                    false,
                ),
                FieldSpec::new(field("step_one"), DispatchRule::LastWrite, None, false),
                FieldSpec::new(field("step_two"), DispatchRule::LastWrite, None, false),
            ]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let first = NodeId::new("first");
            let second = NodeId::new("second");
            graph.add_node(
                first.clone(),
                EngineNodeSpec::paladin(
                    make_paladin("First", "gpt-4"),
                    InputMapping::new("{topic}"),
                    field("step_one"),
                ),
            );
            graph.add_node(
                second.clone(),
                EngineNodeSpec::paladin(
                    make_paladin("Second", "gpt-4"),
                    InputMapping::new("{step_one}"),
                    field("step_two"),
                ),
            );
            graph.add_edge(EdgeSpec {
                from: first.clone(),
                to: second,
                condition: None,
            });
            graph.add_entry(first);
            graph
        }

        let mut runner = ScenarioRunner::new();
        runner.register_graph("chain", Arc::new(|_ports: &ScriptedPorts| build_graph()));

        let llm = LlmScript {
            per_node: BTreeMap::from([
                (
                    "first".to_string(),
                    vec![ScriptEntry::Text("one".to_string())],
                ),
                (
                    "second".to_string(),
                    vec![ScriptEntry::Text("two".to_string())],
                ),
            ]),
            ..Default::default()
        };

        let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenario.eval.yaml");

        // Control: no interruption.
        let control_scenario = ScenarioDoc {
            schema_version: crate::scenario::EVAL_SCHEMA_VERSION.to_string(),
            target: ScenarioTarget::Registered {
                registered: "chain".to_string(),
            },
            store: ScenarioStoreKind::SqliteTemp,
            llm: llm.clone(),
            cases: vec![ScenarioCase {
                name: "control".to_string(),
                input: BTreeMap::new(),
                interrupt_after_superstep: None,
                parley_responses: vec![],
                llm: None,
                assertions: vec![],
            }],
            live: Default::default(),
            registries: None,
        };
        let (control_graph, control_router) = runner
            .resolve_target(&scenario_path, &control_scenario)
            .expect("resolve control target");
        let control_port =
            build_port(control_router, RunOptions::default()).expect("scripted port");
        let store = Arc::new(InMemoryWaypointStore::new());
        let thread = ThreadId::new(format!("control-{}", Uuid::new_v4())).expect("valid thread id");
        let (_records, _outcome, control_final_state) = run_plain(
            store,
            &control_graph,
            control_port,
            thread,
            StateDelta::new(),
            &[],
        )
        .await
        .expect("control run succeeds");

        // Interrupted: crash after superstep 1 (only `first` has completed).
        let interrupted_scenario = ScenarioDoc {
            store: ScenarioStoreKind::SqliteTemp,
            cases: vec![ScenarioCase {
                name: "interrupted".to_string(),
                interrupt_after_superstep: Some(1),
                ..control_scenario.cases[0].clone()
            }],
            ..control_scenario
        };
        let report = runner
            .run_case(
                &scenario_path,
                &interrupted_scenario,
                &interrupted_scenario.cases[0],
                RunOptions::default(),
            )
            .await;

        match &report.outcome {
            CaseOutcome::Ran { final_state, .. } => {
                assert_eq!(
                    *final_state, control_final_state,
                    "resumed final Battlefield must equal the control run's"
                );
            }
            CaseOutcome::Errored(message) => panic!("interrupted case errored: {message}"),
        }
    }

    #[test]
    fn evals_target_discovers_one_trial_per_case() {
        let dir = std::env::temp_dir().join(format!("paladin-eval-trials-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        // Deliberately `alpha.yaml`/`beta.yaml`, not the `.eval.yaml` double
        // extension the real `evals/` tree uses: `Path::file_stem` only
        // strips the LAST extension, so a `<name>.eval.yaml` file's own
        // trial-name stem would be `<name>.eval` -- a single extension here
        // keeps this test's expected names exactly `alpha`/`beta`.
        std::fs::write(
            dir.join("alpha.yaml"),
            r#"
schema_version: "1"
target:
  registered: "nowhere"
cases:
  - name: case_one
    assertions: []
  - name: case_two
    assertions: []
"#,
        )
        .expect("write alpha fixture");
        std::fs::write(
            dir.join("beta.yaml"),
            r#"
schema_version: "1"
target:
  registered: "nowhere"
cases:
  - name: only_case
    assertions: []
"#,
        )
        .expect("write beta fixture");

        let runner = ScenarioRunner::new();
        let pattern = dir.join("*.yaml");
        let trials = runner.trials(pattern.to_str().expect("valid utf8 path"));

        assert_eq!(trials.len(), 3, "one trial per (file, case) pair");
        let names: Vec<&str> = trials.iter().map(Trial::name).collect();
        assert!(names.contains(&"alpha::case_one"));
        assert!(names.contains(&"alpha::case_two"));
        assert!(names.contains(&"beta::only_case"));

        let filter = Arguments {
            filter: Some("case_one".to_string()),
            ..Default::default()
        };
        let selected: Vec<&Trial> = trials
            .iter()
            .filter(|t| !filter.is_filtered_out(t))
            .collect();
        assert_eq!(selected.len(), 1, "a name filter must select a single case");
        assert_eq!(selected[0].name(), "alpha::case_one");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // -- Task 3: the LiveMode gate (D-35) -----------------------------------
    //
    // These exercise only the GATE and CLASSIFICATION logic, never a real
    // provider call: `live_mode_requires_flag_and_env_and_keys` is the only
    // test in this module that reads/writes `PALADIN_EVAL_LIVE_ENV`, so it
    // owns that env var exclusively (no other test depends on its value) --
    // the `unsafe` blocks are edition-2024's `std::env::set_var`/`remove_var`
    // requirement, not a signal these mutations are broadly unsafe here.

    #[test]
    fn live_mode_requires_flag_and_env_and_keys() {
        // SAFETY: this is the ONLY test in this module (or crate) reading or
        // writing PALADIN_EVAL_LIVE_ENV; no other test's outcome depends on
        // its value, so this process-wide mutation cannot race a concurrent
        // test's own assertion.
        unsafe {
            std::env::remove_var(PALADIN_EVAL_LIVE_ENV);
        }

        assert!(matches!(
            check_live_mode(false),
            Err(LiveModeError::FlagNotSet)
        ));
        assert!(matches!(
            check_live_mode(true),
            Err(LiveModeError::EnvNotSet)
        ));

        // SAFETY: see above.
        unsafe {
            std::env::set_var(PALADIN_EVAL_LIVE_ENV, "1");
        }
        // This crate's own `paladin-llm` edge is `default-features = false,
        // features = ["mock"]` in a default (non-`live`-featured) test
        // build, so `LlmProviderFactory::get_default_provider()` finds no
        // compiled-in provider regardless of any real credential env var --
        // NoProviderKey is the gate's own, correct refusal here, per
        // ADR-0012 (never falls back to a mock).
        assert!(matches!(
            check_live_mode(true),
            Err(LiveModeError::NoProviderKey)
        ));

        // SAFETY: see above.
        unsafe {
            std::env::remove_var(PALADIN_EVAL_LIVE_ENV);
        }
    }

    fn content_assertion_fixtures() -> Vec<Assertion> {
        vec![
            Assertion::FinalStateFieldEquals {
                field: "status".to_string(),
                value: serde_json::json!("done"),
            },
            Assertion::FinalStateFieldMatches {
                field: "status".to_string(),
                pattern: "^do".to_string(),
            },
            Assertion::FieldJsonPathEquals {
                path: "/status".to_string(),
                value: serde_json::json!("done"),
            },
            Assertion::FinalStateSnapshot,
        ]
    }

    fn structural_assertion_fixtures() -> Vec<Assertion> {
        vec![
            Assertion::NodeExecuted {
                node: "worker".to_string(),
                times: Times::Exact(1),
            },
            Assertion::NodeNotExecuted {
                node: "idle".to_string(),
            },
            Assertion::EdgeFired {
                from: "a".to_string(),
                to: "b".to_string(),
            },
            Assertion::RouteTaken(vec!["a".to_string()]),
            Assertion::RunStatus(RunStatusValue::Completed),
            Assertion::TotalTokensMax(10),
            Assertion::SuperstepsMax(10),
            Assertion::ParleyRaised {
                kind: "approval".to_string(),
                node: "worker".to_string(),
            },
        ]
    }

    #[test]
    fn content_assertions_are_skipped_in_live_mode_without_opt_in() {
        for assertion in content_assertion_fixtures() {
            assert!(
                should_skip_for_live(&assertion, true, false),
                "{assertion:?} must be skipped in live mode without the scenario's opt-in"
            );
        }
    }

    #[test]
    fn content_assertions_run_in_live_mode_with_opt_in() {
        for assertion in content_assertion_fixtures() {
            assert!(
                !should_skip_for_live(&assertion, true, true),
                "{assertion:?} must run in live mode once the scenario opts in"
            );
        }
    }

    #[test]
    fn structural_assertions_always_run_in_live_mode() {
        for assertion in structural_assertion_fixtures() {
            assert!(
                !should_skip_for_live(&assertion, true, false),
                "{assertion:?} is structural and must always run in live mode"
            );
            assert!(
                !should_skip_for_live(&assertion, true, true),
                "{assertion:?} is structural and must always run in live mode, opt-in or not"
            );
        }
        // Sanity: never skipped outside live mode either, regardless of kind.
        for assertion in content_assertion_fixtures()
            .into_iter()
            .chain(structural_assertion_fixtures())
        {
            assert!(!should_skip_for_live(&assertion, false, false));
        }
    }

    #[tokio::test]
    async fn default_test_run_never_enters_live_mode() {
        // A plain RunOptions::default() (`live: false`, matching every
        // eval_scenarios! harness trial) must run entirely on scripted
        // mocks -- proven by SUCCEEDING here with no PALADIN_EVAL_LIVE_ENV
        // set and no provider credential available at all: if this code
        // path ever silently entered live mode, `check_live_mode` would
        // refuse (no env var, no key) and this case would error instead of
        // passing.
        let scenario = minimal_scenario(
            ScenarioTarget::GraphDoc {
                graph_doc: PathBuf::from(
                    "../paladin-battalion/tests/fixtures/graph_docs/linear.json",
                ),
            },
            LlmScript {
                global: vec![
                    ScriptEntry::Text("s1".to_string()),
                    ScriptEntry::Text("s2".to_string()),
                    ScriptEntry::Text("s3".to_string()),
                ],
                ..Default::default()
            },
            vec![Assertion::RunStatus(RunStatusValue::Completed)],
        );

        let runner = ScenarioRunner::new();
        let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenario.eval.yaml");
        assert!(
            !RunOptions::default().live,
            "the harness's own default options must never carry live: true"
        );
        let report = runner
            .run_case(
                &scenario_path,
                &scenario,
                &scenario.cases[0],
                RunOptions::default(),
            )
            .await;
        assert!(
            report.passed(),
            "a plain (non-live) run must succeed entirely on scripted mocks: {}",
            report.render_failures()
        );
    }
}
