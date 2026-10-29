# ADR-0063: One retry stack for model calls -- adapters are transport, resilience is decorators, one composition order

## Status

Proposed

**Date:** 2026-10-29

**Phase:** none yet. This ADR is a design proposal written against the v0.11.0 tree (`main` at `8e02a45`) for
discussion before any code moves. Its Stage 1 is proposed as a phase of the v0.11.1 milestone (`.project/v0.11.1/`);
Stages 2 and 3 are proposed for v0.12.0. The Code Conformance section therefore names the proofs each stage must
produce, not tests that exist today.

**Amends ADR-0058** (rate pacing) in one place only: the *mechanics* of D-03 "pace first, hop last" are re-expressed as
the hop's own retry loop (group (d) below); the observable behaviour D-03 fixes is kept. **Builds on ADR-0059** (clean
breaks in a 0.x release carry no deprecation cycle) and **ADR-0052** (pricing is the outermost decorator). It does not
re-open any ADR-0058 decision on gating, Redis, or the stampede lock.

## Context

v0.11.0 has one policy *type* for retries and four *implementations* of the model-call retry loop. The policy type is
the Aegis `RetryPolicy` (`crates/paladin-core/src/platform/container/aegis.rs:99`): total attempts, initial interval,
factor, cap, jitter, predicate. It is already what the WarEngine node retry, the Formation, Phalanx and Campaign
services, the Conclave bridge and the agent runtime's `ModelRetryMiddleware` all read, and `backoff_delay`
(`crates/paladin-battalion/src/engine/retry.rs:51`) is its one back-off function. That half of the unification is done.

Everything beside and below it is not:

1. **Nine private transport-retry loops, one per adapter.** OpenAI (`openai/adapter.rs:1004`), Anthropic
   (`anthropic/adapter.rs:768`), DeepSeek, Gemini (`gemini/adapter.rs:1012`) and the compat engine shared by Kimi,
   Qwen, Grok, Ollama and OpenAI-compatible (`compat/engine.rs:1185`) each run their own `for attempt in ..` loop
   over network, timeout and 5xx errors. None reads `RetryPolicy`. They share only the saturating
   `exponential_backoff` helper (`backoff.rs`) and otherwise disagree on everything measurable:

   | Loop | Counts | Base | Cap | Jitter |
   |---|---|---|---|---|
   | OpenAI, DeepSeek, compat presets | retries after the first (`0` sends once) | 1 s (OpenAI), 100 ms (compat) | 10 s | up to 20 % (OpenAI), up to 100 ms (compat) |
   | Anthropic, Gemini | total attempts (`0` and `1` both send once) | adapter-local `delay_ms` | 10 s | up to 200 ms |
   | Aegis `RetryPolicy` (the one the rest of the tree uses) | total attempts | 500 ms | 60 s | uniform in `[0, delay)` |

   `docs/src/user-guides/switching-providers.md:209` documents the two counting conventions as a known
   non-unification. None of these loops emits a trace event; a retried HTTP call is visible only in the adapter's log.

2. **A tenth loop in `VisionConfig`** (`config/vision.rs:13-25`): `max_retries`, `initial_backoff_ms`,
   `backoff_multiplier`, its own three knobs and its own `VisionError::MaxRetriesExceeded`.

3. **A legacy no-policy loop at the agent-loop call site.** `PaladinExecutionService::execute_with_retry_and_temperature`
   (`src/application/services/paladin/paladin_execution_service.rs:3086-3091`) reads `cx.retry_policy`; when no
   `ModelRetryMiddleware` installed one, it retries every non-`Permanent` error with a fixed `100 ms * 2^(attempt-1)`
   back-off for up to `max_loops.min(10)` attempts. That shape is kept "byte-for-byte" (X-03) and is undocumented: the
   server's `agent_runtime.model_retry` is `enabled: false` out of the box (`config.example.yml:240`), so every default
   deployment retries through this loop while the documentation says retry is off. A dead-code twin,
   `execute_with_retry` (`:3312`, `#[allow(dead_code)]`), sits beside it.

4. **Two inert or translated knobs.** `PaladinBuilder::retry_attempts` (`paladin_builder.rs:522`) stores
   `config.retry_attempts`, which nothing reads; `docs/src/user-guides/paladin-configuration.md:311` and the
   execution-service module docs show it as the way to configure retries. `ConclaveConfig::retry_attempts` counts
   retries, clamps to 5, and is derived by the Commander bridge as `aegis.retry.max_attempts - 1`
   (`commander.rs:546`).

5. **Four entry points for one composition.** `paladin::infrastructure::cadence::compose_llm`, `with_cadence`,
   `FallbackLlmAdapter::with_cadence` and `ModelFallbackMiddleware::paced` each build part of the same
   `Pricing(Fallback(Cadence(provider)))` stack, and a consumer must know which to call for which shape.
   `MIGRATION.md` §10.8's "Direct-adapter consumers" bullet is the only prose that tells them.

6. **Retry and pacing do not talk.** `backoff_delay` takes a policy and an attempt number; it never sees the error, so
   a `RetryPolicy` sleeping after a `429` ignores `LlmError::retry_after`. The `max(back-off, gate)` behaviour
   `cadence.rs`'s module docs describe is emergent from two independent sleeps and disappears the moment Cadence is
   not composed, which is every direct-adapter consumer by default.

The result for a developer new to the library is the question that prompted this ADR: "to get a 429 retried, which of
`max_retries`, `retry_attempts`, `model_retry`, `RetryPolicy`, `with_cadence` or `with_aegis` do I set, and in which
order do the decorators go?" Today the honest answer runs to a page.

What is *not* wrong, and this ADR keeps: the `Transience` classification (a `429` is `Transient`, a quota `429` is the
`Permanent` `UsageLimitExceeded`); the Cadence gate-not-retry rule (ADR-0058 D-01) and its D-06 cap; pace-first-hop-last
as observable behaviour (D-03); fleet pacing through Redis; pricing outermost (ADR-0052); `max_attempts` counting total
attempts; and the engine's node-scoped retry with its Waypoint and cancellation semantics. The problem is that the
model-call scope has four loops and the transport layer has ten, not that the layering is wrong.

## Decision

In one sentence: **adapters are pure transport; everything resilient is an `LlmPort` decorator; the decorators compose
in one fixed order through one builder; a model call is retried at exactly two scopes, and both read the same policy.**

### (a) Adapters are transport (D-01)

**D-01. A provider adapter makes one HTTP exchange and maps one answer.** No adapter retries, sleeps or counts attempts.
The nine loops in group 1 of the Context and the `VisionConfig` loop in group 2 are removed. The adapter's job becomes:
build the request, send it once, parse the status and body through `map_http_status_with_hints`, classify the error,
return. The `max_retries` field on every `*Config` and the `*_MAX_RETRIES` environment variables are removed as a
clean break (ADR-0059 precedent); a stored config carrying `max_retries` is refused at load with a message naming the
replacement, exactly as v0.11.0 refused `timeout_seconds` and `retry_policy` on `BattalionConfig`. The three
`VisionConfig` retry fields go the same way.

Rationale: a retry loop inside the adapter is invisible to every layer above it (no trace event, no attempt count the
caller can see), it is the place the nine conventions diverge, and ADR-0058 D-02 already took the `429` case out of
these loops for exactly that reason. Taking the rest out finishes the job. **Reversibility: costly** once the
conformance suite's attempt-count cases are rewritten against the decorator.

### (b) One retry decorator, one policy, provider delay as a floor (D-02, D-03)

**D-02. `RetryLlmAdapter` in `paladin-llm`** is a stateless `Arc<dyn LlmPort>` decorator in the shape of
`CadenceLlmAdapter` and `PricingLlmAdapter`: identity methods delegate, `model_profile` and `check_reasoning` are
forwarded (MIGRATION §10.8 DR-21), `with_*` constructor idiom. It takes the Aegis `RetryPolicy` and nothing else:

- **Attempts** are `policy.max_attempts`, total, including the first. There is no second counting convention anywhere
  in the tree after this.
- **Which errors** retry is `policy.retry_on.admits(err.transience())` -- the same pure predicate the engine and the
  agent loop already call. `Permanent` never retries; `TransientOnly` is the default; `TransientAndUnknown` is the
  opt-in; `Custom(name)` is refused at construction in this crate (there is no evaluator registry below the engine;
  see Open Question 3).
- **The delay** before attempt `n` is `backoff_delay(policy, n)` from `paladin-battalion`? No: `paladin-llm` does not
  depend on `paladin-battalion` and must not (ADR-0058 D-04 made the same call for Cadence). `backoff_delay` moves to
  `paladin-core` beside `RetryPolicy` as `RetryPolicy::delay_before(attempt)`, and `paladin-battalion::engine::retry`
  re-exports it. One formula, one home, two callers.
- **Streaming** retries only a failed future or a failed first item, using the first-item peek `FallbackLlmAdapter`
  and `CadenceLlmAdapter` already share; a mid-stream error after the first item passes through untouched. Nothing
  new is invented for streams.
- **An exhausted budget** returns `LlmError::RetriesExhausted { attempts, last }` (new, `#[non_exhaustive]`), whose
  `transience()` is the last error's, so a chain outside it hops on a transient exhaustion and short-circuits on a
  permanent one exactly as it does for the bare error today. The `attempts` list is the same shape
  `AllProvidersFailed.attempts` already carries. See Open Question 1 for the alternative.

**D-03. A provider's retry delay is a floor on the retry sleep.** When the error the decorator is about to retry carries
`LlmError::retry_after`, the sleep is `max(delay_before(n), retry_after)`. The floor is honoured in full up to the same
ceiling Cadence uses (`CadenceSettings::max_wait`, 300 s by default, passed to the decorator as its one setting beyond
the policy); a delay beyond it is not slept, the error is surfaced at once, mirroring ADR-0058 D-06 so a worker is
never held for an hour by a retry layer either. This closes Context group 6: a bare `Retry(provider)` stack with no
Cadence composed now respects `Retry-After`, and with Cadence composed the two floors agree because they read the same
number.

**Reversibility (D-02, D-03): reversible** until the adapter loops are deleted under D-01; thereafter costly.

### (c) One composition order, one builder (D-04, D-05)

**D-04. The stack is `Pricing(Fallback(Retry(Cadence(provider)), ...))`, in that order, always.** Each adjacency has one
reason:

- **Cadence innermost**, directly on the provider, so its key is the provider's own name (ADR-0058 D-07) and so every
  attempt from every layer above passes through the gate.
- **Retry outside Cadence**, so a retried `429` waits out the gate before it is re-sent, and so the retry sleep and the
  gate compose as `max`, now by construction rather than by accident.
- **Retry inside Fallback**, so each hop owns its own attempts and the chain sees one exhausted error per hop. This is
  where ADR-0058 D-03 is re-expressed: "retry the same provider after its gate, up to the pace budget, then hop" is
  exactly a per-hop `RetryLlmAdapter` whose wall-clock bound is `fallback_pace_budget`. `RetryPolicy` gains an optional
  `max_elapsed: Option<Duration>` (default `None`, additive) and the paced chain builder sets it from
  `CadenceSettings::fallback_pace_budget`. The chain's own `429`-re-entry code path in `fallback.rs` is deleted; the
  chain hops on any `Transient`/`Unknown` error it receives, with no exception, which is what its module docs said
  before Phase 43. Observable behaviour under D-03 is preserved: same provider, after its gate, bounded by the budget,
  then hop; `AllProvidersFailed.attempts` still lists every paced attempt because the hop's `RetriesExhausted` carries
  them.
- **Pricing outermost** (ADR-0052), unchanged.

A stack has **exactly one `RetryLlmAdapter` per hop**. Nothing above the chain adds a second retry loop around a model
call; the agent-loop call site (group (e)) and the engine's model call make one call each. **Reversibility: costly**
(the order is documented, tested and published as the meaning of the one builder).

**D-05. One builder, `LlmStack`, in the facade crate (`paladin::infrastructure::llm_stack`),** is the single way a
consumer or a composition root builds that order:

```text
LlmStack::new(provider)                     // or ::from_steps(Vec<FallbackStep>)
    .retry(RetryPolicy)                     // default: RetryPolicy::default()
    .cadence(Option<&CadenceWiring>)        // default: None (direct consumer), Some (server)
    .pricing(&Arc<PriceTable>)              // default: no pricing layer
    .build() -> Arc<dyn LlmPort>
```

`compose_llm`, `with_cadence` (facade and `paladin-llm`), `FallbackLlmAdapter::with_cadence` and
`ModelFallbackMiddleware::paced` are removed as a clean break in the same release; the server's composition roots
(agent host, facade provisioner, run engine wiring) call `LlmStack`. The doctest on `LlmStack::build` is the one
consumer recipe, and MIGRATION's "Direct-adapter consumers" bullet points at it. **Reversibility: reversible** (a
builder over existing decorators; the removed functions were thin).

### (d) ADR-0058 D-03 amended in mechanics only

Recorded separately so the amendment is visible: ADR-0058 D-03's *decision* (a `429` inside a fallback hop does not
hop immediately; the same provider is retried after its gate up to `fallback_pace_budget_secs`; a 5xx, timeout or
network error hops at once) **stands**. Its *implementation* (the chain re-enters the hop on a `429`, reading the gate
itself) is replaced by D-04's per-hop `RetryLlmAdapter` with `max_elapsed`. The one behavioural difference is
deliberate: today a 5xx inside a paced hop hops at once because the chain only re-enters on a `429`; under D-04 the
hop's retry policy decides, so a 5xx is retried on the same provider up to `max_attempts` before the chain hops. That
is the behaviour every adapter already had below the chain in v0.11.0 (its private 5xx loop), so the number of 5xx
re-sends a consumer observes does not change; only the layer that does them does. `TraceEvent::FallbackHop` still fires
only at a real hop.

### (e) The agent-loop call site makes exactly one call (D-06, D-07)

**D-06. `execute_with_retry_and_temperature` collapses to a single model call.** The retry loop, the `cx.retry_policy`
field, the legacy no-policy `100 ms * 2^n` arm, the `max_loops.min(10)` attempt bound, the dead `execute_with_retry`
and `PaladinError::MaxRetriesExceeded` are removed. The retry the call site used to perform is performed by the
`RetryLlmAdapter` inside the port it calls. The `MiddlewareAction::Retry` event the call site emits today is emitted by
the decorator through the existing `paladin::cadence`-style log target and, where a trace sink is attached, by the
resilience middleware observing `RetriesExhausted.attempts` on the way out (Open Question 2 decides whether a
`TraceEvent` variant is wanted). Temperature is a fixed input to the method, not varied per attempt (checked: the
"and_temperature" is the caller-supplied value, used identically on every attempt), so nothing about the loop needs
the call site.

**D-07. The resilience middleware is one port-shaping middleware.** `ModelRetryMiddleware` (sets a policy the call site
reads) and `ModelFallbackMiddleware` (sets `llm_override`) merge into `ModelResilienceMiddleware`, which builds one
`LlmStack` at construction from `agent_runtime.model_retry`, `agent_runtime.model_fallback` and
`AgentRuntimeDeps.cadence`, and sets it as `llm_override` in `before_model`. The service's own default port, built by
the composition root, is itself an `LlmStack` with `RetryPolicy::default()`, so a run with no middleware has a retry
policy that is documented rather than the legacy loop. `agent_runtime.model_retry.enabled` becomes `true` by default,
which is what today's default behaviour effectively is, except that the attempts and delays become the documented
`RetryPolicy::default()` (3 attempts, 500 ms, factor 2, 60 s cap, jitter) instead of up to 10 attempts at 100 ms
doubling. **Reversibility: costly** (the call-site loop and its tests are deleted).

### (f) The remaining knobs read the one policy (D-08)

**D-08.** `PaladinBuilder::retry_attempts(n)` is replaced by `PaladinBuilder::retry_policy(RetryPolicy)`, which the
builder hands to the stack it builds; `retry_attempts` is removed (it never worked). `ConclaveConfig::retry_attempts`
is replaced by `ConclaveConfig::retry: RetryPolicy`, and the Commander bridge's `max_attempts - 1` translation and the
clamp-to-5 go with it. `agent_runtime.model_retry` keeps its shape (it is already `RetryPolicy`-shaped) and gains
`max_elapsed_ms: Option<u64>`. No config key anywhere says "retries"; every one says `max_attempts`.
**Reversibility: reversible.**

### (g) Two scopes, named, and that is all (D-09)

**D-09. A model call is retried at exactly two scopes, and both read `RetryPolicy`.**

| Scope | Unit retried | Owner | Where the policy comes from | Only this scope has |
|---|---|---|---|---|
| **Call** | one `LlmPort::generate` / `generate_stream` | `RetryLlmAdapter` inside the hop | `LlmStack::retry`, fed by `agent_runtime.model_retry`, `PaladinBuilder::retry_policy`, or the consumer | the `retry_after` floor, `max_elapsed`, the gate |
| **Node** | one WarEngine node attempt inside a superstep | `engine::retry` | `Aegis.retry` per node, `BattalionConfig.aegis` | `RetryPredicate::Custom` evaluators, Waypoint isolation, cancellation-raced back-off |

The scopes compose multiplicatively: a node with `max_attempts: 3` over a call policy with `max_attempts: 3` can send
nine requests. This is stated in the docs as the meaning of two scopes. It is not new: v0.11.0 composed node attempts
with the adapter's private loop the same way, with a third loop at the call site on top. The engine's node retry is
**not** changed by this ADR. **Reversibility: n/a** (a statement of the model, not a mechanism).

### (h) Staging

| Stage | Release | Content | Break? |
|---|---|---|---|
| **1** | v0.11.1 (proposed phase) | A new mdBook page `docs/src/user-guides/retries-pacing-fallback.md` with the layer diagram, D-09's two-scope table as it stands *today*, the current composition recipe, and which error each layer handles; fix `docs/src/operations/troubleshooting.md:105-116` (its Rate Limiting fix tells users `max_retries` handles a `429`; it does not); state in `switching-providers.md:209` that the adapter counters exclude `429`; make `PaladinBuilder::retry_attempts(n)` install `RetryPolicy { max_attempts: n + 1, ..Default::default() }` into the service's default context or remove it from the docs (recommend: install it; a documented knob that does nothing is a bug under the milestone's own definition); **decide:** add the `retry_after` floor to `backoff_delay`'s callers in the execution service (a bare stack ignoring `Retry-After` is a defect against the provider contract, but it is the one Stage 1 item that changes timing). No MIGRATION §11 row. | No |
| **2** | v0.12.0 | D-01, D-02, D-03, D-04 (incl. group (d)), D-05: `RetryLlmAdapter`, `RetryPolicy::delay_before` and `max_elapsed`, adapters stripped, `max_retries` and `VisionConfig` retry removed, `LlmStack`, four entry points removed, conformance suite rewritten to prove "an adapter sends exactly once". | Yes: config keys, four public functions, `LlmError` gains a variant |
| **3** | v0.12.0 (same release; may be a later phase) | D-06, D-07, D-08: one call at the call site, `ModelResilienceMiddleware`, `MaxRetriesExceeded` removed, `retry_attempts` replaced on `PaladinBuilder` and `ConclaveConfig`, `model_retry.enabled` default `true`. | Yes: `PaladinError` variant, two builder methods, a default flips |

Stage 1 is the only part proposed for v0.11.1 because the milestone's scope guard (`03-suggestions-and-scoping.md` §5)
forbids a MIGRATION §11 break in a patch, and Stages 2 and 3 are breaks. Stage 1 has no dependency on Stages 2 and 3
and is worth shipping alone: it is the page a new developer needed in v0.11.0.

## Considered Options

- **Leave it: one policy type is enough unification** (rejected). The policy type is unified; the loops are not. The
  question that prompted this ADR could not be answered in under a page, and the default deployment retries through an
  undocumented loop while its config says retry is off.
- **Rewrite retry from scratch, one engine for both scopes** (rejected). The hard parts (`Transience`, gate-not-retry,
  pace-then-hop, Redis pacing, total-attempt counting, node isolation) are right and tested. A node retry and a call
  retry are different units of work with different side effects (Waypoints, cancellation); one loop cannot serve both
  without flags, and flags are what this ADR removes.
- **Keep the adapter loops but make them read `RetryPolicy`** (rejected). Nine loops that agree are still nine loops
  with no trace and no attempt count visible above them, and a per-run policy override would still need the call-site
  loop. The decorator is the same amount of code once, in one place.
- **Retry outside Fallback, `Pricing(Retry(Fallback(Cadence(h)...)))`** (rejected). A retry around the whole chain
  re-enters hop 1 after hop 2 failed, which is a provider switch and back for a transient error; and it has no per-hop
  budget, so pace-then-hop (ADR-0058 D-03) could not be expressed.
- **Keep the chain's own `429` re-entry beside a per-hop `Retry`** (rejected). Two loops on one hop multiply attempts,
  the exact failure ADR-0058 D-01 removed from the Cadence layer.
- **`ModelRetryMiddleware` wraps `effective_llm` in a second `RetryLlmAdapter`** (rejected). Double loop again. The
  middleware must *build* the stack with the policy, not wrap a stack that already has one.
- **Keep `compose_llm` and friends as deprecated aliases for one release** (not chosen, recorded). ADR-0059 set the
  0.x precedent of clean breaks with no shim; four thin functions with one-line replacements are the easiest case for
  it. If the operator prefers aliases, they are one `#[deprecated]` attribute each and change nothing here.
- **A new `llm.retry` config block beside `agent_runtime.model_retry`** (rejected). Two config homes for one policy is
  the problem restated. `agent_runtime.model_retry` already has the fields and sits beside `model_fallback`.
- **Return the last error unchanged on exhaustion, as Cadence does** (not chosen; Open Question 1). Loses the attempt
  list that `AllProvidersFailed` set the precedent for, and the call site's `MaxRetriesExceeded` consumers would have
  nothing to match.
- **Vary temperature per attempt at the call site** (never implemented, removed as an idea). The method's name
  suggested it; the code never did it.

## Code Locations

Today's, by Context group; the Stage that touches each is named.

- `crates/paladin-llm/src/{openai,anthropic,deepseek,gemini}/adapter.rs`, `compat/engine.rs` -- the nine private loops
  (Stage 2, removed).
- `crates/paladin-llm/src/backoff.rs` -- `exponential_backoff` (Stage 2, removed with its last caller).
- `crates/paladin-llm/src/config/vision.rs:13-25` -- `VisionConfig` retry fields (Stage 2, removed).
- `crates/paladin-core/src/platform/container/aegis.rs:99` -- `RetryPolicy` (Stage 2 gains `max_elapsed` and
  `delay_before`).
- `crates/paladin-battalion/src/engine/retry.rs:51` -- `backoff_delay` (Stage 2 becomes a re-export).
- `crates/paladin-llm/src/cadence.rs` -- `CadenceLlmAdapter`, the decorator shape `RetryLlmAdapter` copies (Stage 2,
  unchanged; `with_cadence` free function removed).
- `crates/paladin-llm/src/fallback.rs:14-36, 541` -- pace-then-hop re-entry and `with_cadence` (Stage 2, removed per
  group (d)).
- `src/infrastructure/cadence.rs:128, 203` -- `build_cadence` (kept), `compose_llm` (Stage 2, removed; `LlmStack`
  lands beside it in `src/infrastructure/llm_stack.rs`).
- `src/application/services/paladin/paladin_execution_service.rs:3020-3300, 3312-3430` -- the call-site loop and its
  dead twin (Stage 3, collapsed to one call).
- `src/application/services/paladin/middleware/resilience.rs` -- `ModelRetryMiddleware`, `ModelFallbackMiddleware`
  (Stage 3, merged).
- `src/config/agent_runtime.rs:493-496, 1026` -- `model_retry` wiring and `ModelRetryConfig` (Stage 3).
- `src/application/services/paladin/paladin_builder.rs:522` -- `retry_attempts` (Stage 1 fix or Stage 3 replace).
- `crates/paladin-core/src/platform/container/battalion/conclave.rs:117, 167` and
  `crates/paladin-battalion/src/commander.rs:546` -- Conclave `retry_attempts` and the bridge (Stage 3).
- `docs/src/operations/troubleshooting.md:105-116`, `docs/src/user-guides/paladin-configuration.md:311-317`,
  `docs/src/user-guides/switching-providers.md:209-211`, `MIGRATION.md` §10.8 "Direct-adapter consumers" -- the prose
  Stage 1 corrects and the new page supersedes.

## Code Conformance

Proposed proofs, one per decision; the plan that lands each stage names the test.

- **D-01.** The shared conformance suite (`conformance.rs`) gains the case *an adapter sends exactly once on every
  status*: a mock answering 503 then 200 sees one request and the adapter returns the 503 error. Case 10
  (`rate_limit_is_surfaced_once_with_its_retry_delay`) stays and becomes a special case of it. A `grep` for
  `for attempt` under `crates/paladin-llm/src/` outside `retry.rs` and tests returns nothing.
- **D-02.** `paladin_llm::retry::contract_tests` in the style of `paladin_storage::cadence::contract_tests`: total
  attempts equal `max_attempts`; `Permanent` is never retried; a failed stream future is retried and a mid-stream
  error is not; `RetriesExhausted.transience()` equals the last error's; `model_profile` and `check_reasoning` are
  forwarded. `delay_before` has the same table test `backoff_delay` has today, moved with it.
- **D-03.** A `429` with `retry_after: 7 s` under a policy whose computed delay is 500 ms sleeps at least 7 s (tokio
  paused clock); one with `retry_after: 301 s` returns at once without a second request.
- **D-04.** `LlmStack::build` over a `MockLlmAdapter` yields `Pricing(Fallback(Retry(Cadence(..))))` as read through
  `Debug`; a 429 on hop 1 under a `max_elapsed` of 60 s is re-sent to hop 1 after its gate and hops only once the
  budget is spent; `AllProvidersFailed.attempts` lists the paced attempts. The existing 43-06 pace-then-hop tests are
  re-targeted at the stack rather than deleted, so D-03's observable behaviour is proven to survive.
- **D-05.** Every composition root in `paladin-server` reaches an `LlmPort` only through `LlmStack` (a `grep` for the
  four removed names returns nothing outside `CHANGELOG.md` and `MIGRATION.md`).
- **D-06.** `execute_with_retry_and_temperature` contains one `generate`/`generate_stream` call and no `loop`; the test
  `transient_and_unknown_failures_still_retry_until_max_attempts` moves to the decorator's contract tests;
  `MiddlewareAction::Retry` is emitted once per actual retry (the WR-01 invariant, now proven at the decorator).
- **D-07.** `model_call_port_is_resolved_at_exactly_one_point` stays green; a run with `model_retry.enabled: false`
  sends exactly once on a transient error.
- **D-08.** `PaladinBuilder::retry_policy` reaches the stack (a mock counting attempts sees `max_attempts`);
  `ConclaveConfig` round-trips `retry` and refuses a document carrying `retry_attempts`.
- **D-09.** The new docs page's two-scope table is included from a doctest'd example in `crates/doc-examples` so it
  cannot drift (the `{{#include}}` idiom `fault-tolerance.md` already uses).

## Migration rows (proposed MIGRATION.md §11, v0.11.x → v0.12.0)

| # | Item | What changes | Replacement |
|---|---|---|---|
| M-C-01 | `llm.<provider>.max_retries`, `*_MAX_RETRIES` env vars, `*Config::max_retries` | removed; a document carrying the key is refused at load | `agent_runtime.model_retry` (server) or `LlmStack::retry(RetryPolicy)` (library). `max_retries: N` was "N retries after the first" on OpenAI/DeepSeek/compat and "N total" on Anthropic/Gemini; `max_attempts` is total everywhere. |
| M-C-02 | `VisionConfig::{max_retries, initial_backoff_ms, backoff_multiplier}`, `VisionError::MaxRetriesExceeded` | removed | the vision adapters sit under the same `RetryLlmAdapter`; `LlmError::RetriesExhausted` |
| M-C-03 | `paladin::infrastructure::cadence::{compose_llm, with_cadence}`, `paladin_llm::cadence::with_cadence`, `FallbackLlmAdapter::with_cadence`, `ModelFallbackMiddleware::paced` | removed | `paladin::infrastructure::llm_stack::LlmStack` |
| M-C-04 | `LlmError` | gains `RetriesExhausted { attempts, last }` (`#[non_exhaustive]`) | match `LlmError::RetriesExhausted { .. }`; its `transience()` is the last error's |
| M-C-05 | `RetryPolicy` | gains `max_elapsed: Option<Duration>` (`#[serde(default)]`) and `delay_before(attempt)` | a full struct literal adds `max_elapsed: None` or uses `..Default::default()` |
| M-C-06 | `PaladinError::MaxRetriesExceeded(u32)` | removed | `PaladinError::LlmFailure` wrapping `LlmError::RetriesExhausted` |
| M-C-07 | `PaladinBuilder::retry_attempts(n)` (inert since v0.10) | removed | `PaladinBuilder::retry_policy(RetryPolicy { max_attempts: n + 1, ..Default::default() })` |
| M-C-08 | `ConclaveConfig::retry_attempts`, `with_retry_attempts` | removed; a stored document carrying the key is refused | `ConclaveConfig::retry: RetryPolicy`, `with_retry(..)` |
| M-C-09 | `ModelRetryMiddleware`, `ModelFallbackMiddleware` | merged | `ModelResilienceMiddleware` |
| M-C-10 | `agent_runtime.model_retry.enabled` | default `false` → `true`; the legacy no-policy loop (up to 10 attempts at 100 ms doubling) is gone | set `enabled: false` to send once; the default is `RetryPolicy::default()` |
| M-C-11 | A 5xx inside a paced fallback hop | retried on the same hop up to `max_attempts` before the chain hops (was: hop at once; the adapter's private loop did the same re-sends below the chain) | none required; set `model_retry.max_attempts: 1` to restore hop-at-once |

## Downstream Consumers

- **v0.11.1 (Stage 1 phase).** The new docs page, the three prose corrections and the `retry_attempts` fix. It reads
  this ADR's Context and D-09 for the "today" picture and must say plainly that the stack it documents is the one
  Stage 2 replaces, with a pointer here.
- **v0.12.0 roadmap (Stages 2 and 3).** Two or three phases; the MIGRATION §11 rows above; `make api-surface-update`
  and the `.project/current-exports.txt` baseline; `CHANGELOG.md` `[0.12.0]` Breaking entries; `openapi.json` is not
  touched (no HTTP surface changes).
- **ADR-0058.** Its D-03 paragraph and Code Locations for `fallback.rs` need an "amended by ADR-0063 (mechanics)" line
  when Stage 2 lands; nothing else in it changes.
- **The conformance suite and `paladin_storage::cadence::contract_tests`.** Case 10 stays; the new "sends exactly
  once" case and the `RetryLlmAdapter` contract tests join them.
- **The web3sec consumer and any direct-adapter consumer.** M-C-01 and M-C-03 are the rows that reach them; both have
  one-line replacements. Per the v0.11.1 single-bump rule in `47-HANDOFF-web3sec.md`, these land in v0.12.0, not in
  the patch.
- **Treasurer docs (`docs/src/architecture/treasurer.md`, `configuration.md` §Cadence).** Unchanged in substance;
  each gains one cross-link to the new page.

## Open Questions (for the Stage 2 plan; recommendations given)

1. **Exhausted-error shape.** `LlmError::RetriesExhausted { attempts, last }` (recommended: matches
   `AllProvidersFailed`, gives callers the count `MaxRetriesExceeded` used to give) versus returning the last error
   unchanged (simpler, loses the count).
2. **A `TraceEvent` for a call-level retry.** ADR-0058 rejected a Cadence trace variant as a published-schema change
   for no requirement; the same reasoning applies here unless the resilience middleware's `MiddlewareAction::Retry`
   event, emitted from `RetriesExhausted.attempts`, is judged insufficient. Recommend: no new variant; log target
   `paladin::retry` plus the existing middleware event.
3. **`RetryPredicate::Custom` below the engine.** `RetryLlmAdapter` has no evaluator registry. Recommend: refuse at
   construction with a typed error naming the two admitted predicates, as `Aegis::validate` refuses `max_attempts: 0`.
4. **Whether `max_elapsed` lives on `RetryPolicy` or is a `RetryLlmAdapter` option.** Recommend `RetryPolicy`: it is a
   property of the budget, the engine's node scope could use it too, and `agent_runtime.model_retry` is
   `RetryPolicy`-shaped so it gets the key for free.
5. **Stage 1's `retry_after` floor.** In or out of v0.11.1 is the milestone owner's call; this ADR recommends in, as a
   bug fix, with the timing change named in the CHANGELOG `[0.11.1]` Fixed entry.
