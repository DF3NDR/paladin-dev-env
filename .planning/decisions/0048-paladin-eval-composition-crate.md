# ADR-0048: `paladin-eval` as a published composition crate

## Status

Accepted

**Date:** 2026-09-09

## Context

Phase 28 (`crates/paladin-eval`, D-27/D-28/D-30 through D-35) built a new workspace crate for
writing and running deterministic, scripted-LLM eval scenarios against a real `WarEngine`. Its
default dependency graph reaches five leaf/facade-adjacent crates:

- `paladin-core`, `paladin-ports` — the shared domain/port types every crate depends on.
- `paladin-battalion` — the `WarEngine`/`WarGraph`/`WarGraphDoc` the scenario runner drives.
- `paladin-llm`, consumed with `default-features = false, features = ["mock"]` only — so a
  downstream `[dev-dependencies]` consumer of `paladin-eval` pulls no provider adapter or HTTP
  client, only the mock-adapter machinery `ScenarioLlm` itself does not extend (D-30).
- `paladin-storage`, `sqlite` feature — the `sqlite_temp` scenario store option, and the
  interrupt/resume driver's own temp-file store.

`crates/paladin-eval/src/lib.rs`'s own module doc names one further, deliberate exception: a
`cli`-feature-gated edge from the root facade (`paladin-ai`) *into* `paladin-eval` as an optional
dependency (28-12, D-33), so `paladin-cli eval run` can drive the SAME `ScenarioRunner` the
`libtest-mimic` harness uses. The facade's *default* build carries no edge to `paladin-eval`
(`cargo tree -e normal,build -p paladin-ai | grep -c paladin-eval` prints `0`); only a caller
that opts into `--features cli` gains it.

`.planning/decisions/0031-extracted-crate-dependency-rule.md` (ADR-0031) states the invariant an
uninformed reader might apply here: "no extracted crate may depend on another extracted crate or
on the facade in its default build." Read literally against the dependency edges above,
`paladin-eval` looks like a violation on two counts — it depends on THREE other crates
(`paladin-battalion`, `paladin-llm`, `paladin-storage`), and the facade depends back on it under
`cli`.

ADR-0031 itself answers the scope question directly, in its own `## Decision` section: the
restated invariant governs **extracted** crates — the crates that carry a slice of what was once
facade-resident application logic, extracted outward per Milestone 7's cost-benefit criteria
(`paladin-content`, `paladin-web`, `paladin-notifications`, and the like). `paladin-eval` was
never facade logic being pulled outward; it is a new-in-0.10 **tool** crate, written from day one
as a composition point over the leaf crates it tests. The precedent for this shape already exists
in the tree: `crates/doc-examples` (package `paladin-doc-examples`) depends on six workspace
crates in its own default build and has never been read as an ADR-0031 violation, because nobody
mistook it for an extracted crate either.

## Decision

**`paladin-eval` is classified a composition crate, not an extracted crate, and ADR-0031's
default-build invariant does not apply to it.** A composition crate is one whose *purpose* is to
depend on multiple sibling crates at once — a tool, harness, or example crate assembled to
exercise the rest of the workspace, never a slice of application logic that was itself pulled out
of the facade. `paladin-eval`'s three-plus-one dependency shape (five default edges downward,
one `cli`-gated edge upward from the facade) is exactly what a composition crate looks like and
is accepted as such.

**This classification does not license an unbounded set of future crates to claim the same
exemption.** A crate earns the composition classification only if its own purpose is stated
plainly as "depends on multiple siblings to test/demonstrate/drive them" from its own module
docs — `paladin-eval`'s `lib.rs` and `paladin-doc-examples`'s own crate docs both make this claim
explicitly. A crate whose purpose is a genuine extraction of facade logic (the ADR-0031 case)
still may not acquire a leaf-to-leaf edge outside the two conditions that ADR-0031's own
sub-decision (i) already states (a non-default optional feature, activated by an explicit facade
opt-in).

**`paladin-eval` is published** (`publish = true`, versioned with the workspace, registered in
`scripts/publish-crates.sh`'s dependency-ordered array and the `Makefile`'s `publish-dry-run`
target) because a downstream team's own `[dev-dependencies]` on it — writing their own eval
scenarios against their own graphs — is the entire point of a "dev-dependency-oriented" harness
crate. It is deliberately **excluded** from `.github/workflows/ci.yml`'s `semver` job's package
list: `cargo semver-checks check-release --baseline-version 0.9.0` needs a published `0.9.0`
release of the crate to diff against, and `paladin-eval` did not exist at that version — running
the check against it would error, not skip. The exclusion carries an inline comment in the
workflow file recording this reason, rather than a silent omission a future reader would have to
re-derive.

**`crates/doc-examples`'s dependency SHAPE is cited as precedent for "a composition crate may
depend on several siblings"; it is not cited as precedent for the PUBLISHING decision.**
`paladin-doc-examples` ships with `publish = false` — it is a workspace-internal example crate,
never intended for a downstream consumer's own `Cargo.toml`. `paladin-eval`'s publishing decision
rests on its own reasoning above (a downstream team's own scenario authoring is the whole
product), not on any inference from `doc-examples`'s manifest.

## Considered Options

- **Classify `paladin-eval` as a composition crate, scoped narrowly to crates whose own docs
  state the multi-sibling-dependency purpose explicitly** (accepted) — matches what the crate
  actually is, has a real precedent in `doc-examples`'s dependency shape, and does not weaken
  ADR-0031's invariant for the crates it was written to govern (the genuinely extracted ones).
- **Treat `paladin-eval` as an extracted crate and require it to depend only on `paladin-core`/
  `paladin-ports`** (rejected) — impossible for what the crate does: it cannot drive a
  `WarEngine` without `paladin-battalion`, cannot script an `LlmPort` without `paladin-llm`, and
  cannot offer a `sqlite_temp` store without `paladin-storage`. Rejecting this option is not a
  compromise; those three dependencies are the crate's entire reason to exist.
- **Amend ADR-0031 itself to add a composition-crate carve-out inline** (rejected) — ADR-0031 is
  a restatement of a Milestone 7 extraction rule; folding an unrelated Phase 28 tool crate's
  classification into that document would blur what ADR-0031 is actually about (extraction, not
  composition) for a future reader trying to apply either rule. A separate ADR that CITES
  ADR-0031's own scope language, rather than amending it, keeps each decision legible on its own.
- **Leave `paladin-eval` unpublished (`publish = false`), like `doc-examples`** (rejected) —
  defeats D-27's own stated purpose: a downstream team writing their own eval scenarios needs the
  crate on crates.io as an ordinary `[dev-dependencies]` entry, not a workspace-internal-only
  example.

## Code Locations

- `crates/paladin-eval/Cargo.toml` — `publish = true`, the five downward dependency edges
  (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm` with `default-features =
  false, features = ["mock"]`, `paladin-storage` with `features = ["sqlite"]`).
- `crates/paladin-eval/src/lib.rs` — the crate's own module doc stating the composition-crate
  purpose and the `cli`-feature exception this ADR records.
- `crates/doc-examples/Cargo.toml` — the dependency-shape precedent this ADR cites (six workspace
  crates, `publish = false` — the shape is precedent, the publishing posture is not).
- `.planning/decisions/0031-extracted-crate-dependency-rule.md` — the invariant this ADR clarifies
  the scope of, not contradicts.
- `scripts/publish-crates.sh` — the dependency-ordered publish array, `paladin-eval` inserted
  after `paladin-storage` and before the facade (`paladin-ai`).
- `Makefile` — the `publish-dry-run` target, `paladin-eval`'s dry-run line inserted after
  `paladin-storage`'s.
- `.github/workflows/ci.yml` — the `semver` job's package list, carrying an inline comment
  recording `paladin-eval`'s deliberate exclusion (no `0.9.0` baseline to diff against).
- `Cargo.toml` (root, facade) — the `cli` feature's optional `paladin-eval` dependency edge
  (28-12, D-33), the one deliberate upward exception this ADR records rather than treats as a
  violation.

## Code Conformance

conforms

The tree already ships exactly the shape this ADR classifies: five downward default-build edges,
one `cli`-gated upward edge from the facade, `publish = true`, and the crate registered in the
publish script's dependency-ordered array. This ADR instructs no code change — it records the
classification so a future reader of ADR-0031 does not mistake `paladin-eval`'s dependency edges
for a violation.

## Downstream Consumers

- **Phase 29 (SHIP gates)** — the semver job's exclusion list and the `MIGRATION.md` §9.3 crate
  registration are checked against this ADR's stated reasoning, not re-derived.
- **A future phase adding a thirteenth crate** — if it, too, depends on multiple siblings by
  design (another tool or harness crate), this ADR is the citable precedent for the
  composition-crate classification, narrowed by the same "states the purpose in its own module
  docs" test this ADR applies to `paladin-eval` itself.
- **Any future reader of ADR-0031** who encounters `paladin-eval`'s dependency edges and wonders
  whether the extracted-crate invariant is being silently violated — this ADR is the answer.
