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
use std::path::{Path, PathBuf};

use paladin::config::Settings;
use serde_json::Value;
use serial_test::serial;

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

// ── the full fixture, the Redis-cadence fixture and the Settings::new path ───

fn full_fixture() -> PathBuf {
    repo_path("tests/fixtures/config/config.example.full.yml")
}

fn path_of(segments: &[&str]) -> KeyPath {
    segments.iter().map(|s| (*s).to_string()).collect()
}

/// Asserts the D-10 properties for one fixture: nothing unexplained, and every `EXEMPT` and
/// `KNOWN_GAPS` row still matches a missing path (a row that matches nothing is stale).
fn assert_full_round_trip(label: &str, input: &BTreeSet<KeyPath>, output: &BTreeSet<KeyPath>) {
    let missing = missing_paths(input, output);
    assert_no_unexplained(label, &unexplained_paths(&missing, &[EXEMPT, KNOWN_GAPS]));
    let stale_exempt = stale_rows(&missing, EXEMPT);
    assert!(
        stale_exempt.is_empty(),
        "{label}: EXEMPT rows that match no missing path (stale):\n  {}",
        stale_exempt.join("\n  ")
    );
    let stale_gaps = stale_rows(&missing, KNOWN_GAPS);
    assert!(
        stale_gaps.is_empty(),
        "{label}: KNOWN_GAPS rows that match no missing path (stale, the gap may be fixed):\n  {}",
        stale_gaps.join("\n  ")
    );
}

#[test]
fn full_fixture_round_trips_through_load_from_file() {
    let fixture = full_fixture();
    assert_full_round_trip(
        "config.example.full.yml via Settings::load_from_file",
        &input_key_paths(&fixture),
        &loaded_key_paths(&fixture),
    );
}

/// Restores the process working directory on every exit path, a panic included.
struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        // Best effort in a destructor: nothing useful can be done with a failure here.
        let _ = std::env::set_current_dir(&self.0);
    }
}

#[test]
#[serial]
fn full_fixture_round_trips_through_settings_new() {
    let fixture = full_fixture();
    let input = input_key_paths(&fixture);

    // `Settings::new()` (the `paladin-cli arsenal` path) reads `config.yml` from the current
    // directory, so give it a temporary directory holding the fixture under that name.
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::copy(&fixture, dir.path().join("config.yml")).expect("copy fixture");
    let _restore = CwdGuard(std::env::current_dir().expect("current dir"));
    std::env::set_current_dir(dir.path()).expect("enter temp dir");

    let settings = Settings::new()
        .unwrap_or_else(|e| panic!("Settings::new rejected config.example.full.yml: {e}"));
    let output = key_paths(&serde_json::to_value(&settings).expect("Settings serialises"));
    assert_full_round_trip("config.example.full.yml via Settings::new", &input, &output);
}

#[test]
fn redis_cadence_fixture_round_trips_through_load_from_file() {
    let fixture = repo_path("tests/fixtures/config/config.example.redis-cadence.yml");
    let input = input_key_paths(&fixture);
    let output = loaded_key_paths(&fixture);
    let url_env = path_of(&["treasurer", "cadence", "backend", "redis", "url_env"]);
    assert!(
        output.contains(&url_env),
        "{} did not survive the loader",
        render_path(&url_env)
    );
    let missing = missing_paths(&input, &output);
    assert_no_unexplained(
        "config.example.redis-cadence.yml",
        &unexplained_paths(&missing, &[EXEMPT, KNOWN_GAPS]),
    );
}

#[test]
fn serialised_settings_carry_no_llm_api_key_and_no_webhook_secret() {
    let output = loaded_key_paths(&full_fixture());
    let api_keys: BTreeSet<&KeyPath> = output
        .iter()
        .filter(|p| p.first().map(String::as_str) == Some("llm"))
        .filter(|p| p.last().map(String::as_str) == Some("api_key"))
        .collect();
    assert!(
        api_keys.is_empty(),
        "serialised Settings carries llm api_key paths (WR-04):\n  {}",
        render_sorted(api_keys)
    );
    let secret = path_of(&["treasurer", "allowance", "webhook", "secret"]);
    assert!(
        !output.contains(&secret),
        "serialised Settings carries {}",
        render_path(&secret)
    );
}

/// The string at `path` inside `value`, if every segment resolves and the leaf is a string.
fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |node, key| node.get(*key))
        .and_then(Value::as_str)
}

fn load_to_json(path: &Path) -> Value {
    let name = path.to_str().expect("fixture path is UTF-8");
    let settings = Settings::load_from_file(name)
        .unwrap_or_else(|e| panic!("Settings::load_from_file rejected {}: {e}", path.display()));
    serde_json::to_value(&settings).expect("Settings serialises")
}

#[test]
fn quoted_money_strings_arrive_byte_identical() {
    let fixture = full_fixture();
    let text = std::fs::read_to_string(&fixture).expect("fixture is readable");
    let input: Value = serde_yaml::from_str(&text).expect("fixture is valid YAML");
    let loaded = load_to_json(&fixture);

    let mut checked = 0_usize;
    let mut differing: Vec<String> = Vec::new();
    let mut check = |path: Vec<&str>| {
        // Resolve the input side with the same walk, so the two sides cannot disagree on shape.
        let Some(want) = string_at(&input, &path) else {
            return;
        };
        checked += 1;
        if string_at(&loaded, &path) != Some(want) {
            differing.push(path.join("."));
        }
    };

    let axes = [
        "prompt",
        "completion",
        "cache_read",
        "cache_write",
        "reasoning",
    ];
    if let Some(rows) = input["treasurer"]["pricing"].as_object() {
        for (model, _) in rows {
            for axis in axes {
                check(vec!["treasurer", "pricing", model.as_str(), axis]);
            }
        }
    }
    for scope in ["tenants", "api_keys"] {
        if let Some(entries) = input["treasurer"]["allowance"][scope].as_object() {
            for (name, _) in entries {
                for field in ["amount", "lifetime"] {
                    check(vec!["treasurer", "allowance", scope, name.as_str(), field]);
                }
            }
        }
    }

    differing.sort();
    assert!(
        differing.is_empty(),
        "quoted money strings that did not load byte-for-byte:\n  {}",
        differing.join("\n  ")
    );
    // 4 pricing rows with 2..=5 axes and 3 allowance entries carrying 2..=3 money fields.
    assert!(
        checked >= 16,
        "the fixture carries too few quoted money values ({checked}) for this check to mean anything"
    );
}
