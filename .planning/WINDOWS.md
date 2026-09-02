---
schema_version: 1
open_count: 16
waived_count: 4
fixed_count: 5
total_count: 25
last_updated: 2026-09-02T18:04:11.984Z
---

# Broken Windows Ledger

> Cross-phase defect register. `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | unmet-truth | .planning/ledgers/milestone-01.md |  | REQ-battalion-result-v1 (Epic 4 FR-4.2, cited in ADR-0002's Considered Options as 'superseded by the shipped superset') has no row anywhere in the Milestone 1 ledger's Epic 4 table, even though REQUIREMENTS.md's original ledger body carried it as 'Variant (group 4)'. Plan 01-08 Task 2's subset-check safety gate caught this and HALTED per the plan's explicit instruction rather than reducing REQUIREMENTS.md's Milestone 1 body to a pointer at an incomplete destination. | fixed |  | 2026-07-31T13:22:57.385Z | 2026-07-31T14:46:37.492Z |
| 2 | 03 | deviation | crates/paladin-storage/src/redis.rs |  | Live-server code paths of redis.rs (everything reaching through self.conn) remain uncovered by unit tests; deferred with reason, owner Phase 15 (PIPE), exerciser tests/integration/redis_queue_integration_test.rs (requires Docker) | open |  | 2026-08-02T15:41:28.892Z |  |
| 3 | 07 | deviation | .project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md |  | Task 3's requested single combined commit for ADR-0016 + PRD annotation was split into two atomic commits (9e8db80, 71ea46e) per standard task_commit_protocol; both files present, no content impact. | open |  | 2026-08-06T18:09:04.871Z |  |
| 4 | 07 | deviation | .project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md |  | No fabricated 3rd strikethrough correction for CONTEXT.md D-08(5)'s anticipated section-1 Milestone 1/Epic 2 cross-reference — re-verified absent from live tree (matches ADR-0014's own flagged drift); acceptance criterion expecting >=3 strikethrough lines not met by design. | open |  | 2026-08-06T18:09:08.207Z |  |
| 5 | 14 | unrun-verify | Cargo.toml |  | cargo test --workspace not run to completion for 14-01: system-wide disk exhaustion (830G/875G used, 0 avail on /workspace mount) blocked full workspace compile; targeted plan <verify> commands (paladin-ai lib config::agents, paladin-web full suite, paladin-server binary build, openapi drift guard, check-api-surface.sh) all passed | open |  | 2026-08-12T16:51:08.832Z |  |
| 6 | 14 | unrun-verify | N/A (workspace-wide) |  | 14-04: full 'cargo test --workspace' not run — shared /workspace mount at 99%25 (13G free), matching 14-01's documented disk-exhaustion condition; the plan's own targeted verify (cargo test --bin paladin-server --features web-server, cargo fmt --check, cargo clippy --all-targets --features web-server -- -D warnings) all ran to completion and passed | open |  | 2026-08-12T17:13:58.989Z |  |
| 7 | 14 | deviation | CHANGELOG.md |  | 14-08's acceptance criterion expected >=2 'BREAKING' lines under the dated 0.8.0 section in root CHANGELOG.md; only 1 is present. 14-01 split the phase's two consumer-break BREAKING entries across root CHANGELOG.md (config-key rename) and crates/paladin-web/CHANGELOG.md (AgentAuthConfig field + OpenAPI scheme rename), one per file, per 14-01-SUMMARY.md's own D4 verification and this plan's own instruction to leave per-crate changelogs untouched. Both breaks are documented with a BREAKING entry and cite ADR-0040; only the single-file grep count in the plan's acceptance criteria was miscalibrated. | open |  | 2026-08-12T18:05:57.086Z |  |
| 8 | 15.1 | unrun-verify | SECURITY-EXCEPTIONS.md |  | Plan 15.1-01 Task 2's inline verify python one-liner (block-split regex over the machine-readable register) fails with a pre-existing TOML parse error on the LAST exception block, because its lookahead doesn't stop before the trailing markdown code fence -- reproduced against the pre-edit file too, unrelated to this task's new row. Substituted an isolated per-block parse of just the new RUSTSEC-2026-0249 row (11/11 fields present) plus the real repo guard scripts/check-advisory-register.sh (exit 0) as equivalent proof. | open |  | 2026-08-14T00:49:21.261Z |  |
| 9 | 15.1 | unrun-verify | .github/workflows/ci.yml |  | Task 1 acceptance criterion 'git diff \| grep -c "^[+-].*cargo "' returns 4 not 0 -- matches step *name* text ('Cache cargo registry' etc.) removed by the migration, not actual cargo invocations. Verified via 'run: cargo' scoped grep returning 0 changed invocations. | open |  | 2026-08-14T14:22:48.884Z |  |
| 10 | 15.1 | unrun-verify | .github/workflows/integration-tests.yml |  | Task 2's first automated verify literally asserts survivors=={pre-commit.yml} after migration, but integration-tests.yml (3 hand-rolled cache blocks) is still present -- deletion is plan 15.1-05's job, not yet executed in this wave, exactly per this plan's own Recorded discretion resolutions section. Substituted an assertion expecting survivors=={pre-commit.yml, integration-tests.yml}, both counts matching (1 and 3 respectively). | open |  | 2026-08-14T14:22:56.039Z |  |
| 11 | 15.1 | unrun-verify | .github/workflows/ci.yml |  | Task 2 acceptance criterion 'grep -rc restore-keys ci.yml feature-flags.yml release.yml' returns 0 for ci.yml -- returns 2, both from pre-existing prose comments in the examples job (added by plan 15.1-01, lines ~268/271) explaining why a restore-keys fallback alone is insufficient, not an actual YAML restore-keys: key. Verified via structural YAML walk: no step's with block contains a restore-keys key in any of the three files. | open |  | 2026-08-14T14:23:03.057Z |  |
| 12 | 17 | unrun-verify | tests/integration/ollama_docker_test.rs |  | Ollama Docker-gated Tier 2 suite (17-07 Task 2) authored and proven to compile/clippy-clean/skip-gracefully, but never run against a real Ollama server -- no Docker daemon in the execution sandbox. Runtime behavior (generate/generate_stream/get_available_models/validate_model against real qwen2.5:0.5b) is unverified. | fixed | Resolved 2026-08-23 by a CI run on the pushed branch. The orchestrator queried the GitHub check-runs API directly for commit 76b859d (the SHA it pushed) rather than relying on a report: 44 checks success, 3 skipped, 0 failures. This is first-hand evidence at CURRENT HEAD, which is what these rows lacked -- the previously cited run was at ca211644 (2026-08-19), before ~2,160 lines of gap-closure code landed. The 'Ollama Integration Tests (live server)' job concluded success (completed 2026-08-23T16:55:44Z), exercising the Docker-gated Tier 2 suite this row recorded as unrun. | 2026-08-17T14:17:30.134Z | 2026-08-23T17:55:00.000Z |
| 13 | 17 | unrun-verify | Makefile |  | 17-07 Task 3: the workspace 82% line-coverage gate (make coverage) could not be run in this execution sandbox -- Redis (6380) and MinIO (9010) are unreachable because no Docker daemon is available, and the coverage target's own preflight fails fast on both. The coverage percentage with all six new adapters counted is UNMEASURED, not failing. cargo doc -p paladin-llm --no-deps (0 missing-docs warnings under the six new features) and a scoped clippy pass on touched targets were verified instead. | fixed | Resolved 2026-08-23 by a CI run on the pushed branch. The orchestrator queried the GitHub check-runs API directly for commit 76b859d (the SHA it pushed) rather than relying on a report: 44 checks success, 3 skipped, 0 failures. This is first-hand evidence at CURRENT HEAD, which is what these rows lacked -- the previously cited run was at ca211644 (2026-08-19), before ~2,160 lines of gap-closure code landed. The 'Coverage' job concluded success (completed 2026-08-23T16:59:08Z). That job runs `cargo llvm-cov --fail-under-lines 82`, so a success conclusion IS the >=82% workspace line-coverage assertion (ADR-0006) holding against the gap-closure code -- the exact measurement this row recorded as unrun. The job emitted no percentage into its check-run output, so the pass/fail verdict is recorded here rather than a figure. | 2026-08-17T14:17:37.112Z | 2026-08-23T17:55:00.000Z |
| 14 | 17 | deviation | docker/docker-compose.test.yml |  | 17-07 Task 2: ollama-test healthcheck uses 'ollama list' (native /api/tags) instead of the plan's preferred curl-based /v1/models check, because curl/wget availability in the ollama/ollama:0.3.14 base image could not be verified without Docker in this sandbox. 'ollama list' is a well-precedented dependency-free healthcheck for this exact image. Compose file syntax validated via python yaml.safe_load only -- 'docker compose config' itself was never run. | open |  | 2026-08-17T14:17:46.408Z |  |
| 15 | 17 | unrun-verify | crates/paladin-llm/src/gemini/adapter.rs |  | Snyk code scan (per snyk_rules.instructions.md) could not be run — no Snyk MCP tool or CLI available in this worktree's runtime (no network egress); recorded as not-run, never as passed | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-17T19:33:52.477Z | 2026-08-19T13:55:48.872Z |
| 16 | 17 | unrun-verify | crates/paladin-llm/src/compat/engine.rs,crates/paladin-llm/src/kimi/adapter.rs,crates/paladin-llm/src/qwen/adapter.rs,crates/paladin-llm/src/grok/adapter.rs,crates/paladin-llm/src/ollama/adapter.rs,crates/paladin-llm/src/gemini/adapter.rs |  | Plan 17-10 verification step 7 (Snyk code scan over the five modified WR-04 adapter files plus compat/engine.rs) was not run — snyk_code_scan MCP tool unavailable in the executor runtime | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-17T20:01:16.281Z | 2026-08-19T13:55:49.213Z |
| 17 | 17 | unrun-verify | crates/paladin-llm/src/gemini/adapter.rs |  | Plan 17-11's verification step 7 (Snyk code scan over crates/paladin-llm/src/gemini/adapter.rs) was not run -- no snyk_code_scan MCP tool and no Snyk CLI were available in the executor's runtime (no network egress). 17-11's own SUMMARY.md recorded this as not-run, never as passed. This row is the sibling of ids 15 and 16, filed 2026-08-18 by plan 17-17 after 17-VERIFICATION.md flagged 17-11's row as missing. | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-18T02:10:42.462Z | 2026-08-19T13:55:49.665Z |
| 18 | 17 | unrun-verify | crates/paladin-llm/src/provider_factory.rs,tests/unit/llm/provider_factory_test.rs,crates/paladin-llm/src/openai_compatible/adapter.rs,crates/paladin-llm/src/gemini/adapter.rs,crates/paladin-llm/src/compat/engine.rs |  | Plans 17-12 through 17-16 each attempted the mandated Snyk code scan (per snyk_rules.instructions.md, imported into CLAUDE.md) over the files they modified; none could run -- no snyk_code_scan MCP tool and no Snyk CLI in this environment. All five SUMMARYs (17-12-SUMMARY.md, 17-13-SUMMARY.md, 17-14-SUMMARY.md, 17-15-SUMMARY.md, 17-16-SUMMARY.md) record their scans as not-run, never as passed. Filed 2026-08-18 by plan 17-17. | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-18T02:10:55.983Z | 2026-08-19T13:55:50.375Z |
| 19 | 17 | deviation | .project/current-exports.txt |  | The public-API surface snapshot .project/current-exports.txt was regenerated under default features only, so KimiAdapter, QwenAdapter, GrokAdapter, OllamaAdapter, GeminiAdapter and OpenAiCompatibleAdapter do not appear in it and cannot be checked for public-API drift. This is consistent with D-11's unchanged default feature set and is not itself wrong. 17-REVIEW.md records it as IN-01, non-blocking, with the suggested follow-up of generating an --features llm-all variant or documenting its absence. It was excluded from the 2026-08-18 gap-closure scope by explicit developer decision, taken in an interactive AskUserQuestion checkpoint in the orchestrating /gsd-plan-phase 17 --gaps session -- a recorded human choice, not an --auto inference -- and is therefore carried forward as accepted debt (IN-01) rather than dropped. Filed 2026-08-18 by plan 17-17. | open |  | 2026-08-18T02:11:05.490Z |  |
| 20 | 17 | deviation | crates/paladin-llm/src/gemini/adapter.rs |  | Live vendor smoke run for plan 17-18 (2026-08-22) found Gemini's generate() probe FAILS on GEMINI_DEFAULT_MODEL=gemini-2.5-flash: vendor error 'This model models/gemini-2.5-flash is no longer available to new users. Please update your code to use models/gemini-3.6-flash'. Model-list probe still PASSES (model present in live catalog), so this is a vendor-side default-model deprecation, not a regression from 17-18's CompatRequestParameters change -- Gemini is not built on CompatEngine at all and is structurally unaffected by it. Confirmed identical before and after this plan's code changes. Out of scope for 17-18 (gemini/adapter.rs not in files_modified); candidate follow-up: refresh GEMINI_DEFAULT_MODEL similarly to this plan's Grok refresh. | fixed | Fixed by the orchestrator between waves 1 and 2 of /gsd-execute-phase 17 --gaps-only (commit 954b750). GEMINI_DEFAULT_MODEL -> gemini-3.6-flash and GEMINI_FALLBACK_MODELS -> [gemini-3.6-flash, gemini-3.5-flash], every entry verified by a live generateContent call on 2026-08-22 rather than taken from the vendor deprecation message on faith. No pro-family fallback entry: gemini-2.5-pro and gemini-3-pro-preview are retired, gemini-3.6-pro is absent from v1beta, and gemini-pro-latest / gemini-3.1-pro-preview returned quota errors on the available credential -- an unverified identifier is what this refresh exists to remove. Escalated out of follow-up status because plans 17-19, 17-21 and 17-22 each carry a must_have requiring Gemini to PASS both live probes, and 17-22 requires four vendors PASS; leaving it open would have made three downstream must_haves unachievable. Live harness after the fix: Grok PASS/PASS, Gemini PASS/PASS. | 2026-08-22T16:29:02.316Z | 2026-08-22T16:52:00.000Z |
| 21 | 17 | unrun-verify | crates/paladin-llm/src/qwen/adapter.rs |  | Plan 17-21 Task 2 is BLOCKED on an Alibaba Cloud Model Studio account entitlement, not on any code defect. After Task 1 moved QWEN_DEFAULT_BASE_URL to the US (Virginia) compatible-mode endpoint, the credential authenticates there correctly -- GET /models returns 92 entries with qwen-plus present, versus invalid_api_key at the previous dashscope-intl (Singapore) default, which is the measurement that proves the reversal right. But every chat-completion invocation returns HTTP 403 {"code":"Model.AccessDenied"}. The plan's executor ruled out a stale-identifier explanation across 78 qwen-prefixed identifiers and their -us regional variants, two unrelated model families hosted on the same workspace (deepseek-v4-flash, glm-5.1), and both the OpenAI-compatible and native DashScope invocation endpoints; the orchestrator independently reproduced the same 403 on qwen-plus. Consequence: the Qwen generate() probe cannot PASS, so plan 17-21's remaining must_haves (QWEN_FALLBACK_MODELS refreshed from a live-measured catalog, the five sampling-parameter verdicts, both temperature_range endpoints) are unmeasurable, and plan 17-22's 'four vendors PASS' clause is unachievable. Note that 17-21-SUMMARY.md exists with frontmatter status: blocked, but phase-plan-index keys off file EXISTENCE, so 17-21 reads as complete to the index and will be skipped by a plain --gaps-only re-run. It is deliberately NOT marked complete in ROADMAP.md. Required human action: in the Model Studio console, for the workspace tied to DASHSCOPE_API_KEY, select US (Virginia) and activate model invocation for at least qwen-plus, clearing whatever billing/quota/terms gate the console surfaces -- the API returns only the generic Model.AccessDenied code. Verify with: cargo run -p paladin-llm --example live_vendor_smoke --features kimi,qwen,grok,gemini (DASHSCOPE_BASE_URL left unset); Qwen's generate line should read PASS. Filed 2026-08-22 by the /gsd-execute-phase 17 --gaps-only orchestrator; the developer chose to continue waves 4 and 5 with Qwen recorded as catalog-verified / invocation-blocked rather than wait. | fixed | Resolved 2026-08-23, externally: the operator's DASHSCOPE_API_KEY was replaced with a Singapore-scoped credential (the entitlement-blocked key was Virginia-scoped and workspace-specific). Against the new key and the corrected shipped default (dashscope-intl, Singapore -- plan 17-21 gap closure), every measured request succeeded: GET /models returned 162 entries, generate() returned real completions for qwen-plus and candidate qwen3.7-plus, and all five optional sampling parameters plus both temperature_range endpoints were probed individually with no rejection below DashScope's documented [0.0, 2.0) temperature ceiling. No code change resolved this row -- it was never a code defect -- but the previously-blocked live_vendor_smoke run now exits 0 with all four vendors (Kimi, Qwen, Grok, Gemini) PASSING both probes and no DASHSCOPE_BASE_URL override, closing plan 17-22's 'four vendors PASS' clause. See 17-21-SUMMARY.md's 2026-08-23 update for the full measurement record. [Ledger normalization 2026-08-23: this row was originally written with kind "blocker" and status "resolved", neither of which is in the WINDOWS.md schema vocabulary (kinds: stub\|todo\|fixme\|skipped-test\|lint-warning\|unmet-truth\|unrun-verify\|deviation; statuses: open\|waived\|fixed). The off-schema values made the whole ledger unreadable to gsd-tools. Reclassified to kind=unrun-verify (a live-vendor verification that could not be executed) and status=fixed; the substance of the record is unchanged.] | 2026-08-22T18:05:00.000Z | 2026-08-23T12:55:15.198Z |
| 22 | 22 | unrun-verify | crates/paladin-storage/src/waypoint/postgres.rs |  | Postgres Tier 2 contract-suite pass against a real postgres-test service is unverified (no Docker in the execution environment); compile, lint, and the clean-skip path are proven. Run make test-integration-docker to close. | open |  | 2026-09-02T00:00:49.385Z |  |
| 23 | 22 | deviation | crates/paladin-battalion/src/engine/bridges.rs |  | from_campaign extends the ENG-FR-19 default three-field schema with one dedicated LastWrite field per Paladin (not literally exactly three fields for this constructor) so a general DAG's concurrent fan-out siblings never hit a DispatchConflict; from_formation/from_phalanx remain exactly three fields as specified | open |  | 2026-09-02T03:29:38.826Z |  |
| 24 | 22 | deviation | tests/integration/e2e_crash_resume_test.rs | 112 | loop_gate self-loop node made a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property, rather than fixed structurally; flagged for plan 22-16's fixture audit (acceptance 2a) | open |  | 2026-09-02T18:04:03.616Z |  |
| 25 | 22 | deviation | crates/paladin-battalion/src/engine/superstep.rs | 1220 | self_loop_graph test helper makes its looping node a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property (same root cause as e2e_crash_resume_test.rs); flagged for plan 22-16's fixture audit (acceptance 2a) | open |  | 2026-09-02T18:04:11.984Z |  |

````json
[
  {
    "id": 1,
    "kind": "unmet-truth",
    "phase": "01",
    "file": ".planning/ledgers/milestone-01.md",
    "line": null,
    "description": "REQ-battalion-result-v1 (Epic 4 FR-4.2, cited in ADR-0002's Considered Options as 'superseded by the shipped superset') has no row anywhere in the Milestone 1 ledger's Epic 4 table, even though REQUIREMENTS.md's original ledger body carried it as 'Variant (group 4)'. Plan 01-08 Task 2's subset-check safety gate caught this and HALTED per the plan's explicit instruction rather than reducing REQUIREMENTS.md's Milestone 1 body to a pointer at an incomplete destination.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-31T13:22:57.385Z",
    "resolved_at": "2026-07-31T14:46:37.492Z"
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "03",
    "file": "crates/paladin-storage/src/redis.rs",
    "line": null,
    "description": "Live-server code paths of redis.rs (everything reaching through self.conn) remain uncovered by unit tests; deferred with reason, owner Phase 15 (PIPE), exerciser tests/integration/redis_queue_integration_test.rs (requires Docker)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-02T15:41:28.892Z",
    "resolved_at": null
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "07",
    "file": ".project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md",
    "line": null,
    "description": "Task 3's requested single combined commit for ADR-0016 + PRD annotation was split into two atomic commits (9e8db80, 71ea46e) per standard task_commit_protocol; both files present, no content impact.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-06T18:09:04.871Z",
    "resolved_at": null
  },
  {
    "id": 4,
    "kind": "deviation",
    "phase": "07",
    "file": ".project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md",
    "line": null,
    "description": "No fabricated 3rd strikethrough correction for CONTEXT.md D-08(5)'s anticipated section-1 Milestone 1/Epic 2 cross-reference — re-verified absent from live tree (matches ADR-0014's own flagged drift); acceptance criterion expecting >=3 strikethrough lines not met by design.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-06T18:09:08.207Z",
    "resolved_at": null
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "14",
    "file": "Cargo.toml",
    "line": null,
    "description": "cargo test --workspace not run to completion for 14-01: system-wide disk exhaustion (830G/875G used, 0 avail on /workspace mount) blocked full workspace compile; targeted plan <verify> commands (paladin-ai lib config::agents, paladin-web full suite, paladin-server binary build, openapi drift guard, check-api-surface.sh) all passed",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-12T16:51:08.832Z",
    "resolved_at": null
  },
  {
    "id": 6,
    "kind": "unrun-verify",
    "phase": "14",
    "file": "N/A (workspace-wide)",
    "line": null,
    "description": "14-04: full 'cargo test --workspace' not run — shared /workspace mount at 99%25 (13G free), matching 14-01's documented disk-exhaustion condition; the plan's own targeted verify (cargo test --bin paladin-server --features web-server, cargo fmt --check, cargo clippy --all-targets --features web-server -- -D warnings) all ran to completion and passed",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-12T17:13:58.989Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "deviation",
    "phase": "14",
    "file": "CHANGELOG.md",
    "line": null,
    "description": "14-08's acceptance criterion expected >=2 'BREAKING' lines under the dated 0.8.0 section in root CHANGELOG.md; only 1 is present. 14-01 split the phase's two consumer-break BREAKING entries across root CHANGELOG.md (config-key rename) and crates/paladin-web/CHANGELOG.md (AgentAuthConfig field + OpenAPI scheme rename), one per file, per 14-01-SUMMARY.md's own D4 verification and this plan's own instruction to leave per-crate changelogs untouched. Both breaks are documented with a BREAKING entry and cite ADR-0040; only the single-file grep count in the plan's acceptance criteria was miscalibrated.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-12T18:05:57.086Z",
    "resolved_at": null
  },
  {
    "id": 8,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": "SECURITY-EXCEPTIONS.md",
    "line": null,
    "description": "Plan 15.1-01 Task 2's inline verify python one-liner (block-split regex over the machine-readable register) fails with a pre-existing TOML parse error on the LAST exception block, because its lookahead doesn't stop before the trailing markdown code fence -- reproduced against the pre-edit file too, unrelated to this task's new row. Substituted an isolated per-block parse of just the new RUSTSEC-2026-0249 row (11/11 fields present) plus the real repo guard scripts/check-advisory-register.sh (exit 0) as equivalent proof.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-14T00:49:21.261Z",
    "resolved_at": null
  },
  {
    "id": 9,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": ".github/workflows/ci.yml",
    "line": null,
    "description": "Task 1 acceptance criterion 'git diff | grep -c \"^[+-].*cargo \"' returns 4 not 0 -- matches step *name* text ('Cache cargo registry' etc.) removed by the migration, not actual cargo invocations. Verified via 'run: cargo' scoped grep returning 0 changed invocations.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-14T14:22:48.884Z",
    "resolved_at": null
  },
  {
    "id": 10,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": ".github/workflows/integration-tests.yml",
    "line": null,
    "description": "Task 2's first automated verify literally asserts survivors=={pre-commit.yml} after migration, but integration-tests.yml (3 hand-rolled cache blocks) is still present -- deletion is plan 15.1-05's job, not yet executed in this wave, exactly per this plan's own Recorded discretion resolutions section. Substituted an assertion expecting survivors=={pre-commit.yml, integration-tests.yml}, both counts matching (1 and 3 respectively).",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-14T14:22:56.039Z",
    "resolved_at": null
  },
  {
    "id": 11,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": ".github/workflows/ci.yml",
    "line": null,
    "description": "Task 2 acceptance criterion 'grep -rc restore-keys ci.yml feature-flags.yml release.yml' returns 0 for ci.yml -- returns 2, both from pre-existing prose comments in the examples job (added by plan 15.1-01, lines ~268/271) explaining why a restore-keys fallback alone is insufficient, not an actual YAML restore-keys: key. Verified via structural YAML walk: no step's with block contains a restore-keys key in any of the three files.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-14T14:23:03.057Z",
    "resolved_at": null
  },
  {
    "id": 12,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "tests/integration/ollama_docker_test.rs",
    "line": null,
    "description": "Ollama Docker-gated Tier 2 suite (17-07 Task 2) authored and proven to compile/clippy-clean/skip-gracefully, but never run against a real Ollama server -- no Docker daemon in the execution sandbox. Runtime behavior (generate/generate_stream/get_available_models/validate_model against real qwen2.5:0.5b) is unverified.",
    "status": "fixed",
    "reason": "Resolved 2026-08-23 by a CI run on the pushed branch. The orchestrator queried the GitHub check-runs API directly for commit 76b859d (the SHA it pushed) rather than relying on a report: 44 checks success, 3 skipped, 0 failures. This is first-hand evidence at CURRENT HEAD, which is what these rows lacked -- the previously cited run was at ca211644 (2026-08-19), before ~2,160 lines of gap-closure code landed. The 'Ollama Integration Tests (live server)' job concluded success (completed 2026-08-23T16:55:44Z), exercising the Docker-gated Tier 2 suite this row recorded as unrun.",
    "recorded_at": "2026-08-17T14:17:30.134Z",
    "resolved_at": "2026-08-23T17:55:00.000Z"
  },
  {
    "id": 13,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "Makefile",
    "line": null,
    "description": "17-07 Task 3: the workspace 82% line-coverage gate (make coverage) could not be run in this execution sandbox -- Redis (6380) and MinIO (9010) are unreachable because no Docker daemon is available, and the coverage target's own preflight fails fast on both. The coverage percentage with all six new adapters counted is UNMEASURED, not failing. cargo doc -p paladin-llm --no-deps (0 missing-docs warnings under the six new features) and a scoped clippy pass on touched targets were verified instead.",
    "status": "fixed",
    "reason": "Resolved 2026-08-23 by a CI run on the pushed branch. The orchestrator queried the GitHub check-runs API directly for commit 76b859d (the SHA it pushed) rather than relying on a report: 44 checks success, 3 skipped, 0 failures. This is first-hand evidence at CURRENT HEAD, which is what these rows lacked -- the previously cited run was at ca211644 (2026-08-19), before ~2,160 lines of gap-closure code landed. The 'Coverage' job concluded success (completed 2026-08-23T16:59:08Z). That job runs `cargo llvm-cov --fail-under-lines 82`, so a success conclusion IS the >=82% workspace line-coverage assertion (ADR-0006) holding against the gap-closure code -- the exact measurement this row recorded as unrun. The job emitted no percentage into its check-run output, so the pass/fail verdict is recorded here rather than a figure.",
    "recorded_at": "2026-08-17T14:17:37.112Z",
    "resolved_at": "2026-08-23T17:55:00.000Z"
  },
  {
    "id": 14,
    "kind": "deviation",
    "phase": "17",
    "file": "docker/docker-compose.test.yml",
    "line": null,
    "description": "17-07 Task 2: ollama-test healthcheck uses 'ollama list' (native /api/tags) instead of the plan's preferred curl-based /v1/models check, because curl/wget availability in the ollama/ollama:0.3.14 base image could not be verified without Docker in this sandbox. 'ollama list' is a well-precedented dependency-free healthcheck for this exact image. Compose file syntax validated via python yaml.safe_load only -- 'docker compose config' itself was never run.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-17T14:17:46.408Z",
    "resolved_at": null
  },
  {
    "id": 15,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "crates/paladin-llm/src/gemini/adapter.rs",
    "line": null,
    "description": "Snyk code scan (per snyk_rules.instructions.md) could not be run — no Snyk MCP tool or CLI available in this worktree's runtime (no network egress); recorded as not-run, never as passed",
    "status": "waived",
    "reason": "Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md.",
    "recorded_at": "2026-08-17T19:33:52.477Z",
    "resolved_at": "2026-08-19T13:55:48.872Z"
  },
  {
    "id": 16,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "crates/paladin-llm/src/compat/engine.rs,crates/paladin-llm/src/kimi/adapter.rs,crates/paladin-llm/src/qwen/adapter.rs,crates/paladin-llm/src/grok/adapter.rs,crates/paladin-llm/src/ollama/adapter.rs,crates/paladin-llm/src/gemini/adapter.rs",
    "line": null,
    "description": "Plan 17-10 verification step 7 (Snyk code scan over the five modified WR-04 adapter files plus compat/engine.rs) was not run — snyk_code_scan MCP tool unavailable in the executor runtime",
    "status": "waived",
    "reason": "Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md.",
    "recorded_at": "2026-08-17T20:01:16.281Z",
    "resolved_at": "2026-08-19T13:55:49.213Z"
  },
  {
    "id": 17,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "crates/paladin-llm/src/gemini/adapter.rs",
    "line": null,
    "description": "Plan 17-11's verification step 7 (Snyk code scan over crates/paladin-llm/src/gemini/adapter.rs) was not run -- no snyk_code_scan MCP tool and no Snyk CLI were available in the executor's runtime (no network egress). 17-11's own SUMMARY.md recorded this as not-run, never as passed. This row is the sibling of ids 15 and 16, filed 2026-08-18 by plan 17-17 after 17-VERIFICATION.md flagged 17-11's row as missing.",
    "status": "waived",
    "reason": "Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md.",
    "recorded_at": "2026-08-18T02:10:42.462Z",
    "resolved_at": "2026-08-19T13:55:49.665Z"
  },
  {
    "id": 18,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "crates/paladin-llm/src/provider_factory.rs,tests/unit/llm/provider_factory_test.rs,crates/paladin-llm/src/openai_compatible/adapter.rs,crates/paladin-llm/src/gemini/adapter.rs,crates/paladin-llm/src/compat/engine.rs",
    "line": null,
    "description": "Plans 17-12 through 17-16 each attempted the mandated Snyk code scan (per snyk_rules.instructions.md, imported into CLAUDE.md) over the files they modified; none could run -- no snyk_code_scan MCP tool and no Snyk CLI in this environment. All five SUMMARYs (17-12-SUMMARY.md, 17-13-SUMMARY.md, 17-14-SUMMARY.md, 17-15-SUMMARY.md, 17-16-SUMMARY.md) record their scans as not-run, never as passed. Filed 2026-08-18 by plan 17-17.",
    "status": "waived",
    "reason": "Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md.",
    "recorded_at": "2026-08-18T02:10:55.983Z",
    "resolved_at": "2026-08-19T13:55:50.375Z"
  },
  {
    "id": 19,
    "kind": "deviation",
    "phase": "17",
    "file": ".project/current-exports.txt",
    "line": null,
    "description": "The public-API surface snapshot .project/current-exports.txt was regenerated under default features only, so KimiAdapter, QwenAdapter, GrokAdapter, OllamaAdapter, GeminiAdapter and OpenAiCompatibleAdapter do not appear in it and cannot be checked for public-API drift. This is consistent with D-11's unchanged default feature set and is not itself wrong. 17-REVIEW.md records it as IN-01, non-blocking, with the suggested follow-up of generating an --features llm-all variant or documenting its absence. It was excluded from the 2026-08-18 gap-closure scope by explicit developer decision, taken in an interactive AskUserQuestion checkpoint in the orchestrating /gsd-plan-phase 17 --gaps session -- a recorded human choice, not an --auto inference -- and is therefore carried forward as accepted debt (IN-01) rather than dropped. Filed 2026-08-18 by plan 17-17.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-18T02:11:05.490Z",
    "resolved_at": null
  },
  {
    "id": 20,
    "kind": "deviation",
    "phase": "17",
    "file": "crates/paladin-llm/src/gemini/adapter.rs",
    "line": null,
    "description": "Live vendor smoke run for plan 17-18 (2026-08-22) found Gemini's generate() probe FAILS on GEMINI_DEFAULT_MODEL=gemini-2.5-flash: vendor error 'This model models/gemini-2.5-flash is no longer available to new users. Please update your code to use models/gemini-3.6-flash'. Model-list probe still PASSES (model present in live catalog), so this is a vendor-side default-model deprecation, not a regression from 17-18's CompatRequestParameters change -- Gemini is not built on CompatEngine at all and is structurally unaffected by it. Confirmed identical before and after this plan's code changes. Out of scope for 17-18 (gemini/adapter.rs not in files_modified); candidate follow-up: refresh GEMINI_DEFAULT_MODEL similarly to this plan's Grok refresh.",
    "status": "fixed",
    "reason": "Fixed by the orchestrator between waves 1 and 2 of /gsd-execute-phase 17 --gaps-only (commit 954b750). GEMINI_DEFAULT_MODEL -> gemini-3.6-flash and GEMINI_FALLBACK_MODELS -> [gemini-3.6-flash, gemini-3.5-flash], every entry verified by a live generateContent call on 2026-08-22 rather than taken from the vendor deprecation message on faith. No pro-family fallback entry: gemini-2.5-pro and gemini-3-pro-preview are retired, gemini-3.6-pro is absent from v1beta, and gemini-pro-latest / gemini-3.1-pro-preview returned quota errors on the available credential -- an unverified identifier is what this refresh exists to remove. Escalated out of follow-up status because plans 17-19, 17-21 and 17-22 each carry a must_have requiring Gemini to PASS both live probes, and 17-22 requires four vendors PASS; leaving it open would have made three downstream must_haves unachievable. Live harness after the fix: Grok PASS/PASS, Gemini PASS/PASS.",
    "recorded_at": "2026-08-22T16:29:02.316Z",
    "resolved_at": "2026-08-22T16:52:00.000Z"
  },
  {
    "id": 21,
    "kind": "unrun-verify",
    "phase": "17",
    "file": "crates/paladin-llm/src/qwen/adapter.rs",
    "line": null,
    "description": "Plan 17-21 Task 2 is BLOCKED on an Alibaba Cloud Model Studio account entitlement, not on any code defect. After Task 1 moved QWEN_DEFAULT_BASE_URL to the US (Virginia) compatible-mode endpoint, the credential authenticates there correctly -- GET /models returns 92 entries with qwen-plus present, versus invalid_api_key at the previous dashscope-intl (Singapore) default, which is the measurement that proves the reversal right. But every chat-completion invocation returns HTTP 403 {\"code\":\"Model.AccessDenied\"}. The plan's executor ruled out a stale-identifier explanation across 78 qwen-prefixed identifiers and their -us regional variants, two unrelated model families hosted on the same workspace (deepseek-v4-flash, glm-5.1), and both the OpenAI-compatible and native DashScope invocation endpoints; the orchestrator independently reproduced the same 403 on qwen-plus. Consequence: the Qwen generate() probe cannot PASS, so plan 17-21's remaining must_haves (QWEN_FALLBACK_MODELS refreshed from a live-measured catalog, the five sampling-parameter verdicts, both temperature_range endpoints) are unmeasurable, and plan 17-22's 'four vendors PASS' clause is unachievable. Note that 17-21-SUMMARY.md exists with frontmatter status: blocked, but phase-plan-index keys off file EXISTENCE, so 17-21 reads as complete to the index and will be skipped by a plain --gaps-only re-run. It is deliberately NOT marked complete in ROADMAP.md. Required human action: in the Model Studio console, for the workspace tied to DASHSCOPE_API_KEY, select US (Virginia) and activate model invocation for at least qwen-plus, clearing whatever billing/quota/terms gate the console surfaces -- the API returns only the generic Model.AccessDenied code. Verify with: cargo run -p paladin-llm --example live_vendor_smoke --features kimi,qwen,grok,gemini (DASHSCOPE_BASE_URL left unset); Qwen's generate line should read PASS. Filed 2026-08-22 by the /gsd-execute-phase 17 --gaps-only orchestrator; the developer chose to continue waves 4 and 5 with Qwen recorded as catalog-verified / invocation-blocked rather than wait.",
    "status": "fixed",
    "reason": "Resolved 2026-08-23, externally: the operator's DASHSCOPE_API_KEY was replaced with a Singapore-scoped credential (the entitlement-blocked key was Virginia-scoped and workspace-specific). Against the new key and the corrected shipped default (dashscope-intl, Singapore -- plan 17-21 gap closure), every measured request succeeded: GET /models returned 162 entries, generate() returned real completions for qwen-plus and candidate qwen3.7-plus, and all five optional sampling parameters plus both temperature_range endpoints were probed individually with no rejection below DashScope's documented [0.0, 2.0) temperature ceiling. No code change resolved this row -- it was never a code defect -- but the previously-blocked live_vendor_smoke run now exits 0 with all four vendors (Kimi, Qwen, Grok, Gemini) PASSING both probes and no DASHSCOPE_BASE_URL override, closing plan 17-22's 'four vendors PASS' clause. See 17-21-SUMMARY.md's 2026-08-23 update for the full measurement record. [Ledger normalization 2026-08-23: this row was originally written with kind \"blocker\" and status \"resolved\", neither of which is in the WINDOWS.md schema vocabulary (kinds: stub|todo|fixme|skipped-test|lint-warning|unmet-truth|unrun-verify|deviation; statuses: open|waived|fixed). The off-schema values made the whole ledger unreadable to gsd-tools. Reclassified to kind=unrun-verify (a live-vendor verification that could not be executed) and status=fixed; the substance of the record is unchanged.]",
    "recorded_at": "2026-08-22T18:05:00.000Z",
    "resolved_at": "2026-08-23T12:55:15.198Z"
  },
  {
    "id": 22,
    "kind": "unrun-verify",
    "phase": "22",
    "file": "crates/paladin-storage/src/waypoint/postgres.rs",
    "line": null,
    "description": "Postgres Tier 2 contract-suite pass against a real postgres-test service is unverified (no Docker in the execution environment); compile, lint, and the clean-skip path are proven. Run make test-integration-docker to close.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-02T00:00:49.385Z",
    "resolved_at": null
  },
  {
    "id": 23,
    "kind": "deviation",
    "phase": "22",
    "file": "crates/paladin-battalion/src/engine/bridges.rs",
    "line": null,
    "description": "from_campaign extends the ENG-FR-19 default three-field schema with one dedicated LastWrite field per Paladin (not literally exactly three fields for this constructor) so a general DAG's concurrent fan-out siblings never hit a DispatchConflict; from_formation/from_phalanx remain exactly three fields as specified",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-02T03:29:38.826Z",
    "resolved_at": null
  },
  {
    "id": 24,
    "kind": "deviation",
    "phase": "22",
    "file": "tests/integration/e2e_crash_resume_test.rs",
    "line": 112,
    "description": "loop_gate self-loop node made a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property, rather than fixed structurally; flagged for plan 22-16's fixture audit (acceptance 2a)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-02T18:04:03.616Z",
    "resolved_at": null
  },
  {
    "id": 25,
    "kind": "deviation",
    "phase": "22",
    "file": "crates/paladin-battalion/src/engine/superstep.rs",
    "line": 1220,
    "description": "self_loop_graph test helper makes its looping node a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property (same root cause as e2e_crash_resume_test.rs); flagged for plan 22-16's fixture audit (acceptance 2a)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-02T18:04:11.984Z",
    "resolved_at": null
  }
]
````
