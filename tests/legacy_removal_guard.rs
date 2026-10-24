//! Phase 44 legacy-removal guard (LEGACY-01, LEGACY-02, LEGACY-03, ADR-0059): the legacy
//! Battalion retry, error-strategy and timeout surfaces stay gone.
//!
//! Phase 44 deleted the Battalion's own `RetryPolicy`, `ErrorStrategy` and name-plus-text
//! `NodeError` (the Aegis, Flow and structured-error types of the same names are different types
//! and stay), the whole-run timeout wrappers and their `timeout_seconds` field, the two timeout
//! error variants, the `PaladinError` LLM-string variant and its message-matching predicates, and
//! the third backoff module. Every guarded timeout now flows through one runner
//! (`aegis_attempt.rs`). This test is the check that a later change cannot quietly bring any of
//! that back.
//!
//! Six rules run over the repository tree (std only, no new dependency):
//!
//! 1. **Strategy variants.** The two removed v0.10 strategy variant names, as whole words, in any
//!    scanned file.
//! 2. **Path forms.** A removed item named by its path (`battalion::` plus the removed type, the
//!    two removed timeout variants, the removed `PaladinError` variant and its two predicates),
//!    each ending at an identifier boundary.
//! 3. **Grouped imports.** A `battalion::{ ... }` import list, on one line, naming one of the
//!    three removed types.
//! 4. **Calls and modules.** A call to the removed retry predicate or builder, the removed delay
//!    helper, and the removed backoff module's paths (`retry_predicate` and `engine::retry` are
//!    different modules and pass).
//! 5. **Single timeout site.** In the production code (non-comment lines before `mod tests`) of
//!    the five legacy-pattern service files, a `timeout(` call or a `timeout_seconds` read. The
//!    Aegis attempt runner is the one allowed site.
//! 6. **Core module.** In the production code of the core battalion module and the Conclave
//!    module, the removed field names, `ErrorStrategy`, and a declaration or alias of
//!    `RetryPolicy` or `NodeError`. The test modules legitimately hold a v0.10 JSON fixture whose
//!    key strings must keep deserializing.
//!
//! Scanned files end in `.rs`, `.md`, `.toml`, `.yaml` or `.yml`. The only things skipped are the
//! declared history set: `target`, `node_modules` and every dot-directory (which covers
//! `.planning` and `.project`), every `CHANGELOG.md`, the root `MIGRATION.md`, the two upgrade
//! guides under `docs/src/api-reference/`, and this file. A hit is fixed at its source; it is never
//! silenced by growing that set.
//!
//! The forbidden strings are assembled at run time with `concat`, so this file never contains one
//! whole. A guard that cannot fail proves nothing, so two positive controls run the same scanner
//! over temporary trees: one with a planted violation per rule (each must be reported, naming its
//! rule) and one full of allowed look-alikes and history files (nothing may be reported). Each
//! file-scoped rule also reports a missing target, so a renamed or moved file fails the guard
//! rather than passing it silently.
//!
//! ```bash
//! cargo test --test legacy_removal_guard
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// The five legacy-pattern service files: rule 5 applies to their production code.
const LEGACY_PATTERN_FILES: [&str; 5] = [
    "crates/paladin-battalion/src/formation_service.rs",
    "crates/paladin-battalion/src/phalanx_service.rs",
    "crates/paladin-battalion/src/campaign_service.rs",
    "crates/paladin-battalion/src/commander.rs",
    "crates/paladin-battalion/src/conclave_execution_service.rs",
];

/// The core battalion module files: rule 6 applies to their production code.
const CORE_BATTALION_FILES: [&str; 2] = [
    "crates/paladin-core/src/platform/container/battalion/mod.rs",
    "crates/paladin-core/src/platform/container/battalion/conclave.rs",
];

/// Version-to-version history files: they must name the removed items, so they are not scanned.
/// Every `CHANGELOG.md` is skipped by name in addition to these exact paths.
const HISTORY_FILES: [&str; 3] = [
    "MIGRATION.md",
    "docs/src/api-reference/migration-guide.md",
    "docs/src/api-reference/upgrading.md",
];

/// This guard: it describes the removals, so it is not scanned (its strings are also assembled
/// at run time).
const GUARD_FILE: &str = "tests/legacy_removal_guard.rs";

/// File extensions the scan reads.
const SCANNED_EXTENSIONS: [&str; 5] = ["rs", "md", "toml", "yaml", "yml"];

/// The two removed v0.10 strategy variant names (rule 1).
fn removed_variants() -> [String; 2] {
    [
        ["Continue", "OnError"].concat(),
        ["RetryThen", "Continue"].concat(),
    ]
}

/// The three removed Battalion types (rules 3 and 6).
fn removed_types() -> [String; 3] {
    [
        ["Retry", "Policy"].concat(),
        ["Error", "Strategy"].concat(),
        ["Node", "Error"].concat(),
    ]
}

/// The removed items named by path (rule 2), each checked up to an identifier boundary.
fn removed_paths() -> Vec<String> {
    let [retry_policy, error_strategy, node_error] = removed_types();
    vec![
        ["battalion", "::", &retry_policy].concat(),
        ["battalion", "::", &error_strategy].concat(),
        ["battalion", "::", &node_error].concat(),
        ["Battalion", "Error::Timeout"].concat(),
        ["Conclave", "Error::Timeout"].concat(),
        ["Paladin", "Error::Llm", "Error"].concat(),
        ["Paladin", "Error::is_", "retryable"].concat(),
        ["Paladin", "Error::is_", "terminal"].concat(),
    ]
}

/// The opener of a grouped Battalion import (rule 3).
fn brace_import() -> String {
    ["battalion", "::{"].concat()
}

/// The removed calls (rule 4), matched as written, including the call parenthesis.
fn removed_calls() -> [String; 2] {
    [
        [".is_re", "tryable()"].concat(),
        [".with_retry_", "policy("].concat(),
    ]
}

/// The removed delay helper (rule 4), matched as a whole word.
fn removed_delay_helper() -> String {
    ["calculate_retry_", "delay"].concat()
}

/// The removed backoff module's paths (rule 4), each checked up to an identifier boundary.
fn removed_module_paths() -> [String; 2] {
    [
        ["paladin_battalion", "::re", "try"].concat(),
        ["services::battalion", "::re", "try"].concat(),
    ]
}

/// The `timeout(` call that rule 5 forbids outside the runner.
fn timeout_call() -> String {
    ["time", "out("].concat()
}

/// The removed whole-run timeout field (rules 5 and 6).
fn timeout_field() -> String {
    ["timeout", "_seconds"].concat()
}

/// The other removed field names (rule 6).
fn removed_fields() -> [String; 2] {
    [
        ["retry", "_policy"].concat(),
        ["error", "_strategy"].concat(),
    ]
}

/// `true` for characters that continue an identifier.
fn is_identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `true` when `needle` occurs in `haystack` with the requested boundaries: `bound_start` demands a
/// non-identifier character (or the line start) before it, `bound_end` one after it (or the end).
fn occurs(haystack: &str, needle: &str, bound_start: bool, bound_end: bool) -> bool {
    haystack.match_indices(needle).any(|(start, matched)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + matched.len()..].chars().next();
        (!bound_start || !before.is_some_and(is_identifier))
            && (!bound_end || !after.is_some_and(is_identifier))
    })
}

/// Rules 1 to 4 for one line: every `(rule number, matched text)` it trips.
fn generic_hits(line: &str) -> Vec<(u8, String)> {
    let mut hits = Vec::new();
    for variant in removed_variants() {
        if occurs(line, &variant, true, true) {
            hits.push((1, variant));
        }
    }
    for path in removed_paths() {
        if occurs(line, &path, false, true) {
            hits.push((2, path));
        }
    }
    let opener = brace_import();
    for (start, _) in line.match_indices(&opener) {
        let rest = &line[start + opener.len()..];
        let listed = rest.split('}').next().unwrap_or(rest);
        for removed in removed_types() {
            if occurs(listed, &removed, true, true) {
                hits.push((3, format!("{opener}.. {removed}")));
            }
        }
    }
    for call in removed_calls() {
        if occurs(line, &call, false, false) {
            hits.push((4, call));
        }
    }
    let helper = removed_delay_helper();
    if occurs(line, &helper, true, true) {
        hits.push((4, helper));
    }
    for path in removed_module_paths() {
        if occurs(line, &path, false, true) {
            hits.push((4, path));
        }
    }
    hits
}

/// Rule 5 for one production line of a legacy-pattern file.
fn timeout_site_hits(line: &str) -> Vec<(u8, String)> {
    let mut hits = Vec::new();
    let call = timeout_call();
    if occurs(line, &call, true, false) {
        hits.push((5, call));
    }
    let field = timeout_field();
    if occurs(line, &field, true, true) {
        hits.push((5, field));
    }
    hits
}

/// Rule 6 for one production line of a core battalion file.
fn core_redeclaration_hits(line: &str) -> Vec<(u8, String)> {
    let mut hits = Vec::new();
    let mut names = vec![timeout_field()];
    names.extend(removed_fields());
    for name in names {
        if occurs(line, &name, true, true) {
            hits.push((6, name));
        }
    }
    let [retry_policy, error_strategy, node_error] = removed_types();
    if occurs(line, &error_strategy, true, true) {
        hits.push((6, error_strategy));
    }
    for removed in [retry_policy, node_error] {
        for keyword in ["struct", "type", "as"] {
            let declaration = format!("{keyword} {removed}");
            if occurs(line, &declaration, true, true) {
                hits.push((6, declaration));
            }
        }
    }
    hits
}

/// The non-comment lines of `content` before the first `mod tests`, with 1-based line numbers.
fn production_lines(content: &str) -> impl Iterator<Item = (usize, &str)> {
    content
        .lines()
        .enumerate()
        .take_while(|(_, line)| !line.trim_start().starts_with("mod tests"))
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(index, line)| (index + 1, line))
}

/// Apply a file-scoped `rule` to every file in `files` under `root`. A target that cannot be read
/// is itself a violation: a guard that cannot find its target proves nothing.
fn scan_scoped(
    root: &Path,
    files: &[&str],
    rule: u8,
    hits: fn(&str) -> Vec<(u8, String)>,
    violations: &mut Vec<String>,
) {
    for relative in files {
        match fs::read_to_string(root.join(relative)) {
            Ok(content) => {
                for (number, line) in production_lines(&content) {
                    for (hit_rule, matched) in hits(line) {
                        violations.push(format!(
                            "{relative}:{number}: rule {hit_rule}: `{matched}` in production code"
                        ));
                    }
                }
            }
            Err(error) => violations.push(format!(
                "{relative}: rule {rule}: target file is missing or unreadable ({error}), \
                 so the rule proves nothing"
            )),
        }
    }
}

/// `true` for a directory the walk must not enter: build output, vendored packages and every
/// dot-directory (version control, `.planning`, `.project`, tool caches).
fn skip_directory(name: &str) -> bool {
    matches!(name, "target" | "node_modules") || name.starts_with('.')
}

/// `true` for a file the scan must not read: every `CHANGELOG.md`, the history files and this guard.
fn skip_file(relative: &str, name: &str) -> bool {
    name == "CHANGELOG.md" || relative == GUARD_FILE || HISTORY_FILES.contains(&relative)
}

/// Walk `root` and report one `<path>:<line>: rule <n>: ...` line per violation, sorted.
///
/// Files that cannot be read, or that are not UTF-8, are skipped by the walk: a binary asset cannot
/// carry the vocabulary this guard polices. The file-scoped rules read their targets directly and
/// report one that is missing.
fn scan(root: &Path) -> Vec<String> {
    let mut violations = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // `DirEntry::file_type` does not follow symlinks, so a link can never loop the walk.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if file_type.is_dir() {
                if !skip_directory(&name) {
                    pending.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let scanned = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| SCANNED_EXTENSIONS.contains(&extension));
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if !scanned || skip_file(&relative, &name) {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            for (number, line) in content.lines().enumerate() {
                for (rule, matched) in generic_hits(line) {
                    violations.push(format!(
                        "{relative}:{}: rule {rule}: removed Phase 44 name `{matched}`",
                        number + 1
                    ));
                }
            }
        }
    }

    scan_scoped(
        root,
        &LEGACY_PATTERN_FILES,
        5,
        timeout_site_hits,
        &mut violations,
    );
    scan_scoped(
        root,
        &CORE_BATTALION_FILES,
        6,
        core_redeclaration_hits,
        &mut violations,
    );

    violations.sort();
    violations
}

/// A temporary directory under the system temp dir, removed on drop (also when a test panics).
struct ScratchTree {
    root: PathBuf,
}

impl ScratchTree {
    /// A fresh, uniquely named tree: process id, a per-process counter and the clock keep two
    /// tests, two processes and two runs from sharing a path.
    fn new(label: &str) -> io::Result<Self> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let root = std::env::temp_dir().join(format!(
            "paladin_legacy_removal_guard_{label}_{}_{}_{nanos}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Write `content` at `relative` (a `/`-separated path), creating parent directories.
    fn write(&self, relative: &str, content: &str) -> io::Result<()> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)
    }
}

impl Drop for ScratchTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Every scoped file, in the order rules 5 and 6 list them.
fn scoped_files() -> impl Iterator<Item = &'static str> {
    LEGACY_PATTERN_FILES.into_iter().chain(CORE_BATTALION_FILES)
}

/// Clean content for a scoped file: allowed production code, with `extra` planted among it, then a
/// test module that legitimately uses the removed field names and a whole-run timeout.
fn scoped_source(extra: &str) -> String {
    let timeout = timeout_call();
    let field = timeout_field();
    let [retry_policy, error_strategy] = removed_fields();
    format!(
        "//! Comments may mention {timeout} and {field}.\n\
         /// Docs may too: {timeout} and {field}.\n\
         // {retry_policy} {error_strategy}\n\
         pub fn run() {{}}\n\
         {extra}\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20   fn fixture() {{\n\
         \x20       let _ = tokio::time::{timeout}d, f);\n\
         \x20       let _ = \"{field}\";\n\
         \x20       let _ = \"{retry_policy}\";\n\
         \x20       let _ = \"{error_strategy}\";\n\
         \x20   }}\n\
         }}\n"
    )
}

/// All forbidden strings, one per line: what a history file names.
fn every_removed_name() -> String {
    let mut names = Vec::new();
    names.extend(removed_variants());
    names.extend(removed_paths());
    names.push(format!(
        "use paladin::{}{}}};",
        brace_import(),
        removed_types()[2]
    ));
    names.extend(removed_calls());
    names.push(removed_delay_helper());
    names.extend(removed_module_paths());
    names.join("\n") + "\n"
}

/// A tree of allowed look-alikes and history files: no violation anywhere. Every scoped file is
/// present with clean content.
fn clean_tree(label: &str) -> io::Result<ScratchTree> {
    let tree = ScratchTree::new(label)?;
    for relative in scoped_files() {
        tree.write(relative, &scoped_source(""))?;
    }
    // The same-named, still-live types and the sibling vocabulary.
    tree.write(
        "src/look_alikes.rs",
        "use paladin_core::platform::container::aegis::RetryPolicy;\n\
         use paladin_core::platform::container::node_error::NodeError;\n\
         use paladin_battalion::maneuver::ErrorStrategy;\n\
         let _ = maneuver::ErrorStrategy::ContinueParallel;\n\
         let _ = JobExecutionMode::SequentialContinueOnError;\n\
         let _ = HandoffError::LlmError(String::new());\n\
         let _ = PaladinError::LlmFailure;\n\
         use paladin_battalion::retry_predicate::RetryPredicateEvaluator;\n\
         let _ = paladin_battalion::engine::retry::backoff_delay;\n\
         let _ = battalion::RetryPolicyConfig;\n\
         let _ = battalion::NodeErrorKind;\n\
         use paladin_core::platform::container::battalion::{BattalionConfig, BattalionResult};\n\
         let _ = ManeuverConfig::with_timeout(Duration::from_secs(1));\n\
         let _ = ConclaveExecutionService::is_retryable_error(&error);\n",
    )?;
    // The one allowed whole-attempt timeout site.
    tree.write(
        "crates/paladin-battalion/src/aegis_attempt.rs",
        &format!("let _ = tokio::time::{}d, f);\n", timeout_call()),
    )?;
    // A look-alike builder inside a file named like a legacy-pattern file.
    tree.write(
        LEGACY_PATTERN_FILES[3],
        &scoped_source("let _ = ManeuverConfig::with_timeout(Duration::from_secs(1));"),
    )?;
    tree.write(
        LEGACY_PATTERN_FILES[4],
        &scoped_source("let _ = ConclaveExecutionService::is_retryable_error(&error);"),
    )?;
    // History files name every removed item.
    let history = every_removed_name();
    for relative in [
        "CHANGELOG.md",
        "crates/x/CHANGELOG.md",
        "MIGRATION.md",
        ".planning/phases/44/notes.md",
        ".project/current-exports.txt.md",
        "docs/src/api-reference/upgrading.md",
        "docs/src/api-reference/migration-guide.md",
        "target/debug/build_output.rs",
        "node_modules/pkg/readme.md",
        "docs/assets/recording.cast",
        "docs/assets/exports.txt",
    ] {
        tree.write(relative, &history)?;
    }
    Ok(tree)
}

/// One planted violation per file: `(path, content, rule)`. Every other scoped file stays clean so
/// only the planted ones report.
fn plant_one_violation_per_rule() -> Vec<(String, String, u8)> {
    let [continue_on_error, retry_then_continue] = removed_variants();
    let paths = removed_paths();
    let [retry_policy, error_strategy, _] = removed_types();
    let [predicate_call, builder_call] = removed_calls();
    let [backoff_path, facade_path] = removed_module_paths();
    let opener = brace_import();
    let [field_retry, _] = removed_fields();
    let timeout = timeout_call();
    let field = timeout_field();

    vec![
        (
            "docs/a.md".into(),
            format!("Use {continue_on_error} here.\n"),
            1,
        ),
        (
            "src/b.rs".into(),
            format!("let s = {retry_then_continue};\n"),
            1,
        ),
        (
            "ops/c.yaml".into(),
            format!("# strategy: {continue_on_error}\n"),
            1,
        ),
        (
            "ops/d.yml".into(),
            format!("# strategy: {retry_then_continue}\n"),
            1,
        ),
        (
            "src/e.rs".into(),
            format!("use paladin::{};\n", paths[0]),
            2,
        ),
        ("config/f.toml".into(), format!("# was {}\n", paths[1]), 2),
        (
            "src/g.rs".into(),
            format!("let _ = {}(300);\n", paths[3]),
            2,
        ),
        ("src/h.rs".into(), format!("let _ = {};\n", paths[4]), 2),
        ("src/i.rs".into(), format!("let _ = {}(x);\n", paths[5]), 2),
        ("src/j.rs".into(), format!("let _ = {};\n", paths[6]), 2),
        ("src/k.rs".into(), format!("let _ = {};\n", paths[7]), 2),
        (
            "src/l.rs".into(),
            format!("use paladin::{opener}BattalionConfig, {error_strategy}}};\n"),
            3,
        ),
        (
            "src/m.rs".into(),
            format!("use paladin::{opener}{retry_policy}}};\n"),
            3,
        ),
        (
            "src/n.rs".into(),
            format!("if error{predicate_call} {{}}\n"),
            4,
        ),
        (
            "src/o.rs".into(),
            format!("config{builder_call}policy);\n"),
            4,
        ),
        (
            "src/p.rs".into(),
            format!("let _ = {}(&policy, 1);\n", removed_delay_helper()),
            4,
        ),
        ("src/q.rs".into(), format!("use {backoff_path}::x;\n"), 4),
        ("docs/r.md".into(), format!("see {facade_path}\n"), 4),
        (
            LEGACY_PATTERN_FILES[0].into(),
            scoped_source(&format!("let _ = tokio::time::{timeout}d, f);")),
            5,
        ),
        (
            LEGACY_PATTERN_FILES[1].into(),
            scoped_source(&format!("let t = self.{field};")),
            5,
        ),
        (
            CORE_BATTALION_FILES[0].into(),
            scoped_source(&format!("pub {field}: u64,")),
            6,
        ),
        (
            CORE_BATTALION_FILES[1].into(),
            scoped_source(&format!("pub {field_retry}: u32,")),
            6,
        ),
    ]
}

/// Every forbidden form of the two file-scoped rules, planted one at a time into every file the
/// rule covers: each must produce exactly one violation, naming the file and the rule.
fn assert_scoped_variants_are_caught() -> io::Result<()> {
    let timeout = timeout_call();
    let field = timeout_field();
    let [retry_policy, error_strategy, node_error] = removed_types();
    let [field_retry, field_strategy] = removed_fields();

    let rule_5 = [
        format!("let r = tokio::time::{timeout}d, f).await;"),
        format!("    {timeout}d, f).await"),
        format!("let t = self.{field};"),
    ];
    let rule_6 = [
        format!("pub {field}: u64,"),
        format!("pub {field_retry}: Policy,"),
        format!("pub {field_strategy}: Strategy,"),
        format!("pub enum {error_strategy} {{}}"),
        format!("pub struct {retry_policy} {{}}"),
        format!("pub type {retry_policy} = Other;"),
        format!("pub use other::Policy as {retry_policy};"),
        format!("pub struct {node_error} {{}}"),
        format!("pub type {node_error} = Other;"),
        format!("pub use other::Error as {node_error};"),
    ];

    for (files, rule, plants) in [
        (&LEGACY_PATTERN_FILES[..], 5_u8, &rule_5[..]),
        (&CORE_BATTALION_FILES[..], 6_u8, &rule_6[..]),
    ] {
        for relative in files {
            for plant in plants {
                let tree = clean_tree("scoped_variant")?;
                tree.write(relative, &scoped_source(plant))?;
                let violations = scan(&tree.root);
                assert_eq!(violations.len(), 1, "plant `{plant}`: {violations:#?}");
                assert!(
                    violations[0].starts_with(&format!("{relative}:"))
                        && violations[0].contains(&format!(": rule {rule}:")),
                    "plant `{plant}` in {relative}: {violations:#?}"
                );
            }
        }
    }
    Ok(())
}

/// A clean tree missing one scoped file: exactly one violation, naming the missing target. A guard
/// that cannot find its target proves nothing.
fn assert_missing_targets_are_reported() -> io::Result<()> {
    for relative in scoped_files() {
        let tree = clean_tree("missing_target")?;
        fs::remove_file(tree.root.join(relative))?;
        let violations = scan(&tree.root);
        assert_eq!(violations.len(), 1, "{relative}: {violations:#?}");
        assert!(
            violations[0].starts_with(&format!("{relative}: "))
                && violations[0].contains("target file is missing"),
            "missing {relative} not reported: {violations:#?}"
        );
    }
    Ok(())
}

/// Positive control: the scanner reports every planted legacy use, one per planted file, naming its
/// rule; every file-scoped form is caught; and a missing file-scoped target is reported.
#[test]
fn scanner_reports_planted_legacy_uses() -> io::Result<()> {
    let planted = plant_one_violation_per_rule();
    let tree = ScratchTree::new("planted")?;
    for relative in scoped_files().filter(|f| !planted.iter().any(|(path, _, _)| path == f)) {
        tree.write(relative, &scoped_source(""))?;
    }
    for (relative, content, _) in &planted {
        tree.write(relative, content)?;
    }

    let violations = scan(&tree.root);

    assert_eq!(violations.len(), planted.len(), "{violations:#?}");
    for (relative, _, rule) in &planted {
        let mine: Vec<&String> = violations
            .iter()
            .filter(|line| line.starts_with(&format!("{relative}:")))
            .collect();
        assert_eq!(
            mine.len(),
            1,
            "expected exactly one violation for {relative}: {violations:#?}"
        );
        assert!(
            mine[0].contains(&format!(": rule {rule}:")),
            "violation for {relative} does not name rule {rule}: {}",
            mine[0]
        );
    }

    assert_scoped_variants_are_caught()?;
    assert_missing_targets_are_reported()
}

/// Negative control: the live same-named types, the sibling vocabulary, the history files and the
/// runner's own `timeout(` are not violations.
#[test]
fn scanner_accepts_a_clean_tree() -> io::Result<()> {
    let tree = clean_tree("clean")?;

    let violations = scan(&tree.root);

    assert!(violations.is_empty(), "{violations:#?}");
    Ok(())
}

/// The guard: the repository itself names none of the removed Phase 44 items outside the history
/// files, and the legacy-pattern service files have no whole-run timeout site.
#[test]
fn legacy_surfaces_are_gone_from_the_tree() {
    let violations = scan(Path::new(env!("CARGO_MANIFEST_DIR")));

    assert!(
        violations.is_empty(),
        "Phase 44 removed these legacy surfaces (LEGACY-01/02/03, ADR-0059); remove these uses:\n{}",
        violations.join("\n")
    );
}
