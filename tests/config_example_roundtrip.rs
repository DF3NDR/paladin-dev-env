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
//!   form or its serialised JSON, as a habit: no credential field serialises (WR-04, WR-09), but a
//!   path list is all a failure needs. Every credential in a fixture is also an obviously fake
//!   `test-key-not-a-secret` literal.
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
    Row {
        pattern: &["llm_api_key"],
        note: "secret: skip_serializing on Settings.llm_api_key (WR-09)",
    },
    Row {
        pattern: &["http", "auth", "api_keys", "*", "key"],
        note: "secret: skip_serializing on ApiKeyConfig.key (WR-09)",
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

    // Measured on HEAD: the eight provider keys plus `llm_api_key` (WR-09). `ollama` carries
    // no `api_key` in the example.
    let mut want: BTreeSet<KeyPath> = [
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
    want.insert(vec!["llm_api_key".to_string()]);
    assert_eq!(
        missing,
        want,
        "the verbatim example must drop exactly the nine exempt secret paths, got:\n  {}",
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

    // WR-09: the root key and every `http.auth.api_keys[*].key` are omitted too. The fixture
    // carries both, so the absence is real and not a vacuous pass.
    let input = input_key_paths(&full_fixture());
    assert!(
        input.contains(&path_of(&["llm_api_key"])),
        "the fixture no longer carries llm_api_key, so this check is vacuous"
    );
    assert!(
        input.contains(&path_of(&["http", "auth", "api_keys", "[0]", "key"])),
        "the fixture no longer carries an api_keys key, so this check is vacuous"
    );
    let llm_api_key = path_of(&["llm_api_key"]);
    assert!(
        !output.contains(&llm_api_key),
        "serialised Settings carries {} (WR-09)",
        render_path(&llm_api_key)
    );
    let api_key_secrets: BTreeSet<&KeyPath> = output
        .iter()
        .filter(|p| matches(&["http", "auth", "api_keys", "*", "key"], p))
        .collect();
    assert!(
        api_key_secrets.is_empty(),
        "serialised Settings carries http.auth.api_keys[*].key paths (WR-09):\n  {}",
        render_sorted(api_key_secrets)
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

// ── drift guard: the full fixture cannot fall behind the example ─────────────

/// Prose comment lines of `config.example.yml` that look like a commented key (`word:`) but are
/// not one. The pattern is the single leaf the scanner reports; the note is the reason. A row
/// that matches no scanned line is stale, like every other table.
const PROSE_COMMENT_LINES: &[Row] = &[Row {
    pattern: &["Live-verified"],
    note: "prose: the kimi and gemini 'Live-verified: ...' status sentences",
}];

/// The marker line after which `config.example.yml` is env-only prose (no config keys).
const PLATFORM_BANNER: &str = "# Platform API";

/// `(top-level section, leaf key)` for every commented key line above [`PLATFORM_BANNER`].
///
/// A commented key line is a `#` comment whose body, after an optional `- `, starts with a key
/// (`[A-Za-z_][A-Za-z0-9_./-]*`) followed by `:` and then whitespace or the end of the line. The
/// section is the last live (non-comment, column 0) top-level key above the line.
fn commented_key_lines(text: &str) -> Vec<(String, String)> {
    let mut section = String::new();
    let mut found = Vec::new();
    for line in text.lines() {
        if line.starts_with(PLATFORM_BANNER) {
            break;
        }
        let trimmed = line.trim_start();
        if let Some(body) = trimmed.strip_prefix('#') {
            let body = body.trim_start();
            let body = body.strip_prefix("- ").unwrap_or(body);
            let key: String = body
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
                .collect();
            let starts_like_a_key = key
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
            let after = &body[key.len()..];
            let colon_follows = after
                .strip_prefix(':')
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace));
            if starts_like_a_key && colon_follows {
                found.push((section.clone(), key));
            }
        } else if let Some((top, _)) = line
            .split_once(':')
            .filter(|_| !line.starts_with(char::is_whitespace))
        {
            section = top.trim().to_string();
        }
    }
    found
}

/// Commented example keys (minus prose rows) that no fixture path under the same section ends in.
fn commented_pairs_missing(
    example_text: &str,
    fixture: &BTreeSet<KeyPath>,
    prose: &[Row],
) -> BTreeSet<(String, String)> {
    commented_key_lines(example_text)
        .into_iter()
        .filter(|(_, leaf)| {
            !prose
                .iter()
                .any(|row| matches(row.pattern, std::slice::from_ref(leaf)))
        })
        .filter(|(section, leaf)| {
            !fixture
                .iter()
                .any(|p| p.first() == Some(section) && p.last() == Some(leaf))
        })
        .collect()
}

fn render_pairs(pairs: &BTreeSet<(String, String)>) -> String {
    pairs
        .iter()
        .map(|(section, leaf)| format!("{section} / {leaf}"))
        .collect::<Vec<_>>()
        .join("\n  ")
}

#[test]
fn full_fixture_carries_every_example_key() {
    let example = repo_path("config.example.yml");
    let text = std::fs::read_to_string(&example).expect("config.example.yml is readable");
    let fixture = input_key_paths(&full_fixture());

    let live_missing = missing_paths(&input_key_paths(&example), &fixture);
    assert!(
        live_missing.is_empty(),
        "live key paths of config.example.yml absent from config.example.full.yml:\n  {}",
        render_sorted(&live_missing)
    );

    let commented_missing = commented_pairs_missing(&text, &fixture, PROSE_COMMENT_LINES);
    assert!(
        commented_missing.is_empty(),
        "commented key lines of config.example.yml (section / key) with no matching key in \
         config.example.full.yml:\n  {}",
        render_pairs(&commented_missing)
    );

    // Every prose row must still match a scanned line, or it is stale.
    let scanned: BTreeSet<KeyPath> = commented_key_lines(&text)
        .into_iter()
        .map(|(_, leaf)| vec![leaf])
        .collect();
    let stale = stale_rows(&scanned, PROSE_COMMENT_LINES);
    assert!(
        stale.is_empty(),
        "PROSE_COMMENT_LINES rows that match no commented line (stale):\n  {}",
        stale.join("\n  ")
    );
}

#[test]
fn drift_guard_reports_a_commented_key_missing_from_the_fixture() {
    let example = "trace:\n  log_sink: true\n  #   brand_new_knob: 1\n  # plain prose, no key.\n";
    let fixture_value: Value = serde_json::json!({"trace": {"log_sink": true}});
    let fixture = key_paths(&fixture_value);

    let missing = commented_pairs_missing(example, &fixture, &[]);
    let want: BTreeSet<(String, String)> = [("trace".to_string(), "brand_new_knob".to_string())]
        .into_iter()
        .collect();
    assert_eq!(missing, want, "got:\n  {}", render_pairs(&missing));

    // The live-path half: a live example key the fixture lacks is reported by path.
    let example_value: Value = serde_json::json!({"trace": {"log_sink": true, "persist": false}});
    let live_missing = missing_paths(&key_paths(&example_value), &fixture);
    let want_live: BTreeSet<KeyPath> = [path_of(&["trace", "persist"])].into_iter().collect();
    assert_eq!(live_missing, want_live);

    // A key the fixture does carry is not reported.
    let carried: Value = serde_json::json!({"trace": {"brand_new_knob": 1}});
    assert!(commented_pairs_missing(example, &key_paths(&carried), &[]).is_empty());
}

// ── lead L5: sections the example claims are written out in full ─────────────

/// Top-level sections whose example comment says every field is written out explicitly.
const COMPLETE_SECTIONS: &[&str] = &["agent_runtime", "trace"];

#[test]
fn example_sections_claimed_complete_write_every_field() {
    let example = repo_path("config.example.yml");
    let input = input_key_paths(&example);
    let output = loaded_key_paths(&example);

    let unwritten: BTreeSet<KeyPath> = output
        .difference(&input)
        .filter(|p| {
            p.first()
                .is_some_and(|top| COMPLETE_SECTIONS.contains(&top.as_str()))
        })
        .cloned()
        .collect();
    assert!(
        unwritten.is_empty(),
        "config.example.yml claims these sections are written out in full, but the loaded \
         Settings serialises these paths the example does not write:\n  {}",
        render_sorted(&unwritten)
    );
}

// ── price precision: quoted exact, unquoted credible values value-equal ──────

/// Normalises a decimal string for value comparison without float arithmetic: trailing zeros of
/// the fractional part, and a then-trailing `.`, are trimmed (`"2.50"` and `"2.5"` agree).
fn normalise_decimal(text: &str) -> String {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text.to_string()
    }
}

/// The required top-level keys, copied from the example, plus one `treasurer.pricing.probe` row
/// whose prices are written by the caller.
fn probe_config(pricing_row: &str) -> String {
    format!(
        "llm_type: \"openai\"\nllm_url: \"https://api.openai.com/v1\"\nllm_api_key: \"\"\n\
         max_file_size: 10485760\nsources: []\nserver:\n  host: \"0.0.0.0\"\n  port: 8080\n\
         treasurer:\n  pricing:\n    probe:\n{pricing_row}"
    )
}

/// Loads `yaml` through `Settings::load_from_file` and returns the `treasurer.pricing.probe`
/// axes as the strings the loader produced.
fn load_probe_row(yaml: &str) -> Value {
    let file = tempfile::Builder::new()
        .suffix(".yml")
        .tempfile()
        .expect("temp file");
    std::fs::write(file.path(), yaml).expect("write temp config");
    load_to_json(file.path())["treasurer"]["pricing"]["probe"].clone()
}

#[test]
fn unquoted_credible_prices_keep_their_decimal_value() {
    let row = load_probe_row(&probe_config(
        "      prompt: 2.50\n      completion: 10.00\n      cache_read: 0.075\n      cache_write: 15\n",
    ));
    let want = [
        ("prompt", "2.5"),
        ("completion", "10"),
        ("cache_read", "0.075"),
        ("cache_write", "15"),
    ];
    let mut differing: Vec<String> = Vec::new();
    for (axis, expected) in want {
        let loaded = row.get(axis).and_then(Value::as_str);
        if loaded.map(normalise_decimal).as_deref() != Some(expected) {
            differing.push(format!("treasurer.pricing.probe.{axis}"));
        }
    }
    assert!(
        differing.is_empty(),
        "unquoted prices whose decimal value changed on load:\n  {}",
        differing.join("\n  ")
    );
}

#[test]
fn normalise_decimal_trims_only_the_fractional_tail() {
    assert_eq!(normalise_decimal("2.50"), "2.5");
    assert_eq!(normalise_decimal("10.00"), "10");
    assert_eq!(normalise_decimal("15"), "15");
    assert_eq!(normalise_decimal("100"), "100", "integers keep their zeros");
    assert_eq!(normalise_decimal("0.075"), "0.075");
}
