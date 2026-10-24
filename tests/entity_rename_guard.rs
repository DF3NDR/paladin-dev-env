//! Phase 48 entity-rename guard (ENTITY-01, ENTITY-02, ENTITY-03, ENTITY-05, ADR-0060): the base
//! wrapper stays `Entity<T>`, the old wrapper and version-family names stay gone, and the graph
//! vocabulary stays frozen.
//!
//! Phase 48 renamed the base-layer wrapper to `Entity<T>` (payload field `data`), renamed the
//! version family to `EntityVersion*`, unified the citadel module's placeholder `PaladinData` /
//! `PaladinStatus` / `Paladin` with the real Paladin types, and kept the five unused base items
//! (`Field`, `Collection`, `Element` and their two version services) pending review. "Node" now
//! means only a WarGraph/Campaign vertex (ADR-0060). This test is the check that a later change
//! cannot quietly bring an old name back or move a graph definition.
//!
//! Five rules run over the repository tree (std only, no new dependency):
//!
//! 1. **Old paths.** The old wrapper module path and the old version-module path, each ending at
//!    an identifier boundary, and their slash forms (`entity/<old wrapper>.rs`,
//!    `service/<old version module>.rs`), in any scanned file.
//! 2. **Old identifiers.** The old wrapper name immediately followed by a generic opener, `::new`
//!    or `::<` (with a non-identifier character before it, so a state-node constructor or a node
//!    identifier constructor passes), the old version-family prefix anywhere (which catches every
//!    member of that family, including the in-memory repository), and the old per-vertex version
//!    cap name.
//! 3. **Entity-sense fields.** In the production code of the wrapper file and the eight container
//!    files (`paladin.rs`, `citadel.rs`, `battalion/council.rs`, `battalion/grove.rs`, `user.rs`,
//!    `log.rs`, `content.rs`, `prompt.rs`) no `.node` field access and no `node:` field; in the
//!    production code of the renamed version module no old not-found variant and no old
//!    identifier or payload field read or declaration. The serde read-alias strings do not match
//!    these needles.
//! 4. **Single declarations.** Among scanned `.rs` files, in code only (comments, string literals
//!    and character literals are blanked first, so a block comment or a raw string cannot hold
//!    one), the Paladin alias, the Paladin data struct and the Paladin status enum each occur
//!    exactly once, in `paladin.rs`; the structured `NodeError` exactly once, in `node_error.rs`;
//!    the `Entity` wrapper exactly once, in `base/entity/wrapper.rs`. A line is normalized before
//!    it is compared (whitespace collapsed, `pub(crate)` and the other restricted visibilities
//!    read as `pub`, no space before `<`), so a respelled duplicate still counts. Zero is a
//!    violation just like two, or a wrong file.
//! 5. **Graph freeze.** Every graph definition (`NodeId`, `NodeSpec`, `StateNode`, `NodeContext`,
//!    `NodeInterceptor`, `NodeDoc`, `NodeExecutionRecord`, `NodeOutcomeKind`, `NodeProgressKind`,
//!    the `TraceEvent::Node*` variants, the node-cache port, key, error, in-memory and Redis
//!    adapters and the `node_cache` config section) is still declared in its current file: a
//!    production line there must begin with the declaration (after indentation), so a match arm, a
//!    comment, a test or an example that merely repeats the text does not count. The `nodes`
//!    property and the `LimitsDoc.max_node_visits` property of `wargraph-doc.schema.json` are
//!    looked up in the parsed document, not searched for as text. A moved or renamed definition
//!    fails.
//!
//! The five retained base items are NOT banned (D-07 (d)): the paths of `base::entity::field`,
//! `base::entity::collection`, `base::component::element` and the two retained services, and the
//! entity-sense residuals inside them that ADR-0060 lists as the bounded exception (`Field.nid`,
//! the field-version `node_id` fields and `get_node_field_versions`), are never matched.
//!
//! Scanned files end in `.rs`, `.md`, `.toml`, `.yaml` or `.yml`. The only things skipped are the
//! declared history set: `target`, `node_modules` and the dot-directories `.git`, `.planning` and
//! `.project` (every other dot-directory is scanned, so `.github/` and `.claude/`, which carry the
//! project's convention text, are covered), every `CHANGELOG.md`, the root `MIGRATION.md`, the
//! two upgrade guides under `docs/src/api-reference/`, the semver allowlist
//! `.cargo/semver-checks-allowlist.toml` (it mirrors `MIGRATION.md` and so names the removed items
//! by necessity), and this file. A hit is fixed at its source; it is never silenced by growing
//! that set.
//!
//! The forbidden strings are assembled at run time with `concat`, so this file never contains one
//! whole. A guard that cannot fail proves nothing, so the positive controls run the same scanner
//! over temporary trees: one with a planted violation per rule variant (each must be reported,
//! naming its rule), one with each file-scoped target removed in turn (each removal must be
//! reported, never skipped), one full of look-alikes, retained-item paths and history files
//! (nothing may be reported) and one with none of the directories `.dockerignore` excludes
//! (`docs/`, `notes/`, `k8s/`, `.github/`), the shape of the Docker test image's build context (the
//! graph schema is not demanded there, and the skip prints a notice; nothing may be reported). A
//! tree that lacks only `docs/` is not that image, and its missing schema is reported.
//!
//! ```bash
//! cargo test --test entity_rename_guard
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// The base wrapper file: rule 3 applies to its production code and rule 4 homes `Entity` here.
const WRAPPER_FILE: &str = "crates/paladin-core/src/base/entity/wrapper.rs";

/// The wrapper file and the eight container files built on it: rule 3 (entity-sense fields)
/// applies to their production code.
const ENTITY_FIELD_FILES: [&str; 9] = [
    WRAPPER_FILE,
    "crates/paladin-core/src/platform/container/paladin.rs",
    "crates/paladin-core/src/platform/container/citadel.rs",
    "crates/paladin-core/src/platform/container/battalion/council.rs",
    "crates/paladin-core/src/platform/container/battalion/grove.rs",
    "crates/paladin-core/src/platform/container/user.rs",
    "crates/paladin-core/src/platform/container/log.rs",
    "crates/paladin-core/src/platform/container/content.rs",
    "crates/paladin-core/src/platform/container/prompt.rs",
];

/// The renamed version module: rule 3 (old identifiers and fields) applies to its production code.
const VERSION_SERVICE_FILE: &str = "crates/paladin-core/src/base/service/entity_version_service.rs";

/// Where `Paladin`, `PaladinData` and `PaladinStatus` are declared (rule 4).
const PALADIN_FILE: &str = "crates/paladin-core/src/platform/container/paladin.rs";

/// Where the structured `NodeError` is declared (rule 4).
const NODE_ERROR_FILE: &str = "crates/paladin-core/src/platform/container/node_error.rs";

/// A declaration that must occur exactly once in the workspace, in its home file (rule 4).
struct SingleDeclaration {
    /// The text searched for, with an identifier boundary before it.
    needle: &'static str,
    /// Whether the text must also end at an identifier boundary (a trailing `<` or `=` ends it).
    bound_end: bool,
    /// The one file that declares it.
    home: &'static str,
    /// A clean declaration line, for the positive controls.
    line: &'static str,
}

/// The rule 4 table.
const SINGLE_DECLARATIONS: [SingleDeclaration; 5] = [
    SingleDeclaration {
        needle: "pub type Paladin =",
        bound_end: false,
        home: PALADIN_FILE,
        line: "pub type Paladin = Entity<PaladinData>;",
    },
    SingleDeclaration {
        needle: "pub struct PaladinData",
        bound_end: true,
        home: PALADIN_FILE,
        line: "pub struct PaladinData {}",
    },
    SingleDeclaration {
        needle: "pub enum PaladinStatus",
        bound_end: true,
        home: PALADIN_FILE,
        line: "pub enum PaladinStatus {}",
    },
    SingleDeclaration {
        needle: "pub struct NodeError",
        bound_end: true,
        home: NODE_ERROR_FILE,
        line: "pub struct NodeError {}",
    },
    SingleDeclaration {
        needle: "pub struct Entity<",
        bound_end: false,
        home: WRAPPER_FILE,
        line: "pub struct Entity<T> { pub data: T }",
    },
];

/// The graph schema, read by explicit path: its `.json` extension is not otherwise scanned.
const SCHEMA_FILE: &str = "docs/schemas/wargraph-doc.schema.json";

/// The schema keys rule 5 pins, as JSON pointers into the parsed document: the `nodes` property
/// and the `max_node_visits` property of `LimitsDoc`.
const SCHEMA_KEYS: [&str; 2] = [
    "/properties/nodes",
    "/$defs/LimitsDoc/properties/max_node_visits",
];

/// The directories `.dockerignore` keeps out of the Docker test image's build context that this
/// repository also has. Only a tree with none of them is treated as that image.
const DOCKER_EXCLUDED_DIRS: [&str; 4] = ["docs", "notes", "k8s", ".github"];

/// The rule 5 table: each file and the graph declarations it must still begin a production line
/// with.
const GRAPH_DECLARATIONS: [(&str, &[&str]); 11] = [
    (
        "crates/paladin-core/src/platform/container/waypoint.rs",
        &[
            "pub struct NodeId(",
            "pub enum NodeOutcomeKind",
            "pub struct NodeExecutionRecord",
        ],
    ),
    (
        "crates/paladin-core/src/platform/container/trace.rs",
        &[
            "pub enum NodeProgressKind",
            "NodeStarted {",
            "NodeProgress {",
            "NodeFinished {",
        ],
    ),
    (
        "crates/paladin-battalion/src/engine/graph.rs",
        &["pub enum NodeSpec"],
    ),
    (
        "crates/paladin-battalion/src/engine/node.rs",
        &["pub struct NodeContext", "pub trait StateNode"],
    ),
    (
        "crates/paladin-battalion/src/engine/hooks.rs",
        &["pub trait NodeInterceptor"],
    ),
    (
        "crates/paladin-battalion/src/engine/graph_doc.rs",
        &["pub struct NodeDoc"],
    ),
    (
        "crates/paladin-ports/src/output/node_cache_port.rs",
        &[
            "pub struct NodeCacheKey(",
            "pub enum NodeCacheError",
            "pub trait NodeCachePort",
        ],
    ),
    (
        "crates/paladin-storage/src/node_cache/in_memory.rs",
        &["pub struct InMemoryNodeCache"],
    ),
    (
        "crates/paladin-storage/src/node_cache/redis.rs",
        &[
            "pub struct RedisNodeCache {",
            "pub struct RedisNodeCacheConfig",
        ],
    ),
    (
        "src/config/node_cache.rs",
        &["pub struct NodeCacheConfig", "pub enum NodeCacheBackend"],
    ),
    ("src/config/mod.rs", &["pub mod node_cache;"]),
];

/// Version-to-version history files: they must name the old items, so they are not scanned.
/// Every `CHANGELOG.md` is skipped by name in addition to these exact paths.
const HISTORY_FILES: [&str; 4] = [
    "MIGRATION.md",
    ".cargo/semver-checks-allowlist.toml",
    "docs/src/api-reference/migration-guide.md",
    "docs/src/api-reference/upgrading.md",
];

/// The dot-directories the walk skips, by exact name. Every other one is scanned, so `.github/`
/// (the project's convention source of truth), `.cargo/` and the like cannot quietly bring an old
/// name back.
const SKIPPED_DOT_DIRECTORIES: [&str; 3] = [".git", ".planning", ".project"];

/// This guard: it describes the old names, so it is not scanned (its strings are also assembled
/// at run time).
const GUARD_FILE: &str = "tests/entity_rename_guard.rs";

/// File extensions the scan reads.
const SCANNED_EXTENSIONS: [&str; 5] = ["rs", "md", "toml", "yaml", "yml"];

/// The old wrapper's lowercase module name.
fn old_lower() -> String {
    ["no", "de"].concat()
}

/// The old wrapper's type name.
fn old_upper() -> String {
    ["No", "de"].concat()
}

/// The old wrapper module path and the old version-module path (rule 1), each checked up to an
/// identifier boundary.
fn old_paths() -> [String; 2] {
    [
        ["base::entity::", &old_lower()].concat(),
        ["base::service::", &old_lower(), "_version_service"].concat(),
    ]
}

/// The slash forms of the two old module files (rule 1), each checked from an identifier boundary.
fn old_slash_paths() -> [String; 2] {
    [
        ["entity/", &old_lower(), ".rs"].concat(),
        ["service/", &old_lower(), "_version_service.rs"].concat(),
    ]
}

/// The old wrapper name followed by a generic opener, `::new` or `::<` (rule 2), each checked from
/// an identifier boundary.
fn old_generic_forms() -> [String; 3] {
    [
        [&old_upper(), "<"].concat(),
        [&old_upper(), "::new"].concat(),
        [&old_upper(), "::<"].concat(),
    ]
}

/// The old version-family prefix (rule 2), matched with no boundary so every member matches.
fn old_family_prefix() -> String {
    [&old_upper(), "Version"].concat()
}

/// The old per-vertex version cap (rule 2), matched as a whole word.
fn old_version_cap() -> String {
    ["max_versions_per_", &old_lower()].concat()
}

/// The old not-found variant of the version module (rule 3).
fn old_not_found() -> String {
    [&old_upper(), "NotFound"].concat()
}

/// `true` for characters that continue an identifier.
fn is_identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `true` when `needle` occurs in `haystack` with the requested boundaries: `bound_start` demands a
/// non-identifier character (or the line start) before it, `bound_end` one after it (or the end).
/// Matching is exact and case-sensitive: no case folding, no Unicode normalization.
fn occurs(haystack: &str, needle: &str, bound_start: bool, bound_end: bool) -> bool {
    haystack.match_indices(needle).any(|(start, matched)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + matched.len()..].chars().next();
        (!bound_start || !before.is_some_and(is_identifier))
            && (!bound_end || !after.is_some_and(is_identifier))
    })
}

/// Rules 1 and 2 for one line: every `(rule number, matched text)` it trips.
fn generic_hits(line: &str) -> Vec<(u8, String)> {
    let mut hits = Vec::new();
    for path in old_paths() {
        if occurs(line, &path, false, true) {
            hits.push((1, path));
        }
    }
    for path in old_slash_paths() {
        if occurs(line, &path, true, false) {
            hits.push((1, path));
        }
    }
    for form in old_generic_forms() {
        if occurs(line, &form, true, false) {
            hits.push((2, form));
        }
    }
    let prefix = old_family_prefix();
    if occurs(line, &prefix, false, false) {
        hits.push((2, prefix));
    }
    let cap = old_version_cap();
    if occurs(line, &cap, true, true) {
        hits.push((2, cap));
    }
    hits
}

/// A declaration line in a canonical spelling, so formatting and restricted visibility cannot hide
/// it from rule 4: whitespace collapsed, `pub(crate)`, `pub(super)`, `pub(self)` and `pub(in ..)`
/// read as `pub`, no space before `<`, and exactly one space before `=`.
fn normalize_declaration(line: &str) -> String {
    let mut text = line.split_whitespace().collect::<Vec<_>>().join(" ");
    while let Some(start) = text.find("pub(") {
        let Some(length) = text[start..].find(')') else {
            break;
        };
        text.replace_range(start + 3..=start + length, "");
    }
    text.replace(" <", "<")
        .replace(" =", "=")
        .replace('=', " =")
}

/// The end (exclusive, in chars) of the raw string literal starting at `start`, if one does.
fn raw_string_end(chars: &[char], start: usize) -> Option<usize> {
    if start > 0 && is_identifier(chars[start - 1]) {
        return None;
    }
    let mut at = start;
    if chars.get(at) == Some(&'b') {
        at += 1;
    }
    if chars.get(at) != Some(&'r') {
        return None;
    }
    at += 1;
    let hashes = chars[at..].iter().take_while(|c| **c == '#').count();
    at += hashes;
    if chars.get(at) != Some(&'"') {
        return None;
    }
    at += 1;
    while at < chars.len() {
        if chars[at] == '"' && chars[at + 1..].iter().take_while(|c| **c == '#').count() >= hashes {
            return Some(at + 1 + hashes);
        }
        at += 1;
    }
    Some(chars.len())
}

/// `content` with comments removed and string and character literals emptied, newlines kept, so
/// line `n` of the result is line `n` of the input. It lets rule 4 see only code: a declaration
/// inside a block comment, a doc example, or a raw or ordinary string is not a declaration.
fn blank_non_code(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut out = String::with_capacity(content.len());
    let mut at = 0;
    while at < chars.len() {
        let current = chars[at];
        let next = chars.get(at + 1).copied();
        if current == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
        } else if current == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[at] == '\n' {
                        out.push('\n');
                    }
                    at += 1;
                }
            }
            out.push(' ');
        } else if let Some(end) = raw_string_end(&chars, at) {
            out.extend(chars[at..end].iter().filter(|c| **c == '\n'));
            out.push_str("\"\"");
            at = end;
        } else if current == '"' {
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' {
                    at += 1;
                }
                if chars.get(at) == Some(&'\n') {
                    out.push('\n');
                }
                at += 1;
            }
            at += 1;
            out.push_str("\"\"");
        } else if current == '\'' && next == Some('\\') {
            at += 3;
            while at < chars.len() && chars[at] != '\'' {
                at += 1;
            }
            at += 1;
            out.push_str("''");
        } else if current == '\'' && chars.get(at + 2) == Some(&'\'') {
            at += 3;
            out.push_str("''");
        } else {
            out.push(current);
            at += 1;
        }
    }
    out
}

/// Rule 3 for one production line of the wrapper file or a container file.
fn entity_field_hits(line: &str) -> Vec<(u8, String)> {
    let mut hits = Vec::new();
    if occurs(line, ".node", false, true) {
        hits.push((3, ".node".to_string()));
    }
    if occurs(line, "node:", true, false) {
        hits.push((3, "node:".to_string()));
    }
    hits
}

/// Rule 3 for one production line of the renamed version module.
fn version_service_hits(line: &str) -> Vec<(u8, String)> {
    let mut hits = Vec::new();
    let not_found = old_not_found();
    if occurs(line, &not_found, false, false) {
        hits.push((3, not_found));
    }
    for field in ["node_id:", "node_data:"] {
        if occurs(line, field, true, false) {
            hits.push((3, field.to_string()));
        }
    }
    for read in [".node_id", ".node_data"] {
        if occurs(line, read, false, true) {
            hits.push((3, read.to_string()));
        }
    }
    hits
}

/// `true` for the line that opens an inline test module: `mod tests {` or `mod test {`, with an
/// optional visibility qualifier. Matching the whole item (not a prefix) keeps
/// `mod tests_support {` and the out-of-line `mod tests;` from ending the scan early and leaving
/// the rest of the file unread.
fn opens_test_module(line: &str) -> bool {
    let item = line.trim();
    let item = ["pub(crate) ", "pub(super) ", "pub "]
        .iter()
        .find_map(|visibility| item.strip_prefix(visibility))
        .unwrap_or(item);
    item == "mod tests {" || item == "mod test {"
}

/// The non-comment lines of `content` before the first inline test module, with 1-based line
/// numbers.
fn production_lines(content: &str) -> impl Iterator<Item = (usize, &str)> {
    content
        .lines()
        .enumerate()
        .take_while(|(_, line)| !opens_test_module(line))
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(index, line)| (index + 1, line))
}

/// Apply a file-scoped rule 3 `hits` function to every file in `files` under `root`. A target that
/// cannot be read, or that holds no production line at all, is itself a violation: a guard that
/// cannot find its target proves nothing.
fn scan_scoped(
    root: &Path,
    files: &[&str],
    hits: fn(&str) -> Vec<(u8, String)>,
    violations: &mut Vec<String>,
) {
    for relative in files {
        match fs::read_to_string(root.join(relative)) {
            Ok(content) => {
                if production_lines(&content).next().is_none() {
                    violations.push(format!(
                        "{relative}: rule 3: target file has no production lines, \
                         so the rule proves nothing"
                    ));
                }
                for (number, line) in production_lines(&content) {
                    for (hit_rule, matched) in hits(line) {
                        violations.push(format!(
                            "{relative}:{number}: rule {hit_rule}: `{matched}` in production code"
                        ));
                    }
                }
            }
            Err(error) => violations.push(format!(
                "{relative}: rule 3: target file is missing or unreadable ({error}), \
                 so the rule proves nothing"
            )),
        }
    }
}

/// `true` when `line` declares `needle`: the needle starts the line (after indentation) and, when
/// it ends in an identifier character, is not the prefix of a longer name.
fn declares(line: &str, needle: &str) -> bool {
    let line = line.trim_start();
    line.starts_with(needle)
        && (needle.ends_with(|c: char| !is_identifier(c))
            || !line[needle.len()..]
                .chars()
                .next()
                .is_some_and(is_identifier))
}

/// Rule 5: every file in `GRAPH_DECLARATIONS` must exist and still declare each of its items on a
/// production line. A line that only repeats the text (a match arm, a comment, a test, an example)
/// is not a declaration, so deleting the real one cannot pass.
fn scan_declared(root: &Path, violations: &mut Vec<String>) {
    for (relative, needles) in GRAPH_DECLARATIONS {
        match fs::read_to_string(root.join(relative)) {
            Ok(content) => {
                for needle in needles {
                    if !production_lines(&content).any(|(_, line)| declares(line, needle)) {
                        violations.push(format!(
                            "{relative}: rule 5: graph declaration `{needle}` is no longer here \
                             (a moved or renamed definition)"
                        ));
                    }
                }
            }
            Err(error) => violations.push(format!(
                "{relative}: rule 5: target file is missing or unreadable ({error}), \
                 so the rule proves nothing"
            )),
        }
    }
}

/// `true` when `root` holds none of `DOCKER_EXCLUDED_DIRS`: the shape of the Docker test image.
fn is_docker_build_context(root: &Path) -> bool {
    DOCKER_EXCLUDED_DIRS
        .iter()
        .all(|directory| !root.join(directory).exists())
}

/// Rule 5 for the graph schema: the parsed document must still define each key in `SCHEMA_KEYS`.
/// A key that only appears elsewhere in the file (an example, the `required` list) does not count.
///
/// The one exception is the Docker test image's build context. `.dockerignore` excludes `docs/`,
/// `notes/`, `k8s/` and `.github/` from it, so the schema cannot be there and demanding it fails
/// `cargo test` in that image for a reason unrelated to the freeze. Only a tree holding none of
/// `DOCKER_EXCLUDED_DIRS` is treated as that image, so a checkout that merely lacks `docs/` (a
/// sparse checkout, a packaged tree) is still reported. The skip prints a notice (shown with
/// `--nocapture`, or when any test fails) rather than passing silently. In that image the schema
/// freeze is not pinned at all; it is enforced by the workspace test run in the checkout.
fn scan_schema(root: &Path, violations: &mut Vec<String>) {
    if is_docker_build_context(root) {
        eprintln!(
            "rule 5: schema check skipped, no {DOCKER_EXCLUDED_DIRS:?} directory in this build \
             context (the Docker test image); {SCHEMA_FILE} is not checked here"
        );
        return;
    }
    let content = match fs::read_to_string(root.join(SCHEMA_FILE)) {
        Ok(content) => content,
        Err(error) => {
            violations.push(format!(
                "{SCHEMA_FILE}: rule 5: target file is missing or unreadable ({error}), \
                 so the rule proves nothing"
            ));
            return;
        }
    };
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(document) => {
            for key in SCHEMA_KEYS {
                if document.pointer(key).is_none() {
                    violations.push(format!(
                        "{SCHEMA_FILE}: rule 5: schema key `{key}` is no longer defined \
                         (a moved or renamed definition)"
                    ));
                }
            }
        }
        Err(error) => violations.push(format!(
            "{SCHEMA_FILE}: rule 5: not valid JSON ({error}), so the rule proves nothing"
        )),
    }
}

/// Rule 4: report every declaration that is absent from its home, repeated in it, or present
/// anywhere else. `found` holds, per `SINGLE_DECLARATIONS` entry, the `(file, line)` locations.
fn report_declaration_counts(found: &[Vec<(String, usize)>], violations: &mut Vec<String>) {
    for (declaration, locations) in SINGLE_DECLARATIONS.iter().zip(found) {
        let (needle, home) = (declaration.needle, declaration.home);
        let here: Vec<&(String, usize)> =
            locations.iter().filter(|(path, _)| path == home).collect();
        if here.is_empty() {
            violations.push(format!(
                "{home}: rule 4: `{needle}` is declared 0 times here, expected exactly once \
                 (a count of zero proves nothing)"
            ));
        }
        for (path, number) in here.into_iter().skip(1) {
            violations.push(format!(
                "{path}:{number}: rule 4: `{needle}` is declared again in its home file"
            ));
        }
        for (path, number) in locations.iter().filter(|(path, _)| path != home) {
            violations.push(format!(
                "{path}:{number}: rule 4: `{needle}` is declared outside {home}, \
                 expected exactly once there"
            ));
        }
    }
}

/// `true` for a directory the walk must not enter: build output, vendored packages and the named
/// dot-directories (version control, `.planning`, `.project`).
fn skip_directory(name: &str) -> bool {
    matches!(name, "target" | "node_modules") || SKIPPED_DOT_DIRECTORIES.contains(&name)
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
    let mut found: Vec<Vec<(String, usize)>> = vec![Vec::new(); SINGLE_DECLARATIONS.len()];
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // `DirEntry::file_type` does not follow symlinks, so a link can never loop the walk or
            // lead it out of the repository.
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
            let extension = path.extension().and_then(|extension| extension.to_str());
            let scanned =
                extension.is_some_and(|extension| SCANNED_EXTENSIONS.contains(&extension));
            let is_rust = extension == Some("rs");
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
            let code = if is_rust {
                blank_non_code(&content)
            } else {
                String::new()
            };
            let mut code_lines = code.lines();
            for (index, line) in content.lines().enumerate() {
                let code_line = code_lines.next().unwrap_or_default();
                let number = index + 1;
                for (rule, matched) in generic_hits(line) {
                    violations.push(format!(
                        "{relative}:{number}: rule {rule}: removed Phase 48 name `{matched}`"
                    ));
                }
                if is_rust {
                    let normalized = normalize_declaration(code_line);
                    for (declaration, locations) in SINGLE_DECLARATIONS.iter().zip(&mut found) {
                        if occurs(&normalized, declaration.needle, true, declaration.bound_end) {
                            locations.push((relative.clone(), number));
                        }
                    }
                }
            }
        }
    }

    scan_scoped(
        root,
        &ENTITY_FIELD_FILES,
        entity_field_hits,
        &mut violations,
    );
    scan_scoped(
        root,
        &[VERSION_SERVICE_FILE],
        version_service_hits,
        &mut violations,
    );
    report_declaration_counts(&found, &mut violations);
    scan_declared(root, &mut violations);
    scan_schema(root, &mut violations);

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
            "paladin_entity_rename_guard_{label}_{}_{}_{nanos}",
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

/// Every file-scoped target the scanner reads, once each: rule 3 files, rule 5 files and the
/// rule 4 home files.
fn all_targets() -> Vec<&'static str> {
    let mut targets: Vec<&'static str> = ENTITY_FIELD_FILES.to_vec();
    targets.push(VERSION_SERVICE_FILE);
    targets.extend(GRAPH_DECLARATIONS.iter().map(|(path, _)| *path));
    targets.push(SCHEMA_FILE);
    for declaration in &SINGLE_DECLARATIONS {
        if !targets.contains(&declaration.home) {
            targets.push(declaration.home);
        }
    }
    targets
}

/// `true` for a rule 3 target (entity-sense fields or the version module).
fn is_rule_3_target(relative: &str) -> bool {
    ENTITY_FIELD_FILES.contains(&relative) || relative == VERSION_SERVICE_FILE
}

/// How many reports removing `relative` must produce: one per rule it is a target of, plus one per
/// rule 4 declaration homed there (their count drops to zero).
fn expected_reports_for_removal(relative: &str) -> usize {
    usize::from(is_rule_3_target(relative))
        + usize::from(
            relative == SCHEMA_FILE || GRAPH_DECLARATIONS.iter().any(|(path, _)| *path == relative),
        )
        + SINGLE_DECLARATIONS
            .iter()
            .filter(|declaration| declaration.home == relative)
            .count()
}

/// Clean content for a wrapper or container file: allowed production code with `declared` and
/// `extra` among it, then a test module that legitimately reads and writes the old v0.10 keys.
fn entity_source(declared: &str, extra: &str) -> String {
    format!(
        "//! Docs may mention .node and node: freely.\n\
         /// So may item docs: .node, node:.\n\
         // self.node.x and node: T in a comment\n\
         #[serde(alias = \"node\")]\n\
         {declared}\
         pub fn run() {{}}\n\
         {extra}\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20   fn fixture() {{\n\
         \x20       let _ = old.node;\n\
         \x20       let _ = Legacy {{ node: 1 }};\n\
         \x20   }}\n\
         }}\n"
    )
}

/// Clean content for the renamed version module: the serde read-aliases for the old keys are
/// allowed, and so is the old vocabulary inside the test module.
fn version_source(extra: &str) -> String {
    let not_found = old_not_found();
    format!(
        "//! Docs may mention {not_found}, node_id: and .node_data freely.\n\
         // {not_found} node_id: node_data: in a comment\n\
         #[serde(alias = \"node_id\")]\n\
         pub entity_id: String,\n\
         #[serde(alias = \"node_data\")]\n\
         pub entity_data: T,\n\
         {extra}\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20   fn fixture() {{\n\
         \x20       let _ = {not_found};\n\
         \x20       let _ = v.node_id;\n\
         \x20       let _ = Old {{ node_id: 1, node_data: 2 }};\n\
         \x20   }}\n\
         }}\n"
    )
}

/// A clean graph schema: every pinned key defined, and the same names repeated where they do not
/// count (the `required` list and a `default` example). `skip` removes one pinned key (a JSON
/// pointer from `SCHEMA_KEYS`), leaving only those repeats behind.
fn schema_source(skip: Option<&str>) -> String {
    let mut document = serde_json::json!({
        "properties": { "nodes": {}, "edges": {} },
        "required": ["nodes", "edges"],
        "$defs": {
            "LimitsDoc": {
                "properties": { "max_node_visits": {} },
                "default": { "max_node_visits": 100 }
            }
        }
    });
    let removed = skip.and_then(|pointer| {
        let (parent, key) = pointer.rsplit_once('/')?;
        document.pointer_mut(parent)?.as_object_mut()?.remove(key)
    });
    debug_assert_eq!(removed.is_some(), skip.is_some(), "skip names a pinned key");
    document.to_string()
}

/// Clean content for any scoped target, with `extra` planted among it. `skip` leaves one
/// declaration needle (or, for the schema, one pinned key) out: a deleted or moved declaration.
fn clean_content_skipping(relative: &str, extra: &str, skip: Option<&str>) -> String {
    let keep = |needle: &str| skip != Some(needle);
    let declared: String = SINGLE_DECLARATIONS
        .iter()
        .filter(|declaration| declaration.home == relative && keep(declaration.needle))
        .map(|declaration| format!("{}\n", declaration.line))
        .collect();
    if ENTITY_FIELD_FILES.contains(&relative) {
        entity_source(&declared, extra)
    } else if relative == SCHEMA_FILE {
        schema_source(skip)
    } else if relative == VERSION_SERVICE_FILE {
        version_source(extra)
    } else if let Some((_, needles)) = GRAPH_DECLARATIONS
        .iter()
        .find(|(path, _)| *path == relative)
    {
        let lines: String = needles
            .iter()
            .filter(|needle| keep(needle))
            .map(|needle| format!("{needle}\n"))
            .collect();
        format!("{lines}{extra}\n")
    } else {
        format!("{declared}{extra}\n")
    }
}

/// Clean content for any scoped target, with `extra` planted among it.
fn clean_content(relative: &str, extra: &str) -> String {
    clean_content_skipping(relative, extra, None)
}

/// All forbidden strings, one per line: what a history file names. It also declares the rule 4
/// types again, to prove a skipped file is never counted.
fn every_old_name() -> String {
    let mut names = Vec::new();
    names.extend(old_paths());
    names.extend(old_slash_paths());
    names.extend(old_generic_forms().map(|form| format!("let _ = {form}(1);")));
    names.push(["let _ = ", &old_family_prefix(), "Service;"].concat());
    names.push(old_version_cap());
    names.push(old_not_found());
    names.extend(
        SINGLE_DECLARATIONS
            .iter()
            .map(|declaration| declaration.line.to_string()),
    );
    names.join("\n") + "\n"
}

/// Look-alikes the rules must not match: graph-sense and neighbouring names that share a prefix
/// or a spelling with an old name, plus the allowed `Entity` paths.
fn look_alikes() -> String {
    [
        "use paladin_core::base::entity::Entity;",
        "use paladin_core::base::entity::message::Message;",
        "use paladin_core::platform::container::node_error::NodeError;",
        "let _ = StateNode::new(spec);",
        "let _ = NodeId::new(\"a\");",
        "let _ = NodeCacheKey::new(\"k\");",
        "let _ = BattalionError::Node(e);",
        "let _ = EngineError::Node;",
        "let _ = graph.node(id);",
        "let _ = progress.node;",
        "let node_id = 1;",
        "let _: Arc<dyn StateNode> = x;",
        "let _: Vec<NodeId> = ids;",
        "let _ = EntityVersion::<u8>::new(1);",
        "let _ = max_versions_per_entity;",
        "// Node-cache reads and a bare `Node` in prose are graph-sense vocabulary.",
        "// pub type Paladin = Other; and pub struct Entity<T> are only commented here.",
        "/// pub struct NodeError {} in a doc comment is not a declaration.",
        "/* a block comment\npub struct NodeError {}\n/* nested */ pub enum PaladinStatus {}\n*/",
        "let _ = r#\"\npub struct Entity<T> { pub data: T }\npub type Paladin = X;\n\"#;",
        "let _ = \"pub struct PaladinData {}\";",
        "fn lifetimes<'a>(x: &'a str) -> char { let _ = '\"'; let _ = '\\''; 'x' }",
    ]
    .join("\n")
        + "\n"
}

/// The retained base items: their paths and the entity-sense residuals inside them are not banned
/// (D-07 (d), ADR-0060 (g)).
fn retained_item_uses() -> String {
    [
        "use paladin_core::base::entity::field::Field;",
        "use paladin_core::base::entity::collection::Collection;",
        "use paladin_core::base::component::element::Element;",
        "use paladin_core::base::service::field_version_service::FieldVersionService;",
        "use paladin_core::base::service::collection_versioning_service::CollectionVersionService;",
        "let _ = FieldVersionService::get_node_field_versions(&service, nid);",
        "pub struct Field { pub nid: String }",
        "let version = FieldVersion { node_id: id, value };",
        "let id = version.node_id;",
    ]
    .join("\n")
        + "\n"
}

/// A tree of look-alikes, retained-item uses and history files: no violation anywhere. Every
/// scoped target is present with clean content.
fn clean_tree(label: &str) -> io::Result<ScratchTree> {
    let tree = ScratchTree::new(label)?;
    for relative in all_targets() {
        tree.write(relative, &clean_content(relative, ""))?;
    }
    tree.write("src/look_alikes.rs", &look_alikes())?;
    tree.write("docs/look_alikes.md", &look_alikes())?;
    tree.write(
        "crates/paladin-core/src/base/entity/field.rs",
        &retained_item_uses(),
    )?;
    tree.write(
        "crates/paladin-core/src/base/service/field_version_service.rs",
        &retained_item_uses(),
    )?;
    // Dot-directories other than the skipped ones are scanned, so they must be clean too.
    tree.write(".github/copilot-instructions.md", &look_alikes())?;
    tree.write(".github/workflows/ci.yml", "# nothing forbidden here\n")?;
    tree.write(".claude/commands/look_alikes.md", &look_alikes())?;
    // Prose that restates a declaration is not a `.rs` declaration.
    tree.write("docs/notes.md", "pub type Paladin = Entity<PaladinData>;\n")?;
    // History files name every old item; skipped directories and non-scanned extensions too.
    let history = every_old_name();
    for relative in [
        "CHANGELOG.md",
        "crates/x/CHANGELOG.md",
        "MIGRATION.md",
        ".cargo/semver-checks-allowlist.toml",
        ".git/info/notes.md",
        ".planning/phases/48/notes.md",
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

/// One planted violation per file: `(path, content, rule)`. Every other scoped target stays clean
/// so only the planted ones report.
fn plant_one_violation_per_rule() -> Vec<(String, String, u8)> {
    let [wrapper_path, version_path] = old_paths();
    let [wrapper_slash, version_slash] = old_slash_paths();
    let upper = old_upper();
    let prefix = old_family_prefix();
    let cap = old_version_cap();
    let not_found = old_not_found();
    let container = "crates/paladin-core/src/platform/container/user.rs";
    let version_file = VERSION_SERVICE_FILE;

    let mut planted = vec![
        // Rule 1: both old paths and both slash forms, across file types.
        ("docs/a.md".into(), format!("see `{wrapper_path}`\n"), 1),
        ("config/b.toml".into(), format!("# was {version_path}\n"), 1),
        (
            "docs/c.md".into(),
            format!("crates/x/src/base/{wrapper_slash}\n"),
            1,
        ),
        ("ops/d.yaml".into(), format!("# base/{version_slash}\n"), 1),
        // Rule 2: the three generic forms, the family prefix and the version cap.
        (
            "src/e.rs".into(),
            format!("let w: Vec<{upper}<u8>> = x;\n"),
            2,
        ),
        ("src/f.rs".into(), format!("let w = {upper}::new(1);\n"), 2),
        (
            "src/g.rs".into(),
            format!("let w = {upper}::<u8>::from(1);\n"),
            2,
        ),
        (
            "docs/h.md".into(),
            format!("`{prefix}Repository` is gone\n"),
            2,
        ),
        ("ops/i.yml".into(), format!("{cap}: 5\n"), 2),
        // Dot-directories that are scanned: the convention text and CI config, and tool commands.
        (
            ".github/copilot-instructions.md".into(),
            format!("use `{upper}<T>` for the wrapper\n"),
            2,
        ),
        (".github/workflows/ci.yml".into(), format!("# {cap}\n"), 2),
        (
            ".claude/commands/note.md".into(),
            format!("see `{wrapper_path}`\n"),
            1,
        ),
        // Rule 3: an entity-sense read in a container file, a field in the wrapper, an old
        // variant in the version module.
        (
            container.into(),
            clean_content(container, "let x = self.node.x;"),
            3,
        ),
        (
            WRAPPER_FILE.into(),
            clean_content(WRAPPER_FILE, "pub node: T,"),
            3,
        ),
        (
            version_file.into(),
            clean_content(version_file, &format!("Err(Error::{not_found}(id))")),
            3,
        ),
        // Rule 5: a deleted graph declaration and a deleted schema key.
        (
            GRAPH_DECLARATIONS[0].0.into(),
            clean_content_skipping(
                GRAPH_DECLARATIONS[0].0,
                "",
                Some("pub enum NodeOutcomeKind"),
            ),
            5,
        ),
        (
            SCHEMA_FILE.into(),
            clean_content_skipping(SCHEMA_FILE, "", Some(SCHEMA_KEYS[0])),
            5,
        ),
    ];
    // Rule 4: a second declaration of each single-declaration item, one per file.
    for (index, declaration) in SINGLE_DECLARATIONS.iter().enumerate() {
        planted.push((
            format!("src/second_{index}.rs"),
            format!("{}\n", declaration.line),
            4,
        ));
    }
    planted
}

/// Every forbidden form of the file-scoped rule 3, planted one at a time into every file the rule
/// covers: each must produce exactly one violation, naming the file and the rule.
fn assert_scoped_variants_are_caught() -> io::Result<()> {
    let not_found = old_not_found();
    let entity_plants = [
        "let x = self.node.x;".to_string(),
        "pub node: T,".to_string(),
        "let e = Entity { node: value };".to_string(),
    ];
    let version_plants = [
        format!("Err(Error::{not_found}(id))"),
        "pub node_id: String,".to_string(),
        "pub node_data: T,".to_string(),
        "let i = version.node_id;".to_string(),
        "let d = version.node_data;".to_string(),
    ];

    for (files, plants) in [
        (&ENTITY_FIELD_FILES[..], &entity_plants[..]),
        (&[VERSION_SERVICE_FILE][..], &version_plants[..]),
    ] {
        for relative in files {
            for plant in plants {
                let tree = clean_tree("scoped_variant")?;
                tree.write(relative, &clean_content(relative, plant))?;
                let violations = scan(&tree.root);
                assert_eq!(violations.len(), 1, "plant `{plant}`: {violations:#?}");
                assert!(
                    violations[0].starts_with(&format!("{relative}:"))
                        && violations[0].contains(": rule 3:"),
                    "plant `{plant}` in {relative}: {violations:#?}"
                );
            }
        }
    }
    Ok(())
}

/// The same declaration respelled the ways Rust allows: restricted visibility, extra whitespace,
/// a space before the generic opener.
fn respelled(line: &str) -> Vec<String> {
    vec![
        line.replacen("pub ", "pub(crate) ", 1),
        line.replacen("pub ", "pub(super) ", 1),
        line.replacen("pub ", "pub(in crate::base) ", 1),
        line.replace(' ', "   "),
        line.replacen('<', " <", 1),
        line.replacen(" =", "=", 1),
    ]
}

/// Rule 4's exact counts: a repeat in the home file, a second declaration elsewhere, and a
/// declaration moved to the wrong file are each reported; a moved one reports twice (zero here,
/// one elsewhere).
fn assert_declaration_counts_are_exact() -> io::Result<()> {
    for declaration in &SINGLE_DECLARATIONS {
        let home = declaration.home;

        let tree = clean_tree("declaration_repeat")?;
        tree.write(home, &clean_content(home, declaration.line))?;
        let violations = scan(&tree.root);
        assert_eq!(
            violations.len(),
            1,
            "repeat of `{}`: {violations:#?}",
            declaration.needle
        );
        assert!(
            violations[0].starts_with(&format!("{home}:")) && violations[0].contains(": rule 4:"),
            "{violations:#?}"
        );

        for variant in respelled(declaration.line) {
            let tree = clean_tree("declaration_respelled")?;
            tree.write("src/respelled.rs", &format!("{variant}\n"))?;
            let violations = scan(&tree.root);
            assert_eq!(
                violations.len(),
                1,
                "respelling `{variant}`: {violations:#?}"
            );
            assert!(
                violations[0].starts_with("src/respelled.rs:")
                    && violations[0].contains(": rule 4:"),
                "respelling `{variant}`: {violations:#?}"
            );
        }

        // A literal that looks like the start of a string or lifetime must not hide what follows.
        let tree = clean_tree("declaration_after_literals")?;
        tree.write(
            "src/after_literals.rs",
            &format!(
                "fn f<'a>(x: &'a str) -> char {{ let _ = '\"'; let _ = '\\''; 'x' }}\n{}\n",
                declaration.line
            ),
        )?;
        let violations = scan(&tree.root);
        assert_eq!(violations.len(), 1, "after literals: {violations:#?}");
        assert!(
            violations[0].starts_with("src/after_literals.rs:2:")
                && violations[0].contains(": rule 4:"),
            "after literals: {violations:#?}"
        );

        let tree = clean_tree("declaration_moved")?;
        tree.write(
            home,
            &clean_content_skipping(home, "", Some(declaration.needle)),
        )?;
        tree.write("src/elsewhere.rs", &format!("{}\n", declaration.line))?;
        let violations = scan(&tree.root);
        assert_eq!(
            violations.len(),
            2,
            "move of `{}`: {violations:#?}",
            declaration.needle
        );
        assert!(
            violations
                .iter()
                .all(|line| line.contains(": rule 4:") && line.contains(declaration.needle)),
            "{violations:#?}"
        );
    }
    Ok(())
}

/// Positive control: the scanner reports every planted violation, one per planted file, naming its
/// rule; every file-scoped form is caught; and rule 4's counts are exact.
#[test]
fn scanner_reports_planted_entity_rename_violations() -> io::Result<()> {
    let planted = plant_one_violation_per_rule();
    let tree = ScratchTree::new("planted")?;
    for relative in all_targets()
        .into_iter()
        .filter(|target| !planted.iter().any(|(path, _, _)| path == target))
    {
        tree.write(relative, &clean_content(relative, ""))?;
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
    assert_declaration_counts_are_exact()
}

/// Positive control: removing any file-scoped target is reported, never skipped, and so is
/// emptying a rule 3 target. A guard that cannot find its target proves nothing.
#[test]
fn scanner_reports_missing_scoped_targets() -> io::Result<()> {
    for relative in all_targets() {
        let tree = clean_tree("missing_target")?;
        fs::remove_file(tree.root.join(relative))?;
        let violations = scan(&tree.root);
        assert_eq!(
            violations.len(),
            expected_reports_for_removal(relative),
            "{relative}: {violations:#?}"
        );
        assert!(
            violations
                .iter()
                .all(|line| line.starts_with(&format!("{relative}: "))),
            "removal of {relative} reported against another file: {violations:#?}"
        );
        assert!(
            violations
                .iter()
                .any(|line| line.contains("missing") || line.contains("declared 0")),
            "missing {relative} not reported as missing: {violations:#?}"
        );
    }

    for relative in all_targets()
        .into_iter()
        .filter(|target| is_rule_3_target(target))
    {
        let tree = clean_tree("emptied_target")?;
        tree.write(relative, "")?;
        let violations = scan(&tree.root);
        let homed = SINGLE_DECLARATIONS
            .iter()
            .filter(|declaration| declaration.home == relative)
            .count();
        assert_eq!(
            violations.len(),
            1 + homed,
            "emptied {relative}: {violations:#?}"
        );
        assert!(
            violations
                .iter()
                .any(|line| line.contains("proves nothing")),
            "emptied {relative} passed vacuously: {violations:#?}"
        );
    }
    Ok(())
}

/// Positive control: rule 5 counts a declaration, not a repeat of its text. Removing the real
/// declaration is reported even when the same text remains in a match arm, a comment, a test
/// module, or (for the schema) the `required` list and an example.
#[test]
fn scanner_reports_graph_declarations_that_only_appear_elsewhere() -> io::Result<()> {
    let trace = "crates/paladin-core/src/platform/container/trace.rs";
    let repeats = "match event {\n    TraceEvent::NodeStarted { .. } => 1,\n}\n\
                   // NodeStarted {\n\
                   #[cfg(test)]\n\
                   mod tests {\n    NodeStarted {\n}";
    let tree = clean_tree("declaration_repeats")?;
    tree.write(
        trace,
        &clean_content_skipping(trace, repeats, Some("NodeStarted {")),
    )?;
    let violations = scan(&tree.root);
    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(
        violations[0].starts_with(&format!("{trace}:"))
            && violations[0].contains(": rule 5:")
            && violations[0].contains("`NodeStarted {`"),
        "{violations:#?}"
    );

    for key in SCHEMA_KEYS {
        let tree = clean_tree("schema_key_repeats")?;
        tree.write(
            SCHEMA_FILE,
            &clean_content_skipping(SCHEMA_FILE, "", Some(key)),
        )?;
        let violations = scan(&tree.root);
        assert_eq!(violations.len(), 1, "removed `{key}`: {violations:#?}");
        assert!(
            violations[0].starts_with(&format!("{SCHEMA_FILE}:"))
                && violations[0].contains(": rule 5:")
                && violations[0].contains(key),
            "removed `{key}`: {violations:#?}"
        );
    }

    let tree = clean_tree("schema_not_json")?;
    tree.write(SCHEMA_FILE, "\"nodes\" \"max_node_visits\"")?;
    let violations = scan(&tree.root);
    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("not valid JSON"), "{violations:#?}");
    Ok(())
}

/// Negative control: the look-alikes, the retained base items, the history files and the skipped
/// directories are not violations.
#[test]
fn scanner_accepts_a_clean_tree() -> io::Result<()> {
    let tree = clean_tree("clean")?;

    let violations = scan(&tree.root);

    assert!(violations.is_empty(), "{violations:#?}");
    Ok(())
}

/// Positive control: the Docker test image's build context carries none of the directories
/// `.dockerignore` excludes (`docs/`, `notes/`, `k8s/`, `.github/`), so it cannot hold the graph
/// schema and the schema is not demanded there.
#[test]
fn scanner_accepts_a_build_context_without_docs() -> io::Result<()> {
    let tree = clean_tree("no_docs")?;
    for directory in DOCKER_EXCLUDED_DIRS {
        let path = tree.root.join(directory);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }

    let violations = scan(&tree.root);

    assert!(violations.is_empty(), "{violations:#?}");
    Ok(())
}

/// Positive control: a tree that lacks only `docs/` is not the Docker image, so the missing schema
/// is reported. The skip must not hide a sparse checkout or a packaged source tree.
#[test]
fn scanner_reports_a_missing_schema_when_only_docs_is_absent() -> io::Result<()> {
    let tree = clean_tree("only_docs_absent")?;
    fs::remove_dir_all(tree.root.join("docs"))?;
    tree.write("notes/design.md", "design notes\n")?;

    let violations = scan(&tree.root);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(
        violations[0].starts_with(&format!("{SCHEMA_FILE}:"))
            && violations[0].contains("missing or unreadable"),
        "{violations:#?}"
    );
    Ok(())
}

/// Positive control: only an inline test module ends the production scan. A module whose name
/// merely starts with `tests`, or the out-of-line `mod tests;`, must not hide the code after it,
/// while a visibility-qualified test module is still recognised as test code.
#[test]
fn production_scan_stops_only_at_an_inline_test_module() -> io::Result<()> {
    for lookalike in [
        "mod tests_support {",
        "mod testsuite {",
        "mod tests;",
        "mod tester {",
    ] {
        let relative = "crates/paladin-core/src/platform/container/user.rs";
        let tree = clean_tree("lookalike_test_module")?;
        let extra = format!("{lookalike}\n}}\nlet x = self.node.x;");
        tree.write(relative, &clean_content(relative, &extra))?;
        let violations = scan(&tree.root);
        assert_eq!(violations.len(), 1, "after `{lookalike}`: {violations:#?}");
        assert!(
            violations[0].starts_with(&format!("{relative}:"))
                && violations[0].contains(": rule 3:"),
            "after `{lookalike}`: {violations:#?}"
        );
    }

    for opener in [
        "mod tests {",
        "pub(crate) mod tests {",
        "pub mod test {",
        "  mod tests {",
    ] {
        let content = format!("let kept = 1;\n{opener}\n    let dropped = old.node;\n}}\n");
        let lines: Vec<&str> = production_lines(&content).map(|(_, line)| line).collect();
        assert_eq!(lines, ["let kept = 1;"], "opener `{opener}`");
    }
    Ok(())
}

/// The guard: the repository itself names none of the old Phase 48 entity names outside the history
/// files, declares `Paladin`, `PaladinData`, `PaladinStatus`, `NodeError` and `Entity` exactly
/// once each, and still declares every graph definition in its current file.
#[test]
fn entity_surfaces_are_renamed_in_the_tree() -> io::Result<()> {
    let violations = scan(Path::new(env!("CARGO_MANIFEST_DIR")));

    assert!(
        violations.is_empty(),
        "Phase 48 renamed the base entity wrapper and froze the graph vocabulary \
         (ENTITY-01..05, ADR-0060); fix these at their source:\n{}",
        violations.join("\n")
    );
    Ok(())
}
