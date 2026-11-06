# Phase 55: Correctness Fixes, Test Hygiene & Lint Bar - Pattern Map

**Mapped:** 2026-11-01
**Files analyzed:** 27 (new and modified)
**Analogs found:** 25 / 27 (two partial)

Line numbers below were read on 2026-11-01 at the current HEAD. Locate by content, never by line (D-00e).
The RESEARCH.md `## Code Inventory` already holds measured ranges for the modify-in-place files; this file
adds the analog excerpts to copy from.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `scripts/check-no-stub-panics.sh` | gate script | batch / file-I/O scan | `scripts/check-version-strings.sh` (+ `check-crate-names.sh` for publishable discovery) | exact |
| `tests/scripts/check-no-stub-panics_test.sh` | shell test harness | batch | `tests/scripts/check-version-strings_test.sh` | exact |
| `tests/scripts/fixtures/check-no-stub-panics/` (optional) | fixture | file-I/O | `tests/scripts/fixtures/check-publish-order/` (but house harness builds under `mktemp`) | role-match |
| `Makefile` (`check-gates` prereq, new target) + `.github/workflows/ci.yml` step | config | batch | `check-version-strings` target (Makefile 224-226, 268) and its CI step (`ci.yml` ~181) | exact |
| `unpriced_reachable_models` (+ render fn + thin logger) in `src/infrastructure/web/run_api_wiring.rs` | service helper | transform (pure) + log | `scan_report` / `report_unattributed_schedules`, `assert_treasurer_enforced` | exact |
| `pub(crate) build_run_api_with_port` seam in same file | wiring | request-response (composition root) | `build_run_api` -> `build_run_api_with_cadence` delegation (613, 787) | exact |
| Scripted `PaladinPort` test double (`run_api_wiring.rs` tests) | test double | request-response | `AlwaysSucceedsPaladinPort` (`worker_tests.rs:1103`), `UnusedPaladinPort` (`cancel_tests.rs:50`) | exact |
| `LocalRunTokens::is_registered` in `run/cancel.rs` | service method | CRUD (read) | sibling `remove` / `cancel_if_local` | exact |
| `local_cancel_signals_token` poll in `run/cancel_tests.rs` | test | event-driven | same test (replace 50 ms sleep) | exact |
| `HaltReason::ModelUnpriced { model }` in `crates/paladin-core/src/platform/container/allowance.rs` | model (enum variant) | transform | the `AllowanceExhausted` / `LedgerUnavailable` arms in the same enum | exact |
| `src/application/services/run/worker.rs` (Unpriced arm -> Halt) + `worker_tests.rs` flip | service + test | request-response | existing `Halt(reason)` arm; sibling test `agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm` | exact |
| `crates/paladin-storage/src/sqlite_content_repository.rs`, `mysql_content_repository.rs` (20 bodies + table tests) | repository adapter | CRUD (refusal) | `RepositoryError::QueryError` use elsewhere in same files; existing in-module tests | role-match |
| `crates/paladin-content/src/services/content_filtering_service.rs` (delete) + two commented `mod` lines in `services/mod.rs` | service (dead) | n/a | n/a (removal) | n/a |
| `crates/paladin-llm/src/registry/seeded.rs` (`claude_opus_4_8`) + `registry/mod.rs` pins | config / test | transform | sibling rows (`claude-fable-5` `true`, `claude-haiku-5-5` `false`) | exact |
| `crates/paladin-llm/src/reasoning.rs` + `config/layers.rs` (one `pub(crate) const`) | utility | transform | the two existing consts | exact |
| `crates/paladin-llm` cfg-gating (`cadence.rs`, `fallback.rs`, `conformance.rs`, `provider_factory.rs`) | test helpers | n/a | `thinking_effort_wire_value`'s existing `#[cfg(any(feature...))]` gate | exact |
| `crates/paladin-battalion/src/commander.rs` tests ~3805-3960 | test | n/a | relocated tests in `tests/integration/commander_error_paths_test.rs` | role-match |
| `crates/paladin-core/src/base/**` (rustdoc on retained items) | model | n/a | rustdoc style of sibling `Entity<T>` wrapper | role-match |
| `src/application/services/paladin/planning_service.rs`, `crates/paladin-content/src/adapters/input/file_content_fetcher.rs` | service / adapter | request-response | `VersioningError` / `RepositoryError` fence idiom above | role-match |
| 5 `lib.rs` (`#![allow(rustdoc::...)]`) + rustdoc link fixes | config | n/a | Phase 36 doc-link fix pattern (code spans / full paths) | exact |
| `.planning/decisions/0033-cargo-doc-warning-bar.md` | doc | n/a | its own `**Amendment, 2026-09-18 ...**` blocks (l.29, 185) | exact |
| `55-CORRECTNESS-EVIDENCE.md` | evidence | n/a | `54-first-run-usability/54-FIRST-EVIDENCE.md` | exact |
| `55-BASE-LAYER-REVIEW.md` | evidence | n/a | `53-VERSION-DECISION.md` (decision-record shape) | role-match |
| `crates/*/CHANGELOG.md` `## [Unreleased]` anchors | doc | n/a | `crates/paladin-memory/CHANGELOG.md` | exact |
| `docs/src/appendix/doc-coverage-report.md`, `MIGRATION.md` 11.1, `.planning/WINDOWS.md` rows 67/73/56 | doc | n/a | existing rows (M-B-29 is last; next M-B-30) | exact |

## Pattern Assignments

### `scripts/check-no-stub-panics.sh` (gate script, batch)

**Analog:** `scripts/check-version-strings.sh`. Prototype scanner is already in RESEARCH Code Examples 1; wrap it in this shell.

**Header and seam** (check-version-strings.sh 68-80). Use a test-only root override env var (name is discretionary, e.g. `STUB_PANICS_ROOT`), no argument, no write mode:
```bash
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${VERSION_STRINGS_ROOT:-${WORKSPACE_ROOT}}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
fi

REPORT=$(python3 - "${ROOT}" <<'PY'
import glob, json, os, re, sys, tomllib
root = sys.argv[1]
...
PY
)
```

**Publishable discovery** (copy the tomllib manifest loader, version-strings 111-139):
```python
def load_manifest(path):
    try:
        with open(path, "rb") as fh:
            return tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError):
        return None
# root Cargo.toml + sorted(glob.glob(os.path.join(root, "crates", "*", "Cargo.toml")))
# skip a package only when manifest["package"]["publish"] is False (never by directory name);
# a crates/<name>/ dir with no Cargo.toml is skipped silently (check-crate-names.sh header).
```

**Named failure + status-line protocol** (version-strings 251-285). Report every finding before exit; first line is the status token; "scanned zero files" is a named failure (`ZERO_FILES`); findings printed as `path:line: ...` sorted by path then line:
```python
if named or findings:
    print(named[0][0] if named else "MISMATCH")
    for _, detail in named:
        print(detail)
    for path, lineno, found in findings:
        print(f"{path}:{lineno}: found '{found}', want '{want}'")
    sys.exit(0)
print("OK")
print(f"OK: {len(texts)} file(s) scanned, ...")
```

**Bash tail** (version-strings 289-306): `STATUS_LINE=$(head -n1 <<<"${REPORT}")`, `DETAIL=$(tail -n +2 ...)`, `exit 0` on `OK`, else print the finding list plus "If this failure is unexpected" guidance and `exit 1`. No exemption list (D-01 says zero exemptions), so omit `STALE_EXEMPTION`/`EXEMPT`.

**Scanner rules to keep (RESEARCH Code Examples 1 and Pitfall 1):** resolve `#[cfg(test)] mod x;` to `x.rs` or `x/mod.rs` transitively; exempt only `test` and `all(test, ...)`; `cfg(not(test))` is NOT exempt; blank comments (nested block) and strings (raw-string aware). Real-tree expectation before fixes: 624 files, 19 test-only, 26 findings; after: 0.

Make the file executable (CI has `chmod +x scripts/*.sh`, but commit mode 755). Run `shellcheck` on it.

---

### `tests/scripts/check-no-stub-panics_test.sh` (harness)

**Analog:** `tests/scripts/check-version-strings_test.sh` (read in full).

**Skeleton to copy verbatim** (lines 22-49): `set -uo pipefail` (no `-e`), `GUARD=...`, existence check, `SCRATCH="$(mktemp -d ...)"` with `trap cleanup EXIT`, `FAILED`/`ASSERTIONS` counters, `real_sums` snapshot of the real scanned files (here `crates/*/src`).

**Helper set to copy** (lines 51-160): `make_tree DIR` (a minimal passing fixture tree: root `Cargo.toml`, one publishable `crates/x/Cargo.toml`, `src/lib.rs`), `fresh NAME`, `plant FILE TEXT` (sets `$PLANT_LINE`), `line_of`, `tree_sums`, `run_guard` (swap the env var name), `assert_fire ROOT DESC NEEDLE...`, `assert_silent ROOT DESC [NEEDLE...]`.
```bash
run_guard() {
    LAST_OUTPUT="$(VERSION_STRINGS_ROOT="$1" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}
```

**Case order to copy:** failing cases first (`assert_fire` with `file:line` needle via `PLANT_LINE`), then named failure (zero-files tree, copy lines 206-210), then ordering/determinism and no-mutation cases (lines 292-333), then passing cases, then the real tree, then real-tree checksum no-mutation (lines 382-411), then the summary footer (413-420).

**Fixtures this guard needs (RESEARCH Code Examples 1 and Pitfall 1):** fail: production `todo!`, production `unimplemented!`, `#[cfg(not(test))]` `todo!`, file reached by `pub mod sub;`. Pass: `///` doc example, string literal, raw string, `/* */`, `#[cfg(test)] mod tests {}`, `#[cfg(all(test, feature = "mock"))] mod x {}`, transitive out-of-line `#[cfg(test)] mod ext;` containing `mod deeper;`, a `publish = false` crate. Build them under `mktemp` with `plant`/`printf`, as the analog does (a checked-in `fixtures/` dir is optional; the only existing one is `check-publish-order`).

Auto-discovered by `make test-shell-guards` (Makefile ~276); run `shellcheck`.

---

### Makefile and `ci.yml` wiring

**Analog:** `check-version-strings` (Makefile 224-226) and `check-gates` (268).
```make
.PHONY: check-version-strings
check-version-strings: ## Verify every copyable version string ... (FIRST-03)
	@./scripts/check-version-strings.sh
...
check-gates: check-changelogs check-crate-names ... check-dependency-allowlist check-version-strings ## Run all offline release-gate guards
```
Add `check-no-stub-panics` as a `.PHONY` target with a `##` help string and append to the `check-gates` prerequisite list. CI: add a step in the gate-scripts job after `Check user-facing version strings (FIRST-03)` (`ci.yml` ~181), running the script directly (CI does not call `make check-gates`). `actionlint` is absent locally: record an `unrun-verify` row (WINDOWS rows 83/84 precedent).

---

### `unpriced_reachable_models` and boot warning (`src/infrastructure/web/run_api_wiring.rs`)

**Analog 1 (pure function with a small refusal table):** `assert_treasurer_enforced` (644-654).
```rust
fn assert_treasurer_enforced(
    treasurer_configured: bool,
    has_engine_factory: bool,
) -> Result<(), String> {
    if treasurer_configured && !has_engine_factory {
        Err(TREASURER_UNENFORCED_MESSAGE.to_string())
    } else {
        Ok(())
    }
}
```
Doc style: "Pure, so the whole table is tested without building a server."

**Analog 2 (pure report fn + thin logger, no logger capture in tests):** `scan_report` (721) and `report_unattributed_schedules` (760).
```rust
fn scan_report(outcome: &ScanOutcome) -> Vec<(log::Level, String)> {
    let mut lines = Vec::new();
    if !outcome.unattributed.is_empty() {
        let mut ids: Vec<&RunScheduleId> = outcome.unattributed.iter().collect();
        ids.sort();
        ...
        lines.push((log::Level::Warn, format!("...")));
    }
    lines
}

fn report_unattributed_schedules(outcome: &ScanOutcome) {
    for (level, line) in scan_report(outcome) {
        match level {
            log::Level::Error => log::error!("{line}"),
            _ => log::warn!("{line}"),
        }
    }
}
```
**Apply:** `unpriced_reachable_models(models: impl Iterator<Item=&str>, table: &PriceTable) -> Vec<String>` (sorted, deduplicated, uses `PriceTable::row(&str)`, the same call as `treasurer/derive.rs:123`); a pure render fn returning one line per model; a thin logger using `log::warn!(target: PRICING_LOG_TARGET, ...)` (`paladin_llm::pricing::PRICING_LOG_TARGET = "paladin::pricing"`). Lines name the model, never a key value; say workflow-assistant graph-node models are not enumerated. Call site: inside the `else` branch where the Treasurer is built, right after `treasurer_config.price_table()` (~974) and before `Treasurer::new(..).with_pricing(Arc::new(price_table))` moves it. Source of models: `AgentRegistry::list()` (`crates/paladin-web/src/agent_registry.rs:190`) reading `data.model`. Tests: extend near the `assert_treasurer_enforced` tests (~1696-1790); assert on the pure output.

---

### `build_run_api_with_port` seam (`run_api_wiring.rs`)

**Analog:** the existing `build_run_api` -> `build_run_api_with_cadence` delegation (613 and 787). The single network-reaching call is at ~890:
```rust
paladin_port_from_settings_with_cadence(settings, treasury_ledger.clone(), cadence)
```
**Apply:** move the body into `pub(crate) async fn build_run_api_with_port(.., paladin_port_override: Option<Arc<dyn PaladinPort>>)`; there do `match override { Some(p) => p, None => paladin_port_from_settings_with_cadence(..)? }`; `build_run_api_with_cadence` becomes a one-line delegation passing `None`. `pub(crate)` keeps `make api-surface` unchanged (RESEARCH Open Question 2 flags that D-08 said "byte-for-byte" and this is a delegation). Keep the existing error mapping of the `?` as is.

### Scripted `PaladinPort` double (`run_api_wiring.rs` tests)

**Analog:** `AlwaysSucceedsPaladinPort` (`src/application/services/run/worker_tests.rs:1103-1133`):
```rust
struct AlwaysSucceedsPaladinPort;

#[async_trait]
impl PaladinPort for AlwaysSucceedsPaladinPort {
    async fn execute(&self, _paladin: &Paladin, _input: &str)
        -> Result<PaladinResult, PaladinError> {
        Ok(PaladinResult {
            output: "agent completed".to_string(),
            usage: TokenUsage::new(11, 7),
            ..Default::default()
        })
    }
    async fn execute_stream(&self, _paladin: &Paladin, _input: &str)
        -> Result<PaladinStream, PaladinError> {
        unreachable!("this test never calls execute_stream")
    }
    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> { Ok(()) }
}
```
`UnusedPaladinPort` (`cancel_tests.rs:50-70`) is the `unreachable!` variant, already duplicated in `run_api_wiring.rs` ~1727. For the two `build_run_api_persists_*` tests: inject the success double via the seam, drop the `OPENAI_API_KEY` set_var and `#[serial_test::serial(paladin_run_api_wiring_openai_api_key)]` on those two only (the other 16 sites keep the serial group). The existing `FailingExecutor` at ~2954 inside `persisted_trace_records` is never consulted on the agent-kind path; remove its use there. Verify with `env -u HTTPS_PROXY -u https_proxy cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists`.

---

### `LocalRunTokens::is_registered` (`src/application/services/run/cancel.rs`)

**Analog:** sibling methods (cancel.rs ~188-215):
```rust
pub async fn remove(&self, run_id: &RunId) {
    self.tokens.write().await.remove(run_id);
}

pub async fn cancel_if_local(&self, run_id: &RunId) -> bool {
    let tokens = self.tokens.read().await;
    match tokens.get(run_id) {
        Some(token) => { token.cancel(); true }
        None => false,
    }
}
```
**Apply:** same doc-comment style, read lock, `#[cfg(test)] pub(crate)` (plain `pub(crate)` is `dead_code` outside tests):
```rust
#[cfg(test)]
pub(crate) async fn is_registered(&self, run_id: &RunId) -> bool {
    self.tokens.read().await.contains_key(run_id)
}
```
In `cancel_tests.rs` `local_cancel_signals_token` replace the 50 ms sleep with `while !tokens.is_registered(&run_id).await { tokio::time::sleep(Duration::from_millis(5)).await }` inside the existing 30 s outer `timeout`. `pool.local_tokens()` returns a clone sharing the map. Flake proof: 20 full-suite runs, not isolated runs.

---

### `HaltReason::ModelUnpriced { model }` (`crates/paladin-core/src/platform/container/allowance.rs`)

**Analog:** the enum and its three match sites in the same file (235-360).

**Enum** (235-248): `#[serde(tag = "reason", rename_all = "snake_case")] #[non_exhaustive]`; add after `LedgerUnavailable`:
```rust
/// The run's model has no `treasurer.pricing` row, so no budget can be derived ...
ModelUnpriced { model: String },
```
**as_str** (256): add `HaltReason::ModelUnpriced { .. } => "model_unpriced"`; update the rustdoc "`\"allowance_exhausted\"` or `\"ledger_unavailable\"`" text and doctest.

**herald_line** (305): copy the `LedgerUnavailable` arm shape, with the U+26D4 escape the file uses:
```rust
HaltReason::ModelUnpriced { model } => {
    format!("\u{26D4} halted: model {model} has no treasurer.pricing row")
}
```
Extend the doctest with `assert_eq!` as the existing doctest does.

**wire_json** (345): copy the `LedgerUnavailable` arm but add the field:
```rust
HaltReason::ModelUnpriced { model } => {
    serde_json::json!({ "reason": self.as_str(), "model": model })
}
```
Add serde round-trip, herald, wire tests beside the existing ones (~1040-1127 herald tests, ~1307-1382 serde/wire tests). Wire must be `{"reason":"model_unpriced","model":"..."}`. Also update `RunResponse.halt_reason` field doc at `crates/paladin-web/src/run_controller.rs:445` then `make openapi`. Docs: `platform-api.md` (~156-165, ~226-229, ~797-798), `treasurer.md` (~34, ~41), `configuration.md` (~478-480, ~592). `MIGRATION.md` 11.1 row M-B-30; CHANGELOG root and `paladin-core` `[Unreleased]`.

### `worker.rs` Unpriced arm and `worker_tests.rs` flip

**Analog:** the existing `DispatchBudget::Halt(HaltReason)` arm (worker.rs ~435, ~1691, `derive_dispatch_budget` ~1873). Map `AdmissionError::ModelUnpriced { model }` to `DispatchBudget::Halt(HaltReason::ModelUnpriced { model })`, delete `Unpriced(String)` and its arm (which today calls `emit_agent_run_finished(.., Failed, ..)` + `persist_failure`). Update the rustdoc at `derive_dispatch_budget` and the "three no-call outcomes" comment.

Test: rename and rewrite `agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm` (~2976) to the assertions of `agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm` (~2885): status `Halted`, `run.error.is_none()`, `run.halt_reason == Some(HaltReason::ModelUnpriced { model: "gpt-4".into() })`, SSE terminal `status: "halted"` plus `halt_reason == wire_json()`, delivery `RunEventKind::Halted`. Watch it fail first with the current `Failed` behaviour (Pitfall 3).

---

### `ContentListRepository` refusal bodies (storage adapters)

**Analog:** `impl ContentListRepository for SqliteStore` at `sqlite_content_repository.rs:648` (ten `todo!("Implement ContentList ...")` bodies); `mysql_content_repository.rs` has the same ten for `MySqlContentRepository`. Port: `crates/paladin-ports/src/output/repository_port.rs:84` (synchronous trait, all `Result<_, RepositoryError>`; `RepositoryError::QueryError(String)` is the D-02 variant, enum is exhaustive).

**Apply:** one greppable message shape per backend, for example
```rust
fn get_by_id(&self, _id: Uuid) -> Result<Option<ContentList>, RepositoryError> {
    Err(RepositoryError::QueryError(
        "ContentListRepository::get_by_id is not implemented for the sqlite backend".to_string(),
    ))
}
```
(match each method's real signature; `count()` returns `Result<u64, _>`). Rustdoc says unfinished and points at `55-BASE-LAYER-REVIEW.md`. Do not interpolate backend text. The feature-gated table test goes in the file's existing `#[cfg(test)]` module: SQLite via `SqliteStore::new("sqlite::memory:")` (async, offline); MySQL via a lazy pool (`MySqlPoolOptions::new().connect_lazy(..)`, ASSUMED, tokio context needed). Check with `cargo check -p paladin-storage --features mysql --tests --offline`. Delete `content_filtering_service.rs` and the two commented `mod` lines in `services/mod.rs`.

---

### `seeded.rs` `claude_opus_4_8`, `reasoning.rs` / `layers.rs` const, `paladin-llm` cfg gates

- **seeded.rs** analog: sibling rows `claude-fable-5` (`true`, ~285/360) and `claude-haiku-5-5` (`false`, ~166). Change is `.with_reasoning(true)` plus a comment citing `50.1-SEED-PROBE.md` / `51-EFFORT-PROBE.md`, only if Open Question 1 resolves that way (the field contract says "returned reasoning text", and the 50.1 row records `absent`; escape clause in D-06 keeps `false`). Update pins at `registry/mod.rs:1702` and `:1734`.
- **TESTS-05:** `SELF_HOSTED_PROVIDERS` (`reasoning.rs:257`, `const SELF_HOSTED_PROVIDERS: [&str; 2] = ["ollama", "openai-compatible"];`) becomes `pub(crate)`; `config/layers.rs:342` `REASONING_WIRE_PROVIDERS` is deleted and its two uses (~499 and the test ~671) import it; add one test asserting both call sites use the same slice.
- **TESTS-03/04:** analog gate form is already in `conformance.rs` for `thinking_effort_wire_value`: `#[cfg(any(feature = "kimi", feature = "ollama", feature = "openai-compatible"))]`. Apply the same shape to `chat_effort_wire_value` (openai, deepseek, grok), and gate `PinnedSpread` / `pin_waiter_spread` (`cadence.rs`) and `ProfiledStubPort` (`fallback.rs`) on `all(test, feature = "mock")`. No `#[allow(dead_code)]`. Matrix command: `cargo clippy -p paladin-llm --tests --no-default-features --features <f>` for the 13 combos in RESEARCH Code Inventory 8.

### `commander.rs` tests (TESTS-06)

Replace each of the four `// TODO:` blocks with a pointer comment naming the relocated test in `tests/integration/commander_error_paths_test.rs` (table in RESEARCH Code Inventory 9). Keep the running assertions. For the Phalanx stub say the relocated test builds a Formation.

### Fences: `planning_service.rs`, `file_content_fetcher.rs`, base-layer rustdoc

- **Fence idiom analog:** the typed-error pattern above (existing exhaustive `VersioningError`: `VersionNotFound`, `EntityNotFound`, `InvalidVersionNumber`, `VersioningDisabled`, `StorageError`, `SerializationError`; no new variant allowed) for the two versioning services' fabricated `Uuid::new_v4()` sites (`field_version_service.rs` ~487; `collection_versioning_service.rs` ~415-417).
- `planning_service.rs` ~417 `Subtask::new(.., "Expected output from subtask execution")`: the field is read in `execute_subtask` (~512) as `EXPECTED OUTPUT: {}`, so fence the prompt too (omit the line when empty) and pin both in tests (Pitfall 4).
- `file_content_fetcher.rs` `ingest_content` (~152): return an `Err` for a valid URL instead of the no-op `Ok(())`; flip `test_ingest_content_validation` (~308) from `is_ok()` to `is_err()` for the valid case, watched failing first.
- Base-layer rustdoc: convert `/* */` headers to `///` or `//!` (`field.rs`, `collection.rs`, `component/element.rs`); `tests/entity_rename_guard.rs` blanks comments and strings, so doc edits are safe but identifier renames are not.

### Rustdoc allow removal (CIREL-08)

Eight lines: `crates/paladin-ports/src/lib.rs:51-52`, `crates/paladin-llm/src/lib.rs:49`, `crates/paladin-storage/src/lib.rs:20`, `crates/paladin-notifications/src/lib.rs:14`, `src/lib.rs:117-119` (leave the `#![warn(rustdoc::broken_intra_doc_links)]` above). Fix patterns are in RESEARCH Code Inventory 10 (explicit path target, plain code span for feature-gated or higher-layer targets, escape heading brackets). Each crate commit must pass both bars (`cargo doc --workspace --no-deps` and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --keep-going`).

### ADR-0033 amendment and `doc-coverage-report.md`

**Analog:** ADR-0033's own amendment blocks, e.g. `**Amendment, 2026-09-18, Phase 36.1 / plan 36.1-12 — Known crate-level suppressions.**` (l.185). Add a dated block `**Amendment, 2026-11-01, Phase 55 / CIREL-08 — ...**` recording the measured 474 / 455 figures and that all eight allows were removed. Storage allow is now at line 20, not 19. `doc-coverage-report.md` (48 lines, "Archived — historical document" banner, linked from `SUMMARY.md:115`) is regenerated as a dated measurement record (D-12).

### `55-CORRECTNESS-EVIDENCE.md`

**Analog:** `54-first-run-usability/54-FIRST-EVIDENCE.md`. Copy: title line `# Phase N ... Evidence`, a preamble giving date, branch, phase head SHA and workspace version, a note that "Before" cells come from plan SUMMARYs and "Head" cells were re-run, then the table:
```
| Item | Command | Before (from the plan SUMMARY) | Head output | Disposition |
```
Dispositions used: `changed in <plan>`, `already closed (<phase>)`. One row per FIXES/TESTS/CIREL item, with the proving command (RESEARCH gives them, e.g. the 13-combo clippy run showing `thinking_effort_wire_value` already gated).

### `55-BASE-LAYER-REVIEW.md`

**Analog:** `53-tenant-isolation-closure/53-VERSION-DECISION.md`: a decision record with head SHA, date, command, a classification verdict, then verbatim evidence. One row per item (Field, Collection/CollectionType/Item, element, the two versioning services, `planning_service.rs` placeholder, `file_content_fetcher.rs`, the 20 storage bodies) with keep / fence / remove decision and the caller read. Say "no design record exists beyond the todo's own account" (RESEARCH Code Inventory 5). The semver block in 53-VERSION-DECISION (twelve `semver: <pkg>: N checks: N pass, M skip` lines from `scripts/check-semver.sh`) is the shape for D-13's semver record in the evidence file.

### Crate `CHANGELOG.md` `[Unreleased]` anchors

**Analog:** `crates/paladin-memory/CHANGELOG.md` lines 1-14:
```markdown
## [Unreleased]

### Fixed

- Correction to the `[0.11.0]` entry: ... (`MIGRATION.md` section 10.1 `M-B-12`; FIRST-05).

## [0.11.0] - 2026-10-26
```
Only `paladin-content`, `paladin-ai-core`, `paladin-memory`, `paladin-web` and the root have the anchor today. `paladin-storage`, `paladin-llm`, `paladin-ports`, `paladin-battalion`, `paladin-eval`, `paladin-herald`, `paladin-notifications` need a new `## [Unreleased]` heading inserted above `## [0.11.0]`. Never edit a published `[0.11.0]` section.

## Shared Patterns

### Offline, check-only gate scripts
**Source:** `scripts/check-version-strings.sh` (68-80, 251-306). **Apply to:** `check-no-stub-panics.sh`. `set -euo pipefail`, python3 + tomllib only, no network, no cargo, no write mode, named never-vacuous failures, every finding listed before exit, test-only env root seam.

### Fixture-tree shell harness
**Source:** `tests/scripts/check-version-strings_test.sh` (22-160). **Apply to:** the new harness. `mktemp -d` plus `trap`, fail cases first, passing cases, real tree, checksum no-mutation, `assert_fire` / `assert_silent`.

### Pure function plus thin logger
**Source:** `run_api_wiring.rs` `scan_report` (721) and `report_unattributed_schedules` (760). **Apply to:** the FIXES-06 boot warning. Tests assert the pure output, never a captured logger.

### Additive-first under the patch semver gate
**Source:** RESEARCH Pattern 3. **Apply to:** every Rust change. Use `pub(crate)` / `#[cfg(test)]` where the only caller is in-crate; the only public additions are `HaltReason::ModelUnpriced` and one OpenAPI description string; no `#[deprecated]`; flagged changes are reshaped, never registered in MIGRATION 11.2.

### Typed error fences, no panics in library code
**Source:** `RepositoryError::QueryError` (storage fences) and `VersioningError` (base-layer fences). **Apply to:** all stub-fence edits. Existing variants only (both enums are exhaustive); messages name the method and backend, never a key value, backend text only to `log::error!`.

### House `HaltReason` surface
**Source:** `allowance.rs` 235-360. **Apply to:** FIXES-03. Every new reason touches `as_str`, `herald_line`, `wire_json`, serde/herald/wire tests, the doctests, `platform-api.md`, `treasurer.md`, `configuration.md`, OpenAPI description, MIGRATION 11.1 and CHANGELOG.

### TDD red-then-green
**Apply to:** all behaviour changes. Watch each fail first: the worker test flip, the two storage table tests, the `ingest_content` flip, the guard harness (write the harness before the script).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Out-of-line test-module resolution in the guard scanner | utility (python) | transform | No existing gate resolves `mod x;` to files; use RESEARCH Code Examples 1 prototype (validated: 624 files scanned, 19 test-only, 26 findings). Blanking technique mirrors `tests/entity_rename_guard.rs`. |
| Lazy-pool MySQL table test | test | CRUD | No existing offline MySQL test double; `connect_lazy` is ASSUMED (RESEARCH Code Inventory 1), verify at implementation. |

## Metadata

**Analog search scope:** `scripts/`, `tests/scripts/`, `Makefile`, `src/infrastructure/web/`, `src/application/services/run/`, `crates/paladin-core/src/platform/container/`, `crates/paladin-storage/src/`, `crates/paladin-llm/src/`, `.planning/phases/53-*` and `54-*`, `.planning/decisions/0033-*`, crate CHANGELOGs.
**Files scanned:** about 14 read in full or by range, plus targeted greps.
**Pattern extraction date:** 2026-11-01
