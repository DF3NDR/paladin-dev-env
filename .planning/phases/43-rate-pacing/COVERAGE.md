# API Coverage — provider rate-limit response surface (OpenAI, Anthropic, generic HTTP) and Redis

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
>
> Phase 43 adds no new provider integration and no new endpoint call. It consumes the
> rate-limit **response surface** of the provider HTTP APIs the crate already integrates (status
> 429, `Retry-After`, provider rate-limit headers, quota error codes) and a fixed set of Redis
> commands behind four Lua scripts. The capability surface enumerated below is that response
> surface; header names are verified against the official pages in plan 43-12.

| capability | decision | reason |
|---|---|---|
| http-429-status (every adapter) | INTEGRATE | |
| retry-after-delta-seconds (every adapter) | INTEGRATE | |
| retry-after-http-date (IMF-fixdate, RFC 850, asctime) | INTEGRATE | |
| retry-after-ms (Azure OpenAI / openai-python extra) | INTEGRATE | |
| openai-x-ratelimit-limit-requests | INTEGRATE | |
| openai-x-ratelimit-limit-tokens | INTEGRATE | |
| openai-x-ratelimit-remaining-requests | INTEGRATE | |
| openai-x-ratelimit-remaining-tokens | INTEGRATE | |
| openai-x-ratelimit-reset-requests (duration string) | INTEGRATE | |
| openai-x-ratelimit-reset-tokens (duration string) | INTEGRATE | |
| openai-project-scoped-token-headers | OPT-OUT | exact names not verifiable from official docs in this environment (research A1); carried-only data that never gates, so a wrong name would only ever yield None |
| openai-insufficient_quota-error-code | INTEGRATE | |
| anthropic-retry-after | INTEGRATE | |
| anthropic-ratelimit-requests-limit/remaining/reset | INTEGRATE | |
| anthropic-ratelimit-tokens-limit/remaining/reset | INTEGRATE | |
| anthropic-ratelimit-input-tokens-limit/remaining/reset | INTEGRATE | |
| anthropic-ratelimit-output-tokens-limit/remaining/reset | INTEGRATE | |
| anthropic-priority-input/output-tokens-headers | OPT-OUT | Priority Tier only; the generic tokens dimensions already describe the binding limit and these never gate (D-04) |
| anthropic-fast-mode-headers | OPT-OUT | names not enumerated on the official rate-limits page; cannot be verified per PACE-01 |
| anthropic-enforced_spend_limit_reached-429 | INTEGRATE | |
| gemini-RetryInfo-retryDelay-body-field | OPT-OUT | body-level field, not a header; PACE-01 names OpenAI and Anthropic headers only; Gemini still honours a Retry-After header through the generic family |
| proactive-gating-from-remaining/reset-on-success | OPT-OUT | deferred by the operator (43-CONTEXT.md Deferred Ideas); the values are parsed and carried so a later phase can gate on them |
| redis-eval-script (six cadence and node-cache Lua scripts) | INTEGRATE | |
| redis-time (server clock inside scripts) | INTEGRATE | |
| redis-key-expiry (PEXPIRE / SET PX on every key) | INTEGRATE | |
| redis-incr-fencing-counter | INTEGRATE | |
| redis-pubsub/streams/cluster-specific commands | OPT-OUT | not needed: pacing state is a per-key hash read and written atomically by scripts on one primary (flagged assumption A1-43-07) |
</content>
</invoke>
