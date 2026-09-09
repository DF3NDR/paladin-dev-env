//! `paladin-cli graph export` -- render a graph document to Mermaid or DOT on
//! stdout (D-23, OBS-FR-08).
//!
//! `<FILE>` is a [`WarGraphDoc`] JSON or YAML document, read straight off
//! disk; `--assistant <id>[@<version>]` instead resolves a stored assistant's
//! `Workflow` definition through [`AssistantRepositoryPort`], reached over
//! the store the loaded `RunStoreConfig` names (SQLite locally, Postgres by
//! URL) -- exactly as the server would (ADR-0023). The CLI has no HTTP
//! client and gains none here: every lookup goes through the port.
//!
//! Output is pipe-friendly by construction: [`render_graph_export`] returns
//! exactly what [`to_mermaid`]/[`to_dot`] produced for the resolved shape,
//! with no extra text and no colour, written verbatim to stdout (or to
//! `--out`, with a short confirmation on stdout instead).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use paladin_battalion::engine::export::{GraphShape, to_dot, to_mermaid};
use paladin_battalion::engine::graph_doc::WarGraphDoc;
use paladin_core::platform::container::assistant::{AssistantId, AssistantKind};
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;

use crate::application::cli::error::CliError;
use crate::config::env_utils::EnvOverridable;
use crate::config::run_store::{RunStoreBackend, RunStoreConfig};

/// `--format` values `graph export` accepts, a closed choice via a derived
/// `clap::ValueEnum` -- the newest CLI commands' convention (`eval.rs`'s
/// derive-everything shape) rather than `muster.rs`'s older hand-rolled
/// `parse`/`as_str` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Render a Mermaid `flowchart TD` diagram.
    Mermaid,
    /// Render a Graphviz `digraph`.
    Dot,
}

/// `paladin-cli graph` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum GraphCommands {
    /// Render a graph document (from a file or a stored assistant) as a diagram.
    Export(GraphExportArgs),
}

/// Arguments for `paladin-cli graph export`.
#[derive(Debug, clap::Args)]
pub struct GraphExportArgs {
    /// A `WarGraphDoc` JSON or YAML file to render. Exactly one of `<FILE>`
    /// and `--assistant` is required.
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
    /// Render `<assistant-id>` (optionally `@<version>`, defaulting to the
    /// assistant's `latest`) through the configured assistant store instead
    /// of a file.
    #[arg(long)]
    pub assistant: Option<String>,
    /// `mermaid` or `dot`.
    #[arg(long, value_enum)]
    pub format: ExportFormat,
    /// Write the diagram to this path instead of stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// `paladin-cli graph export --format mermaid|dot (<FILE> | --assistant
/// <id>[@<version>]) [--out <path>]` (D-23): resolves a graph document from
/// a file or the configured assistant store, renders it, and writes the
/// result to `--out` or stdout with no colour.
pub async fn run_graph_export(
    format: ExportFormat,
    file: Option<PathBuf>,
    assistant: Option<String>,
    out: Option<PathBuf>,
) -> Result<(), CliError> {
    let rendered = render_graph_export(format, file, assistant).await?;
    write_output(&rendered, out).await
}

/// The testable core behind [`run_graph_export`]: resolves the graph
/// document and renders it, returning exactly the string
/// [`to_mermaid`]/[`to_dot`] produced (no extra text), without touching
/// stdout or a file.
pub async fn render_graph_export(
    format: ExportFormat,
    file: Option<PathBuf>,
    assistant: Option<String>,
) -> Result<String, CliError> {
    let doc = resolve_graph_document(file, assistant).await?;
    let shape = GraphShape::from_doc(&doc);
    Ok(match format {
        ExportFormat::Mermaid => to_mermaid(&shape),
        ExportFormat::Dot => to_dot(&shape),
    })
}

/// Resolve exactly one of `<FILE>`/`--assistant` into a [`WarGraphDoc`],
/// erroring clearly when both or neither are given.
async fn resolve_graph_document(
    file: Option<PathBuf>,
    assistant: Option<String>,
) -> Result<WarGraphDoc, CliError> {
    match (file, assistant) {
        (Some(_), Some(_)) => Err(CliError::invalid_argument(
            "exactly one of <FILE> or --assistant is required, not both",
        )),
        (None, None) => Err(CliError::invalid_argument(
            "exactly one of <FILE> or --assistant is required",
        )),
        (Some(path), None) => load_graph_doc_file(&path).await,
        (None, Some(spec)) => load_graph_doc_from_assistant(&spec).await,
    }
}

/// Read and parse a `WarGraphDoc` from `path` (JSON or YAML by extension,
/// mirroring `paladin_eval::Scenario::from_path`'s identical dispatch): a
/// missing file and a malformed document each produce a distinct,
/// path-naming [`CliError`], never a panic.
pub(crate) async fn load_graph_doc_file(path: &Path) -> Result<WarGraphDoc, CliError> {
    let contents =
        tokio::fs::read_to_string(path)
            .await
            .map_err(|source| CliError::FileReadError {
                path: path.display().to_string(),
                message: source.to_string(),
            })?;
    parse_graph_doc(path, &contents)
}

/// Parse `contents` as a `WarGraphDoc`, choosing YAML for a `.yaml`/`.yml`
/// extension and JSON for everything else.
fn parse_graph_doc(path: &Path, contents: &str) -> Result<WarGraphDoc, CliError> {
    let is_yaml = matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml") | Some("yml")
    );
    if is_yaml {
        serde_yaml::from_str(contents).map_err(|source| CliError::InvalidYaml {
            path: path.to_path_buf(),
            source,
        })
    } else {
        serde_json::from_str(contents).map_err(|source| CliError::FileReadError {
            path: path.display().to_string(),
            message: format!("invalid graph document JSON: {source}"),
        })
    }
}

/// Fetch `spec` (`<id>` or `<id>@<version>`, defaulting to the assistant's
/// `latest`) through [`AssistantRepositoryPort`] over the configured store
/// (ADR-0023) and parse its `Workflow` body as a `WarGraphDoc`. An unknown
/// id, an unknown version, or a non-`Workflow` (`Agent`) assistant each
/// produce a distinct, naming error.
async fn load_graph_doc_from_assistant(spec: &str) -> Result<WarGraphDoc, CliError> {
    let (id_str, version) = match spec.split_once('@') {
        Some((id, v)) => (
            id,
            Some(v.parse::<u32>().map_err(|_| {
                CliError::invalid_argument(format!("invalid --assistant version: '{v}'"))
            })?),
        ),
        None => (spec, None),
    };
    let assistant_id = AssistantId::new(id_str)
        .map_err(|e| CliError::invalid_argument(format!("invalid assistant id '{id_str}': {e}")))?;

    let repository = build_assistant_repository().await?;

    let resolved_version = match version {
        Some(v) => repository
            .get_version(&assistant_id, v)
            .await
            .map_err(|e| CliError::execution(format!("assistant store error: {e}")))?
            .ok_or_else(|| {
                CliError::execution(format!("assistant '{id_str}' has no version {v}"))
            })?,
        None => {
            let assistant = repository
                .get(&assistant_id)
                .await
                .map_err(|e| CliError::execution(format!("assistant store error: {e}")))?
                .ok_or_else(|| CliError::execution(format!("unknown assistant: '{id_str}'")))?;
            repository
                .get_version(&assistant_id, assistant.latest)
                .await
                .map_err(|e| CliError::execution(format!("assistant store error: {e}")))?
                .ok_or_else(|| {
                    CliError::execution(format!(
                        "assistant '{id_str}' has no version {}",
                        assistant.latest
                    ))
                })?
        }
    };

    if resolved_version.definition.kind != AssistantKind::Workflow {
        return Err(CliError::execution(format!(
            "assistant '{id_str}' is an Agent, not a Workflow -- no graph document to export"
        )));
    }

    serde_json::from_value(resolved_version.definition.body.clone()).map_err(|source| {
        CliError::execution(format!(
            "assistant '{id_str}' version {} carries an unparseable graph document: {source}",
            resolved_version.version
        ))
    })
}

/// Build the [`AssistantRepositoryPort`] the configured `RunStoreConfig`
/// names (ADR-0023) -- the SAME store `paladin-server` would use. A
/// `Disabled` backend is a clear, actionable configuration error, never a
/// silent fallback; a configured `Postgres` backend on a binary built
/// without the `storage-postgres` feature errors naming the missing
/// feature, mirroring `run_api_wiring.rs`'s identical precedent.
pub(crate) async fn build_assistant_repository()
-> Result<Arc<dyn AssistantRepositoryPort>, CliError> {
    let mut config = RunStoreConfig::default();
    config.apply_env_overrides();
    config
        .validate()
        .map_err(|e| CliError::configuration(format!("invalid run store configuration: {e}")))?;

    match &config.backend {
        RunStoreBackend::Disabled => Err(CliError::configuration(
            "no assistant store is configured -- set APP_RUN_STORE_BACKEND=sqlite (and \
             APP_RUN_STORE_PATH) or =postgres to resolve --assistant",
        )),
        RunStoreBackend::Sqlite { path } => {
            let store = paladin_storage::assistant::sqlite::SqliteAssistantRepository::new(path)
                .await
                .map_err(|e| {
                    CliError::execution(format!(
                        "failed to open sqlite assistant store at '{path}': {e}"
                    ))
                })?;
            Ok(Arc::new(store))
        }
        RunStoreBackend::Postgres { url_env } => build_postgres_assistant_repository(url_env).await,
    }
}

#[cfg(feature = "storage-postgres")]
async fn build_postgres_assistant_repository(
    url_env: &str,
) -> Result<Arc<dyn AssistantRepositoryPort>, CliError> {
    let url = std::env::var(url_env).map_err(|_| {
        CliError::configuration(format!(
            "run store postgres backend names env var '{url_env}', which is not set"
        ))
    })?;
    let store = paladin_storage::assistant::postgres::PostgresAssistantRepository::new(&url)
        .await
        .map_err(|e| {
            CliError::execution(format!("failed to open postgres assistant store: {e}"))
        })?;
    Ok(Arc::new(store))
}

/// When this binary is built without `storage-postgres`, a configured
/// `Postgres` backend is a startup error naming the missing feature, never a
/// silent failure to resolve `--assistant`.
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_assistant_repository(
    url_env: &str,
) -> Result<Arc<dyn AssistantRepositoryPort>, CliError> {
    Err(CliError::configuration(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,cli, or set APP_RUN_STORE_BACKEND=sqlite"
    )))
}

/// Write `rendered` to `out` (with a short stdout confirmation) or straight
/// to stdout (D-23's locked pipe-friendly, no-colour default).
pub(crate) async fn write_output(rendered: &str, out: Option<PathBuf>) -> Result<(), CliError> {
    match out {
        Some(path) => {
            tokio::fs::write(&path, rendered)
                .await
                .map_err(|source| CliError::IoError {
                    message: format!("failed to write diagram to '{}'", path.display()),
                    source,
                })?;
            println!("wrote diagram to {}", path.display());
            Ok(())
        }
        None => {
            // `print!`, not `println!`: `rendered` (`to_mermaid`/`to_dot`'s
            // own output, or `RunExportReport::render`'s annotated diagram)
            // already ends with its own trailing newline -- `println!` would
            // add a second one, breaking the "prints the same string ... and
            // nothing else" contract a byte-exact golden comparison depends
            // on.
            use std::io::Write;
            print!("{rendered}");
            std::io::stdout()
                .flush()
                .map_err(|source| CliError::IoError {
                    message: "failed to flush stdout".to_string(),
                    source,
                })?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_export_unknown_format_is_an_error() {
        use clap::Parser;

        #[derive(Debug, clap::Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: GraphExportArgs,
        }

        let result =
            Wrapper::try_parse_from(["paladin-cli", "some-file.json", "--format", "bogus"]);
        assert!(
            result.is_err(),
            "an unrecognised --format value must be rejected"
        );
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("mermaid") && message.contains("dot"),
            "the error should list the accepted values: {message}"
        );
    }

    #[test]
    fn graph_export_args_parse_file_and_format() {
        use clap::Parser;

        #[derive(clap::Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: GraphExportArgs,
        }

        let parsed = Wrapper::parse_from(["paladin-cli", "graph.json", "--format", "dot"]);
        assert_eq!(parsed.args.file, Some(PathBuf::from("graph.json")));
        assert!(parsed.args.assistant.is_none());
        assert_eq!(parsed.args.format, ExportFormat::Dot);
    }
}
