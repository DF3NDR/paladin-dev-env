//! `paladin-cli treasury spend` -- view settled Treasurer spend per tenant, API key, run or
//! model over a time window (LEDGR-04, D-09), reading the configured store directly through
//! [`RunStoreConfig`] exactly as `run export` reads the trace store -- no HTTP round-trip, no
//! server required.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::treasury_ledger::{
    SpendGroupBy, SpendQuery, SpendRow, format_cost,
};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;

use crate::application::cli::error::CliError;
use crate::application::cli::formatters::table::TableFormatter;
use crate::config::env_utils::EnvOverridable;
use crate::config::run_store::{RunStoreBackend, RunStoreConfig};

/// The default lookback window when neither `--since` nor `--until` is given (D-09): the last
/// 24 hours ending at the store clock.
const DEFAULT_LOOKBACK_HOURS: i64 = 24;

/// `paladin-cli treasury` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum TreasuryCommands {
    /// View settled spend per tenant, API key, run or model over a time window.
    Spend(TreasurySpendArgs),
}

/// `--group-by` values `treasury spend` accepts, a closed choice via a derived
/// `clap::ValueEnum` (mirrors `graph.rs`'s `ExportFormat` convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SpendGroupByArg {
    /// Group by `tenant_id`.
    Tenant,
    /// Group by `api_key_id`.
    ApiKey,
    /// Group by `run_id`.
    Run,
    /// Group by each model name inside a settlement's per-model breakdown.
    Model,
}

impl From<SpendGroupByArg> for SpendGroupBy {
    fn from(value: SpendGroupByArg) -> Self {
        match value {
            SpendGroupByArg::Tenant => SpendGroupBy::Tenant,
            SpendGroupByArg::ApiKey => SpendGroupBy::ApiKey,
            SpendGroupByArg::Run => SpendGroupBy::Run,
            SpendGroupByArg::Model => SpendGroupBy::Model,
        }
    }
}

/// `--format` values `treasury spend` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum SpendFormat {
    /// A human-readable table (default).
    Table,
    /// Pretty-printed JSON.
    Json,
}

/// Arguments for `paladin-cli treasury spend`.
#[derive(Debug, clap::Args)]
pub struct TreasurySpendArgs {
    /// The window's start instant (RFC 3339, inclusive). Defaults to `--until` minus 24 hours.
    #[arg(long, value_parser = parse_rfc3339_arg)]
    pub since: Option<DateTime<Utc>>,
    /// The window's end instant (RFC 3339, exclusive). Defaults to the store's own clock.
    #[arg(long, value_parser = parse_rfc3339_arg)]
    pub until: Option<DateTime<Utc>>,
    /// The dimension to group settled spend by.
    #[arg(long, value_enum, default_value = "tenant")]
    pub group_by: SpendGroupByArg,
    /// Restrict to one tenant.
    #[arg(long)]
    pub tenant: Option<String>,
    /// Restrict to one API key.
    #[arg(long = "api-key")]
    pub api_key: Option<String>,
    /// Restrict to one run.
    #[arg(long, value_parser = parse_run_id_arg)]
    pub run: Option<RunId>,
    /// `table` (default) or `json`.
    #[arg(long, value_enum, default_value = "table")]
    pub format: SpendFormat,
}

fn parse_run_id_arg(s: &str) -> Result<RunId, String> {
    RunId::parse(s).map_err(|e| e.to_string())
}

fn parse_rfc3339_arg(s: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("invalid RFC 3339 timestamp '{s}': {e}"))
}

/// `paladin-cli treasury spend` (LEDGR-04, D-09): opens the configured ledger store, queries
/// settled spend over the requested window, and prints the rendered report to stdout.
pub async fn run_treasury_spend(args: TreasurySpendArgs) -> Result<(), CliError> {
    let ledger = build_treasury_ledger().await?;
    let rendered = render_treasury_spend(&args, ledger.as_ref()).await?;
    println!("{rendered}");
    Ok(())
}

/// The testable core behind [`run_treasury_spend`]: resolves the window, queries `ledger`, and
/// renders the report, returning the rendered string without touching stdout.
///
/// # Errors
///
/// Returns [`CliError::validation`] when the resolved window is inverted (`since >= until`), and
/// [`CliError::execution`] when the ledger itself returns an error.
pub async fn render_treasury_spend(
    args: &TreasurySpendArgs,
    ledger: &dyn TreasuryLedgerPort,
) -> Result<String, CliError> {
    let until = match args.until {
        Some(until) => until,
        None => ledger
            .store_now()
            .await
            .map_err(|e| CliError::execution(format!("treasury ledger error: {e}")))?,
    };
    let since = args
        .since
        .unwrap_or_else(|| until - Duration::hours(DEFAULT_LOOKBACK_HOURS));

    if since >= until {
        return Err(CliError::validation(format!(
            "--since ({}) must be earlier than --until ({})",
            since.to_rfc3339(),
            until.to_rfc3339()
        )));
    }

    let query = SpendQuery {
        group_by: args.group_by.into(),
        since: Some(since),
        until: Some(until),
        tenant_id: args.tenant.clone(),
        api_key_id: args.api_key.clone(),
        run_ids: args.run.clone().into_iter().collect(),
    };

    let rows = ledger
        .spend(query)
        .await
        .map_err(|e| CliError::execution(format!("treasury ledger error: {e}")))?;

    match args.format {
        SpendFormat::Table => Ok(render_table(args.group_by, since, until, &rows)),
        SpendFormat::Json => render_json(&rows),
    }
}

fn group_label(group_by: SpendGroupByArg) -> &'static str {
    match group_by {
        SpendGroupByArg::Tenant => "tenant",
        SpendGroupByArg::ApiKey => "api-key",
        SpendGroupByArg::Run => "run",
        SpendGroupByArg::Model => "model",
    }
}

fn group_column_header(group_by: SpendGroupByArg) -> &'static str {
    match group_by {
        SpendGroupByArg::Tenant => "TENANT",
        SpendGroupByArg::ApiKey => "API KEY",
        SpendGroupByArg::Run => "RUN",
        SpendGroupByArg::Model => "MODEL",
    }
}

/// Render `rows` as a table, one row per (group, currency) -- never a combined total across
/// currencies (D-09) -- preceded by a header line naming the group and the window. An empty
/// `rows` prints a no-spend line instead of an (otherwise empty) table.
fn render_table(
    group_by: SpendGroupByArg,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    rows: &[SpendRow],
) -> String {
    let header_line = format!(
        "Spend by {} from {} to {}",
        group_label(group_by),
        since.to_rfc3339(),
        until.to_rfc3339()
    );

    if rows.is_empty() {
        return format!(
            "{header_line}\nNo settled spend between {} and {}.",
            since.to_rfc3339(),
            until.to_rfc3339()
        );
    }

    let mut table = TableFormatter::new();
    table.set_header(vec![group_column_header(group_by), "SPEND", "SETTLEMENTS"]);
    for row in rows {
        table.add_row(vec![
            row.group.clone(),
            format_cost(&row.amount),
            row.settlements.to_string(),
        ]);
    }

    format!("{header_line}\n{}", table.render())
}

/// One JSON-rendered [`SpendRow`] (LEDGR-04): the raw nano-unit figure alongside the D-04
/// display string, so a scripted caller can consume either.
#[derive(Debug, serde::Serialize)]
struct SpendRowJson {
    group: String,
    currency: String,
    nanos: i64,
    spend: String,
    settlements: u64,
}

fn render_json(rows: &[SpendRow]) -> Result<String, CliError> {
    let json_rows: Vec<SpendRowJson> = rows
        .iter()
        .map(|row| SpendRowJson {
            group: row.group.clone(),
            currency: row.amount.currency().as_str().to_string(),
            nanos: row.amount.nanos(),
            spend: format_cost(&row.amount),
            settlements: row.settlements,
        })
        .collect();
    Ok(serde_json::to_string_pretty(&json_rows)?)
}

/// Build the [`TreasuryLedgerPort`] the configured `RunStoreConfig` names (D-09, mirrors
/// `run.rs`'s `build_run_repository`).
async fn build_treasury_ledger() -> Result<Arc<dyn TreasuryLedgerPort>, CliError> {
    let mut config = RunStoreConfig::default();
    config.apply_env_overrides();
    config
        .validate()
        .map_err(|e| CliError::configuration(format!("invalid run store configuration: {e}")))?;

    match &config.backend {
        RunStoreBackend::Disabled => Err(CliError::configuration(
            "no treasury ledger store is configured -- set APP_RUN_STORE_BACKEND=sqlite (and \
             APP_RUN_STORE_PATH) or =postgres",
        )),
        RunStoreBackend::Sqlite { path } => {
            let store = paladin_storage::treasury::sqlite::SqliteTreasuryLedger::new(path)
                .await
                .map_err(|e| {
                    CliError::execution(format!(
                        "failed to open sqlite treasury ledger at '{path}': {e}"
                    ))
                })?;
            Ok(Arc::new(store))
        }
        RunStoreBackend::Postgres { url_env } => build_postgres_treasury_ledger(url_env).await,
    }
}

/// Reads the Postgres URL from the named env var and opens a [`PostgresTreasuryLedger`] against
/// it (D-09) -- mirrors `run.rs`'s `build_postgres_run_repository` exactly.
#[cfg(feature = "storage-postgres")]
async fn build_postgres_treasury_ledger(
    url_env: &str,
) -> Result<Arc<dyn TreasuryLedgerPort>, CliError> {
    let url = std::env::var(url_env).map_err(|_| {
        CliError::configuration(format!(
            "run store postgres backend names env var '{url_env}', which is not set"
        ))
    })?;
    let store = paladin_storage::treasury::postgres::PostgresTreasuryLedger::new(&url)
        .await
        .map_err(|e| {
            CliError::execution(format!("failed to open postgres treasury ledger: {e}"))
        })?;
    Ok(Arc::new(store))
}

/// This binary was built without the `storage-postgres` feature -- returns the same
/// `CliError::configuration` shape `run.rs`'s `not(feature = "storage-postgres")` arm uses, so an
/// operator sees one consistent message regardless of which store this gap affects.
#[cfg(not(feature = "storage-postgres"))]
async fn build_postgres_treasury_ledger(
    url_env: &str,
) -> Result<Arc<dyn TreasuryLedgerPort>, CliError> {
    Err(CliError::configuration(format!(
        "run_store.backend is configured as 'postgres' (env var '{url_env}') but this binary \
         was built without the 'storage-postgres' feature; rebuild with \
         --features storage-postgres,cli, or set APP_RUN_STORE_BACKEND=sqlite"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    use paladin_core::platform::container::treasury_ledger::{
        LedgerScope, SettleRequest, SettlementKey,
    };
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").unwrap()
    }

    /// A `SqliteTreasuryLedger` over a unique on-disk file (required for the on-disk migration
    /// round-trip this suite exercises), whose backing file (and `-wal`/`-shm` siblings) is
    /// removed when the guard drops.
    struct TestLedger {
        store: paladin_storage::treasury::sqlite::SqliteTreasuryLedger,
        path: PathBuf,
    }

    impl Drop for TestLedger {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(format!("{}-wal", self.path.display()));
            let _ = std::fs::remove_file(format!("{}-shm", self.path.display()));
        }
    }

    async fn fresh_ledger() -> TestLedger {
        let path = std::env::temp_dir().join(format!(
            "paladin_treasury_cli_test_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", path.display());
        let store = paladin_storage::treasury::sqlite::SqliteTreasuryLedger::new(&url)
            .await
            .unwrap();
        TestLedger { store, path }
    }

    fn base_args() -> TreasurySpendArgs {
        TreasurySpendArgs {
            since: None,
            until: None,
            group_by: SpendGroupByArg::Tenant,
            tenant: None,
            api_key: None,
            run: None,
            format: SpendFormat::Table,
        }
    }

    #[tokio::test]
    async fn treasury_spend_tracer_reads_settlements_from_sqlite_ledger() {
        let ledger = fresh_ledger().await;
        let run_id = RunId::new_v7();

        ledger
            .store
            .settle(SettleRequest::unreserved(
                LedgerScope::unattributed(),
                SettlementKey::new(run_id.clone(), 1, 1),
                Cost::new(45_000_000, usd()),
                BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
            ))
            .await
            .unwrap();
        ledger
            .store
            .settle(SettleRequest::unreserved(
                LedgerScope::unattributed(),
                SettlementKey::new(run_id.clone(), 2, 1),
                Cost::new(1_500_000, usd()),
                BTreeMap::from([("gpt-4o-mini".to_string(), 1_500_000_i64)]),
            ))
            .await
            .unwrap();

        let mut by_model = base_args();
        by_model.group_by = SpendGroupByArg::Model;
        let rendered = render_treasury_spend(&by_model, &ledger.store)
            .await
            .unwrap();
        assert!(rendered.contains("gpt-4"));
        assert!(rendered.contains("0.0450 USD"));
        assert!(rendered.contains("gpt-4o-mini"));
        assert!(rendered.contains("0.0015 USD"));

        let mut by_run = base_args();
        by_run.group_by = SpendGroupByArg::Run;
        let rendered = render_treasury_spend(&by_run, &ledger.store).await.unwrap();
        assert!(rendered.contains(run_id.as_str()));
        assert!(rendered.contains("0.0465 USD"));
        assert!(rendered.contains('2'));

        let mut json_args = base_args();
        json_args.group_by = SpendGroupByArg::Model;
        json_args.format = SpendFormat::Json;
        let rendered = render_treasury_spend(&json_args, &ledger.store)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        let array = parsed.as_array().unwrap();
        assert_eq!(array.len(), 2);
        let nanos: Vec<i64> = array
            .iter()
            .map(|row| row["nanos"].as_i64().unwrap())
            .collect();
        assert!(nanos.contains(&45_000_000));
        assert!(nanos.contains(&1_500_000));
    }

    #[tokio::test]
    async fn treasury_spend_empty_window_prints_no_spend_line() {
        let ledger = fresh_ledger().await;
        let mut args = base_args();
        args.since = Some(
            DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );
        args.until = Some(
            DateTime::parse_from_rfc3339("2020-01-02T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );

        let rendered = render_treasury_spend(&args, &ledger.store).await.unwrap();
        assert!(rendered.contains("No settled spend between"));

        let mut json_args = args;
        json_args.format = SpendFormat::Json;
        let rendered = render_treasury_spend(&json_args, &ledger.store)
            .await
            .unwrap();
        assert_eq!(rendered.trim(), "[]");
    }

    #[tokio::test]
    async fn treasury_spend_rejects_inverted_window() {
        let ledger = fresh_ledger().await;
        let mut args = base_args();
        args.since = Some(
            DateTime::parse_from_rfc3339("2020-01-02T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );
        args.until = Some(
            DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );

        let err = render_treasury_spend(&args, &ledger.store)
            .await
            .unwrap_err();
        assert!(matches!(err, CliError::ValidationError { .. }));
    }

    #[tokio::test]
    async fn treasury_spend_prints_one_row_per_currency() {
        let ledger = fresh_ledger().await;
        let run_id = RunId::new_v7();
        let eur = CurrencyCode::new("EUR").unwrap();

        ledger
            .store
            .settle(SettleRequest::unreserved(
                LedgerScope::unattributed(),
                SettlementKey::new(run_id.clone(), 1, 1),
                Cost::new(45_000_000, usd()),
                BTreeMap::from([("gpt-4".to_string(), 45_000_000_i64)]),
            ))
            .await
            .unwrap();
        ledger
            .store
            .settle(SettleRequest::unreserved(
                LedgerScope::unattributed(),
                SettlementKey::new(run_id, 2, 1),
                Cost::new(10_000_000, eur),
                BTreeMap::from([("gpt-4".to_string(), 10_000_000_i64)]),
            ))
            .await
            .unwrap();

        let mut args = base_args();
        args.group_by = SpendGroupByArg::Run;
        let rendered = render_treasury_spend(&args, &ledger.store).await.unwrap();
        assert!(rendered.contains("USD"));
        assert!(rendered.contains("EUR"));
        assert!(
            !rendered.contains("55_000_000") && !rendered.contains("0.0550"),
            "spend must never combine two currencies into one figure: {rendered}"
        );
    }
}
