# Phase 43: Rate Pacing - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-07
**Phase:** 43-rate-pacing
**Areas discussed:** Back-off ownership, Pacing semantics & degradation, Naming, config & defaults, Stampede lock placement

---

## Back-off ownership

### Who waits and re-issues the call after a 429?

| Option | Description | Selected |
|--------|-------------|----------|
| Decorator gates, existing layers retry | Decorator records a per-(provider, model) not-before instant, surfaces the 429 unchanged; RetryPolicy and FallbackLlmAdapter keep owning retries | ✓ |
| Decorator retries itself | Decorator sleeps and re-issues, bounded by its own attempt count | |
| Decorator gates and also retries once | Gate the key and retry exactly once after the delay | |

**User's choice:** Decorator gates, existing layers retry (recommended).

### What happens to the adapters' internal 429 retry loops?

| Option | Description | Selected |
|--------|-------------|----------|
| Surface 429 immediately | Adapters stop retrying 429 in their loops; network/5xx retries untouched; per-adapter conformance case | ✓ |
| Keep adapter loops, honour Retry-After inside them | Two layers sleep on the same signal | |
| Leave adapters alone | Adapter loop thrashes first, decorator paces later calls | |

**User's choice:** Surface 429 immediately (recommended). Asked twice because the first call was closed before an answer was recorded.

### What should a 429 on provider A do to a fallback chain?

First round (operator asked to pause and discuss before choosing):

| Option | Description | Selected |
|--------|-------------|----------|
| Hop now, gate A for later calls | Today's behaviour; A's key gated for subsequent calls | |
| Wait out A's pace before hopping | Sleep A's delay, retry A, then B | |
| Hop unless B is gated too | Fallback consults pacing state | |

**Notes:** The operator flagged immediate hopping as a footgun: the phase exists so a 429 retries the same provider after a delay, and a mid-run provider switch harms benchmark consistency. Discussion established that a 429 loses no provider-side state (every request carries the full history) and that the real costs of hopping are consistency, the prompt cache and pacing never learning the provider's rate.

Second round:

| Option | Description | Selected |
|--------|-------------|----------|
| Pace first, hop last | Retry A after Retry-After up to a configurable cumulative wait ceiling (default ~60s); only then hop. 5xx/network hop immediately | ✓ |
| Never hop on 429 | A 429 always stays on A | |
| Hop immediately on 429 | Keep today's behaviour | |

**User's choice:** Pace first, hop last (recommended).

---

## Pacing semantics & degradation

### Reactive only, or proactive from headers?

| Option | Description | Selected |
|--------|-------------|----------|
| Reactive only | Gate only after a 429; headers parsed and carried but never gate alone | ✓ |
| Proactive from headers | Space calls when remaining drops below a threshold | |
| Reactive now, proactive later | Reactive ships; proactive deferred | |

**User's choice:** Reactive only (recommended). Proactive gating recorded as a deferred idea.

### What does conservative per-process pacing mean when Redis is down?

| Option | Description | Selected |
|--------|-------------|----------|
| Local gate with a stricter multiplier | In-process gate, delays × configurable factor (default 2x), one warning per outage, auto-recovery | ✓ |
| Local gate, same delays | Identical in-process gate | |
| Local gate plus a fixed floor | Minimum interval for any key that has seen a 429 | |

**User's choice:** Local gate with a stricter multiplier (recommended).

### What if Retry-After would block a run for a very long time?

| Option | Description | Selected |
|--------|-------------|----------|
| Cap the gate, surface beyond it | Wait up to max_wait; beyond it surface RateLimitExceeded with the full delay immediately | ✓ |
| Honor it fully | Wait however long the provider asked | |
| Cap and treat as provider down | Reclassify as non-transient beyond the cap | |

**User's choice:** Cap the gate, surface beyond it (recommended).

---

## Naming, config & defaults

### Medieval Military name for the pacing officer

| Option | Description | Selected |
|--------|-------------|----------|
| Quartermaster | PacingLlmAdapter + QuartermasterPort, treasurer.pacing | |
| Drummer / Cadence | CadencePort, CadenceLlmAdapter, treasurer.cadence | ✓ |
| Plain: Pacing | PacingPort, PacingLlmAdapter, treasurer.pacing | |

**User's choice:** Cadence (operator's own pick over the recommended Quartermaster).

### On by default or opt-in?

| Option | Description | Selected |
|--------|-------------|----------|
| On by default, in-process | Omitting treasurer.cadence yields in-process pacing; Redis opt-in; `enabled: false` turns it off | ✓ |
| Opt-in only | Inert until configured | |

**User's choice:** On by default, in-process (recommended).

### Where does the Redis adapter live and which feature gates it?

| Option | Description | Selected |
|--------|-------------|----------|
| paladin-storage, new feature | Beside node_cache/redis.rs and run_queue/redis.rs, shared redis dep, new feature; lock shares it | ✓ (as `redis-cadence`) |
| paladin-storage, reuse redis-cache feature | Fold under redis-cache | |
| paladin-llm with its own redis dependency | Adapter next to the decorator | |

**User's choice:** The first option, with the feature named `redis-cadence` instead of `redis-pacing`.

---

## Stampede lock placement

### Where does the lock live?

| Option | Description | Selected |
|--------|-------------|----------|
| Lock on CadencePort, engine calls it | try_lock/unlock on CadencePort; engine's node-cache binding takes an optional cadence handle; NodeCachePort untouched | ✓ |
| Extend NodeCachePort | Lock methods on the cache port | |
| Separate StampedeLockPort | A third port | |

**User's choice:** Lock on CadencePort, engine calls it (recommended).

### What does a lock loser do?

| Option | Description | Selected |
|--------|-------------|----------|
| Wait and re-read, then execute | Poll cache.get until the entry appears or the TTL elapses; then execute uncached | ✓ |
| Execute uncached immediately | Run the node, do not store | |
| Wait for the holder's result only | Block until the entry appears | |

**User's choice:** Wait and re-read, then execute (recommended).

### Fail-open or fail-closed when the lock is unavailable?

| Option | Description | Selected |
|--------|-------------|----------|
| Fail open with in-process lock | Per-process lock keyed the same way; one warning per outage; node always executes | ✓ |
| Fail open, no lock at all | Skip the lock entirely | |
| Fail closed | Cached node does not execute; run errors | |

**User's choice:** Fail open with in-process lock (recommended).

### Lock TTL and fencing token

| Option | Description | Selected |
|--------|-------------|----------|
| TTL from config, token from Redis INCR | lock_ttl_secs (~60s); monotonic integer token; Lua delete-only-if-token-matches; token attached to cache.put | ✓ |
| TTL from node timeout | Derive TTL from the node's timeout | |
| Fixed TTL, random token | Hard-coded TTL, UUID token | |

**User's choice:** TTL from config, token from Redis INCR (recommended).

---

## Claude's Discretion

- Shape of the typed retry delay on `LlmError::RateLimitExceeded` (struct variant vs sibling value), subject to the X-10 register.
- Gate wait mechanics, jitter distribution, trace event shape for gated calls.
- Lua script layout and key namespace.
- Exact decorator nesting relative to pricing and fallback.
- Contract-test shape and default numeric values within the stated orders of magnitude.

## Deferred Ideas

- Proactive pacing from remaining/reset headers on successful responses.
- Per-tenant or per-API-key pacing (no requirement).
