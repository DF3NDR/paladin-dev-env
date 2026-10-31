# Phase 53 version decision (v0.11.1 patch versus v0.12.0)

Plan 53-08, success criterion 5, requirement R9, CONTEXT D-02. This file records the patch-versus-minor
decision for the Dogfood Cleanup milestone on measured evidence taken on the phase head named below.

## Semver evidence

- **Head SHA measured:** `4ef9ddd2aa7886f3e73041e0927b27c41801fc3b` (`4ef9ddd2a`), branch `claude/laughing-dirac-e0h2ax`,
  working tree clean at the start and end of the run.
- **Date:** 2026-10-31 (run started 15:06:34Z; semver leg 15:06:44Z to 15:11:55Z).
- **Source of the lines:** local reproduction of the CI job `Semver Checks (vs v0.11.0)` with the same script CI runs
  (not the PR job; the blocking checkpoint of Task 2 did not fire).
- **Command:** `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` (`cargo semver-checks check-release --package <pkg>
  --default-features --baseline-version 0.11.0 --release-type patch` for each of the twelve published packages).
- **Classification: GREEN** (twelve package lines, each with a non-zero check count and no `fail`; verdict `0 failure(s)`).
  Script exit status 0. Raw log: `/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/p53-08-head4ef9/semver.log`.

Lines copied verbatim from the script's output:

```
semver: paladin-ai-core: 223 checks: 223 pass, 31 skip
semver: paladin-ports: 223 checks: 223 pass, 31 skip
semver: paladin-herald: 223 checks: 223 pass, 31 skip
semver: paladin-battalion: 223 checks: 223 pass, 31 skip
semver: paladin-llm: 223 checks: 223 pass, 31 skip
semver: paladin-memory: 223 checks: 223 pass, 31 skip
semver: paladin-web: 223 checks: 223 pass, 31 skip
semver: paladin-notifications: 223 checks: 223 pass, 31 skip
semver: paladin-content: 223 checks: 223 pass, 31 skip
semver: paladin-storage: 223 checks: 223 pass, 31 skip
semver: paladin-eval: 223 checks: 223 pass, 31 skip
semver: paladin-ai: 223 checks: 223 pass, 31 skip
semver: 12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

The per-package counts equal Phase 52's green baseline (`223 checks: 223 pass, 31 skip`,
`52-SEMVER-RED-EVIDENCE.md`): phase 53 added public items (inherent methods, `pub` consts) and no lint fired.

## Gate sweep

All six commands ran to completion on head `4ef9ddd2a` (logs under
`/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/p53-08-head4ef9/`):

| Gate | Result line (exit status) |
|---|---|
| `make check-migration-allowlist` | `Allowlist is set-equal to the MIGRATION.md §11.2 deliberate-breaking register (crate|type pairs).` and `Every crate-wide cargo-semver-checks allow is mirrored by an allowlist entry (crate|lint pairs).` (exit 0) |
| `make check-gates` | `✅ OK: 12 crate(s) checked in publish order; every versioned workspace dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES exactly matches the cargo-metadata publishable set.` (exit 0) |
| `make security` | `advisories ok, bans ok, licenses ok, sources ok` (exit 0) |
| `make api-surface` | `✅ API surface unchanged` (4325 items) (exit 0) |
| `make clean-code` | `✅ doc-check passed (ADR-0033)` then `All 105 D-05 public API entry points carry a plural '# Examples' heading.` (exit 0; fmt, clippy, check and doc-check all passed) |
| `cargo test --workspace --no-fail-fast` | 62 test-result lines, 7774 passed, 229 ignored, 2 failed (exit 101): the only failures are the two known, pre-existing, network-bound tests `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `...::build_run_api_persists_run_traces_when_trace_persist_is_set` (`the run never reached a terminal status`, `run_api_wiring.rs:3036`), owned by TESTS-01 / Phase 55 and untouched by this phase; every other test result line is `ok`, including `openapi::tests::openapi_matches_committed_baseline` |

Known exceptions (recorded, not fixed, skipped or deleted): the two `build_run_api_persists_*` tests above. No other gate
is red.

- **coverage:** CI `Coverage` job (`cargo llvm-cov --fail-under-lines`, 82 % workspace floor, ADR-0006). `cargo-llvm-cov` is
  not installed in this environment, so this is CI-only evidence and is not claimed locally.

The sweep head is `4ef9ddd2a`. The commits made after it by this plan (`ff5b1167d` evidence section, `f2a906543`
STATE/ROADMAP/MIGRATION edits) change only `.planning/` files and one sentence in `MIGRATION.md` section 11's intro;
`make check-migration-allowlist` was re-run on `f2a906543` and is green (same two result lines as above).

## Decision

**Decision (2026-10-31): the milestone stays `v0.11.1` (patch). Every Phase 53 change is additive.**

Reason, each point measured above or below:

1. No change is registered in `MIGRATION.md` section 11.2: the table is header-only.
2. `.cargo/semver-checks-allowlist.toml` holds zero `[[entry]]` tables.
3. `Semver Checks (vs v0.11.0)` reproduced locally at `--release-type patch` reads `223 checks: 223 pass, 31 skip` for all
   twelve published packages with `0 failure(s)` (see `## Semver evidence`; head `4ef9ddd2a`).
4. `make api-surface` reports `API surface unchanged` and the other five gates are green (`## Gate sweep`).

No persisted or public-surface break was forced by the security fixes, so D-02's `v0.12.0` branch does not apply and no
human decision on a deliberate break is needed. The decision is taken in Phase 53, before Phase 58 opens, and is mirrored in
`.planning/STATE.md` (`### Decisions`), `.planning/ROADMAP.md` (milestone row and the Phase 53 entry) and `MIGRATION.md`
section 11's intro. Phase 58's pre-tag audit should re-run `scripts/check-semver.sh` on its own head and compare it with
this record rather than rely on it: Phases 54 to 57 can still add public items, and a `#[deprecated]` attribute or an enum
or signature change in any of them would turn a line red at patch level.

## Register state

`MIGRATION.md` section 11.2 as of head `f2a906543` (header and separator only, no data row):

```
| Crate | Type / Trait | Change | Mitigation | Deliberate-breaking? | Req |
|---|---|---|---|---|---|
```

- `.cargo/semver-checks-allowlist.toml`: `[[entry]]` count `0` (header comment `EMPTY AS OF PHASE 52`).
- `make check-migration-allowlist` on `f2a906543` (exit 0): `Allowlist is set-equal to the MIGRATION.md §11.2
  deliberate-breaking register (crate|type pairs).` and `Every crate-wide cargo-semver-checks allow is mirrored by an
  allowlist entry (crate|lint pairs).`
- `MIGRATION.md` sections 9 and 10 were not touched by this plan.

## Behavioral changes (not breaks)

These are HTTP, boot and configuration behavior changes recorded in `MIGRATION.md` sections 11.1, 11.5, 11.6 and 11.8.
None changes a Rust signature, a type or a serialized schema, so none is a section 11.2 break and none appears in the
allowlist.

| Id | Plan | Requirement | Behavioral change |
|---|---|---|---|
| `M-B-23` | 53-02 | TENANT-06 | An unreadable (`Serialization`) run row on the run id routes is the missing-run `404` for a non-admin and the fixed `run store error` 500 for an Admin. |
| `M-B-24` | 53-03 | TENANT-03, TENANT-06 | `/v1/threads/{id}*` routes hide a foreign tenant's thread behind the missing-thread `404` (was `200` on reads, `403` on resume and delete); a thread belongs to its latest run's tenant. |
| `M-B-25` | 53-04 | TENANT-03 | `GET /v1/threads` lists only the caller's tenant's threads; a page may be short (even empty) while `next_cursor` is set. |
| `M-B-26` | 53-05 | TENANT-07 | `GET /v1/agents/{id}/jobs/{job_id}` answers `404 unknown job` to another tenant and to a wrong agent id (was `200`); legacy unowned jobs are Admin-only. |
| `M-B-27` | 53-06 | TENANT-08 | A configured Treasurer with no engine factory on the run worker pool refuses to boot (`paladin-server` as shipped is unaffected). |
| `M-B-28` | 53-06 | TENANT-10 | Boot logs one advisory `warn!` listing schedules with `NULL created_by`; boot is never blocked and nothing is backfilled. |
| 11.5 entry | 53-06 | TENANT-09 | Reserved literals (`open-access`, `unattributed`, `anonymous` as key names; `open-access`, `unattributed` as tenants) are refused at config load. |

## Deviations

- **`get_job` is not marked `#[deprecated]` (PD-05-1, plan 53-05).** The orchestrator's option was to deprecate the
  superseded unscoped `paladin_web::agent_controller::get_job`. Measured on 2026-10-31 in planning: one `#[deprecated]` on a
  `pub fn` printed `223 checks: 222 pass, 1 fail, 0 warn, 31 skip` with `--- failure function_marked_deprecated ---`
  (`semver requires new minor version`, exit 100), whereas added inherent methods, `pub` consts, structs and free
  functions printed `223 checks: 223 pass, 31 skip`. The attribute would have forced either an allowlist entry or `v0.12.0`,
  so D-02's "reshape to additive first" applied: `get_job` keeps its signature, body and `#[utoipa::path]`, is unrouted, and
  the supersession is stated in its rustdoc and `MIGRATION.md` section 11.7. The phase-head line for the package that owns
  it is `semver: paladin-web: 223 checks: 223 pass, 31 skip`. The attribute is deferred to the next minor
  (`#[deprecated(note = "use get_job_scoped")]` plus its section 11.2 / 11.7 handling then).
- **Known test failures carried through the sweep.** `cargo test --workspace` is not fully green on the head: the two
  network-bound `run_api_wiring::tests::build_run_api_persists_*` tests fail as they did before this phase (TESTS-01,
  Phase 55). They are recorded, not fixed, skipped or deleted.
- **Coverage not measured locally.** `cargo-llvm-cov` is not installed here; the 82 % floor is CI's `Coverage` job.
- **Two measurement retries.** Two earlier executors of this plan stopped on red gates that were then repaired on the
  phase branch (`ef86cb46a`: six intra-doc links to the private `load_visible_thread`; `4ef9ddd2a`: the regenerated
  `crates/paladin-web/openapi.json` baseline, six description strings, no path, schema or operationId change). Nothing
  from those attempts is reused: every line above was re-measured on `4ef9ddd2a`.
- **Commit order.** The plan wants this file's commit to be the last of the phase. The executor protocol then adds one
  metadata commit (this plan's SUMMARY, STATE position and ROADMAP progress); that commit contains no code and does not
  alter this decision.
