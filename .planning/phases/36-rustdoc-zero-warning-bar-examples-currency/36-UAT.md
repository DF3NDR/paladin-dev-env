---
status: complete
phase: 36-rustdoc-zero-warning-bar-examples-currency
source: 36-01-SUMMARY.md, 36-02-SUMMARY.md, 36-03-SUMMARY.md, 36-04-SUMMARY.md, 36-05-SUMMARY.md, 36-06-SUMMARY.md, 36-07-SUMMARY.md, 36-08-SUMMARY.md, 36-09-SUMMARY.md, 36-10-SUMMARY.md, 36-11-SUMMARY.md, 36-12-SUMMARY.md, 36-13-SUMMARY.md
started: 2026-09-18T01:36:59Z
updated: 2026-09-18T01:41:00Z
---

## Current Test

[testing complete]

<!-- Coverage confirmation (all 59 deliverables auto-covered): user confirmed "yes pass" -->

## Tests

### 1. [Plan 36-01 / D1] paladin-memory documents warning-free under default and --all-features
expected: paladin-memory documents warning-free under default and --all-features builds (RD-01, RD-66, RD-126 closed)
result: pass
source: automated
coverage_id: D1 (plan 36-01)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-memory --all-features --no-deps (exit 0); cargo doc -p paladin-memory --no-deps (0 warning: lines)

### 2. [Plan 36-01 / D2] paladin-ports documents warning-free under default and --all-features
expected: paladin-ports documents warning-free under default and --all-features builds (RD-51, RD-127 closed)
result: pass
source: automated
coverage_id: D2 (plan 36-01)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-ports --all-features --no-deps (exit 0); cargo doc -p paladin-ports --no-deps (0 warning: lines)

### 3. [Plan 36-01 / D3] paladin-storage documents warning-free under default and --all-feature
expected: paladin-storage documents warning-free under default and --all-features builds (RD-46, RD-128 closed)
result: pass
source: automated
coverage_id: D3 (plan 36-01)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-storage --all-features --no-deps (exit 0); cargo doc -p paladin-storage --no-deps (0 warning: lines)

### 4. [Plan 36-01 / D4] examples/token_economy_commissary.rs -- offline Commissary/window-reso
expected: examples/token_economy_commissary.rs -- offline Commissary/window-resolution capability demo (EX-109, EX-111, EX-112, EX-113, EX-114, EX-115)
result: pass
source: automated
coverage_id: D4 (plan 36-01)
verification: cargo build --example token_economy_commissary (exit 0, no --features); cargo run --example token_economy_commissary with OPENAI_API_KEY/ANTHROPIC_API_KEY/DEEPSEEK_API_KEY unset (exit 0)

### 5. [Plan 36-01 / D5] examples/README.md gains a Token Economy Examples section with matchin
expected: examples/README.md gains a Token Economy Examples section with matching TOC bullet
result: pass
source: automated
coverage_id: D5 (plan 36-01)
verification: grep -q '^### \\[token_economy_commissary\\.rs\\]' examples/README.md

### 6. [Plan 36-01 / D6] 36-EVIDENCE.md + 36-evidence/ evidence harness seeded with the baselin
expected: 36-EVIDENCE.md + 36-evidence/ evidence harness seeded with the baseline and this plan's closure table
result: pass
source: automated
coverage_id: D6 (plan 36-01)
verification: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md and 36-evidence/36-01-*.txt exist

### 7. [Plan 36-02 / D1] paladin-battalion documents warning-free under default and --all-featu
expected: paladin-battalion documents warning-free under default and --all-features builds; all 72 rows (RD-10..RD-45, RD-67..RD-102) across 34 location groups closed
result: pass
source: automated
coverage_id: D1 (plan 36-02)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-battalion --all-features --no-deps (exit 0); cargo doc -p paladin-battalion --no-deps (0 warning: lines, down from 36)

### 8. [Plan 36-02 / D2] No private item widened to pub, no rustdoc lint-suppression attribute
expected: No private item widened to pub, no rustdoc lint-suppression attribute added, no non-doc-comment line changed anywhere in the nine touched files
result: pass
source: automated
coverage_id: D2 (plan 36-02)
verification: grep -rn for the four private helper fn signatures (push_field, validate_schedulable, validate_aegis_undeclared_nodes, validate_parley_value_for_kind) plus analyze_and_select: all still non-pub; grep -rn 'allow(rustdoc::' crates/paladin-battalion/src: no output; git diff filtered to non-doc-marker lines: no output

### 9. [Plan 36-02 / D3] Workspace stays green after the doc-comment rewrites: cargo check --wo
expected: Workspace stays green after the doc-comment rewrites: cargo check --workspace --all-targets --all-features, cargo test --workspace --doc, cargo fmt --all -- --check, make api-surface
result: pass
source: automated
coverage_id: D3 (plan 36-02)
verification: cargo check --workspace --all-targets --all-features exit 0; cargo test --workspace --doc 462 passed/0 failed (unchanged from 36-01's baseline figure); cargo fmt --all -- --check clean; ./scripts/check-api-surface.sh .project/current-exports.txt reports unchanged (3959 items)

### 10. [Plan 36-02 / D4] 36-evidence/36-02-battalion.txt captures the per-crate sweep verbatim
expected: 36-evidence/36-02-battalion.txt captures the per-crate sweep verbatim (D-10, D-24)
result: pass
source: automated
coverage_id: D4 (plan 36-02)
verification: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-02-battalion.txt exists and contains both the default-feature diagnostic count and the all-features exit code

### 11. [Plan 36-03 / D1] trace.rs's ten-link module doc header resolves cleanly (RD-54..RD-63,
expected: trace.rs's ten-link module doc header resolves cleanly (RD-54..RD-63, RD-105..RD-114 closed)
result: pass
source: automated
coverage_id: D1 (plan 36-03)
verification: cargo doc -p paladin-ai-core --no-deps (0 lines matching 'container/trace' in the capture, down from the header's contribution to the crate's 14 default-feature diagnostics)

### 12. [Plan 36-03 / D2] directive.rs de-linked StateNode::run (RD-52, RD-103 closed) with no n
expected: directive.rs de-linked StateNode::run (RD-52, RD-103 closed) with no new dependency added
result: pass
source: automated
coverage_id: D2 (plan 36-03)
verification: git diff HEAD~1 -- crates/paladin-core/Cargo.toml (empty); cargo doc -p paladin-ai-core --no-deps (0 warnings)

### 13. [Plan 36-03 / D3] structured.rs resolved extract_json with an explicit in-crate path (RD
expected: structured.rs resolved extract_json with an explicit in-crate path (RD-53, RD-104 closed)
result: pass
source: automated
coverage_id: D3 (plan 36-03)
verification: cargo doc -p paladin-ai-core --no-deps (0 warnings); grep -n 'pub fn extract_json' confirms pre-existing pub, not widened

### 14. [Plan 36-03 / D4] webhook.rs resolved WebhookDelivery and WEBHOOK_DELIVERY_SCHEMA_VERSIO
expected: webhook.rs resolved WebhookDelivery and WEBHOOK_DELIVERY_SCHEMA_VERSION with explicit in-crate paths (RD-64, RD-65, RD-115, RD-116 closed); security-invariant prose unchanged
result: pass
source: automated
coverage_id: D4 (plan 36-03)
verification: cargo doc -p paladin-ai-core --no-deps (0 warnings); grep -n 'No signing key on the row' confirms lines 1-18 untouched

### 15. [Plan 36-03 / D5] paladin-ai-core documents warning-free under both bar commands; worksp
expected: paladin-ai-core documents warning-free under both bar commands; workspace stays green; make api-surface unchanged
result: pass
source: automated
coverage_id: D5 (plan 36-03)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-ai-core --all-features --no-deps (exit 0); cargo check --workspace --all-targets --all-features (exit 0); cargo test --workspace --doc (0 failed); cargo fmt --all -- --check (clean); make api-surface (unchanged, 3959 items)

### 16. [Plan 36-03 / D6] 36-evidence/36-03-core.txt captures the per-crate sweep verbatim (D-10
expected: 36-evidence/36-03-core.txt captures the per-crate sweep verbatim (D-10, D-24)
result: pass
source: automated
coverage_id: D6 (plan 36-03)
verification: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-03-core.txt exists and contains both the default-feature diagnostic count and the all-features exit code

### 17. [Plan 36-04 / D1] paladin-llm documents warning-free under default and --all-features bu
expected: paladin-llm documents warning-free under default and --all-features builds; all 13 rows (RD-47..RD-50, RD-117..RD-125) across 9 location groups closed
result: pass
source: automated
coverage_id: D1 (plan 36-04)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-llm --all-features --no-deps (exit 0); cargo doc -p paladin-llm --no-deps (0 warning: lines, down from 4)

### 18. [Plan 36-04 / D2] paladin-web documents warning-free under default and --all-features bu
expected: paladin-web documents warning-free under default and --all-features builds; all 11 rows (RD-07..RD-09, RD-129..RD-136) across 8 location groups closed
result: pass
source: automated
coverage_id: D2 (plan 36-04)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-web --all-features --no-deps (exit 0); cargo doc -p paladin-web --no-deps (0 warning: lines, down from 3)

### 19. [Plan 36-04 / D3] No private item widened to pub, no new rustdoc lint-suppression attrib
expected: No private item widened to pub, no new rustdoc lint-suppression attribute added, no docsrs conditional-attribute machinery introduced, no non-doc-comment line changed in either crate
result: pass
source: automated
coverage_id: D3 (plan 36-04)
verification: grep -rn for JWT_MIN_SEGMENT_LEN, PESSIMISTIC_TOKENS_PER_1000_BYTES, MAX_HISTORY_LIMIT, NOT_WIRED_MESSAGE, escape_for_script, map_parley_error: all still non-pub; grep -rn 'cfg_attr(docsrs' crates/paladin-llm/src crates/paladin-web/src: no output; git diff of both commits restricted to doc-comment lines only (verified by inspection)

### 20. [Plan 36-04 / D4] Workspace stays green after the doc-comment rewrites: cargo check --wo
expected: Workspace stays green after the doc-comment rewrites: cargo check --workspace --all-targets --all-features, cargo test --workspace --doc, cargo fmt --all -- --check, make api-surface
result: pass
source: automated
coverage_id: D4 (plan 36-04)
verification: cargo check --workspace --all-targets --all-features exit 0; cargo test --workspace --doc 0 failed; cargo fmt --all -- --check clean; make api-surface unchanged (3959 items)

### 21. [Plan 36-04 / D5] 36-evidence/36-04-llm-web.txt captures the per-crate sweeps for both p
expected: 36-evidence/36-04-llm-web.txt captures the per-crate sweeps for both paladin-llm and paladin-web verbatim (D-10, D-24)
result: pass
source: automated
coverage_id: D5 (plan 36-04)
verification: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-04-llm-web.txt exists and contains before/after captures and negative-evidence greps for both crates

### 22. [Plan 36-05 / D1] The five default-feature facade groups (RD-02..RD-06, followers RD-138
expected: The five default-feature facade groups (RD-02..RD-06, followers RD-138..RD-141, RD-143) de-linked; paladin-ai documents zero default-feature rustdoc warnings
result: pass
source: automated
coverage_id: D1 (plan 36-05)
verification: cargo doc -p paladin-ai --no-deps (0 lines matching warning:); cargo check --workspace --all-targets --all-features (exit 0)

### 23. [Plan 36-05 / D2] The two feature-gated facade groups (RD-137/cli, RD-142/otel) de-linke
expected: The two feature-gated facade groups (RD-137/cli, RD-142/otel) de-linked in place, no docsrs machinery added
result: pass
source: automated
coverage_id: D2 (plan 36-05)
verification: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-ai --all-features --no-deps (exit 0)

### 24. [Plan 36-05 / D3] Workspace-wide safety net stays green: cargo check --workspace --all-t
expected: Workspace-wide safety net stays green: cargo check --workspace --all-targets --all-features, cargo test --workspace --doc, cargo fmt --all -- --check, make api-surface (unchanged)
result: pass
source: automated
coverage_id: D3 (plan 36-05)
verification: cargo check exit 0; cargo test --workspace --doc 0 failed; cargo fmt --check exit 0; ./scripts/check-api-surface.sh .project/current-exports.txt reports unchanged

### 25. [Plan 36-05 / D4] 36-evidence/36-05-facade.txt captures per-task verification plus the i
expected: 36-evidence/36-05-facade.txt captures per-task verification plus the informational workspace-wide all-features run
result: pass
source: automated
coverage_id: D4 (plan 36-05)
verification: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-05-facade.txt exists and records both bar commands' output

### 26. [Plan 36-06 / D1] war_engine_configuration.rs demonstrates WaypointPort injection, Engin
expected: war_engine_configuration.rs demonstrates WaypointPort injection, EngineConfig (all five audited fields), the APP_ENGINE_MAX_SUPERSTEPS override, checkpoint history read-back, WaypointRetentionService pruning, and GRAPH_FINGERPRINT_VERSION (EX-62, EX-63, EX-64, EX-65, EX-66, EX-80)
result: pass
source: automated
coverage_id: D1 (plan 36-06)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example war_engine_configuration (exit 0); stdout inspected for all six capability markers

### 27. [Plan 36-06 / D2] control_flow_dynamic_routing.rs demonstrates a custom edge condition f
expected: control_flow_dynamic_routing.rs demonstrates a custom edge condition failing closed when unregistered then taken once registered, a nested Battalion subgraph, LLM-driven routing via MockLlmAdapter/LlmDecisionEvaluator, and the Muster fan-out cap enforced from an environment override (EX-67, EX-68, EX-69, EX-70)
result: pass
source: automated
coverage_id: D2 (plan 36-06)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example control_flow_dynamic_routing (exit 0); stdout inspected for all four capability markers

### 28. [Plan 36-06 / D3] Both programs are default-feature targets covered by the bulk cargo bu
expected: Both programs are default-feature targets covered by the bulk cargo build --examples selector, and neither commit touches src/ or crates/ -- make api-surface reports the surface unchanged
result: pass
source: automated
coverage_id: D3 (plan 36-06)
verification: cargo build --examples (exit 0, both binaries present); git diff --stat -- src/ crates/ (empty); make api-surface (API surface unchanged, 3959 items)

### 29. [Plan 36-07 / D1] human_in_the_loop_gate.rs demonstrates a run pausing at a Gate node, a
expected: human_in_the_loop_gate.rs demonstrates a run pausing at a Gate node, a typed resume_with total-validation rejection of an unrelated parley id followed by the correct response completing the run, and a ChronicleService history read-back plus a replay onto a new branch resumed with the opposite decision to a divergent result (EX-71, EX-72, EX-73)
result: pass
source: automated
coverage_id: D1 (plan 36-07)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example human_in_the_loop_gate (exit 0); stdout inspected for the pause, the typed rejection, the completion, the chronicle history, and the divergent branch result

### 30. [Plan 36-07 / D2] graceful_shutdown.rs demonstrates ShutdownCoordinator draining a fan-o
expected: graceful_shutdown.rs demonstrates ShutdownCoordinator draining a fan-out of in-flight work (one node finishing, one aborted and recorded Skipped in the same Halted checkpoint), the APP_ENGINE_SHUTDOWN_GRACE_SECS override bounding a real run's abort at the new value, and the APP_ENGINE_GRACEFUL_SHUTDOWN toggle's exit-immediately vs. wait-and-drain contrast, all without installing a real signal handler (EX-74, EX-75, EX-76)
result: pass
source: automated
coverage_id: D2 (plan 36-07)
verification: timeout 120 env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example graceful_shutdown (exit 0, ~1.4s wall time); stdout inspected for the drain report, the grace-period before/after and bounded-abort timing, and the toggle contrast

### 31. [Plan 36-07 / D3] Both programs are default-feature targets covered by the bulk cargo bu
expected: Both programs are default-feature targets covered by the bulk cargo build --examples selector, neither commit touches src/ or crates/, and make api-surface reports the surface unchanged
result: pass
source: automated
coverage_id: D3 (plan 36-07)
verification: cargo build --examples (exit 0, both binaries present); git diff --stat -- src/ crates/ (empty); ./scripts/check-api-surface.sh .project/current-exports.txt (API surface unchanged, 3959 items)

### 32. [Plan 36-08 / D1] agent_runtime_middleware.rs demonstrates a custom ExecutionMiddleware'
expected: agent_runtime_middleware.rs demonstrates a custom ExecutionMiddleware's three hooks firing in order, AgentRuntimeConfig::build_chain resolving built-in middleware from configuration, a custom TokenCounterPort contrasted with the defaulted heuristic, HistoryTrimmer + SummarizationMiddleware reducing a long history, ConfinedVault namespacing with a denied cross-namespace read, and the fail-run tool error mode's structured PaladinError::ArmamentFailed (EX-83, EX-84, EX-85, EX-86, EX-87, EX-89)
result: pass
source: automated
coverage_id: D1 (plan 36-08)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example agent_runtime_middleware (exit 0); stdout inspected for all six capability markers

### 33. [Plan 36-08 / D2] structured_output_schema.rs demonstrates a JSON Schema derived from a
expected: structured_output_schema.rs demonstrates a JSON Schema derived from a Rust type via schemars::schema_for! and a typed value returned by the structured-execution path, plus a schema-violating response rejected via PaladinError::StructuredOutputInvalid rather than silently accepted (EX-88, EX-90)
result: pass
source: automated
coverage_id: D2 (plan 36-08)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example structured_output_schema (exit 0); stdout inspected for the derived schema, the typed value's fields, and the typed rejection

### 34. [Plan 36-08 / D3] sanctum_rag_retrieval.rs demonstrates a retrieval result's typed conte
expected: sanctum_rag_retrieval.rs demonstrates a retrieval result's typed context, its ShedItem shed records, a typed RagRetrievalError from a deliberately failing retrieval, the timeout-bounded retrieve_context_with_timeout free function, and an exact TokenCounterPort injected via with_token_counter, as a runnable sibling that leaves the existing paladin_with_rag.rs untouched (EX-116, EX-117, EX-118, EX-119, EX-120)
result: pass
source: automated
coverage_id: D3 (plan 36-08)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example sanctum_rag_retrieval (exit 0); stdout inspected for all five capability markers; git status --porcelain examples/paladin_with_rag.rs empty

### 35. [Plan 36-08 / D4] All three programs are default-feature targets covered by the bulk car
expected: All three programs are default-feature targets covered by the bulk cargo build --examples selector, no commit touches src/ or crates/, and make api-surface / check-api-surface.sh reports the surface unchanged across all three commits
result: pass
source: automated
coverage_id: D4 (plan 36-08)
verification: cargo build --examples (exit 0, all three binaries present); git diff --stat HEAD~3..HEAD -- src/ crates/ (empty); ./scripts/check-api-surface.sh .project/current-exports.txt (unchanged, 3959 items, checked after each commit)

### 36. [Plan 36-08 / D5] The post-audit drift in the five RAG capability-token greps (RagRetrie
expected: The post-audit drift in the five RAG capability-token greps (RagRetrievalResult, ShedItem, RagRetrievalError, retrieve_context_with_timeout, with_token_counter) is re-checked against examples/ and crates/doc-examples/src/ and recorded verbatim rather than skipped (D-02)
result: pass
source: automated
coverage_id: D5 (plan 36-08)
verification: 36-evidence/36-08-examples.txt Task 3 drift table: 3 of 5 tokens confirmed stale against crates/doc-examples/src/sanctum_vector_memory.rs (Phase 35), examples/ gallery itself still 0 hits pre-commit

### 37. [Plan 36-09 / D1] examples/http_service_host.rs and crates/doc-examples/src/http_service
expected: examples/http_service_host.rs and crates/doc-examples/src/http_service_host.rs mount agent_router, thread_router and run_router in the shipped merge order (agent, then thread, then run, then docs), closing the EX-33/EX-55 router-parity defect; the example's drive sequence calls one thread route and one run route and prints the status each got
result: pass
source: automated
coverage_id: D1 (plan 36-09)
verification: cargo run --example http_service_host --features web-server (exit 0); stdout shows GET /v1/threads/demo-thread/state -> 501 and GET /v1/runs/{run_id} -> 501; ./scripts/check-doc-examples.sh and mdbook build docs/ both exit 0 after the doc-examples module edit

### 38. [Plan 36-09 / D2] examples/platform_api_client.rs demonstrates the Platform API surface
expected: examples/platform_api_client.rs demonstrates the Platform API surface in-process and offline: submit/stream/cancel a run, create+list assistant versions, create+list a schedule, thread state/resume/history, the dev-ui inspector route, token usage (prompt/completion split, never a bare total), and queue/store backend selection (EX-77, EX-78, EX-79, EX-91, EX-92, EX-93, EX-94, EX-95, EX-98, EX-99, EX-104, EX-110)
result: pass
source: automated
coverage_id: D2 (plan 36-09)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example platform_api_client --features \"web-server,dev-ui\" (exit 0); stdout inspected for all twelve capability markers, including the documented thread-state/dev-ui deviation

### 39. [Plan 36-09 / D3] examples/webhook_receiver.rs demonstrates signature verification over
expected: examples/webhook_receiver.rs demonstrates signature verification over raw captured bytes reusing the shipped sign_webhook_body function, rejects a tampered body via constant-time hmac::Mac::verify_slice, and demonstrates the APP_WEBHOOKS_ALLOW_PRIVATE override against the receiver's own loopback address plus the always-rejected cloud metadata address (EX-96, EX-97)
result: pass
source: automated
coverage_id: D3 (plan 36-09)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example webhook_receiver --features web-server (exit 0); stdout shows a verified genuine delivery, a rejected tampered body, and the SSRF override narration; secret never printed (grep -cE for secret/signing_key print patterns is 0)

### 40. [Plan 36-09 / D4] Both new programs are declared as gated example targets (required-feat
expected: Both new programs are declared as gated example targets (required-features), the bare cargo build --examples selector skips them, and no commit in this plan touches src/ or crates/paladin-* except the doc-examples HTTP-service-host module; make api-surface / check-api-surface.sh reports the surface unchanged across all three commits
result: pass
source: automated
coverage_id: D4 (plan 36-09)
verification: cargo build --examples (exit 0, gated targets skipped); git diff --stat HEAD~3..HEAD -- src/ crates/ (only crates/doc-examples/src/http_service_host.rs); ./scripts/check-api-surface.sh .project/current-exports.txt (unchanged, 3959 items, checked after each commit)

### 41. [Plan 36-10 / D1] node_result_cache.rs constructs the Redis-backed node-result cache ada
expected: node_result_cache.rs constructs the Redis-backed node-result cache adapter the redis-cache feature enables and wires it onto a WarEngine via with_node_cache, runs a graph twice to show a miss then a hit (no re-execution, proven via an execution counter and the persisted Waypoint's cache_hit field), then toggles APP_NODE_CACHE_ENABLED off and re-runs against a graph with no CachePolicy attached to show the cache bypassed entirely (EX-81, EX-82)
result: pass
source: automated
coverage_id: D1 (plan 36-10)
verification: cargo build --example node_result_cache --features \"redis-cache\" (exit 0); cargo build --examples (exit 0, target skipped); program built-not-run per D-16 (no Redis server in this devcontainer or CI)

### 42. [Plan 36-10 / D2] observability_tracing.rs demonstrates the trace envelope (real TraceEv
expected: observability_tracing.rs demonstrates the trace envelope (real TraceEvent variant names via a custom in-process TraceSink), TraceConfig configuration (build_run_sink's None-vs-Some resolution across a changed log_sink field), the PALADIN_TRACE_OTEL_ENABLED environment toggle (and its interaction with the otel Cargo feature this build lacks), and the persisted trace history read back via RunTracePort::read (EX-100, EX-101, EX-102, EX-108)
result: pass
source: automated
coverage_id: D2 (plan 36-10)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example observability_tracing (exit 0); stdout inspected for all four capability markers -- 7 TraceRecords captured, 7 distinct real event variant names, None-vs-Some sink resolution printed, otel toggle before/after printed, 7 persisted rows read back

### 43. [Plan 36-10 / D3] observability_otel_export.rs wires the otel-gated OtelTraceSink onto a
expected: observability_otel_export.rs wires the otel-gated OtelTraceSink onto a WarEngine run and shows the endpoint configuration it exports through, build-verified only per D-16 (no reachable OTLP collector in this devcontainer or CI)
result: pass
source: automated
coverage_id: D3 (plan 36-10)
verification: cargo build --example observability_otel_export --features \"otel\" (exit 0); program built-not-run per D-16

### 44. [Plan 36-10 / D4] eval_scenarios_demo.rs declares two scenarios in Rust against a Paladi
expected: eval_scenarios_demo.rs declares two scenarios in Rust against a Paladin built with the mock adapter and runs them through ScenarioRunner::run_case, names the PALADIN_EVAL_LIVE toggle's effect via check_live_mode(false)'s typed refusal, and drives a written .eval.yaml file's glob through ScenarioRunner::trials/run_case in-process while printing the equivalent CLI command (EX-105, EX-106, EX-107)
result: pass
source: automated
coverage_id: D4 (plan 36-10)
verification: env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example eval_scenarios_demo (exit 0); stdout inspected for all three capability markers -- both declared cases PASSED, check_live_mode(false) -> Err(FlagNotSet) printed, glob resolves 1 trial and the written-file case PASSED

### 45. [Plan 36-10 / D5] Both gated targets are declared in the root manifest with required-fea
expected: Both gated targets are declared in the root manifest with required-features, the bare cargo build --examples selector skips them, no commit in this plan touches src/ or crates/, and make api-surface / check-api-surface.sh reports the surface unchanged across all four commits
result: pass
source: automated
coverage_id: D5 (plan 36-10)
verification: cargo build --examples (exit 0, both gated targets skipped); git diff --stat HEAD~4..HEAD -- src/ crates/ (empty); ./scripts/check-api-surface.sh .project/current-exports.txt (unchanged, 3959 items, checked after every commit)

### 46. [Plan 36-11 / D1] EX-01 closed: examples/README.md's Getting Started block states the wo
expected: EX-01 closed: examples/README.md's Getting Started block states the workspace's real minimum Rust version (1.88, matching Cargo.toml's rust-version), and every feature name in the 'Run with Specific Features' block (redis-queue, s3-storage) exists in the root Cargo.toml [features] table
result: pass
source: automated
coverage_id: D1 (plan 36-11)
verification: RUSTV=$(grep -m1 '^rust-version' Cargo.toml | sed 's/.*\"\\(.*\\)\".*/\\1/'); grep -q \"Rust $RUSTV\" examples/README.md (exit 0, RUSTV=1.88)

### 47. [Plan 36-11 / D2] EX-122 closed: the three drifted PaladinResult snippet lines (Basic Pa
expected: EX-122 closed: the three drifted PaladinResult snippet lines (Basic Paladin Examples, Logging and Observability, Building a Custom Example) now read the real field names -- output, usage.total_tokens, execution_time_ms -- and no README line names a PaladinResult field that does not exist
result: pass
source: automated
coverage_id: D2 (plan 36-11)
verification: grep -cE 'response\\.content|response\\.token_usage|response\\.execution_time([^_]|$)' examples/README.md -> 0

### 48. [Plan 36-11 / D3] EX-121 closed: all eleven previously unlisted on-disk programs (comman
expected: EX-121 closed: all eleven previously unlisted on-disk programs (commander_council.rs, commander_grove.rs, conclave_expert_panel.rs, council_discussion.rs, document_processing.rs, grove_routing.rs, http_service_host.rs, paladin_with_rag.rs, vision_analysis.rs, vision_battalion.rs, war_engine_memory_baseline.rs) now have a ### [name.rs] section and a matching TOC entry
result: pass
source: automated
coverage_id: D3 (plan 36-11)
verification: comm -23 against the eleven-name list and the extracted ### [name.rs] header list -> empty

### 49. [Plan 36-11 / D4] All 24 previously-undocumented on-disk programs (the eleven from EX-12
expected: All 24 previously-undocumented on-disk programs (the eleven from EX-121 plus the thirteen new Phase 36 programs) have a README section, every 59-row EX-nn gap capability has exactly one Demonstrates line naming it, and the both-directions listed-vs-on-disk cross-check is empty (62 on-disk .rs files == 62 listed sections == 62 Demonstrates lines)
result: pass
source: automated
coverage_id: D4 (plan 36-11)
verification: comm -23 and comm -13 between /tmp/36-11-ondisk.txt (62) and /tmp/36-11-listed.txt (62) both empty; grep -c '^\\*\\*Demonstrates:\\*\\*' examples/README.md == 62

### 50. [Plan 36-11 / D5] The single README commit (D-26) touches only examples/README.md and th
expected: The single README commit (D-26) touches only examples/README.md and this plan's own evidence file -- no example .rs file, no Cargo.toml, no CI/script/entry-point file is modified
result: pass
source: automated
coverage_id: D5 (plan 36-11)
verification: git diff --stat HEAD~1 HEAD -> examples/README.md and 36-evidence/36-11-readme.txt only

### 51. [Plan 36-12 / D1] scripts/check-all-examples.sh rewritten to run the same 7 CI Example M
expected: scripts/check-all-examples.sh rewritten to run the same 7 CI Example Muster invocations (no cargo check --example, no --all-features) plus the binary-count assertion; make check-examples target added, not wired into clean-code or the pre-push hook
result: pass
source: automated
coverage_id: D1 (plan 36-12)
verification: make check-examples (exit 0, 62/62 binaries); make lint-shell (exit 0); grep -c 'cargo check --example' scripts/check-all-examples.sh = 0; grep -c -- '--all-features' scripts/check-all-examples.sh = 0; grep -c 'clean-code: .*check-examples' Makefile = 0; git diff --stat HEAD~1 -- src/ crates/ examples/ empty

### 52. [Plan 36-12 / D2] make doc-check wired as the single local source of truth for both ADR-
expected: make doc-check wired as the single local source of truth for both ADR-0033 bar commands plus cargo test --workspace --doc, in order; clean-code now depends on doc-check; test-doc and doc (--open) kept unchanged
result: pass
source: automated
coverage_id: D2 (plan 36-12)
verification: make doc-check (exit 0, three labelled steps); grep -qE '^clean-code: .*doc-check' Makefile; grep -c '^test-doc:' Makefile = 1; grep -c '^doc:' Makefile = 1

### 53. [Plan 36-12 / D3] .pre-commit-config.yaml gained a doc-check pre-push hook (same files f
expected: .pre-commit-config.yaml gained a doc-check pre-push hook (same files filter as check-api-surface); ci.yml lint job gained 'Check documentation (all features, -D warnings)' immediately after the existing documentation step, with the workspace doctest step NOT duplicated
result: pass
source: automated
coverage_id: D3 (plan 36-12)
verification: grep -q 'id: doc-check' .pre-commit-config.yaml; grep -q 'Check documentation (all features, -D warnings)' .github/workflows/ci.yml; grep -c 'cargo test --workspace --doc' .github/workflows/ci.yml = 1

### 54. [Plan 36-12 / D4] CI Example Muster job now builds all 8 gated example targets (platform
expected: CI Example Muster job now builds all 8 gated example targets (platform_api_client, node_result_cache, observability_otel_export added; webhook_receiver folded into the existing web-server step) and its explanatory comment / binary-count assertion are corrected to the real 62-file / 8-target / 54-auto-discovered counts, matching scripts/check-all-examples.sh's invocation list one-for-one
result: pass
source: automated
coverage_id: D4 (plan 36-12)
verification: grep -c 'run: cargo build --example' .github/workflows/ci.yml = 7 (matches script); find examples -name '*.rs' | wc -l = 62 matches the comment; grep -c '^\\[\\[example\\]\\]' Cargo.toml = 8 matches the comment

### 55. [Plan 36-12 / D5] Closing zero measurement: both ADR-0033 bar commands exit 0 on the gat
expected: Closing zero measurement: both ADR-0033 bar commands exit 0 on the gate commit, cargo test --workspace --doc holds at the 462 passed / 0 failed / 210 ignored baseline exactly, make check-examples and make api-surface (3959 items) are unchanged, and the gate-wiring edits plus this measurement land in one commit so git bisect never lands on a commit where the gate exists but fails (D-13)
result: pass
source: automated
coverage_id: D5 (plan 36-12)
verification: cargo doc --workspace --no-deps | tee ... && ! grep -q warning: (exit 0, 0 lines); RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --all-features --no-deps (exit 0); cargo test --workspace --doc (462/0/210); ./scripts/check-api-surface.sh (3959 items unchanged); git show --stat HEAD lists only Makefile, .pre-commit-config.yaml, .github/workflows/ci.yml and the closing-measurement evidence file, no .rs file

### 56. [Plan 36-13 / D1] 36-EVIDENCE.md carries the full 143 RD-nn + 64 EX-nn closure map, each
expected: 36-EVIDENCE.md carries the full 143 RD-nn + 64 EX-nn closure map, each row traced to a commit, with the two documented drift observations and the baseline-vs-closing measurement table
result: pass
source: automated
coverage_id: D1 (plan 36-13)
verification: grep -oE 'RD-[0-9]+' 36-EVIDENCE.md | sort -u | wc -l -> 143 (plus 1 not-minted mention of RD-144); grep -oE 'EX-[0-9]+' 36-EVIDENCE.md | sort -u | wc -l -> 64 (plus 1 not-minted mention of EX-123)

### 57. [Plan 36-13 / D2] WINDOWS.md rows 36 and 37 both read fixed, moved only through gsd-tool
expected: WINDOWS.md rows 36 and 37 both read fixed, moved only through gsd-tools
result: pass
source: automated
coverage_id: D2 (plan 36-13)
verification: grep -n '^| 3[67] ' .planning/WINDOWS.md -> both rows show status 'fixed' with a non-null resolved_at timestamp

### 58. [Plan 36-13 / D3] CHANGELOG.md [0.10.0] Documentation section carries four reader-facing
expected: CHANGELOG.md [0.10.0] Documentation section carries four reader-facing Phase 36 bullets, naming no RD-nn/EX-nn identifier
result: pass
source: automated
coverage_id: D3 (plan 36-13)
verification: git show 12aaf84e -- CHANGELOG.md (four bullets appended after the existing Phase 35 entries); grep -cE 'RD-[0-9]+|EX-[0-9]+' CHANGELOG.md -> 0

### 59. [Plan 36-13 / D4] 36-CI-EVIDENCE.md records the real pushed-branch CI run proving the ne
expected: 36-CI-EVIDENCE.md records the real pushed-branch CI run proving the new all-features documentation gate, the doctest step, and the Example Muster job
result: pass
source: automated
coverage_id: D4 (plan 36-13)
verification: gh run view 35290763563 --json jobs (Code Quality, Unit Tests (stable), Unit Tests (beta), Example Muster (Feature Matrix) all conclusion=success); raw logs ci-job-lint.log / ci-job-unit.log / ci-job-examples.log cross-checked

## Summary

total: 59
passed: 59
issues: 0
pending: 0
skipped: 0

## Gaps

[none yet]
