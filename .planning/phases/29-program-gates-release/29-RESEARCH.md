# Phase 29: Program Gates & Release - Research

**Researched:** 2026-09-09
**Domain:** Rust workspace release engineering — MIGRATION.md closure, backward-compat proof
tests, program acceptance audit, version bump & dry-run publish. No new library/framework is
being adopted; this is a verification-and-evidence phase over an existing 12-crate workspace.
**Confidence:** HIGH — every claim below is either `[VERIFIED]` by running the actual command/
test against HEAD in this devcontainer, or a direct code/file citation with line numbers.
Nothing here is `[ASSUMED]`.

## Summary

Phase 29 builds nothing new; it proves the v0.10.0 program is done and prepares the release
commit. The 12-item research list in the task brief was answered by reading and, wherever
possible, **executing** the actual tree (not just reading CONTEXT.md's claims about it). Four
findings materially correct or sharpen premises baked into `29-CONTEXT.md`'s decisions, and must
reach the planner before D-07/D-08/D-01/D-04 are turned into tasks:

1. **`config.example.yml` cannot be loaded by `Settings::load_from_file` — at v0.9.0 OR at
   HEAD.** `[VERIFIED]` by running the loader against it: `LOAD FAILED: missing configuration
   field "llm.ollama.api_key"`. This is a pre-existing, cross-cutting gap (the `ollama:` block in
   `config.example.yml` has never carried an `api_key` line, at either tag), not something Phase
   29 introduces. D-06/D-07's frozen fixture, if it is a byte-for-byte copy of
   `v0.9.0:config.example.yml`, will fail to load and the SHIP-02 boot test cannot be written as
   literally specified. The existing precedent test (`test_load_from_file_regression`) already
   works around this by using `config.test.yml`, not `config.example.yml`.
2. **The "no `/v1/runs`… route answers anything but 404" premise in D-07 is not what the code
   does.** `[VERIFIED]` by test and by MIGRATION.md §9.6 itself: every new v0.10 route (`/v1/runs`,
   `/v1/threads`, `/v1/assistants`, `/v1/schedules`) answers **`501 Not Implemented`** when its
   backing config is disabled, not 404 — confirmed by `run_api_wiring.rs::defaults_wire_nothing_
   and_answer_501` and `thread_controller.rs::thread_router_returns_501_over_http_when_unwired_
   and_auth_disabled`, and documented in `MIGRATION.md` §9.6's own response-code tables (e.g. line
   458: `GET /v1/runs` → `200, 400, 401, 501`). Only the truly unwired `dev-ui` router (never called
   from `paladin-server.rs` at all, gated behind a non-default cargo feature) falls through to a
   plain axum "no route matched" 404 — for a different reason (not registered) than "disabled by
   config" (501).
3. **None of the nine platform config structs D-07 names (`run_queue`, `run_store`, `run_worker`,
   `run_stream`, `schedules`, `webhooks`, `assistants`, `waypoint_store`, `engine`) are fields on
   `Settings` at all.** `[VERIFIED]` by reading `settings.rs`'s full field list and
   `paladin-server.rs`'s `run()`. Only `agent_runtime`, `trace`, `web_server` are `Settings`
   fields (YAML-driven). The other nine are built with `X::default()` then `apply_env_overrides()`
   directly in the binary's `run()` — entirely independent of the config **file**. The "Config
   resolution" half of D-07 (asserting these nine equal `Default` on a loaded `Settings`) tests a
   struct field that does not exist; the real assertion has to be "these structs' `::default()`
   values are inert AND no ambient `APP_*`/host env var flips them" — a different, narrower claim
   than "the v0.9 file leaves them off."
4. **`cargo publish --workspace --dry-run --allow-dirty` genuinely works on this toolchain and was
   run to completion in this research session.** `[VERIFIED]` — all 12 publishable crates
   (`paladin-ai-core`, `paladin-ports`, `paladin-herald`, `paladin-llm`, `paladin-memory`,
   `paladin-notifications`, `paladin-storage`, `paladin-web`, `paladin-battalion`,
   `paladin-content`, `paladin-eval`, `paladin-ai`) packaged and verified in dependency order,
   each ending in the expected `warning: aborting upload due to dry run` — zero errors, at the
   current (unbumped) `0.9.0` version. `paladin-doc-examples` (`publish = false`) was correctly
   skipped. This fully confirms D-20's premise before any code is written.

Two further corrections to CONTEXT.md's stated facts, both low-risk but worth fixing before they
propagate into the plan text:

5. **§9.2 has 26 data rows (not 28) and 10 `Y`-marked rows mapping to only 9 distinct
   `(crate, type)` pairs** (`paladin-ai | Settings` is marked `Y` twice — line 178, the
   `agent_runtime` field, and line 191, the `trace`/`web_server` fields — and MIGRATION.md's own
   text at line 191 says the second doesn't need a new allowlist entry). D-04's row-level CI check
   must dedupe to a **set** of pairs before comparing against the allowlist's 9 entries, or a
   naive row-count comparison will report a false 10-vs-9 mismatch on a tree that is actually
   correct.
6. **`grep -c 'TBD' MIGRATION.md` returns 4 today, not 3** — the header note (lines 7 **and** 8)
   each carry one literal `TBD`, plus §9.5 (line 318) and §9.8 (line 630). D-01's rewritten header
   must remove both header occurrences and must not describe "the TBD convention" using the literal
   word `TBD`, or the new CI gate will immediately re-trip on its own prose.

**Primary recommendation:** treat this phase as evidence-gathering plus small, targeted edits to
already-existing scripts/CI steps/docs — every mechanism named in CONTEXT.md's decisions
(`cargo-release`, `finalize-crate-changelogs.sh`, `check-release-consistency.sh`,
`publish-crates.sh`, the semver/MSRV CI jobs, `gsd-tools windows`) already exists and works;
the plan's job is sequencing edits to them and writing the two new tests (D-07, D-08) with the
corrected premises above.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| MIGRATION.md / semver register closure | Docs / CI config | — | Documentation + a `.github/workflows/ci.yml` gate step; no runtime code |
| v0.9-config boot proof | API / Backend (binary composition root) | — | Exercises `paladin-server`'s own `run()` wiring in-process; no new route logic |
| OpenAPI golden diff | API / Backend | — | Reads the existing `utoipa`-generated `OpenApi` document; no schema changes |
| Program acceptance audit | Docs / CI evidence | Database/Storage (WINDOWS.md ledger) | A markdown artefact plus `gsd-tools windows` ledger transitions |
| Version bump & dry-run publish | Release tooling (Cargo/CI) | Docs (changelogs) | `cargo-release` + `cargo publish --dry-run`, both CLI-level, no application code |

No browser/client or CDN tier is touched by this phase at all — everything is backend Rust,
CI config, or documentation.

## User Constraints

<user_constraints>
### Locked Decisions (from 29-CONTEXT.md, D-01…D-25)

All 25 decisions in `29-CONTEXT.md` are locked (`--auto` mode; every decision took its
recommended option). They are not re-litigated here. In summary, by area:

- **SHIP-01 (MIGRATION.md closure):** D-01 turns "no TBD" into a durable CI grep-gate; D-02
  writes §9.8 as one ordered checklist using only CLI commands that actually exist; D-03 adds a
  new hand-written `docs/src/api-reference/upgrading.md` page (not an `{{#include}}` of
  MIGRATION.md, because `docs/book.toml`'s `warning-policy = "error"` linkcheck would break on
  MIGRATION.md's relative repo-root links); D-04 tightens the allowlist↔§9.2 CI check from
  crate-level to row-level; D-05 audits §9.2's 28 (see Pitfall 6 below: actually 26) rows against
  `cargo semver-checks` output and the existing `scripts/extract-public-api.sh` snapshot (no new
  tool).
- **SHIP-02 (backward-compat proofs):** D-06 freezes `git show v0.9.0:config.example.yml` as a
  committed fixture with provenance; D-07 writes one new root integration test
  (`tests/integration/v0_9_config_boot_test.rs`, target name `v0_9_config_boot`) asserting config
  resolution AND route-mounting behavior; D-08 writes a path-restricted `$ref`-closure OpenAPI
  golden diff test in `crates/paladin-web/tests/openapi_golden_v0_9.rs`; D-09 notes both are
  ordinary `cargo test --workspace` targets, no new CI job.
- **SHIP-03 (program acceptance audit):** D-10 creates `.project/v0.10.0/09-program-acceptance-
  audit.md` (pointed to by `29-ACCEPTANCE-AUDIT.md`); D-11 builds a per-FR table seeded from
  existing phase VERIFICATION/VALIDATION artefacts; D-12 bounds allowed fixes to docs/tests/
  citations/CI-gates/changelog text only — any production code fix is an X-03 stop-and-flag; D-13
  scopes "orphan behavior" to integration/E2E test targets added since v0.9.0 plus the four
  `paladin-eval` scenarios; D-14 checks ubiquitous-language conformance by table+grep, filing
  (not renaming) deviations; D-15 re-verifies BUG-01 evidence by SHA and grep, not re-testing;
  D-16 ⚠ accepts the Phase 28 bench-overhead FAIL as a documented deviation (developer may
  overturn at plan review); D-17 leaves judgment-tier sign-offs as unchecked `- [ ]` boxes for a
  human.
- **SHIP-04 (bump, changelogs, dry-run publish):** D-18 lands the 0.10.0 bump on this branch via
  `cargo release version 0.10.0 --execute --no-confirm --workspace`, no tag; D-19 uses `make
  finalize-crate-changelogs VERSION=0.10.0` then hand-curates the root CHANGELOG.md; D-20 rewrites
  the buggy `Makefile` `publish-dry-run` target to a single `cargo publish --workspace --dry-run`
  (confirmed working below), leaving `scripts/publish-crates.sh` (the real publish carrier) and
  `release.yml`'s `dry_run` dispatch untouched; D-21 records CI evidence for both the PR head and
  the eventual `main` merge commit; D-22 relies on existing CI doc gates, no new tooling.
- **Close-out hygiene:** D-23 mirrors 27/28's CI-evidence form; D-24 triages WINDOWS.md's 25 open
  rows via `gsd-tools windows waive|fixed` (see Q8 below for exact syntax — it is `windows
  <verb>`, not `query windows.*`); D-25 bounds the doc sweep to five named items, including
  confirming whether `NextStep::Halt` exists (answered definitively in Q9 below: it does not).

### Claude's Discretion
- Exact wording/layout of the Upgrading page and the curated root changelog.
- Whether the throwaway FR-grep (D-11) becomes a kept `scripts/audit-fr-coverage.sh`.
- File split of the audit document (one file default; appendix if >~600 lines).
- Plan/wave ordering — CONTEXT.md's suggested order: (1) SHIP-02 tests + D-04 gate, (2) SHIP-03
  audit + WINDOWS triage, (3) SHIP-01 MIGRATION closure + Upgrading page, (4) SHIP-04 bump +
  changelogs + evidence, last.
- Whether the v0.9.0 fixtures carry provenance in a sibling README vs per-file headers.

### Deferred Ideas (OUT OF SCOPE)
- `TraceDispatcher::emit`/`LogTraceSink` serialization optimisation (D-16 follow-up).
- `Frontier` → `Vanguard` rename (D-14 files it; X-10 break, not this phase).
- Adopting `cargo public-api` as a first-class tool (D-05 uses the existing snapshot).
- Folding `migration-guide.md`'s v0.1–v0.5 history into MIGRATION.md.
- `release.yml`'s `dry_run` dispatch calling `cargo publish --workspace --dry-run` (only if a
  one-line swap — see Q6 below: it is NOT a one-line swap, so leave it).
- The pre-existing `qdrant --all-features` rustdoc break; **and, newly identified below, the much
  larger pre-existing default-feature `cargo doc` warning set (72 warnings) — see Pitfall 7. Both
  stay deferred; neither is a Phase 29 SHIP-04 blocker per the requirement text.**
- FUT-01…05, a v0.11 MIGRATION.md scaffold, and everything else named out of scope by the corpus.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHIP-01 | MIGRATION.md complete, no TBD, §9.2 matches allowlist exactly, linked from README + Upgrading page | Q4 (row-level match verified exactly for 9 pairs, with a dedup caveat), Q11 (exact TBD count = 4, not 3), Q1/Q2 (D-02's real CLI commands) |
| SHIP-02 | Backward compat proven: v0.9-config boot test + openapi golden diff | Q2 (Settings field enumeration + the config.example.yml load failure + the 404-vs-501 correction), Q3 (openapi_spec() reachability, $ref closure, 6-path v0.9.0 baseline confirmed) |
| SHIP-03 | Program acceptance audit: E2E-1/2/3, doc-08 protocol, BUG-01 grep-absent | Q9 (Halt errata), Q10 (FR counts = 159 across docs 01-07; 13 new `[[test]]` targets + ~30 new test files since v0.9.0) |
| SHIP-04 | v0.10.0 releasable: bump, changelogs, dry-run publish, docs gates, semver/MSRV green | Q5 (cargo-release 1.1.2 confirmed, exact `0.9.0` occurrence inventory), Q6 (dry-run publish VERIFIED end-to-end), Q7 (exact script invocation syntax), Q12 (cargo doc warning counts measured both ways) |
</phase_requirements>

## Standard Stack

No new libraries are introduced by this phase. Every tool named below is already vendored,
installed, or present in this devcontainer and was directly exercised during this research.

### Core (already in place, confirmed present/working)
| Tool | Version (measured) | Purpose | Confirmed |
|------|---------------------|---------|-----------|
| `cargo-release` | 1.1.2 | Workspace version bump (lockstep) | `cargo release --version` ran clean; `release.toml` confirms `shared-version = true`, `publish = false`, `push = false` |
| `cargo publish` | via cargo 1.97.1 | Dry-run packaging/verification | `--workspace` and `--dry-run` both exist as documented flags; `cargo publish --workspace --dry-run --allow-dirty` run to completion, all 12 crates verified, zero errors `[VERIFIED]` |
| `mdbook` | 0.4.40 | Book build | `mdbook --version` succeeded locally |
| `mdbook-linkcheck` | 0.7.7 | Doc link-check (`warning-policy = "error"`) | `mdbook-linkcheck --version` succeeded locally |
| `cargo semver-checks` | (existing CI job) | Public-API diff vs published v0.9.0 | `.github/workflows/ci.yml:304-357` — job pins `--baseline-version 0.9.0` literally; **do not rewrite these occurrences during the version bump** (see Pitfall 4) |
| `rustc`/`cargo` | 1.97.1 | Toolchain (release-time; MSRV gate separately pins 1.88) | `rustc --version` |

### Supporting scripts (already exist, exact invocation confirmed)
| Script | Invocation | Confirmed |
|--------|-----------|-----------|
| `scripts/finalize-crate-changelogs.sh` | `--version <X.Y.Z>` (bare semver, **no** leading `v`) `[--date YYYY-MM-DD] [--metadata-json <path>] [--workspace-root <path>]` — `make finalize-crate-changelogs VERSION=0.10.0` | Usage string read at `scripts/finalize-crate-changelogs.sh:122` |
| `scripts/check-release-consistency.sh` | `--tag <vX.Y.Z|X.Y.Z>` (leading `v` accepted) `[--metadata-json][--workspace-root][--sha][--ci-runs-json]` — `./scripts/check-release-consistency.sh --tag v0.10.0` | Usage string at `scripts/check-release-consistency.sh:274` |
| `scripts/publish-crates.sh` | `--version <X.Y.Z> [--dry-run] [--poll-timeout <s>] [--poll-interval <s>] [--crates-file <path>]` | Header comment + arg parsing, unchanged per D-20 |
| `node .claude/gsd-core/bin/gsd-tools.cjs windows <status|append|waive|fixed> ...` | `status` (no args); `append --kind K --phase N [--file F][--line L] --description D`; `waive <id> "<reason>"`; `fixed <id>` | Read directly from `bin/lib/broken-windows.cjs:585-660` and the top-level `'windows': routeWindows` dispatch table at `bin/gsd-tools.cjs:2106`. **Correction to CONTEXT.md's phrasing:** the command is `gsd-tools windows <verb>`, not `query windows.*`. Valid `--kind` values: `stub, todo, fixme, skipped-test, lint-warning, unmet-truth, unrun-verify, deviation`. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Per-crate `cargo publish --dry-run` loop (today's broken Makefile target) | `cargo publish --workspace --dry-run` | The workspace form resolves intra-workspace deps locally instead of against the (not-yet-published) registry version — this is *why* the per-crate loop needs `\|\| true` and the workspace form doesn't. **Confirmed working, not merely documented, in this session.** |
| `cargo public-api` (new dependency) | The existing `scripts/extract-public-api.sh` / `.project/current-exports.txt` snapshot, diffed against `git show v0.9.0:.project/current-exports.txt` | D-05's own rejection reason: a new tool in a release-gate phase for a diff the tree already produces |

**Installation:** none required — everything above is already present in this devcontainer.

## Package Legitimacy Audit

Not applicable — this phase adds no new external dependency to any `Cargo.toml`. All tooling
used (`cargo-release`, `mdbook`, `mdbook-linkcheck`) is already an installed devcontainer tool or
an existing workspace dev-dependency; no new `[[entry]]` in any manifest is anticipated.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────┐
                    │   MIGRATION.md (§9.1-9.8)    │
                    │  + .cargo/semver-checks-     │
                    │    allowlist.toml            │
                    └───────────┬─────────────────┘
                                │ D-01 (TBD gate) / D-04 (row-level match)
                                ▼
                    ┌─────────────────────────────┐
                    │ .github/workflows/ci.yml     │
                    │  `semver` job (11/11 crates, │
                    │  --baseline-version 0.9.0)   │
                    └───────────┬─────────────────┘
                                │ gates PR merge
                                ▼
   ┌────────────────────────────────────────────────────────┐
   │  cargo test --workspace                                 │
   │  ┌──────────────────────────┐  ┌───────────────────────┐│
   │  │ v0_9_config_boot_test.rs │  │ openapi_golden_v0_9.rs ││
   │  │ (root, D-07)             │  │ (paladin-web, D-08)    ││
   │  │ 1. Settings::load_from_  │  │ openapi_spec() ->      ││
   │  │    file(fixture) — NOTE: │  │ path-restrict to 6     ││
   │  │    fails on config.      │  │ v0.9 paths -> $ref     ││
   │  │    example.yml today     │  │ closure -> diff vs     ││
   │  │    (Pitfall 1)           │  │ frozen v0.9.0 baseline ││
   │  │ 2. build paladin-server  │  └───────────────────────┘│
   │  │    in-process from that │                            │
   │  │    Settings -> hit every│                            │
   │  │    v0.10 route -> expect│                            │
   │  │    501 (NOT 404,        │                            │
   │  │    Pitfall 2)           │                            │
   │  └──────────────────────────┘                           │
   └────────────────────────────────────────────────────────┘
                                │
                                ▼
     .project/v0.10.0/09-program-acceptance-audit.md (D-10)
     ── per-FR table (159 FRs across docs 01-07, D-11) ──
     ── orphan-behavior scope: 13 new [[test]] targets +  
        ~30 new test files since v0.9.0 (D-13) ──
     ── BUG-01 grep-absence + ubiquitous-language table (D-14/D-15) ──
                                │
                                ▼
        cargo release version 0.10.0 --execute --workspace (D-18)
        -> regenerate openapi.json baseline (info.version only)
        -> make finalize-crate-changelogs VERSION=0.10.0 (D-19)
        -> cargo publish --workspace --dry-run (D-20, VERIFIED)
        -> WINDOWS.md triage via gsd-tools windows waive|fixed (D-24)
                                │
                                ▼
                29-CI-EVIDENCE.md (pre-merge) -> /gsd-ship -> main merge
                (post-merge evidence + tag, D-21, out of this phase)
```

### Recommended Project Structure (new files this phase adds)
```
tests/
├── fixtures/config/v0.9.0-config.example.yml   # D-06, frozen, provenance header
├── integration/
│   └── v0_9_config_boot_test.rs                # D-07, [[test]] name = "v0_9_config_boot"
crates/paladin-web/tests/
├── fixtures/openapi-v0.9.0.json                # D-08, frozen, provenance sibling .md
└── openapi_golden_v0_9.rs                      # D-08
docs/src/api-reference/
└── upgrading.md                                # D-03, new page above Migration Guide in SUMMARY.md
.project/v0.10.0/
└── 09-program-acceptance-audit.md              # D-10, ten `##` sections mirroring doc-08
.planning/phases/29-program-gates-release/
└── 29-ACCEPTANCE-AUDIT.md                      # D-10 pointer + verdict line
```

### Pattern 1: Config struct inertness is proven two different ways in this codebase
**What:** `Settings`-embedded structs (`agent_runtime`, `trace`, `web_server`) prove inertness via
`#[serde(default)]` + a YAML round-trip test. The nine platform structs (`run_store`,
`run_queue`, `run_worker`, `run_stream`, `assistants`, `schedules`, `webhooks`,
`waypoint_store`, `engine`) prove inertness via `X::default()` + `apply_env_overrides()` +
`validate()` called directly in `paladin-server.rs`'s `run()`, with **no** file-driven path at
all.
**When to use:** Any assertion about "a v0.9 config boots with legacy behavior" must test BOTH
mechanisms, not just `Settings::load_from_file`. A test that only loads a YAML file will
correctly show the nine platform structs are absent from `Settings` — trivially, since they were
never fields on it — but that proves nothing about env-var pollution, which is the actual risk
D-07's "env-isolation pattern" motivation names.
**Example:**
```rust
// Source: src/bin/paladin-server.rs:103-158 (verified reading, HEAD)
let mut run_store_config = RunStoreConfig::default();
run_store_config.apply_env_overrides();   // reads APP_RUN_STORE_* directly, NOT from Settings
run_store_config.validate().map_err(...)?;
```

### Pattern 2: 501 vs 404 vs plain-axum-404 are three distinct "not available" signals
**What:** A disabled platform subsystem answers `501 Not Implemented` (an explicit `ApiError::
not_implemented` branch, proving the route IS registered but its backing port is `None`). A
route that was never merged into the router at all (e.g. `dev-ui` when `paladin-server.rs` never
calls `create_dev_ui_router`) falls through to axum's own unmatched-route 404, which carries no
custom body. `/health`/`/ready` and pre-existing `/v1/agents/*` routes are the only true "200 or
domain-specific 4xx" surface untouched by any of this.
**When to use:** Any assertion in D-07's behavioral half or D-08's golden diff must name the
correct code per route family — asserting "404" against `/v1/runs` when it is disabled will make
the test fail against a correctly-behaving server.
**Example:**
```rust
// Source: crates/paladin-web/src/thread_controller.rs (test name, verified present at HEAD)
async fn thread_router_returns_501_over_http_when_unwired_and_auth_disabled() { /* ... */ }
// Source: src/infrastructure/web/run_api_wiring.rs (test name, verified present at HEAD)
async fn defaults_wire_nothing_and_answer_501() { /* asserts StatusCode::NOT_IMPLEMENTED */ }
```

### Anti-Patterns to Avoid
- **Row-count comparison for D-04's allowlist check:** §9.2 has 10 `Y`-marked rows but only 9
  distinct `(crate, type)` pairs (the `paladin-ai | Settings` row is duplicated across the
  `agent_runtime` field and the `trace`/`web_server` fields, landed in two different phases). The
  CI step must build a **deduplicated set** on both sides before comparing, exactly as the
  existing crate-level step already does ("sorted unique crate names").
- **Blind find-and-replace of "0.9.0" → "0.10.0" across the tree:** `.github/workflows/ci.yml`'s
  `semver` job hardcodes `--baseline-version 0.9.0` in five places (lines 276, 304, 334, 339, 357)
  by design — it diffs the new release against the last **published** version, which stays
  `0.9.0` regardless of the working tree's own version. Rewriting these would silently disable
  the entire semver-diff job (it would diff v0.10.0 against itself, or error on a nonexistent
  `0.10.0` baseline before the tag is cut).
- **Rewriting MIGRATION.md's header note to still contain the literal word "TBD":** D-01's new CI
  step (`grep -c 'TBD' MIGRATION.md` must be zero) will immediately re-trip if the closed-state
  header describes "the TBD convention" using that exact four-letter token — use "placeholder" or
  paraphrase instead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Version bump across 13 manifests | A sed/awk script over every `Cargo.toml` | `cargo release version 0.10.0 --execute --no-confirm --workspace` | Already installed (1.1.2), already configured for this exact lockstep shape in `release.toml`, already the `make release` mechanism |
| Per-crate changelog dating | Manual edits to 12 files | `scripts/finalize-crate-changelogs.sh --version 0.10.0` | Idempotent, enumerates publishable packages from `cargo metadata` (never a hardcoded list), already used at v0.9.0 |
| "Is every crate really publishable in order" evidence | A hand-run `cargo publish --dry-run` loop with manual bookkeeping | `cargo publish --workspace --dry-run --allow-dirty` | **Verified working end-to-end in this session** — packages and cross-verifies all 12 crates using local path resolution in one command, correctly skips `publish = false` `paladin-doc-examples` |
| Public-API diff vs v0.9.0 | Adopting `cargo public-api` | `scripts/extract-public-api.sh` + `git show v0.9.0:.project/current-exports.txt` | Already exists, already CI-maintained (`api-surface` job), zero new dependency |
| WINDOWS.md row-status edits | Direct markdown/YAML-frontmatter edits | `node .claude/gsd-core/bin/gsd-tools.cjs windows waive <id> "<reason>"` / `windows fixed <id>` | Atomic write (`writeLedgerAtomic`), validates id/kind against the schema, keeps `open_count`/`waived_count`/`fixed_count` frontmatter consistent |

**Key insight:** every mechanism this phase needs was built by an earlier phase specifically to
support this close-out (the Makefile comments, script headers, and `release.toml` all say so
explicitly). The work is almost entirely "run it, fix the two things that don't quite match
today's tree (the Makefile target's stale crate names, the two `config.example.yml`/501-vs-404
premise corrections above), write two new tests, and produce two new documents."

## Common Pitfalls

### Pitfall 1: `config.example.yml` does not load, at either tag — verified, not assumed
**What goes wrong:** A test built on "commit `git show v0.9.0:config.example.yml` verbatim, then
assert `Settings::load_from_file` succeeds" fails immediately with `missing configuration field
"llm.ollama.api_key"` — confirmed by directly running the loader against the file in this
session. The `ollama:` block in `config.example.yml` documents no `api_key` line, at v0.9.0 or
HEAD (`git show v0.9.0:config.example.yml | grep -A5 ollama` is byte-identical to HEAD's block).
**Why it happens:** `LlmProviderConfig::api_key` (`crates/paladin-llm/src/config/llm.rs:11`) is a
required `String`, not `Option<String>`; the example file's Ollama section was written assuming
no key is needed for a local Ollama server, which is true operationally but not schema-valid.
**How to avoid:** Either (a) use `config.test.yml` at v0.9.0 as the frozen fixture instead
(it has no `llm.ollama` section at either tag and is proven to load via the existing
`test_load_from_file_regression`), documenting that it is the closest thing to "the sample config
an operator has" that actually parses; or (b) keep `config.example.yml` as the fixture but note
in the test/fixture provenance comment that this is a **known, pre-existing, out-of-scope
defect** unrelated to Phase 29 and either strip the `ollama:` block or add a placeholder
`api_key: "unused"` line — but note this makes the fixture NOT byte-identical to `git show
v0.9.0:config.example.yml`, which is exactly what D-06 rejected ("hand-trimmed" fixtures). This
tension is a genuine open decision for the plan, not something this research can resolve
unilaterally — flagged prominently in Open Questions below.
**Warning signs:** Any `<verify>` step that assumes `Settings::load_from_file(fixture).is_ok()`
without having actually run it against the exact frozen file first.

### Pitfall 2: The v0.10 platform routes answer 501, never 404, when disabled
**What goes wrong:** A test asserting `assert_eq!(response.status(), StatusCode::NOT_FOUND)`
against `/v1/runs`, `/v1/threads`, `/v1/assistants`, or `/v1/schedules` on a v0.9-shaped config
will fail — the actual, documented, tested response is `501`.
**Why it happens:** `run_api_wiring.rs`'s own module doc states the behavior explicitly:
"every new route answers `501 not_implemented`... when [`RunStoreBackend::Disabled`]". MIGRATION.md
§9.6's response-code tables (lines 409-520) list `501` for every one of these routes.
**How to avoid:** Write the boot test's route assertions against `501` for the nine new-in-v0.10
route families, and reserve `404` only for the truly-unregistered `dev-ui` route (a different,
unwired-at-compile-or-composition-time case) and for genuinely unknown paths.
**Warning signs:** A route assertion table in the plan that says "404" next to `/v1/runs` etc.

### Pitfall 3: The nine platform config structs are not `Settings` fields
**What goes wrong:** Code (or a test) that does `settings.run_store` or similar will not compile
— `Settings` (src/config/settings.rs:27-78) has no such field. Only `agents`, `timeouts`, `http`,
`agent_runtime`, `trace`, `web_server` exist on `Settings` among the v0.10-era additions.
**Why it happens:** These nine structs are deliberately kept OFF `Settings` (a documented X-10
avoidance decision — see the `EngineConfig` comment at `paladin-server.rs:96-101`: "`EngineConfig`
(not `Settings`, X-10 avoidance) is the one config struct feeding both the engine and this
process-level wait") specifically so adding them never touches `Settings`'s already-`Y`-registered
semver surface.
**How to avoid:** Write D-07's "config resolution" assertions against each struct's own
`X::default()` (proving inertness) and against a scoped-env-var test proving `apply_env_overrides`
does nothing when no `APP_*`/specific env var is set — not against a deserialized `Settings`
value, which never carries these fields.
**Warning signs:** A plan task referencing `settings.run_queue` or similar field access.

### Pitfall 4: The `--baseline-version 0.9.0` occurrences in ci.yml are permanent, not a bump target
**What goes wrong:** A version-bump script or sweeping `sed 's/0.9.0/0.10.0/g'` over `.yml` files
corrupts the semver job, which exists specifically to diff the working tree against the
**last-published** version.
**Why it happens:** The literal string `0.9.0` appears 5 times in `.github/workflows/ci.yml`
(lines 276, 304, 334, 339, 357), all inside the `semver` job's own comments/args, deliberately
pinned.
**How to avoid:** Scope the version-bump grep/sweep to `Cargo.toml` files, `CHANGELOG.md` dated
headings, and `crates/paladin-web/openapi.json`'s `info.version` only — never `.github/
workflows/*.yml`. `cargo-release` itself will not touch this file (it only rewrites manifests),
so this risk is really about a *manual* verification grep, not the bump command itself.
**Warning signs:** A `grep -rl '0.9.0'` sweep intended for the bump that includes `.github/`.

### Pitfall 5: §9.2's `Y`-row count (10) does not equal its distinct-pair count (9)
**What goes wrong:** D-04's row-level allowlist check, if implemented as "count of `Y` rows must
equal count of allowlist entries," reports a false mismatch (10 ≠ 9) on a tree that is actually
fully consistent.
**Why it happens:** `paladin-ai | Settings` is marked `Y` on two separate MIGRATION.md rows (line
178, `agent_runtime` field, Phase 26; line 191, `trace`+`web_server` fields, Phase 28) because the
type gained public fields in two different phases. MIGRATION.md's own row-191 text says the
second occurrence deliberately needs no new allowlist entry, since the existing
`constructible_struct_adds_field` suppression already covers the type for any additional field.
**How to avoid:** Build the CI check as a **set** comparison (`sort -u` both sides) exactly as the
existing crate-level step already does, not a row/entry count comparison.
**Warning signs:** A verification step phrased as "N rows marked Y must equal N allowlist
entries" rather than "the set of (crate, type) pairs must match exactly."

### Pitfall 6: `MIGRATION.md` §9.2 has 26 data rows, not 28
**What goes wrong:** A verification step that asserts "28 rows" (CONTEXT.md's own count) will
report a false discrepancy.
**Why it happens:** Counting rows between `## 9.2` (line 159) and `## 9.3` (line 268) yields 28
total `|`-prefixed lines, but 2 of those are the table header and its `---` separator row, leaving
26 actual data rows (lines 167-192).
**How to avoid:** When writing the audit, count only lines matching the data-row pattern (starts
with `` | ` `` — a backtick immediately after the leading pipe), which correctly excludes the
header/separator.

### Pitfall 7: `cargo doc --workspace --no-deps` (the exact CI command) is ALREADY red at HEAD
**What goes wrong:** SHIP-04 requires "no NEW broken intra-doc links," and the `lint` job's
"Check documentation" step (`ci.yml:62-63`: `cargo doc --workspace --no-deps 2>&1 | tee ... && !
grep -q "warning:" ...`) is a true zero-tolerance gate. **Measured live in this session:** this
exact command currently emits **72** `warning:` lines against default features — meaning this CI
step is failing on `main` right now, independent of and much larger than the previously-tracked
"qdrant `--all-features`" deferred item (which is a *different*, non-default-feature concern; the
72 measured here are on default features, the exact CI invocation). STATE.md's Phase 26 carried
concern already flagged "~60 pre-existing rustdoc warnings" — the count has grown to 72 by Phase
29's start.
**Why it happens:** Accumulated `[missing_docs]`/private-item-doc-link warnings across multiple
phases' additions (`src/config/agent_runtime.rs`, `src/infrastructure/telemetry/otel_sink.rs`,
`src/presets/mod.rs`, `src/application/services/run/worker.rs`, and more), none individually
large enough to trip a phase's own verification, accumulating into a persistently-red CI step
that nobody's phase scope covered.
**How to avoid:** SHIP-04's actual requirement text only asks that Phase 29 introduce no *new*
broken links and that the *semver and MSRV* jobs be green — it does not require the `lint` job's
doc-warning count to reach zero. Record the exact count (72) in `29-CI-EVIDENCE.md` as a
carried, pre-existing, out-of-D-25-scope condition (bounded doc sweep does not include fixing
these), and do not let the plan silently assume this step is green when it evidently is not.
**Warning signs:** A `<verify>` step assuming the `lint` job is fully green without checking the
specific "Check documentation" sub-step, or a plan task that scopes fixing all 72 warnings under
D-25's "bounded doc sweep" (it explicitly is not part of that list).

## Code Examples

### Q1 — `paladin-cli` verify-step commands that actually exist (D-02)
```
# Source: src/bin/paladin-cli.rs Commands enum (verified against clap derive at HEAD)
paladin-cli setup-check [--verbose]        # env/toolchain/provider/service health check
paladin-cli maneuver validate <flow-args>  # validates flow expression + Paladin config
paladin-cli graph export --format mermaid|dot (<FILE>|--assistant <id>) [--out <path>]
paladin-cli eval run <glob> [--repeat N] [--bless] [--live] [--registries <name>]
```
**There is no `paladin-cli health` command and no `paladin-cli graph validate` subcommand.**
`GraphCommands` (`src/application/cli/commands/graph.rs:42`) has exactly one variant:
`Export(GraphExportArgs)`. §9.8's checklist must name real commands: `setup-check` for an
environment/connectivity check, `maneuver validate` for config validation (if the deployment uses
the Maneuver DSL), and `eval run` against the E2E fixture scenarios for a behavioral
post-deploy check — never an invented `graph validate`.

### Q3 — OpenAPI golden diff seam (D-08), confirmed reachable
```rust
// Source: crates/paladin-web/src/openapi.rs:84-86 (pub fn, crate-root `pub mod openapi;`)
pub fn openapi_spec() -> OpenApi {
    build_openapi(AgentApiState::new(Arc::new(AgentRegistry::new())))
}
```
Reachable from a new `crates/paladin-web/tests/openapi_golden_v0_9.rs` as
`paladin_web::openapi::openapi_spec()` — no feature gate, no `required-features` entry needed
(the crate has no `[[test]]` blocks in `Cargo.toml`; any `.rs` file dropped into `tests/` is
auto-discovered). The v0.9.0 baseline has exactly 6 paths, all `/v1/agents...`, and
`components.securitySchemes` is present — `[VERIFIED]` via `git show v0.9.0:crates/paladin-web/
openapi.json | python3 -c "..."`.

### Q9 — `NextStep::Halt` does not exist (D-25 errata)
```rust
// Source: crates/paladin-core/src/platform/container/directive.rs:40-89 (full enum, verified)
pub enum NextStep {
    Edges,
    Goto(Vec<NodeId>),
    Muster(Vec<MusterTask>),
    End,
    Parley(ParleyRequest),
}
```
No `Halt` variant exists on `NextStep`/`Directive`. `Halted` exists only as a **run-outcome**
concept: `RunOutcome::Halted { waypoint }` and `WaypointStatus::Halted`
(`crates/paladin-battalion/src/engine/mod.rs:315` and around), reached via cancellation
(`ENG-FR-23`), never as a node's routing choice. `.project/v0.10.0/00-program-overview.md` §4's
Directive-row mention of "Halt" should be corrected to reference `WaypointStatus::Halted` /
`RunOutcome::Halted` (a distinct, run-level concept from the five real `NextStep` variants), not
treated as a sixth `NextStep` variant.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Per-crate `cargo publish --dry-run -p X \|\| true` loop, wrong crate names, missing `paladin-herald` | `cargo publish --workspace --dry-run` | This phase (D-20) | One command, correct crate set, no `\|\| true` masking real failures — verified working in this session |
| Crate-level allowlist↔§9.2 set-equality | Row-level `(crate, type)` pair set-equality | This phase (D-04) | Catches a row/entry naming a *different type* under the same crate, which the crate-level check structurally cannot detect |
| MIGRATION.md TBDs closed ad hoc, re-flagged every phase's VERIFICATION | A CI gate (`grep -c TBD`) | This phase (D-01) | Prevents the pattern recurring in v0.11 |

**Deprecated/outdated:** the `docs/RELEASE_CHECKLIST.md` path referenced by the current
`publish-dry-run` Makefile target message does not exist (`[VERIFIED]`, `ls` returns "No such
file or directory"); the real file is `docs/src/appendix/release-checklist.md`. D-20 already
plans to fix this pointer in the same edit.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `docs/src/api-reference/upgrading.md` should sit "directly above" the Migration Guide entry in `docs/src/SUMMARY.md` at line 70 (not independently re-verified against a post-Phase-28 SUMMARY.md edit) | Architecture Patterns / D-03 | Low — a one-line reorder if the exact line number drifted; the *target section* (SUMMARY.md, above Migration Guide) is not in question |
| A2 | The FR count of 159 (28+21+20+24+27+19+20 across docs 01-07) counts every `\b(ENG\|CF\|HITL\|FT\|RT\|PLAT\|OBS)-FR-\d+[a-z]?\b` occurrence per file as one unique FR; a handful of these may be sub-lettered variants of the same base FR (e.g. `ENG-FR-12a`) that the audit table might reasonably collapse to one row with its parent | Phase Requirements / Q10 | Low — affects the *expected row count* of D-11's per-FR table by a small amount, not its correctness |

**If this table is empty:** N/A — two low-risk sizing assumptions remain above; every functional
claim in this document (file contents, test names, CLI commands, script syntax, tool versions,
route status codes) was verified directly against the tree or by execution.

## Open Questions (RESOLVED)

> **Resolution record (plan-phase, 2026-09-10):** both questions below are RESOLVED and adopted by the plans — Q1 (which v0.9 fixture to freeze) → option (b), the two-fixture resolution, owned by `29-01-PLAN.md` and recorded in MIGRATION.md §9.5 by `29-05-PLAN.md`; Q2 (the pre-existing 72-warning `cargo doc` step) → recorded as a named WINDOWS.md `deviation` without expanding D-25's fix scope, owned by `29-07-PLAN.md` / `29-09-PLAN.md`.

1. **Which v0.9-sample-config fixture should D-06/D-07 actually freeze, given `config.example.yml`
   does not parse at either tag?**
   - What we know: `git show v0.9.0:config.example.yml` is byte-identical to HEAD's file in the
     `llm.ollama` block that causes the failure — this is not new drift from Phase 22-28, it
     predates the whole v0.10.0 program.
   - What's unclear: whether the planner should (a) use `config.test.yml` at v0.9.0 instead
     (loads cleanly, but is a test fixture, not "the sample config an operator has" — weakens the
     SHIP-02 narrative slightly), (b) freeze `config.example.yml` verbatim and additionally note
     in the test that it is *expected* to fail to load, with a **separate** loadable fixture doing
     the actual assertion (two fixtures, more faithful to both the "verbatim provenance" and the
     "provable claim" goals), or (c) fix the one missing `api_key: ""` line in the frozen copy
     with a clearly-documented deviation note (breaks "committed verbatim, no hand-trimming").
   - Recommendation: option (b) — freeze `config.example.yml` verbatim for provenance/documentation
     purposes (it IS what a v0.9 operator has), but also freeze `config.test.yml` at v0.9.0 (already
     loadable, already the codebase's precedent for this exact test class) as the fixture the new
     `v0_9_config_boot_test.rs` actually loads, with a code comment explaining why. This satisfies
     D-06's underlying goal (prove against something authentically v0.9-shaped) without asserting a
     false claim about a file that has never parsed.

2. **Should the pre-existing 72-warning `cargo doc --workspace --no-deps` red step be fixed in this
   phase, deferred again, or explicitly waived in WINDOWS.md?**
   - What we know: SHIP-04's requirement text does not require it (only "no NEW broken links" +
     semver/MSRV green); D-25's bounded doc sweep does not list it; it has grown from ~60 (Phase 26)
     to 72 (now) without any phase claiming ownership.
   - What's unclear: whether leaving a release-gate phase's own CI evidence silent about a known-red
     "Check documentation" step invites the same "TBD nobody owns" pattern D-01 exists to prevent.
   - Recommendation: record the exact count and the `lint` job's red status explicitly in
     `29-CI-EVIDENCE.md` (not silently omit it), and file it as a new WINDOWS.md `deviation` row if
     one does not already exist for it, so it is a tracked, named debt rather than an undocumented
     gap — without expanding D-25's scope to actually fix it.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo-release` | D-18 version bump | ✓ | 1.1.2 | — |
| `cargo publish --workspace --dry-run` | D-20 dry-run evidence | ✓ | cargo 1.97.1 | — |
| `mdbook` | D-22 docs gate (local check) | ✓ | 0.4.40 | — |
| `mdbook-linkcheck` | D-22 docs gate (local check) | ✓ | 0.7.7 | — |
| Network egress to crates.io | `cargo publish --dry-run`'s index update | ✓ | — (HTTP/2 403 on a bare `curl -I`, but `cargo`'s own sparse-index client succeeded — a plain `curl` probe is not a reliable proxy for cargo's registry client) | — |
| Docker | Postgres/Redis Tier-2 suites cited in WINDOWS.md rows | ✗ (not probed this session; prior phases' evidence consistently records "no Docker in this devcontainer") | — | Route to CI job run IDs, as every prior phase (23-28) already does |

**Missing dependencies with no fallback:** none — every tool this phase's decisions name is
present and was exercised.

**Missing dependencies with fallback:** Docker-gated suites (unchanged from every prior phase in
this program) — evidence comes from CI run IDs, not local execution.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (standard Rust test harness); `cargo-nextest` not detected/used in this repo's scripts |
| Config file | none dedicated — behavior driven by `Cargo.toml` `[[test]]` blocks and default `tests/*.rs` discovery |
| Quick run command | `cargo test --test v0_9_config_boot` / `cargo test -p paladin-web openapi_golden_v0_9` (once written) |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SHIP-01 | `MIGRATION.md` has zero TBD | CI gate (grep) | new step in `.github/workflows/ci.yml` `semver` job | ❌ Wave (D-01) |
| SHIP-01 | §9.2 register matches allowlist exactly (row-level) | CI gate (awk/shell) | extended step in `.github/workflows/ci.yml` `semver` job | ❌ Wave (D-04) |
| SHIP-02 | v0.9 config boots with legacy behavior (config + routes) | integration | `cargo test --test v0_9_config_boot` | ❌ Wave (D-07) |
| SHIP-02 | openapi.json pre-existing-paths golden diff is empty | integration | `cargo test -p paladin-web --test openapi_golden_v0_9` | ❌ Wave (D-08) |
| SHIP-03 | E2E-1/2/3 pass green | integration (existing) | `cargo test --test e2e_crash_resume --test e2e_approval_gate --test e2e_muster_defer_order` | ✅ already exist |
| SHIP-03 | BUG-01 old path grep-absent | manual/scripted grep | `grep -rn "defaulting to true" crates/ src/` (expect 0) | ✅ script is a one-liner, not a file |
| SHIP-04 | `cargo publish --workspace --dry-run` green | manual/CI evidence | `cargo publish --workspace --dry-run --allow-dirty` | ✅ VERIFIED this session |
| SHIP-04 | semver 11/11, MSRV green on release commit | CI evidence | existing `semver`/`msrv` jobs | ✅ jobs exist |

### Sampling Rate
- **Per task commit:** the specific new test/script touched by that task (e.g. `cargo test --test
  v0_9_config_boot` after D-07's task).
- **Per wave merge:** `cargo test --workspace` plus the relevant scripts
  (`check-release-consistency.sh`, `finalize-crate-changelogs.sh` in `--metadata-json` dry-run mode
  if available) for that wave's scope.
- **Phase gate:** Full suite green (`cargo test --workspace`), `cargo publish --workspace
  --dry-run` green, `semver`/`msrv` CI jobs green on the final commit, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/integration/v0_9_config_boot_test.rs` — covers SHIP-02 (config + route-mount half)
- [ ] `crates/paladin-web/tests/openapi_golden_v0_9.rs` — covers SHIP-02 (openapi golden diff)
- [ ] `tests/fixtures/config/v0.9.0-config.example.yml` (and/or `config.test.yml` per Open
      Question 1's resolution) — fixture backing the above
- [ ] `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` — fixture backing the openapi diff
- Framework install: none — `cargo test` is already the house framework, no new dependency

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No new surface | Existing `require_authentication` / auth middleware untouched by this phase (X-03: no behavioral change) |
| V3 Session Management | No new surface | N/A — no new session mechanism |
| V4 Access Control | No new surface | Existing admin/writer scopes on mutating routes (Phase 27) untouched |
| V5 Input Validation | No new surface | The two new tests (D-07/D-08) are read-only assertions against existing, already-validated config/response shapes |
| V6 Cryptography | No new surface | The webhook `X-Paladin-Signature` HMAC mechanism (Phase 27/28) is untouched; this phase adds no new signing path |

### Known Threat Patterns for this stack
No new attack surface is introduced by Phase 29 — it is documentation, CI configuration, and two
read-only integration tests over existing, already-security-reviewed code paths. The relevant
control is **regression prevention**, not new mitigation: the SHIP-02 boot test is itself a
security-relevant guardrail (it proves that a v0.9 deployment upgrading to v0.10 does not
accidentally expose the new `/v1/runs`/`/v1/threads`/`/v1/assistants`/`/v1/schedules` surface,
which per `.github/instructions/security.instructions.md`'s webhook SSRF section carries a
credential-shaped header and an outbound-HTTP capability once enabled). No STRIDE analysis beyond
"confirm this stays off by default" is warranted for a phase that adds no code path.

## Sources

### Primary (HIGH confidence — direct code/file reads and command execution this session)
- `src/bin/paladin-cli.rs`, `src/application/cli/commands/{graph,eval,maneuver,setup_check}.rs` —
  Q1, exact CLI surface
- `src/config/settings.rs`, `src/config/{web_server,run_queue,run_store,run_worker,run_stream,
  schedules,webhooks,assistants,waypoint_store,waypoint_retention,node_cache,engine}.rs`,
  `src/bin/paladin-server.rs` — Q2, config field/wiring enumeration
- `crates/paladin-web/src/{openapi,run_controller,thread_controller,app}.rs`,
  `src/infrastructure/web/run_api_wiring.rs` — Q2/Q3, route-mounting and status-code behavior
- `.cargo/semver-checks-allowlist.toml`, `MIGRATION.md` §9.2 (lines 159-268) — Q4, row-level match
- `release.toml`, root `Cargo.toml`, `crates/*/Cargo.toml`, `crates/doc-examples/Cargo.toml` — Q5
- `Makefile:552-565`, `.github/workflows/release.yml:13-14,584-653`, `scripts/publish-crates.sh` —
  Q6, plus **live execution** of `cargo publish --workspace --dry-run --allow-dirty` (full log
  captured, all 12 crates verified, zero errors)
- `scripts/finalize-crate-changelogs.sh`, `scripts/check-release-consistency.sh`, root
  `CHANGELOG.md` — Q7
- `.claude/gsd-core/bin/gsd-tools.cjs:2106`, `.claude/gsd-core/bin/lib/broken-windows.cjs:585-713`,
  `.planning/WINDOWS.md` frontmatter — Q8
- `crates/paladin-core/src/platform/container/directive.rs:40-89` — Q9
- `.project/v0.10.0/0[1-7]-*.md` (FR grep), `git diff v0.9.0 -- Cargo.toml` (`[[test]]` diff),
  `git diff --diff-filter=A --name-only v0.9.0 -- tests/integration/ crates/*/tests/` — Q10
- `MIGRATION.md` lines 5-10, 318, 630 (`grep -c "TBD"` = 4) — Q11
- **Live execution** of `mdbook --version`, `mdbook-linkcheck --version`,
  `cargo doc --workspace --no-deps` (72 warnings) and `--all-features` (84 warnings) — Q12
- **Live execution** of the actual `Settings::load_from_file("config.example.yml")` call via a
  scratch `#[test]` in `tests/unit/settings_config_test.rs` (reverted after capture) — Pitfall 1

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` Phase 26/28 carried-concern notes on rustdoc warning counts (corroborates,
  does not solely establish, Pitfall 7's live-measured figure)

### Tertiary (LOW confidence)
- None — every claim in this document was either executed or read directly from the file cited.

## Metadata

**Confidence breakdown:**
- Standard stack / tooling presence: HIGH — every tool version was measured by running it
- CLI/config/route behavior: HIGH — read directly from source, cross-checked against existing
  test names and MIGRATION.md's own documented response-code tables
- Dry-run publish viability: HIGH — executed to completion in this session
- FR/test-count sizing (Q10): MEDIUM — mechanically greppable, small risk of double-counting
  lettered FR-variants (see Assumption A2)

**Research date:** 2026-09-09
**Valid until:** the next commit that touches `src/config/settings.rs`, `MIGRATION.md` §9.2, or
`.cargo/semver-checks-allowlist.toml` on this branch — this is a fast-moving, code-verification
document tied to a specific HEAD, not a stable-library reference. Re-verify the "Y-row count" and
"grep -c TBD" figures if any commit lands on this branch before planning completes.
