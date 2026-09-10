---
schema_version: 1
open_count: 4
waived_count: 25
fixed_count: 5
total_count: 34
last_updated: 2026-09-10T01:58:41.503Z
---

# Broken Windows Ledger

> Cross-phase defect register. `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | unmet-truth | .planning/ledgers/milestone-01.md |  | REQ-battalion-result-v1 (Epic 4 FR-4.2, cited in ADR-0002's Considered Options as 'superseded by the shipped superset') has no row anywhere in the Milestone 1 ledger's Epic 4 table, even though REQUIREMENTS.md's original ledger body carried it as 'Variant (group 4)'. Plan 01-08 Task 2's subset-check safety gate caught this and HALTED per the plan's explicit instruction rather than reducing REQUIREMENTS.md's Milestone 1 body to a pointer at an incomplete destination. | fixed |  | 2026-07-31T13:22:57.385Z | 2026-07-31T14:46:37.492Z |
| 2 | 03 | deviation | crates/paladin-storage/src/redis.rs |  | Live-server code paths of redis.rs (everything reaching through self.conn) remain uncovered by unit tests; deferred with reason, owner Phase 15 (PIPE), exerciser tests/integration/redis_queue_integration_test.rs (requires Docker) | waived | Predates the v0.9.0 tag (Phase 03, filed 2026-08-02) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: redis.rs's live-server code paths reaching through self.conn are still exercised only by tests/integration/redis_queue_integration_test.rs, which requires Docker and has never run in this devcontainer; owner remains Phase 15 (PIPE) per the row's own text. | 2026-08-02T15:41:28.892Z | 2026-09-10T01:58:05.756Z |
| 3 | 07 | deviation | .project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md |  | Task 3's requested single combined commit for ADR-0016 + PRD annotation was split into two atomic commits (9e8db80, 71ea46e) per standard task_commit_protocol; both files present, no content impact. | waived | Predates the v0.9.0 tag (Phase 07, filed 2026-08-06) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: this is a process note (a requested single combined commit was split into two atomic commits, 9e8db80 and 71ea46e, per standard task_commit_protocol) with confirmed no content impact; the row was simply never triaged before v0.9.0 shipped. | 2026-08-06T18:09:04.871Z | 2026-09-10T01:58:05.936Z |
| 4 | 07 | deviation | .project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md |  | No fabricated 3rd strikethrough correction for CONTEXT.md D-08(5)'s anticipated section-1 Milestone 1/Epic 2 cross-reference — re-verified absent from live tree (matches ADR-0014's own flagged drift); acceptance criterion expecting >=3 strikethrough lines not met by design. | waived | Predates the v0.9.0 tag (Phase 07, filed 2026-08-06) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the anticipated third strikethrough correction was re-verified absent from the live tree at filing time (matching ADR-0014's own flagged drift) -- the row documents an acceptance criterion expecting >=3 strikethrough lines that was not met by design, not an outstanding defect. | 2026-08-06T18:09:08.207Z | 2026-09-10T01:58:06.102Z |
| 5 | 14 | unrun-verify | Cargo.toml |  | cargo test --workspace not run to completion for 14-01: system-wide disk exhaustion (830G/875G used, 0 avail on /workspace mount) blocked full workspace compile; targeted plan <verify> commands (paladin-ai lib config::agents, paladin-web full suite, paladin-server binary build, openapi drift guard, check-api-surface.sh) all passed | waived | Predates the v0.9.0 tag (Phase 14, filed 2026-08-12) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: a full cargo test --workspace for plan 14-01 was never re-run after the system-wide disk exhaustion (830G/875G used) that blocked it at filing time; only the plan's own targeted <verify> commands (paladin-ai lib config::agents, paladin-web full suite, paladin-server binary build, openapi drift guard, check-api-surface.sh) are confirmed passing. | 2026-08-12T16:51:08.832Z | 2026-09-10T01:58:06.265Z |
| 6 | 14 | unrun-verify | N/A (workspace-wide) |  | 14-04: full 'cargo test --workspace' not run — shared /workspace mount at 99%25 (13G free), matching 14-01's documented disk-exhaustion condition; the plan's own targeted verify (cargo test --bin paladin-server --features web-server, cargo fmt --check, cargo clippy --all-targets --features web-server -- -D warnings) all ran to completion and passed | waived | Predates the v0.9.0 tag (Phase 14, filed 2026-08-12) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: a full cargo test --workspace for plan 14-04 was never re-run after the shared /workspace mount's disk-pressure condition (13G free) that blocked it at filing time; only the plan's own targeted verify commands (paladin-server binary tests, cargo fmt --check, cargo clippy -D warnings) are confirmed passing. | 2026-08-12T17:13:58.989Z | 2026-09-10T01:58:06.436Z |
| 7 | 14 | deviation | CHANGELOG.md |  | 14-08's acceptance criterion expected >=2 'BREAKING' lines under the dated 0.8.0 section in root CHANGELOG.md; only 1 is present. 14-01 split the phase's two consumer-break BREAKING entries across root CHANGELOG.md (config-key rename) and crates/paladin-web/CHANGELOG.md (AgentAuthConfig field + OpenAPI scheme rename), one per file, per 14-01-SUMMARY.md's own D4 verification and this plan's own instruction to leave per-crate changelogs untouched. Both breaks are documented with a BREAKING entry and cite ADR-0040; only the single-file grep count in the plan's acceptance criteria was miscalibrated. | waived | Predates the v0.9.0 tag (Phase 14, filed 2026-08-12) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the acceptance criterion's >=2-BREAKING-lines-in-one-file expectation was miscalibrated -- 14-01-SUMMARY.md's own D4 verification confirms the phase's two consumer-break BREAKING entries were correctly split one per file across root CHANGELOG.md and crates/paladin-web/CHANGELOG.md, both citing ADR-0040. | 2026-08-12T18:05:57.086Z | 2026-09-10T01:58:06.593Z |
| 8 | 15.1 | unrun-verify | SECURITY-EXCEPTIONS.md |  | Plan 15.1-01 Task 2's inline verify python one-liner (block-split regex over the machine-readable register) fails with a pre-existing TOML parse error on the LAST exception block, because its lookahead doesn't stop before the trailing markdown code fence -- reproduced against the pre-edit file too, unrelated to this task's new row. Substituted an isolated per-block parse of just the new RUSTSEC-2026-0249 row (11/11 fields present) plus the real repo guard scripts/check-advisory-register.sh (exit 0) as equivalent proof. | waived | Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: the inline verify python one-liner's pre-existing lookahead bug over SECURITY-EXCEPTIONS.md's trailing markdown fence was never fixed or re-run; the substituted isolated per-block parse of the new RUSTSEC-2026-0249 row plus scripts/check-advisory-register.sh (exit 0) stands as the equivalent proof on record, not a re-run of the original check. | 2026-08-14T00:49:21.261Z | 2026-09-10T01:58:06.758Z |
| 9 | 15.1 | unrun-verify | .github/workflows/ci.yml |  | Task 1 acceptance criterion 'git diff \| grep -c "^[+-].*cargo "' returns 4 not 0 -- matches step *name* text ('Cache cargo registry' etc.) removed by the migration, not actual cargo invocations. Verified via 'run: cargo' scoped grep returning 0 changed invocations. | waived | Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the acceptance criterion's grep counted CI step *name* text ('Cache cargo registry' etc.) removed by the migration, not actual cargo invocations; the scoped 'run: cargo' grep returning 0 changed invocations was already performed and is the correct verification. | 2026-08-14T14:22:48.884Z | 2026-09-10T01:58:06.922Z |
| 10 | 15.1 | unrun-verify | .github/workflows/integration-tests.yml |  | Task 2's first automated verify literally asserts survivors=={pre-commit.yml} after migration, but integration-tests.yml (3 hand-rolled cache blocks) is still present -- deletion is plan 15.1-05's job, not yet executed in this wave, exactly per this plan's own Recorded discretion resolutions section. Substituted an assertion expecting survivors=={pre-commit.yml, integration-tests.yml}, both counts matching (1 and 3 respectively). | waived | Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: integration-tests.yml's three hand-rolled cache blocks were still present at filing time by design (deletion deferred to plan 15.1-05); whether 15.1-05 actually removed them afterward was never re-checked against this row. | 2026-08-14T14:22:56.039Z | 2026-09-10T01:58:07.111Z |
| 11 | 15.1 | unrun-verify | .github/workflows/ci.yml |  | Task 2 acceptance criterion 'grep -rc restore-keys ci.yml feature-flags.yml release.yml' returns 0 for ci.yml -- returns 2, both from pre-existing prose comments in the examples job (added by plan 15.1-01, lines ~268/271) explaining why a restore-keys fallback alone is insufficient, not an actual YAML restore-keys: key. Verified via structural YAML walk: no step's with block contains a restore-keys key in any of the three files. | waived | Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the 2 grep matches for restore-keys in ci.yml are pre-existing prose comments (lines ~268/271) explaining why a restore-keys fallback alone is insufficient, not actual YAML restore-keys keys; the structural YAML walk confirming no step's with-block contains the key was already performed. | 2026-08-14T14:23:03.057Z | 2026-09-10T01:58:07.279Z |
| 12 | 17 | unrun-verify | tests/integration/ollama_docker_test.rs |  | Ollama Docker-gated Tier 2 suite (17-07 Task 2) authored and proven to compile/clippy-clean/skip-gracefully, but never run against a real Ollama server -- no Docker daemon in the execution sandbox. Runtime behavior (generate/generate_stream/get_available_models/validate_model against real qwen2.5:0.5b) is unverified. | fixed | Resolved 2026-08-23 by a CI run on the pushed branch. The orchestrator queried the GitHub check-runs API directly for commit 76b859d (the SHA it pushed) rather than relying on a report: 44 checks success, 3 skipped, 0 failures. This is first-hand evidence at CURRENT HEAD, which is what these rows lacked -- the previously cited run was at ca211644 (2026-08-19), before ~2,160 lines of gap-closure code landed. The 'Ollama Integration Tests (live server)' job concluded success (completed 2026-08-23T16:55:44Z), exercising the Docker-gated Tier 2 suite this row recorded as unrun. | 2026-08-17T14:17:30.134Z | 2026-08-23T17:55:00.000Z |
| 13 | 17 | unrun-verify | Makefile |  | 17-07 Task 3: the workspace 82% line-coverage gate (make coverage) could not be run in this execution sandbox -- Redis (6380) and MinIO (9010) are unreachable because no Docker daemon is available, and the coverage target's own preflight fails fast on both. The coverage percentage with all six new adapters counted is UNMEASURED, not failing. cargo doc -p paladin-llm --no-deps (0 missing-docs warnings under the six new features) and a scoped clippy pass on touched targets were verified instead. | fixed | Resolved 2026-08-23 by a CI run on the pushed branch. The orchestrator queried the GitHub check-runs API directly for commit 76b859d (the SHA it pushed) rather than relying on a report: 44 checks success, 3 skipped, 0 failures. This is first-hand evidence at CURRENT HEAD, which is what these rows lacked -- the previously cited run was at ca211644 (2026-08-19), before ~2,160 lines of gap-closure code landed. The 'Coverage' job concluded success (completed 2026-08-23T16:59:08Z). That job runs `cargo llvm-cov --fail-under-lines 82`, so a success conclusion IS the >=82% workspace line-coverage assertion (ADR-0006) holding against the gap-closure code -- the exact measurement this row recorded as unrun. The job emitted no percentage into its check-run output, so the pass/fail verdict is recorded here rather than a figure. | 2026-08-17T14:17:37.112Z | 2026-08-23T17:55:00.000Z |
| 14 | 17 | deviation | docker/docker-compose.test.yml |  | 17-07 Task 2: ollama-test healthcheck uses 'ollama list' (native /api/tags) instead of the plan's preferred curl-based /v1/models check, because curl/wget availability in the ollama/ollama:0.3.14 base image could not be verified without Docker in this sandbox. 'ollama list' is a well-precedented dependency-free healthcheck for this exact image. Compose file syntax validated via python yaml.safe_load only -- 'docker compose config' itself was never run. | waived | Predates the v0.9.0 tag (Phase 17, filed 2026-08-17) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: the ollama-test healthcheck's use of 'ollama list' instead of a curl-based /v1/models check was never validated against a running Docker daemon in any authoring environment; 'docker compose config' itself was never run, only a python yaml.safe_load syntax check. | 2026-08-17T14:17:46.408Z | 2026-09-10T01:58:07.447Z |
| 15 | 17 | unrun-verify | crates/paladin-llm/src/gemini/adapter.rs |  | Snyk code scan (per snyk_rules.instructions.md) could not be run — no Snyk MCP tool or CLI available in this worktree's runtime (no network egress); recorded as not-run, never as passed | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-17T19:33:52.477Z | 2026-08-19T13:55:48.872Z |
| 16 | 17 | unrun-verify | crates/paladin-llm/src/compat/engine.rs,crates/paladin-llm/src/kimi/adapter.rs,crates/paladin-llm/src/qwen/adapter.rs,crates/paladin-llm/src/grok/adapter.rs,crates/paladin-llm/src/ollama/adapter.rs,crates/paladin-llm/src/gemini/adapter.rs |  | Plan 17-10 verification step 7 (Snyk code scan over the five modified WR-04 adapter files plus compat/engine.rs) was not run — snyk_code_scan MCP tool unavailable in the executor runtime | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-17T20:01:16.281Z | 2026-08-19T13:55:49.213Z |
| 17 | 17 | unrun-verify | crates/paladin-llm/src/gemini/adapter.rs |  | Plan 17-11's verification step 7 (Snyk code scan over crates/paladin-llm/src/gemini/adapter.rs) was not run -- no snyk_code_scan MCP tool and no Snyk CLI were available in the executor's runtime (no network egress). 17-11's own SUMMARY.md recorded this as not-run, never as passed. This row is the sibling of ids 15 and 16, filed 2026-08-18 by plan 17-17 after 17-VERIFICATION.md flagged 17-11's row as missing. | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-18T02:10:42.462Z | 2026-08-19T13:55:49.665Z |
| 18 | 17 | unrun-verify | crates/paladin-llm/src/provider_factory.rs,tests/unit/llm/provider_factory_test.rs,crates/paladin-llm/src/openai_compatible/adapter.rs,crates/paladin-llm/src/gemini/adapter.rs,crates/paladin-llm/src/compat/engine.rs |  | Plans 17-12 through 17-16 each attempted the mandated Snyk code scan (per snyk_rules.instructions.md, imported into CLAUDE.md) over the files they modified; none could run -- no snyk_code_scan MCP tool and no Snyk CLI in this environment. All five SUMMARYs (17-12-SUMMARY.md, 17-13-SUMMARY.md, 17-14-SUMMARY.md, 17-15-SUMMARY.md, 17-16-SUMMARY.md) record their scans as not-run, never as passed. Filed 2026-08-18 by plan 17-17. | waived | Snyk mandate removed 2026-08-18: Snyk has no Rust coverage (SAST found 0 of 4 planted vulnerabilities that it caught in equivalent JavaScript; SCA has no Cargo support). The scan this row waits on cannot produce a meaningful result. Superseded by make security + clippy + manual credential review per .github/instructions/security.instructions.md. | 2026-08-18T02:10:55.983Z | 2026-08-19T13:55:50.375Z |
| 19 | 17 | deviation | .project/current-exports.txt |  | The public-API surface snapshot .project/current-exports.txt was regenerated under default features only, so KimiAdapter, QwenAdapter, GrokAdapter, OllamaAdapter, GeminiAdapter and OpenAiCompatibleAdapter do not appear in it and cannot be checked for public-API drift. This is consistent with D-11's unchanged default feature set and is not itself wrong. 17-REVIEW.md records it as IN-01, non-blocking, with the suggested follow-up of generating an --features llm-all variant or documenting its absence. It was excluded from the 2026-08-18 gap-closure scope by explicit developer decision, taken in an interactive AskUserQuestion checkpoint in the orchestrating /gsd-plan-phase 17 --gaps session -- a recorded human choice, not an --auto inference -- and is therefore carried forward as accepted debt (IN-01) rather than dropped. Filed 2026-08-18 by plan 17-17. | waived | Predates the v0.9.0 tag (Phase 17, filed 2026-08-18) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: the public-API surface snapshot .project/current-exports.txt still excludes KimiAdapter/QwenAdapter/GrokAdapter/OllamaAdapter/GeminiAdapter/OpenAiCompatibleAdapter (default-features-only regeneration); 17-REVIEW.md recorded this as IN-01, non-blocking, deliberately excluded from the 2026-08-18 gap-closure scope by an explicit interactive human decision -- carried forward as accepted debt, not re-measured here. | 2026-08-18T02:11:05.490Z | 2026-09-10T01:58:07.612Z |
| 20 | 17 | deviation | crates/paladin-llm/src/gemini/adapter.rs |  | Live vendor smoke run for plan 17-18 (2026-08-22) found Gemini's generate() probe FAILS on GEMINI_DEFAULT_MODEL=gemini-2.5-flash: vendor error 'This model models/gemini-2.5-flash is no longer available to new users. Please update your code to use models/gemini-3.6-flash'. Model-list probe still PASSES (model present in live catalog), so this is a vendor-side default-model deprecation, not a regression from 17-18's CompatRequestParameters change -- Gemini is not built on CompatEngine at all and is structurally unaffected by it. Confirmed identical before and after this plan's code changes. Out of scope for 17-18 (gemini/adapter.rs not in files_modified); candidate follow-up: refresh GEMINI_DEFAULT_MODEL similarly to this plan's Grok refresh. | fixed | Fixed by the orchestrator between waves 1 and 2 of /gsd-execute-phase 17 --gaps-only (commit 954b750). GEMINI_DEFAULT_MODEL -> gemini-3.6-flash and GEMINI_FALLBACK_MODELS -> [gemini-3.6-flash, gemini-3.5-flash], every entry verified by a live generateContent call on 2026-08-22 rather than taken from the vendor deprecation message on faith. No pro-family fallback entry: gemini-2.5-pro and gemini-3-pro-preview are retired, gemini-3.6-pro is absent from v1beta, and gemini-pro-latest / gemini-3.1-pro-preview returned quota errors on the available credential -- an unverified identifier is what this refresh exists to remove. Escalated out of follow-up status because plans 17-19, 17-21 and 17-22 each carry a must_have requiring Gemini to PASS both live probes, and 17-22 requires four vendors PASS; leaving it open would have made three downstream must_haves unachievable. Live harness after the fix: Grok PASS/PASS, Gemini PASS/PASS. | 2026-08-22T16:29:02.316Z | 2026-08-22T16:52:00.000Z |
| 21 | 17 | unrun-verify | crates/paladin-llm/src/qwen/adapter.rs |  | Plan 17-21 Task 2 is BLOCKED on an Alibaba Cloud Model Studio account entitlement, not on any code defect. After Task 1 moved QWEN_DEFAULT_BASE_URL to the US (Virginia) compatible-mode endpoint, the credential authenticates there correctly -- GET /models returns 92 entries with qwen-plus present, versus invalid_api_key at the previous dashscope-intl (Singapore) default, which is the measurement that proves the reversal right. But every chat-completion invocation returns HTTP 403 {"code":"Model.AccessDenied"}. The plan's executor ruled out a stale-identifier explanation across 78 qwen-prefixed identifiers and their -us regional variants, two unrelated model families hosted on the same workspace (deepseek-v4-flash, glm-5.1), and both the OpenAI-compatible and native DashScope invocation endpoints; the orchestrator independently reproduced the same 403 on qwen-plus. Consequence: the Qwen generate() probe cannot PASS, so plan 17-21's remaining must_haves (QWEN_FALLBACK_MODELS refreshed from a live-measured catalog, the five sampling-parameter verdicts, both temperature_range endpoints) are unmeasurable, and plan 17-22's 'four vendors PASS' clause is unachievable. Note that 17-21-SUMMARY.md exists with frontmatter status: blocked, but phase-plan-index keys off file EXISTENCE, so 17-21 reads as complete to the index and will be skipped by a plain --gaps-only re-run. It is deliberately NOT marked complete in ROADMAP.md. Required human action: in the Model Studio console, for the workspace tied to DASHSCOPE_API_KEY, select US (Virginia) and activate model invocation for at least qwen-plus, clearing whatever billing/quota/terms gate the console surfaces -- the API returns only the generic Model.AccessDenied code. Verify with: cargo run -p paladin-llm --example live_vendor_smoke --features kimi,qwen,grok,gemini (DASHSCOPE_BASE_URL left unset); Qwen's generate line should read PASS. Filed 2026-08-22 by the /gsd-execute-phase 17 --gaps-only orchestrator; the developer chose to continue waves 4 and 5 with Qwen recorded as catalog-verified / invocation-blocked rather than wait. | fixed | Resolved 2026-08-23, externally: the operator's DASHSCOPE_API_KEY was replaced with a Singapore-scoped credential (the entitlement-blocked key was Virginia-scoped and workspace-specific). Against the new key and the corrected shipped default (dashscope-intl, Singapore -- plan 17-21 gap closure), every measured request succeeded: GET /models returned 162 entries, generate() returned real completions for qwen-plus and candidate qwen3.7-plus, and all five optional sampling parameters plus both temperature_range endpoints were probed individually with no rejection below DashScope's documented [0.0, 2.0) temperature ceiling. No code change resolved this row -- it was never a code defect -- but the previously-blocked live_vendor_smoke run now exits 0 with all four vendors (Kimi, Qwen, Grok, Gemini) PASSING both probes and no DASHSCOPE_BASE_URL override, closing plan 17-22's 'four vendors PASS' clause. See 17-21-SUMMARY.md's 2026-08-23 update for the full measurement record. [Ledger normalization 2026-08-23: this row was originally written with kind "blocker" and status "resolved", neither of which is in the WINDOWS.md schema vocabulary (kinds: stub\|todo\|fixme\|skipped-test\|lint-warning\|unmet-truth\|unrun-verify\|deviation; statuses: open\|waived\|fixed). The off-schema values made the whole ledger unreadable to gsd-tools. Reclassified to kind=unrun-verify (a live-vendor verification that could not be executed) and status=fixed; the substance of the record is unchanged.] | 2026-08-22T18:05:00.000Z | 2026-08-23T12:55:15.198Z |
| 22 | 22 | unrun-verify | crates/paladin-storage/src/waypoint/postgres.rs |  | Postgres Tier 2 contract-suite pass against a real postgres-test service is unverified (no Docker in the execution environment); compile, lint, and the clean-skip path are proven. Run make test-integration-docker to close. | open |  | 2026-09-02T00:00:49.385Z |  |
| 23 | 22 | deviation | crates/paladin-battalion/src/engine/bridges.rs |  | from_campaign extends the ENG-FR-19 default three-field schema with one dedicated LastWrite field per Paladin (not literally exactly three fields for this constructor) so a general DAG's concurrent fan-out siblings never hit a DispatchConflict; from_formation/from_phalanx remain exactly three fields as specified | waived | Design deviation recorded and reasoned in 22-11-SUMMARY.md (WarGraph::from_formation/from_phalanx/from_campaign bridge constructors, Phase 22): from_campaign extends the ENG-FR-19 default three-field schema with one dedicated LastWrite field per Paladin because a single shared output field under LastWrite would hard-conflict the instant two Campaign fan-out siblings execute in the same superstep -- a structural certainty for the general-DAG case this bridge supports. from_formation and from_phalanx remain literally exactly three fields, matching ENG-FR-19 precisely; every other must-have truth and acceptance criterion (byte-for-byte golden equivalence, legacy services untouched, NodeId/fingerprint determinism, empty-list handling) is met exactly as written, including the specific 'exactly three fields' test proven against from_formation's own schema. Not an unmet FR per 29-04-SUMMARY.md/29-07-SUMMARY.md, which raise no finding against this row. | 2026-09-02T03:29:38.826Z | 2026-09-10T01:58:40.149Z |
| 24 | 22 | deviation | tests/integration/e2e_crash_resume_test.rs | 112 | loop_gate self-loop node made a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property, rather than fixed structurally; flagged for plan 22-16's fixture audit (acceptance 2a) | waived | Design deviation recorded and audited in 22-16-SUMMARY.md (the per-fixture strandedness/readiness audit this row's own description names as owner): loop_gate's graph-entry arrangement in tests/integration/e2e_crash_resume_test.rs is classified 'readiness dodge', not strandedness -- Frontier::is_ready leaves a self-loop's own edge Pending until the node has run once, so a non-entry arrangement would deadlock the fixture permanently. The arrangement is kept by design; the module doc comment was corrected in 22-16 to name Frontier::is_ready as the real cause rather than removing anything. Cross-referenced by name in 29-04-SUMMARY.md's audit Section 4 as an already-open, already-scoped row -- not raised as an unmet FR by either 29-04-SUMMARY.md or 29-07-SUMMARY.md. | 2026-09-02T18:04:03.616Z | 2026-09-10T01:58:40.314Z |
| 25 | 22 | deviation | crates/paladin-battalion/src/engine/superstep.rs | 1220 | self_loop_graph test helper makes its looping node a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property (same root cause as e2e_crash_resume_test.rs); flagged for plan 22-16's fixture audit (acceptance 2a) | waived | Design deviation recorded and audited in 22-16-SUMMARY.md, same root cause and disposition as WINDOWS.md row 24: the self_loop_graph test helper in crates/paladin-battalion/src/engine/superstep.rs is classified 'readiness dodge' (Frontier::is_ready's Pending-self-edge behavior), kept by design since removing the graph-entry arrangement would deadlock all three tests it backs (self_loop_runs_exactly_three_times_when_approved_on_third_visit, self_loop_never_approved_trips_node_visit_limit_at_five, self_loop_at_four_visits_does_not_trip). Cross-referenced by name in 29-04-SUMMARY.md's audit Section 4 -- not raised as an unmet FR by either 29-04-SUMMARY.md or 29-07-SUMMARY.md. | 2026-09-02T18:04:11.984Z | 2026-09-10T01:58:40.477Z |
| 26 | 24 | unrun-verify | crates/paladin-storage/src/waypoint/contract_tests.rs |  | Phase 24's new Postgres Tier-2 contract-suite cases (awaiting_input_payload_round_trips, fork_of_round_trips, latest_prefers_most_recently_created_across_branches, D-02/D-14/D-15) self-skip locally (no Docker in this devcontainer) and are provable only via CI's postgres-integration job; never recorded as passed locally (D-28). | open |  | 2026-09-05T08:06:14.731Z |  |
| 27 | 24 | unrun-verify | Makefile |  | Phase 24's gate-evidence coverage measurement used cargo llvm-cov --workspace --features web-server (87.11% line coverage, above the 82% ADR-0006 floor) rather than the canonical make coverage/scripts/coverage.sh invocation (--features integration-tests,llm-all), because Redis and MinIO are unreachable -- no Docker daemon in this devcontainer, matching the Phase 17 precedent (row 13). Not recorded as CI's official figure. | open |  | 2026-09-05T08:06:30.095Z |  |
| 28 | 25 | unrun-verify | crates/paladin-storage/src/node_cache/redis.rs |  | RedisNodeCache Tier-2 live-server contract suite (redis_node_cache_runs_the_full_contract_suite, redis_keys_are_namespaced_by_the_configured_prefix, redis_ttl_is_set_on_the_server_not_only_in_the_payload) never executed against a real Redis server in any authoring environment -- Docker/Redis absent from devcontainer; evidence is CI-only via the new redis-cache-integration job | open |  | 2026-09-05T21:18:05.117Z |  |
| 29 | 27 | deviation | crates/paladin-web/src/assistant_controller.rs | 396 | GET /assistants merges synthetic code-registry entries only on the first page (cursor=None); a heterogeneous keyset merge across the stored repository and the in-process AgentRegistry across multiple pages is out of scope for 27-12 (documented in-code). | waived | Design deviation documented in-code (crates/paladin-web/src/assistant_controller.rs:396) and recorded in 27-12-SUMMARY.md (Phase 27): merging synthetic code-registry entries into GET /assistants only on the first page (cursor=None) is 27-12's own stated scope boundary -- a heterogeneous keyset merge across the stored repository and the in-process AgentRegistry spanning multiple pages was explicitly out of scope. Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md. | 2026-09-08T07:12:32.829Z | 2026-09-10T01:58:40.653Z |
| 30 | 27 | deviation | scripts/sdk-smoke/smoke.ts |  | TypeScript generated-client field/method names (Configuration/AssistantsApi/RunsApi) could not be verified against the real openapi-generator-cli output locally (no Java, no Docker in this devcontainer) -- CI's own sdk-clients job is the first real proof; if the generated shape differs, the fix is localized to smoke.py/smoke.ts's field access. | waived | Design deviation recorded in 27-18-SUMMARY.md (Phase 27, which created scripts/sdk-smoke/smoke.ts and the sdk-clients CI job) and reaffirmed in 27-21-SUMMARY.md's own key-decisions note: the generated TypeScript client's field/method names (Configuration/AssistantsApi/RunsApi) cannot be verified locally against real openapi-generator-cli output (no Java, no Docker in this devcontainer); 27-21-SUMMARY.md explicitly declined to commit a local ambient-module stub for exactly this reason, since it could silently diverge from CI's real generator output. CI's sdk-clients job is the first and only real proof and 27-CI-EVIDENCE.md records it PASS (job 102125436564, run 34245093476). Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md. | 2026-09-08T11:32:15.185Z | 2026-09-10T01:58:40.818Z |
| 31 | 27 | deviation | src/application/services/run/worker.rs |  | (WR-02) A run against a code-registered Runnable::Agent assistant is excluded from both the D-24 live event bus and the PLAT-FR-14 webhook delivery hook -- run_agent's two return paths write status/outcome and ack/nack directly, never reaching event_bus.bind/publish or webhook_deliveries.enqueue. Documented on the event_bus/webhook_deliveries field docs and on run_agent itself (worker.rs), mirrored in webhook/mod.rs's WebhookPayload docs and in docs/src/api-reference/platform-api.md's Webhooks Known limitations subsection, and pinned by agent_kind_run_with_a_webhook_enqueues_no_delivery (worker_tests.rs). Closing condition: wire both hooks into run_agent's two return paths (success and record_engine_failure) and invert the pinning test so it asserts a delivery IS enqueued. | waived | (WR-02) Design deviation documented in-code and recorded in 27-23-SUMMARY.md (Phase 27): a run against a code-registered Runnable::Agent assistant is excluded from both the D-24 live event bus and the PLAT-FR-14 webhook delivery hook by run_agent's two direct-write return paths in src/application/services/run/worker.rs. Documented on the event_bus/webhook_deliveries field docs, on run_agent itself, in webhook/mod.rs's WebhookPayload docs, in docs/src/api-reference/platform-api.md's Webhooks Known limitations subsection, and pinned by agent_kind_run_with_a_webhook_enqueues_no_delivery (worker_tests.rs). Closing condition (wiring both hooks into run_agent's two return paths) remains open follow-up work, not an unmet FR per 29-04-SUMMARY.md/29-07-SUMMARY.md, which raise no finding against this row. | 2026-09-08T13:48:16.000Z | 2026-09-10T01:58:40.992Z |
| 32 | 27 | deviation | crates/paladin-web/src/run_controller.rs |  | (WR-03) The three run read routes (GET /runs, GET /runs/{run_id}, GET /runs/{run_id}/webhook-deliveries) require authentication only -- RunQuery carries no caller identity and neither RunRepositoryPort::list/get nor WebhookDeliveryRepositoryPort::list_for_run applies a requester-derived filter, so any authenticated principal of any role can read every run in the deployment, including another caller's webhook target URL (secret redacted, URL not) and run_id enumeration is easier than for a random identifier since run_id is a time-ordered UUIDv7. Documented in run_controller.rs's module docs (Read scope section) and in docs/src/api-reference/platform-api.md's Authentication and scopes section. Accepted for v0.10 as a single-tenant/mutually-trusted-principal deployment model (T-27-23-02). Closing condition: a per-caller/tenant filter on RunQuery/get/list_for_run across the controller and the repository adapters. | waived | (WR-03) Design deviation documented in-code and recorded in 27-23-SUMMARY.md (Phase 27, threat T-27-23-02): the three run read routes (GET /runs, GET /runs/{run_id}, GET /runs/{run_id}/webhook-deliveries) in crates/paladin-web/src/run_controller.rs require authentication only, with no per-caller/tenant filter on RunQuery/list/get, accepted for v0.10 as a single-tenant/mutually-trusted-principal deployment model. Documented in run_controller.rs's module docs (Read scope section) and docs/src/api-reference/platform-api.md's Authentication and scopes section. Closing condition (a per-caller/tenant filter across the controller and repository adapters) remains open follow-up work, not an unmet FR per 29-04-SUMMARY.md/29-07-SUMMARY.md. | 2026-09-08T13:48:16.000Z | 2026-09-10T01:58:41.161Z |
| 33 | 28 | deviation | src/application/services/run/events.rs |  | map_trace_event's ParleyRaised->parley and RunFinished->done/error payloads keep the top-level waypoint_id/parleys/status/message field NAMES but carry null/reduced content (no prompt/choices/expires_at, no cancelled-vs-halted distinction) since TraceEvent::ParleyRaised/RunFinished do not carry that data by design (D-05); full detail still reachable via GET /threads/{id}/state. | waived | Design deviation recorded and documented in 28-11-SUMMARY.md (Phase 28), by design per 28-CONTEXT.md D-05 (payloads are bounded by construction; values are opt-in, redacted, then capped): TraceEvent::ParleyRaised/RunFinished do not carry prompt/choices/expires_at or a cancelled-vs-halted distinction, so map_trace_event's parley/done/error payloads in src/application/services/run/events.rs keep the top-level field names but carry null/reduced content. Full detail remains reachable via GET /threads/{id}/state. Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md. | 2026-09-09T03:42:33.069Z | 2026-09-10T01:58:41.330Z |
| 34 | 28 | stub | src/application/cli/commands/run.rs | 268 | run_run_export: Waypoints-source + no-real-graph resolution derives no fired edges (visits only) -- empty placeholder GraphShape passed to from_waypoints when no real graph is resolved yet | waived | Design deviation/stub recorded in 28-13-SUMMARY.md (Phase 28): run_run_export's Waypoints-source path in src/application/cli/commands/run.rs:268 derives no fired edges (visits only) when no real graph is resolved yet, passing an empty placeholder GraphShape to from_waypoints. Recorded as a known, scoped limitation in that plan's own documentation, not a silently-shipped gap. Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md. | 2026-09-09T04:57:28.596Z | 2026-09-10T01:58:41.503Z |

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
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 03, filed 2026-08-02) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: redis.rs's live-server code paths reaching through self.conn are still exercised only by tests/integration/redis_queue_integration_test.rs, which requires Docker and has never run in this devcontainer; owner remains Phase 15 (PIPE) per the row's own text.",
    "recorded_at": "2026-08-02T15:41:28.892Z",
    "resolved_at": "2026-09-10T01:58:05.756Z"
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "07",
    "file": ".project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md",
    "line": null,
    "description": "Task 3's requested single combined commit for ADR-0016 + PRD annotation was split into two atomic commits (9e8db80, 71ea46e) per standard task_commit_protocol; both files present, no content impact.",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 07, filed 2026-08-06) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: this is a process note (a requested single combined commit was split into two atomic commits, 9e8db80 and 71ea46e, per standard task_commit_protocol) with confirmed no content impact; the row was simply never triaged before v0.9.0 shipped.",
    "recorded_at": "2026-08-06T18:09:04.871Z",
    "resolved_at": "2026-09-10T01:58:05.936Z"
  },
  {
    "id": 4,
    "kind": "deviation",
    "phase": "07",
    "file": ".project/Milestone_5-Workspace-Decomposition/Epic_2/prd-paladin-ports-extraction.md",
    "line": null,
    "description": "No fabricated 3rd strikethrough correction for CONTEXT.md D-08(5)'s anticipated section-1 Milestone 1/Epic 2 cross-reference — re-verified absent from live tree (matches ADR-0014's own flagged drift); acceptance criterion expecting >=3 strikethrough lines not met by design.",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 07, filed 2026-08-06) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the anticipated third strikethrough correction was re-verified absent from the live tree at filing time (matching ADR-0014's own flagged drift) -- the row documents an acceptance criterion expecting >=3 strikethrough lines that was not met by design, not an outstanding defect.",
    "recorded_at": "2026-08-06T18:09:08.207Z",
    "resolved_at": "2026-09-10T01:58:06.102Z"
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "14",
    "file": "Cargo.toml",
    "line": null,
    "description": "cargo test --workspace not run to completion for 14-01: system-wide disk exhaustion (830G/875G used, 0 avail on /workspace mount) blocked full workspace compile; targeted plan <verify> commands (paladin-ai lib config::agents, paladin-web full suite, paladin-server binary build, openapi drift guard, check-api-surface.sh) all passed",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 14, filed 2026-08-12) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: a full cargo test --workspace for plan 14-01 was never re-run after the system-wide disk exhaustion (830G/875G used) that blocked it at filing time; only the plan's own targeted <verify> commands (paladin-ai lib config::agents, paladin-web full suite, paladin-server binary build, openapi drift guard, check-api-surface.sh) are confirmed passing.",
    "recorded_at": "2026-08-12T16:51:08.832Z",
    "resolved_at": "2026-09-10T01:58:06.265Z"
  },
  {
    "id": 6,
    "kind": "unrun-verify",
    "phase": "14",
    "file": "N/A (workspace-wide)",
    "line": null,
    "description": "14-04: full 'cargo test --workspace' not run — shared /workspace mount at 99%25 (13G free), matching 14-01's documented disk-exhaustion condition; the plan's own targeted verify (cargo test --bin paladin-server --features web-server, cargo fmt --check, cargo clippy --all-targets --features web-server -- -D warnings) all ran to completion and passed",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 14, filed 2026-08-12) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: a full cargo test --workspace for plan 14-04 was never re-run after the shared /workspace mount's disk-pressure condition (13G free) that blocked it at filing time; only the plan's own targeted verify commands (paladin-server binary tests, cargo fmt --check, cargo clippy -D warnings) are confirmed passing.",
    "recorded_at": "2026-08-12T17:13:58.989Z",
    "resolved_at": "2026-09-10T01:58:06.436Z"
  },
  {
    "id": 7,
    "kind": "deviation",
    "phase": "14",
    "file": "CHANGELOG.md",
    "line": null,
    "description": "14-08's acceptance criterion expected >=2 'BREAKING' lines under the dated 0.8.0 section in root CHANGELOG.md; only 1 is present. 14-01 split the phase's two consumer-break BREAKING entries across root CHANGELOG.md (config-key rename) and crates/paladin-web/CHANGELOG.md (AgentAuthConfig field + OpenAPI scheme rename), one per file, per 14-01-SUMMARY.md's own D4 verification and this plan's own instruction to leave per-crate changelogs untouched. Both breaks are documented with a BREAKING entry and cite ADR-0040; only the single-file grep count in the plan's acceptance criteria was miscalibrated.",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 14, filed 2026-08-12) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the acceptance criterion's >=2-BREAKING-lines-in-one-file expectation was miscalibrated -- 14-01-SUMMARY.md's own D4 verification confirms the phase's two consumer-break BREAKING entries were correctly split one per file across root CHANGELOG.md and crates/paladin-web/CHANGELOG.md, both citing ADR-0040.",
    "recorded_at": "2026-08-12T18:05:57.086Z",
    "resolved_at": "2026-09-10T01:58:06.593Z"
  },
  {
    "id": 8,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": "SECURITY-EXCEPTIONS.md",
    "line": null,
    "description": "Plan 15.1-01 Task 2's inline verify python one-liner (block-split regex over the machine-readable register) fails with a pre-existing TOML parse error on the LAST exception block, because its lookahead doesn't stop before the trailing markdown code fence -- reproduced against the pre-edit file too, unrelated to this task's new row. Substituted an isolated per-block parse of just the new RUSTSEC-2026-0249 row (11/11 fields present) plus the real repo guard scripts/check-advisory-register.sh (exit 0) as equivalent proof.",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: the inline verify python one-liner's pre-existing lookahead bug over SECURITY-EXCEPTIONS.md's trailing markdown fence was never fixed or re-run; the substituted isolated per-block parse of the new RUSTSEC-2026-0249 row plus scripts/check-advisory-register.sh (exit 0) stands as the equivalent proof on record, not a re-run of the original check.",
    "recorded_at": "2026-08-14T00:49:21.261Z",
    "resolved_at": "2026-09-10T01:58:06.758Z"
  },
  {
    "id": 9,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": ".github/workflows/ci.yml",
    "line": null,
    "description": "Task 1 acceptance criterion 'git diff | grep -c \"^[+-].*cargo \"' returns 4 not 0 -- matches step *name* text ('Cache cargo registry' etc.) removed by the migration, not actual cargo invocations. Verified via 'run: cargo' scoped grep returning 0 changed invocations.",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the acceptance criterion's grep counted CI step *name* text ('Cache cargo registry' etc.) removed by the migration, not actual cargo invocations; the scoped 'run: cargo' grep returning 0 changed invocations was already performed and is the correct verification.",
    "recorded_at": "2026-08-14T14:22:48.884Z",
    "resolved_at": "2026-09-10T01:58:06.922Z"
  },
  {
    "id": 10,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": ".github/workflows/integration-tests.yml",
    "line": null,
    "description": "Task 2's first automated verify literally asserts survivors=={pre-commit.yml} after migration, but integration-tests.yml (3 hand-rolled cache blocks) is still present -- deletion is plan 15.1-05's job, not yet executed in this wave, exactly per this plan's own Recorded discretion resolutions section. Substituted an assertion expecting survivors=={pre-commit.yml, integration-tests.yml}, both counts matching (1 and 3 respectively).",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: integration-tests.yml's three hand-rolled cache blocks were still present at filing time by design (deletion deferred to plan 15.1-05); whether 15.1-05 actually removed them afterward was never re-checked against this row.",
    "recorded_at": "2026-08-14T14:22:56.039Z",
    "resolved_at": "2026-09-10T01:58:07.111Z"
  },
  {
    "id": 11,
    "kind": "unrun-verify",
    "phase": "15.1",
    "file": ".github/workflows/ci.yml",
    "line": null,
    "description": "Task 2 acceptance criterion 'grep -rc restore-keys ci.yml feature-flags.yml release.yml' returns 0 for ci.yml -- returns 2, both from pre-existing prose comments in the examples job (added by plan 15.1-01, lines ~268/271) explaining why a restore-keys fallback alone is insufficient, not an actual YAML restore-keys: key. Verified via structural YAML walk: no step's with block contains a restore-keys key in any of the three files.",
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 15.1, filed 2026-08-14) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). Nothing remains unproven: the 2 grep matches for restore-keys in ci.yml are pre-existing prose comments (lines ~268/271) explaining why a restore-keys fallback alone is insufficient, not actual YAML restore-keys keys; the structural YAML walk confirming no step's with-block contains the key was already performed.",
    "recorded_at": "2026-08-14T14:23:03.057Z",
    "resolved_at": "2026-09-10T01:58:07.279Z"
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
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 17, filed 2026-08-17) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: the ollama-test healthcheck's use of 'ollama list' instead of a curl-based /v1/models check was never validated against a running Docker daemon in any authoring environment; 'docker compose config' itself was never run, only a python yaml.safe_load syntax check.",
    "recorded_at": "2026-08-17T14:17:46.408Z",
    "resolved_at": "2026-09-10T01:58:07.447Z"
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
    "status": "waived",
    "reason": "Predates the v0.9.0 tag (Phase 17, filed 2026-08-18) and shipped inside v0.9.0 as accepted debt per .planning/milestones/v0.9.0-MILESTONE-AUDIT.md (status: tech_debt, 0 critical blockers). What remains unproven: the public-API surface snapshot .project/current-exports.txt still excludes KimiAdapter/QwenAdapter/GrokAdapter/OllamaAdapter/GeminiAdapter/OpenAiCompatibleAdapter (default-features-only regeneration); 17-REVIEW.md recorded this as IN-01, non-blocking, deliberately excluded from the 2026-08-18 gap-closure scope by an explicit interactive human decision -- carried forward as accepted debt, not re-measured here.",
    "recorded_at": "2026-08-18T02:11:05.490Z",
    "resolved_at": "2026-09-10T01:58:07.612Z"
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
    "status": "waived",
    "reason": "Design deviation recorded and reasoned in 22-11-SUMMARY.md (WarGraph::from_formation/from_phalanx/from_campaign bridge constructors, Phase 22): from_campaign extends the ENG-FR-19 default three-field schema with one dedicated LastWrite field per Paladin because a single shared output field under LastWrite would hard-conflict the instant two Campaign fan-out siblings execute in the same superstep -- a structural certainty for the general-DAG case this bridge supports. from_formation and from_phalanx remain literally exactly three fields, matching ENG-FR-19 precisely; every other must-have truth and acceptance criterion (byte-for-byte golden equivalence, legacy services untouched, NodeId/fingerprint determinism, empty-list handling) is met exactly as written, including the specific 'exactly three fields' test proven against from_formation's own schema. Not an unmet FR per 29-04-SUMMARY.md/29-07-SUMMARY.md, which raise no finding against this row.",
    "recorded_at": "2026-09-02T03:29:38.826Z",
    "resolved_at": "2026-09-10T01:58:40.149Z"
  },
  {
    "id": 24,
    "kind": "deviation",
    "phase": "22",
    "file": "tests/integration/e2e_crash_resume_test.rs",
    "line": 112,
    "description": "loop_gate self-loop node made a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property, rather than fixed structurally; flagged for plan 22-16's fixture audit (acceptance 2a)",
    "status": "waived",
    "reason": "Design deviation recorded and audited in 22-16-SUMMARY.md (the per-fixture strandedness/readiness audit this row's own description names as owner): loop_gate's graph-entry arrangement in tests/integration/e2e_crash_resume_test.rs is classified 'readiness dodge', not strandedness -- Frontier::is_ready leaves a self-loop's own edge Pending until the node has run once, so a non-entry arrangement would deadlock the fixture permanently. The arrangement is kept by design; the module doc comment was corrected in 22-16 to name Frontier::is_ready as the real cause rather than removing anything. Cross-referenced by name in 29-04-SUMMARY.md's audit Section 4 as an already-open, already-scoped row -- not raised as an unmet FR by either 29-04-SUMMARY.md or 29-07-SUMMARY.md.",
    "recorded_at": "2026-09-02T18:04:03.616Z",
    "resolved_at": "2026-09-10T01:58:40.314Z"
  },
  {
    "id": 25,
    "kind": "deviation",
    "phase": "22",
    "file": "crates/paladin-battalion/src/engine/superstep.rs",
    "line": 1220,
    "description": "self_loop_graph test helper makes its looping node a graph entry to sidestep the Frontier::is_ready self-loop join-deadlock property (same root cause as e2e_crash_resume_test.rs); flagged for plan 22-16's fixture audit (acceptance 2a)",
    "status": "waived",
    "reason": "Design deviation recorded and audited in 22-16-SUMMARY.md, same root cause and disposition as WINDOWS.md row 24: the self_loop_graph test helper in crates/paladin-battalion/src/engine/superstep.rs is classified 'readiness dodge' (Frontier::is_ready's Pending-self-edge behavior), kept by design since removing the graph-entry arrangement would deadlock all three tests it backs (self_loop_runs_exactly_three_times_when_approved_on_third_visit, self_loop_never_approved_trips_node_visit_limit_at_five, self_loop_at_four_visits_does_not_trip). Cross-referenced by name in 29-04-SUMMARY.md's audit Section 4 -- not raised as an unmet FR by either 29-04-SUMMARY.md or 29-07-SUMMARY.md.",
    "recorded_at": "2026-09-02T18:04:11.984Z",
    "resolved_at": "2026-09-10T01:58:40.477Z"
  },
  {
    "id": 26,
    "kind": "unrun-verify",
    "phase": "24",
    "file": "crates/paladin-storage/src/waypoint/contract_tests.rs",
    "line": null,
    "description": "Phase 24's new Postgres Tier-2 contract-suite cases (awaiting_input_payload_round_trips, fork_of_round_trips, latest_prefers_most_recently_created_across_branches, D-02/D-14/D-15) self-skip locally (no Docker in this devcontainer) and are provable only via CI's postgres-integration job; never recorded as passed locally (D-28).",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-05T08:06:14.731Z",
    "resolved_at": null
  },
  {
    "id": 27,
    "kind": "unrun-verify",
    "phase": "24",
    "file": "Makefile",
    "line": null,
    "description": "Phase 24's gate-evidence coverage measurement used cargo llvm-cov --workspace --features web-server (87.11% line coverage, above the 82% ADR-0006 floor) rather than the canonical make coverage/scripts/coverage.sh invocation (--features integration-tests,llm-all), because Redis and MinIO are unreachable -- no Docker daemon in this devcontainer, matching the Phase 17 precedent (row 13). Not recorded as CI's official figure.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-05T08:06:30.095Z",
    "resolved_at": null
  },
  {
    "id": 28,
    "kind": "unrun-verify",
    "phase": "25",
    "file": "crates/paladin-storage/src/node_cache/redis.rs",
    "line": null,
    "description": "RedisNodeCache Tier-2 live-server contract suite (redis_node_cache_runs_the_full_contract_suite, redis_keys_are_namespaced_by_the_configured_prefix, redis_ttl_is_set_on_the_server_not_only_in_the_payload) never executed against a real Redis server in any authoring environment -- Docker/Redis absent from devcontainer; evidence is CI-only via the new redis-cache-integration job",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-05T21:18:05.117Z",
    "resolved_at": null
  },
  {
    "id": 29,
    "kind": "deviation",
    "phase": "27",
    "file": "crates/paladin-web/src/assistant_controller.rs",
    "line": 396,
    "description": "GET /assistants merges synthetic code-registry entries only on the first page (cursor=None); a heterogeneous keyset merge across the stored repository and the in-process AgentRegistry across multiple pages is out of scope for 27-12 (documented in-code).",
    "status": "waived",
    "reason": "Design deviation documented in-code (crates/paladin-web/src/assistant_controller.rs:396) and recorded in 27-12-SUMMARY.md (Phase 27): merging synthetic code-registry entries into GET /assistants only on the first page (cursor=None) is 27-12's own stated scope boundary -- a heterogeneous keyset merge across the stored repository and the in-process AgentRegistry spanning multiple pages was explicitly out of scope. Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md.",
    "recorded_at": "2026-09-08T07:12:32.829Z",
    "resolved_at": "2026-09-10T01:58:40.653Z"
  },
  {
    "id": 30,
    "kind": "deviation",
    "phase": "27",
    "file": "scripts/sdk-smoke/smoke.ts",
    "line": null,
    "description": "TypeScript generated-client field/method names (Configuration/AssistantsApi/RunsApi) could not be verified against the real openapi-generator-cli output locally (no Java, no Docker in this devcontainer) -- CI's own sdk-clients job is the first real proof; if the generated shape differs, the fix is localized to smoke.py/smoke.ts's field access.",
    "status": "waived",
    "reason": "Design deviation recorded in 27-18-SUMMARY.md (Phase 27, which created scripts/sdk-smoke/smoke.ts and the sdk-clients CI job) and reaffirmed in 27-21-SUMMARY.md's own key-decisions note: the generated TypeScript client's field/method names (Configuration/AssistantsApi/RunsApi) cannot be verified locally against real openapi-generator-cli output (no Java, no Docker in this devcontainer); 27-21-SUMMARY.md explicitly declined to commit a local ambient-module stub for exactly this reason, since it could silently diverge from CI's real generator output. CI's sdk-clients job is the first and only real proof and 27-CI-EVIDENCE.md records it PASS (job 102125436564, run 34245093476). Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md.",
    "recorded_at": "2026-09-08T11:32:15.185Z",
    "resolved_at": "2026-09-10T01:58:40.818Z"
  },
  {
    "id": 31,
    "kind": "deviation",
    "phase": "27",
    "file": "src/application/services/run/worker.rs",
    "line": null,
    "description": "(WR-02) A run against a code-registered Runnable::Agent assistant is excluded from both the D-24 live event bus and the PLAT-FR-14 webhook delivery hook -- run_agent's two return paths write status/outcome and ack/nack directly, never reaching event_bus.bind/publish or webhook_deliveries.enqueue. Documented on the event_bus/webhook_deliveries field docs and on run_agent itself (worker.rs), mirrored in webhook/mod.rs's WebhookPayload docs and in docs/src/api-reference/platform-api.md's Webhooks Known limitations subsection, and pinned by agent_kind_run_with_a_webhook_enqueues_no_delivery (worker_tests.rs). Closing condition: wire both hooks into run_agent's two return paths (success and record_engine_failure) and invert the pinning test so it asserts a delivery IS enqueued.",
    "status": "waived",
    "reason": "(WR-02) Design deviation documented in-code and recorded in 27-23-SUMMARY.md (Phase 27): a run against a code-registered Runnable::Agent assistant is excluded from both the D-24 live event bus and the PLAT-FR-14 webhook delivery hook by run_agent's two direct-write return paths in src/application/services/run/worker.rs. Documented on the event_bus/webhook_deliveries field docs, on run_agent itself, in webhook/mod.rs's WebhookPayload docs, in docs/src/api-reference/platform-api.md's Webhooks Known limitations subsection, and pinned by agent_kind_run_with_a_webhook_enqueues_no_delivery (worker_tests.rs). Closing condition (wiring both hooks into run_agent's two return paths) remains open follow-up work, not an unmet FR per 29-04-SUMMARY.md/29-07-SUMMARY.md, which raise no finding against this row.",
    "recorded_at": "2026-09-08T13:48:16.000Z",
    "resolved_at": "2026-09-10T01:58:40.992Z"
  },
  {
    "id": 32,
    "kind": "deviation",
    "phase": "27",
    "file": "crates/paladin-web/src/run_controller.rs",
    "line": null,
    "description": "(WR-03) The three run read routes (GET /runs, GET /runs/{run_id}, GET /runs/{run_id}/webhook-deliveries) require authentication only -- RunQuery carries no caller identity and neither RunRepositoryPort::list/get nor WebhookDeliveryRepositoryPort::list_for_run applies a requester-derived filter, so any authenticated principal of any role can read every run in the deployment, including another caller's webhook target URL (secret redacted, URL not) and run_id enumeration is easier than for a random identifier since run_id is a time-ordered UUIDv7. Documented in run_controller.rs's module docs (Read scope section) and in docs/src/api-reference/platform-api.md's Authentication and scopes section. Accepted for v0.10 as a single-tenant/mutually-trusted-principal deployment model (T-27-23-02). Closing condition: a per-caller/tenant filter on RunQuery/get/list_for_run across the controller and the repository adapters.",
    "status": "waived",
    "reason": "(WR-03) Design deviation documented in-code and recorded in 27-23-SUMMARY.md (Phase 27, threat T-27-23-02): the three run read routes (GET /runs, GET /runs/{run_id}, GET /runs/{run_id}/webhook-deliveries) in crates/paladin-web/src/run_controller.rs require authentication only, with no per-caller/tenant filter on RunQuery/list/get, accepted for v0.10 as a single-tenant/mutually-trusted-principal deployment model. Documented in run_controller.rs's module docs (Read scope section) and docs/src/api-reference/platform-api.md's Authentication and scopes section. Closing condition (a per-caller/tenant filter across the controller and repository adapters) remains open follow-up work, not an unmet FR per 29-04-SUMMARY.md/29-07-SUMMARY.md.",
    "recorded_at": "2026-09-08T13:48:16.000Z",
    "resolved_at": "2026-09-10T01:58:41.161Z"
  },
  {
    "id": 33,
    "kind": "deviation",
    "phase": "28",
    "file": "src/application/services/run/events.rs",
    "line": null,
    "description": "map_trace_event's ParleyRaised->parley and RunFinished->done/error payloads keep the top-level waypoint_id/parleys/status/message field NAMES but carry null/reduced content (no prompt/choices/expires_at, no cancelled-vs-halted distinction) since TraceEvent::ParleyRaised/RunFinished do not carry that data by design (D-05); full detail still reachable via GET /threads/{id}/state.",
    "status": "waived",
    "reason": "Design deviation recorded and documented in 28-11-SUMMARY.md (Phase 28), by design per 28-CONTEXT.md D-05 (payloads are bounded by construction; values are opt-in, redacted, then capped): TraceEvent::ParleyRaised/RunFinished do not carry prompt/choices/expires_at or a cancelled-vs-halted distinction, so map_trace_event's parley/done/error payloads in src/application/services/run/events.rs keep the top-level field names but carry null/reduced content. Full detail remains reachable via GET /threads/{id}/state. Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md.",
    "recorded_at": "2026-09-09T03:42:33.069Z",
    "resolved_at": "2026-09-10T01:58:41.330Z"
  },
  {
    "id": 34,
    "kind": "stub",
    "phase": "28",
    "file": "src/application/cli/commands/run.rs",
    "line": 268,
    "description": "run_run_export: Waypoints-source + no-real-graph resolution derives no fired edges (visits only) -- empty placeholder GraphShape passed to from_waypoints when no real graph is resolved yet",
    "status": "waived",
    "reason": "Design deviation/stub recorded in 28-13-SUMMARY.md (Phase 28): run_run_export's Waypoints-source path in src/application/cli/commands/run.rs:268 derives no fired edges (visits only) when no real graph is resolved yet, passing an empty placeholder GraphShape to from_waypoints. Recorded as a known, scoped limitation in that plan's own documentation, not a silently-shipped gap. Not raised as an unmet FR by 29-04-SUMMARY.md or 29-07-SUMMARY.md.",
    "recorded_at": "2026-09-09T04:57:28.596Z",
    "resolved_at": "2026-09-10T01:58:41.503Z"
  }
]
````
