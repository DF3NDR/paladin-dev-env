# Phase 57: Register Hygiene Close-Out - Pattern Map

**Mapped:** 2026-11-04
**Files analyzed:** 24 (new or modified, grouped where they share one pattern)
**Analogs found:** 21 / 24

Most of this phase is planning-register work, so the "analogs" for those files are earlier phase artifacts
and tooling precedents. Only the rows 85-87 deliverable (and any code the sweep mints) is Rust.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/paladin-ports/src/output/run_repository_port.rs` (new defaulted `thread_owner_run`) | port | request-response | same file, `insert_with_latest` (lines 395-425) | exact |
| `crates/paladin-storage/src/run/{in_memory,sqlite,postgres}.rs` (override with `LIMIT 1`) | adapter | CRUD | same files, `insert_with_latest` overrides (sqlite.rs:750) | exact |
| `crates/paladin-storage/src/run/contract_tests.rs` (new clause) | test | CRUD | same file, `list_filters_by_thread_assistant_and_status` (:377), `get_on_unsupported_schema_version_fails` (:796) | exact |
| `crates/paladin-core/src/platform/container/principal.rs` (`permits_thread` docs and tests) | model | transform | same file, lines 270-330 | exact |
| `crates/paladin-web/src/thread_controller.rs` (`visible_thread_page`, `latest_run_for_thread`, `MockRunRepository`) | controller | request-response | same file, `Serialization` skip arm (:806-814) | exact |
| `src/application/services/run/submission.rs` (`ensure_thread_visible`) | service | request-response | same file, :277-300 | exact |
| `.planning/decisions/0063-thread-and-job-tenant-visibility.md`, `docs/src/user-guides/parley-and-chronicle.md`, `MIGRATION.md` 11.1, `CHANGELOG.md` `[Unreleased]`, `crates/paladin-web/openapi.json` | config / docs | transform | existing "latest run" wording in the same files (grep `latest run`) | exact |
| Possible sweep fix: `crates/paladin-storage/src/minio.rs:550-556` (45 CR-01 `list_files` delimiter) | adapter | file-I/O | `crates/paladin-storage/src/run/contract_tests.rs` contract-test shape | role-match |
| `SECURITY-EXCEPTIONS.md` (4th zip row, re-dating, history paragraph) | config | transform | rows RUSTSEC-2026-0367/0368/0369 (lines 136-173) | exact |
| `.cargo/audit.toml`, `deny.toml` (mirror RUSTSEC-2026-0429 and wording) | config | transform | `.cargo/audit.toml:50-52`, `deny.toml:123-125` | exact |
| `.planning/phases/57-*/57-REGISTER-EVIDENCE.md` | docs / evidence | transform | `55-CORRECTNESS-EVIDENCE.md`, `53-TENANT-EVIDENCE.md` | exact |
| `.planning/phases/57-*/57-REVIEW-DISPOSITIONS.md` | docs / register | batch | `.project/v0.11.1/02-dogfood-findings-log.md` (checked by `scripts/check-findings-log.sh`) | role-match |
| `.planning/phases/57-*/57-JUDGMENT-READ.md` | docs / worksheet | batch | `40-VERIFICATION.md` Prohibitions table (:165-197), `42-VERIFICATION.md` `human_verification_resolved` | role-match |
| 13 archived `NN-REVIEW.md` (append `## Dispositions (Phase 57)`) | docs | batch | Phase 46/55 append-only precedent | role-match |
| `40-VALIDATION.md:76` (Manual-Only row) and `42-VALIDATION.md` (new row) | docs | transform | the existing Manual-Only rows (`42-VALIDATION.md:96-101`) | exact |
| `45-01-SUMMARY.md`, `51-13/14/15-SUMMARY.md` (`requirements-completed`) | docs | transform | `51-17..20-SUMMARY.md` (real `requirements-completed` values) | exact |
| `.planning/debug/{3 sessions}.md` -> `.planning/debug/resolved/` | docs | file-I/O | `templates/DEBUG.md` and the debug.md resolved convention | exact |
| `.planning/WINDOWS.md` (via `gsd-tools windows` only) | tooling | CRUD | rows 58, 62, 63, 73 closure (55-13-SUMMARY deviation note) | exact |
| Optional `scripts/check-review-dispositions.sh` + `tests/scripts/check-review-dispositions_test.sh` | utility / test | batch | `scripts/check-findings-log.sh` + `tests/scripts/check-findings-log_test.sh` | exact |

## Pattern Assignments

### `crates/paladin-ports/src/output/run_repository_port.rs` (port, request-response)

**Analog:** `insert_with_latest`, same file (lines 395-425).

**Defaulted-method pattern (semver-additive; copy this doc shape, including a "Default implementation" section):**
```rust
    /// # Default implementation
    ///
    /// Inserts `run` as-is ... and it means adding this method does not break any
    /// pre-existing implementor of this trait (X-10.4). Override it to get
    /// the real freeze-at-submit resolution.
    async fn insert_with_latest(&self, run: &Run) -> Result<u32, RunRepositoryError> {
        self.insert(run).await?;
        Ok(run.assistant.version)
    }
```
The new `thread_owner_run(&self, thread: &ThreadId) -> Result<Option<Run>, RunRepositoryError>` MUST carry a default
body (walk `list`, keep the earliest run with `submitted_by.is_some()`), otherwise the patch gate fails and the
eleven in-tree implementors stop compiling. Imports already present in the file (lines 39-49): `async_trait`,
`RunReadScope`, `Run`, `RunId`, `ThreadId`. Add a doc test in the style of the module's existing example (`:216-291`)
since public APIs need doc tests. The in-file test `default_impl_of_the_latest_freezing_insert_method_inserts_verbatim`
(:558) is the model for a default-impl test against `MockRepository`.

---

### `crates/paladin-storage/src/run/{in_memory,sqlite,postgres}.rs` (adapter, CRUD)

**Analog:** the `insert_with_latest` overrides (`sqlite.rs:750`; same method in `in_memory.rs`, `postgres.rs`).
Override with `WHERE thread_id = ? AND tenant_id IS NOT NULL ORDER BY submitted_at ASC, run_id ASC LIMIT 1`
(columns come from migration 008; no new migration). Each adapter wires the contract clauses from its own
`#[cfg(test)] mod tests`, one thin `#[tokio::test]` per clause:
```rust
// sqlite.rs:1011-1014 shape
async fn insert_with_latest_resolves_current_latest_and_freezes_it() {
    ...
    contract_tests::insert_with_latest_resolves_current_latest_and_freezes_it(...).await;
}
// in_memory.rs:487 shape
contract_tests::insert_then_get_round_trips_every_field(&InMemoryRunRepository::new()).await
```

### `crates/paladin-storage/src/run/contract_tests.rs` (test, CRUD)

**Analog:** same file. Clauses are `pub async fn name(port: &dyn RunRepositoryPort)` using `sample_run(&thread, "assistant", contract_timestamp())` (:52-59). Model the corrupt-other-row clause on
`get_on_unsupported_schema_version_fails` (:796) (plants a bad-schema row) and the ordering clause on
`list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap` (:313). Required clauses:
earliest attributed run wins; an unattributed run and an Admin-fork run never move ownership; a corrupt
non-owner row does not fail the lookup.

---

### `crates/paladin-core/src/platform/container/principal.rs` (model, transform)

**Analog:** same file, lines 270-330. Keep the `permits_thread(&self, Option<&Run>)` signature; rename the parameter
to `owner`, rewrite the doc paragraph ("belongs to the tenant of its **latest run**"), keep the doctest shape:
```rust
    pub fn permits_thread(&self, latest: Option<&Run>) -> bool {
        match (self, latest) {
            (Self::All, _) => true,
            (Self::Tenant(_), None) => false,
            (Self::Tenant(_), Some(run)) => self.permits(run),
        }
```
Do not rename test `permits_thread_admits_all_and_only_the_latest_runs_tenant` without updating its citations
(WINDOWS reasons, `56-DOGFOOD-EVIDENCE.md`, `.project/v0.11.1/02-dogfood-findings-log.md`, 56-04 verify commands).
Add new tests beside it.

---

### `crates/paladin-web/src/thread_controller.rs` (controller, request-response)

**Analog:** same file. **Row 85 skip-and-log arm** (:806-814), extend the guard to `UnknownSchemaVersion`:
```rust
                Err(RunRepositoryError::Serialization { message })
                    if !matches!(scope, RunReadScope::All) =>
                {
                    // Hidden, not shown: an unreadable row has no attributable owner.
                    log::error!(
                        "thread API list owner lookup failed: run serialization error: \
                         {message} (thread skipped as not visible)"
                    );
                }
                Err(other) => { return Err(run_repo_error_for_scope(&RunReadScope::All, ...)); }
```
Log line must not interpolate more backend text than the existing arm does (V7). Check the `UnknownSchemaVersion`
variant's fields in `RunRepositoryError` before writing the pattern.

**Row 86/87 lookup replacement** (:739-753): `latest_run_for_thread` calls `runs.list(RunQuery { thread_id, limit: 1, ..Default::default() })`. Replace the body with `runs.thread_owner_run(thread).await` and rename (callers at :800 and :891). Update the three utoipa `description` strings (~lines 929, 1053, 1263) and the module doc (:65) that say "latest run".

**Hideable-error helper** (:693-727): `run_repo_error_for_scope(scope, ctx, err, fallback)` with a `hideable` matches set. Pitfall: it is `pub(crate)` and shared with `run_controller`; widening it changes `GET /runs/{id}` too (see RESEARCH Pattern 2, A2).

**Test double** (`MockRunRepository`, ~:2392-2510): has `ThreadRunFailure::Serialization` injection through `list` and a `list_calls` counter. Override the new method in the double with per-thread failure injection and re-point the bounded-walk cost assertions. Rewrite `mixed_tenant_thread_belongs_to_the_latest_runs_tenant` (:2887) to the earliest-attributed rule. The Waypoint failure-injection enum at :1577-1640 is the pattern for an `UnknownSchemaVersion` variant.

---

### `src/application/services/run/submission.rs` (service, request-response)

**Analog:** same file, `ensure_thread_visible` (:277-300). Replace the `list(limit: 1)` call:
```rust
        let page = self
            .repository
            .list(RunQuery { thread_id: Some(thread_id.clone()), limit: 1, ..Default::default() })
            .await
            .map_err(map_repository_error)?;
        match page.items.into_iter().next() {
            Some(latest) if !scope.permits_thread(Some(&latest)) => { Err(RunSubmissionError::UnknownThread { ... }) }
```
with `self.repository.thread_owner_run(thread_id).await.map_err(map_repository_error)?`. Keep the "calls `permits_thread` only with `Some`" thread-birth rule and the early returns for `requested_by == None` and `RunReadScope::All`.

---

### `SECURITY-EXCEPTIONS.md`, `.cargo/audit.toml`, `deny.toml` (config)

**Analog:** the RUSTSEC-2026-0367/0368/0369 `[[exception]]` blocks (lines 136-173). Copy one whole block for
RUSTSEC-2026-0429 (fields: id, class, crate, path, why_present, why_not_fixable, owner, review_date, scope,
compensating_control, revisit_condition) and re-date all four per D-31. Mirrors:
```toml
# .cargo/audit.toml:50-52 (ignore array)
    "RUSTSEC-2026-0369",
# deny.toml:125
    "RUSTSEC-2026-0369", # zip (permissions through symlinks) — build-dependency of utoipa-swagger-ui (paladin-web), build time only
```
Also update the prose counts in `SECURITY-EXCEPTIONS.md` (eight to nine vulnerability rows, fifteen to sixteen live suppressions) and the history paragraph at line 61. Verify with `./scripts/check-advisory-register.sh` AND `make security` (the script never reads the advisory db). The compensating control for 0429 is the same (build script never calls `extract`).

---

### `57-REGISTER-EVIDENCE.md` (evidence, transform)

**Analog:** `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-CORRECTNESS-EVIDENCE.md` (header block: date, branch, head SHA, "Before cells copied from SUMMARYs / Head cells re-run"; then a table `| Item | Command | Before | Head output | Disposition |`). Also `53-TENANT-EVIDENCE.md`. One section per WINDOWS row with a CI run id, job id, test name or measurement.

### WINDOWS closure (via `gsd-tools windows`)

**Analog:** rows 58, 62, 63, 73 (see 55-13-SUMMARY deviation note). Sequence: `windows fixed <id>` (no reason arg), hand-edit the reason cell in BOTH the table row and the JSON entry (no `|`), prove `parseLedger`/`renderLedger` byte-identical, then `windows status`. Waivers take the reason inline in D-03 shape:
```bash
$G windows waive 65 "Re-homed: owner=Treasurer maintainers, revisit=next feature milestone roadmap, basis=ADR-0057 closing conditions. <rationale>"
$G windows append --kind deviation --phase 45 --file crates/paladin-storage/src/minio.rs --line 550 --description "..."
```
Do all closes in the LAST plan, after sweep and judgment plans have minted their rows.

### `57-REVIEW-DISPOSITIONS.md` and optional checker (batch)

**Analog:** `scripts/check-findings-log.sh` (header lines 1-30: parses entries outside code fences, last disposition bullet wins, accepted forms require a pointer, bare placeholders are NO_POINTER) plus `tests/scripts/check-findings-log_test.sh`. Row shape per D-10: `| Phase | Id | Title | Disposition | Evidence | Owner (if waived) |`; totals line `open: 0`. Avoid the 56-REVIEW weaknesses: fail on any archived warning id absent from the register, reject `TBD`/`none`/`todo`, require Owner on `waived`. Note: `scripts/check-dogfood-dispositions.py` named in CONTEXT does not exist. Include the two Criticals (38 CR-01, 45 CR-01) and Phase 40 IN-01/IN-04. Run `shellcheck` on any script.

### `57-JUDGMENT-READ.md` (worksheet)

**Analog:** `40-VERIFICATION.md:165-197` Prohibitions table for verdict style; the 20 prohibitions themselves live in the `must_haves.prohibitions` frontmatter of the six `40-0N-PLAN.md` files. Columns per CONTEXT: `| # | Prohibition (plan) | LLM verdict | Evidence (test or file:line) | Human: confirm / reject | Note |` plus a credential-checklist sub-table using the bullets in `.github/instructions/security.instructions.md`. Diff base `629ef660` is not in the shallow clone; use `gh api repos/DF3NDR/paladin-dev-env/compare/629ef660...be3a9030` (the `DF3NDR/paladin` slug returns 403).

### SUMMARY frontmatter (REG-01)

**Analog:** `51-17..20-SUMMARY.md` carry real `requirements-completed: [...]` values. Edit `45-01-SUMMARY.md:49` (`[]` to `[STORE-02]`), add the key to `51-15-SUMMARY.md` (absent today), and set REASON-02/03/06 on `51-13`/`51-14`. Read back with `gsd-tools query summary-extract <relative path> --fields requirements_completed` from the project root.

### Debug archive (REG-03)

**Analog:** `.planning/debug/resolved/{SLUG}.md` convention (`templates/DEBUG.md`, `workflows/execute-phase.md`). `git mv` each of the three sessions, set `status: resolved`, add `resolved:` citing WINDOWS row 76 (kimi), 75 (openai), 74 (vendor-token-usage). Verify with `gsd-tools audit-open` (debug count 0).

## Shared Patterns

### Additive-only public surface (patch gate)
**Source:** `run_repository_port.rs` `insert_with_latest` (defaulted method, doc "does not break any pre-existing implementor") and `RunTracePort::max_seq`.
**Apply to:** the new port method. Gate: `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh` once, `make api-surface` (plus `make api-surface-update` and a CHANGELOG `[Unreleased]` entry). No `#[deprecated]`; `[0.11.0]` changelog sections and MIGRATION 9/10 stay byte-frozen; edits go to MIGRATION 11.1.

### Role-aware error hiding (hidden == missing 404 for Tenant, redacted 500 for Admin)
**Source:** `thread_controller.rs:693-727` (`run_repo_error_for_scope`) and :806-814.
**Apply to:** the list walk and gate for `UnknownSchemaVersion`; also `ensure_thread_visible`.

### Contract-test wiring across adapters
**Source:** `contract_tests.rs` clause fn + one thin test per adapter (`in_memory.rs:487`, `sqlite.rs:1011`, `postgres.rs`).
**Apply to:** every new storage clause (owner run, corrupt other row, any 45 CR-01 fix).

### Register-edit hygiene
**Source:** `windows` CLI + hand-edited reason cell; D-10/D-13/D-22 append-only archive rule.
**Apply to:** all archived REVIEW/VALIDATION/SUMMARY edits (`git diff` must show pure additions above the appended section); WINDOWS is never edited directly.

### Credential-handling review
**Source:** `.github/instructions/security.instructions.md` (redact before truncate via `crates/paladin-llm/src/redaction.rs`, no key in logs or Debug, no redirect following, no key in response bodies).
**Apply to:** the new thread-owner code path, the Phase 40 re-read (D-21), and any 41 WR-05 / 45 WR-02 / 45 WR-05 fix.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `57-JUDGMENT-READ.md` human confirm/reject column | worksheet | batch | No prior per-item owner-confirm worksheet; only the bare `40-UAT.md` test 3 `pass` exists |
| `.planning/debug/resolved/` directory | docs | file-I/O | Directory does not exist yet; the convention comes from the workflow template, not a prior example |
| Hideable `UnknownSchemaVersion` in `run_repo_error_for_scope` | controller | request-response | The helper only hides `Serialization` and Waypoint schema errors today; no run-store schema-version hide precedent (use RESEARCH Pattern 2) |

## Metadata

**Analog search scope:** `crates/paladin-ports`, `crates/paladin-storage/src/run`, `crates/paladin-core/src/platform/container/principal.rs`, `crates/paladin-web/src/thread_controller.rs`, `src/application/services/run/submission.rs`, `scripts/`, `SECURITY-EXCEPTIONS.md`, `.cargo/audit.toml`, `deny.toml`, Phase 55 evidence files.
**Files scanned:** about 14 read or excerpted, plus targeted greps.
**Caveat for the planner:** RESEARCH.md measured `make security` red (RUSTSEC-2026-0429) and found 45 CR-01 live; sequence the zip rows first and WINDOWS closes last. Lines 1-377 and 378-499 of RESEARCH.md were both read.
**Pattern extraction date:** 2026-11-04
