---
phase: 36
slug: rustdoc-zero-warning-bar-examples-currency
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-17
validated: 2026-10-24
reconciled: "2026-10-24 by Phase 46 plan 46-04, reconcile-only (D-14/D-16): map rebuilt from the archived PLAN/SUMMARY/VERIFICATION; no test written"
---

# Phase 36 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

Reconciled 2026-10-24 by Phase 46 plan 46-04 (CURR-24, D-14/D-16/D-17) on the archived Phase 36
artifacts under `.planning/milestones/v0.10.0-phases/36-rustdoc-zero-warning-bar-examples-currency/`.
The seeded template was replaced by a map rebuilt from the thirteen `36-NN-PLAN.md` task elements (32
tasks: 31 `tracer` / `auto` and 1 `checkpoint:human-verify`), the SUMMARY frontmatter and Deviations
sections, `36-VERIFICATION.md` (5/5 truths, `status: passed`, 2026-09-18), `36-UAT.md` (59 of 59 tests
pass), `36-SECURITY.md` (`status: verified`, 0 open threats) and `36-CI-EVIDENCE.md` (real CI run
`35290763563`). Reconcile-only: workflow Steps 4-5 (the gap menu and the test-writing
`gsd-nyquist-auditor`) were skipped on purpose, because D-14 also rules that no new test is written
in Phase 46; every uncovered task would have gone to `## Gaps` instead, and none did.

Phase 36 changed rustdoc comments and `examples/` programs, not behaviour, and it proved itself with
gates: the zero-warning bar (ADR-0033, today's `make doc-check`), the public-API examples gate
(`make check-api-examples`), the examples muster (`make check-examples`, `scripts/check-all-examples.sh`),
the API-surface freeze (`make api-surface`), and `grep`/`comm` checks over `examples/README.md`. The
map cites those commands.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rustdoc as a gate (`cargo doc`, `-D warnings`) plus `cargo test --workspace --doc` doctests; example binaries built and run offline (`cargo build/run --example`); shell gates (`scripts/check-all-examples.sh`, `scripts/check-public-api-examples.sh`, `scripts/check-api-surface.sh`); `grep`/`comm` over `examples/README.md` and `36-EVIDENCE.md` |
| **Config file** | `Makefile` (`doc-check`, `check-api-examples`, `check-examples`, `api-surface`, `lint-shell`, `clean-code`), `.pre-commit-config.yaml` (`doc-check` hook at pre-push), `.github/workflows/ci.yml` ("Check documentation (all features, -D warnings)" step, Example Muster job), `Cargo.toml` `[[example]]` entries |
| **Quick run command** | `make check-api-examples` (a few seconds; 105 entry points checked on HEAD) |
| **Full suite command** | `make doc-check` (default-feature zero-warning bar, all-features `-D warnings` bar, `cargo test --workspace --doc`) plus `make check-examples` and `make api-surface` |
| **Estimated runtime** | ~8 min cold for the three-gate set on HEAD (`doc-check` is the bulk) |

---

## Sampling Rate

- **After every task commit:** the task's own `<automated>` command (per-crate `cargo doc` with zero `warning:`, per-example build and offline run, each ended with `fmt --check`, `cargo check --workspace --all-targets --all-features` and `check-api-surface.sh`)
- **After every plan wave:** `make doc-check` once plan 36-12 had added it, `make check-examples` for the example waves
- **Before `/gsd-verify-work`:** `make doc-check`, `make check-examples`, `make api-surface` green (they were, at the 2026-09-18 seal)
- **Max feedback latency:** 36 seconds for `make check-api-examples`; the cargo-doc and example-build commands are compile-bound and excused from that bound

---

## Per-Task Verification Map

Test type legend: **gate** = rustdoc / script / `make` gate; **example** = an example binary built and run offline; **gate + example** = both; **checkpoint** = a human gate decided at plan time (not a test).

Status evidence key: **seal** = green in `36-VERIFICATION.md` (2026-09-18) with its target still resolving on HEAD; **HEAD** = re-run in this session (2026-10-24). The per-task rustdoc commands scope `cargo doc` to one crate and grep the diagnostics of one module; the live equivalent on HEAD is the strictly broader `make doc-check`, which exited 0 with no `warning:` line (`/tmp/doc-output.txt` holds 0), the all-features `-D warnings` run clean and 681 doctests passed, 0 failed, 210 ignored (462 / 0 / 210 at the seal; later phases added doctests). The `check-api-surface.sh .project/current-exports.txt` tail of each command is today's `make api-surface`, which printed "API surface unchanged" (4323 items on HEAD, 3959 at the seal) on HEAD. No row below is a gap.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 36-01-01 | 01 | 1 | CURR-11, CURR-12, CURR-14, CURR-15 | T-36-01..03 | Tracer: `paladin-memory` rustdoc closed (zero warnings, `-D warnings` clean) and `token_economy_commissary` built, run offline with no provider key read, and indexed in `examples/README.md` | gate + example | `touch crates/paladin-memory/src/lib.rs && cargo doc -p paladin-memory --no-deps … && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps && cargo build --example token_economy_commissary && cargo run --example token_economy_commissary && grep -q '^### \[token_economy_commissary\.rs\]…' examples/README.md && cargo fmt --all -- --check && ./scripts/check-api-surface.sh .project/current-exports.txt` | ✅ | ✅ green (seal). HEAD: `make doc-check` exit 0; `cargo run --offline --example token_economy_commissary` with the three provider-key variables unset exit 0 ("Done -- fully offline, no provider API key was read"); README heading present |
| 36-01-02 | 01 | 1 | CURR-11, CURR-12, CURR-14, CURR-15 | T-36-01..03 | `paladin-ports` (RD-51, RD-127) and `paladin-storage` (RD-46, RD-128) rustdoc groups closed | gate | `touch crates/paladin-ports/src/lib.rs crates/paladin-storage/src/lib.rs && cargo doc -p paladin-ports -p paladin-storage --no-deps … && RUSTDOCFLAGS="-D warnings" cargo doc -p … --all-features --no-deps && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0, `make api-surface` unchanged) |
| 36-02-01 | 02 | 2 | CURR-11, CURR-12, CURR-15 | T-36-04..06 | `paladin-battalion` engine module header: the Submodules list and the module's own item links (16 groups, 34 rows) resolve | gate | `touch crates/paladin-battalion/src/lib.rs && cargo doc -p paladin-battalion --no-deps … && test "$(grep -c 'engine/mod' /tmp/36-02-t1.txt)" = 0 && cargo check -p paladin-battalion --all-features` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0, 0 `warning:` lines) |
| 36-02-02 | 02 | 2 | CURR-11, CURR-12, CURR-15 | T-36-04..06 | engine `graph`, `cache_key`, `directive_parser`, `input_mapping` (11 groups, 24 rows) resolve | gate | `… cargo doc -p paladin-battalion --no-deps … && test "$(grep -cE 'engine/(graph\|cache_key\|directive_parser\|input_mapping)' /tmp/36-02-t2.txt)" = 0 && cargo check -p paladin-battalion --all-features` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0) |
| 36-02-03 | 02 | 2 | CURR-11, CURR-12, CURR-15 | T-36-04..06 | `commander`, `edge_evaluator`, `llm_decision`, `llm_failure` (7 groups, 14 rows) resolve; `paladin-battalion` clean under both bars; the single crate commit | gate | `… cargo doc -p paladin-battalion --no-deps … 0 warning: && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && cargo test --workspace --doc && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0, doctests 681 / 0 / 210) |
| 36-03-01 | 03 | 2 | CURR-11, CURR-12, CURR-15 | T-36-07..09 | `paladin-ai-core` `container/trace` module doc header: ten unresolved links (10 groups, 20 rows) resolve | gate | `touch crates/paladin-core/src/lib.rs && cargo doc -p paladin-ai-core --no-deps … && test "$(grep -c 'container/trace' /tmp/36-03-t1.txt)" = 0 && cargo check -p paladin-ai-core --all-features` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0). The package is `paladin-ai-core`, its directory `crates/paladin-core` |
| 36-03-02 | 03 | 2 | CURR-11, CURR-12, CURR-15 | T-36-07..09 | `directive`, `structured`, `webhook` containers (4 groups, 8 rows) resolve; `paladin-ai-core` clean under both bars | gate | `… cargo doc -p paladin-ai-core --no-deps … 0 warning: && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && cargo test --workspace --doc && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0) |
| 36-04-01 | 04 | 3 | CURR-11, CURR-12, CURR-15 | T-36-10..13 | `paladin-llm`: 9 groups, 13 rows (`http_status`, `redaction`, `commissary`, compat engine, gemini adapter) resolve; the redaction-before-truncation module's doc change adds no behaviour | gate | `touch crates/paladin-llm/src/lib.rs && cargo doc -p paladin-llm --no-deps … 0 warning: && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0; `make api-surface` unchanged) |
| 36-04-02 | 04 | 3 | CURR-11, CURR-12, CURR-15 | T-36-10..13 | `paladin-web`: 8 groups, 11 rows (thread controller and dev-ui controller) resolve; `crates/paladin-web/openapi.json` regenerated for the two description edits | gate | `touch crates/paladin-web/src/lib.rs && cargo doc -p paladin-web --no-deps … 0 warning: && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && cargo test --workspace --doc && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0) |
| 36-05-01 | 05 | 3 | CURR-11, CURR-12, CURR-15 | T-36-14..16 | The five default-feature facade (`paladin-ai`) groups (RD-02..RD-06 and followers) resolve | gate | `touch src/lib.rs && cargo doc -p paladin-ai --no-deps … && test "$(grep -c 'warning:' /tmp/36-05-t1.txt)" = 0 && cargo check --workspace --all-targets --all-features` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0) |
| 36-05-02 | 05 | 3 | CURR-11, CURR-12, CURR-15 | T-36-14..16 | The two feature-gated facade groups (RD-137 under `cli`, RD-142 under `otel`) resolve; `paladin-ai` clean under both bars. 36-VERIFICATION (live run): both workspace bars exit 0 | gate | `touch src/lib.rs && cargo doc -p paladin-ai --no-deps … 0 warning: && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && cargo test --workspace --doc && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make doc-check` exit 0, all-features run with `RUSTDOCFLAGS="-D warnings"` clean) |
| 36-06-01 | 06 | 4 | CURR-13, CURR-14, CURR-15 | T-36-17..19 | `examples/war_engine_configuration.rs` (EX-62..66, EX-80) builds and runs offline with the three provider-key variables unset | example | `cargo build --example war_engine_configuration && env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example war_engine_configuration && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `cargo run --offline --example war_engine_configuration` exit 0; `make check-examples` exit 0) |
| 36-06-02 | 06 | 4 | CURR-13, CURR-14, CURR-15 | T-36-17..19 | `examples/control_flow_dynamic_routing.rs` (EX-67..70) builds and runs offline; `cargo build --examples` clean | example | `cargo build --example control_flow_dynamic_routing && env -u … cargo run --example control_flow_dynamic_routing && cargo build --examples && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD run exit 0; `cargo build --examples --offline` exit 0, re-run in plan 46-04 Task 1) |
| 36-07-01 | 07 | 4 | CURR-13, CURR-14, CURR-15 | T-36-20..23 | `examples/human_in_the_loop_gate.rs` (EX-71..73) builds and runs offline | example | `cargo build --example human_in_the_loop_gate && env -u … cargo run --example human_in_the_loop_gate && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD run exit 0) |
| 36-07-02 | 07 | 4 | CURR-13, CURR-14, CURR-15 | T-36-20..23 | `examples/graceful_shutdown.rs` (EX-74..76) builds and runs offline under `timeout 120`; `cargo build --examples` clean | example | `cargo build --example graceful_shutdown && timeout 120 env -u … cargo run --example graceful_shutdown && cargo build --examples && …` | ✅ | ✅ green (seal; HEAD: file present and built by `make check-examples` exit 0; not re-run, a signal-driven program, evidence is the seal run in `36-evidence/36-07-examples.txt`) |
| 36-08-01 | 08 | 5 | CURR-13, CURR-14, CURR-15 | T-36-24..28 | `examples/agent_runtime_middleware.rs` (EX-83..87, EX-89) builds and runs offline; the HistoryTrimmer demo is not a no-op (deviation 1) | example | `cargo build --example agent_runtime_middleware && env -u … cargo run --example agent_runtime_middleware && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD run exit 0) |
| 36-08-02 | 08 | 5 | CURR-13, CURR-14, CURR-15 | T-36-24..28 | `examples/structured_output_schema.rs` (EX-88, EX-90) builds and runs offline | example | `cargo build --example structured_output_schema && env -u … cargo run --example structured_output_schema && …` | ✅ | ✅ green (seal; HEAD run exit 0) |
| 36-08-03 | 08 | 5 | CURR-13, CURR-14, CURR-15 | T-36-24..28 | `examples/sanctum_rag_retrieval.rs` (EX-116..120) builds and runs offline with deterministic ranking (deviation 2); `cargo build --examples` clean | example | `cargo build --example sanctum_rag_retrieval && env -u … cargo run --example sanctum_rag_retrieval && cargo build --examples && …` | ✅ | ✅ green (seal; HEAD run exit 0) |
| 36-09-01 | 09 | 5 | CURR-13, CURR-14, CURR-15 | T-36-29..34 | Router parity for both HTTP-service-host files (EX-33, EX-55): `merge(agent_router).merge(thread_router).merge(run_router)` as in `paladin-server`; doc examples still compile | gate + example | `cargo build --example http_service_host --features "web-server" && env -u … cargo run --example http_service_host --features "web-server" && ./scripts/check-doc-examples.sh && mdbook build docs/ && RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-doc-examples --all-features --no-deps && …` | ✅ | ✅ green (seal; `36-VERIFICATION.md` key link). HEAD: `make check-doc-examples` exit 0 ("All included examples compile", README Quick Example in sync) and `make check-examples` exit 0 (builds `http_service_host` under `web-server`); `mdbook build` is CI evidence (`mdbook` is not installed) |
| 36-09-02 | 09 | 5 | CURR-13, CURR-14, CURR-15 | T-36-29..34 | `examples/platform_api_client.rs` (EX-77..79, EX-91..95, EX-98, EX-99, EX-104, EX-110) builds and runs under `web-server,dev-ui`, offline | example | `cargo build --example platform_api_client --features "web-server,dev-ui" && timeout 300 env -u … cargo run --example platform_api_client --features "web-server,dev-ui" && cargo build --examples && …` | ✅ | ✅ green (seal; HEAD: file present and built by `make check-examples` exit 0; the long server run is the seal run in `36-evidence/36-09-platform.txt`, not re-run) |
| 36-09-03 | 09 | 5 | CURR-13, CURR-14, CURR-15 | T-36-29..34 | `examples/webhook_receiver.rs` (EX-96, EX-97) builds and runs under `web-server`, offline; the clippy lint is fixed (deviation 1) | example | `cargo build --example webhook_receiver --features "web-server" && timeout 180 env -u … cargo run --example webhook_receiver --features "web-server" && cargo build --examples && …` | ✅ | ✅ green (seal; HEAD: file present and built by `make check-examples` exit 0; the receiver run is the seal run, not re-run) |
| 36-10-01 | 10 | 6 | CURR-13, CURR-14, CURR-15 | T-36-35..38 | `examples/node_result_cache.rs` (EX-81, EX-82) gated on `redis-cache`, build-only (no Redis is contacted) | example | `cargo build --example node_result_cache --features "redis-cache" && cargo build --examples && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (seal; HEAD `make check-examples` exit 0, which builds the `redis-cache` leg) |
| 36-10-02 | 10 | 6 | CURR-13, CURR-14, CURR-15 | T-36-35..38 | `examples/observability_tracing.rs` (EX-100..102, EX-108) runs offline; `observability_otel_export.rs` (EX-103) builds under `otel` and sends nothing | example | `cargo build --example observability_tracing && env -u … cargo run --example observability_tracing && cargo build --example observability_otel_export --features "otel" && cargo build --examples && …` | ✅ | ✅ green (seal; HEAD `observability_tracing` run exit 0; `make check-examples` exit 0 builds the `otel` leg) |
| 36-10-03 | 10 | 6 | CURR-13, CURR-14, CURR-15 | T-36-35..38 | `examples/eval_scenarios_demo.rs` (EX-105..107) builds and runs offline; `check_live_mode(false)?` no longer crashes it (deviation 1) | example | `cargo build --example eval_scenarios_demo && env -u … cargo run --example eval_scenarios_demo && cargo build --examples && …` | ✅ | ✅ green (seal; HEAD run exit 0) |
| 36-11-01 | 11 | 7 | CURR-14 | T-36-39..42 | EX-01 and EX-122: `examples/README.md` carries no drifted `PaladinResult` field names and states the workspace MSRV | gate | `test "$(grep -cE 'response\.content\|response\.token_usage\|response\.execution_time([^_]\|$)' examples/README.md)" = 0 && RUSTV=$(grep -m1 '^rust-version' Cargo.toml …) && grep -q "Rust $RUSTV" examples/README.md` | ✅ | ✅ green (HEAD re-run as written: 0 drifted names, `rust-version = "1.88"`, "Rust 1.88" present) |
| 36-11-02 | 11 | 7 | CURR-14 | T-36-39..42 | EX-121: sections and Table-of-Contents entries for the eleven previously unlisted programs | gate | `grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]' examples/README.md … > /tmp/36-11-listed.txt && printf '%s\n' commander_council.rs … war_engine_memory_baseline.rs … > /tmp/36-11-eleven.txt && test -z "$(comm -23 /tmp/36-11-eleven.txt /tmp/36-11-listed.txt)"` | ✅ | ✅ green (HEAD re-run as written: `comm -23` empty) |
| 36-11-03 | 11 | 7 | CURR-14 | T-36-39..42 | Sections for the fourteen programs this phase added; README and `examples/*.rs` agree in both directions; one `**Demonstrates:**` line per program | gate | `ls examples/*.rs … > /tmp/36-11-ondisk.txt && … && test -z "$(comm -23 ondisk listed)" && test -z "$(comm -13 ondisk listed)" && test "$(grep -c '^\*\*Demonstrates:\*\*' examples/README.md)" = "$(wc -l < /tmp/36-11-ondisk.txt)"` | ✅ | ✅ green (HEAD re-run as written: 62 on disk, 62 listed, both `comm` empty, 62 `Demonstrates:` lines) |
| 36-12-01 | 12 | 7 | CURR-11, CURR-12, CURR-13, CURR-15 | T-36-43..47 | `scripts/check-all-examples.sh` mirrors the CI Example Muster (no bare `cargo check --example`) and is exposed as `make check-examples` | gate | `make lint-shell && test "$(grep -c 'cargo check --example' scripts/check-all-examples.sh)" = 0 && grep -q 'check-examples:' Makefile && make check-examples` | ✅ | ✅ green (HEAD re-run as written: `make lint-shell` exit 0 "shellcheck clean", 0 `cargo check --example`, target present, `make check-examples` exit 0 with "All 62 example binaries present") |
| 36-12-02 | 12 | 7 | CURR-11, CURR-12, CURR-13, CURR-15 | T-36-43..47 | The single gate-wiring commit: `make doc-check`, its `clean-code` dependency, the pre-push hook, the CI all-features documentation step and the closing zero measurement. WINDOWS row 51 (eight crate-level rustdoc `allow` attributes, 309 hidden diagnostics) is `waived`, cited here and not re-opened | gate | `grep -q '^doc-check:' Makefile && grep -qE '^clean-code: .*doc-check' Makefile && grep -q 'id: doc-check' .pre-commit-config.yaml && grep -q 'Check documentation (all features, -D warnings)' .github/workflows/ci.yml && make doc-check && make check-examples && cargo fmt --all -- --check && cargo check --workspace --all-targets --all-features && ./scripts/check-api-surface.sh …` | ✅ | ✅ green (HEAD re-run: the four greps match, `make doc-check` exit 0, `make check-examples` exit 0, `make api-surface` unchanged). The eight `#![allow(rustdoc::…)]` attributes are still present at `src/lib.rs:117-119`, `paladin-llm/src/lib.rs:49`, `paladin-ports/src/lib.rs:51-52`, `paladin-storage/src/lib.rs:20` and `paladin-notifications/src/lib.rs:14` (WINDOWS 51, owner: a later rustdoc-suppressions phase) |
| 36-13-01 | 13 | 8 | CURR-11, CURR-12, CURR-14 | T-36-48..51 | `36-EVIDENCE.md` closure map tiles RD-01..RD-143 with no row missing | gate | `EV=…/36-EVIDENCE.md; for i in $(seq -w 1 143); do grep -q "RD-$i" "$EV" \|\| { echo MISSING; exit 1; }; done` | ✅ | ✅ green (seal). HEAD: the literal `seq -w 1 143` loop is ⚠ miscalibrated (it zero-pads to `RD-001`, 99 spurious misses, recorded in 36-13-SUMMARY deviation 1); the correctly padded loop (`RD-01`..`RD-99`, `RD-100`..`RD-143`) reports no missing row |
| 36-13-02 | 13 | 8 | CURR-11, CURR-12, CURR-14 | T-36-48..51 | WINDOWS rows 36 and 37 resolved through `gsd-tools` (`fixed`); CHANGELOG Documentation bullets carry no `RD-nn` / `EX-nn` identifiers | gate | `test "$(grep -cE 'RD-[0-9]+\|EX-[0-9]+' CHANGELOG.md)" = 0 && test "$(grep -c '^### Documentation' CHANGELOG.md)" = 1 && test "$(grep -cE '^\| 3[67] \|.*\| open \|' .planning/WINDOWS.md)" = 0 && test "$(grep -cE '^\| 3[67] \|.*\| fixed \|' .planning/WINDOWS.md)" = 2` | ✅ | ✅ green (seal, recalibrated). HEAD: 0 identifiers in `CHANGELOG.md`, 0 `open` and 2 `fixed` rows for 36 and 37. The `= 1` heading count is recalibrated, not a regression: `CHANGELOG.md` carries two `### Documentation` headings file-wide (one per released section), as 36-13-SUMMARY deviation 2 recorded at the seal |
| 36-13-03 | 13 | 8 | CURR-11, CURR-12, CURR-14 | T-36-48..51 | Checkpoint (`checkpoint:human-verify`, `gate="blocking"`): the branch was pushed by the maintainer and the real CI run recorded in `36-CI-EVIDENCE.md` | checkpoint | — | — | checkpoint (a plan-time human gate, not a test). Outcome per 36-13-SUMMARY and `36-CI-EVIDENCE.md`: run `35290763563`, jobs `Code Quality`, `Unit Tests (stable)`, `Unit Tests (beta)` and `Example Muster (Feature Matrix)` all `success`, no CI-versus-local disagreement on any of the four gates |

*Status: ⬜ pending · ✅ green · ⚠ miscalibrated or superseded (reason and equivalent named) · ❌ gap · checkpoint*

**Requirement roll-up**

| Requirement | Tasks | Automated & green | Notes |
|---|---|---|---|
| CURR-11 | 36-01-01..02, 36-02-01..03, 36-03-01..02, 36-04-01..02, 36-05-01..02, 36-12-01..02, 36-13-01..03 | 15/15 automated (+1 checkpoint) | zero-warning default bar: `make doc-check` step 1 exit 0 on HEAD |
| CURR-12 | same 16 tasks | 15/15 automated (+1 checkpoint) | all-features `-D warnings` bar and the gate wiring: `make doc-check` exit 0; CI step still present in `ci.yml`; WINDOWS 51 waived and cited |
| CURR-13 | 36-06-01..02, 36-07-01..02, 36-08-01..03, 36-09-01..03, 36-10-01..03, 36-12-01..02 | 15/15 | `make check-examples` exit 0 (62 of 62 binaries), seven examples re-run offline on HEAD |
| CURR-14 | 36-01-01..02, 36-06-01 .. 36-10-03, 36-11-01..03, 36-13-01..03 | 20/20 automated (+1 checkpoint) | README cross-checks re-run as written: 62 on disk = 62 listed |
| CURR-15 | 36-01-01..02, 36-02-01 .. 36-05-02, 36-06-01 .. 36-10-03, 36-12-01..02 | 26/26 | `make api-surface` exit 0, "API surface unchanged" (4323 items) |

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Phase 36 authored no test framework; the gates it relies on all exist on HEAD:

- [x] `Makefile` targets `doc-check`, `check-api-examples`, `check-examples`, `api-surface`, `lint-shell` (the first and third added by plan 36-12)
- [x] `scripts/check-all-examples.sh` (rewritten by plan 36-12), `scripts/check-public-api-examples.sh`, `scripts/check-api-surface.sh`
- [x] `.pre-commit-config.yaml` `doc-check` hook and the `ci.yml` "Check documentation (all features, -D warnings)" step
- [x] `examples/README.md` (62 sections) and the 14 new `examples/*.rs` programs; `36-EVIDENCE.md`, `36-CI-EVIDENCE.md`, `36-evidence/`

---

## Manual-Only Verifications

All phase behaviors have automated verification. Two items rest on a recorded outside run rather than a local re-run, and none is a sign-off a test could replace:

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The new all-features documentation step and the Example Muster job go green on a real pushed branch | CURR-12, CURR-13 (36-13-03) | A pushed CI run cannot be produced from the sandbox; the push was the maintainer's. Closed: `36-CI-EVIDENCE.md`, run `35290763563` | Evidence: `gh run view 35290763563 --json jobs`; `make doc-check` and `make check-examples` are the local mirrors, both exit 0 on HEAD |
| `mdbook build docs/` after the router-parity edit to `http_service_host` | CURR-14 (36-09-01) | `mdbook` is not installed in the sandbox | CI evidence: `36-VERIFICATION.md` and `36-CI-EVIDENCE.md`; re-run with `mdbook build docs` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (31 automated; 1 plan-time human checkpoint, 36-13-03, which is not a test)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none were missing)
- [x] No watch-mode flags
- [x] Feedback latency < 36s for `make check-api-examples` (compile-bound `cargo doc` and example builds excused, see Sampling Rate)
- [x] `nyquist_compliant: true` set in frontmatter: every requirement id has a green automated row, no row is a gap, and the two miscalibrated verify literals (36-13-01, 36-13-02) are recorded with their corrected form

**Approval:** validated 2026-10-24 (Phase 46 plan 46-04, reconcile-only) -- 31 automated rows, 1 checkpoint, 0 gaps

---

## Validation Audit 2026-10-24

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

- **Re-run on HEAD 2026-10-24 (Phase 46 plan 46-04):** `make doc-check` (exit 0; default-feature run with 0 `warning:` lines, all-features `-D warnings` run clean, `cargo test --workspace --doc` 681 passed / 0 failed / 210 ignored), `make check-api-examples` (exit 0; "All 105 D-05 public API entry points carry a plural '# Examples' heading"), `make check-examples` (exit 0; "All 62 example binaries present"), `make api-surface` (exit 0; 4323 items, unchanged), `make lint-shell` (exit 0), `make check-doc-examples` (exit 0), and `cargo run --offline --example <name>` with `OPENAI_API_KEY`, `ANTHROPIC_API_KEY` and `DEEPSEEK_API_KEY` unset for `token_economy_commissary`, `war_engine_configuration`, `control_flow_dynamic_routing`, `human_in_the_loop_gate`, `agent_runtime_middleware`, `structured_output_schema`, `sanctum_rag_retrieval`, `observability_tracing` and `eval_scenarios_demo` (all exit 0), plus the `36-11-*`, `36-12-*` and `36-13-*` grep and `comm` commands as written. The long-running `graceful_shutdown`, `platform_api_client` and `webhook_receiver` runs were not repeated (they bind sockets or wait on signals); their evidence is the seal-time capture and a green build.
- **Drift since the seal, none a failure:** doctests 462 to 681, API surface 3959 to 4323 items and the README example count unchanged at 62; per-task commands that name `/tmp/36-NN-*.txt` captures are now scoped by the broader `make doc-check`.
- **Environment note:** a full sandbox disk made one earlier build fail with `ld terminated with signal 7 [Bus error]` (recorded in `34-VALIDATION.md`); after the git-ignored `target/debug/incremental` cache was removed every command above ran clean with `CARGO_INCREMENTAL=0`. No tracked file was touched.
- **Not spawned:** `gsd-nyquist-auditor`, and no test, doc page or implementation file was written (D-14).

## Gaps

None.
