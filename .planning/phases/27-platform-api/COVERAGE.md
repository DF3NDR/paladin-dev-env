# API Coverage — Phase 27 (Platform API)

No external API integration: this phase **builds** Paladin's own HTTP API rather than consuming a
third party's, so there is no external capability surface to enumerate or subtract from.

The API-coverage detector fired on the phrase "Platform API" in the phase title (signal:
verb `(surface)` / noun `api`). Re-reading the phase scope confirms it is a false positive for this
gate's purpose. What the phase touches instead:

- **Its own surface.** Every endpoint added here is specified by PRD 06 §2 and enumerated in
  `27-CONTEXT.md`; `openapi.json` plus its committed-baseline drift test is the coverage record, and
  PLAT-FR-16 already requires that surface to be complete (auth, scopes, pagination on every route).
- **Redis** — a queue *backend* reached through the in-tree `redis` client behind `RunQueuePort`
  (D-06, D-08). The port's shared contract suite is its coverage record; there is no vendor
  capability list to opt out of.
- **SQLite / Postgres** — storage backends via the already-pinned `sqlx 0.8`, covered by the same
  contract-suite pattern.
- **Outbound webhooks** — deliveries to arbitrary user-configured URLs (D-40…D-43). The remote end
  is not a known service with a capability surface; it is whatever the caller registers, guarded by
  the SSRF table and the HMAC signature.
- **`openapi-generator-cli`** — a build-time code generator consumed by the `sdk-clients` CI job
  (D-49), not a runtime integration.

Recorded per the API-coverage checkpoint's reasoned-declaration path so the `verify:pre` seal gate
has an explicit decision rather than an absent matrix.
