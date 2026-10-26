# Phase 48: Base Entity Rename - Pattern Map

**Mapped:** 2026-10-13
**Files analyzed:** 10 new/modified artefacts (the ~157 mechanically renamed `.rs` files are covered by the RESEARCH.md Pattern 1 scripts and need no analog)
**Analogs found:** 10 / 10

Scope note: analogs were chosen for the artefacts the planner must author by hand. Facts about the bulk rename (perl passes, residual hand fixes, graph-sense exclusions) live in `48-RESEARCH.md` Pattern 1 and Pitfalls 2-3 and are not repeated here.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `tests/entity_rename_guard.rs` (new) | test (repo-scan guard) | batch (filesystem walk) | `tests/legacy_removal_guard.rs` | exact |
| `crates/paladin-core/tests/entity_serde_compat.rs` (new) + `tests/fixtures/pre_rename/*.json` | test (serde fixture) | transform (JSON to type) | `battalion_config_v0_10_document_deserializes_with_default_aegis` in `crates/paladin-core/src/platform/container/battalion/mod.rs:1063-1088` | role-match (inline, `json!` instead of file fixtures) |
| `.planning/decisions/0060-base-entity-rename.md` (new) | doc (ADR) | n/a | `.planning/decisions/0059-legacy-clean-break-removal.md` | exact |
| `.planning/decisions/PROMOTION.md` (modify) | doc (index) | n/a | row 0059 + "Next free ADR number" line (lines 79-83) | exact |
| `MIGRATION.md` section 9.1 (M-B-12), 9.2 rows, 9.4 note, 9.7 sentence (modify) | doc (register) | n/a | Phase 44 rows at `MIGRATION.md:296-305`, `:31-36`, `:447`, `:867` | exact |
| `.cargo/semver-checks-allowlist.toml` (modify) | config | n/a | Phase 44 block, lines 317-370 | exact |
| `crates/paladin-core/Cargo.toml` `[package.metadata.cargo-semver-checks.lints]` (modify) | config | n/a | same table, lines 48-98 (header comment + allow lines) | exact |
| `crates/paladin-core/src/base/entity/wrapper.rs` (git mv of `node.rs`) + `entity/mod.rs` | model (base wrapper) | CRUD (value type) | current `crates/paladin-core/src/base/entity/node.rs` (188 lines) | exact (it is the file) |
| `CHANGELOG.md` + `crates/paladin-core/CHANGELOG.md` (modify) | doc | n/a | Phase 44 blocks, `CHANGELOG.md:648-710` (Changed at 538, Removed at 689) | exact |
| `.project/current-exports.txt` (regenerate) | generated baseline | n/a | `make api-surface-update` (Phase 44 dropped one facade line the same way) | exact |

## Pattern Assignments

### `tests/entity_rename_guard.rs` (test, repo-scan guard)

**Analog:** `tests/legacy_removal_guard.rs` (748 lines). Copy the skeleton verbatim; replace only the rule tables, the file-scoped target lists, `GUARD_FILE`, and the `ScratchTree` label prefix.

**Module header** (lines 1-47): `//!` doc listing numbered rules, the skip set ("declared history set"), the `concat` note, and the positive-control note, ending with the `cargo test --test ...` block. Mirror this structure for the 5 rules in RESEARCH.md Pattern 3.

**Imports and constants** (lines 49-82):
```rust
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

const HISTORY_FILES: [&str; 3] = [
    "MIGRATION.md",
    "docs/src/api-reference/migration-guide.md",
    "docs/src/api-reference/upgrading.md",
];
const GUARD_FILE: &str = "tests/legacy_removal_guard.rs";   // -> "tests/entity_rename_guard.rs"
const SCANNED_EXTENSIONS: [&str; 5] = ["rs", "md", "toml", "yaml", "yml"];
```

**Forbidden strings assembled at run time** (lines 84-158) so the guard file never holds one whole:
```rust
fn removed_variants() -> [String; 2] {
    [
        ["Continue", "OnError"].concat(),
        ["RetryThen", "Continue"].concat(),
    ]
}
```
Phase 48 equivalents: `["base::entity::", "node"].concat()`, `["Node", "Version"].concat()`, `["Field", "VersionService"].concat()`, `["Collection", "VersionService"].concat()` (real type name, not `CollectionVersioningService`; see RESEARCH Pitfall 5), `["Node", "::new("].concat()`.

**Identifier-boundary matcher** (lines 160-174), copy unchanged:
```rust
fn is_identifier(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn occurs(haystack: &str, needle: &str, bound_start: bool, bound_end: bool) -> bool {
    haystack.match_indices(needle).any(|(start, matched)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + matched.len()..].chars().next();
        (!bound_start || !before.is_some_and(is_identifier))
            && (!bound_end || !after.is_some_and(is_identifier))
    })
}
```
Use `bound_start = true` on `Node::new(` and `Node<` so `StateNode::new`, `NodeId::new` pass (RESEARCH rule 2).

**File-scoped rule that reports a missing target** (lines 255-291): `production_lines` + `scan_scoped`. Reuse for rule 3 (the 8 container files) and rule 5 (graph-identifier freeze; for rule 5 the check is "file exists AND contains declaration needle", so write a small variant of `scan_scoped` that reads the whole file rather than `production_lines`).
```rust
fn production_lines(content: &str) -> impl Iterator<Item = (usize, &str)> {
    content.lines().enumerate()
        .take_while(|(_, line)| !line.trim_start().starts_with("mod tests"))
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(index, line)| (index + 1, line))
}
// scan_scoped: on Err(error) push "{relative}: rule {rule}: target file is missing or unreadable ({error}), so the rule proves nothing"
```

**Walk and skip set** (lines 293-379): `skip_directory` (`target`, `node_modules`, every dot-dir), `skip_file` (`CHANGELOG.md`, `GUARD_FILE`, `HISTORY_FILES`), and `scan(root)` with `DirEntry::file_type` (no symlink following). Copy unchanged; add the rule-4 count logic (exactly one `pub type Paladin =` in `paladin.rs`, exactly one `pub struct NodeError` in `node_error.rs`) after the walk by accumulating counts in a `HashMap<&str, Vec<String>>`, and report both zero and more-than-one.

**ScratchTree** (lines 381-417): copy unchanged, change only the `paladin_legacy_removal_guard_{label}_...` prefix to `paladin_entity_rename_guard_`.

**Positive controls** (lines 419-734), the three-test shape:
- `clean_tree(label)` writes every scoped file with clean content plus look-alikes and history files (lines 465-522). For Phase 48 the look-alikes are `StateNode::new`, `NodeId::new`, `BattalionError::Node(`, `EngineError::Node`, `graph.node(`, `progress.node`.
- `plant_one_violation_per_rule()` returns `Vec<(path, content, rule)>` (lines 524-621).
- `scanner_reports_planted_*` asserts `violations.len() == planted.len()`, then exactly one violation per planted file naming `: rule {n}:` (lines 689-722); `assert_missing_targets_are_reported` removes each scoped file in turn and expects exactly one "target file is missing" line (lines 670-685).
- `scanner_accepts_a_clean_tree` (lines 726-734) and the real-tree test (lines 738-747):
```rust
#[test]
fn legacy_surfaces_are_gone_from_the_tree() {
    let violations = scan(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(violations.is_empty(), "Phase 44 removed ...:\n{}", violations.join("\n"));
}
```
All tests return `io::Result<()>`; no `unwrap`/`expect` in the scanner (tests may use `assert!`).

**Placement:** root `tests/` is auto-discovered (RESEARCH Validation Architecture), so no `[[test]]` entry. Keep `cargo test --test legacy_removal_guard` green alongside (CONTEXT D-00e).

---

### `crates/paladin-core/tests/entity_serde_compat.rs` and `crates/paladin-core/tests/fixtures/pre_rename/{paladin,citadel,content,prompt}.json` (test, serde fixture)

**Analog:** `battalion_config_v0_10_document_deserializes_with_default_aegis`, `crates/paladin-core/src/platform/container/battalion/mod.rs:1063-1088`. No existing file-fixture integration test lives in `paladin-core` (`crates/paladin-core/tests/` does not exist yet), so this is a role-match: the inline test is the repo precedent for "old-shaped document still loads".

**Core pattern** (lines 1064-1087):
```rust
#[test]
fn battalion_config_v0_10_document_deserializes_with_default_aegis() {
    // A v0.10-shaped document: ... They are ignored, and `aegis` falls back to its default.
    let document = serde_json::json!({
        "name": "stored_battalion",
        "timeout_seconds": 300,
        "error_strategy": "FailFast",
        "metadata_output_dir": null
    });
    let config: BattalionConfig =
        serde_json::from_value(document).expect("a v0.10 document still deserializes");
    assert_eq!(config.name, "stored_battalion");
    assert_eq!(config.aegis, Aegis::default());
}
```
Adopt: a comment naming the version that wrote the shape, a literal old-shaped document, `.expect("...still deserializes")` with a message (tests may use `expect`), then field assertions.

**Differences for Phase 48** (use RESEARCH.md "Compat test" lines 430-451, already rehearsed and green): load JSON from `env!("CARGO_MANIFEST_DIR")/tests/fixtures/pre_rename/{name}.json` so the test file contains no `Node`/`node` Rust identifiers and compiles after the rename; fixtures are captured from the unmodified tree before plan 48-04 (a post-rename capture would already emit `data`). Add (a) a both-keys test expecting an error containing `duplicate field` and (b) a generic `Entity<TestData>` round-trip using the old `"node"` key. Because the plain `serde_json::json!` inline form also exists, a small inline generic test may use it for (b); inline unit tests for the type itself stay in `wrapper.rs`.

**Dev-dependency check:** `serde_json` is already a `paladin-core` dependency; no new dependency (ADR-0015, D-00d).

---

### `.planning/decisions/0060-base-entity-rename.md` (ADR)

**Analog:** `.planning/decisions/0059-legacy-clean-break-removal.md` (171 lines).

**Section order to copy exactly:** `# ADR-0060: <title>` / `## Status` (`Accepted`) + `**Date:**` + `**Phase:** 48 (Base Entity Rename), plan 48-01; Code Conformance is measured at the closeout, plan 48-08.` / `## Supersedes` (none for ADR numbers; note it supersedes nothing, or ADR-0015/naming rows only if the planner decides) / `## Context` / `## Decision` (lettered subsections) / `## Considered Options` / `## Code Locations` / `## Code Conformance` / `## Downstream Consumers`.

**Decision (a) shape, X-03 supersession scoped to the phase** (ADR-0059 lines 26-36):
```markdown
**D-00a. X-03 is superseded for Phase 44 only**, and exactly for the removals recorded in subsections (b) to (i) below. ... A later removal records its own supersession; it does not inherit this one. This ADR does not inherit ADR-0051 ...
**What still applies.**
- **D-00b.** Every deliberate break gets a `MIGRATION.md` 9.2 row and, when marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry per lint, row-level set-equal ... (`make check-migration-allowlist`).
- **D-00c.** A clean break means no shim: no `#[deprecated]` bridge, no type alias, no re-export that keeps a removed name resolvable, no compatibility deserializer.
```
**Required deviation (RESEARCH Pitfall 7):** restate D-00c as "no *API* shim" and carve out `#[serde(alias = "node")]` explicitly as *data* compatibility for persisted JSON (it keeps no removed Rust name resolvable); record the alias lifetime (kept for the whole 0.x line; removal needs its own ADR). Do not copy "no compatibility deserializer" verbatim.

Other ADR-0059 conventions to copy: `**Reversibility: costly|one-way.**` tags on decisions; "Open Question N, resolved" paragraphs for each RESEARCH Open Question the planner settles; one ADR for supersession plus design (ADR-0059 rejected splitting; RESEARCH A7); version identity `v0.11.0` everywhere (D-00e analog); a trailing "one 9.2 row per `crate | type` pair" decision (D-19 analog, keyed on the first backticked identifier of the Type cell).

**Considered Options** (ADR-0059 lines 106-127 style, one bullet per rejected alternative with a parenthesised reason): `Record<T>`, `Versioned<T>`, `Aggregate<T>`; deprecated `Node<T>` alias (RESEARCH Pitfall 9: a struct field cannot be aliased); `#[serde(rename = "node")]`; `schema_version` on `Entity<T>`; unifying the `citadel::PaladinData` placeholder; `entity/entity.rs` (fails `clippy::module_inception`).

**Code Conformance** (lines 145-164): the word `conforms` then per-decision named tests, e.g. `entity_surfaces_are_renamed_in_the_tree` (new guard), `pre_rename_json_still_loads` (compat test), plus the D-27 semver headline measured with cargo-semver-checks 0.50.0 against published 0.10.1, `--release-type minor`, `--default-features`.

**Vocabulary rule sentence (ENTITY-05):** "node" = a vertex in a WarGraph/Campaign; "Entity" = the base wrapper.

---

### `.planning/decisions/PROMOTION.md` (index)

**Analog:** row 0059 (line 79) and the "Next free ADR number" block (lines 81-83).

Add a row `| 0060 | \`base-entity-rename\` | <one-sentence summary incl. supersession scope and vocabulary rule> |` directly after row 0059, change `**Next free ADR number: 0060**` to `0061`, and add a dated note in the existing style: `*Dated note, 2026-10-xx (plan 48-01, ADR-0060):* the line advances by **one**, from 0060 to 0061, because ... \`ls .planning/decisions/0060-*.md\` (re-run before ...)`.

---

### `MIGRATION.md` rows (sections 9.1, 9.2, 9.4, 9.7)

**Analog:** the Phase 44 rows.

**9.2 row column order** (header at `MIGRATION.md:183`): `| Crate | Type / Trait | Change | Mitigation | Deliberate-breaking? | Req |`; Crate cell is the published name (`paladin-ai-core`). Template, from `MIGRATION.md:299`:
```markdown
| `paladin-ai-core` | `NodeError` (the legacy Battalion summary struct `...::battalion::NodeError`) | <what is removed; the same-named different item that is unaffected>. Replacement: <...> -- landed Phase 44 (44-06, 44-10), D-06 | none -- clean break under ADR-0059; no shim (D-00c). Suppressed via `struct_missing = "allow"` in `crates/paladin-core/Cargo.toml`, mirrored in `.cargo/semver-checks-allowlist.toml` | Y -- <reason> (LEGACY-01); measured at the Phase 44 closeout (plan 44-11) with cargo-semver-checks 0.50.0 against the published 0.10.1, `--release-type minor`, `--default-features`, paladin-ai-core's crate-wide allows disabled and restored byte-for-byte: 196 checks, 9 lints fired across the crate, 1 on this item (`struct_missing`) | LEGACY-01 |
```
RESEARCH.md lines 462-475 already drafts the Phase 48 `Node` row; one row per `crate | first-backticked-identifier` pair (the set-equality key). Needed rows (identifiers must be real symbols): `Node`, `NodeVersionService` (and sibling `EntityVersion*` types as the planner groups them), `Field`, `CollectionVersionService`, `FieldVersionService`, `Element`/`Collection` as separate first identifiers, `ContentItem`, `PromptItem`, and `ContentItemService` for the root crate `paladin-ai` (`function_signature_changed`). A `Y` register-only row (field type change, no lint) uses `N -- register-only ...` as at `MIGRATION.md:300`.

**9.1 behavioural row** (`MIGRATION.md:31-36`, ids `M-B-06..M-B-11`): next id is `M-B-12`; columns are bold-title paragraph with plan/decision refs, then "who is affected", then mitigation. The serde asymmetry (v0.11 reads both keys, writes only `data`/`entity`; a v0.10 binary cannot read v0.11 output) belongs here per RESEARCH Pitfall 6.

**9.4 note** (`MIGRATION.md:447`, bullet starting `**Stored \`BattalionResult\` JSON (Phase 44, LEGACY-01, D-06).**`): copy the form "what changed, which stored documents break, no migration job by design, how to read old documents". For Phase 48 the content inverts (old documents still read via alias) and adds the rolling-upgrade note (consumers before producers, back up before upgrade).

**9.7** (`MIGRATION.md:867`): must still open with `Empty`; append one sentence pointing at Phase 48's clean break in the style of the existing Phase 44 sentence ("Phase 44 (v0.11) removes them outright under ADR-0059, without a deprecation cycle (D-00c) -- see the Phase 44 rows in 9.2.").

---

### `.cargo/semver-checks-allowlist.toml` (config)

**Analog:** Phase 44 block, lines 317-370.

**Block header comment** (lines 317-330): `# --- Phase 44 (Legacy Clean-Break Removal, LEGACY-01..04) ---` then a paragraph naming the plan, tool version (0.50.0), baseline (published 0.10.1, `--release-type minor`, `--default-features`), the statement that each crate's allow lines were disabled and restored byte-for-byte, entries in 9.2 row order with lint ids alphabetical inside a row, and lints that belong to earlier phases not repeated.

**Entry shape** (lines 332-337):
```toml
[[entry]]
crate = "paladin-ai-core"
lint = "struct_missing"
migration_row = "paladin-ai-core | RetryPolicy"
requirement_id = "LEGACY-01"
justification = "The legacy Battalion RetryPolicy struct (...) is deleted outright (ADR-0059, D-00c); ... Every in-tree use migrated in the same commit series; suppressed by a new crate-wide allow line in crates/paladin-core/Cargo.toml."
```
`migration_row` = `<crate> | <first backticked identifier of the Type cell>` (reduced to its first whitespace-delimited token); several entries may share one row (one per lint, see the three `BattalionConfig` entries at lines 332-351); `requirement_id` may be a comma list (`"LEGACY-01, LEGACY-02"`). For Phase 48 use `requirement_id = "ENTITY-0x"`; lints expected per RESEARCH "Measured semver lints": `struct_missing`, `enum_missing`, `trait_missing` (new), `module_missing` (new), `struct_pub_field_missing`, `constructible_struct_adds_field`, plus `function_signature_changed` on crate `paladin-ai` (root). Verify with `make check-migration-allowlist`.

---

### `crates/paladin-core/Cargo.toml` semver lints table

**Analog:** same file, header comment lines 48-89 and table at lines 90-98.
```toml
[package.metadata.cargo-semver-checks.lints]
enum_marked_non_exhaustive = "allow"
constructible_struct_adds_field = "allow"
inherent_method_missing = "allow"
struct_pub_field_missing = "allow"
struct_marked_non_exhaustive = "allow"
struct_missing = "allow"
enum_missing = "allow"
enum_variant_missing = "allow"
```
Phase 48 adds `trait_missing = "allow"` and `module_missing = "allow"` plus a header paragraph in the existing `LEGACY-01..04 (ADR-0059, D-00c, D-27, Phase 44 plan 44-11): ...` voice naming ADR-0060, the lints "new here", and the statement that each has a matching 9.2 row and allowlist entry. Per the file's own rule, the allow line, the row and the entry land in the same commit (RESEARCH "Commit-green constraint"). The facade `paladin-ai` root `Cargo.toml` may need `function_signature_changed` analogously (measure with the D-27 diagnostic; do not guess).

---

### `crates/paladin-core/src/base/entity/wrapper.rs` and `entity/mod.rs` (model)

**Analog:** the file being renamed, `crates/paladin-core/src/base/entity/node.rs`.

**Current struct and impl** (lines 1-75): derive set, field order, hand-written `Hash`, `new`, `new_with_uuid`, `update`, `set_name`, `enable_versioning`, `disable_versioning`, `is_versioning_enabled`.
```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Node<T> {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub node: T,
    pub name: Option<String>,
    pub version: bool,
}
impl<T: Hash> Hash for Node<T> { fn hash(..) { self.uuid.hash(state); self.name.hash(state); self.node.hash(state); } }
```
Keep field order `uuid, created, modified, data, name, version` (wire order and derived `Ord`); the only structural change is `#[serde(alias = "node")] pub data: T` and ctor params renamed. Target text, including the rustdoc "Revisioning contract (`version`)" and "Serialization" sections and a doctest, is in RESEARCH.md lines 389-426. Existing tests live in `node.rs:77-188` (`mod tests`; creation, update, versioning control, hash, serialization round-trip): carry them across with the rename.

**Module wiring** (current `entity/mod.rs`, 4 lines: `pub mod collection; pub mod field; pub mod message; pub mod node;`). Target: `mod wrapper; pub use wrapper::Entity; pub mod message;`. Do **not** use `mod entity;` inside `entity/mod.rs` (fails `clippy::module_inception`, RESEARCH verified).

No `#[serde(...)]` precedent exists in this file; for the alias idiom itself see the in-repo usage of `#[serde(default)]` on `BattalionConfig.aegis` (ADR-0059 D-01) as the register's "compat attribute" precedent.

---

### `CHANGELOG.md` and `crates/paladin-core/CHANGELOG.md` (doc)

**Analog:** `CHANGELOG.md` `## [Unreleased]` (line 8), `### Changed` (line 538, Phase 44 blocks at 648-687), `### Removed` (line 689, Phase 44 block at 698-710).

**Entry shape** (Phase 44, `CHANGELOG.md:698-704`): a bold lead sentence ending with `(Phase N; REQ-IDS; ADR-00NN; see MIGRATION.md 9.1 M-B-xx and the Phase N 9.2 rows)`, then an indented nested list where each removal names its replacement and the same-named unaffected items. Under **Changed** use the same bold-lead form for behavioural change (the serde key rename: reads both, writes `data`/`entity`). The per-crate file (`crates/paladin-core/CHANGELOG.md`, section order Fixed / Added / ...) takes a shorter parallel entry naming the crate-level items. Run `./scripts/check-changelogs.sh` (named in ADR-0059 Code Conformance). The API-baseline entry must state the true diff (RESEARCH Pitfall 1: `.project/current-exports.txt` loses 14 and gains 14 lines, all in `ContentItemService` / `ContentItemServiceError`; the `api_surface_current.txt` named in CONTEXT is a stale v0.2.0 file no gate reads).

---

## Shared Patterns

### Same-commit register rule
**Source:** `crates/paladin-core/Cargo.toml` header comment (lines 48-49), `.cargo/semver-checks-allowlist.toml` header
**Apply to:** every plan that removes or renames a public item (48-02, 48-03, 48-04, 48-05)
A `[package.metadata.cargo-semver-checks.lints]` allow line, its `MIGRATION.md` 9.2 row and its allowlist entry land in the same commit, kept set-equal by `make check-migration-allowlist`.

### Measured, not guessed (D-27)
**Source:** ADR-0059 "D-27 headline" (line 162); `MIGRATION.md:296-305` measurement tails
**Apply to:** all 9.2 `Y` rows
State the tool version (0.50.0), baseline (published 0.10.1, `--release-type minor`, `--default-features`), "crate-wide allows disabled and restored byte-for-byte", and the counts (checks, lints fired, lints on this item).

### Graph-vs-entity disambiguation sentence
**Source:** `MIGRATION.md:297-299` ("The same-named Aegis `RetryPolicy` ... is a different item and is unaffected")
**Apply to:** every Phase 48 row, CHANGELOG bullet and ADR paragraph that could be misread as touching graph vocabulary (`NodeId`, `NodeSpec`, `NodeError`, ...)

### History-file skip set
**Source:** `tests/legacy_removal_guard.rs:69-79, 293-302`
**Apply to:** the new guard and anything that scans prose: `MIGRATION.md`, the two upgrade guides, every `CHANGELOG.md`, dot-dirs (`.planning`, `.project`) are exempt because they must name the removed items.

### Error/lint hygiene for new Rust
**Source:** CLAUDE.md, `.github/instructions/rust.instructions.md`
**Apply to:** the guard and compat test: std only, `io::Result` returns in the guard, no new dependency (ADR-0015, D-00d), rustdoc on any public item, `make clean-code` clean, no `unwrap()` in non-test code.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/paladin-core/tests/` directory (integration-test dir for `paladin-core`) | test | n/a | The directory does not exist; the compat test is the first. Cargo auto-discovers `tests/*.rs`, no `[[test]]` entry needed; file fixtures via `env!("CARGO_MANIFEST_DIR")` are new to this crate. The inline `battalion/mod.rs:1063` test is the nearest precedent. |
| Dated "retained" note for any kept base item (CONTEXT D-05 alternative) | doc | n/a | Only needed if the owner keeps `Field`/`Collection`/`Element`; no precedent row exists. Default path (remove all five) needs none. |

## Metadata

**Analog search scope:** `tests/`, `.planning/decisions/`, `MIGRATION.md`, `.cargo/`, `crates/paladin-core/{Cargo.toml,CHANGELOG.md,src/base,src/platform/container/battalion}`, root `CHANGELOG.md`
**Files scanned:** about 12 read or sectioned (`legacy_removal_guard.rs` in full, ADR-0059 in full, `node.rs` lines 1-80, allowlist 310-370, plus targeted greps/sections of MIGRATION, CHANGELOG, PROMOTION, Cargo.toml)
**Pattern extraction date:** 2026-10-13
**Open items for the planner:** `48-CONTEXT.md` D-01..D-07 are unconfirmed recommendations (owner questions 1-4); RESEARCH.md Open Questions 1-5 (D-02 branch, `ContentItem`/`PromptItem` field rename, `NodeVersion` field names, `citadel.rs` placeholder, alias lifetime) change ADR-0060 wording and row count, not the analogs above.
