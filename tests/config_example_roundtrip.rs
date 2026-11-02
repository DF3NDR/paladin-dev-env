//! DOGF-03 (Phase 56, plan 56-02): the shipped `config.example.yml`, with every optional key
//! present, round-trips through the production config loaders key for key.
//!
//! The rule (D-08..D-12). Most config sections silently drop an unknown or renamed key, so only
//! a diff can see a dropped key. Each fixture is loaded through the loader the binaries use,
//! `Settings::load_from_file` (and `Settings::new()` for the `paladin-cli arsenal` path), never
//! through raw serde. The loaded `Settings` is re-serialised with `serde_json::to_value`, and
//! the test asserts, by key path, that every path written in the input file appears in the
//! output, or is matched by an `EXEMPT` row (a field that is deliberately never serialised) or a
//! `KNOWN_GAPS` row (a logged `DF-nn` finding). Input key paths come from an independent
//! `serde_yaml` parse of the same file, so a key mangled at parse time cannot look identical on
//! both sides.
//!
//! What it does not do:
//!
//! - It compares key PATHS, never values. A value comparison would trip on `f32` temperatures
//!   re-serialising inexactly. The one value check is the money-string pair of tests.
//! - Failure output names key paths only (P6). Nothing here formats `Settings`, its `Debug`
//!   form or its serialised JSON, because `ApiKeyConfig.key` serialises in plaintext. Every
//!   credential in a fixture is also an obviously fake `test-key-not-a-secret` literal.
//! - It sets, reads and depends on no `APP_*` environment variable (D-12). An `APP_*` overlay
//!   can only add or override values, and the diff is one-directional, so an overlay cannot
//!   hide a dropped key.
//! - The full fixture carries no empty `models: {}` under a provider: that field is skipped on
//!   serialisation when empty, and would read as a false drop. An empty sequence or map
//!   elsewhere (`sources: []`, `http.auth.api_keys: []`) contributes its own key path.
//!
//! Key paths are vectors of segments, never `.`-joined strings, so the one model id
//! `gpt-5.5.1` and the nested path `gpt-5` -> `5` -> `1` are different. Maps compare as sets of
//! paths; sequences compare by index (`http.auth.api_keys[1].tenant` must survive as index 1).
//! Every failure message lists its paths sorted, so two runs print the same text.
//!
//! ```bash
//! cargo test -p paladin-ai --test config_example_roundtrip
//! ```

use std::collections::BTreeSet;
use std::path::PathBuf;

use paladin::config::Settings;
use serde_json::Value;

/// One key path: map keys are one segment each, a sequence element is the segment `[i]`.
type KeyPath = Vec<String>;

/// A table row: a segment pattern (`*` matches exactly one segment) and the reason or finding id
/// that explains why a path matching it may be absent from the re-serialised output.
struct Row {
    pattern: &'static [&'static str],
    note: &'static str,
}

/// Fields that are deliberately never serialised. One reason each.
const EXEMPT: &[Row] = &[
    Row {
        pattern: &["llm", "*", "api_key"],
        note: "secret: skip_serializing on LlmProviderConfig.api_key (WR-04)",
    },
    Row {
        pattern: &["treasurer", "allowance", "webhook", "secret"],
        note: "secret: skip_serializing on AllowanceWebhookConfig.secret, redacted in Debug",
    },
];

/// Paths the production loader drops, each citing the logged `DF-nn` finding.
const KNOWN_GAPS: &[Row] = &[];

// ── walker ───────────────────────────────────────────────────────────────────

/// Every key path in `value`: each container records its own path, then its children.
fn key_paths(value: &Value) -> BTreeSet<KeyPath> {
    let mut out = BTreeSet::new();
    walk(value, &mut Vec::new(), &mut out);
    out
}

fn walk(value: &Value, prefix: &mut KeyPath, out: &mut BTreeSet<KeyPath>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                prefix.push(key.clone());
                out.insert(prefix.clone());
                walk(child, prefix, out);
                prefix.pop();
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                prefix.push(format!("[{index}]"));
                out.insert(prefix.clone());
                walk(child, prefix, out);
                prefix.pop();
            }
        }
        _ => {}
    }
}

/// Joins segments with `.` for MESSAGES only; never used for comparison.
fn render_path(path: &[String]) -> String {
    path.join(".")
}

fn render_sorted<'a>(paths: impl IntoIterator<Item = &'a KeyPath>) -> String {
    let mut lines: Vec<String> = paths.into_iter().map(|p| render_path(p)).collect();
    lines.sort();
    lines.join("\n  ")
}

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Key paths of the YAML file at `path`, parsed independently of the `config` crate.
fn input_key_paths(path: &std::path::Path) -> BTreeSet<KeyPath> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let value: Value = serde_yaml::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not valid YAML: {e}", path.display()));
    key_paths(&value)
}

/// Key paths of the `Settings` the production loader builds from `path`.
fn loaded_key_paths(path: &std::path::Path) -> BTreeSet<KeyPath> {
    let name = path.to_str().expect("fixture path is UTF-8");
    let settings = Settings::load_from_file(name)
        .unwrap_or_else(|e| panic!("Settings::load_from_file rejected {}: {e}", path.display()));
    let value = serde_json::to_value(&settings).expect("Settings serialises");
    key_paths(&value)
}

/// Whether `pattern` (with `*` as a one-segment wildcard) matches `path` exactly.
fn matches(pattern: &[&str], path: &[String]) -> bool {
    pattern.len() == path.len()
        && pattern
            .iter()
            .zip(path)
            .all(|(want, got)| *want == "*" || want == got)
}

/// The paths in `input` that the loaded output lacks.
fn missing_paths(input: &BTreeSet<KeyPath>, output: &BTreeSet<KeyPath>) -> BTreeSet<KeyPath> {
    input.difference(output).cloned().collect()
}

/// The missing paths no row of any table explains.
fn unexplained_paths(missing: &BTreeSet<KeyPath>, tables: &[&[Row]]) -> BTreeSet<KeyPath> {
    missing
        .iter()
        .filter(|path| {
            !tables
                .iter()
                .any(|table| table.iter().any(|row| matches(row.pattern, path)))
        })
        .cloned()
        .collect()
}

/// Rows that match no missing path (stale), rendered as `pattern (note)`.
fn stale_rows(missing: &BTreeSet<KeyPath>, rows: &[Row]) -> Vec<String> {
    let mut stale: Vec<String> = rows
        .iter()
        .filter(|row| !missing.iter().any(|path| matches(row.pattern, path)))
        .map(|row| format!("{} ({})", row.pattern.join("."), row.note))
        .collect();
    stale.sort();
    stale
}

fn assert_no_unexplained(label: &str, unexplained: &BTreeSet<KeyPath>) {
    assert!(
        unexplained.is_empty(),
        "{label}: key paths written in the input but absent from the loaded Settings:\n  {}",
        render_sorted(unexplained)
    );
}

// ── tracer tests ─────────────────────────────────────────────────────────────

#[test]
fn walker_reports_a_planted_typo_key_through_the_loader() {
    let example = repo_path("config.example.yml");
    let text = std::fs::read_to_string(&example).expect("config.example.yml is readable");
    let marker = "\ntimeouts:\n";
    assert_eq!(
        text.matches(marker).count(),
        1,
        "one top-level timeouts block"
    );
    let typo = text.replacen(marker, "\ntimeouts:\n  defualt_seconds: 5\n", 1);

    // The loader picks its format from the file suffix.
    let file = tempfile::Builder::new()
        .suffix(".yml")
        .tempfile()
        .expect("temp file");
    std::fs::write(file.path(), typo).expect("write temp config");

    let input = input_key_paths(file.path());
    let output = loaded_key_paths(file.path());
    let missing = missing_paths(&input, &output);
    let unexplained = unexplained_paths(&missing, &[EXEMPT, KNOWN_GAPS]);

    let want: BTreeSet<KeyPath> = [vec!["timeouts".to_string(), "defualt_seconds".to_string()]]
        .into_iter()
        .collect();
    assert_eq!(
        unexplained,
        want,
        "the planted typo must be the only unexplained path, got:\n  {}",
        render_sorted(&unexplained)
    );
}

#[test]
fn walker_keeps_a_dotted_map_key_distinct_from_a_nested_path() {
    let dotted: Value = serde_json::json!({"p": {"gpt-5.5.1": 1}});
    let nested: Value = serde_json::json!({"p": {"gpt-5": {"5": {"1": 1}}}});
    let shared: BTreeSet<KeyPath> = key_paths(&dotted)
        .intersection(&key_paths(&nested))
        .cloned()
        .collect();
    let want: BTreeSet<KeyPath> = [vec!["p".to_string()]].into_iter().collect();
    assert_eq!(shared, want, "only the parent path is shared");
}

#[test]
fn verbatim_example_round_trips_through_load_from_file() {
    let example = repo_path("config.example.yml");
    let input = input_key_paths(&example);
    let output = loaded_key_paths(&example);
    let missing = missing_paths(&input, &output);

    assert_no_unexplained(
        "config.example.yml",
        &unexplained_paths(&missing, &[EXEMPT, KNOWN_GAPS]),
    );

    // Measured on HEAD: exactly the eight provider keys. `ollama` carries none in the example.
    let want: BTreeSet<KeyPath> = [
        "openai",
        "anthropic",
        "deepseek",
        "kimi",
        "qwen",
        "grok",
        "gemini",
        "openai-compatible",
    ]
    .iter()
    .map(|p| vec!["llm".to_string(), (*p).to_string(), "api_key".to_string()])
    .collect();
    assert_eq!(
        missing,
        want,
        "the verbatim example must drop exactly the eight exempt api_key paths, got:\n  {}",
        render_sorted(&missing)
    );
}

#[test]
fn exempt_and_known_gap_rows_go_stale_loudly() {
    let missing: BTreeSet<KeyPath> = [
        vec![
            "llm".to_string(),
            "openai".to_string(),
            "api_key".to_string(),
        ],
        vec![
            "llm".to_string(),
            "ollama".to_string(),
            "api_key".to_string(),
        ],
    ]
    .into_iter()
    .collect();
    let rows = [
        Row {
            pattern: &["llm", "*", "api_key"],
            note: "live",
        },
        Row {
            pattern: &["treasurer", "allowance", "webhook", "secret"],
            note: "matches nothing",
        },
        Row {
            pattern: &["llm", "*"],
            note: "wrong length",
        },
    ];
    assert_eq!(
        stale_rows(&missing, &rows),
        vec![
            "llm.* (wrong length)".to_string(),
            "treasurer.allowance.webhook.secret (matches nothing)".to_string(),
        ]
    );
    assert!(stale_rows(&missing, &rows[..1]).is_empty());
}
