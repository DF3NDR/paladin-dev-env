# v0.11.1 — Dogfood findings log

Running log of issues found while using v0.11.0 for real: the maintainer's own runs, the web3sec
consumer's return, and anyone else exercising the published `0.11.0` crates, `paladin-cli`, or
`paladin-server`. One entry per finding. Append; never rewrite an earlier entry (add a dated note).

When a finding is promoted into the v0.11.1 roadmap it keeps its `DF-nn` id so the roadmap row, the fix
commit and this log stay cross-referenced.

## Entry format

```
### DF-nn — <one-line title>
- **Found:** <date> · **By:** <maintainer | web3sec | other> · **Surface:** <cli | server | library | docs | ci | k8s>
- **Version:** 0.11.0 (crate / tag / commit if different)
- **What happened:** <observed behavior, verbatim error text in a code block>
- **Expected:** <what the docs, the type, or common sense said would happen>
- **Repro:** <command or minimal code; config excerpt with secrets removed>
- **Severity:** S1 security/isolation · S2 wrong result or crash · S3 misleading docs/config · S4 hygiene
- **Related:** <WINDOWS row, review id, inventory row in 01-carried-gaps-inventory.md, or none>
- **Disposition:** open | in v0.11.1 | out (reason) | duplicate of DF-nn
```

## Surfaces worth deliberately exercising

These are the entry points a new user meets first; each is a place where a stale string or an unexpanded
placeholder turns into a confusing first run.

- `README.md` Quick Example and Getting Started → `cargo add` / `Cargo.toml` snippet → first `basic_paladin`
  example.
- `paladin-cli onboarding` and `paladin-cli setup-check` (their live tests are `#[ignore]`d: "Requires valid
  API key", so they are the least-exercised code in CI).
- `paladin-server` from `config.example.yml` with `http.auth.enabled: true` and an `api_keys[].key` taken
  verbatim from the example (see DF-01).
- `k8s/server/` manifests as shipped (configmap header claim, worker-deployment labels).
- `paladin-cli treasury …` spend queries by tenant / key / model, and the CLI-only aggregate view the HTTP
  surface lacks (audit G1).
- The `examples/` set against a seeded provider other than OpenAI, and against Ollama / OpenAI-compatible
  (never probed, no seeded rows).
- The Switching-providers page walked for a provider the maintainer actually has a key for.
- `docs/src/contributing/testing-guide.md` local coverage walkthrough on a Docker-capable machine (todo
  `2026-08-13-verify-local-coverage-reproduction`).

## Findings

Pre-seeded on 2026-10-27 from reading the tree, not from a run. Each needs a real reproduction before it is
promoted.

### DF-01 — Example HTTP auth keys are `${VAR}` placeholders the loader never expands
- **Found:** 2026-10-27 (by reading) · **By:** Claude · **Surface:** server, k8s, docs
- **Version:** 0.11.0
- **What happened:** `config.example.yml` `http.auth.api_keys[].key: "${PALADIN_API_KEY_CI}"` and
  `k8s/server/configmap.yaml` (lines 36, 40; header says "expanded by the server at load time") would load
  the literal placeholder string as the API key. The loader performs no `${VAR}` expansion; the behavior is
  pinned by `src/config/treasurer.rs::yaml_env_placeholder_is_not_expanded`.
- **Expected:** either expansion, or a boot-time refusal naming the unexpanded key.
- **Repro:** copy the example, set `http.auth.enabled: true`, start `paladin-server`, call with the literal
  `${PALADIN_API_KEY_CI}` as the bearer — it should *not* authenticate, and it will.
- **Severity:** S2 (and S1 if someone ships it)
- **Related:** WINDOWS row 62; inventory B1, C3
- **Disposition:** open → recommend **in v0.11.1**

### DF-02 — README advertises version 0.8.0 and a `0.5` install line
- **Found:** 2026-10-27 (by reading) · **By:** Claude · **Surface:** docs
- **Version:** 0.11.0
- **What happened:** `README.md:183` "Current version: **0.8.0**"; `README.md:108` `paladin-ai = "0.5"`.
  `docs/src/api-reference/crate-map.md:190` and `feature-flags.md:132,167,333` pin `"0.10.0"` (the yanked
  partial release); `docs/src/appendix/sentinel.md:66` pins `"0.5"`.
- **Expected:** `0.11.0` (or `0.11`) everywhere a user copies from.
- **Repro:** `grep -rnE 'paladin-ai = "|Current version' README.md docs/src`
- **Severity:** S3
- **Related:** inventory C1, C2; Phase 46 (docs currency) did not catch it, so a gate is the real fix
- **Disposition:** open → recommend **in v0.11.1** with a `check-version-strings` gate

### DF-03 — `ContentFilter::filter_content_list` panics on first call (`todo!()` in library code)
- **Found:** 2026-10-27 (by reading) · **By:** Claude · **Surface:** library (`paladin-content`)
- **Version:** 0.11.0
- **What happened:** `crates/paladin-content/src/services/content_filtering_service.rs:21-26` constructs
  every `ContentList` field with `todo!()`; any caller with a non-empty filtered list panics.
- **Expected:** a result or a typed error; never a panic from a published crate.
- **Repro:** call `filter_content_list` on a list with one matching item.
- **Severity:** S2
- **Related:** inventory B2, G3 (same unfinished subsystem)
- **Disposition:** open → recommend **in v0.11.1** (remove or return `Err`; do not finish the feature)

### DF-04 — Cross-tenant thread access through `/v1/threads/*`, run submission and fork
- **Found:** Phase 40 review (2026-09-29), never dispositioned · **By:** code review · **Surface:** server
- **Version:** 0.11.0
- **What happened:** see inventory A1–A3, A5. Listed here because a multi-tenant dogfood will hit it on the
  first `GET /v1/threads` with a second tenant's key.
- **Severity:** S1
- **Related:** WINDOWS row 58; 40-REVIEW WR-01, WR-02, IN-04
- **Disposition:** open → recommend **in v0.11.1**, first code phase

<!-- append new findings below this line -->

### DF-05 — `config.example.yml` claims `agent_runtime` is written out in full but omits `model_fallback.steps`
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-02) · **Surface:** docs
- **Version:** 0.11.0 (also HEAD 20808e4ae)
- **What happened:** the comment above `agent_runtime:` says every field "is written out explicitly and is
  IDENTICAL to what AgentRuntimeConfig::default() produces". The example writes
  `model_fallback: { enabled: false, providers: [] }`, but `ModelFallbackConfig` also has a `steps` field. The
  loaded `Settings` serialises a path the example never writes. Output of
  `example_sections_claimed_complete_write_every_field` (plan 56-02, lead L5) on the unmodified example:
  ```
  config.example.yml claims these sections are written out in full, but the loaded Settings serialises these paths the example does not write:
    agent_runtime.model_fallback.steps
  ```
- **Expected:** every field of the section written out, as the comment says, so an operator reading the example
  sees that `steps` exists.
- **Repro:** `cargo test -p paladin-ai --test config_example_roundtrip example_sections_claimed_complete_write_every_field`
- **Severity:** S3 (misleading config, D-04)
- **Related:** DOGF-03; lead L5
- **Disposition:** in v0.11.1 — fixed in plan 56-02: the example writes `steps: []` under `agent_runtime.model_fallback`; regression: `example_sections_claimed_complete_write_every_field`

### DF-06 — An unquoted treasurer price with 19 significant digits loses digits on load
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-02) · **Surface:** library
- **Version:** 0.11.0 (also HEAD 20808e4ae)
- **What happened:** `treasurer.pricing.<id>` axes are `String` fields documented as decimal strings. An
  operator who writes one unquoted gets it read as a number by the `config` crate and re-rendered. Credible
  prices keep their decimal value (`2.50` loads as `"2.5"`, `10.00` as `"10"`, `0.075` and `15` unchanged,
  pinned by `unquoted_credible_prices_keep_their_decimal_value`); a 19-significant-digit price does not:
  ```
  prompt: 1234567890.123456789   ->   loaded as "1234567890.1234567"
  ```
- **Expected:** the same decimal value, or a load error; never a silently different price.
- **Repro:** a config whose `treasurer.pricing.probe.prompt` is the unquoted literal above, loaded through
  `Settings::load_from_file` (one-off measurement in plan 56-02, not kept as a test).
- **Severity:** S4 (hygiene, D-04: no credible per-1M-token price has 19 significant digits, and the example
  already says prices are DECIMAL STRINGS and shows them quoted)
- **Related:** DOGF-03; quoted values are pinned byte-identical by `quoted_money_strings_arrive_byte_identical`
- **Disposition:** out (re-homed: .planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md)
