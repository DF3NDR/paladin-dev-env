---
phase: 49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump
plan: 01
subsystem: testing
tags: [rust, cargo-test, compile-only, consumer-surface, api-stability, makefile]

requires:
  - phase: 48
    provides: "the v0.10.1 library surface the consumer (web3sec) imports"
provides:
  - "tests/consumer_surface_muster.rs: compile-only muster over the re-verified handoff Appendix-A surface (ADOPT-04)"
  - "one [[test]] consumer_surface_muster table with required-features = [llm-anthropic, llm-deepseek]"
  - "make check-consumer-surface alias, deliberately outside check-gates"
  - "49-VALIDATION.md per-task verification map (14 rows across plans 49-01..49-06)"
affects: [49-02, 49-03, 49-04, 49-05, 49-06, phase-50, phase-51, phase-47]

tech-stack:
  added: []
  patterns:
    - "Compile-only muster: every pin lives in a never-called fn; the single #[test] body is comments only"
    - "Strict full struct literals (no ..) so an additive field turns the muster red on purpose"
    - "fn-pointer coercions pin associated-function signatures (Commissary::new, from_port, PaladinResult::new, TokenUsage builders)"

key-files:
  created:
    - tests/consumer_surface_muster.rs
  modified:
    - Cargo.toml
    - Makefile
    - .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-VALIDATION.md

key-decisions:
  - "check-consumer-surface is NOT part of check-gates: those eight members are offline script guards that finish in about a second and no workflow invokes check-gates, while this target compiles the facade crate. make check, make lint and CI already compile or run the muster."
  - "required-features on the [[test]] table makes an explicit --test run fail loudly (requires the features) instead of skipping when the consumer's provider features are off."
  - "The validation map has 14 rows, not the 13 the must_haves truth states: the plan files hold 3+2+2+3+2+2 = 14 task elements and Task 3 itself says to follow the files."

patterns-established:
  - "Consumer-surface muster: a library change that breaks a consumer path, signature, struct shape or trait fails make check / make lint / CI before a consumer bump finds it"
  - "Red-muster rule: an intended break updates the pin, its MIGRATION.md 9.2 row and the 9.8 library-consumer table in one commit; the library is never shimmed to turn it green"

requirements-completed: [ADOPT-04]

coverage:
  - id: D1
    description: "Compile-only consumer-surface muster as exactly one declared [[test]] target, green under cargo test"
    requirement: "ADOPT-04"
    verification:
      - kind: integration
        ref: "cargo test --offline --test consumer_surface_muster (1 passed; 0 failed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Muster wired into the existing gates (make check, make lint, make check-consumer-surface) without changing check-gates"
    requirement: "ADOPT-04"
    verification:
      - kind: other
        ref: "make check; make lint; make check-consumer-surface; diff of the check-gates line against HEAD"
        status: pass
    human_judgment: false
  - id: D3
    description: "Muster cannot pass vacuously and no library API changed"
    requirement: "ADOPT-04"
    verification:
      - kind: other
        ref: "cargo test --no-default-features fails with 'requires the features'; PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface reports unchanged (4274 items); git diff --name-only 91be2280 -- crates src is empty"
        status: pass
    human_judgment: false

duration: 2h 05min (across two executor sessions, including the disk-exhaustion stop)
completed: 2026-10-14
status: complete
---

# Phase 49 Plan 01: Consumer-surface muster Summary

**A compile-only `tests/consumer_surface_muster.rs` (one `[[test]]` target, feature-gated) that pins every re-verified Appendix-A import path, signature, struct shape and trait the web3sec consumer takes from Paladin, wired into `make check` / `make lint` / CI, with a `make check-consumer-surface` alias and a filled phase verification map.**

## Performance

- **Duration:** about 2h 05min across two executor sessions (the first stopped at a disk-exhaustion checkpoint)
- **Started:** 2026-10-14T19:07:00Z (approximate; first session)
- **Completed:** 2026-10-14T21:20:00Z
- **Tasks:** 3 of 3
- **Files modified:** 4 (1 created, 3 modified), none under `crates/` or `src/`

## Accomplishments

- The muster compiles and passes under `cargo test` as exactly one declared target: Commissary signatures (B-1), `TokenUsage` / `PaladinResult` / `LlmResponse` / `PaladinData` strict literals (B-2, B-3, DR-5, DR-6), three trait-implementer stubs coerced to `Arc<dyn ...>`, and identity fns proving prelude and ports paths name one type.
- The muster is reached by the gates that already run (`make check`, `make lint`, CI clippy / all-targets check / integration test) and by a new `make check-consumer-surface`; `check-gates:` is byte-identical to HEAD.
- `make api-surface` reports the public API unchanged (4274 items), `Cargo.lock` is unchanged, and the library diff against the phase base is empty.
- `49-VALIDATION.md` now carries one verification-map row per task across plans 49-01..49-06, the Wave 0 list and the manual-only table (ADOPT-01..03 and the coverage floor).

## Task Commits

1. **Task 1: Consumer-surface muster and its one [[test]] table** - `0288be8f` (test)
2. **Task 2: check-consumer-surface target and gate-wiring proof** - `4b44c6ac` (build)
3. **Task 3: Fill the 49-VALIDATION.md verification map** - `7d92e4ba` (docs)

**Plan metadata:** recorded in the final `docs(49-01): complete consumer-surface muster plan` commit.

## Gate Results (exit codes)

| Gate | Result |
|------|--------|
| `cargo test --offline --test consumer_surface_muster` | exit 0, `1 passed; 0 failed` |
| `cargo clippy --offline --test consumer_surface_muster -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo test --offline --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard` | exit 0 |
| `make check-consumer-surface` | exit 0, `1 passed` (re-confirmed in this session) |
| `make check` | exit 0 |
| `cargo test --offline --test consumer_surface_muster --no-default-features` | exit 101, `requires the features: llm-anthropic, llm-deepseek` (required) |
| `df -h .` before `make lint` | 7.4 GB free (7.3 GB after the full lint and api-surface runs) |
| `make lint` (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) | exit 0 on the re-run (the first attempt died with ENOSPC, see Deviations) |
| `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | exit 0, `API surface unchanged` (4274 items) |
| `check-gates:` line vs `git show HEAD:Makefile` | byte-identical |
| `git diff --name-only 91be2280 -- crates src`; `git status --porcelain -- .github crates src` | both empty |
| `git diff --exit-code 91be2280 -- Cargo.lock` | no change |

Every cargo invocation in this session ran with `CARGO_INCREMENTAL=0`.

## Files Created/Modified

- `tests/consumer_surface_muster.rs` - the compile-only muster (never-called pin fns, three stub trait implementers, one comment-only `#[test]`)
- `Cargo.toml` - one `[[test]] consumer_surface_muster` table with `required-features`; no dependency line changed
- `Makefile` - `.PHONY: check-consumer-surface`, explanatory comment, target; outside `check-gates`
- `.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-VALIDATION.md` - verification map, Wave 0, manual-only, sign-off (frontmatter still `draft`, `nyquist_compliant: false`, `wave_0_complete: false`)

## Decisions Made

- `check-consumer-surface` stays outside `check-gates` (it builds the facade crate; the eight members are offline script guards and no workflow invokes `check-gates`). No CI step was added.
- The `--test` form in the Makefile recipe fails loudly if the provider features are off (Pitfall 5).
- The map has 14 rows because the six plan files contain 14 task elements; the plan's own must_haves text says 13, and Task 3's action and its automated verify both follow the files.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Disk exhaustion during `make lint` (checkpoint, then resumed)**
- **Found during:** Task 2 (first executor session)
- **Issue:** `make lint` died with `No space left on device` before emitting any clippy diagnostic. The plan's A3-49-01 fallback threshold (below 1.5 GB free) had been crossed with a regenerated incremental cache.
- **Fix:** The executor stopped and returned a checkpoint. The orchestrator removed the rebuildable `target/debug/incremental` cache (about 5 GB freed, 7.4 GB free afterward); nothing else in `target/` was touched. This session re-ran `make lint` and `make api-surface` with `CARGO_INCREMENTAL=0` so the cache was not regenerated; both exited 0 locally, so the CI-evidence fallback was not needed.
- **Files modified:** none (disk cleanup only, outside the repository tracking)
- **Verification:** `make lint` exit 0; free space 7.3 GB afterward.
- **Committed in:** n/a

### Pin deltas versus the research draft

None. Every pin compiled against the current tree as drafted (the A2-49-01 assumption held); the library was not touched.

### Plan text discrepancy

The must_haves truth says "13 tasks in plans 49-01..49-06"; the files hold 14, and the map follows the files as Task 3 instructs.

---

**Total deviations:** 1 (Rule 3 environment blocker, resolved by the orchestrator), 0 pin deltas.
**Impact on plan:** none on scope; the ENOSPC stop only delayed the Task 2 gate evidence.

## Credential Review (T-49-02, security.instructions.md)

- Every credential-shaped argument in the muster is `String::new()` (the `AnthropicConfig::new` call takes three empty strings); no key, bearer string or URL literal appears (grep for `sk-`, `Bearer`, `http`, `api_key` finds none).
- No log statement or `Debug` formatting of a credential-carrying type exists in the file; no HTTP client is built at test time.
- Every process-spawning, environment-reading or client-building pin (`MCPClient::connect_stdio`, `DeepSeekConfig::from_env`, adapter construction) sits inside `service_and_adapter_signatures`, which nothing calls (its name occurs once); the only `#[test]` body is comments only. No panicking shortcut (`unwrap`, `expect`, `panic!`) exists in the file.
- `make security` was not part of this plan's verification; no dependency was added and `Cargo.lock` is unchanged.

## Known Stubs

None. The three `Muster*` types are compile-time trait-implementer fixtures by design, never called; they are not placeholders for missing wiring.

## Issues Encountered

- First-session ENOSPC during `make lint`; resolved as described under Deviations.
- Task 3's first extraction attempt matched the literal text `<automated>` inside Task 3's own action prose; the extraction was re-anchored to the line-start tag and all 14 commands were taken verbatim from the plan files.

## Threat Flags

None. No new network endpoint, auth path, file access or schema surface was introduced; the plan adds a test file, a manifest test table and a Makefile alias.

## Next Phase Readiness

- Plan 49-02 (MIGRATION.md / CHANGELOG), the Wave-1 sibling, is independent of this one. Plan 49-03 (red proof) depends on this plan's muster and Makefile target, both now committed.
- Plan 49-06 reconciles the muster with the consumer's Step-0 inventory (flagged assumption A1-49-01: pins on builder functions the consumer may use are not yet present).
- Follow-up recorded in the plan: Phases 50 and 51 change `LlmRequest`, `PaladinData` and the adapter/config pairs, so the strict literals turn red on purpose and must be updated in the same commit as the MIGRATION.md rows.

## Self-Check: PASSED

- FOUND: tests/consumer_surface_muster.rs, Cargo.toml, Makefile, 49-VALIDATION.md, 49-01-SUMMARY.md
- FOUND commits: 0288be8f, 4b44c6ac, 7d92e4ba
