---
phase: 28
slug: observability-tooling
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (high)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-09-09
last_audit: 2026-09-09
---

# Phase 28 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

**Verdict: THREAT-SECURE.** All 75 threats closed; no open threat remains at or above the
`high` blocking threshold. The auditor returned `SECURED` with 74/75 closed and one `low`
accepted-risk row (T-28-05-03) whose plan-cited rustdoc rationale was missing from the code;
the orchestrator closed it in this same commit by adding the rationale to `requests()`'s
rustdoc (`crates/paladin-eval/src/scripted_llm.rs`), mirroring the pattern `assertion.rs`
already used for T-28-08-03.

Register origin: `register_authored_at_plan_time: true` — every one of the 17 plans in this
phase carried a parseable `<threat_model>` block. The auditor therefore verified mitigations
rather than building a retroactive register. Severity breakdown: 27 high ·
39 medium · 9 low. Disposition: 69 mitigate · 6 accept.

---

## Trust Boundaries

Consolidated from the 17 plan-level boundary tables.

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Battlefield state → `TraceRecord` / `FieldChange` | Field values (user data, possibly credentials) could cross into an observability payload | State values — `None` by default, redact-then-truncate when enabled |
| Engine run path → sink implementations | Third-party or misbehaving sink code runs on the observability path; a panic or block must not reach the run | Trace events |
| Node heartbeat loop → bounded trace queue | A flooding node could evict real events from the drop-oldest queue | Heartbeat events |
| Trace record → process log stream | Every record becomes a log line an operator, log shipper or aggregator can read | Node ids, outcomes, byte counts |
| Trace record → persisted `run_traces` row → replayed stream | Observability data becomes durable, readable state served back to callers | Rows keyed by `ThreadId` / `seq` |
| Trace record → SSE wire → API client | Internal execution detail crosses to an authenticated external consumer | Field names and byte counts only |
| Operator config file → `OtelConfig.headers` → HTTP request / logs / errors | OTLP headers are credential-shaped values moving through the exporter | Secret-by-assumption header values |
| Process → operator-configured OTLP collector endpoint | Outbound HTTP to a configured host carrying credential-shaped headers | Trace attributes: ids, outcomes, costs |
| Caller-supplied `ThreadId` → SQL query / storage reads | An identifier from an untrusted caller reaches the storage layer | Bound parameters |
| Stored execution history → `InspectorView` → web layer / HTML document | Run structure crosses toward an HTTP surface and is embedded in markup a browser parses | Inspector JSON, Mermaid-URL JSON |
| HTTP client → the `dev-ui` route | An external request reaches operator-grade information about run structure | Run structure, field names |
| Operator browser → configured Mermaid CDN | The page loads and executes third-party JavaScript at runtime | Third-party script |
| Graph node ids / condition text / observed edges → rendered diagram | Author-supplied identifiers become DOT / Mermaid markup | Escaped identifiers |
| Waypoint / trace history → developer terminal / file (`paladin-cli run`, `graph`) | Run history becomes a shareable artifact outside the server's auth boundary | Ids, outcomes, costs; never values |
| CLI arguments → filesystem and storage | Caller-supplied paths, thread ids and run ids drive reads and an optional `--out` write | Paths, ids |
| `.eval.yaml` scenario file → `paladin-eval` harness | A scenario file is untrusted structured input parsed by a harness that then runs a graph | Assertion params, regexes, JSON paths, `graph_doc` paths, registered names |
| Scenario glob / `graph_doc` path → filesystem | Caller-supplied patterns select files the harness reads and, with `--bless`, writes beside | Paths |
| Scenario files in the repository → the default test run | Committed scenario content is executed by every developer's `cargo test` | Scripted fixtures |
| Live mode → external LLM providers with real API keys | A test harness can be made to spend credits and send fixture prompts to a third party | Prompts, API keys |
| Failure rendering → developer console / CI log | Rendered failures include observed state and captured prompts | Fixture content |
| Documentation → operator configuration decisions | A documented example an operator copies becomes their production configuration | Example OTLP headers, `state_values` flag |
| Release pipeline crate lists → crates.io | A registration error publishes or omits a crate (`paladin-eval` is new) | Crate publish order |

---

## Threat Register

75 threats across plans 28-01..28-17. Verified by `gsd-security-auditor` at ASVS L1 depth on
2026-09-09 (grep/presence-level, with L2 data-flow tracing where a single grep was insufficient:
redact-before-truncate ordering, composition-root reachability). The `Mitigation` column is the
plan-time mitigation plan; the verification evidence for every `high` threat is in *Verified
Mitigation Highlights* below.

| Threat ID | Plan | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|------|----------|-----------|----------|-------------|------------|--------|
| T-28-01-01 | 28-01 | Information disclosure | `FieldChange` payload in `TraceRecord` | high | mitigate | `field`/`dispatch`/`writers`/`value_bytes` only; `value` is `None` unless explicitly enabled, and then redacted before truncation (D-05); module docs carry the rule. | closed |
| T-28-01-02 | 28-01 | Denial of service | Sink blocking or panicking on the run path | high | mitigate | Bounded drop-oldest queue, never awaited on the run path, `catch_unwind` per sink call, consumer never breaks; timing test asserts run independence (D-08). | closed |
| T-28-01-03 | 28-01 | Repudiation | Silent event loss making the trace an unreliable account | medium | mitigate | Drops counted, reconciled three ways, `RunFinished.trace_dropped_total` stamped, first drop warned (D-06, D-07). | closed |
| T-28-01-04 | 28-01 | Tampering | Cross-run `seq` contamination under concurrency | medium | mitigate | Per-dispatcher atomic counter, X-05 stress test asserts sixteen independent gapless sequences. | closed |
| T-28-02-01 | 28-02 | Information disclosure | `OtelConfig.headers` in `Debug` output or logs | high | mitigate | Manual `Debug` printing header keys with redacted values; no `Debug` derive on the type; no log line interpolates a header value (D-36). | closed |
| T-28-02-02 | 28-02 | Spoofing | `otel.endpoint` pointing at a non-`http(s)` or malformed destination | medium | mitigate | `validate_typed` rejects any non-`http(s)` scheme when `otel.enabled`; the exporter's own no-redirect client is 28-09's control. | closed |
| T-28-02-03 | 28-02 | Repudiation | `otel.enabled` silently doing nothing on a build without the feature | medium | mitigate | Typed `FeatureNotCompiled` error at validation time, never a silent no-op (D-36). | closed |
| T-28-02-04 | 28-02 | Information disclosure | `state_values` accidentally on in a shipped example config | medium | accept | Default is `false` in both `Default` and both YAML files; the comment states the consequence. Residual risk is an operator's deliberate change. | closed |
| T-28-03-01 | 28-03 | Information disclosure | `FieldChange.value` when value inclusion is enabled | high | mitigate | Redact through `crates/paladin-llm/src/redaction.rs` BEFORE truncating to the configured cap, per the security-instructions ordering rule; default is names-and-sizes only (... | closed |
| T-28-03-02 | 28-03 | Denial of service | Heartbeat flood evicting real events from the bounded queue | medium | mitigate | Per-node last-emitted rate limit at the configured interval, tested at the interval boundary under a paused clock (D-04). | closed |
| T-28-03-03 | 28-03 | Information disclosure | `ParleyRaised` carrying prompt text | medium | mitigate | The variant carries `parley_id`, `node_id` and `kind` only — no `prompt`, no author-supplied context. | closed |
| T-28-03-04 | 28-03 | Repudiation | `RunFinished` unable to distinguish success from failure | medium | mitigate | `status` derived from the `RunOutcome` arm, table-tested across all four (D-02, closing 27-CONTEXT D-25's correction). | closed |
| T-28-04-01 | 28-04 | Tampering | SQL injection through `thread_id` or `after_seq` | high | mitigate | Every statement uses bound parameters in named `const` SQL blocks; no query string is built by formatting a caller value, asserted by a zero-`format!` check on the adapters. | closed |
| T-28-04-02 | 28-04 | Information disclosure | Long-lived trace rows outliving their run's retention window | medium | mitigate | `prune_thread` joined to the existing Waypoint retention routine with the same bounds; no `run_traces`-specific tunable that could be set to never expire (D-17). | closed |
| T-28-04-03 | 28-04 | Tampering | Reading a row written by a newer, unknown schema | medium | mitigate | Per-row `schema_version` and a typed `UnsupportedSchemaVersion { found }` on read (X-04). | closed |
| T-28-04-04 | 28-04 | Denial of service | Unbounded `read` returning a whole run in one response | medium | mitigate | `limit` is a required parameter; the replay consumer (28-11) paginates by `after_seq`. | closed |
| T-28-04-05 | 28-04 | Information disclosure | `ThreadId` treated as an authorization boundary by a caller | low | accept | Documented in the module docs exactly as `waypoint_port.rs` documents it; authorization is the web layer's job. | closed |
| T-28-05-01 | 28-05 | Elevation of privilege | Scenario content as a code-execution vector | high | mitigate | The format carries only structured parameters; `custom(fn)` is Rust-API-only and never deserialized; `#[serde(deny_unknown_fields)]` closes the document shape; nothing in t... | closed |
| T-28-05-02 | 28-05 | Tampering | A scenario written for a newer format silently misparsing | medium | mitigate | `schema_version` checked before use with a typed `UnsupportedSchemaVersion { found }`; a golden JSON Schema documents the accepted shape (X-04). | closed |
| T-28-05-03 | 28-05 | Information disclosure | Captured request log carrying prompt text into failure output | low | accept | The log is test-harness-local and exists precisely to render a failure; scenarios are scripted with fixture prompts, not production data. Documented in `requests()`'s rustdoc. | closed |
| T-28-05-04 | 28-05 | Tampering | Supply-chain risk from the two new dependencies | medium | mitigate | `glob` 0.3.4 and `libtest-mimic` 0.8.2 both cleared the package-legitimacy audit with an `OK` verdict, are MIT/Apache-2.0 already in `deny.toml`'s allowlist, and clear MSRV... | closed |
| T-28-06-01 | 28-06 | Information disclosure | Log lines carrying state values or prompt text | high | mitigate | The record carries field names and byte sizes by default; `NodeProgress::StreamChunk` carries a byte count and never the text; `ParleyRaised` carries no prompt; the log sin... | closed |
| T-28-06-02 | 28-06 | Denial of service | Default-on logging flooding an operator's log pipeline | medium | mitigate | Bounded queue with counted drops upstream; the heartbeat rate limit; `trace.log_sink` and `RUST_LOG=paladin::trace=off` both silence it without a code change (D-11). | closed |
| T-28-06-03 | 28-06 | Tampering | A sink altering run behaviour | high | mitigate | `build_run_sink` produces a `CompositeSink` behind the never-awaited dispatcher; `worker_run_result_is_identical_with_and_without_sinks` asserts result equality; the prohib... | closed |
| T-28-06-04 | 28-06 | Repudiation | Producers below the engine stamping their own counters | medium | mitigate | Every below-engine producer takes `Arc<dyn TraceEmitter>`, never a raw sink; the worker pulls one handle per run; `fallback_hop_lands_in_the_run_sequence` proves interleavi... | closed |
| T-28-07-01 | 28-07 | Tampering | Node id or condition text breaking out of a Mermaid/DOT label | medium | mitigate | Ids are sanitized to `n{i}` for the diagram's own identifiers and the real id appears only inside a quoted, escaped label, following `FlowVisualizer`'s escaping; the inspec... | closed |
| T-28-07-02 | 28-07 | Information disclosure | Exported diagrams carrying prompt or state content | low | mitigate | `GraphShape` carries ids, kinds, flags and condition KINDS only — never prompts, templates or state values. | closed |
| T-28-07-03 | 28-07 | Repudiation | A silent rendering change slipping past review | low | mitigate | Ten committed goldens make every rendering change a visible diff; the bless path is explicit and named in the failure message. | closed |
| T-28-08-01 | 28-08 | Denial of service | A pathological regex in `final_state_field_matches` | medium | mitigate | Regexes are compiled through the `regex` crate, which has no backtracking and linear-time guarantees by construction; a compile failure is a typed assertion failure, not a... | closed |
| T-28-08-02 | 28-08 | Elevation of privilege | `custom(fn)` reachable from a scenario file | high | mitigate | The custom hook is a Rust-API-only type with no serde derive and no variant in the file-format enum; asserted by a zero-occurrence check in `scenario.rs`. | closed |
| T-28-08-03 | 28-08 | Information disclosure | Rendered failures echoing state values into CI logs | low | accept | Failures must show observed values to be actionable, and scenarios are scripted fixtures rather than production data; documented in `render_failure`'s rustdoc. | closed |
| T-28-08-04 | 28-08 | Repudiation | A silently-passing assertion whose evidence never existed | high | mitigate | Every evaluator that depends on a `RunFinished` record fails explicitly when the stream has none; `evaluate` dispatches with no wildcard arm; an absent snapshot file is a f... | closed |
| T-28-09-01 | 28-09 | Information disclosure | Credential header forwarded on a redirect to an attacker-influenced host | high | mitigate | `reqwest::Client::builder().redirect(Policy::none())` passed through `with_http_client`; `otlp_client_does_not_follow_redirects` proves the second host receives nothing. | closed |
| T-28-09-02 | 28-09 | Information disclosure | Header values in logs, `Debug` output or an exporter error | high | mitigate | `OtelConfig`'s manual redacting `Debug` (28-02); no log line or error body in this file interpolates a header value; the manual credential-handling review in `security.inst... | closed |
| T-28-09-03 | 28-09 | Spoofing / SSRF-adjacent | A configured endpoint pointing at an internal host | medium | mitigate | `TraceConfig::validate_typed` rejects non-`http(s)` schemes when the exporter is enabled; the endpoint is operator-supplied configuration, not caller-supplied input, so the... | closed |
| T-28-09-04 | 28-09 | Information disclosure | Span attributes carrying state values | medium | mitigate | Attributes are ids, counts, outcomes and flags; `DeltaMerged` becomes a span event carrying field NAMES; the exporter emits only what the record already holds. | closed |
| T-28-09-05 | 28-09 | Denial of service | Exporter blocking the run when the collector is slow or down | high | mitigate | The sink sits behind the never-awaited bounded dispatcher; an export failure is a diagnostics-only `Ok(())`; the composite isolates it from sibling sinks. | closed |
| T-28-09-06 | 28-09 | Tampering | Supply-chain risk from the three new dependencies | medium | mitigate | All three cleared the package-legitimacy audit with an `OK` verdict, are Apache-2.0 already in `deny.toml`'s allowlist, and clear MSRV 1.88 with headroom; `make security` a... | closed |
| T-28-10-01 | 28-10 | Information disclosure | Overlay carrying state values into a diagram | medium | mitigate | `Visit` carries superstep, attempt, outcome, duration, tokens and cache flag only — never a field value; `DeltaMerged` is not read by the overlay at all. | closed |
| T-28-10-02 | 28-10 | Repudiation | A derived fired-edge set presented as exact | medium | mitigate | `source` is carried on the overlay and rendered; the Waypoints-sourced case leaves `evaluated_edges` empty rather than guessing, and the inspector labels the missing half (... | closed |
| T-28-10-03 | 28-10 | Tampering | Node id breaking out of a diagram label | medium | mitigate | Labels reuse the static renderer's quoting and escaping, proven by the shared goldens. | closed |
| T-28-10-04 | 28-10 | Repudiation | An observed-only diagram mistaken for the whole graph | medium | mitigate | The locked title line states the limitation on the diagram itself, and `observed_only` is a field consumers can branch on (D-22). | closed |
| T-28-11-01 | 28-11 | Information disclosure | State field VALUES reaching the SSE wire | high | mitigate | `map_trace_event` emits field NAMES and byte counts for `state_delta` and never a `FieldChange.value`, regardless of the `trace.state_values` config — the Phase 27 prohibit... | closed |
| T-28-11-02 | 28-11 | Information disclosure | Replay serving another tenant's run | high | mitigate | Replay reads through `RunTracePort` scoped by the same `ThreadId` the existing stream endpoint already authorizes; no new authorization surface is introduced and the endpoi... | closed |
| T-28-11-03 | 28-11 | Denial of service | Replaying an unbounded run in one response | medium | mitigate | `read` is paginated by `after_seq` with a limit; the replay loop streams incrementally rather than materialising the whole run. | closed |
| T-28-11-04 | 28-11 | Tampering | A persisted-write failure silently changing run behaviour | high | mitigate | The persisting sink swallows errors into a logged `Ok(())` behind the never-awaited dispatcher; `persisting_sink_write_failure_does_not_fail_the_run` asserts outcome equality. | closed |
| T-28-11-05 | 28-11 | Repudiation | A replayed stream indistinguishable from a live one | low | mitigate | Every replayed event carries `mode: replay` and the original `at`, so a consumer can tell the difference. | closed |
| T-28-12-01 | 28-12 | Elevation of privilege | Live mode running unintentionally and spending credits | high | mitigate | Three independent gates — flag, environment variable and provider keys — all required; no workflow sets the variable; `default_test_run_never_enters_live_mode` asserts the... | closed |
| T-28-12-02 | 28-12 | Information disclosure | Fixture prompts sent to a third-party provider in live mode | medium | mitigate | Live mode is opt-in three times over and documented as a pre-release smoke path; ADR-0012 governs the key handling. | closed |
| T-28-12-03 | 28-12 | Tampering | `--bless` writing outside the scenario's directory | medium | mitigate | The snapshot path is derived from the resolved scenario file's own directory and case name, never from a caller-supplied path; path traversal generally is canon — covered b... | closed |
| T-28-12-04 | 28-12 | Elevation of privilege | A scenario naming an arbitrary constructor | medium | mitigate | `registered` targets resolve only against names the HOST registered in Rust; an unknown name is a clear error listing the registered set, never a dynamic load. | closed |
| T-28-12-05 | 28-12 | Repudiation | A flaky scenario passing on a lucky run | medium | mitigate | `--repeat N` treats any divergence across repeats as a failure and names the diverging `seq` range, rather than reporting a pass rate and exiting zero. | closed |
| T-28-13-01 | 28-13 | Elevation of privilege | The CLI gaining a network path to the API | medium | mitigate | Both commands reach storage through ports only, asserted by a zero-occurrence check on the HTTP client crate in each file; ADR-0023's isolation is preserved. | closed |
| T-28-13-02 | 28-13 | Information disclosure | Exported overlays carrying state content | medium | mitigate | The overlay carries ids, outcomes, counts and costs only; field values never enter `ExecutionOverlay`. | closed |
| T-28-13-03 | 28-13 | Tampering | `--out` writing to an unexpected location | low | accept | The path is supplied by the operator running the command on their own machine, with the same authority the shell already grants; path traversal generally is canon and cover... | closed |
| T-28-13-04 | 28-13 | Repudiation | A derived overlay presented as exact | medium | mitigate | The command prints the resolved source and whether the shape is observed-only, and the diagram itself carries the locked observed-only title. | closed |
| T-28-14-01 | 28-14 | Information disclosure | State field VALUES reaching the view | high | mitigate | `field_changes` is `Vec<FieldName>` by type, and `serialized_view_contains_no_field_values` proves it for a whole serialized view; there is no configuration that changes this. | closed |
| T-28-14-02 | 28-14 | Elevation of privilege | `paladin-web` gaining an engine dependency to render the diagram | high | mitigate | The `mermaid` string is rendered facade-side and handed over as text; the port's types are core-only, asserted by a zero-occurrence check on the battalion crate in the port... | closed |
| T-28-14-03 | 28-14 | Information disclosure | A thread id acting as a capability | medium | mitigate | The port documents, as `waypoint_port.rs` does, that `ThreadId` is not an authorization boundary; the route's admin auth layer is 28-15's control. | closed |
| T-28-14-04 | 28-14 | Repudiation | A derived overlay presented as exact through the view | medium | mitigate | `source` and `observed_only` are carried on the view and the page labels the Waypoints-only case explicitly. | closed |
| T-28-15-01 | 28-15 | Information disclosure | Run structure and field names reachable without admin | high | mitigate | The route is mounted under the same authentication and admin-scope layers the existing admin routes use, and is compiled out of any build without the feature; both are asse... | closed |
| T-28-15-02 | 28-15 | Tampering | Script-element breakout through the embedded payload | high | mitigate | The serialized view has the script-terminating and comment-opening sequences escaped before substitution, asserted by a test using a view containing both. | closed |
| T-28-15-03 | 28-15 | Information disclosure | Field VALUES rendered on the page | high | mitigate | The view carries names only by type (28-14), the page has no values-shown mode, and the prohibition above states the rule. | closed |
| T-28-15-04 | 28-15 | Tampering | Third-party JavaScript from the configured CDN | medium | accept | Mermaid is loaded at runtime from an operator-configured URL in an admin-authenticated operator's own browser, against a page whose only embedded data is that operator's ow... | closed |
| T-28-15-05 | 28-15 | Information disclosure | The dev tool leaking into the published API surface | medium | mitigate | No OpenAPI path attribute and the controller is not one of the drift-guard's three router sources; the committed document is asserted byte-identical with the feature enabled. | closed |
| T-28-15-06 | 28-15 | Denial of service | A very large view producing an unusable page | low | mitigate | Bounded scroll heights on all four panels with no truncation; the whole payload stays in the DOM because the page makes no fetches. | closed |
| T-28-16-01 | 28-16 | Elevation of privilege | A committed scenario reaching a live provider during a default test run | high | mitigate | The scenarios name registered targets with fully scripted LLM behaviour and no live opt-in; the runner's three-gate live mode (28-12) cannot activate without a flag and an... | closed |
| T-28-16-02 | 28-16 | Tampering | The extraction silently changing an acceptance test's behaviour | high | mitigate | The move is verbatim and parameterised only on the ports; all three integration tests are run before and after with unchanged assertions, and a behavioural difference is a... | closed |
| T-28-16-03 | 28-16 | Repudiation | A flaky acceptance scenario passing on a lucky run | medium | mitigate | The twenty-repeat check is part of this plan's acceptance and its output is recorded in the summary. | closed |
| T-28-16-04 | 28-16 | Information disclosure | Committed snapshot files carrying machine-specific or sensitive content | low | mitigate | Snapshots are final Battlefield states from scripted fixtures; the blessed file is reviewed before commit. | closed |
| T-28-17-01 | 28-17 | Information disclosure | A documented OTLP example showing a real-looking credential header | high | mitigate | The collector example uses an obvious placeholder and the page states that header values are secrets, are redacted from debug output and must not be committed. | closed |
| T-28-17-02 | 28-17 | Information disclosure | Documenting the trace value-capture flag without its consequence | high | mitigate | The operations page states that enabling it puts redacted, capped field values into logs and stored rows, that it never reaches the stream wire, and that the default is off. | closed |
| T-28-17-03 | 28-17 | Tampering | Publishing the new crate in the wrong dependency order | medium | mitigate | Insertion follows the script's stated invariant that every dependency is on the registry before its dependent publishes; the dry-run target exercises it. | closed |
| T-28-17-04 | 28-17 | Repudiation | Claiming a Docker-dependent gate passed without evidence | high | mitigate | The CI-evidence file records workflow, job, run identifier, commit and outcome for every non-local gate, and the acceptance criteria require real run identifiers rather tha... | closed |
| T-28-17-05 | 28-17 | Tampering | An unregistered public-API change slipping past review | medium | mitigate | The inventory is taken before any row is written, the baseline is regenerated, and the api-surface check is required to pass on the final commit. | closed |

*Status: closed · open — all 75 rows are `closed` as of the 2026-09-09 audit*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` (`high`) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Verified Mitigation Highlights

All 27 `high` threats, with the evidence the auditor cited. File paths are relative to the
workspace root; line numbers are as of the audited tree.

| Threat | Evidence |
|---|---|
| T-28-01-01 / T-28-03-01 — state values in `FieldChange` | `crates/paladin-core/src/platform/container/trace.rs:75-93` `FieldChange { value: Option<String> }`; `crates/paladin-battalion/src/engine/hooks.rs:406-412` `redacted_value()` calls the redactor **then** `bounded_excerpt` — redact-before-truncate proven in code |
| T-28-01-02 — sink blocking / panicking on the run path | `hooks.rs:286-330` synchronous non-`.await` `emit()`, drop-oldest `buf.pop_front()`; `catch_unwind` around `sink.on_event` (`hooks.rs:239`); tests `slow_sink_does_not_slow_the_run` (`:991`), `panicking_sink_does_not_kill_the_consumer` (`:957`) |
| T-28-02-01 / T-28-09-02 — OTLP header values in `Debug` / logs / exporter errors | `src/config/trace.rs:167-180` manual `Debug` redacting header values, test `otel_debug_redacts_header_values`; `src/infrastructure/telemetry/otel_sink.rs:141-145` headers flow only into `.with_headers(...)`, never logged |
| T-28-04-01 — SQL injection via `thread_id` / `after_seq` | Zero `format!` occurrences in `crates/paladin-storage/src/run_trace/{postgres,sqlite}.rs`; every query is a named `const` SQL string with `.bind()` |
| T-28-05-01 / T-28-08-02 — scenario content as a code-execution vector | `crates/paladin-eval/src/scenario.rs:587-591` closed `Assertion` enum with `deny_unknown_fields` and no `Custom` variant; `CustomAssertion` has no `serde` derive (`assertion.rs:16-28`) |
| T-28-06-01 — log lines carrying values or prompts | `map_trace_event` (`src/application/services/run/events.rs`) and `LogTraceSink` carry only names / byte counts; `ParleyRaised { parley_id, node_id, parley_kind }` has no prompt field (`trace.rs:272-278`) |
| T-28-06-03 — a sink altering run behaviour | `worker_run_result_is_identical_with_and_without_sinks` (`src/application/services/run/worker_tests.rs:1133`) |
| T-28-08-04 — silently-passing assertion | `assertion.rs:263-282` exhaustive `evaluate` match; tests `run_status_fails_when_run_never_finished`, `final_state_snapshot_fails_when_absent_naming_bless` |
| T-28-09-01 — credential header forwarded on redirect | `otel_sink.rs:447` `reqwest::redirect::Policy::none()`; test `otlp_client_does_not_follow_redirects` (`tests/integration/otel_transport_test.rs:292`) — the house pattern from `security.instructions.md` |
| T-28-09-05 — exporter blocking the run | `otel_sink.rs:604` `on_event` returns `Ok(())`; never awaited on the run path via the dispatcher isolation of T-28-01-02 |
| T-28-11-01 / T-28-14-01 / T-28-15-03 — state VALUES on the SSE wire / in the view / on the page | `events.rs:170-186` `DeltaMerged` arm emits `fields` / `bytes` only (zero `.value` occurrences); `src/application/services/run/inspector.rs:367-391` `field_changes: Vec<FieldName>`, test `serialized_view_contains_no_field_values` |
| T-28-11-02 — replay serving another tenant's run | `RunTracePort::read(&self, thread: &ThreadId, ...)` scoped by thread; authorization is the web layer's (see *production-wiring caveat*) |
| T-28-11-04 — persisted-write failure changing run behaviour | `persisting_sink.rs` test `persisting_sink_write_failure_does_not_fail_the_run` |
| T-28-12-01 / T-28-16-01 — live mode running unintentionally | `crates/paladin-eval/src/runner.rs:1050-1101` three-gate `check_live_mode`; test `default_test_run_never_enters_live_mode`; `evals/e2e-*.eval.yaml` use `registered:` targets, no `live` keyword; ADR `0012-live-api-test-key-behaviour.md` |
| T-28-14-02 — `paladin-web` gaining an engine dependency | `crates/paladin-ports/Cargo.toml` carries no `paladin-battalion` dependency; the view is a port-level type |
| T-28-15-01 — run structure reachable without admin | `crates/paladin-web/src/app.rs:86-92` `#[cfg(feature = "dev-ui")]` behind `require_auth` + `require_admin` layers; test `dev_ui_authenticated_non_admin_request_is_rejected` |
| T-28-15-02 — script-element breakout | `crates/paladin-web/src/dev_ui_controller.rs:103,149-160` `escape_for_script` over **both** the inspector JSON and, after the WR-02 fix (`8325accc`), the Mermaid-URL JSON; test `dev_ui_page_escapes_the_mermaid_url_payload` run during the audit — 1 passed (not a 0-select) |
| T-28-16-02 — extraction changing an acceptance test's behaviour | `28-16-SUMMARY.md`: verbatim extraction with identical test counts (32/35/37) and 20/20 `--repeat` runs |
| T-28-17-01 / T-28-17-02 — documentation leaking a credential shape or hiding the value-capture consequence | `docs/src/operations/observability.md:103,112` placeholder `Bearer <redacted>` with a redaction statement; `:49,167-169` states redact-then-cap, never-on-wire, default-off |
| T-28-17-04 — claiming a Docker-dependent gate passed without evidence | `28-CI-EVIDENCE.md` records real run ids and commits per gate |

**Code-review cross-check (`28-REVIEW.md` / `28-REVIEW-FIX.md`):** CR-01 (`RunStarted.run_id`
always `None`, breaking OTel correlation) fixed `6c0c8b69` — test `run_started_carries_the_bound_run_id`
passes; WR-01 fixed `4b3a223a`; WR-02 (the T-28-15-02 gap) fixed `8325accc` and re-verified above;
WR-03 doc-only, WR-04 skipped by design — neither maps to a registered threat.

---

## Accepted Risks Log

Six threats carry an `accept` disposition. All are below the `high` blocking threshold.

| Risk ID | Threat Ref | Severity | Rationale | Accepted By | Date |
|---------|------------|----------|-----------|-------------|------|
| AR-28-01 | T-28-02-04 | medium | `state_values` accidentally on in a shipped example config — default is `false` in `Default`, `config.example.yml:247-250` and `config.test.yml:147`, each with a comment stating the consequence. Residual risk is an operator's deliberate change. | Phase 28 plan 28-02 | 2026-09-09 |
| AR-28-02 | T-28-15-04 | medium | Third-party Mermaid JavaScript loaded from an operator-configured CDN in an admin-authenticated operator's own browser, against a page whose only embedded data is that operator's own view. Vendoring a multi-megabyte asset into a published crate was rejected (D-26); air-gapped operators point the URL at a local mirror. Documented at `dev_ui_controller.rs:52-78`. | Phase 28 plan 28-15 | 2026-09-09 |
| AR-28-03 | T-28-04-05 | low | `ThreadId` treated as an authorization boundary by a caller — `run_trace_port.rs:31-36` documents, exactly as `waypoint_port.rs` does, that it is not one; authorization is the web layer's job. | Phase 28 plan 28-04 | 2026-09-09 |
| AR-28-04 | T-28-05-03 | low | Captured request log carrying prompt text into failure output — test-harness-local, scripted fixture prompts, never production data. The plan cited `requests()`'s rustdoc as the documentation; that rustdoc was missing at audit time and was added in this commit (`scripted_llm.rs`, `# Threat accepted here` section). | Phase 28 plan 28-05 | 2026-09-09 |
| AR-28-05 | T-28-08-03 | low | Rendered failures echoing observed state into CI logs — a failure must show observed values to be actionable; scenarios are scripted fixtures. Documented at `assertion.rs:16-28`, naming the threat id. | Phase 28 plan 28-08 | 2026-09-09 |
| AR-28-06 | T-28-13-03 | low | `--out` writing to an unexpected location — the path is supplied by the operator on their own machine with the authority the shell already grants. Confirmed in `28-13-SUMMARY.md`'s Threat Flags. | Phase 28 plan 28-13 | 2026-09-09 |

*Accepted risks do not resurface in future audit runs.*

---

## Production-Wiring Caveat (recorded, not a finding)

Two surfaces this register describes as living behind an authenticated endpoint's own auth
layers are **not reachable in the shipped `paladin-server` binary today**, discovered while
tracing T-28-11-02 and T-28-15-01:

- `RunEventStreamService::with_replay` and `RunWorkerPool::with_run_trace_port` are never
  called at the composition root (`src/infrastructure/web/run_api_wiring.rs`) — zero occurrences
  outside tests. `28-11-SUMMARY.md` records this under *Next Phase Readiness* as a deferred
  wiring task. Replay mode (T-28-11-02/03/05) and trace persistence (T-28-11-04) are therefore
  tested but unexercised by any live request.
- `create_dev_ui_router` is never merged into the server's router (only `agent_router`,
  `thread_router` and `run_router` are) — the `dev-ui` feature-gated route (T-28-15-01..06) is
  unreachable in the binary regardless of the cargo feature.

Neither weakens a mitigation (every cited control and test is present), and both reduce rather
than expand live attack surface. They are recorded so that the wiring change which eventually
exposes these surfaces is not assumed to inherit a security review it never received: **re-run
`/gsd-secure-phase` on the phase that wires them.**

---

## Threat Flags from Summaries

`28-13-SUMMARY.md` is the only summary with a `## Threat Flags` section; it maps every
finding to T-28-13-01..04 and reports no new attack surface. No other executor flagged new
surface through the designated channel this phase.

---

## Security Audit Trail

## Security Audit 2026-09-09

| Metric | Count |
|--------|-------|
| Threats found | 75 |
| Closed | 75 |
| Open | 0 |
| Open at or above `high` | 0 |

The auditor's raw verdict was `SECURED` with 74 closed and T-28-05-03 (`low`, `accept`) open
because its plan-cited rustdoc rationale did not exist. The disposition was already documented
in the plan's register, so the gap was documentation, not control; the orchestrator added the
rustdoc in this commit and closed the row.

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-09 | 75 | 74 | 1 (low, non-blocking) | gsd-security-auditor (ASVS L1, block_on high) |
| 2026-09-09 | 75 | 75 | 0 | /gsd-secure-phase orchestrator — T-28-05-03 rustdoc added |

Severity breakdown: 27 high (27 closed) · 39 medium (39 closed) · 9 low (9 closed).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** granted 2026-09-09 — no blocking threats remain.
