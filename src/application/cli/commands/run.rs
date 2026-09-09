//! `paladin-cli run export` -- render a thread's execution overlay (D-21,
//! D-22, D-23, OBS-FR-09).
//!
//! Resolves the thread from `--thread` or, when `--run <run_id>` is given
//! instead, from the run row itself (`--run` is sugar that derives both the
//! thread AND the graph, D-23). The overlay prefers persisted trace rows
//! ([`RunTracePort::read`], exact fired AND evaluated edges) and falls back
//! to Waypoint history ([`WaypointPort::history`], always available, derived
//! fired edges only) when no trace was persisted for the thread.
//!
//! The graph shape follows D-22's locked resolution order: an explicit
//! `--graph <FILE>` document, then the run's assistant version's document
//! (only reachable when the thread belongs to a run), then
//! [`GraphShape::observed`] -- the observed-only fallback, rendered with the
//! locked title, so a thread with no static graph document still answers
//! "which branch fired" from what was actually seen.
//!
//! Every lookup reaches storage through a port (`RunRepositoryPort`,
//! `AssistantRepositoryPort`, `RunTracePort`, `WaypointPort`), over the store
//! the loaded `RunStoreConfig`/`WaypointStoreConfig` name -- never over HTTP
//! (ADR-0023).

use std::path::PathBuf;
use std::sync::Arc;

use paladin_battalion::engine::export::{
    ExecutionOverlay, GraphShape, OverlaySource, to_mermaid_overlay,
};
use paladin_battalion::engine::graph_doc::WarGraphDoc;
use paladin_core::platform::container::assistant::{AssistantId, AssistantKind};
use paladin_core::platform::container::run::{Run, RunId};
use paladin_core::platform::container::trace::TraceRecord;
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::waypoint_port::WaypointPort;

use super::graph::{build_assistant_repository, load_graph_doc_file, write_output};
use crate::application::cli::error::CliError;
use crate::config::env_utils::EnvOverridable;
use crate::config::run_store::{RunStoreBackend, RunStoreConfig};
use crate::config::waypoint_store::{WaypointStoreBackend, WaypointStoreConfig};

/// A page size bounding every `RunTracePort::read`/history walk this module
/// performs, so a very long thread is never requested unbounded in one call
/// (mirrors `WaypointPort::prune_thread`'s own precedent).
const PAGE_SIZE: u32 = 500;

/// `paladin-cli run` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum RunCommands {
    /// Render a thread's execution overlay as an annotated diagram.
    Export(RunExportArgs),
}

/// Arguments for `paladin-cli run export`.
#[derive(Debug, clap::Args)]
pub struct RunExportArgs {
    /// The thread to render. Required unless `--run` is given (which
    /// derives it from the run row).
    #[arg(long, value_parser = parse_thread_id_arg)]
    pub thread: Option<ThreadId>,
    /// Render the overlay as of this Waypoint rather than the whole history.
    #[arg(long, value_parser = parse_waypoint_id_arg)]
    pub waypoint: Option<WaypointId>,
    /// Derive the thread and (absent `--graph`) the graph document from this
    /// run's row.
    #[arg(long, value_parser = parse_run_id_arg)]
    pub run: Option<RunId>,
    /// An explicit `WarGraphDoc` JSON/YAML file, taking precedence over any
    /// run-derived graph (D-22's first resolution bucket).
    #[arg(long)]
    pub graph: Option<PathBuf>,
    /// Write the diagram to this path instead of stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

fn parse_thread_id_arg(s: &str) -> Result<ThreadId, String> {
    ThreadId::new(s).map_err(|e| e.to_string())
}

/// `WaypointId` is `#[serde(transparent)]` over a `Uuid` with no public
/// `from_uuid`/`parse` constructor (core type, ADR-0016) -- round-tripping
/// through a JSON string value reuses its own `Deserialize` impl, mirroring
/// `paladin-web`'s `thread_controller.rs::parse_waypoint_id`.
fn parse_waypoint_id_arg(s: &str) -> Result<WaypointId, String> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|_| format!("invalid waypoint id: '{s}'"))
}

fn parse_run_id_arg(s: &str) -> Result<RunId, String> {
    RunId::parse(s).map_err(|e| e.to_string())
}

/// Where [`render_run_export`] resolved the graph shape from (D-22),
/// rendered alongside the diagram so a reader knows whether the edges shown
/// are against the real graph or only what was observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphResolution {
    /// An explicit `--graph <FILE>` document.
    Explicit(PathBuf),
    /// The run's assistant version's document.
    AssistantVersion {
        /// The resolved assistant id.
        assistant_id: String,
        /// The resolved assistant version.
        version: u32,
    },
    /// No static graph document was available (D-22's third bucket):
    /// [`GraphShape::observed`] was used instead.
    ObservedOnly,
}

/// The report [`render_run_export`] produces: the resolved overlay source,
/// the resolved graph, and the rendered diagram -- annotated so a reader
/// knows plainly whether the edge data is exact or derived, and whether the
/// shape is the real graph or only what was observed (T-28-13-04).
#[derive(Debug, Clone, PartialEq)]
pub struct RunExportReport {
    /// Where the overlay's data came from.
    pub overlay_source: OverlaySource,
    /// Where the graph shape came from.
    pub resolution: GraphResolution,
    /// The exact `to_mermaid_overlay` output.
    pub diagram: String,
}

impl RunExportReport {
    /// Render the source/resolution annotation followed by the diagram --
    /// the exact text `run_run_export` writes to stdout (or `--out`).
    pub fn render(&self) -> String {
        let source_line = match self.overlay_source {
            OverlaySource::Waypoints => {
                "source: waypoints (always-available history, derived fired edges, no \
                 evaluated edges)"
                    .to_string()
            }
            OverlaySource::Trace => {
                "source: trace (persisted, exact fired and evaluated edges)".to_string()
            }
        };
        let resolution_line = match &self.resolution {
            GraphResolution::Explicit(path) => {
                format!("graph: {} (explicit --graph)", path.display())
            }
            GraphResolution::AssistantVersion {
                assistant_id,
                version,
            } => format!("graph: assistant '{assistant_id}' version {version} (from the run)"),
            GraphResolution::ObservedOnly => {
                "graph: observed-only (no graph document available)".to_string()
            }
        };
        format!("{source_line}\n{resolution_line}\n{}", self.diagram)
    }
}

/// `paladin-cli run export --thread <id> [--waypoint <id>] [--run <run_id>]
/// [--graph <FILE>] [--out <path>]` (D-23): renders the thread's execution
/// overlay, annotated with its resolved source and graph, to `--out` or
/// stdout with no colour.
pub async fn run_run_export(
    thread: Option<ThreadId>,
    waypoint: Option<WaypointId>,
    run: Option<RunId>,
    graph: Option<PathBuf>,
    out: Option<PathBuf>,
) -> Result<(), CliError> {
    let report = render_run_export(thread, waypoint, run, graph).await?;
    write_output(&report.render(), out).await
}

/// The testable core behind [`run_run_export`]: resolves the thread, the
/// overlay and the graph shape, and renders the diagram, returning the full
/// [`RunExportReport`] without touching stdout or a file.
pub async fn render_run_export(
    thread: Option<ThreadId>,
    waypoint: Option<WaypointId>,
    run: Option<RunId>,
    graph: Option<PathBuf>,
) -> Result<RunExportReport, CliError> {
    let run_row = resolve_run_row(run.as_ref()).await?;
    let thread_id = resolve_thread_id(thread, run_row.as_ref())?;

    // D-22 resolution order, buckets 1 and 2: an explicit file first, then
    // the run's assistant version's document -- resolved BEFORE the overlay,
    // since a real shape (when one exists) is what `ExecutionOverlay::
    // from_waypoints`'s fired-edge derivation validates candidate edges
    // against.
    let real_doc = resolve_real_graph_document(graph, run_row.as_ref()).await?;
    let provisional_shape = real_doc
        .as_ref()
        .map(|(doc, _)| GraphShape::from_doc(doc))
        .unwrap_or(GraphShape {
            nodes: Vec::new(),
            edges: Vec::new(),
            entry: Vec::new(),
        });

    let mut overlay = build_overlay(&thread_id, waypoint, &provisional_shape).await?;

    let (shape, resolution) = match real_doc {
        Some((doc, resolution)) => (GraphShape::from_doc(&doc), resolution),
        None => {
            // D-22 bucket 3: no static graph document is available for this
            // thread -- the observed subgraph, built from exactly what the
            // overlay saw, with the locked observed-only title.
            overlay.observed_only = true;
            (
                GraphShape::observed(&overlay),
                GraphResolution::ObservedOnly,
            )
        }
    };

    let diagram = to_mermaid_overlay(&shape, &overlay);

    Ok(RunExportReport {
        overlay_source: overlay.source,
        resolution,
        diagram,
    })
}

/// Load the run row named by `--run`, if any. `Ok(None)` when `--run` was
/// not given -- an unknown run id is a clear, naming error, never a silent
/// `None`.
async fn resolve_run_row(run: Option<&RunId>) -> Result<Option<Run>, CliError> {
    let Some(run_id) = run else {
        return Ok(None);
    };
    let repository = build_run_repository().await?;
    let row = repository
        .get(run_id)
        .await
        .map_err(|e| CliError::execution(format!("run store error: {e}")))?
        .ok_or_else(|| CliError::execution(format!("unknown run: '{run_id}'")))?;
    Ok(Some(row))
}

/// Resolve the thread to render: `--thread` directly, or (absent it) the
/// resolved run row's own thread (`--run` sugar, D-23). Neither given is a
/// clear, actionable error.
fn resolve_thread_id(
    thread: Option<ThreadId>,
    run_row: Option<&Run>,
) -> Result<ThreadId, CliError> {
    match (thread, run_row) {
        (Some(thread_id), _) => Ok(thread_id),
        (None, Some(row)) => Ok(row.thread_id.clone()),
        (None, None) => Err(CliError::invalid_argument(
            "one of --thread or --run is required",
        )),
    }
}

/// D-22 resolution buckets 1 and 2: an explicit `--graph <FILE>` document
/// takes precedence; absent it, the resolved run row's assistant version's
/// `Workflow` document is used (an `Agent` assistant, or no run row at all,
/// falls through to `None` -- the caller's observed-only bucket 3).
async fn resolve_real_graph_document(
    graph: Option<PathBuf>,
    run_row: Option<&Run>,
) -> Result<Option<(WarGraphDoc, GraphResolution)>, CliError> {
    if let Some(path) = graph {
        let doc = load_graph_doc_file(&path).await?;
        return Ok(Some((doc, GraphResolution::Explicit(path))));
    }

    let Some(row) = run_row else {
        return Ok(None);
    };

    let assistant_id = AssistantId::new(row.assistant.assistant_id.clone()).map_err(|e| {
        CliError::execution(format!(
            "run '{}' names an invalid assistant id '{}': {e}",
            row.run_id, row.assistant.assistant_id
        ))
    })?;
    let repository = build_assistant_repository().await?;
    let version = repository
        .get_version(&assistant_id, row.assistant.version)
        .await
        .map_err(|e| CliError::execution(format!("assistant store error: {e}")))?;

    let Some(version) = version else {
        return Ok(None);
    };
    if version.definition.kind != AssistantKind::Workflow {
        // An Agent-kind assistant has no graph document -- not an error,
        // just nothing for this bucket to resolve; the caller falls through
        // to observed-only.
        return Ok(None);
    }

    let doc: WarGraphDoc =
        serde_json::from_value(version.definition.body.clone()).map_err(|source| {
            CliError::execution(format!(
                "assistant '{}' version {} carries an unparseable graph document: {source}",
                row.assistant.assistant_id, row.assistant.version
            ))
        })?;

    Ok(Some((
        doc,
        GraphResolution::AssistantVersion {
            assistant_id: row.assistant.assistant_id.clone(),
            version: row.assistant.version,
        },
    )))
}

/// Build the [`ExecutionOverlay`], preferring persisted trace rows (exact
/// edges) and falling back to Waypoint history (always available, derived
/// fired edges) when no trace was persisted for `thread_id`. `shape` is used
/// only by the Waypoints path, to validate which completed->vanguard
/// candidate transitions are genuine shape edges; when no real shape is
/// available (D-22 bucket 3) an empty shape is passed, so a Waypoints-
/// sourced observed-only overlay carries visits but no derived fired edges --
/// a documented, narrow gap (only the Trace source can derive exact edges
/// with no shape at all).
async fn build_overlay(
    thread_id: &ThreadId,
    waypoint_cap: Option<WaypointId>,
    shape: &GraphShape,
) -> Result<ExecutionOverlay, CliError> {
    if let Some(trace_store) = try_build_run_trace_store().await? {
        let records = read_all_trace_records(trace_store.as_ref(), thread_id).await?;
        if !records.is_empty() {
            return Ok(ExecutionOverlay::from_trace_records(&records));
        }
    }

    let waypoint_store = build_waypoint_store().await?;
    let summaries = waypoint_store
        .history(thread_id, None, None)
        .await
        .map_err(|e| CliError::execution(format!("waypoint store error: {e}")))?;

    if summaries.is_empty() {
        return Err(CliError::execution(format!(
            "thread '{thread_id}' has no Waypoint history and no persisted trace -- nothing to \
             export"
        )));
    }

    // `WaypointPort::history` returns newest-first: capping at `--waypoint`
    // keeps that entry and everything OLDER, i.e. "the overlay as of that
    // Waypoint".
    let capped = match waypoint_cap {
        Some(cap_id) => {
            let cap_index = summaries
                .iter()
                .position(|summary| summary.waypoint_id == cap_id)
                .ok_or_else(|| {
                    CliError::invalid_argument(format!(
                        "unknown --waypoint id for thread '{thread_id}'"
                    ))
                })?;
            summaries[cap_index..].to_vec()
        }
        None => summaries,
    };

    let mut waypoints: Vec<Waypoint> = Vec::with_capacity(capped.len());
    for summary in &capped {
        if let Some(wp) = waypoint_store
            .get(thread_id, &summary.waypoint_id)
            .await
            .map_err(|e| CliError::execution(format!("waypoint store error: {e}")))?
        {
            waypoints.push(wp);
        }
    }

    Ok(ExecutionOverlay::from_waypoints(&waypoints, shape))
}

/// Page through every persisted trace record for `thread_id`, ascending
/// `seq`, bounded by [`PAGE_SIZE`] per call.
async fn read_all_trace_records(
    trace_store: &dyn RunTracePort,
    thread_id: &ThreadId,
) -> Result<Vec<TraceRecord>, CliError> {
    let mut records = Vec::new();
    let mut after_seq = 0u64;
    loop {
        let page = trace_store
            .read(thread_id, after_seq, PAGE_SIZE)
            .await
            .map_err(|e| CliError::execution(format!("trace store error: {e}")))?;
        let page_len = page.len();
        if let Some(last) = page.last() {
            after_seq = last.seq;
        }
        records.extend(page);
        if page_len < PAGE_SIZE as usize {
            break;
        }
    }
    Ok(records)
}

/// Build the [`RunTracePort`] the configured `RunStoreConfig` names, if any
/// backend is configured at all. `Disabled` yields `None` (fall back to
/// Waypoint history, not an error) -- a genuinely unreachable configured
/// backend still errors.
async fn try_build_run_trace_store() -> Result<Option<Arc<dyn RunTracePort>>, CliError> {
    let mut config = RunStoreConfig::default();
    config.apply_env_overrides();
    config
        .validate()
        .map_err(|e| CliError::configuration(format!("invalid run store configuration: {e}")))?;

    match &config.backend {
        RunStoreBackend::Disabled => Ok(None),
        RunStoreBackend::Sqlite { path } => {
            let store = paladin_storage::run_trace::sqlite::SqliteRunTraceStore::new(path)
                .await
                .map_err(|e| {
                    CliError::execution(format!(
                        "failed to open sqlite trace store at '{path}': {e}"
                    ))
                })?;
            Ok(Some(Arc::new(store)))
        }
        RunStoreBackend::Postgres { url_env } => {
            build_postgres_trace_store(url_env).await.map(Some)
        }
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_trace_store(url_env: &str) -> Result<Arc<dyn RunTracePort>, CliError> {
    let url = std::env::var(url_env).map_err(|_| {
        CliError::configuration(format!(
            "run store postgres backend names env var '{url_env}', which is not set"
        ))
    })?;
    let store = paladin_storage::run_trace::postgres::PostgresRunTraceStore::new(&url)
        .await
        .map_err(|e| CliError::execution(format!("failed to open postgres trace store: {e}")))?;
    Ok(Arc::new(store))
}

#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_trace_store(url_env: &str) -> Result<Arc<dyn RunTracePort>, CliError> {
    Err(CliError::configuration(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,cli, or set APP_RUN_STORE_BACKEND=sqlite"
    )))
}

/// Build the [`RunRepositoryPort`] the configured `RunStoreConfig` names
/// (ADR-0023) -- required whenever `--run` is given.
async fn build_run_repository() -> Result<Arc<dyn RunRepositoryPort>, CliError> {
    let mut config = RunStoreConfig::default();
    config.apply_env_overrides();
    config
        .validate()
        .map_err(|e| CliError::configuration(format!("invalid run store configuration: {e}")))?;

    match &config.backend {
        RunStoreBackend::Disabled => Err(CliError::configuration(
            "no run store is configured -- set APP_RUN_STORE_BACKEND=sqlite (and \
             APP_RUN_STORE_PATH) or =postgres to resolve --run",
        )),
        RunStoreBackend::Sqlite { path } => {
            let store = paladin_storage::run::sqlite::SqliteRunRepository::new(path)
                .await
                .map_err(|e| {
                    CliError::execution(format!("failed to open sqlite run store at '{path}': {e}"))
                })?;
            Ok(Arc::new(store))
        }
        RunStoreBackend::Postgres { url_env } => build_postgres_run_repository(url_env).await,
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_run_repository(
    url_env: &str,
) -> Result<Arc<dyn RunRepositoryPort>, CliError> {
    let url = std::env::var(url_env).map_err(|_| {
        CliError::configuration(format!(
            "run store postgres backend names env var '{url_env}', which is not set"
        ))
    })?;
    let store = paladin_storage::run::postgres::PostgresRunRepository::new(&url)
        .await
        .map_err(|e| CliError::execution(format!("failed to open postgres run store: {e}")))?;
    Ok(Arc::new(store))
}

#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_run_repository(
    url_env: &str,
) -> Result<Arc<dyn RunRepositoryPort>, CliError> {
    Err(CliError::configuration(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,cli, or set APP_RUN_STORE_BACKEND=sqlite"
    )))
}

/// Build the [`WaypointPort`] the configured `WaypointStoreConfig` names --
/// the Waypoint-history fallback source, required whenever no trace rows
/// exist. `Disabled` is a clear, actionable configuration error: with
/// neither a trace store nor a waypoint store, there is nothing to export.
async fn build_waypoint_store() -> Result<Arc<dyn WaypointPort>, CliError> {
    let mut config = WaypointStoreConfig::default();
    config.apply_env_overrides();
    config.validate().map_err(|e| {
        CliError::configuration(format!("invalid waypoint store configuration: {e}"))
    })?;

    match &config.backend {
        WaypointStoreBackend::Disabled => Err(CliError::configuration(
            "no waypoint store is configured -- set APP_WAYPOINT_STORE_BACKEND=sqlite (and \
             APP_WAYPOINT_STORE_SQLITE_PATH) or =postgres",
        )),
        WaypointStoreBackend::Sqlite { path } => {
            let store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(path)
                .await
                .map_err(|e| {
                    CliError::execution(format!(
                        "failed to open sqlite waypoint store at '{path}': {e}"
                    ))
                })?;
            Ok(Arc::new(store))
        }
        WaypointStoreBackend::Postgres { url_env } => build_postgres_waypoint_store(url_env).await,
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_waypoint_store(url_env: &str) -> Result<Arc<dyn WaypointPort>, CliError> {
    let url = std::env::var(url_env).map_err(|_| {
        CliError::configuration(format!(
            "waypoint store postgres backend names env var '{url_env}', which is not set"
        ))
    })?;
    let store = paladin_storage::waypoint::postgres::PostgresWaypointStore::new(&url)
        .await
        .map_err(|e| CliError::execution(format!("failed to open postgres waypoint store: {e}")))?;
    Ok(Arc::new(store))
}

#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_waypoint_store(url_env: &str) -> Result<Arc<dyn WaypointPort>, CliError> {
    Err(CliError::configuration(format!(
        "waypoint_store.backend is configured as 'postgres' (env var '{url_env}') but this \
         binary was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,cli, or set APP_WAYPOINT_STORE_BACKEND=sqlite"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_export_args_parse_every_flag() {
        use clap::Parser;

        #[derive(clap::Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: RunExportArgs,
        }

        let run_id = RunId::new_v7();
        let parsed = Wrapper::parse_from([
            "paladin-cli",
            "--thread",
            "t-1",
            "--run",
            run_id.as_str(),
            "--graph",
            "graph.json",
            "--out",
            "out.mermaid",
        ]);
        assert_eq!(parsed.args.thread, Some(ThreadId::new("t-1").unwrap()));
        assert_eq!(parsed.args.run, Some(run_id));
        assert_eq!(parsed.args.graph, Some(PathBuf::from("graph.json")));
        assert_eq!(parsed.args.out, Some(PathBuf::from("out.mermaid")));
    }

    #[test]
    fn render_report_notes_source_and_resolution() {
        let report = RunExportReport {
            overlay_source: OverlaySource::Waypoints,
            resolution: GraphResolution::ObservedOnly,
            diagram: "flowchart TD\n  n0[test]\n".to_string(),
        };
        let rendered = report.render();
        assert!(rendered.contains("waypoints"));
        assert!(rendered.contains("observed-only"));
        assert!(rendered.contains("flowchart TD"));
    }
}
