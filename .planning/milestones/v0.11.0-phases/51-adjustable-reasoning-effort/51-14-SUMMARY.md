---
phase: 51-adjustable-reasoning-effort
plan: 14
subsystem: closeout-register-and-live-evidence
tags: [reasoning-effort, live-smoke, adapter-path, semver-register, cargo-semver-checks, migration, changelog, phase-gates]
status: complete-with-blocker

requires:
  - phase: 51-02
    provides: "live_vendor_smoke --effort-smoke (adapter-path smoke)"
  - phase: 51-04
    provides: "CompatRequestParameters Y row and interim allowlist entry"
  - phase: 51-07
    provides: "PaladinData Y row and interim allowlist entry"
  - phase: 51-08
    provides: "AgentDefinition Y row and interim allowlist entry"
  - phase: 51-09
    provides: "TraceEvent Y row and interim allowlist entry"
  - phase: 51-13
    provides: "the Reasoning Effort page and its documented llm.kimi example"
provides:
  - "51-EFFORT-SMOKE.md: the dated, scrubbed adapter-path transcript for claude-opus-4-8, deepseek-v4-pro and kimi-k3, plus the documented llm.kimi configuration answering"
  - "Measured lint ids (cargo-semver-checks 0.50.0 vs the published 0.10.1) on every Phase 51 allowlist entry, with every lint table restored byte for byte"
  - "MIGRATION.md 9.2 N rows for the additive Phase 51 surface, 9.5 reasoning-effort keys, 9.8 DR-20 to DR-26"
  - "CHANGELOG completeness for the single v0.11.0 bump"
affects: []

tech-stack:
  added: []
  patterns:
    - "copy-delete-run-restore measurement of the semver lint table, cmp-proven, before any register text is written"

key-files:
  created:
    - .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-SMOKE.md
  modified:
    - .cargo/semver-checks-allowlist.toml
    - Cargo.toml
    - crates/paladin-core/Cargo.toml
    - crates/paladin-llm/Cargo.toml
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-battalion/src/engine/cache_key.rs
    - crates/paladin-llm/src/reasoning.rs
    - crates/paladin-llm/src/conformance.rs
    - .planning/phases/51-adjustable-reasoning-effort/deferred-items.md

key-decisions:
  - "TraceEvent fires against 0.10.1 (enum_struct_variant_field_added), so its register entry stays Y and the lint is added to paladin-core's table; it is not retired to N/A."
  - "make security reports three new RustSec advisories in transitive TLS-root crates; recorded as a blocker (a gate that runs and reports a finding), not relabelled a CI gate, and the lockfile was not touched."

requirements-completed: []
requirements-progress: [REASON-03, REASON-06, REASON-07]

completed: 2026-10-22
---

# Phase 51 Plan 14: Phase closeout Summary

**The shipped adapters' effort path is proven live on web3sec's three models and the documented `llm.kimi` configuration answers; every Phase 51 semver register entry carries a lint id that cargo-semver-checks 0.50.0 reported against the published 0.10.1; MIGRATION and the CHANGELOGs are complete. Every gate is green except `make security`, which reports three new RustSec advisories in transitive TLS-root crates this phase never touched: the phase is not sealed until the owner decides how to resolve them.**

`requirements-completed` is empty on purpose: REASON-03, REASON-06 and REASON-07 are evidenced here, but REQUIREMENTS.md, STATE.md and ROADMAP.md are not touched (the orchestrator owns them), and the sealing gate (`make security`) is open.

## Checkpoint decision

Task 1 (`checkpoint:human-verify`, `gate="blocking-human"`) was resolved by the owner before dispatch. Task 1 built nothing and has no commit of its own. The answers, verbatim, dated 2026-10-22, given after the orchestrator showed the `--effort-smoke --dry-run` plan (6 adapter calls for `claude-opus-4-8`, `deepseek-v4-pro` and `kimi-k3` at the lowest and highest admitted level, plus the documented `llm.kimi` example):

- Smoke: `smoke: yes` (the owner's words: "smoke: yes"). Live adapter-path calls were authorised.
- Install: `install: yes` (the owner's words: "isntall:yes", i.e. `install: yes`). The orchestrator then ran `cargo install cargo-semver-checks --version 0.50.0 --locked`; `cargo semver-checks --version` reports 0.50.0, so the install step was `install: already present` and `cargo install` was not run again. The owner also had the orchestrator pin the tool in `.devcontainer/Dockerfile.dev` (commit `fde9bce5`, in this plan's base).

No live call was made and no install ran before these answers.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Owner approves the smoke spend and the tool install | none (resolved by the owner, recorded above) |
| 2 | Tracer: adapter-path smoke and measured register | `8ae06649` |
| 3 | MIGRATION 9.2 / 9.5 / 9.8, CHANGELOGs and every gate | `2c914cfe` |

Task 2 is a tracer; its feedback gate was consumed by the owner's checkpoint and live-run authorisation. The smoke (the tracer's `<verify>` slice) passed end to end before any expansion work: all 6 plus 2 requests returned `Ok` with finish `stop`.

## Adapter-path smoke (REASON-03, REASON-06)

Full transcript, scrub description and verdict table: `51-EFFORT-SMOKE.md` (UTC 2026-10-22, commit `4a9c290e`).

| Model | Lowest to highest admitted | Outcome | Reasoning tokens | Higher level reported more? |
|-------|----------------------------|---------|------------------|------------------------------|
| `claude-opus-4-8` | `low` to `max` | both `Ok`, finish `stop`, requested equals effective | 0 to 35 | yes |
| `deepseek-v4-pro` | `low` to `max` | both `Ok`, finish `stop`, requested equals effective | 70 to 77 | yes |
| `kimi-k3` (run 1, defaults) | `low` to `max` | both `Ok`, finish `stop`, requested equals effective | 29 to 76 | yes |
| `kimi-k3` (run 2, documented config) | `low` to `max` | both `Ok`, finish `stop` | 47 to 26 | no (recorded as measured, not re-rolled) |

- No model refused: each was admitted by its seeded row, sent and answered. One call per level is evidence that the level is accepted and reaches the model, not a statistical ordering of effort; the multi-call effect evidence is `51-EFFORT-PROBE.md`.
- **The documented `llm.kimi` example** (`reasoning_effort: low`, `max_tokens: 16000`, `timeout_seconds: 300`, passed as `--max-tokens 16000 --timeout-seconds 300`) answered: `Ok`, finish `stop`, 47 reasoning and 63 completion tokens in 4,464 ms against a 300 s timeout and a 16,000 cap. The harness's lowest admitted level for `kimi-k3` is `low`, so its first row is the page's level (no harness extension was needed). The harness prints no content length by design; non-empty content is evidenced by `Ok`, because every adapter turns empty text on any finish reason into `EmptyCompletion`. The page's example needed no correction, so `docs/src/user-guides/reasoning-effort.md` was not edited.
- **Scrub:** a scratch scanner (key shapes, bearer and authorization-style values, JWT triples, forge and cloud token prefixes, key and token query values, long mixed-case tokens, and the literal value of every credential variable in the environment and of every file under `~/.config/paladin/`, 26 values compared programmatically, never printed), shown to fire on a planted sample first, reported `clean` on both runs and on the committed file; the plan's negative key-shape grep found nothing. Credential presence was checked with `${VAR:+set}` only.

## Measured semver register (D-00b)

Tool: `cargo-semver-checks 0.50.0`, baseline `--baseline-version 0.10.1`, `--release-type minor`. For each package the `[package.metadata.cargo-semver-checks.lints]` table was removed from a copy of the manifest, the tool run, the original copied back and `cmp`-proven.

| Allowlist entry | Package, configurations | Checks | Lint(s) reported naming the item | Outcome |
|-----------------|--------------------------|--------|-----------------------------------|---------|
| `paladin-llm \| CompatRequestParameters` | `paladin-llm`, default features and the nine adapter features (both identical) | 196: 192 pass, 4 fail, 58 skip | `constructible_struct_adds_field` (`CompatRequestParameters.reasoning`, engine.rs:182) and `method_parameter_count_changed` (`CompatRequestParameters::all`, 0 parameters before, 1 now) | now two entries (one per lint); `constructible_struct_adds_field` added to `crates/paladin-llm/Cargo.toml`'s table (the other fired lints were already allowed) |
| `paladin-ai-core \| PaladinData` | `paladin-ai-core` (no features), default | 196: 185 pass, 11 fail, 58 skip | `constructible_struct_adds_field` (`PaladinData.reasoning_effort`, `.reasoning_fallback`) | entry corrected; lint already allowed, no table line |
| `paladin-ai \| AgentDefinition` | `paladin-ai`, `--default-features` and `--default-features --features llm-all` (both identical) | 196: 195 pass, 1 fail, 58 skip | `constructible_struct_adds_field` (`AgentDefinition.reasoning_effort`, `.reasoning_fallback`) | entry corrected; lint already allowed, no table line |
| `paladin-ai-core \| TraceEvent` | same `paladin-ai-core` run as `PaladinData` | as above | `enum_struct_variant_field_added` (`TraceEvent::NodeFinished`: `reasoning_requested`, `reasoning_effective`; the same lint also named the pre-Phase-51 `cost` and `halt_reason` additions) | **fires, so the row stays `Y` and is not retired to `N/A`**; `enum_struct_variant_field_added = "allow"` added to `crates/paladin-core/Cargo.toml`'s table. CI's `semver` job uses the `v0.9.0` baseline, which predates `TraceEvent`, so the allow is dormant there |

- No interim `register-only (unmeasured: ...)` entry remains (`grep` finds none); MIGRATION 9.2's four `Y` rows now carry the measured text.
- `cmp` proofs: `crates/paladin-llm/Cargo.toml` CMP_OK + `git diff --exit-code` clean; `crates/paladin-core/Cargo.toml` CMP_OK + clean; root `Cargo.toml` CMP_OK; `crates/paladin-ports/Cargo.toml` CMP_OK (also measured, see below). The only net code change to any table is the two `allow` lines named above (plus comment paragraphs).
- **`paladin-ports` was also measured** (table removed and restored, CMP_OK): 196 checks, 4 lints fired; the only Phase 51-adjacent name was the Phase 50 `LlmResponse.reasoning`. No lint named `ModelProfile`, `LlmRequest`, `LlmError::UnsupportedParameter` or the effort types, which confirms the `N` rows for them.
- **Tool caveat (not Phase 51):** `cargo semver-checks check-release --package paladin-ai` with its default every-feature build fails inside `paladin-memory`'s qdrant adapter (`missing field memory in VectorParams`, a newer qdrant-client resolved without the lockfile) before any check runs. CI runs `--default-features`, which works, and was used.
- **Observation for the v0.11.0 tag:** against 0.10.1, `paladin-ai-core` also fires lints for items other phases changed (`RunFinishStatus::AwaitingInput` discriminant, the earlier `NodeFinished.cost` and `RunFinished.halt_reason`, `PaladinResult`, `BattalionConfig.aegis` and others) that CI's v0.9.0 baseline cannot see. They are outside this plan and untouched.

## MIGRATION and CHANGELOG (REASON-07, web3sec single-bump rule)

- **9.2:** ten `N` rows under the Phase 51 `Y` rows: `ReasoningEffort` / `ReasoningFallback` (+ metadata keys), the effort types (`EffortSupport`, `EffortLevels`, `BudgetBounds`, `ResolvedEffort`, `EffortRefusal`), `ModelProfile.effort` / `effort_verified_on`, `LlmRequest.reasoning` / `reasoning_fallback`, `LlmError::UnsupportedParameter`, `ModelOverride.effort`, `ReasoningWire`, the nine `*Config` fields and builders (+ `reasoning_wire` on the two self-hosted configs), the `LlmProviderConfig` fields, `PaladinBuilder::reasoning_effort` / `reasoning_fallback`.
- **9.5:** one bullet with sub-bullets for `llm.<provider>.reasoning_effort`, `.reasoning_fallback`, `.reasoning_wire`, `llm.<provider>.models.<model-id>.effort`, `agents[].reasoning_effort`, `agents[].reasoning_fallback` (YAML shapes, default unset).
- **9.8:** DR-20 to DR-26 (explicit effort refuses unless measured or overridden; a provider default refuses on models that cannot honour it; the compat struct break; the `PaladinData` / `AgentDefinition` literal breaks; the `NodeFinished` fields; node-cache keys; nothing changes if no effort is set), each with the action a consumer takes.
- **CHANGELOGs:** the Phase 51 code entries from plans 01 to 12 were already present in the root and crate CHANGELOGs. This plan added the missing ones: the Reasoning Effort guide, ADR-0062, the harness modes and the measured register (root), the harness modes and new lint-table line (`paladin-llm`), the new lint-table line (`paladin-core`). `paladin-ports` and `paladin-battalion` already carried every Phase 51 item. `.project/current-exports.txt` needed no change (`make api-surface`: unchanged).

## Gate record

| Gate | Result |
|------|--------|
| `cargo test --offline --workspace --all-features --no-fail-fast` | exit 0; 71 targets, 9,269 passed, 0 failed, 327 ignored. (The first run, without `--no-fail-fast`, stopped at four `paladin-ai` `cancel_tests::*` cases that hit their own 30 s timeouts under full-workspace load; they passed 5 of 5 when run alone and the whole workspace passed on the second run. Load-sensitive flake, not a Phase 51 path.) |
| `cargo test --offline --workspace --all-features --doc` | exit 0; 13 targets, 711 passed, 0 failed, 212 ignored |
| `make clean-code` (fmt, clippy `-D warnings`, lint-shell, check, doc-check with zero default-feature warnings and all-feature `-D warnings`, doc tests, public-API examples) | exit 0 |
| `make check-gates` | exit 0 |
| `make api-surface` | exit 0 (4,323 items, "API surface unchanged") |
| `make check-api-examples` | exit 0 (105 entry points carry `# Examples`) |
| `make check-migration-allowlist` | exit 0 (pair sets equal; every crate-wide allow mirrored) |
| `make check-consumer-surface` | exit 0 |
| `make security` | **exit 2: BLOCKER, see below** |
| 82 % workspace line coverage (ADR-0006) | **CI gate: the `coverage` job in `.github/workflows/ci.yml`.** `cargo llvm-cov` is absent here (`error: no such command: llvm-cov`). Local proxy: the 9,269 tests and 711 doc tests above pass, every Phase 51 module carries unit tests for each public branch and a doc test per public item (`make check-api-examples`: 105 of 105), and the mapping page is drift-checked against the nine built adapters. The phase is not sealed until that job is green. |

### `make security` (blocker, never relabelled a CI gate)

`cargo audit` ran against `Cargo.lock` (691 crates, advisory DB fetched today, 1,349 advisories) and reported **3 vulnerabilities**; `cargo deny check` also reports `advisories FAILED` (bans, licenses and sources ok):

| Crate (locked) | Advisory | Published | Fixed in |
|----------------|----------|-----------|----------|
| `rustls-webpki 0.103.15` | RUSTSEC-2026-0382, certificate verification may accept distrusted intermediates | 2026-10-20 | `>=0.103.16` |
| `webpki-root-certs 1.0.8` | RUSTSEC-2026-0383, bundled roots trust distrusted emSign intermediates | 2026-10-19 | `>=1.0.10` |
| `webpki-roots 1.0.7` | RUSTSEC-2026-0381, same class | not shown | `>=1.0.10` |

Not caused by Phase 51: the advisories were published on 2026-10-19 and 2026-10-20, `git diff` of `Cargo.lock` across the whole phase is empty, and the crates come in through `rustls`/`reqwest`/`hyper-rustls`/`bollard`. `security.instructions.md` makes a new advisory a blocker unless it is explicitly allowlisted in `.cargo/audit.toml` with a reason, and `SECURITY-EXCEPTIONS.md` governs that register; allowlisting a TLS trust-store advisory is an owner governance decision, so no exception was written. A read-only `cargo update --dry-run -p rustls-webpki@0.103.15 -p webpki-roots@1.0.7 -p webpki-root-certs@1.0.8` shows the fix is a three-package patch-level lockfile bump (`0.103.16`, `1.0.10`, `1.0.10`); it was **not applied** (it changes `Cargo.lock`, outside this plan's `files_modified`, and wants a full re-test). **The phase is not sealed until the owner either applies that bump (then re-runs `make security` and the workspace tests) or allowlists the advisories with a reason.**

## Manual credential-handling review (security.instructions.md)

CodeQL's advisory scan is not coverage; this is the review that is.

- **Redact before truncate on every new error path that embeds a vendor body:** Phase 51 adds no path that embeds a vendor body in an error or log. The refusal (`LlmError::UnsupportedParameter`) is built from the provider, model, effort and a locally computed reason; the substitution `warn!` (target `paladin::reasoning`) names provider, model and the two effort spellings only. The harness (`live_vendor_smoke`) prints `diagnostic_excerpt`-style redact-then-bound excerpts of non-2xx bodies and never response content.
- **No key in any new log or `Debug`:** the new types (`ReasoningEffort`, `EffortSupport`, `ReasoningWire` and the rest) carry no credential field; the nine `*Config` types keep their manual `Debug` that prints the key as `<redacted>`. A scan of the Phase 51 diff (`b6ccecc5~1..HEAD`) for `api_key`, `Authorization`, bearer, `derive(Debug)` and log macros found no new key-bearing format string.
- **Credential-bearing clients still never follow redirects:** the Phase 51 diff adds no `reqwest::Client` construction and no redirect policy change in any adapter; each adapter's own `Policy::none()` client is untouched. The harness's raw probe client was reviewed in plan 51-02.
- **This plan's transcript:** scanned as described above; the smoke ran only after the owner's answer.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Three Phase 51 documentation and dead-code findings that made `make clean-code` fail**
- **Found during:** carried in from plan 51-13 (`deferred-items.md`); required by this plan's must-haves (`make clean-code` exits 0).
- **Fix:** `crates/paladin-battalion/src/engine/cache_key.rs` line 44: the `NODE_CACHE_KEY_VERSION` link is now path-qualified; `crates/paladin-llm/src/reasoning.rs` lines 11 and 34: the two redundant explicit link targets became plain `[`ReasoningFallback`]` / `[`ReasoningEffort`]` links; `crates/paladin-llm/src/conformance.rs`: `thinking_effort_wire_value` is gated on the three adapter features (`kimi`, `ollama`, `openai-compatible`) whose fixtures call it. These files are outside `files_modified`.
- **Commit:** `2c914cfe`

**2. [Rule 2 - Consistency] MIGRATION.md 9.2 `Y` rows updated in the Task 2 commit, not Task 3**
- The four `Y` rows' deliberate-breaking cells still read `register-only (unmeasured: ...)`, contradicting the allowlist entries the same commit corrected; they were updated with the measurement (T-51-42). Commit `8ae06649`.

### Notes against the plan's wording

- **Two entries for `CompatRequestParameters`:** the plan expected one lint per entry; the tool reported two lints naming the item, so the row carries two entries (the pair set is deduplicated, `make check-migration-allowlist` exits 0).
- **`TraceEvent` stays `Y`:** the measurement decided it fires (see the table); recorded either way as the brief required.
- **`paladin-ports` was measured too** (confirmation of the `N` rows), which the plan did not ask for.
- **No Phase 51 heading existed in 9.2**; the `N` rows sit directly under the four `Y` rows.
- **Hooks:** commits used `--no-verify` as the wave allows; the orchestrator runs the full hook chain (gitleaks included, which is not installed here) over the merged wave.

## Deferred Issues

Logged in `deferred-items.md`: an `unused import: ReasoningFallback` warning in `crates/paladin-llm/src/provider_factory.rs` line 1935 (test module `reasoning_defaults`), visible only in the default-feature `paladin-llm` lib test build (plan 51-06); `make clean-code` is unaffected.

## Authentication Gates

None. The three keys resolved from the environment.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path or schema was added; the only new files are planning evidence. T-51-40 (key in transcript) is mitigated by the scanner and grep; T-51-41 (unapproved spend) by the blocking-human checkpoint, with 8 calls made against the 6 plus the documented example shown; T-51-SC by the owner-approved, version-pinned install; T-51-42 by lint ids copied only from recorded tool output and the `cmp` proofs.

## Self-Check: PASSED

- FOUND: `.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-SMOKE.md` (contains `kimi-k3`; negative key-shape grep clean), `.cargo/semver-checks-allowlist.toml` (no `register-only (unmeasured` entry), `MIGRATION.md` (`| DR-20 |`, `llm.<provider>.reasoning_wire`).
- FOUND: commits `8ae06649` and `2c914cfe` on `worktree-agent-a81cfbdd279066ef4`.
- Open item stated above, not a self-check failure: `make security` (3 new advisories) and CI's `coverage` job gate the seal.
