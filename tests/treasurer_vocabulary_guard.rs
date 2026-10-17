//! ALLOW-05 vocabulary guard: `Treasurer` stays a framework-only word.
//!
//! ADR-0050 reserves the word `Treasurer` for the framework's own officer (the Treasurer that
//! admits, settles and halts runs against an allowance). A downstream application must be free to
//! model its own money domain without that word colliding with the framework's, so the framework
//! must never grow a *downstream* use of it, and must never borrow the downstream application's
//! fixture vocabulary for the framework role. This test is the guard ADR-0050's downstream
//! guardrail asks for.
//!
//! In this repository "downstream" means the code that stands in for a consuming application and
//! is not framework source:
//!
//! - `examples/` and `benches/` at the root, and `crates/<name>/examples/` and
//!   `crates/<name>/benches/` inside a crate;
//! - `fixtures/` at the root and any `tests/fixtures/` tree (at the root or inside a crate).
//!
//! Two rules are enforced over every UTF-8 file in the repository tree:
//!
//! 1. **Officer word.** A file under one of the downstream locations above must not contain the
//!    officer word as a whole word (case-sensitive).
//! 2. **Fixture term.** The downstream fixture term (the name the downstream application's own
//!    money-holding fixture uses) must not appear outside a small allowlist of files that document
//!    the guardrail: the one in-tree mention in the ledger module's docs, this test file, and the
//!    planning and project records under `.planning/` and `.project/`.
//!
//! The scanner is a plain `std::fs` walk (no new dependency). It skips `.git`, `target`,
//! `node_modules` and every other dot-directory except `.planning` and `.project`. Both forbidden
//! strings are assembled at run time so this file never matches its own scan.
//!
//! A guard that cannot fail proves nothing, so two positive controls run the same scanner over
//! small temporary trees: one with planted downstream uses (it must report all of them) and one
//! that is clean (it must report none). Only then does the repository scan carry weight.
//!
//! ```bash
//! cargo test --test treasurer_vocabulary_guard
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// The framework officer's name, assembled so this file never contains it as one token.
fn officer_word() -> String {
    ["Treas", "urer"].concat()
}

/// The downstream application's fixture term, assembled so this file never contains it.
fn fixture_term() -> String {
    ["Garrison", "Treasury"].concat()
}

/// Files allowed to contain the downstream fixture term: the one in-tree mention in the ledger
/// module's docs, and this guard itself.
const FIXTURE_TERM_ALLOWED_FILES: [&str; 2] = [
    "crates/paladin-core/src/platform/container/treasury_ledger.rs",
    "tests/treasurer_vocabulary_guard.rs",
];

/// Trees that document the guardrail and are therefore allowed to contain the fixture term.
const FIXTURE_TERM_ALLOWED_PREFIXES: [&str; 2] = [".planning/", ".project/"];

/// `true` for a directory the walk must not enter: build output, VCS data, vendored packages and
/// every dot-directory other than the two planning trees.
fn skip_directory(name: &str) -> bool {
    matches!(name, "target" | "node_modules")
        || (name.starts_with('.') && name != ".planning" && name != ".project")
}

/// `true` when `relative` (a `/`-separated path from the scan root) is a downstream location.
fn is_downstream_path(relative: &str) -> bool {
    if relative.starts_with("examples/")
        || relative.starts_with("benches/")
        || relative.starts_with("fixtures/")
        || relative.starts_with("tests/fixtures/")
        || relative.contains("/tests/fixtures/")
    {
        return true;
    }
    // crates/<name>/examples/... and crates/<name>/benches/...
    let mut parts = relative.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some("crates"), Some(_), Some("examples" | "benches"))
    ) && relative.matches('/').count() >= 3
}

/// `true` when `haystack` contains `word` delimited by non-identifier characters (or the ends).
fn contains_whole_word(haystack: &str, word: &str) -> bool {
    let is_identifier = |c: char| c.is_alphanumeric() || c == '_';
    haystack.match_indices(word).any(|(start, matched)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + matched.len()..].chars().next();
        !before.is_some_and(is_identifier) && !after.is_some_and(is_identifier)
    })
}

/// Walk `root` and report one `<relative path>: <rule>` line per violation, sorted.
///
/// Files that cannot be read, or that are not UTF-8, are skipped: a binary asset cannot carry the
/// vocabulary this guard polices.
fn scan(root: &Path) -> Vec<String> {
    let officer = officer_word();
    let fixture = fixture_term();
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
            if file_type.is_dir() {
                let name = entry.file_name();
                if !skip_directory(&name.to_string_lossy()) {
                    pending.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };

            if is_downstream_path(&relative) && contains_whole_word(&content, &officer) {
                violations.push(format!(
                    "{relative}: the framework-only word `{officer}` appears in a downstream location"
                ));
            }
            let allowed = FIXTURE_TERM_ALLOWED_FILES.contains(&relative.as_str())
                || FIXTURE_TERM_ALLOWED_PREFIXES
                    .iter()
                    .any(|prefix| relative.starts_with(prefix));
            if !allowed && content.contains(&fixture) {
                violations.push(format!(
                    "{relative}: the downstream fixture term `{fixture}` appears outside the guardrail allowlist"
                ));
            }
        }
    }

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
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let root = std::env::temp_dir().join(format!(
            "paladin_vocabulary_guard_{label}_{}_{}_{nanos}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&root).expect("create scratch tree root");
        Self { root }
    }

    /// Write `content` at `relative` (a `/`-separated path), creating parent directories.
    fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create scratch parent directory");
        }
        fs::write(&path, content).expect("write scratch file");
    }
}

impl Drop for ScratchTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Positive control: the scanner reports every planted downstream use, with its path.
#[test]
fn scanner_reports_a_planted_downstream_use() {
    let officer = officer_word();
    let fixture = fixture_term();
    let tree = ScratchTree::new("planted");
    tree.write(
        "examples/demo.rs",
        &format!("let officer = {officer}::new();\n"),
    );
    tree.write("benches/b.rs", &format!("// bench the {officer}\n"));
    tree.write("crates/x/examples/e.rs", &format!("use x::{officer};\n"));
    tree.write("src/lib.rs", &format!("struct {fixture};\n"));

    let violations = scan(&tree.root);

    assert_eq!(violations.len(), 4, "{violations:#?}");
    for expected in [
        "examples/demo.rs",
        "benches/b.rs",
        "crates/x/examples/e.rs",
        "src/lib.rs",
    ] {
        assert!(
            violations
                .iter()
                .any(|line| line.starts_with(&format!("{expected}: "))),
            "no violation reported for {expected}: {violations:#?}"
        );
    }
}

/// Negative control: framework source, docs and the allowlisted records are not violations, and
/// neither is a longer identifier that merely contains the officer word.
#[test]
fn scanner_accepts_a_clean_tree() {
    let officer = officer_word();
    let fixture = fixture_term();
    let tree = ScratchTree::new("clean");
    tree.write("src/lib.rs", &format!("pub struct {officer};\n"));
    tree.write("crates/x/src/lib.rs", &format!("pub fn {officer}() {{}}\n"));
    tree.write("docs/guide.md", &format!("The {officer} halts a run.\n"));
    tree.write(
        "crates/paladin-core/src/platform/container/treasury_ledger.rs",
        &format!("//! Not the downstream {fixture}.\n"),
    );
    tree.write(
        ".planning/decisions/0050.md",
        &format!("{fixture} and {officer}\n"),
    );
    tree.write(".project/overview.md", &format!("{fixture}\n"));
    // A longer identifier is not the whole word, even in a downstream location.
    tree.write(
        "examples/ok.rs",
        &format!("let _ = {officer}Config::new();\n"),
    );
    // Build output and hidden directories are never entered.
    tree.write("target/debug/examples/skipped.rs", &format!("{fixture}\n"));
    tree.write(".hidden/notes.md", &format!("{fixture}\n"));

    let violations = scan(&tree.root);

    assert!(violations.is_empty(), "{violations:#?}");
}

/// The guard: the repository itself has no downstream use of the officer word and no stray use of
/// the downstream fixture term.
#[test]
fn treasurer_is_a_framework_only_word() {
    let violations = scan(Path::new(env!("CARGO_MANIFEST_DIR")));

    assert!(
        violations.is_empty(),
        "`{}` is a framework-only word (ALLOW-05, ADR-0050); remove these uses:\n{}",
        officer_word(),
        violations.join("\n")
    );
}
