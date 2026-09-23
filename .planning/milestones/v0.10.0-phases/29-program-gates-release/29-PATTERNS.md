# Phase 29: Program Gates & Release - Pattern Map

**Mapped:** 2026-09-09
**Files analyzed:** 17 (new/modified, per CONTEXT.md/RESEARCH.md)
**Analogs found:** 15 / 17

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `tests/integration/v0_9_config_boot_test.rs` | test (integration) | request-response (in-process HTTP boot) | `tests/paladin_server_smoke.rs` (`server_serves_openapi_spec_and_docs`) + `tests/unit/settings_config_test.rs` (`test_load_from_file_regression`) | role-match, composite |
| `tests/fixtures/config/v0.9.0-config.example.yml` | config fixture | file-I/O | `config.test.yml` (root) | role-match |
| `crates/paladin-web/tests/openapi_golden_v0_9.rs` | test (integration) | transform (JSON diff) | `crates/paladin-web/src/openapi.rs::openapi_matches_committed_baseline` (in-crate test) | exact (same seam, different scope) |
| `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` | fixture | file-I/O | `crates/paladin-web/openapi.json` (committed baseline) | exact |
| `.github/workflows/ci.yml` `semver` job — TBD gate step | CI config step | batch (grep gate) | existing "Verify allowlist is set-equal…" step (same job, lines ~359-409) | exact (sibling step, same job) |
| `.github/workflows/ci.yml` `semver` job — row-level allowlist step | CI config step | batch (set comparison) | same existing allowlist step (extend awk field extraction) | exact |
| `MIGRATION.md` §9.5/§9.6/§9.8 + header | doc | transform (prose edit) | existing §9.4 entries (same file, same prose conventions) | exact |
| `docs/src/api-reference/upgrading.md` (new) | doc | request-response (static page) | `docs/src/api-reference/migration-guide.md` (structure/tone) | role-match |
| `docs/src/SUMMARY.md` entry | doc config | CRUD (list insert) | existing "Migration Guide" line (68-72) | exact |
| `docs/src/appendix/release-checklist.md` dry-run section | doc | transform (prose edit) | same file's existing sections | exact |
| `Makefile` `publish-dry-run` target | config/utility | batch (shell loop → single command) | `Makefile` `release` target's `UPDATE_OPENAPI=1 cargo test …` pattern (adjacent target, same file) | role-match |
| `.project/v0.10.0/09-program-acceptance-audit.md` (new) | doc (audit) | transform (aggregation) | `.project/v0.10.0/08-traceability-matrix.md` (ten-step protocol structure) | exact |
| `29-ACCEPTANCE-AUDIT.md` (pointer) | doc | request-response (pointer file) | `27-CI-EVIDENCE.md` / `28-CI-EVIDENCE.md` (phase-dir evidence-pointer convention) | role-match |
| `29-CI-EVIDENCE.md` | doc (evidence) | batch (evidence table) | `28-CI-EVIDENCE.md` / `27-CI-EVIDENCE.md` | exact |
| `.planning/WINDOWS.md` row transitions | data/ledger | event-driven (CLI-mutated) | `gsd-tools.cjs windows waive/fixed` handler itself (no new code — CLI invocation only) | exact (tool, not file to write) |
| Twelve `Cargo.toml` version bumps + `Cargo.lock` + `openapi.json` regen | config | batch (release mechanics) | `release.toml` + `Makefile` `release` target (lines ~576-620), v0.9.0 precedent (PR #50) | exact |
| Twelve `CHANGELOG.md` `[0.10.0]` sections | doc | batch (changelog stamping) | `scripts/finalize-crate-changelogs.sh` output shape + root `CHANGELOG.md`'s existing `[Unreleased]` body | exact |

## Pattern Assignments

### `tests/integration/v0_9_config_boot_test.rs` (integration test, request-response)

**Analogs:** `tests/paladin_server_smoke.rs` (in-process boot + HTTP drive) and
`tests/unit/settings_config_test.rs` (file-load + env-isolation).

**Imports/module doc pattern** (`tests/paladin_server_smoke.rs:1-25`):
```rust
//! Boot smoke test for the HTTP service host (Milestone 12, Epic 2).
//!
//! Builds a hermetic agent backed by [`MockLlmAdapter`] (no network / API keys), serves
//! the agent router on an ephemeral port via `axum::serve`, and drives it over real HTTP
//! with `reqwest` — mirroring what the `paladin-server` binary does.
#![cfg(feature = "web-server")]

use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, HttpLayersConfig, Principal, agent_router,
    with_http_layers,
};
```
Reuse: same module-doc convention (state the analogy to the real binary explicitly), same
`#![cfg(feature = "web-server")]` gate if the boot test needs the web-server feature, same
in-process `axum::serve` + ephemeral-port + `reqwest::Client` pattern, same graceful-shutdown
oneshot-channel teardown at the end of the test (`tests/paladin_server_smoke.rs:283-292`).

**Server-boot + route-probe core pattern** (`tests/paladin_server_smoke.rs:284-320`):
```rust
#[tokio::test]
async fn server_serves_openapi_spec_and_docs() {
    use paladin::infrastructure::web::openapi::{build_openapi, docs_router};
    let state = state_with_mock_agent("researcher").await;
    let spec = build_openapi(state.clone());
    let app = with_http_layers(agent_router(state).merge(docs_router(spec)), &HttpLayersConfig::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).with_graceful_shutdown(async move { let _ = shutdown_rx.await; }).await.expect("server runs");
    });
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/openapi.json")).send().await.expect("GET /openapi.json");
    assert_eq!(resp.status().as_u16(), 200);
    // ...
    shutdown_tx.send(()).expect("send shutdown");
    server.await.expect("server task joins after shutdown");
}
```
**Critical correction (RESEARCH.md Pitfall 2):** route assertions against the nine new-in-v0.10
platform route families (`/v1/runs`, `/v1/threads`, `/v1/assistants`, `/v1/schedules`) must expect
`StatusCode::NOT_IMPLEMENTED` (501), not 404 — see `src/infrastructure/web/run_api_wiring.rs:10`
(module doc: "answers `501 not_implemented`") and its test
`defaults_wire_nothing_and_answer_501` (line 644), plus
`crates/paladin-web/src/thread_controller.rs::thread_router_returns_501_over_http_when_unwired_and_auth_disabled`.
404 is reserved only for the genuinely unregistered `dev-ui` router.

**Env-isolation pattern** (`tests/unit/settings_config_test.rs:16-40`):
```rust
#[test]
#[serial]
fn test_settings_with_file_storage_config() {
    let minio_vars = ["APP_MINIO_ENDPOINT", /* ... */];
    let saved: Vec<(&str, Option<String>)> =
        minio_vars.iter().map(|k| (*k, std::env::var(k).ok())).collect();
    unsafe { for k in &minio_vars { /* remove_var */ } }
    // ... restore saved values at the end
}
```
Reuse this scoped-env-var save/clear/restore + `#[serial]` shape for isolating the boot test from
ambient `APP_*` variables (D-07's stated motivation).

**File-load regression pattern** (`tests/unit/settings_config_test.rs:418-440`):
```rust
#[test]
#[serial]
fn test_load_from_file_regression() {
    let settings = Settings::load_from_file("config.test.yml").expect("config.test.yml should load");
    assert_eq!(settings.server.host, "127.0.0.1");
    let garrison = settings.get_garrison_config();
    assert_eq!(garrison.garrison_type, "in_memory");
    // ...
}
```
**Critical correction (RESEARCH.md Pitfall 1):** `config.example.yml` does NOT load at either
tag (`missing configuration field "llm.ollama.api_key"`) — do not freeze it as the fixture the
test actually calls `load_from_file` against. Follow the precedent above and use a frozen
`config.test.yml`-shaped fixture (Open Question 1, option (b)) as the file this test loads;
`config.example.yml`'s v0.9.0 copy can still be kept for provenance/documentation only.
**Critical correction (RESEARCH.md Pitfall 3):** none of `run_queue`/`run_store`/`run_worker`/
`run_stream`/`schedules`/`webhooks`/`assistants`/`waypoint_store`/`engine` are `Settings` fields —
do not write `settings.run_store` etc.; assert inertness against each struct's own `X::default()`
plus a scoped-env-var check that `apply_env_overrides()` is a no-op with no `APP_*` set, following
`src/bin/paladin-server.rs:103-158`'s own construction pattern:
```rust
let mut run_store_config = RunStoreConfig::default();
run_store_config.apply_env_overrides();
run_store_config.validate().map_err(...)?;
```

---

### `crates/paladin-web/tests/openapi_golden_v0_9.rs` (integration test, transform)

**Analog:** `crates/paladin-web/src/openapi.rs` (`openapi_matches_committed_baseline`, lines ~264-281).

**Generator seam** (`crates/paladin-web/src/openapi.rs:84-86`):
```rust
pub fn openapi_spec() -> OpenApi {
    build_openapi(AgentApiState::new(Arc::new(AgentRegistry::new())))
}
```
Reachable as `paladin_web::openapi::openapi_spec()` from the new external test file with no
feature gate.

**Drift-guard / golden-diff core pattern** (`crates/paladin-web/src/openapi.rs:258-281`):
```rust
fn baseline_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json")
}

#[test]
fn openapi_matches_committed_baseline() {
    let generated = openapi_spec().to_pretty_json().expect("serialize spec");
    let path = baseline_path();
    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::write(&path, format!("{generated}\n")).expect("write baseline");
        return;
    }
    let baseline = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(generated.trim(), baseline.trim(), "OpenAPI spec drifted from {}. ...", path.display());
}
```
Reuse: same `baseline_path()`-via-`CARGO_MANIFEST_DIR` convention (pointed at the new
`tests/fixtures/openapi-v0.9.0.json` instead), same `serde_json::Value`-equality-with-readable-diff
intent, but scoped per D-08 to the six v0.9 `paths` + their `$ref`-closure schemas +
`components.securitySchemes`, with `info.version` excluded from comparison (the only sanctioned
normalisation).

---

### `.github/workflows/ci.yml` `semver` job — new TBD gate + row-level allowlist steps (CI config, batch)

**Analog:** the existing allowlist set-equality step, same job (`.github/workflows/ci.yml`
~lines 359-409).

**Shell/awk style to copy** (extraction + set comparison, shellcheck-clean, tolerant of empty sets):
```yaml
      - name: Verify allowlist is set-equal to the MIGRATION.md §9.2 register
        run: |
          set -euo pipefail
          test -f .cargo/semver-checks-allowlist.toml
          test -f MIGRATION.md
          awk '/^## 9\.2 /{flag=1; next} /^## 9\.3 /{flag=0} flag' MIGRATION.md \
            | { grep -E '^\|' || test $? -eq 1; } \
            | awk -F'|' '{
                crate=$2; gsub(/^[ \t`]+|[ \t`]+$/, "", crate);
                deliberate=$6; gsub(/^[ \t]+|[ \t]+$/, "", deliberate);
                if (deliberate ~ /^Y/) print crate
              }' \
            | sort -u > /tmp/migration-deliberate-crates.txt
          { grep -E '^\s*crate\s*=' .cargo/semver-checks-allowlist.toml || test $? -eq 1; } \
            | awk -F'"' '{print $2}' \
            | sort -u > /tmp/allowlist-crates.txt
          if ! diff -u /tmp/migration-deliberate-crates.txt /tmp/allowlist-crates.txt; then
            echo "::error::Allowlist and MIGRATION.md §9.2 deliberate-breaking rows are not set-equal (see diff above)."
            exit 1
          fi
```
**D-01 (TBD gate):** a sibling step, same style — `test -f MIGRATION.md`, then
`grep -c 'TBD' MIGRATION.md` must be `0` (RESEARCH.md Q11: actual current count is **4**, not 3 —
both header-note occurrences at lines 7-8 plus §9.5/§9.8 — the rewritten header must not
reintroduce the literal token "TBD"; use "placeholder" or paraphrase).

**D-04 (row-level tightening):** extend the same awk pipeline to emit `crate|type` pairs instead
of crate-only — pull each `[[entry]]`'s `migration_row` field (already present, e.g.
`"paladin-ai-core | StopReason"`) from `.cargo/semver-checks-allowlist.toml` and the §9.2 row's
first two cells (type cell reduced to its first backtick-quoted identifier), then `sort -u` both
sides before `diff -u`, exactly as the existing crate-only step already does. **Critical
correction (RESEARCH.md Pitfall 5/6):** §9.2 has 26 data rows (not 28) and 10 `Y`-marked rows
mapping to only 9 distinct pairs (`paladin-ai | Settings` appears twice, deliberately, per
MIGRATION.md's own line-191 text) — the check must be a **set** comparison, never a row-count or
entry-count comparison.

**Do not touch:** the five literal `--baseline-version 0.9.0` occurrences in this same job
(lines 276, 304, 334, 339, 357) — pinned to the last-published version, not a bump target
(RESEARCH.md Pitfall 4).

---

### `MIGRATION.md` §9.5/§9.6/§9.8 + header (doc, transform)

**Analog:** the file's own existing §9.4 entries (dense, precedent-citing prose with concrete
file paths, test names, and "Landed Phase N" provenance) — copy the same density/format when
closing §9.5/§9.6/§9.8.

**§9.8 checklist commands — use only real `paladin-cli` subcommands** (verified against
`src/bin/paladin-cli.rs` and `src/application/cli/commands/graph.rs:42`):
```
paladin-cli setup-check [--verbose]        # env/toolchain/provider/service health check
paladin-cli maneuver validate <flow-args>  # validates flow expression + Paladin config
paladin-cli graph export --format mermaid|dot (<FILE>|--assistant <id>) [--out <path>]
paladin-cli eval run <glob> [--repeat N] [--bless] [--live] [--registries <name>]
```
There is **no** `paladin-cli health` and **no** `graph validate` subcommand — D-02's checklist
must not invent one; use `setup-check` / `maneuver validate` / `eval run` instead.

**Header rewrite constraint:** must describe the now-closed TBD convention without using the
literal word "TBD" anywhere in the file (see the CI gate above) — current header lines 7-8 read:
```
> ... Every `TBD` below carries the requirement or phase
> that owns closing it; `SHIP-01` (Phase 29) is responsible for clearing every remaining `TBD`
> before the v0.10.0 release, per overview §9's living-document contract.
```
Rewrite to past-tense/closed-state prose (e.g. "placeholder", "marker") retaining the same
citation style.

---

### `docs/src/api-reference/upgrading.md` (new doc page)

**Analog:** `docs/src/api-reference/migration-guide.md` (structure/tone) — a `#` title, a short
orientation paragraph, then dated `##` sections:
```markdown
# Migration Guide

This guide covers all breaking changes since v0.1.0 up to the current **v0.5.0** release.

## Table of Contents
...
## Migrating to v0.5.0 (from v0.4.x)

**No user-facing breaking changes.** v0.5.0 is the documentation-overhaul release ...
```
Reuse: same `#`-title + short-orientation-paragraph opening; register the new page in
`docs/src/SUMMARY.md` directly above the existing line:
```markdown
- [Migration Guide](api-reference/migration-guide.md)
```
(insert `- [Upgrading](api-reference/upgrading.md)` immediately above it, same list style,
`docs/src/SUMMARY.md:68-72`). Per D-03, do NOT use `{{#include ../../../MIGRATION.md}}` —
`docs/book.toml`'s `[output.linkcheck] warning-policy = "error"` will fail on MIGRATION.md's
relative repo-root links; write the page's content by hand instead, linking `MIGRATION.md` by its
repository URL.

---

### `Makefile` `publish-dry-run` target (utility, batch)

**Analog (what NOT to keep — the current broken target)** (`Makefile:552-565`):
```makefile
publish-dry-run: release-check ## Run dependency-first `cargo publish --dry-run` for all crates
	@$(CARGO) publish --dry-run -p paladin-core || true
	...
	@echo "... See docs/RELEASE_CHECKLIST.md for interpretation ..."
```
Broken: `paladin-core` is a directory name, not the package name (`paladin-ai-core`); every line
swallows failures with `|| true`; `paladin-herald` is missing; the closing message points at a
nonexistent `docs/RELEASE_CHECKLIST.md`.

**Analog to model the fix on** (`Makefile` `release` target's regeneration step, lines ~611-620):
```makefile
	@$(CARGO) release version "$(VERSION)" --execute --no-confirm --workspace
	@echo "$(CYAN)Regenerating OpenAPI baseline for $(VERSION)...$(NC)"
	@UPDATE_OPENAPI=1 $(CARGO) test -p paladin-web openapi_matches_committed_baseline --quiet
```
D-20's fix: rewrite `publish-dry-run` to a single command, no `|| true`, message pointing at the
real path:
```makefile
publish-dry-run: release-check
	@$(CARGO) publish --workspace --dry-run
	@echo "$(YELLOW)See docs/src/appendix/release-checklist.md for interpretation and publish-order gating.$(NC)"
```
(`cargo publish --workspace --dry-run` verified working end-to-end in RESEARCH.md Q6, all 12
publishable crates, zero errors.)

---

### `.project/v0.10.0/09-program-acceptance-audit.md` (new audit doc)

**Analog:** `.project/v0.10.0/08-traceability-matrix.md` lines 98-108 (the ten-step verification
protocol this document executes) — mirror its per-step `##` heading + verdict-line structure.
Pointer file `29-ACCEPTANCE-AUDIT.md` follows the `27-CI-EVIDENCE.md`/`28-CI-EVIDENCE.md`
phase-dir evidence-pointer convention (link + one verdict line).

---

### `29-CI-EVIDENCE.md` (evidence doc)

**Analog:** `28-CI-EVIDENCE.md` / `27-CI-EVIDENCE.md` — mirror their exact table shape (local
sweep commands table, then CI-run-ID table). Explicitly record the pre-existing 72-warning
`cargo doc --workspace --no-deps` red `lint`-job step (RESEARCH.md Pitfall 7) rather than
omitting it — following the same "record, don't silently pass over" convention 28-17-SUMMARY used
for its own carried-forward gaps.

---

### Twelve `Cargo.toml` bumps / `Cargo.lock` / `openapi.json` regen / twelve `CHANGELOG.md` sections

**Analog:** `release.toml` + `Makefile` `release` target (lines 576-620) + v0.9.0's own precedent
(PR #50 bump, tag on merge `0b5d4106`).

**Bump command** (`Makefile:611`):
```makefile
@$(CARGO) release version "$(VERSION)" --execute --no-confirm --workspace
```

**Changelog stamping** (`scripts/finalize-crate-changelogs.sh` header, verified idempotent,
enumerates publishable packages from `cargo metadata --no-deps`, never a hardcoded list):
```
make finalize-crate-changelogs VERSION=0.10.0
```
Disposition order per file: (1) already has a `## [0.10.0]` section → left untouched; (2) has
`## [Unreleased]` anchor → new dated section inserted immediately after it, anchor preserved;
(3) neither → left untouched, recorded as a named failure. The root `CHANGELOG.md`'s
`[Unreleased]` body is then hand-curated (grouped Added/Changed/Fixed, M-B-01..04 called out
first, D-16 bench deviation under "Known limitations") per D-19 — the script only inserts the
heading, the executor moves the entries.

## Shared Patterns

### Register + gate, both directions, no wildcards
**Source:** `.github/workflows/ci.yml` allowlist set-equality step (lines 359-409).
**Apply to:** D-01's TBD gate and D-04's row-level tightening — both should be `sort -u`-based
set comparisons over extracted fields, never row/entry counts, never one-directional checks.

### Evidence-based close-out with explicit gaps recorded, not silently passed over
**Source:** `28-17-SUMMARY.md` lines 205-218 ("Open acceptance gaps a Phase 29 SHIP reviewer
should see"), `27-CI-EVIDENCE.md` / `28-CI-EVIDENCE.md`.
**Apply to:** `29-CI-EVIDENCE.md` and `.project/v0.10.0/09-program-acceptance-audit.md` — record
the 72-warning `cargo doc` red step, the D-16 bench-overhead deviation, and any WINDOWS.md finding
explicitly rather than omitting them because they are out of this phase's fix-scope.

### Frozen fixtures with provenance header
**Source:** `.project/current-exports.txt` (api-surface snapshot), `crates/paladin-web/openapi.json`
(committed baseline with `UPDATE_OPENAPI=1` regeneration convention).
**Apply to:** `tests/fixtures/config/v0.9.0-config.example.yml` and
`crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` — each needs a header (or sibling
`README.md`, per Claude's Discretion) recording the source tag, blob SHA, and the exact `git show`
command that produced it.

### Config struct inertness proven two different ways
**Source:** `src/bin/paladin-server.rs:96-158` (`EngineConfig` composition-root comment: "not
`Settings`, X-10 avoidance") vs. `Settings`'s own `#[serde(default)]` YAML round-trip
(`tests/unit/settings_config_test.rs`).
**Apply to:** `v0_9_config_boot_test.rs` — the nine platform structs must be asserted inert via
`X::default()` + scoped-env-var no-op, never via a `Settings` field access (Pitfall 3).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `.planning/WINDOWS.md` row transitions | ledger mutation | event-driven | No file to author directly — mutated exclusively through `gsd-tools.cjs windows waive <id> "<reason>"` / `windows fixed <id>` (`bin/lib/broken-windows.cjs:585-660`); there is no source-code analog to copy from, only the CLI invocation pattern already documented in RESEARCH.md Q8. |
| `.project/v0.10.0/00-program-overview.md` §4 errata line (D-25) | doc | transform | One-line factual correction (`NextStep::Halt` does not exist — verified against `crates/paladin-core/src/platform/container/directive.rs:40-89`); no structural analog needed, just the corrected enum text. |

## Metadata

**Analog search scope:** `crates/paladin-web/src/openapi.rs`, `tests/paladin_server_smoke.rs`,
`tests/unit/settings_config_test.rs`, `.github/workflows/ci.yml` (semver job), `Makefile`,
`scripts/finalize-crate-changelogs.sh`, `MIGRATION.md`, `docs/src/api-reference/migration-guide.md`,
`docs/src/SUMMARY.md`, `src/infrastructure/web/run_api_wiring.rs`, `.project/v0.10.0/08-traceability-matrix.md`.
**Files scanned:** ~17 target files against ~11 analog sources, all read directly (no re-reads of
overlapping ranges).
**Pattern extraction date:** 2026-09-09
