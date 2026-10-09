# Phase 43: Provider rate-limit header evidence (PACE-01)

Per-name verification record for every provider header and error code the Cadence parser
(`crates/paladin-llm/src/rate_limit_headers.rs`) and the quota mapping
(`openai/adapter.rs`, `anthropic/adapter.rs`) rely on. Only official provider pages count
(threat T-43-46); search excerpts and third-party pages are never marked verified.

Fetch date: 2026-10-09. Method: `curl -sL` through the session proxy (TLS verification on).

Statuses: `VERIFIED (quote)` - fetched by the executor, verbatim quote recorded;
`BLOCKED (egress)` - the official page could not be fetched from the sandbox;
`MISMATCH` - the official page disagrees with the code.

## For the operator

The sandbox cannot reach `platform.openai.com`, `developers.openai.com`, `help.openai.com` or
`cookbook.openai.com` (`curl` returns no connection, HTTP code `000`, for every one of them), and
no Context7 CLI/MCP tool was available to serve the OpenAI guide. Please confirm, against
https://platform.openai.com/docs/guides/rate-limits (the "Rate limits in headers" table) and
https://platform.openai.com/docs/guides/error-codes, each of the following, or give the correction:

1. `x-ratelimit-limit-requests`
2. `x-ratelimit-limit-tokens`
3. `x-ratelimit-remaining-requests`
4. `x-ratelimit-remaining-tokens`
5. `x-ratelimit-reset-requests`
6. `x-ratelimit-reset-tokens`
7. The format of the two reset values: Go-style duration strings such as `1s`, `6m0s`, `6ms`
   (the parser `parse_go_duration` accepts exactly this form).
8. OpenAI `Retry-After` semantics (code assumes: seconds, optional, "minimum number of seconds to
   wait before retrying a temporary rate-limit error, when present").
9. `retry-after-ms` - expected NOT documented by OpenAI (research A3; the code treats it as an
   optional extra for the OpenAI-compatible family). Confirm "not documented" or give the
   official name.
10. A quota/billing 429 carries the error code `insufficient_quota`.

Every Anthropic row below is `VERIFIED (quote)`; nothing is required from the operator there.

## OpenAI

| Name | Official URL | Fetch date | Status | Quote |
|------|--------------|------------|--------|-------|
| `x-ratelimit-limit-requests` | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `x-ratelimit-limit-tokens` | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `x-ratelimit-remaining-requests` | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `x-ratelimit-remaining-tokens` | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `x-ratelimit-reset-requests` | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `x-ratelimit-reset-tokens` | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| reset duration format (`1s`, `6m0s`, `6ms`) | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `retry-after` (OpenAI semantics) | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `retry-after-ms` (expected not documented, A3) | https://platform.openai.com/docs/guides/rate-limits | 2026-10-09 | BLOCKED (egress) | - |
| `insufficient_quota` | https://platform.openai.com/docs/guides/error-codes | 2026-10-09 | BLOCKED (egress) | - |

Pages attempted and unreachable: `platform.openai.com/docs/guides/rate-limits`,
`developers.openai.com/api/docs/guides/rate-limits`, `platform.openai.com/docs/guides/error-codes`,
`platform.openai.com/docs/api-reference/introduction`, `help.openai.com` (429 article),
`cookbook.openai.com` (rate-limit example).

## Anthropic

Sources: https://platform.claude.com/docs/en/api/rate-limits ("Response headers" table and
"Reaching your spend cap") and https://platform.claude.com/docs/en/api/errors, both fetched
2026-10-09 (HTTP 200).

| Name | Official URL | Fetch date | Status | Quote |
|------|--------------|------------|--------|-------|
| `retry-after` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The number of seconds to wait until you can retry the request. Earlier retries will fail. Not sent with the spend-cap 429 (see Reaching your spend cap)." |
| `anthropic-ratelimit-requests-limit` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The maximum number of requests allowed within any rate limit period." |
| `anthropic-ratelimit-requests-remaining` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The number of requests remaining before being rate limited." |
| `anthropic-ratelimit-requests-reset` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The time when the request rate limit will be fully replenished, provided in RFC 3339 format." |
| `anthropic-ratelimit-tokens-limit` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The maximum number of tokens allowed within any rate limit period." |
| `anthropic-ratelimit-tokens-remaining` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The number of tokens remaining (rounded to the nearest thousand) before being rate limited." |
| `anthropic-ratelimit-tokens-reset` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The time when the token rate limit will be fully replenished, provided in RFC 3339 format." |
| `anthropic-ratelimit-input-tokens-limit` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The maximum number of input tokens allowed within any rate limit period." |
| `anthropic-ratelimit-input-tokens-remaining` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The number of input tokens remaining (rounded to the nearest thousand) before being rate limited." |
| `anthropic-ratelimit-input-tokens-reset` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The time when the input token rate limit will be fully replenished, provided in RFC 3339 format." |
| `anthropic-ratelimit-output-tokens-limit` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The maximum number of output tokens allowed within any rate limit period." |
| `anthropic-ratelimit-output-tokens-remaining` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The number of output tokens remaining (rounded to the nearest thousand) before being rate limited." |
| `anthropic-ratelimit-output-tokens-reset` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The time when the output token rate limit will be fully replenished, provided in RFC 3339 format." |
| `enforced_spend_limit_reached` | .../api/rate-limits | 2026-10-09 | VERIFIED (quote) | "The error type is rate_limit_error, the same as for a rate limit, but the response has no retry-after header. ... On the Messages API, error.details.error_code is enforced_spend_limit_reached. Use it to tell this response apart from a rate limit." |

Corroboration for the spend-cap 429 on https://platform.claude.com/docs/en/api/errors: "429 -
rate_limit_error: ... A tier spend-cap 429 has no retry-after header and keeps failing until
access resumes".

Notes (no code change required):

- The page also documents `anthropic-priority-{input,output}-tokens-{limit,remaining,reset}`
  (Priority Tier only) and `anthropic-workspace-id`. The parser does not read these; it only
  reads the twelve `anthropic-ratelimit-*` names above, so no mismatch.
- The page states `anthropic-ratelimit-tokens-*` "display the values for the most restrictive
  limit currently in effect", consistent with the rustdoc on `ANTHROPIC_RATELIMIT_PREFIX`.
