# Pitfalls Research

**Domain:** Adding cross-run spend governance (pricing, allowances, ledger, rate pacing) and
related clean-break/infra debt to an existing durable Rust multi-agent runtime — Milestone 14
"Treasurer" / v0.11.0
**Researched:** 2026-09-24
**Confidence:** HIGH (grounded directly in this repo's shipped code, PRD, ADRs and PROJECT.md —
not generic industry pitfalls) except where noted MEDIUM/LOW (RustFS specifics, which are
externally sourced)

This research is scoped narrowly to *this* codebase adding *these* features, not general "billing
system" advice. Every pitfall below cites the concrete file(s)/pattern already in the tree that
make the mistake likely, or the concrete PRD/ADR clause that constrains the fix.

## Critical Pitfalls

### Pitfall 1: `cost_estimate` is already a public `f64` — currency math done in floating point

**What goes wrong:**
`ExecutionMetadata.cost_estimate: Option<f64>` (`crates/paladin-core/src/platform/container/herald.rs:505`)
already ships as a public field on a `#[non_exhaustive]`-free struct, with a builder method
`cost_estimate(mut self, cost_estimate: f64)`. If the Treasurer computes `prompt_tokens *
price_per_token + completion_tokens * price_per_token + ...` in `f64` and writes the sum straight
into this field, every accumulation is subject to binary floating-point rounding error. Summed
across a campaign's many runs into a rolling allowance total, or persisted and re-read from a
ledger, these errors compound and a per-key allowance can drift out of sync with what a human
audit of raw token counts × published prices would compute — sometimes refusing a draw that
should be allowed, sometimes allowing one that should be refused.

**Why it happens:**
The field type was chosen (Phase 30, VOCAB-05) purely as a *reservation* — nobody was solving the
money-precision problem yet, they were writing rustdoc naming a future producer. `f64` was the
obvious "it's a number" choice and nobody revisited it because the field had no producer to force
the question. Now that this milestone builds the producer, the type is already semver-locked
public API, so the field itself cannot be silently swapped for a decimal/integer type without a
breaking change — and `cargo semver-checks` will catch a public field type change (X-10) even if a
human reviewer misses it.

**How to avoid:**
- Do currency math internally in integer minor-units (e.g. micro-cents, or a fixed-point decimal
  type such as `rust_decimal::Decimal`) and only convert to `f64` at the `cost_estimate` boundary
  for display/serialization — never accumulate in `f64`.
- Decide explicitly, and document in the pricing config: is a unit price "currency per token" or
  "currency per 1K/1M tokens"? Get this wrong and every cost is off by 3-6 orders of magnitude
  silently (no panic, no error — just a wrong number that "looks plausible" at small scale).
- The **allowance ledger and enforcement decision must never be made in `f64`.** Even if
  `cost_estimate` stays `f64` for backward compatibility, the Treasurer's internal running total
  against an allowance cap should be integer/decimal arithmetic, converted to `f64` only for the
  herald-facing field.
- If precision requirements make `f64` genuinely unacceptable for the public field, that is a
  deliberate breaking-change decision to surface early (own ADR, own MIGRATION row) — not
  something to discover after R2 ships.

**Warning signs:**
- A test asserting `cost_estimate == 0.1` after summing `0.1` three times fails intermittently.
- Allowance enforcement tests pass with round numbers (10000 tokens, $0.01/1K) but fail with
  irregular per-model prices (e.g. $0.0000027/token) — the classic floating-point smell.
- No dedicated newtype/decimal wrapper anywhere in `paladin-core`'s pricing module; raw `f64`
  arithmetic scattered across the pricing function and the ledger.

**Phase to address:**
The pricing/cost phase (R1/R2, FUT-08) — this is a foundational decision that every later phase
(allowances, ledger) depends on. Fix the internal representation before Treasurer/allowance
enforcement is built on top of it.

---

### Pitfall 2: Allowance check-then-act races across concurrent workers (double-spend)

**What goes wrong:**
The worker pool already runs multiple concurrent workers pulling from the Redis/in-memory run
queue (`crates/paladin-storage/src/run_queue/`) — that's the whole point of Phase 27's platform
API. If the Treasurer's admission check ("does this tenant/key still have allowance headroom?")
is implemented as a plain read-then-write against the ledger — `SELECT current_spend FROM
ledger WHERE key = ? ; if current_spend + estimated_cost <= allowance { INSERT/UPDATE }` — two
workers admitting two runs for the same key at nearly the same instant can both read the same
"headroom remaining" value, both decide to admit, and the allowance is oversubscribed. This is
structurally the same TOCTOU bug the Redis run-queue module went out of its way to avoid by
moving every claim/extend/ack/nack into one atomic Lua `EVAL` (module docs, `redis.rs:44-64`:
"never a multi-round-trip read/decide/write from this process, which two concurrent replicas
could otherwise interleave").

**Why it happens:**
The natural first implementation of "check an allowance" reads as ordinary application code (load
balance, compare, act) because that is how a single-process, single-writer prototype would work.
The multi-worker, multi-replica reality of this runtime (already proven adversarial in
`redis_run_queue_two_instances_sharing_one_prefix_never_double_claim`) is easy to forget when
writing a *new* subsystem that "just" reads a number and compares it.

**How to avoid:**
- Model the allowance draw as **reserve-then-settle**, not check-then-spend: at admission,
  atomically reserve the *estimated* cost against the allowance (decrementing headroom
  immediately, in the same atomic operation as the check) — do not separately "check" and then
  later "write." At completion, settle the reservation against the *actual* cost (refund the
  delta if actual < estimate, or — per R3 — halt the in-flight run cleanly if actual would exceed
  what remains).
- For the SQLite/Postgres ledger, reuse the existing `RunRepositoryPort` pattern's discipline:
  transactional check-and-write in one statement (e.g. `UPDATE ledger SET reserved = reserved +
  ? WHERE key = ? AND reserved + ? <= allowance` and inspect the row-count, the same idiom used
  for optimistic concurrency elsewhere in this codebase) rather than a separate `SELECT` +
  `UPDATE`.
- For the in-memory adapter, an `Arc<Mutex<...>>` (or per-key sharded lock) around the whole
  reserve operation is sufficient — but it must cover read+decide+write as one critical section,
  not just the write.
- Write the concurrency contract test first (TDD, per CLAUDE.md): N concurrent tasks each try to
  draw against an allowance sized for exactly N-1 of them; assert exactly N-1 succeed and one is
  refused, mirroring `redis_run_queue_concurrent_workers_each_message_exactly_once`'s shape.

**Warning signs:**
- A ledger port trait method named `check_allowance()` that is separate from
  `record_spend()`/`reserve()`, invoked from two different call sites (admission vs. completion)
  with no shared atomic operation between them.
- No concurrent-draw contract test exists (the "N-1 of N succeed" shape above) in the new ledger's
  test suite before it is wired into admission.
- The Postgres/SQLite adapter implements the reservation as two round trips instead of one
  UPDATE/RETURNING or equivalent.

**Phase to address:**
The Treasurer/allowances phase (R3) and the spend-ledger phase (R5) together — the ledger's write
API shape (reserve/settle, not check/record) must be designed jointly with the admission logic
that calls it, not designed ledger-first and retrofitted.

---

### Pitfall 3: Charging twice after lease loss / redelivery (no idempotent settle)

**What goes wrong:**
Phase 27 already documents the exact failure mode the ledger must survive: a worker claims a run
via `RunQueuePort::dequeue`, the lease expires (worker crash, GC pause, network partition) before
it acks, and the run queue redelivers the same run to a different worker with `attempt`
incremented (`RUN_QUEUE_CLAIM_LUA`'s reclaim path). PROJECT.md calls this "resume-not-restart
redelivery." If the Treasurer settles spend against the ledger as a side effect of each execution
*attempt* rather than each *run's actual token usage exactly once*, a redelivered run that
re-executes some already-billed work (or whose original worker was merely slow, not dead, and
also eventually finishes and settles) can settle the same tokens against the ledger twice —
directly analogous to a payment-processor double-charge from a retried webhook.

**Why it happens:**
The engine's checkpoint/resume model (Waypoint) is specifically designed so that resumed work
does *not* re-execute already-completed supersteps — but a naive Treasurer integration that hooks
"settle spend" onto "an LLM call returned `TokenUsage`" rather than onto "this specific,
idempotency-keyed unit of usage was recorded for the first time" will double-count exactly the
LLM calls that *did* execute more than once (e.g., a call whose response never reached the
original worker before its lease expired, so it re-executes on redelivery) or will double-settle
if the ledger write and the run's terminal state transition are not the same atomic operation.

**How to avoid:**
- Give every ledger-settling write an idempotency key derived from something stable across
  redelivery — e.g. `(run_id, superstep_seq, node_attempt)` from the existing `Waypoint`/attempt
  model, or a dedicated `ledger_entry_id` computed deterministically rather than a fresh UUID per
  settle call. Use `INSERT ... ON CONFLICT DO NOTHING` (Postgres/SQLite) or an equivalent
  upsert-once guard, not a bare `INSERT`.
- Settle spend as part of the *same* transaction/operation that records the terminal Waypoint or
  the trace's `RunFinished` event — not as an independent side effect that can happen once, twice,
  or (on a crash between LLM call and ledger write) zero times.
- Explicitly test the "worker crashes after LLM call succeeds but before ledger settle" case and
  the "lease expires, run redelivered, original worker's settle arrives late" case — these are the
  same class of bug Phase 27's cross-instance cancellation and redelivery tests already exist to
  catch for run state; the ledger needs its own version of that test.

**Warning signs:**
- Ledger settle logic lives inside the same code path as the LLM call itself (fire-and-forget
  write after receiving `TokenUsage`) with no correlation to the run's `attempt` counter.
- No idempotency key/unique constraint on the ledger's settle table.
- A chaos-style test that kills a worker mid-run and asserts on final ledger totals does not exist.

**Phase to address:**
The spend-ledger phase (R5), designed jointly with whatever hooks the Treasurer phase (R3) adds
into the existing worker-pool/lease/resume machinery from Phase 27.

---

### Pitfall 4: Rolling-window allowance math breaks on clock skew, DST, and worker-vs-DB clock disagreement

**What goes wrong:**
"Rolling per-tenant/per-API-key allowances" that "reset over rolling periods" (R3) require
comparing "now" against a window boundary. The run-queue Redis adapter already had to solve
almost this exact problem for lease expiry and deliberately chose the **server's own `TIME`
command** as the single clock source specifically because "two `RedisRunQueue` replicas never
disagree about whether a lease has expired" (D-08, `redis.rs:50-52`). If the Treasurer's rolling
window instead compares each worker's own local wall clock against a window boundary computed
once elsewhere (or against a stored `window_start` timestamp using a different clock), workers on
hosts with skewed clocks can disagree about whether a window has rolled over — one worker admits a
draw believing the window reset, another refuses believing it hasn't. Naive `"rolling 30 days"`
math built on local calendar/timezone arithmetic (rather than a fixed duration from a UTC anchor)
also breaks across DST transitions in whatever timezone a human-readable "resets at midnight"
requirement implies, and "midnight" is itself ambiguous across tenants in different timezones.

**Why it happens:**
"Rolling window" sounds like calendar arithmetic (a month, a day) and pulls the mind toward
`chrono`'s local-time/DST-aware types, especially since the PRD doesn't specify units. In a
distributed, multi-worker system, though, the correct mental model is "a fixed duration relative
to a UTC instant," identical in spirit to the lease-expiry problem this repo already solved once.

**How to avoid:**
- Store everything in UTC; define "rolling period" as a fixed `Duration` (e.g. sliding N-day
  window from each draw's timestamp, or a fixed-boundary window recomputed from a UTC epoch) —
  never local-calendar arithmetic.
- For the SQLite/Postgres ledger, use the database's own clock for window-boundary comparisons in
  the same statement that reserves/settles (mirroring the Redis adapter's "read the server's own
  now" pattern) rather than a value computed in the calling worker process and passed in.
- For the in-memory adapter (single process, so cross-replica skew is moot) still use
  `Utc::now()`, never `Local::now()`, and never mix the two within the same comparison.
- Decide and document explicitly whether "rolling" means a sliding window (recomputed relative to
  each draw) or a fixed-boundary window (resets at a scheduled instant) — the PRD's "resets over
  rolling periods, with an optional lifetime cap" is ambiguous between the two, and the choice
  changes both the math and the ledger query shape.
- Write a table test across DST-transition dates and leap-second-adjacent dates asserting window
  boundaries land where the fixed-duration model predicts, not where naive calendar math would put
  them.

**Warning signs:**
- `chrono::Local` or any timezone-aware "start of day"/"start of month" helper appears anywhere in
  the allowance-window code.
- Window-boundary computation happens in the calling worker process using that process's own
  `Instant`/`SystemTime` rather than the ledger backend's clock.
- No test exercises the ledger across a real DST transition or a window boundary under simulated
  clock skew between two "workers."

**Phase to address:**
The Treasurer/allowances phase (R3), same phase as Pitfall 2 — window semantics and the atomic
reserve/settle operation are two aspects of the same design and should be specified together.

---

### Pitfall 5: Streaming responses settle spend on a value that doesn't exist until the stream ends

**What goes wrong:**
Phase 31 (lossless token accounting) already established the "terminal-chunk `usage` contract" —
`StreamingResponse`/`ChunkMetadata` are `#[non_exhaustive]` and usage is only guaranteed complete
on the *final* chunk, proven by a shared conformance case across eight adapters. If the Treasurer
tries to make an admission decision, or worse, an early *refusal* decision, based on token usage
that is not yet known (because the response is still streaming), it either has nothing to check
against (usage is `None`/zero until the terminal chunk) or — if it naively uses a running partial
count from intermediate chunks — under- or over-estimates against models where prompt tokens
aren't reported until the end either. A run that streams a large completion could blow well past
an allowance before the Treasurer ever sees a `TokenUsage` to check against, because the *check*
can only happen with real numbers after the stream (and the cost) is already fully incurred with
the provider.

**Why it happens:**
Non-streaming request/response is the natural model to design against first, and it makes
"check-before-spend" trivially well-defined (call hasn't happened yet). Streaming inherently
inverts the timing: for admission you only have an *estimate* (context length, model, a
configured or historical per-token-count heuristic), and the *actual* cost is only knowable after
the provider has already been charged in the real world. The PRD's own text acknowledges this
implicitly by requiring "a run in flight halts cleanly ... when a draw would overspend" rather
than promising pre-flight exactness.

**How to avoid:**
- Admission-time checks against an allowance must be based on an **estimate** (e.g. the
  Commissary's own context-window/prompt-size accounting, which already exists and is
  synchronous/pre-call) — never on completion tokens, which cannot exist yet.
  - **Cross-check against ADR-0050:** the Treasurer *installs* `TokenBudget`, and `TokenBudget`
    already caps a run's accumulated `total_tokens` mid-run and finishes gracefully with
    `StopReason::TokenBudget` — that per-run mid-stream cutoff mechanism is exactly the shape R3's
    "resumable mid-run halt" should reuse for streaming, rather than inventing a second one.
  - For streaming specifically, drive the mid-run halt off the same node-level mechanism
    `TokenBudget` already uses to interrupt an in-progress run (which necessarily tolerates that
    the *exact* final count isn't known until the model stops), not off a single post-hoc
    admission check.
- Settle spend against the ledger only from the terminal chunk's `usage` (Phase 31's contract),
  never from a running estimate during the stream — that avoids the ledger ever recording a
  provisional number as final.
- Document explicitly (mdBook Treasurer page, R6) that pre-flight refusal is necessarily
  estimate-based and a rolling allowance can technically be driven negative by an in-flight
  streaming run that overshoots its estimate before the mid-run halt lands — this is expected
  behavior, not a bug, and should be stated as such rather than discovered by a confused operator.

**Warning signs:**
- Any code path treats a non-terminal `ChunkMetadata`'s `usage` field as authoritative for a
  ledger settle.
- The mid-run halt mechanism is built as new code parallel to `TokenBudget`'s existing
  `StopReason::TokenBudget` cutoff rather than composed with/reusing it, duplicating a
  fault-injection surface Phase 26 already built and tested.
- No test simulates a streaming run whose estimate undershoots reality and verifies the halt still
  produces a clean, resumable checkpoint (not a mid-stream panic or a silently unbounded overspend).

**Phase to address:**
The Treasurer/allowances phase (R3) for the mid-run halt mechanism; the pricing/cost phase (R1/R2)
for making sure the estimate function used at admission is clearly distinguished (in naming and
in code) from the settle function used at completion.

---

### Pitfall 6: Unknown/unpriced models silently produce `cost_estimate: None` where callers expect a number — or, worse, a model matched to the wrong price

**What goes wrong:**
R1/R2 state plainly: "It stays `None` for a model with no configured price." That's the right
default, but two related mistakes are easy to make in the surrounding code:
1. **Downstream consumers treat `None` as `0`.** A herald, CLI summary, or (critically) an
   allowance-enforcement path that does `spend += cost_estimate.unwrap_or(0.0)` silently treats an
   unpriced model's usage as *free*, which for an allowance system means a caller can exhaust
   real budget on an unpriced/mis-keyed model with zero ledger impact and no refusal — the exact
   opposite of the governance this milestone exists to add.
2. **Model-name matching in the pricing table is looser than the model strings LLM adapters
   actually emit**, so a *priced* model silently falls through to the unpriced path. This
   codebase already has nine-plus provider adapters (OpenAI, Anthropic, DeepSeek, Gemini, Ollama,
   Grok, Kimi, Qwen, the generic OpenAI-compatible adapter) each free to report its own
   model-string format (aliases, date-suffixed snapshots like `gpt-4-0613`, vendor-prefixed IDs).
   A pricing table keyed on exact string match against one canonical spelling will silently miss
   every alias/snapshot variant a provider actually returns.

**Why it happens:**
`Option<f64>::unwrap_or(0.0)` is the path of least resistance anywhere a plain number is wanted
for arithmetic, and it type-checks cleanly, so it's easy to write without thinking through the
governance implication. Model-string drift across nine adapters is an existing, already-observed
class of problem in this codebase (the shared LLM conformance suite exists specifically because
adapters diverge in subtle ways), not a new risk this milestone introduces — but pricing is the
first place a *silent* divergence (falls through to `None`/free) has a financial consequence
rather than just a herald cosmetic difference.

**How to avoid:**
- Distinguish, in both API and tests, "no price configured for a known model" (accept: cost stays
  `None`, herald shows "cost unknown," documented as R2's `None` default) from "an allowance
  admission/settle path saw a run whose model has no price." For the *ledger and allowance* path
  specifically, treat `None` as "unknown, flag it" — not as "zero" — e.g. log/trace a distinct
  event, and/or make the operator's config explicitly choose a policy (refuse admission for
  unpriced models under governance, vs. allow-and-don't-meter) rather than silently defaulting to
  free.
- Never `unwrap_or(0.0)` a `cost_estimate` in ledger/allowance code; make the "no price" case an
  explicit, named branch everywhere spend is aggregated for enforcement purposes.
- Normalize the pricing table's model-key lookup through whatever mechanism the codebase already
  uses for provider/model identification consistently (check whether `paladin_llm::window`'s
  model-window resolution — the Phase 32 shared precedence walk — has a reusable
  normalization/alias step; if so, reuse it rather than inventing a second, divergent model-name
  matcher for pricing).
- Test the pricing lookup against the *actual* model-string literals each adapter's own test
  fixtures/conformance suite already uses, not synthetic canonical names invented for the pricing
  tests alone.

**Warning signs:**
- `cost_estimate.unwrap_or(0.0)` or `cost_estimate.unwrap_or_default()` anywhere in allowance or
  ledger code (fine in a herald purely for display; not fine where it feeds an enforcement
  decision).
- The pricing table's key type is a bare `String` compared with `==` rather than going through a
  shared normalization function.
- No test cross-references the pricing table's expected keys against the model strings the shared
  LLM conformance suite (Phase 26) actually exercises across all nine adapters.

**Phase to address:**
The pricing/cost phase (R1/R2) for the lookup/normalization and the `None`-vs-zero distinction in
the type/API; the Treasurer/allowances phase (R3) for making sure enforcement code never silently
launders `None` into `0`.

---

### Pitfall 7: Retries and fallback models double-count spend

**What goes wrong:**
Phase 25 already built `FallbackLlmAdapter` (Transient/Unknown-only hops) and per-node retry with
`AttemptRecord` history. Both mechanisms mean a single logical "one LLM call the user asked for"
can correspond to *multiple* real provider calls under the hood — a retried request that failed
transiently and succeeded on attempt 2, or a request that fell back from model A to model B after
A returned a transient/unknown error. If the Treasurer sums `TokenUsage` (and therefore cost)
across *every* `AttemptRecord`/fallback hop rather than only the attempt whose result was actually
used, a single logical call gets billed for every failed attempt too — inflating spend beyond what
actually happened at the provider (a failed attempt that returned an error before any tokens were
generated costs nothing at most providers; one that failed after partial generation may cost
something; a successful fallback to model B should bill model B's price, not model A's, and should
not double-bill both A and B for the same logical unit of work).

**Why it happens:**
Retry/fallback trace events (`AttemptRecord`, per-attempt trace events from Phase 25) already
exist and are easy to iterate over — "just sum every `TokenUsage` seen for this node" is the
naive aggregation and will over-count whenever more than one attempt occurred. The correct
behavior (bill only the attempt(s) that actually incurred provider cost, at the price of the model
that actually served them) requires deliberately reading transience/outcome per attempt, not just
summing everything.

**How to avoid:**
- Settle spend per *attempt*, priced at that attempt's actual model, but base allowance/ledger
  accounting on the true provider-incurred cost: a transient failure with zero tokens returned
  costs nothing and should settle for `0`; a transient failure that did return partial usage
  before erroring should still bill that partial usage (the provider did the work); a successful
  fallback bills the model that actually served the final, used result plus any prior attempts
  that themselves incurred provider cost.
- Reuse `PaladinResult.served_by` (already added in Phase 25 specifically to record which model
  actually served a fallback) as the authoritative source for "which model's price applies to the
  successful attempt" rather than re-deriving it.
- Write a table test enumerating: (a) single attempt success, (b) N transient retries then
  success, (c) fallback A→B success, (d) all attempts exhausted/failed — asserting the expected
  ledger settle amount for each, cross-referenced against each attempt's actual `TokenUsage`.

**Warning signs:**
- Ledger settle code does `for attempt in attempt_history { total += price(attempt) }` with no
  distinction between attempts that incurred provider cost and ones that didn't, or without using
  `served_by`.
- No test exercises spend accounting through a fallback or a multi-attempt retry path — only
  through a single, successful, non-retried call.

**Phase to address:**
The spend-ledger phase (R5) in close coordination with the pricing/cost phase (R1/R2) — this is
where the "which `TokenUsage` counts" question must be answered, since it sits exactly at the
Aegis (Phase 25 fault-tolerance) / Token Economy (Phase 31 accounting) integration seam this
milestone is the first to actually spend money against.

---

### Pitfall 8: Cache-token pricing gets the discount direction or inclusion backwards

**What goes wrong:**
Phase 31 added `cache_read_tokens`/`cache_write_tokens` as separate, optional fields on
`TokenUsage`, and specifically corrected an "Anthropic cache-inclusive `prompt_tokens`" bug — i.e.
this codebase has already discovered, once, that different providers report cache tokens with
different inclusion semantics (some report `prompt_tokens` as already including cached tokens,
some report them separately/additively). A pricing function keyed naively on
`prompt_tokens * prompt_price + cache_read_tokens * cache_read_price + ...` will silently
double-count cache-read tokens for a provider whose `prompt_tokens` already includes them (paying
full prompt price *and* cache price for the same tokens), or under-price a provider where they are
genuinely additive. Cache-write tokens are also typically priced *more expensively* than a normal
prompt token (it's extra work to populate a cache), and cache-read tokens *less* — a pricing table
that applies one flat "cache" price to both write and read, rather than the PRD's stated
"prompt/completion/cache/reasoning" granularity, will misprice by provider-specific ratios that
can be a large fraction of total cost for heavy-cache workloads.

**Why it happens:**
The natural mental model of "sum the fields, multiply by their price" is exactly right *if and
only if* the fields are already mutually exclusive/additive — and Phase 31's own history shows
that was not a safe assumption to make about the raw provider data without adapter-level
correction. The correction already happened once for `prompt_tokens` inclusion; there's no
guarantee every future provider or provider-version keeps behaving the same way, and pricing adds
a second place (beyond the accounting fix) where the same inclusion-semantics bug can resurface.

**How to avoid:**
- Price against the *already-corrected*, adapter-normalized `TokenUsage` fields Phase 31
  guarantees are mutually exclusive (the "inclusive `total_tokens` contract" the milestone
  established) — never re-derive or re-infer inclusion from raw provider response bytes in the
  pricing function itself.
- Require the pricing table to carry distinct prices for `cache_read`/`cache_write`/`reasoning` as
  the PRD specifies (R1: "prompt/completion/cache/reasoning") — resist the temptation to
  special-case "if no cache price configured, fall back to prompt price," which quietly reproduces
  the "treat cache as regular prompt tokens" bug the schema was designed to avoid.
- Reuse the same per-adapter conformance-suite pattern Phase 31 used (a shared test case run
  across all provider adapters) to prove the pricing function's per-field summation matches a
  hand-computed expected cost for each adapter's actual `TokenUsage` shape, including the
  Anthropic case that was previously wrong.

**Warning signs:**
- The pricing function reads `total_tokens` and re-splits it heuristically instead of consuming
  the already-split `prompt_tokens`/`completion_tokens`/`cache_read_tokens`/
  `cache_write_tokens`/`reasoning_tokens` fields directly.
- Only one "cache price" exists in the pricing config schema instead of separate read/write
  prices.
- No pricing test exists per-adapter; only a single synthetic `TokenUsage` fixture is used across
  all pricing tests.

**Phase to address:**
The pricing/cost phase (R1/R2) — this must be validated against Phase 31's actual per-adapter
`TokenUsage` conformance fixtures, not invented fresh.

---

### Pitfall 9: Redis-shared rate pacing — thundering herd, lock expiry races, and fail-open-vs-closed left undecided

**What goes wrong:**
R4 requires "pacing shared across workers through Redis, and a distributed cache-stampede lock."
Unlike the run-queue's Lua-script atomicity (a proven, existing pattern in this codebase) or the
node-cache Redis adapter, **there is no existing stampede-lock precedent in this tree to copy** —
`grep` across `node_cache/redis.rs` for lock-acquisition patterns (`SET ... NX`) returns nothing.
This is genuinely new surface, and the classic failure modes are:
- **Thundering herd on backoff release:** if every worker computes the same `Retry-After` delay
  from the same 429 and all resume simultaneously, pacing does nothing to prevent a second
  synchronized burst — jitter must be added deliberately, per-worker, not just a shared flat delay.
- **Lock expiry race:** a `SET key value NX PX ttl`-style stampede lock that expires while the
  lock holder is still doing the protected work (e.g. TTL too short relative to actual backoff
  duration, or a slow worker/GC pause) lets a second worker acquire the "same" lock and both
  proceed simultaneously — the exact bug class the Redis run-queue's atomic Lua scripts and
  server-side `TIME` read exist to prevent for leases; a rate-pacing lock needs the equivalent
  discipline (server-time-based, or a fencing token / compare-and-delete-on-owner-match release,
  not a blind `DEL`).
- **Redis unavailable — fail-open vs fail-closed is an explicit design decision the PRD does not
  make and this codebase has already made *differently* in two other places:** the run queue
  (`RunQueuePort`) has an in-memory *fallback adapter*, i.e. Redis absence degrades to a
  functioning single-process queue (fail through, differently, not fail-open on the safety
  property); the webhook SSRF guard (`security.instructions.md`) fails *closed* (a bad target is
  always rejected, never defaults to "allow"). Rate pacing needs its own explicit answer: if Redis
  is unreachable, does pacing become purely in-process/per-worker (safe direction — may over-pace
  under legitimate concurrent capacity, but never allows an uncoordinated thundering herd against
  the provider) or does it silently stop pacing entirely (dangerous — could hammer a
  rate-limited/paying provider harder than the shared design intended)? Left unspecified, whichever
  branch is easiest to write under time pressure (usually: catch the Redis error and proceed as if
  unpaced) becomes the accidental default.

**Why it happens:**
This is the one requirement in the milestone with no adjacent in-tree adapter to imitate line for
line (unlike the ledger, which can mirror `RunRepositoryPort`, or the queue-adjacent locking,
which can mirror `RunQueuePort`'s Lua scripts). Without a pattern to copy, it's easy to reach for
`redis::AsyncCommands::set` with `NX`/`EX` options ad hoc rather than deliberately porting the
run-queue module's "everything atomic, server clock, explicit expiry semantics" discipline.

**How to avoid:**
- Explicitly reuse the run-queue Redis adapter's proven idioms: a Lua script for
  acquire-with-fencing-token and release-only-if-owner (never a bare `DEL`), server-side `TIME`
  for all expiry math (never the calling process's clock), and the same
  `ConnectionManager`/redaction-on-`Debug` construction pattern already established in
  `crate::node_cache::redis::RedisNodeCache` and `RedisRunQueue` — "no second Redis client type,"
  per the run-queue module's own stated convention, should extend to the pacing adapter too.
  Consider whether pacing's shared-rate-limit state can literally live in the same
  `paladin-storage` Redis surface (a fourth key namespace) rather than inventing a new module
  layout.
  - **`crate::node_cache::redis::redis_reachable`'s cheap-TCP-probe-before-connect pattern** is
    also directly reusable for pacing's own tests, for the same reason it exists there: avoiding a
    slow connect-timeout hang when the test Redis instance is absent.
- Add jitter (randomized, per-worker) on top of any shared `Retry-After`-derived delay, not just a
  single shared deterministic delay — this is the standard fix for synchronized-release thundering
  herd and costs little to add.
- Make the fail-open-vs-fail-closed decision *explicitly*, write it down (own ADR or at minimum an
  explicit line in the pacing design doc/PATTERNS), and test it: simulate Redis unavailable mid-run
  and assert the chosen behavior (recommended: degrade to conservative per-process pacing, never to
  "no pacing at all" — mirroring the webhook SSRF guard's fail-closed posture on the safety-critical
  side, while accepting the run-queue's precedent that Redis absence should degrade rather than
  hard-fail the whole subsystem).
- Set stampede-lock TTL with real headroom over the longest expected backoff, and prefer a
  heartbeat/extend pattern (exactly like `RUN_QUEUE_EXTEND_LUA`) over a single fixed TTL for any
  lock held across a variable-duration operation.

**Warning signs:**
- Lock release uses a bare `DEL` rather than a compare-and-delete-on-owner Lua script.
- Lock/pacing state expiry math uses `Instant::now()`/`SystemTime::now()` in the calling process
  rather than Redis's own clock.
- No test simulates two-or-more concurrent workers racing to acquire the same pacing lock (the
  `redis_run_queue_two_instances_sharing_one_prefix_never_double_claim` shape, adapted).
- No test simulates "Redis unreachable" for the pacing path and asserts a specific, intentional
  fallback behavior.
- All computed backoff delays are byte-identical across workers with no jitter term.

**Phase to address:**
The rate-pacing phase (R4, FUT-09) — flag this sub-item for **deeper phase-specific research**
before planning, per this milestone's own research-flag convention: unlike every other pitfall
above, this one has no adjacent in-tree pattern to lean on and the fail-open/fail-closed decision
is a genuine open design question the PRD does not resolve.

---

### Pitfall 10: Per-caller `GET /runs*` scoping leaks across tenants/keys via an overlooked route or filter bypass

**What goes wrong:**
WINDOWS.md row 32 already flags this as a **known, currently-open gap**: "single-tenant scoping of
the run-inspection routes" is carried forward, unresolved, from Phase 27 (WR-03). PROJECT.md
records the interim mitigation for the analogous thread-resume issue (Phase 24 CR-01: "any
authenticated role could resume any thread — now `require_admin` on resume, interim narrowing")
was itself only a stopgap pending this exact PLAT-06/row-32 fix. The obvious way to "fix" this is
to add a `WHERE tenant_id = ? AND api_key_id = ?` filter to the primary `list_runs`/`get_run`
handler — but historically, in systems with more than one entry point into "read a run" (the list
endpoint, the get-by-id endpoint, the SSE stream endpoint, the webhook-triggering internal lookup,
any admin/debug route), it's exactly the *secondary* entry point that gets missed, because the
primary list view is what gets tested and the single-record `GET /runs/{id}` or the SSE
subscribe-by-id path is checked separately (or not at all) for "does this caller's tenant/key
actually own this run_id."

**Why it happens:**
Scoping-by-filter on a list query is the intuitive, visible fix; scoping-by-ownership-check on a
direct-lookup-by-ID route is a different code shape (an authorization check, not a query filter)
and is easy to forget precisely because the direct-lookup handler "already worked" (returned data)
before this fix and continues to "work" (returns data — just for the wrong caller) after a
list-only fix ships.

**How to avoid:**
- Enumerate every route that can return run data by ID or by any filter — not just the primary
  list endpoint — before fixing: `GET /v1/runs`, `GET /v1/runs/{id}`, the SSE stream endpoint(s),
  and any webhook/notification path that echoes run details back to a caller-supplied target.
- Prefer a single, shared authorization check function (`assert_caller_owns_run(caller, run) ->
  Result<(), ApiError>`) invoked at the top of *every* run-data-returning handler, over duplicating
  a `WHERE` clause in each query — the same "one shared function everyone must call" discipline
  the webhook SSRF guard already uses ("a single table-tested function, never duplicated logic," per
  `security.instructions.md`), specifically because that discipline is what prevents exactly this
  class of "fixed the list view, missed the detail view" bug.
- Add a negative-path test per route: authenticate as tenant/key A, attempt to read a run that
  belongs to tenant/key B, assert `404`/`403` (not `200` with someone else's data) — for every
  route enumerated above, not just the list endpoint.
- Re-check whether `require_admin`'s interim narrowing on thread-resume (Phase 24 CR-01) should be
  relaxed once this fix lands, or whether it's actually a separate, still-needed control — don't
  assume fixing row 32 automatically also closes the Phase 24 interim mitigation's rationale.

**Warning signs:**
- The fix touches only the list-endpoint's SQL/query-builder code, with no corresponding change to
  a by-ID lookup handler or the SSE subscription handler.
- No negative-path ("caller B cannot see caller A's run") test exists per route.
- Authorization logic is duplicated (slightly differently) across more than one handler rather than
  centralized in one function.

**Phase to address:**
The platform-deviations phase (row 32 disposition) — this is explicitly named in the milestone
scope as work to close, and should be planned with the "one shared authorization function, tested
per route" pattern from the start rather than as a single-endpoint patch.

---

### Pitfall 11: RustFS is not a drop-in MinIO replacement — S3 API surface gaps, `mc`-equivalent tooling, and image/licence maturity are unverified assumptions

**What goes wrong:**
The todo item itself frames this correctly as "evaluate," not "swap" — but the temptation once a
feature-flagged adapter compiles and a happy-path smoke test passes is to declare parity and cut
over dev/test/CI/k8s all at once. Concrete gaps that have burned other projects making the same
MinIO→alternative-S3-store move, none of which are yet verified true or false for RustFS in this
todo's own "open questions" list:
- **Checksum/ETag behavior:** S3-compatible stores frequently diverge on multipart-upload ETag
  computation (MinIO/AWS compute ETag as an MD5-of-part-MD5s-with-part-count-suffix for multipart
  objects; a store that instead returns a plain MD5 or a different composite will silently break
  any caller — including this codebase's own tests — that treats ETag as a content-integrity
  check rather than an opaque cache-validation token).
- **`mc`-equivalent tooling absence:** the dev/test compose and any operational runbook that shells
  out to `mc` (bucket creation/bootstrap, policy setting) has no guarantee RustFS ships an
  equivalent CLI with the same subcommands/flags — this may require a different bootstrap
  mechanism entirely (SDK-based bucket creation at startup, rather than a compose `mc` init
  container), which is a bigger change than a docker-compose image-tag swap.
- **Presigned URLs and multipart uploads**, both explicitly flagged as open questions in the todo,
  are exactly the two S3 features most commonly incompletely implemented by newer S3-compatible
  servers — verify against what `paladin-storage`'s actual `FileStoragePort` adapter calls, not
  against a generic "S3-compatible" marketing claim.
- **Multi-arch image availability and release cadence** is the literal root cause of the outage
  this todo exists to prevent (Docker Hub deleting the community MinIO images). Adopting RustFS
  without verifying it has an actively maintained, multi-arch-published image with a healthy
  cadence risks reproducing the exact same failure mode with a different vendor in 12-18 months.
- **Licence acceptability under `cargo-deny` policy** is explicitly called out as unverified in the
  todo — this project already has an active `deny.toml` governance discipline (SECURITY-EXCEPTIONS.md)
  and RustFS's licence must clear that bar before any adapter ships, not be discovered after
  integration work is sunk.

**Why it happens:**
"S3-compatible" is a marketing/community label with no formal conformance test suite every
self-described implementer actually passes; the gaps above are the well-known, recurring set of
places where "S3-compatible" stores diverge from AWS/MinIO behavior in practice, not a Paladin-
specific risk. The pressure to move fast (this todo exists *because* CI was red-lined by a
disappeared image) makes "it started up and `docker compose up` worked" feel like sufficient
proof, when the actual proof needed is the adapter-parity contract-test-suite approach the todo's
own "Solution" section already specifies.

**How to avoid:**
- Follow the todo's own solution shape exactly: build the RustFS adapter behind its own Cargo
  feature flag, run the *existing* `FileStoragePort` contract-test suite unchanged against it, and
  only flip dev/test/CI/k8s over once that suite is provably green — do not treat a single manual
  smoke test as parity evidence.
- Explicitly add contract-test coverage for presigned URLs, multipart upload, and ETag format if
  the existing `FileStoragePort` contract suite doesn't already exercise them at the level of
  detail needed to catch a divergence (check the current suite before assuming it does).
- Verify multi-arch image publication and licence acceptability as literal go/no-go gates before
  writing adapter code, not as follow-up housekeeping after the adapter is built — both are cited
  as still-open in the todo and both can independently kill the whole approach.
- Keep the MinIO/S3 adapter and its pinned `quay.io` fallback alive and documented as the
  production-recommended path (per the todo's own fourth open question) unless/until RustFS is
  proven in production-shaped conditions, not just dev/test/CI.

**Warning signs:**
- The cutover PR touches `docker-compose.yml`/`ci.yml`/`k8s/minio.yaml` before or in the same
  change as the contract-test-suite proof, rather than strictly after.
- No test exercises presigned URLs or multipart upload against the RustFS backend specifically.
- `cargo deny check licenses` is not run against the new dependency before the adapter is merged.
- No evidence gathered on RustFS's actual image publish cadence/multi-arch support beyond "it has a
  Docker Hub page."

**Phase to address:**
The RustFS phase (FUT-10) — and per the todo's own "genuinely undecided" framing, this phase's
first work item should be closing the four explicitly-open questions (API surface coverage,
image/cadence/maturity, licence, whether production `k8s/minio.yaml` follows or stays separate)
before any adapter code is written, since any one of them could change the phase's scope
substantially. **Confidence: MEDIUM** on the specific RustFS gaps above (externally sourced
general S3-compatibility failure patterns, not confirmed against RustFS's actual current API
coverage) — treat this pitfall as "things to verify," not "confirmed RustFS defects."

---

### Pitfall 12: Clean-break removal of `RetryPolicy`/`ErrorStrategy`/`NodeError`/legacy timeouts/`PaladinError::LlmError(String)` breaks more call sites than the milestone description implies, and changes retry/transience behavior silently

**What goes wrong:**
This is not a small, contained deletion. `RetryPolicy` appears in 15 files across
`paladin-core`, `paladin-battalion`, `doc-examples`, and `src/application/services/paladin/
middleware/` (both `resilience.rs` and `context.rs`); `ErrorStrategy` appears in 8 files including
`maneuver` (the Flow DSL) and `commander.rs` (the strategy router); `PaladinError::LlmError`
appears in 27 non-test call sites spanning `paladin-ports`' own doctests, `paladin-eval`'s runner,
and — critically — `conclave_execution_service.rs`'s `is_retryable_error`, which currently
classifies retryability by **lowercasing and substring-matching the error message** ("rate
limit", "timeout", "network", "connection", "503", "429"). Two distinct risks:
1. **Scope creep risk:** a "clean break" that touches Formation/Phalanx/Campaign timeout handling,
   Battalion's error/retry/strategy types, *and* `PaladinError::LlmError`, intersects live,
   already-shipped subsystems (Conclave, Commander, Maneuver/Flow DSL, doc-examples, `paladin-eval`)
   that were not called out by name in the milestone's target-features summary. Treating this as
   "delete four things" undercounts the actual blast radius; every one of those 50 call sites needs
   a decision (migrate to the new Aegis-era mechanism, or is the call site itself dead code this
   removal should also delete).
2. **Silent behavior change risk:** `paladin_error.rs:256` currently classifies the legacy
   `PaladinError::LlmError(_)` arm as `Transience::Unknown` unconditionally ("the blanket answer the
   legacy arm always gave"), while `conclave_execution_service.rs`'s separate, *string-matching*
   `is_retryable_error` treats messages containing "429"/"rate limit" as retryable. Removing the
   variant forces every one of those call sites onto the *structured* `LlmError`/`Transience`
   taxonomy Phase 25 already built (which correctly distinguishes `RateLimitExceeded` from generic
   `Unknown`) — which is a **behavior improvement**, but only if every migrated call site is
   actually rewired to the structured taxonomy. If a call site is migrated by just swapping in a
   different string-erasing wrapper (e.g. `PaladinError::ExecutionError(msg)` with the same
   substring-matching retry logic left otherwise unchanged), the milestone ships a rename, not the
   fix, and the new rate-pacing feature (R4) — which needs a *reliable*, structured way to detect
   429/`Retry-After` — inherits the same fragility this removal was supposed to retire.

**Why it happens:**
`grep`-driven scoping ("remove X, Y, Z") undercounts real impact when the codebase has organically
accumulated ad hoc, string-based workarounds (like `conclave_execution_service.rs`'s message
substring matching) that depend on the *removed* type's stringly-typed shape without referencing
its name directly in an obvious way — the grep for `PaladinError::LlmError` finds it, but a
reviewer skimming file counts, not diffs, can still miss that this call site's *entire retry
strategy* rides on the string that variant used to carry.

**How to avoid:**
- Before writing removal code, produce a literal call-site inventory (the counts above are a
  starting point, not the final word) and classify each: (a) migrates cleanly to the new
  Aegis-era mechanism, (b) is dead/superseded code this removal should also delete outright, (c)
  is a *behavior-bearing* workaround (like the Conclave substring matcher) that needs deliberate
  rewriting, not mechanical find-replace.
- Specifically rewrite `conclave_execution_service.rs::is_retryable_error` (and any sibling pattern
  elsewhere) onto the structured `Transience`/`LlmError` taxonomy Phase 25 established, as part of
  this removal — not as a follow-up ticket — since R4's rate pacing depends on exactly this
  distinction being reliable.
- Follow the same recorded-supersession discipline the milestone description already promises: a
  new ADR (the X-03 supersession precedent from ADR-0051 is the template — "one coordinated
  downstream consumer... adopts the whole milestone at once," never a shim), one `MIGRATION.md`
  §9.2 row *per distinct breaking change* (not one row covering all four removals bundled), and a
  matching `cargo semver-checks` allowlist entry for each. Phase 32's own experience is directly
  relevant here: it found `cargo-semver-checks` **does not have a lint for inherent-method-arity
  changes** — discovered only empirically, not from tool output — so do not trust `semver-checks`
  alone to catch every shape of this removal; supplement with the same kind of manual empirical
  check (`--release-type minor` dry runs) Phase 32 used.
- Update `paladin-eval`'s runner and any doc-examples that reference the removed types/variant as
  part of this same phase, not as drift discovered later — `paladin-eval/src/runner.rs` already
  has commentary describing exactly why the old arm's blanket transience answer was a problem,
  which is a strong signal this call site needs deliberate attention, not mechanical deletion.

**Warning signs:**
- The removal PR's diff touches only the definition sites (`aegis.rs`, `node_error.rs`,
  `battalion/mod.rs`) and the compiler-forced call-site fixups, with no accompanying review of
  *why* each call site used the removed type/variant.
- `conclave_execution_service.rs`'s retry logic still does string matching after the removal
  lands, just against a different `PaladinError` variant's `Display` output.
- A single `MIGRATION.md` §9.2 row covers "removed legacy Battalion error/retry/timeout types and
  `PaladinError::LlmError`" as one bundled entry rather than one row per distinct breaking change
  (the milestone's own working agreement — CLAUDE.md's commit-per-parent-task discipline — implies
  this granularity anyway).
- `cargo semver-checks` reports fewer breaking-change lints than the actual number of removed
  public items, and nobody manually reconciles the gap (the Phase 32 precedent for exactly this
  under-detection).

**Phase to address:**
The legacy-clean-break phase — scope it explicitly around the full call-site inventory above, not
just the four named types/variants, and sequence it *before or alongside* the rate-pacing phase
(R4) since R4's reliability depends on the structured-error migration this removal forces.

---

### Pitfall 13: Tracing-overhead optimisation regresses correctness (dropped events, lost ordering) while chasing the ≤3% bar

**What goes wrong:**
PROJECT.md records the current, accepted state plainly: Phase 28's tracing overhead measured
+22.18% (log sink) / +18.46% (composite) against a ≤3% target, "non-CI-gating by D-37 and accepted
by maintainer sign-off." This milestone explicitly takes on closing that gap ("optimise
`TraceDispatcher::emit` / `LogTraceSink` serialisation toward PRD 07's ≤3% bar," D-16, row 35).
The obvious levers for a 6-7x overhead reduction — batching/buffering trace writes, sampling
(dropping some events), moving serialization off the hot path onto a background task/channel — are
each individually capable of silently breaking guarantees this codebase already depends on
elsewhere: the trace stream's "per-run gapless `seq`" (Phase 28's own design) and the
`RunStreamMode::Replay` feature built on persisted `run_traces` both assume **every** event is
captured, in order, with no gaps. A buffering/batching change that drops events under backpressure,
or an async hand-off that reorders events relative to the synchronous execution that produced them,
breaks `seq` gaplessness and therefore breaks replay correctness — a regression that will not show
up in a synthetic throughput benchmark (which is what would be used to confirm the ≤3% bar is met)
but will show up as a subtle, hard-to-reproduce replay bug in production.

**Why it happens:**
Overhead-reduction work is naturally validated against a performance benchmark (does the number go
down?), and the correctness properties it's easy to break (gapless sequencing, ordering,
completeness) are validated by a *different* test suite (the observability/replay tests) that
isn't necessarily re-run as part of "does this optimisation work" iteration — especially under
time pressure to hit a specific numeric bar.

**How to avoid:**
- Any optimisation that introduces buffering, batching, sampling, or async hand-off for trace
  events must be validated against the *existing* gapless-`seq` and replay-correctness tests
  (Phase 28's own suite) on every iteration, not just the throughput benchmark — treat "unchanged
  gapless-seq test result" as a hard gate on the optimisation work, equally weighted with the
  performance number.
- If sampling/dropping is ever considered as a lever, it must be an explicit, documented,
  opt-in policy decision (e.g. "drop verbose per-attribute detail under X" is fine; "may drop an
  entire `NodeFinished` event under load" is not) — never an incidental side effect of a
  backpressure-handling change.
- Prefer optimisations that reduce *serialization cost per event* (the PRD's own framing:
  "`TraceDispatcher::emit` / `LogTraceSink` serialisation") over optimisations that change *event
  delivery semantics* (buffering, batching, async) — the former class is much less likely to touch
  correctness properties at all.
- Re-run the synthetic all-Function-node benchmark from Phase 28 *and* the full observability test
  suite together as the phase's own exit gate, and record both numbers in the phase's evidence file
  — not just the improved percentage.

**Warning signs:**
- A proposed fix introduces a channel/queue between event emission and event persistence/export
  with no corresponding backpressure-drop test.
- The gapless-`seq` test suite is not re-run (or not even identified) as part of this phase's own
  verification plan.
- The optimisation PR's description talks only about the benchmark number, with no mention of
  replay/ordering correctness.

**Phase to address:**
The tracing-overhead phase (D-16, row 35) — should explicitly list "gapless-seq / replay
correctness unchanged" as a named exit criterion alongside the ≤3% overhead target, not as an
implicit assumption.

---

### Pitfall 14: `Treasurer` bleeds into downstream `GarrisonTreasury` vocabulary, or acquires an in-tree code symbol before it's actually meant to

**What goes wrong:**
ADR-0050 already establishes a precise, durable invariant: after Phase 30, `grep -rn Treasurer
crates src` should return **only** `///`/`//!` rustdoc lines, never a `struct`/`enum`/`trait`/
`mod`/`fn`/`impl`/`use`/`type` declaration or a path-qualified use — until *this* milestone
actually builds it. Two distinct ways this milestone can violate the guardrail the ADR sets up:
1. **The obvious one, now resolved by this milestone's own existence:** once Milestone 14 starts
   writing real code, `Treasurer` *should* become a real symbol — that's expected and correct. The
   risk is doing so *inconsistently* with the ADR's own stated scope: the ADR says the role "owns"
   allowances/pricing/pacing and "installs" `TokenBudget` rather than replacing it. Code that
   instead has `Treasurer` directly mutate/replace `TokenBudget`'s internals, or that renames
   `TokenBudget`/`TokenCounterPort`/`TokenUsage`/`max_tokens` in the course of wiring the Treasurer
   in, violates the "not renamed" guarantee PROJECT.md's Current Milestone section repeats
   explicitly ("the Commissary is untouched, and `TokenBudget`, `TokenCounterPort`, `TokenUsage`
   and `max_tokens` are not renamed").
2. **The downstream-collision one:** `Treasurer` must stay a **framework-only** word and never be
   used as, or confused with, the downstream Web3 Security Paladin app's `GarrisonTreasury` fixture
   — an *audit-target* domain term in that other repo, unrelated to this framework role. This is a
   documentation/convention guardrail, not a lint enforced by CI in *this* repo (ADR-0050 says so
   explicitly), which means nothing will mechanically stop a contributor from, e.g., writing an
   example or doc page that uses "treasury" in a way a downstream reader could plausibly confuse
   with their own fixture, or from choosing a sub-type name that echoes `GarrisonTreasury` (e.g. a
   hypothetical `GarrisonTreasurer` or `TreasuryGarrison` type) without anyone here having visibility
   into the collision.

**Why it happens:**
This is a cross-repo naming convention with no automated enforcement in this repo, maintained
purely by institutional memory (this ADR, PROJECT.md's repeated restatement) — exactly the kind of
constraint that erodes over a multi-phase milestone unless each phase's author deliberately
re-reads it, especially once "Treasurer" becomes a real, frequently-typed symbol and the earlier
carefulness about "is this just a rustdoc mention" naturally relaxes.

**How to avoid:**
- Re-read ADR-0050 and the PROJECT.md "Current Milestone" guardrail paragraph at the start of
  *every* phase in this milestone that touches Treasurer naming, not just the first one — treat it
  as a standing constraint, not a one-time gate.
- Keep `Treasurer` as the top-level role name for the new service/module, but audit any
  *sub-type* or *field* names introduced under it for accidental echoes of "Garrison" +
  "Treasury"/"Treasurer" combinations before merging — a quick manual check against the exact
  string `GarrisonTreasury`, not just `Treasurer`, at each phase's close.
- Verify with a grep-based check (mirroring Phase 30's own verification style: "symbol-scoped
  reserved-term grep") that `TokenBudget`/`TokenCounterPort`/`TokenUsage`/`max_tokens` identifiers
  are unchanged (same names, same public shapes except where this milestone's own PRD explicitly
  authorizes a change) at the close of every phase that touches the Treasurer/allowance wiring.
- When writing the mdBook Treasurer page (R6) and any new example, explicitly state the "installs,
  does not replace" relationship to `TokenBudget` in the page itself, mirroring the ADR's own
  wording — this keeps the constraint visible to future contributors reading docs, not just to
  those who read the ADR.

**Warning signs:**
- A new type or field name combines "Garrison" and "Treasur(y/er)" in the same identifier anywhere
  in `crates/`.
- `TokenBudget`, `TokenCounterPort`, `TokenUsage`, or `max_tokens` gets a rename, deprecation
  alias, or field-level restructuring as an incidental side effect of Treasurer wiring rather than
  as its own explicitly-scoped, ADR-recorded decision.
- The Treasurer directly holds/mutates `TokenBudget`'s internal state (rather than composing with
  it through its existing middleware install point at
  `src/application/services/paladin/middleware/limits.rs`) — a design smell that the "installs,
  does not replace" boundary is eroding.

**Phase to address:**
Every phase in this milestone that introduces a Treasurer code symbol should carry this as a
standing verification item (not a single dedicated phase) — most concretely the Treasurer/
allowances phase (R3) where the first real symbols land, and the docs phase (R6) where the public-
facing description of the boundary is written down for good.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|-----------------|------------------|
| Keep `cost_estimate` as public `f64`, do internal math in `f64` too | No breaking API change, fastest to ship R1/R2 | Compounding rounding error in a value that gates real refusal decisions (Pitfall 1) | Never for the internal allowance/ledger math; acceptable only for the already-public display field if internal accounting is decimal/integer |
| Check-then-act allowance admission (no atomic reserve) | Simple, reads like ordinary application code | Double-spend under the worker pool's real concurrency (Pitfall 2) | Never in the shared ledger path; acceptable only in a genuinely single-writer, non-concurrent dev/demo mode explicitly labeled as such |
| Settle spend as a fire-and-forget side effect of the LLM call | Least code to write, fastest path to a working demo | Double-charge on redelivery, or silent loss on crash-before-settle (Pitfall 3) | Never — this is the exact bug class Phase 27's redelivery model exists to make visible |
| Local-calendar rolling-window math (`chrono::Local`, "start of day") | Matches a human's intuitive mental model of "resets daily" | Cross-worker disagreement under clock skew, DST bugs (Pitfall 4) | Never for the enforcement path; acceptable only for a herald-facing human-readable "next reset" display computed from the canonical UTC value |
| `unwrap_or(0.0)` on `cost_estimate` in enforcement code | Type-checks immediately, avoids handling the `None` branch | Unpriced models silently spend for free against an allowance (Pitfall 6) | Never in allowance/ledger code; acceptable only in purely cosmetic herald/display formatting where "cost unknown" and "$0.00" are visually distinguished anyway |
| Sum every retry/fallback attempt's `TokenUsage` for billing | Simplest aggregation, no need to read `served_by`/outcome | Over-bills a run relative to actual provider-incurred cost (Pitfall 7) | Never |
| A single flat "cache token price" instead of separate read/write prices | Smaller pricing config schema | Mispricing proportional to a workload's cache-heaviness (Pitfall 8) | Only as an explicit, documented interim default while a provider genuinely has no published split price — not as the general design |
| Ad hoc `SET`/`DEL` Redis calls for the stampede lock instead of Lua-script fencing | Faster to write, no new Lua to test | Lock-expiry race lets two workers proceed concurrently (Pitfall 9) | Never for the shared production path; acceptable only in a throwaway spike explicitly not merged |
| Fix `GET /runs*` scoping only on the list endpoint | Smallest diff, closes the most visible instance of row 32 | Direct-lookup/SSE routes remain unscoped, leaving a real tenant-isolation leak (Pitfall 10) | Never — treat as incomplete until every route is covered |
| Declare RustFS parity from a manual smoke test | Fast, satisfying, unblocks CI quickly | Silent presigned-URL/multipart/ETag divergence discovered later in production (Pitfall 11) | Never for the cutover decision; a manual smoke test is fine as a first sanity check before running the real contract suite |
| Bundle all four legacy removals into one `MIGRATION.md` §9.2 row | Less documentation writing | Obscures which specific removal broke a given downstream call site; harder for the "one coordinated downstream consumer" to adopt incrementally (Pitfall 12) | Never — this milestone's own X-03/ADR-0051 precedent already establishes per-change rows |
| Reduce tracing overhead via buffering/sampling without re-running replay tests | Fastest path to the ≤3% number | Silent gaps in the gapless-`seq` guarantee, breaking `RunStreamMode::Replay` (Pitfall 13) | Never |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|-----------------|-------------------|
| SQLite/Postgres ledger via `RunRepositoryPort`-style adapters | Implementing "check allowance" and "record spend" as two separate port methods called from two separate code paths | Model as one atomic reserve/settle operation per backend (transactional `UPDATE ... WHERE` with row-count check, or equivalent), mirroring the discipline the Redis run-queue module already established for leases |
| Redis-shared rate pacing | Treating Redis pacing state as a simple counter with independent read/increment calls | Use Lua-script atomicity and server-side `TIME`, exactly like `RUN_QUEUE_CLAIM_LUA`/`RUN_QUEUE_EXTEND_LUA`; reuse `RedisNodeCache`'s `ConnectionManager`/redaction/`redis_reachable` idioms rather than inventing a new Redis client pattern |
| `TokenBudget` middleware (`src/application/services/paladin/middleware/limits.rs`) | Treasurer bypassing/duplicating `TokenBudget`'s existing mid-run cutoff (`StopReason::TokenBudget`) with a second, parallel mechanism | Treasurer composes with/installs `TokenBudget`, per ADR-0050 — reuse its existing mid-run interruption mechanism for the "resumable mid-run halt" requirement (R3) rather than building a second one |
| `paladin_llm::window` model-window resolution (Phase 32's shared precedence walk) | Pricing table does its own, independent model-string normalization/matching | Check whether the existing window-resolution model-name handling can be reused/extended for pricing lookups, to avoid two divergent notions of "which model is this" |
| `PaladinResult.served_by` (Phase 25) | Re-deriving "which model actually served this run" from raw attempt history instead of reading the field that already records it | Use `served_by` as the authoritative source for which model's price applies on a fallback-served run |
| Phase 28 trace stream (`TraceDispatcher`, gapless `seq`, `RunStreamMode::Replay`) | Optimising `TraceDispatcher::emit`/`LogTraceSink` for throughput without re-validating gaplessness/replay | Any optimisation must re-run and pass the existing observability/replay test suite as a hard gate, not just the throughput benchmark |
| Webhook SSRF guard pattern (`src/application/services/run/webhook/ssrf.rs`) | Building the `/runs*` scoping fix and the rate-pacing fail-closed decision as ad hoc, per-call-site logic | Reuse the "single, shared, table-tested function every caller must go through" discipline this codebase already applies to SSRF for both the run-scoping authorization check and (where applicable) the pacing fail-open/closed decision |
| Docker Hub / quay.io MinIO image (existing pin) | Assuming RustFS solves the "unmaintained image" problem without verifying its own publish cadence/multi-arch support | Explicitly verify RustFS's image maintenance posture before treating the migration as "fixed" — otherwise this milestone risks trading one single-point-of-failure image dependency for another |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|-----------------|
| Ledger reserve/settle as two Postgres/SQLite round trips instead of one atomic statement | Fine under a single test worker; intermittent allowance overspend only under real concurrent load | Single atomic UPDATE/RETURNING (or equivalent) per reserve/settle, proven by a concurrent-draw contract test | As soon as more than one worker draws against the same key concurrently — i.e. immediately in any real deployment with `worker_pool_size > 1` |
| Redis pacing lock TTL shorter than realistic backoff duration | Works in fast local tests; races appear only when a real provider's `Retry-After` is large (many seconds) or a worker is briefly GC-paused/network-slow | Heartbeat/extend the lock (mirroring `RUN_QUEUE_EXTEND_LUA`) rather than a single fixed TTL sized for the "happy path" duration | Under real provider rate-limit backoffs (which can be tens of seconds) or under host-level latency spikes |
| Flat, non-jittered shared backoff delay | Passes a single-worker pacing test; under real multi-worker load, requests re-burst in lockstep after every backoff window | Add per-worker randomized jitter on top of any shared delay | As soon as more than a handful of workers pace against the same shared 429 signal simultaneously |
| Tracing-overhead fix that adds an unbounded in-memory buffer ahead of the trace sink | Looks like a clean throughput win in a short benchmark | Under sustained load or a slow sink, the buffer grows unbounded (OOM) or is forced to drop events (breaking gaplessness) | At sustained trace-event rates the buffer's drain rate can't keep up with, which a short synthetic benchmark may not reveal |
| Pricing lookup re-parses/re-normalizes a model string on every settle call | Invisible at unit-test scale | Cache/precompute the normalized model→price mapping once per run rather than per token-usage event, if pricing lookup ever becomes hot-path | Only matters at genuinely high request-rate scale; note per this project's own success-metric discipline, no verified throughput baseline exists yet to size this against — do not over-engineer ahead of a measured need |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Ledger/allowance config (tenant/API-key identifiers, connection strings for a Postgres/SQLite ledger backend) `Debug`-formatted or logged outward | Credential/identifier leakage in logs, matching the exact class of issue `security.instructions.md` already governs for other adapters | Apply the same redaction discipline already established for `RedisRunQueueConfig`/`RedisNodeCacheConfig` (hand-written `Debug` impl redacting passwords/connection strings) to any new ledger or pacing config type |
| `/runs*` scoping fixed only at the list endpoint, leaving direct-lookup/SSE routes exposed | Cross-tenant data leak — a real confidentiality bug, not just a correctness bug (Pitfall 10) | Centralize authorization in one function called by every run-data route; add negative-path tests per route |
| Rate-pacing Redis lock released with a blind `DEL` rather than owner-checked release | A second worker's lock (acquired after the first's apparent-but-wrong release) can be released by the wrong owner, defeating the mutual exclusion the lock exists to provide | Fencing-token or compare-and-delete-on-owner-match release, via Lua script, mirroring the run-queue's atomicity discipline |
| Allowance enforcement fails open when the ledger backend (Postgres/SQLite/Redis-backed) is unreachable | A tenant/key whose allowance should refuse a draw is instead admitted during a ledger outage, defeating the entire governance feature at the exact moment it's most needed (an outage is also when cost-runaway risk is highest) | Decide and test the fail-closed posture explicitly for ledger unavailability at admission time (refuse rather than silently admit), distinct from — and likely stricter than — the fail-open-vs-closed decision for pacing (Pitfall 9), which has different safety tradeoffs |
| Pricing table or allowance config accepts operator-supplied values with no validation (e.g. negative prices, negative allowances) | A misconfigured negative price could make `cost_estimate` negative, which could corrupt ledger totals or accidentally *increase* remaining allowance on settle | Validate pricing/allowance config at load time (reject negative/NaN/infinite values) — the same "validate function arguments" discipline `.github/instructions/rust.instructions.md` already mandates generally |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-------------------|
| A refused draw at admission returns a generic error with no indication of *why* (allowance exhausted vs. unrelated failure) | Operators/callers can't distinguish "you're out of budget" from "the service is broken," leading to wasted debugging time or repeated retries against a hard refusal | A distinct, typed error/response for allowance exhaustion (distinguishable from other admission failures), carrying enough detail (which allowance, current usage, cap) to act on |
| A mid-run halt (R3's "resumable mid-run halt") surfaces as an opaque failure rather than a clearly-labeled, resumable state | Caller doesn't know the run can be resumed once more allowance is available, and may treat it as a permanent failure | Reuse the existing typed-error-plus-checkpoint-preserved pattern this codebase already has conventions for (Aegis's typed error handlers, Parley's `AwaitingInput`-style states) so the halt reads as "paused, resumable," not "crashed" |
| `cost_estimate: None` for an unpriced model renders in a herald/CLI as blank or as `0`/`$0.00` | Operators reading a cost report can't tell "this genuinely cost nothing" from "we don't know what this cost," undermining trust in the whole cost-reporting feature | Render `None` distinctly from `Some(0.0)` in every herald format (e.g. "cost: unknown (no price configured)" vs. "cost: $0.00") |
| Rate-pacing backoff silently slows a campaign with no visible signal to the operator watching progress | Looks like the system has stalled/hung rather than deliberately pacing against a provider's rate limit | Emit a trace event / log line when pacing engages (and when it releases), so the existing observability stack (Phase 28) makes pacing visible rather than invisible |

## "Looks Done But Isn't" Checklist

- [ ] **Pricing/cost (R1/R2):** often missing per-provider cache-token inclusion correctness —
  verify the pricing function is tested against each adapter's *actual* `TokenUsage` shape
  (including the previously-buggy Anthropic cache-inclusive case), not one synthetic fixture.
- [ ] **Allowance enforcement (R3):** often missing genuine concurrent-draw proof — verify a
  contract test exists that races N concurrent admissions against an allowance sized for N-1 and
  asserts exactly one refusal, not just a single-threaded "refuses when exhausted" unit test.
- [ ] **Mid-run halt (R3):** often missing the "actually resumable" proof — verify a halted run can
  be resumed (via the existing checkpoint/resume machinery) and completes correctly once more
  allowance is available, not just that it stops cleanly.
- [ ] **Spend ledger (R5):** often missing idempotency under redelivery — verify a test kills a
  worker mid-run (or simulates lease expiry + redelivery) and asserts the ledger's final total
  matches actual usage exactly once, not a happy-path-only settle test.
- [ ] **Rate pacing (R4):** often missing the fail-open/fail-closed decision under Redis
  unavailability — verify an explicit test exists for "Redis unreachable during pacing," not just
  the happy-path shared-pacing test.
- [ ] **Rate pacing (R4):** often missing jitter — verify backoff delays are not byte-identical
  across simulated concurrent workers.
- [ ] **`/runs*` scoping (row 32):** often missing coverage of non-list routes — verify negative-path
  tests exist for the by-ID lookup route and the SSE stream route, not just the list endpoint.
- [ ] **RustFS adapter (FUT-10):** often missing presigned-URL and multipart-upload contract-test
  coverage — verify the *existing* `FileStoragePort` contract suite (extended if needed) passes
  against RustFS specifically for these two features, not just basic put/get/delete.
- [ ] **Legacy clean-break:** often missing full call-site migration — verify
  `conclave_execution_service.rs`'s (and any sibling's) string-substring retry-classification logic
  was actually rewritten onto the structured `Transience` taxonomy, not just recompiled against a
  different `PaladinError` variant with the same string-matching shape underneath.
- [ ] **Legacy clean-break MIGRATION rows:** often missing granularity — verify `MIGRATION.md` §9.2
  carries one row per distinct breaking change (not one bundled row for all four removals) with a
  matching `cargo semver-checks` allowlist entry each.
- [ ] **Tracing optimisation (D-16):** often missing correctness re-verification — verify the
  gapless-`seq`/replay test suite was re-run and passed on the optimised code, with both numbers
  (overhead percentage and correctness-suite result) recorded in the phase's evidence file.
- [ ] **`Treasurer` guardrail:** often missing a final-close re-check — verify no new identifier
  anywhere in the milestone's diff combines "Garrison" and "Treasur(y/er)," and that
  `TokenBudget`/`TokenCounterPort`/`TokenUsage`/`max_tokens` public shapes are unchanged except
  where this milestone's own PRD explicitly authorizes a change.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|----------------|------------------|
| Floating-point drift already shipped in the ledger (Pitfall 1) | MEDIUM | Introduce an internal decimal/integer representation behind the existing port trait, backfill/reconcile the ledger from raw `TokenUsage` history if retained, and treat the public `f64` `cost_estimate` field as a display-only projection going forward — no breaking API change required if the fix stays internal |
| Double-spend discovered in production (Pitfall 2/3) | HIGH | Requires a ledger reconciliation pass (recompute expected spend from raw execution history/traces where retained), a hotfix to make reserve/settle atomic, and — since this affects real allowance enforcement — likely an operator-facing correction/credit mechanism; budget for this being the most expensive class of bug to recover from post-ship |
| Rolling-window clock-skew bug discovered in production (Pitfall 4) | MEDIUM | Migrate window-boundary computation to the ledger backend's own clock in a single coordinated change; may require a one-time backfill/recompute of existing window state, but does not require an API break |
| `/runs*` cross-tenant leak discovered after ship (Pitfall 10) | HIGH | Security incident response posture: patch every affected route atomically (not incrementally, to avoid a window where some routes are fixed and others aren't and callers/tests get a false sense of completion), audit access logs for evidence of actual cross-tenant reads during the exposure window, and disclose per whatever this project's security-issue process requires |
| RustFS parity gap discovered after dev/test/CI cutover (Pitfall 11) | LOW–MEDIUM | The MinIO/S3 adapter stays in the tree behind its own feature flag per the todo's own design (dual adapters, not a replace-in-place) — reverting compose/CI/k8s to the MinIO adapter is a config-only rollback, not a code rollback, provided the dual-adapter structure was actually followed |
| Legacy clean-break missed a call site's behavior migration (Pitfall 12) | MEDIUM | Because this is pre-1.0 with the "one coordinated downstream consumer adopts the whole milestone at once" model (ADR-0051 precedent), a missed call site can be patched in a follow-up release without a second deprecation cycle — but should still get its own `MIGRATION.md` row documenting the correction, not a silent fix |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|-------------------|----------------|
| 1. Floating-point money | Pricing/cost phase (R1/R2) | Decimal/integer internal representation proven by a rounding-stress test (repeated fractional accumulation); `cost_estimate` public type decision recorded explicitly (ADR if changed) |
| 2. Concurrent allowance double-spend | Treasurer/allowances phase (R3) + ledger phase (R5) | Concurrent-draw contract test (N workers, allowance sized for N-1, exactly one refusal) per backend |
| 3. Double-charge on redelivery | Ledger phase (R5) | Idempotency-key/unique-constraint test; simulated worker-crash-mid-settle test |
| 4. Clock skew / rolling window | Treasurer/allowances phase (R3) | UTC-only enforcement; DST-transition table test; ledger-backend-clock-sourced boundary test |
| 5. Streaming usage timing | Treasurer/allowances phase (R3) | Mid-stream halt test using `TokenBudget`'s existing cutoff mechanism; settle-only-on-terminal-chunk test |
| 6. Unknown/unpriced models | Pricing/cost phase (R1/R2) + Treasurer/allowances phase (R3) | No `unwrap_or(0.0)` in enforcement code (grep-checkable); pricing-lookup test against every adapter's real model strings |
| 7. Retry/fallback double-counting | Ledger phase (R5) | Table test across single-attempt / multi-retry / fallback / all-failed scenarios, asserting expected settle amount using `served_by` |
| 8. Cache-token pricing | Pricing/cost phase (R1/R2) | Per-adapter pricing conformance test reusing Phase 31's `TokenUsage` fixtures, including the Anthropic cache-inclusive case |
| 9. Redis pacing stampede/fail-open | Rate-pacing phase (R4) — flagged for deeper research | Two-instance concurrent-lock-acquisition test; explicit Redis-unavailable fallback test; jitter presence test |
| 10. `/runs*` tenant scoping leak | Platform-deviations phase (row 32) | Negative-path test per route (list, by-ID, SSE); single shared authorization function (grep-checkable: one call site pattern, not duplicated logic) |
| 11. RustFS parity gaps | RustFS phase (FUT-10) | Existing `FileStoragePort` contract suite green against RustFS, including presigned-URL/multipart cases; licence/image-cadence gates closed before adapter code merges |
| 12. Clean-break scope/behavior regression | Legacy-clean-break phase | Full call-site inventory reviewed (not just grep count); `conclave_execution_service.rs`-style string-matching logic rewritten, not just recompiled; one `MIGRATION.md` row + allowlist entry per distinct removal |
| 13. Tracing-optimisation correctness regression | Tracing-overhead phase (D-16, row 35) | Gapless-`seq`/replay test suite green alongside the ≤3% overhead number, both recorded in phase evidence |
| 14. `Treasurer` guardrail erosion | Every phase touching Treasurer naming (concentrated in R3, R6) | Per-phase-close grep for `GarrisonTreasury`-adjacent identifiers; `TokenBudget`/`TokenCounterPort`/`TokenUsage`/`max_tokens` shape-unchanged check |

## Sources

- **This repository, read directly (HIGH confidence, primary source for all pitfalls except #11):**
  - `.planning/PROJECT.md` (Current Milestone section, Phase 27/30/31/32 close-out records, WINDOWS.md rows 31/32/35, X-03 supersession record)
  - `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` (R1-R6, requirements and open questions)
  - `.planning/decisions/0050-treasurer-reservation.md` (ADR-0050 — Treasurer scope, `TokenBudget` install-not-replace rule, `GarrisonTreasury` downstream guardrail)
  - `.planning/codebase/CONCERNS.md` (pre-existing tech-debt patterns this milestone's new code must not repeat)
  - `.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md` (RustFS evaluation scope and open questions)
  - `crates/paladin-storage/src/run_queue/redis.rs` (the in-tree Lua-atomicity / server-clock / lease-expiry pattern this milestone's ledger and pacing work should mirror)
  - `crates/paladin-core/src/platform/container/herald.rs` (`ExecutionMetadata.cost_estimate: Option<f64>`, already-public field)
  - `crates/paladin-core/src/platform/container/paladin_error.rs` and `crates/paladin-battalion/src/conclave_execution_service.rs` (the legacy `PaladinError::LlmError` string-erasure and substring-matching retry classification this milestone's clean-break must actually fix, not just rename)
  - `crates/paladin-llm/src/error.rs` (structured `LlmError::RateLimitExceeded`, the correct signal for rate pacing)
- **General S3-compatibility failure patterns for Pitfall 11 (MEDIUM/LOW confidence — general industry knowledge, not confirmed against RustFS's actual current implementation):** ETag/checksum divergence across S3-compatible implementations, and presigned-URL/multipart-upload as commonly incomplete features in newer S3-compatible object stores, are well-known, recurring gaps across the S3-compatible-storage ecosystem generally. Treat every RustFS-specific claim in Pitfall 11 as "verify before relying on," per the todo's own "genuinely undecided" framing — no live RustFS-specific research was performed for this document; the pitfall is scoped from the *pattern* of what tends to go wrong with "S3-compatible" claims in general, cross-referenced against this project's own explicitly-still-open questions.

---
*Pitfalls research for: Paladin v0.11.0 "Treasurer Spend Governance"*
*Researched: 2026-09-24*
