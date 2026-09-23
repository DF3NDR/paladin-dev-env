---
phase: 36
slug: rustdoc-zero-warning-bar-examples-currency
status: verified
threats_open: 0
asvs_level: 1
created: 2026-09-18
---

# Phase 36 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|----------------|
| developer shell → example process | `cargo run --example token_economy_commissary` executes in the developer's environment and may read process environment variables | process env vars (provider keys excluded by design) |
| repository → published rustdoc site | doc comments edited here are rendered publicly by `cargo doc`; prose that over-describes an internal item leaks design detail | doc comments / example source (no secrets) |
| repository → published rustdoc site | doc comments edited here render publicly; prose that over-describes an internal item leaks design detail | doc comments / example source (no secrets) |
| doc-comment edit → public API baseline | a link "fixed" by widening visibility silently moves the published surface | doc comments / example source (no secrets) |
| repository → published rustdoc site | the core crate's doc comments are the innermost crate's public contract and render publicly | doc comments / example source (no secrets) |
| doc-comment edit → hexagonal dependency direction | adding a dependency to make a link resolve would invert the architecture's inward-only rule | doc comments / example source (no secrets) |
| repository → published rustdoc site | the redaction module and the dev-ui escaping helper are security-relevant; their doc comments render publicly | doc comments / example source (no secrets) |
| feature gate → documented surface | a link that resolves under one feature set and not the other re-opens the bar the moment CI runs the other invocation | doc comments / example source (no secrets) |
| repository → published rustdoc site | the facade's doc comments are what a library consumer reads first and render publicly | doc comments / example source (no secrets) |
| telemetry sink → collector endpoint | the sink's private client builder carries credentials outward; its documentation must not over-describe that path | trace/telemetry state values |
| developer shell → example process | the example executes in the developer's environment and may read process environment variables | process env vars (provider keys excluded by design) |
| example process → engine configuration | both programs mutate engine environment variables in-process to demonstrate overrides | process env vars (provider keys excluded by design) |
| human responses → engine resume | the typed resume accepts caller-supplied responses for the awaited gate, which is the trust boundary the total-validation rule guards | doc comments / example source (no secrets) |
| example process → engine configuration | the program mutates the shutdown grace-period and graceful-shutdown variables in-process | doc comments / example source (no secrets) |
| tool output → model context | the fail-run tool error mode demonstration feeds a failing tool's text back through the sanitizer | doc comments / example source (no secrets) |
| agent A memory ↔ agent B memory | the confined vault demonstration deliberately attempts a cross-namespace read | agent memory namespace |
| model response → typed value | structured output parses an untrusted model response against a schema | doc comments / example source (no secrets) |
| HTTP client → in-process API | the example drives real HTTP routes with auth, including an admin-gated dev-ui route | doc comments / example source (no secrets) |
| webhook sender → webhook receiver | an untrusted request body arrives carrying a signature header that must be verified before the body is trusted | webhook body + HMAC signature header |
| example process → local network | the SSRF-guard discussion concerns outbound destinations chosen by a caller | outbound HTTP destination (SSRF-relevant) |
| repository → book page | editing the doc-examples module re-renders a deployment-topologies page | doc comments / example source (no secrets) |
| example process → external service | the cache program reaches a Redis server and the export program reaches a collector; neither is present in CI | trace/telemetry state values |
| trace sink → external collector | trace state values may carry user content; exporting them is an information-flow decision | doc comments / example source (no secrets) |
| eval runner → provider | live mode makes scenarios call a real provider with a real key | doc comments / example source (no secrets) |
| repository → reader following the README | a run command or a prerequisite the page states wrongly sends a reader down a path that fails or, worse, one that silently skips a target | doc comments / example source (no secrets) |
| README prose → shipped API | a snippet naming a field that does not exist teaches an API the tree does not have | doc comments / example source (no secrets) |
| developer push → required CI check | this plan changes what a required check enforces, so every future pull request is affected | gate-command definitions (no secrets) |
| local gate → CI gate | if `make doc-check` and the CI steps drift apart, a developer's green local run stops predicting CI | gate-command definitions (no secrets) |
| local examples script → CI examples job | if the two invocation lists drift, a gated example regresses locally unnoticed | gate-command definitions (no secrets) |
| local measurement → CI measurement | the devcontainer toolchain and the CI toolchain may differ; the gate lives in CI | gate-command definitions (no secrets) |
| phase record → downstream phases | Phase 36.1 SC4 and Phase 37 verify this phase by ID against the closure map | doc comments / example source (no secrets) |
| repository → published changelog | changelog bullets are read by users who have no access to the internal work list | doc comments / example source (no secrets) |
| ledger file → tool-managed state | `WINDOWS.md` carries recomputed counts that a hand edit desynchronises | doc comments / example source (no secrets) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-36-01 | Information Disclosure | `examples/token_economy_commissary.rs` | medium | mitigate | The program is offline by construction (mock LLM adapter, no provider client). It reads no API key, prints no environment value, and its header comment states that no key and no external service is needed (D-29). Acceptance criterion greps the file for credential-shaped output. | closed |
| T-36-02 | Information Disclosure | de-linked private mentions in `paladin-ports` / `paladin-storage` doc comments | low | mitigate | D-05 requires rewording toward the public entry point rather than describing internal mechanics, so a de-linked mention never carries more detail than the private item's own doc comment already did. | closed |
| T-36-03 | Tampering | public API surface | high | mitigate | `make api-surface` runs before every commit touching `src/` or `crates/` and must report no change (D-28); widening `pub(crate)` to `pub` to satisfy a link is forbidden (D-05). | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | This phase installs no new external package — `36-RESEARCH.md`'s Package Legitimacy Audit records "not applicable", and every crate the new examples use (`schemars`, `axum`, `reqwest`, `paladin-eval`, `paladin-storage` features) is already a pinned workspace dependency. No `cargo add` runs. | closed |
| T-36-04 | Tampering | `paladin-battalion` public API surface | high | mitigate | `make api-surface` runs before the single crate commit and must report no change (D-28); widening `pub(crate)` to `pub` is forbidden by D-05, and the acceptance criteria grep that the specific private helpers — `push_field`, `validate_schedulable`, `validate_aegis_undeclared_nodes`, `validate_parley_value_for_kind` and `analyze_and_select` — never gain a public declaration. | closed |
| T-36-05 | Information Disclosure | de-linked private mentions in the graph, commander and cache-key files | low | mitigate | D-05 requires rewording toward the public entry point rather than describing internal mechanics — a de-linked mention of the graph fingerprint helper or the cache-key prefix constants must not describe the internal algorithm in more detail than the private item's own doc comment already did. | closed |
| T-36-06 | Repudiation | closure bookkeeping | medium | mitigate | The commit subject cites the RD range and the SUMMARY carries the D-24 closure table with cited and actual file:line, so every one of the 72 IDs is traceable to a commit (D-09, D-24, D-26). | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; `36-RESEARCH.md`'s Package Legitimacy Audit records "not applicable — this phase installs no new external package". | closed |
| T-36-07 | Tampering | `paladin-ai-core` dependency graph | high | mitigate | The directive container names an item owned by an outer crate; the fix de-links rather than adding a dependency. Acceptance criterion asserts the commit adds no line to that crate's manifest. | closed |
| T-36-08 | Tampering | public API surface | high | mitigate | `make api-surface` runs before the commit and must report no change (D-28); D-05 forbids widening visibility, and the JSON-extraction helper specifically must not gain a public declaration. | closed |
| T-36-09 | Information Disclosure | the webhook container's doc block | medium | mitigate | That container's own header records the "no signing key on the row" prohibition and the signature invariant; this plan changes link syntax only and must not reword, weaken or restate those security statements. The acceptance criterion restricts the diff to doc-comment lines. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; `36-RESEARCH.md`'s Package Legitimacy Audit records "not applicable — this phase installs no new external package". | closed |
| T-36-10 | Information Disclosure | the paladin-llm redaction module's doc block | medium | mitigate | The module states the redact-before-truncate invariant that keeps a secret's tail from surviving truncation. This plan changes link syntax only; the acceptance criterion restricts the diff to doc-comment lines and the action forbids rewording that statement. | closed |
| T-36-11 | Information Disclosure | the dev-ui controller's script-escaping de-link | medium | mitigate | D-05 requires rewording toward the public route contract; the de-linked mention must not describe the script-escaping algorithm in more detail than the private helper's own doc comment already did. | closed |
| T-36-12 | Tampering | public API surface of paladin-llm and paladin-web | high | mitigate | `make api-surface` runs before each commit and must report no change (D-28). Acceptance criteria grep that the five named private items never gain a public declaration. | closed |
| T-36-13 | Elevation of Privilege | feature-gated documentation | low | mitigate | Twelve of these rows exist only under all features; the per-crate all-features sweep is run for both crates after the fix, so a fix that holds under one feature set and not the other cannot pass (D-07, D-10). | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; `36-RESEARCH.md`'s Package Legitimacy Audit records "not applicable — this phase installs no new external package". | closed |
| T-36-14 | Tampering | `paladin-ai` public API surface | high | mitigate | `make api-surface` runs before the commit and must report no change (D-28). Acceptance criteria grep that the seven named private items never gain a public declaration. | closed |
| T-36-15 | Information Disclosure | the telemetry sink's de-linked client-builder mention | medium | mitigate | The private helper builds an HTTP client that may carry a credential header to a collector endpoint; D-05's "reword toward the public entry point" keeps the prose on the public sink contract rather than the client's header or redirect handling. | closed |
| T-36-16 | Spoofing | the agent-runtime provider-precedence prose | medium | mitigate | De-linking the known-provider-names constant must not leave a precedence claim that disagrees with the shipped constant — the constant is in the task's read list precisely so the reworded sentence stays true (D-00c: the shipped tree outranks the document). | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; `36-RESEARCH.md`'s Package Legitimacy Audit records "not applicable — this phase installs no new external package". | closed |
| T-36-17 | Information Disclosure | both new example programs | medium | mitigate | Neither program reads or prints a provider key; both use the mock LLM adapter and in-memory ports, and their header comments state that no key and no service is needed (D-16, D-29). Acceptance criteria grep each file for the three provider key variable names. | closed |
| T-36-18 | Tampering | process environment | low | mitigate | The superstep-cap and muster-cap demonstrations set the variable in-process and restore it afterwards, so a reader who copies the pattern does not leave a stale override behind. | closed |
| T-36-19 | Elevation of Privilege | the custom-edge-condition fail-closed demonstration | medium | mitigate | EX-67's whole point is that an unregistered custom condition fails closed rather than open. The program must show the unregistered run's real outcome, not a swallowed error — an example that hid it would teach the opposite security posture. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; every crate these programs use is already a pinned workspace dependency, per `36-RESEARCH.md`'s Package Legitimacy Audit. | closed |
| T-36-20 | Spoofing | the engine's typed resume total validation | high | mitigate | EX-72's capability is precisely that an incomplete or mismatched resume is rejected with a typed error rather than silently accepted. The program must print the rejection path, so an example reader learns to rely on it rather than pre-validating by hand. | closed |
| T-36-21 | Information Disclosure | both new example programs | medium | mitigate | Neither program reads or prints a provider key; both use the mock LLM adapter and in-memory ports and say so in their header comments (D-16, D-29). Acceptance criteria grep each file for the three provider key variable names. | closed |
| T-36-22 | Denial of Service | the graceful-shutdown example | medium | mitigate | An example that installed a real termination-signal handler and waited would hang the run and the CI example build's own verification. The program triggers the coordinator directly and the acceptance criteria assert both a bounded-timeout clean exit and the absence of signal-handler imports. | closed |
| T-36-23 | Tampering | process environment | low | mitigate | Both environment demonstrations set the variable in-process and restore it afterwards. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; every crate these programs use is already a pinned workspace dependency, per `36-RESEARCH.md`'s Package Legitimacy Audit. | closed |
| T-36-24 | Information Disclosure | the confined-vault namespacing demonstration | high | mitigate | EX-87's capability is that one agent cannot read another's memories. The program must attempt the cross-namespace read and print its empty or denied result — an example that only wrote and read within one namespace would not demonstrate the boundary at all. | closed |
| T-36-25 | Information Disclosure | the fail-run tool error mode demonstration | medium | mitigate | The shipped policy redacts tool text before bounding it, which is the same redact-then-truncate invariant the security instructions name. The program prints the sanitized text, never a raw error body, and the sanitization order is stated in the printed output. | closed |
| T-36-26 | Tampering | structured output schema validation | high | mitigate | EX-88's capability includes rejecting a response that violates the schema. The program runs the violating case and prints the typed failure, so a reader learns the machinery validates rather than trusts. | closed |
| T-36-27 | Information Disclosure | all three example programs | medium | mitigate | None reads or prints a provider key; all use the mock LLM adapter and in-memory ports and say so in their header comments (D-16, D-29). Acceptance criteria grep each file for the three provider key variable names. | closed |
| T-36-28 | Repudiation | EX-119 post-audit grep drift | medium | mitigate | Phase 35 added a doc-examples module after the audit SHA, making one recorded zero-hit grep stale. D-02 requires the drift to be recorded with the command output that proves it; Task 3 captures all five hit counts into the evidence file and the SUMMARY's closure table. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; schemars and every other crate these programs use is already a pinned workspace dependency, per `36-RESEARCH.md`'s Package Legitimacy Audit. | closed |
| T-36-29 | Spoofing | the webhook receiver's signature verification | critical | mitigate | Verification recomputes the keyed hash over the exact raw body bytes captured before deserialisation, using the same scheme as the shipped delivery service, compared in constant time. An example that re-serialised the body or compared with a plain equality operator would teach a forgeable and timing-leaky check. | closed |
| T-36-30 | Information Disclosure | the webhook shared secret | critical | mitigate | The secret is generated in-process, never printed, never logged, never placed in a header comment or a fenced command. An acceptance criterion greps the file for any print of a secret-shaped identifier (D-29). | closed |
| T-36-31 | Information Disclosure | the SSRF-guard documentation in the example | high | mitigate | The example states both what the private-address override relaxes and what it does not — the cloud metadata address is rejected regardless — and states the documented DNS-rebinding limitation, so it does not read as a stronger guarantee than the tree provides. | closed |
| T-36-32 | Elevation of Privilege | the admin-gated dev-ui route demonstration | high | mitigate | The dev-ui call uses an explicitly constructed admin principal and the printed output states that the route is both admin-gated and feature-gated, so a reader does not copy an unauthenticated call. The auth key is held in memory and never printed. | closed |
| T-36-33 | Tampering | the HTTP-service-host parity claim | medium | mitigate | The current files claim server parity while omitting two routers; a reader copying them deploys an app missing the thread and run routes. The fix mounts all three routers in the shipped order and drives at least one route from each so the claim is demonstrated, not merely compiled. | closed |
| T-36-34 | Repudiation | the book page re-render | medium | mitigate | The doc-examples module is included by the HTTP-service-host deployment page; D-19 requires the doc-examples check and the book build to be re-run and recorded green after the edit. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; axum and the HTTP client crate are already pinned workspace dependencies, per `36-RESEARCH.md`'s Package Legitimacy Audit. The only manifest edit is two example target declarations. | closed |
| T-36-35 | Denial of Service | the CI examples job | high | mitigate | An example that needs Redis or a collector and is *run* in CI would flake on every build. Both are build-verified only, matching the existing pattern where CI builds every example and runs none (D-16). The evidence file records each as built-not-run with the reason. | closed |
| T-36-36 | Information Disclosure | the tracing example's trace state values | medium | mitigate | The trace config's state-values field controls whether node state is captured into trace records; the program prints the field's value and what enabling it captures, so a reader configuring an exporter knows what would leave the process. | closed |
| T-36-37 | Information Disclosure | the eval demo's live mode | medium | mitigate | Live mode makes scenarios call a real provider with a real key. The program names the variable and its effect but never enables it, never reads a provider key, and states in its header that it runs in the default non-live mode (D-29). | closed |
| T-36-38 | Information Disclosure | both gated programs' prerequisites | low | mitigate | Neither program embeds a connection string or a credential; each states the service it needs and how to start it locally, so a reader does not copy a hard-coded endpoint. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; the eval crate is already an unconditional dev-dependency and the redis-cache and otel features resolve to already-pinned workspace dependencies, per `36-RESEARCH.md`'s Package Legitimacy Audit. The only manifest edit is two example target declarations. | closed |
| T-36-39 | Spoofing | gated programs' run commands | medium | mitigate | A run command missing its `--features` list produces a target cargo silently skips with exit code 0 — the same silent-skip failure mode the CI job's four-invocation split exists to catch. Each gated section's command is checked against that target's `[[example]]` `required-features` list in `Cargo.toml`. | closed |
| T-36-40 | Information Disclosure | example prerequisites stated in the README | low | mitigate | Sections for programs needing Redis or an OTLP collector name the service rather than embedding an endpoint or a credential, so a reader does not copy a connection string out of documentation. | closed |
| T-36-41 | Tampering | corrected snippet blocks | medium | mitigate | The three drifted lines are corrected against `crates/paladin-core/src/platform/container/execution_result.rs`, the shipped struct, not against another document — D-00c's shipped-tree-outranks-document rule. New sections carry no snippet block at all, so no fourth copy of an API shape is created (D-21). | closed |
| T-36-42 | Repudiation | gallery completeness claim | medium | mitigate | The both-directions `comm` cross-check is run and its two empty differences recorded in the evidence file, so "the gallery is complete" is a reproduced measurement rather than an assertion. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | This plan touches one markdown file and one evidence file; no package-manager install runs and no `.rs` file is staged. | closed |
| T-36-43 | Tampering | the ratified zero bar | critical | mitigate | The bar must not be narrowed, scoped down or given a carve-out to make the gate pass. `make doc-check` reproduces the `ci.yml` lint-job expression byte-identically, copied from the workflow rather than paraphrased, and any residual diagnostic is minted as a new row and reported as a deviation rather than suppressed (D-00a, D-02). | closed |
| T-36-44 | Repudiation | the closing commit | high | mitigate | D-13 requires the gate edits and the closing measurement in one commit, so `git bisect` can never land on a commit where the gate exists but fails, and the evidence file in that same commit is the proof it was green at install time. | closed |
| T-36-45 | Spoofing | gated example coverage in CI | high | mitigate | `cargo build --examples` exits 0 while silently skipping every target whose required-features are unmet. Each gated target gets an explicit CI step, and the acceptance criterion asserts the CI step count equals the script's invocation count so the two lists cannot drift apart. | closed |
| T-36-46 | Denial of Service | the pre-push hook | medium | mitigate | `make check-examples` is deliberately kept out of the pre-push hook — several full example builds per push would make the hook unusable and push developers toward bypassing it entirely (D-14). Only `doc-check` joins pre-push, filtered to Rust sources and the manifest. | closed |
| T-36-47 | Repudiation | local-versus-CI disagreement | medium | mitigate | D-03 fixes the precedence in advance: the CI figure wins and the difference is recorded in `36-CI-EVIDENCE.md`, never explained away. Plan 36-13 captures the real CI run that D-12 requires. | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | No package-manager install runs in this plan; the edits are a shell script, a Makefile, a hook config and a workflow file. | closed |
| T-36-48 | Repudiation | the 207-row closure map | high | mitigate | A row that is neither closed nor dispositioned silently breaks the verification chain two phases downstream. The verify loop asserts every `RD-` identifier from 01 to 143 appears, and the action forbids omitting or inventing a closure — an unmatched ID is recorded as outstanding and reported as a deviation (D-02, D-24). | closed |
| T-36-49 | Spoofing | the local zero measurement | high | mitigate | A local green does not prove a required check passes. D-12 requires a real pushed-branch run, and D-03 makes the CI figure authoritative on any disagreement; the checkpoint blocks phase verification until that run is recorded. | closed |
| T-36-50 | Tampering | `.planning/WINDOWS.md` | medium | mitigate | The ledger recomputes its own counts, so a hand edit leaves the file internally inconsistent. Rows move status only through the `gsd-tools` windows command, and the acceptance criteria assert the diff touches only the two rows' status and timestamp fields plus the counts block (D-27). | closed |
| T-36-51 | Information Disclosure | changelog bullets | low | mitigate | Internal work-list identifiers mean nothing to a reader and imply access to planning artifacts that are not published. The acceptance criterion greps the whole changelog for any such identifier and requires zero (D-00j). | closed |
| T-36-SC | Tampering | cargo package installs | low | accept | This plan touches planning artifacts and one changelog file; no package-manager install runs and no source file is staged. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-36-SC | T-36-SC (×13, one per plan) | No package-manager install runs in any of the 13 plans; every crate used by the new examples/adapters was already a pinned workspace dependency per `36-RESEARCH.md`'s Package Legitimacy Audit. Supply-chain risk from a *new* dependency is therefore not applicable to this phase. | maintainer (via plan-time threat model, accepted disposition) | 2026-09-18 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-18 | 64 | 64 | 0 | /gsd-verify-work orchestrator (L1 grep-depth, ASVS level 1, register_authored_at_plan_time: true for all 13 plans — short-circuit per secure-phase.md Step 3) |

**Verification method:** all 13 plans (36-01..36-13) authored a `<threat_model>` block at plan time. Each threat's disposition was `mitigate` (51 threats) or `accept` (13 `T-36-SC` supply-chain entries, one per plan). For `mitigate` threats, the stated mitigation was spot-checked against the actual implementation and/or the coverage evidence already confirmed during UAT (`36-UAT.md`, 59/59 deliverables auto-passed against automated commands):

- **API-surface / no-pub-widening threats** (T-36-03, T-36-04, T-36-08, T-36-12, T-36-14): confirmed via the `make api-surface` / `check-api-surface.sh` unchanged-count checks already recorded in every plan's SUMMARY.md coverage block (3959 items, unchanged across all 13 plans).
- **No provider key read/printed** (T-36-01, T-36-17, T-36-21, T-36-27, T-36-37): `grep -E 'OPENAI_API_KEY|ANTHROPIC_API_KEY|DEEPSEEK_API_KEY'` across all ten new/gated example programs — the only hit is `token_economy_commissary.rs`'s header comment stating no key is needed, not a read or print.
- **Webhook signature verification constant-time + secret never printed** (T-36-29, T-36-30): confirmed `hmac::Mac::verify_slice` used for comparison (never `==`) and no secret-print statement in `examples/webhook_receiver.rs`.
- **SSRF cloud-metadata always-rejected + documented DNS-rebinding limitation** (T-36-31): confirmed both the always-reject statement and the rebinding-limitation prose are present in `examples/webhook_receiver.rs`.
- **HTTP-service-host router parity** (T-36-33): confirmed all three routers (`agent_router`, `thread_router`, `run_router`) are mounted and merged in both `examples/http_service_host.rs` and `crates/doc-examples/src/http_service_host.rs`.
- **Closure-map completeness** (T-36-48): confirmed every `RD-01`..`RD-143` identifier appears in `36-EVIDENCE.md` — zero missing.
- **Real pushed-branch CI run recorded** (T-36-49): confirmed `36-CI-EVIDENCE.md` records a real `gh run view` result (run `35290763563`/head `20195975`) rather than only a local sweep.
- **`WINDOWS.md` moved only through `gsd-tools`** (T-36-50): confirmed rows 36 and 37 both show `status: fixed` with a non-null `resolved_at` timestamp.
- **No internal work-list identifiers in the published changelog** (T-36-51): confirmed `grep -cE 'RD-[0-9]+|EX-[0-9]+' CHANGELOG.md` returns 0.
- **`make doc-check` reproduces `ci.yml`'s lint-job bar commands byte-identically** (T-36-43): confirmed the two `cargo doc` invocations in `Makefile`'s `doc-check` target match `.github/workflows/ci.yml`'s "Check documentation" / "Check documentation (all features, -D warnings)" steps command-for-command.
- **Remaining mitigate-disposition threats** (information-disclosure prose constraints, environment-variable restore-after-use, fail-closed/fail-open demonstration fidelity, repudiation/traceability bookkeeping) are documentation- and example-behavior claims already covered by the 59/59 auto-passed UAT deliverables in `36-UAT.md`, each backed by a cited `cargo doc -D warnings` / `cargo test --workspace --doc` / `cargo build --examples` / `make api-surface` run recorded in `36-EVIDENCE.md` and `36-evidence/*.txt`.

No threat required escalation; no new threat was found beyond the 13 plans' own registers.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-18
