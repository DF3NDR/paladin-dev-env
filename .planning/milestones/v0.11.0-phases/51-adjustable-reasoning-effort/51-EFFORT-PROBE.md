# 51-EFFORT-PROBE: live reasoning-effort probe over the confirmed candidate set (Phase 51, REASON-03 and REASON-04, D-01 to D-04)

This file is the sole source of effort data for plans 51-11 and 51-12 (ADR-0061 seed rule unchanged); anything that is
not in it must be left out of the seeded rows. The header below is Claude's curation of the transcript; the transcript
follows the final `---` rule verbatim. The header quotes numbers and decides no seed value.

## Provenance

- Who ran it and where: Claude, as the plan 51-10 executor agent (model Sonnet 5.5, in an isolated git worktree of the
  operator's repository), ran `live_vendor_smoke --effort` at the owner's explicit direction ("run here."), inside the
  operator's own devcontainer, on the operator's own vendor keys. The owner did not themselves execute the harness.
  Seven of the nine providers had their credential variable present (OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok,
  Gemini; names only, as each run's own credential line shows); the Gemini key is paid tier, so no free-tier request
  limit applied.
- Nine runs are concatenated in the transcript, in this order. Each starts with its own `# Live vendor effort probe`
  heading, a UTC date line and a `commit:` line (all nine read `commit: 32ce9dd0`, the repository HEAD when the runs were
  made; the harness is plan 51-02's, unchanged). Each run reported exit status 0 and wrote nothing to stderr, as
  observed by the runner.

| run | date (UTC) | commit | providers PROBED | models probed | probe rows |
|---|---|---|---|---|---|
| 1 | 2026-10-22 14:18:09Z | 32ce9dd0 | OpenAI | 17 | 170 |
| 2 | 2026-10-22 14:21:13Z | 32ce9dd0 | Anthropic | 11 | 187 |
| 3 | 2026-10-22 14:25:25Z | 32ce9dd0 | DeepSeek | 2 | 24 |
| 4 | 2026-10-22 14:16:46Z | 32ce9dd0 | Kimi | 1 | 18 |
| 5 | 2026-10-22 14:25:53Z | 32ce9dd0 | Kimi | 1 | 18 |
| 6 | 2026-10-22 14:27:06Z | 32ce9dd0 | Qwen | 13 | 104 |
| 7 | 2026-10-22 14:46:52Z | 32ce9dd0 | Qwen | 1 | 8 |
| 8 | 2026-10-22 14:35:28Z | 32ce9dd0 | Grok | 5 | 45 |
| 9 | 2026-10-22 14:36:42Z | 32ce9dd0 | Gemini | 12 | 144 |

  Run order and why: the runs are numbered in transcript order; by their date lines they ran in the order 4, 1, 2, 3, 5,
  6, 8, 9, 7. Run 4 is the `kimi-k3`-first slice the plan requires (both wire forms, checked for rows with
  numbers and scanned before anything else was spent); runs 1, 2, 3, 5, 6, 8 and 9 are the remaining vendors one at a time
  in vendor order (the Kimi run probed only `kimi-k2.6`, since `kimi-k3` had just been probed in run 4 and was not paid
  for twice); run 7 re-probed `qwen3.7-max` alone (see "Retry" below).
- Retry: in run 6, `qwen3.7-max` row 2 (`enable_thinking` `off`) lost its answer to a client-side transport failure
  (verdict `error`, detail `error sending request`, no HTTP status). The same candidate was re-probed once, alone, in
  run 7, which measured that row cleanly. Both runs stay in the transcript. For `qwen3.7-max`, run 7's rows are read as
  the measurement of the lost row only; the other seven rows of runs 6 and 7 are two independent measurements of the same
  call and are both shown. The tables below use run 6's rows except for that one row (marked "run 7").
- Row count: 710 planned calls plus 8 for the retry make 718 probe rows; the table "Call and status counts" counts them.
- The raw transcript was never committed or pushed: it stayed an untracked local file until this scrub, and is deleted in
  the same commit that adds this file.
- Curated on 2026-10-22. The transcript is verbatim, concatenated run by run with one blank line between runs; no
  measured value was edited. The only credential-related text the harness prints is a header name (`Authorization`,
  `x-api-key`, `x-goog-api-key`, as a name, its value never printed) and the credential variable names; both are names,
  not values. Response bodies appear only inside the harness's bounded, redacted excerpts of non-2xx replies.
- Scrub check: the assembled file and each raw run were scanned with a scratch script (kept in the scratchpad, not
  committed) for everything the Rust `redact_secret_patterns` function recognises (the HTTP authorization-scheme token
  marker in both capitalisations, the vendor-prefixed secret-key marker, the AWS access-key-id marker, the key and
  token query-value markers at a word boundary, and three-segment base64url triples), plus the Google, xAI, GitHub and
  Slack key shapes, long mixed-case tokens, authorization-style header lines that carry a value, and the literal value of
  every credential variable set in this environment and of every file under the operator's `~/.config/paladin/` (values
  compared programmatically, never printed). The scanner was first shown to fire on a planted sample. It reported nothing
  on any run and nothing on the assembled file, so nothing was redacted or edited.
- The transcript part of this file is byte-identical to the local raw file: a comparison of the two, run immediately
  before the raw file was deleted, was empty.
- The verdict tables below were computed by a scratch script from the transcript rows (not by hand), using only the rules
  stated under "Verdict table".

## Owner decision

Recorded before any live call, from the owner's answers at plan 51-10's checkpoint, dated 2026-10-22. The owner's words,
verbatim:

- Option selected: `run-here` ("run here."). This selection is also the go-ahead to spend.
- Struck ids: none. Added ids: none. The owner confirmed the drafted candidate set: "that should be the ones that have
  been previously probed ... the models that are the currently active latest models available".
- D-02 treatment of an unprobed seeded `reasoning: false` entry: "D02 unmeasured" (`EffortSupport::Unmeasured`: every
  explicit effort on such a model refuses, naming the override, rather than the `NoReasoning` state D-02 is written with).

## Providers PROBED and SKIPPED

PROBED (7 of 9): OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini. SKIPPED (2 of 9): Ollama
(`SKIPPED (OLLAMA_BASE_URL not set)`: no local daemon is configured) and OpenAI-compatible
(`SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)`: no endpoint is configured). In each run the providers that were not part
of that run's `--models` flag print `NOT PROBED: no --models candidate for this provider`; that is the command shape (one
vendor per run), not a failure. No provider was skipped for an authentication failure, an outage or a rate limit.

### Provider status per run

| run | OpenAI | Anthropic | DeepSeek | Kimi | Qwen | Grok | Gemini | Ollama | OpenAI-compatible |
|---|---|---|---|---|---|---|---|---|---|
| 1 | PROBED (17) | no candidates | no candidates | no candidates | no candidates | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 2 | no candidates | PROBED (11) | no candidates | no candidates | no candidates | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 3 | no candidates | no candidates | PROBED (2) | no candidates | no candidates | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 4 | no candidates | no candidates | no candidates | PROBED (1) | no candidates | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 5 | no candidates | no candidates | no candidates | PROBED (1) | no candidates | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 6 | no candidates | no candidates | no candidates | no candidates | PROBED (13) | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 7 | no candidates | no candidates | no candidates | no candidates | PROBED (1) | no candidates | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 8 | no candidates | no candidates | no candidates | no candidates | no candidates | PROBED (5) | no candidates | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |
| 9 | no candidates | no candidates | no candidates | no candidates | no candidates | no candidates | PROBED (12) | SKIPPED (OLLAMA_BASE_URL not set) | SKIPPED (OPENAI_COMPATIBLE_API_KEY not set) |

Models probed per provider (counted from the transcript's `### <model>` tables; the 62 final ids, no model is a curated
default because the effort modes probe only `--models` ids):

| provider | status | models probed | probe rows |
|---|---|---|---|
| OpenAI | PROBED | 17 | 170 |
| Anthropic | PROBED | 11 | 187 |
| DeepSeek | PROBED | 2 | 24 |
| Kimi | PROBED | 2 | 36 |
| Qwen | PROBED | 13 | 112 |
| Grok | PROBED | 5 | 45 |
| Gemini | PROBED | 12 | 144 |
| Ollama | SKIPPED | 0 | 0 |
| OpenAI-compatible | SKIPPED | 0 | 0 |

Call and status counts (status of every probe row):

| provider | models | rows | 2xx | 400/422 | other status or error |
|---|---|---|---|---|---|
| OpenAI | 17 | 170 | 95 | 75 | 0 |
| Anthropic | 11 | 187 | 139 | 48 | 0 |
| DeepSeek | 2 | 24 | 20 | 4 | 0 |
| Kimi | 2 | 36 | 26 | 10 | 0 |
| Qwen | 13 | 112 | 72 | 39 | 1 |
| Grok | 5 | 45 | 29 | 16 | 0 |
| Gemini | 12 | 144 | 110 | 34 | 0 |
| **total** | 62 | 718 | 491 | 226 | 1 |

Rows with a status other than 2xx, 400 or 422 (run, model, row, form, value, status, verdict):

- Qwen: run 6, `qwen3.7-max`, row 2, `enable_thinking` `off`, status `-`, verdict `error`

### Candidate set

Drafted: 62 ids, in `51-EFFORT-PROBE-CANDIDATES.md` (the planner's D-02 draft: the rule rows, plus the research Pitfall 1
additions for Anthropic and OpenAI, plus a few Qwen and Gemini additions). Struck by the owner: none. Added by the owner:
none. Final set: 62 ids (the delta is 62 - 0 + 0 = 62), recorded in the candidates file under `## Final candidate set`
before the first call. Every one of the 62 ids was probed.

| vendor | drafted | struck | added | final | models probed |
|---|---|---|---|---|---|
| Anthropic | 11 | 0 | 0 | 11 | 11 |
| OpenAI | 17 | 0 | 0 | 17 | 17 |
| DeepSeek | 2 | 0 | 0 | 2 | 2 |
| Kimi | 2 | 0 | 0 | 2 | 2 |
| Qwen | 13 | 0 | 0 | 13 | 13 |
| Grok | 5 | 0 | 0 | 5 | 5 |
| Gemini | 12 | 0 | 0 | 12 | 12 |
| total | 62 | 0 | 0 | 62 | 62 |

Not candidates, not probed: `gpt-4o-mini`, `chat-latest`, `grok-4.20-non-reasoning`, the Gemini 2.5 ids and the
`kimi-k2.7-code*` ids (the planner's exclusions, none added back by the owner). Measured end to end for every one of the
62 ids means every one of the planned rows has a status in the transcript; the single exception is the one lost row above,
re-measured in run 7.

### web3sec models

The three models web3sec runs (CONTEXT Specific Ideas) were each probed end to end, every planned row answered with a
status: `claude-opus-4-8` (17 rows, Anthropic, run 2), `deepseek-v4-pro` (12 rows, DeepSeek, run 3) and `kimi-k3` (18
rows, Kimi, run 4). Their rows, from the generated per-model tables (the same numbers appear in the verdict section):

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `claude-opus-4-8` | 200, 0 | `output_config` | low=0 medium=0 high=0 xhigh=0 max=0 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-opus-4-8` | 200, 0 | `output_config+adaptive` | low=0 medium=0 high=0 xhigh=0 max=41 | - | - | - | - | - | - | - |
| `claude-opus-4-8` | 200, 0 | `thinking_object` | - | - | off: 200, rt=0 | auto: 200, rt=0 | - | - | - | - |
| `claude-opus-4-8` | 200, 0 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `deepseek-v4-pro` | 200, 35 | `top_level_effort` | minimal=37 low=25 medium=36 high=35 xhigh=35 max=39 | - | - | - | - | bogus-effort-51: 422, rt=- | high+temperature=0.5: 200, rt=35 | - |
| `deepseek-v4-pro` | 200, 35 | `thinking_object` | - | - | off: 200, rt=- | auto: 200, rt=110 | - | bogus-effort-51: 422, rt=- | - | - |

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `kimi-k3` | 200, 33 | `top_level_effort` | minimal=46 low=47 medium=48 high=35 xhigh=51 max=86 | - | - | - | - | bogus-effort-51: 200, rt=43 | - | - |
| `kimi-k3` | 200, 33 | `thinking_object` | low=40 high=48 max=116 | minimal(400), medium(400), xhigh(400) | off: 200, rt=- | auto: 200, rt=71 | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |

For `claude-opus-4-8` the two `thinking_budget` rows (1024 and 1023) are in the Anthropic per-model table as
`400`; `claude-opus-4-8` reports 0 reasoning tokens on every `output_config` row, and 41 on the paired `max` row.

### Kimi wire form

Research Pitfall 2: Kimi's wire form is decided by valid-value effect, not by invalid-value rejection alone. This section
only quotes; it decides nothing. The `top_level_effort` form is a top-level `reasoning_effort` field; the
`thinking_object` form is `thinking.effort` inside the `thinking` object (`thinking.type` is `enabled`). Valid `low` and
`max`, and the invalid value `bogus-effort-51`, through both forms, for both Kimi models:

`kimi-k3` (rows copied unchanged from the transcript below):

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 4 | top_level_effort | low | 200 | stop | 47 | 63 | 4813 | accepted | - |
| 5 | thinking_object | low | 200 | stop | 40 | 56 | 4339 | accepted | - |
| 12 | top_level_effort | max | 200 | stop | 86 | 102 | 5263 | accepted | - |
| 13 | thinking_object | max | 200 | stop | 116 | 132 | 5856 | accepted | - |
| 16 | top_level_effort | bogus-effort-51 | 200 | stop | 43 | 59 | 3952 | accepted | - |
| 17 | thinking_object | bogus-effort-51 | 400 | - | - | - | 306 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: bogus-effort-51","type":"invalid_request_error"}} |

`kimi-k2.6` (rows copied unchanged from the transcript below):

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 4 | top_level_effort | low | 200 | stop | 128 | 131 | 3393 | accepted | - |
| 5 | thinking_object | low | 200 | stop | 141 | 144 | 3460 | accepted | - |
| 12 | top_level_effort | max | 200 | stop | 327 | 330 | 6624 | accepted | - |
| 13 | thinking_object | max | 200 | stop | 141 | 144 | 3476 | accepted | - |
| 16 | top_level_effort | bogus-effort-51 | 200 | stop | 212 | 215 | 5045 | accepted | - |
| 17 | thinking_object | bogus-effort-51 | 400 | - | - | - | 256 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: bogus-effort-51","type":"invalid_request_error"}} |

Computed from those rows and the full transcript, for the record: through `top_level_effort` the invalid value drew
status 200 on both models (reasoning tokens 43 on `kimi-k3`, 212 on `kimi-k2.6`); through `thinking_object` it drew 400
on both. The `thinking_object` form accepted `low`, `high` and `max` and rejected `minimal`, `medium` and `xhigh` with
400 on both models. `kimi-k3`: `low`/`max` reasoning tokens were 47/86 through `top_level_effort` and 40/116 through
`thinking_object`. `kimi-k2.6`: 128/327 and 141/141.

## Verdict table

Rules (applied mechanically; no seed value is decided here):

- A row's verdict is the harness's: accepted is a 2xx; rejected is only HTTP 400 or 422; anything else (401, 403, 429,
  5xx, 3xx, transport) is error. The one error row in the transcript is the retried `qwen3.7-max` row above.
- A form is `parsed` when its invalid value drew a 4xx (400 or 422). "yes" means every probed model's invalid row drew a
  4xx; "partial (n of N)" means only some did; "no" means none did; "n/a" means the form has no invalid row of its own
  (Anthropic's `output_config+adaptive`, `thinking_object` and `thinking_budget` share the `output_config` invalid row's
  field or have only budget rows; Qwen's `enable_thinking` toggle has none).
- A form shows `effect` when the reasoning tokens of the highest accepted level exceed those of the lowest accepted level
  on at least one model. The levels are ordered minimal, low, medium, high, xhigh, max. Both rows must report a
  `reasoning tokens` integer; a `-` means the response carried no count and is not treated as 0, so that model is not
  comparable. The count in the "highest above lowest" column is `models where it held of models comparable`. One sample
  per row: these are single calls, not averages.
- `confirmed` (D-04) = parsed ("yes", or "partial" shown as such) and effect "yes". A form that is not confirmed is
  recorded as is.
- Detector: the top-level `reasoning_effort` field with the invalid value. On Kimi, OpenAI, DeepSeek and Grok the
  `top_level_effort` invalid row is the detector; on Qwen the separate `detector` form is. Drawn 4xx = the field is
  rejected (parsed); a 2xx = silently accepted.
- The sampling row's status, the off / auto / budget outcomes, the Anthropic paired-versus-alone thinking tokens and the
  Gemini spelling rows are listed per model below, as numbers and statuses only.

### Per vendor and form

| vendor | form | models with the form | invalid value drew 400/422 | invalid value accepted (2xx) | field parsed | models with levels | models where highest accepted level reports more reasoning tokens than the lowest (of those comparable) | effect | confirmed (D-04) |
|---|---|---|---|---|---|---|---|---|---|
| OpenAI | `top_level_effort` | 17 | 17 of 17 | 0 of 17 | yes | 17 | 14 of 17 | yes | yes |
| Anthropic | `output_config` | 11 | 11 of 11 | 0 of 11 | yes | 11 | 7 of 9 | yes | yes |
| Anthropic | `output_config+adaptive` | 11 | 0 of 0 | 0 of 0 | n/a (no invalid row of its own) | 11 | 10 of 11 | yes | n/a |
| Anthropic | `thinking_object` | 11 | 0 of 0 | 0 of 0 | n/a (no invalid row of its own) | 0 | 0 of 0 | n/a (no level rows) | n/a |
| Anthropic | `thinking_budget` | 11 | 0 of 0 | 0 of 0 | n/a (no invalid row of its own) | 0 | 0 of 0 | n/a (no level rows) | n/a |
| DeepSeek | `top_level_effort` | 2 | 2 of 2 | 0 of 2 | yes | 2 | 2 of 2 | yes | yes |
| DeepSeek | `thinking_object` | 2 | 2 of 2 | 0 of 2 | yes | 0 | 0 of 0 | n/a (no level rows) | n/a |
| Kimi | `top_level_effort` | 2 | 0 of 2 | 2 of 2 | no | 2 | 2 of 2 | yes | no |
| Kimi | `thinking_object` | 2 | 2 of 2 | 0 of 2 | yes | 2 | 1 of 2 | yes | yes |
| Qwen | `enable_thinking` | 13 | 0 of 0 | 0 of 0 | n/a (no invalid row of its own) | 0 | 0 of 0 | n/a (no level rows) | n/a |
| Qwen | `thinking_budget` | 13 | 12 of 13 | 1 of 13 | partial (12 of 13) | 0 | 0 of 0 | n/a (no level rows) | n/a |
| Qwen | `detector` | 13 | 9 of 13 | 4 of 13 | partial (9 of 13) | 0 | 0 of 0 | n/a (no level rows) | n/a |
| Grok | `top_level_effort` | 5 | 5 of 5 | 0 of 5 | yes | 4 | 2 of 4 | yes | yes |
| Gemini | `thinking_level` | 12 | 12 of 12 | 0 of 12 | yes | 12 | 5 of 5 | yes | yes |
| Gemini | `thinking_budget` | 12 | 12 of 12 | 0 of 12 | yes | 0 | 0 of 0 | n/a (no level rows) | n/a |

### OpenAI per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `gpt-5.4-nano` | 200, 0 | `top_level_effort` | low=0 medium=8 high=6 xhigh=13 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-6.1-sol` | 200, 5 | `top_level_effort` | low=6 medium=8 high=14 xhigh=11 | minimal(400), max(400) | none: 400, rt=- | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-6-sol` | 200, 10 | `top_level_effort` | low=7 medium=8 high=14 xhigh=14 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-6-luna` | 200, 14 | `top_level_effort` | low=8 medium=13 high=11 xhigh=8 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-6-astra` | 200, 7 | `top_level_effort` | low=4 medium=7 high=11 xhigh=14 | minimal(400), max(400) | none: 400, rt=- | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.6-sol` | 200, 0 | `top_level_effort` | low=0 medium=0 high=0 xhigh=0 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.6-luna` | 200, 13 | `top_level_effort` | low=13 medium=9 high=13 xhigh=14 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.6-terra` | 200, 9 | `top_level_effort` | low=9 medium=13 high=12 xhigh=13 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.5` | 200, 13 | `top_level_effort` | low=13 medium=14 high=20 xhigh=41 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.4` | 200, 0 | `top_level_effort` | low=4 medium=12 high=15 xhigh=14 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.4-mini` | 200, 0 | `top_level_effort` | low=12 medium=13 high=18 xhigh=50 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.2` | 200, 0 | `top_level_effort` | low=6 medium=12 high=26 xhigh=14 | minimal(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5.1` | 200, 0 | `top_level_effort` | low=0 medium=10 high=112 | minimal(400), xhigh(400), max(400) | none: 200, rt=0 | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5` | 200, 64 | `top_level_effort` | minimal=0 low=0 medium=384 high=128 | xhigh(400), max(400) | none: 400, rt=- | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5-mini` | 200, 0 | `top_level_effort` | minimal=0 low=0 medium=64 high=64 | xhigh(400), max(400) | none: 400, rt=- | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `gpt-5-nano` | 200, 128 | `top_level_effort` | minimal=0 low=64 medium=64 high=448 | xhigh(400), max(400) | none: 400, rt=- | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `o3` | 200, 0 | `top_level_effort` | low=0 medium=64 high=0 xhigh=0 | minimal(400), max(400) | none: 400, rt=- | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |

### Anthropic per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `claude-haiku-5-5` | 200, 116 | `output_config` | low=101 medium=62 high=102 xhigh=92 max=137 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-haiku-5-5` | 200, 116 | `output_config+adaptive` | low=86 medium=108 high=60 xhigh=71 max=227 | - | - | - | - | - | - | - |
| `claude-haiku-5-5` | 200, 116 | `thinking_object` | - | - | off: 200, rt=0 | auto: 200, rt=95 | - | - | - | - |
| `claude-haiku-5-5` | 200, 116 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-opus-4-8` | 200, 0 | `output_config` | low=0 medium=0 high=0 xhigh=0 max=0 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-opus-4-8` | 200, 0 | `output_config+adaptive` | low=0 medium=0 high=0 xhigh=0 max=41 | - | - | - | - | - | - | - |
| `claude-opus-4-8` | 200, 0 | `thinking_object` | - | - | off: 200, rt=0 | auto: 200, rt=0 | - | - | - | - |
| `claude-opus-4-8` | 200, 0 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-fable-5-1` | 200, 0 | `output_config` | low=0 medium=0 high=0 xhigh=60 max=36 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-fable-5-1` | 200, 0 | `output_config+adaptive` | low=0 medium=0 high=68 xhigh=123 max=36 | - | - | - | - | - | - | - |
| `claude-fable-5-1` | 200, 0 | `thinking_object` | - | - | off: 400, rt=- | auto: 200, rt=30 | - | - | - | - |
| `claude-fable-5-1` | 200, 0 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-fable-5` | 200, 19 | `output_config` | low=7 medium=7 high=11 xhigh=19 max=144 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-fable-5` | 200, 19 | `output_config+adaptive` | low=7 medium=11 high=11 xhigh=19 max=149 | - | - | - | - | - | - | - |
| `claude-fable-5` | 200, 19 | `thinking_object` | - | - | off: 400, rt=- | auto: 200, rt=11 | - | - | - | - |
| `claude-fable-5` | 200, 19 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-opus-5-5` | 200, 10 | `output_config` | low=8 medium=8 high=10 xhigh=10 max=106 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-opus-5-5` | 200, 10 | `output_config+adaptive` | low=170 medium=8 high=428 xhigh=10 max=106 | - | - | - | - | - | - | - |
| `claude-opus-5-5` | 200, 10 | `thinking_object` | - | - | off: 400, rt=- | auto: 200, rt=8 | - | - | - | - |
| `claude-opus-5-5` | 200, 10 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-opus-5` | 200, 12 | `output_config` | low=11 medium=12 high=12 xhigh=12 max=63 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-opus-5` | 200, 12 | `output_config+adaptive` | low=11 medium=12 high=12 xhigh=112 max=129 | - | - | - | - | - | - | - |
| `claude-opus-5` | 200, 12 | `thinking_object` | - | - | off: 200, rt=0 | auto: 200, rt=12 | - | - | - | - |
| `claude-opus-5` | 200, 12 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-sonnet-5-5` | 200, 0 | `output_config` | low=0 medium=0 high=0 xhigh=29 max=65 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-sonnet-5-5` | 200, 0 | `output_config+adaptive` | low=0 medium=0 high=0 xhigh=17 max=66 | - | - | - | - | - | - | - |
| `claude-sonnet-5-5` | 200, 0 | `thinking_object` | - | - | off: 400, rt=- | auto: 200, rt=0 | - | - | - | - |
| `claude-sonnet-5-5` | 200, 0 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-sonnet-5` | 200, 35 | `output_config` | low=0 medium=0 high=0 xhigh=35 max=164 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-sonnet-5` | 200, 35 | `output_config+adaptive` | low=0 medium=0 high=44 xhigh=47 max=141 | - | - | - | - | - | - | - |
| `claude-sonnet-5` | 200, 35 | `thinking_object` | - | - | off: 200, rt=0 | auto: 200, rt=0 | - | - | - | - |
| `claude-sonnet-5` | 200, 35 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-opus-4-7` | 200, 0 | `output_config` | low=0 medium=0 high=0 xhigh=0 max=0 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `claude-opus-4-7` | 200, 0 | `output_config+adaptive` | low=0 medium=0 high=0 xhigh=0 max=50 | - | - | - | - | - | - | - |
| `claude-opus-4-7` | 200, 0 | `thinking_object` | - | - | off: 200, rt=0 | auto: 200, rt=0 | - | - | - | - |
| `claude-opus-4-7` | 200, 0 | `thinking_budget` | - | - | - | - | 1024: 400, rt=-; 1023: 400, rt=- | - | - | - |
| `claude-opus-4-6` | 200, - | `output_config` | low=- medium=- high=- max=- | xhigh(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=- | - |
| `claude-opus-4-6` | 200, - | `output_config+adaptive` | low=0 medium=0 high=46 max=48 | xhigh(400) | - | - | - | - | - | - |
| `claude-opus-4-6` | 200, - | `thinking_object` | - | - | off: 200, rt=- | auto: 200, rt=46 | - | - | - | - |
| `claude-opus-4-6` | 200, - | `thinking_budget` | - | - | - | - | 1024: 200, rt=39; 1023: 400, rt=- | - | - | - |
| `claude-sonnet-4-6` | 200, - | `output_config` | low=- medium=- high=- max=- | xhigh(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=- | - |
| `claude-sonnet-4-6` | 200, - | `output_config+adaptive` | low=14 medium=46 high=46 max=48 | xhigh(400) | - | - | - | - | - | - |
| `claude-sonnet-4-6` | 200, - | `thinking_object` | - | - | off: 200, rt=- | auto: 200, rt=46 | - | - | - | - |
| `claude-sonnet-4-6` | 200, - | `thinking_budget` | - | - | - | - | 1024: 200, rt=39; 1023: 400, rt=- | - | - | - |

### Anthropic: thinking tokens with `output_config` alone versus paired with adaptive thinking

| model | alone: lowest accepted level = tokens | alone: highest accepted level = tokens | paired: lowest accepted level = tokens | paired: highest accepted level = tokens |
|---|---|---|---|---|
| `claude-haiku-5-5` | low = 101 | max = 137 | low = 86 | max = 227 |
| `claude-opus-4-8` | low = 0 | max = 0 | low = 0 | max = 41 |
| `claude-fable-5-1` | low = 0 | max = 36 | low = 0 | max = 36 |
| `claude-fable-5` | low = 7 | max = 144 | low = 7 | max = 149 |
| `claude-opus-5-5` | low = 8 | max = 106 | low = 170 | max = 106 |
| `claude-opus-5` | low = 11 | max = 63 | low = 11 | max = 129 |
| `claude-sonnet-5-5` | low = 0 | max = 65 | low = 0 | max = 66 |
| `claude-sonnet-5` | low = 0 | max = 164 | low = 0 | max = 141 |
| `claude-opus-4-7` | low = 0 | max = 0 | low = 0 | max = 50 |
| `claude-opus-4-6` | low = - | max = - | low = 0 | max = 48 |
| `claude-sonnet-4-6` | low = - | max = - | low = 14 | max = 48 |

### DeepSeek per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `deepseek-flash` | 200, 20 | `top_level_effort` | minimal=20 low=24 medium=19 high=17 xhigh=18 max=22 | - | - | - | - | bogus-effort-51: 422, rt=- | high+temperature=0.5: 200, rt=19 | - |
| `deepseek-flash` | 200, 20 | `thinking_object` | - | - | off: 200, rt=- | auto: 200, rt=17 | - | bogus-effort-51: 422, rt=- | - | - |
| `deepseek-v4-pro` | 200, 35 | `top_level_effort` | minimal=37 low=25 medium=36 high=35 xhigh=35 max=39 | - | - | - | - | bogus-effort-51: 422, rt=- | high+temperature=0.5: 200, rt=35 | - |
| `deepseek-v4-pro` | 200, 35 | `thinking_object` | - | - | off: 200, rt=- | auto: 200, rt=110 | - | bogus-effort-51: 422, rt=- | - | - |

### Kimi per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `kimi-k3` | 200, 33 | `top_level_effort` | minimal=46 low=47 medium=48 high=35 xhigh=51 max=86 | - | - | - | - | bogus-effort-51: 200, rt=43 | - | - |
| `kimi-k3` | 200, 33 | `thinking_object` | low=40 high=48 max=116 | minimal(400), medium(400), xhigh(400) | off: 200, rt=- | auto: 200, rt=71 | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `kimi-k2.6` | 200, 164 | `top_level_effort` | minimal=195 low=128 medium=416 high=378 xhigh=472 max=327 | - | - | - | - | bogus-effort-51: 200, rt=212 | - | - |
| `kimi-k2.6` | 200, 164 | `thinking_object` | low=141 high=620 max=141 | minimal(400), medium(400), xhigh(400) | off: 200, rt=1 | auto: 200, rt=140 | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |

### Qwen per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `qwen3.8-flash` | 200, 53 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=50 | - | - | auto+temperature=0.5: 200, rt=51 | - |
| `qwen3.8-flash` | 200, 53 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.8-flash` | 200, 53 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.8-max` | 200, 53 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=47 | - | - | auto+temperature=0.5: 200, rt=48 | - |
| `qwen3.8-max` | 200, 53 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.8-max` | 200, 53 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.7-plus` | 200, 186 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=146 | - | - | auto+temperature=0.5: 200, rt=792 | - |
| `qwen3.7-plus` | 200, 186 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.7-plus` | 200, 186 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.7-max` | 200, 200 | `enable_thinking` | - | - | off: 200, rt=- (run 7) | auto: 200, rt=247 | - | - | auto+temperature=0.5: 200, rt=540 | - |
| `qwen3.7-max` | 200, 200 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.7-max` | 200, 200 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.7-flash` | 200, 354 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=823 | - | - | auto+temperature=0.5: 200, rt=516 | - |
| `qwen3.7-flash` | 200, 354 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.7-flash` | 200, 354 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.6-plus` | 200, 1981 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=2645 | - | - | auto+temperature=0.5: 200, rt=992 | - |
| `qwen3.6-plus` | 200, 1981 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.6-plus` | 200, 1981 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.6-flash` | 200, 290 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=318 | - | - | auto+temperature=0.5: 200, rt=335 | - |
| `qwen3.6-flash` | 200, 290 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.6-flash` | 200, 290 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.5-plus` | 200, 4903 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=4699 | - | - | auto+temperature=0.5: 200, rt=1652 | - |
| `qwen3.5-plus` | 200, 4903 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.5-plus` | 200, 4903 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen3.5-flash` | 200, 3422 | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=462 | - | - | auto+temperature=0.5: 200, rt=1948 | - |
| `qwen3.5-flash` | 200, 3422 | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen3.5-flash` | 200, 3422 | `detector` | - | - | - | - | - | bogus-effort-51: 400, rt=- | - | - |
| `qwen-plus` | 200, - | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=1047 | - | - | auto+temperature=0.5: 200, rt=1010 | - |
| `qwen-plus` | 200, - | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen-plus` | 200, - | `detector` | - | - | - | - | - | bogus-effort-51: 200, rt=- | - | - |
| `qwen-max` | 200, - | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=- | - | - | auto+temperature=0.5: 200, rt=- | - |
| `qwen-max` | 200, - | `thinking_budget` | - | - | - | - | 1: 200, rt=-; 0: 200, rt=- | -1: 200, rt=- | - | - |
| `qwen-max` | 200, - | `detector` | - | - | - | - | - | bogus-effort-51: 200, rt=- | - | - |
| `qwen-flash` | 200, - | `enable_thinking` | - | - | off: 200, rt=- | auto: 200, rt=336 | - | - | auto+temperature=0.5: 200, rt=269 | - |
| `qwen-flash` | 200, - | `thinking_budget` | - | - | - | - | 1: 200, rt=1; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen-flash` | 200, - | `detector` | - | - | - | - | - | bogus-effort-51: 200, rt=- | - | - |
| `qwen-plus-latest` | 200, - | `enable_thinking` | - | - | off: 200, rt=- | auto: 400, rt=- | - | - | auto+temperature=0.5: 400, rt=- | - |
| `qwen-plus-latest` | 200, - | `thinking_budget` | - | - | - | - | 1: 400, rt=-; 0: 400, rt=- | -1: 400, rt=- | - | - |
| `qwen-plus-latest` | 200, - | `detector` | - | - | - | - | - | bogus-effort-51: 200, rt=- | - | - |

### Grok per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `grok-4.3` | 200, 327 | `top_level_effort` | minimal=353 low=270 medium=877 high=426 xhigh=245 | max(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=230 | - |
| `grok-4.6` | 200, 189 | `top_level_effort` | minimal=145 low=154 medium=126 high=296 xhigh=267 | max(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=263 | - |
| `grok-4.5` | 200, 130 | `top_level_effort` | minimal=133 low=128 medium=132 high=126 xhigh=120 | max(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=116 | - |
| `grok-4.20-reasoning` | 200, 351 | `top_level_effort` | - | minimal(400), low(400), medium(400), high(400), xhigh(400), max(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 400, rt=- | - |
| `grok-4.7` | 200, 93 | `top_level_effort` | minimal=96 low=91 medium=84 high=105 xhigh=99 | max(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=77 | - |

### Gemini per model

| model | default (status, reasoning tokens) | form | accepted levels (reasoning tokens) | rejected levels | off | auto | budget rows | invalid row | sampling row | spelling row |
|---|---|---|---|---|---|---|---|---|---|---|
| `gemini-4-argon` | 200, 34 | `thinking_level` | low=- medium=227 high=415 | minimal(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=267 | HIGH: 200, rt=293 |
| `gemini-4-argon` | 200, 34 | `thinking_budget` | - | - | off: 200, rt=- | - | 128: 200, rt=-; 1: 200, rt=- | -5: 400, rt=- | - | - |
| `gemini-3.8-flash` | 200, 245 | `thinking_level` | low=34 medium=235 high=232 | minimal(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=187 | HIGH: 200, rt=350 |
| `gemini-3.8-flash` | 200, 245 | `thinking_budget` | - | - | off: 200, rt=206 | - | 128: 200, rt=36; 1: 200, rt=48 | -5: 400, rt=- | - | - |
| `gemini-3.7-flash` | 200, 243 | `thinking_level` | low=60 medium=200 high=324 | minimal(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=270 | HIGH: 200, rt=180 |
| `gemini-3.7-flash` | 200, 243 | `thinking_budget` | - | - | off: 200, rt=48 | - | 128: 200, rt=103; 1: 200, rt=34 | -5: 400, rt=- | - | - |
| `gemini-3.1-pro-preview` | 200, 424 | `thinking_level` | low=322 medium=981 high=498 | minimal(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=665 | HIGH: 200, rt=446 |
| `gemini-3.1-pro-preview` | 200, 424 | `thinking_budget` | - | - | off: 400, rt=- | - | 128: 200, rt=430; 1: 200, rt=595 | -5: 400, rt=- | - | - |
| `gemini-pro-latest` | 200, 399 | `thinking_level` | low=355 medium=521 high=406 | minimal(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=510 | HIGH: 200, rt=571 |
| `gemini-pro-latest` | 200, 399 | `thinking_budget` | - | - | off: 400, rt=- | - | 128: 200, rt=339; 1: 200, rt=517 | -5: 400, rt=- | - | - |
| `gemini-flash-latest` | 200, 181 | `thinking_level` | low=188 medium=198 high=259 | minimal(400) | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=230 | HIGH: 200, rt=245 |
| `gemini-flash-latest` | 200, 181 | `thinking_budget` | - | - | off: 200, rt=109 | - | 128: 200, rt=36; 1: 200, rt=37 | -5: 400, rt=- | - | - |
| `gemini-flash-lite-latest` | 200, - | `thinking_level` | minimal=- low=163 medium=223 high=217 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=364 | HIGH: 200, rt=228 |
| `gemini-flash-lite-latest` | 200, - | `thinking_budget` | - | - | off: 400, rt=- | - | 128: 200, rt=163; 1: 200, rt=161 | -5: 400, rt=- | - | - |
| `gemini-3.1-flash-lite` | 200, - | `thinking_level` | minimal=- low=143 medium=280 high=370 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=676 | HIGH: 200, rt=361 |
| `gemini-3.1-flash-lite` | 200, - | `thinking_budget` | - | - | off: 200, rt=- | - | 128: 200, rt=-; 1: 200, rt=- | -5: 400, rt=- | - | - |
| `gemini-3-flash-preview` | 200, 278 | `thinking_level` | minimal=- low=236 medium=315 high=288 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=280 | HIGH: 200, rt=244 |
| `gemini-3-flash-preview` | 200, 278 | `thinking_budget` | - | - | off: 200, rt=- | - | 128: 200, rt=226; 1: 200, rt=- | -5: 400, rt=- | - | - |
| `gemini-3.5-flash-lite` | 200, - | `thinking_level` | minimal=- low=75 medium=209 high=328 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=304 | HIGH: 200, rt=440 |
| `gemini-3.5-flash-lite` | 200, - | `thinking_budget` | - | - | off: 400, rt=- | - | 128: 200, rt=80; 1: 200, rt=162 | -5: 400, rt=- | - | - |
| `gemini-3.6-flash` | 200, 165 | `thinking_level` | minimal=- low=155 medium=286 high=181 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=172 | HIGH: 200, rt=206 |
| `gemini-3.6-flash` | 200, 165 | `thinking_budget` | - | - | off: 200, rt=- | - | 128: 200, rt=186; 1: 200, rt=- | -5: 400, rt=- | - | - |
| `gemini-3.5-flash` | 200, 234 | `thinking_level` | minimal=- low=181 medium=167 high=207 | - | - | - | - | bogus-effort-51: 400, rt=- | high+temperature=0.5: 200, rt=218 | HIGH: 200, rt=180 |
| `gemini-3.5-flash` | 200, 234 | `thinking_budget` | - | - | off: 200, rt=- | - | 128: 200, rt=155; 1: 200, rt=- | -5: 400, rt=- | - | - |

In the per-model tables, an accepted level is written `level=reasoning tokens` and a `-` means the response carried no
reasoning-token count; a rejected level is written `level(status)`. `rt` is `reasoning tokens`. The Gemini `spelling row`
is the uppercase `HIGH` thinking level; it is not counted among the levels. The sampling row is `high` (or `auto` for
Qwen) together with `temperature=0.5`. Rows of `qwen3.7-max` marked "run 7" are the retry's.

## Vendor-documented bounds and coercions (docs, not observed)

DOCS, not observed by the probe. Each entry was fetched on 2026-10-22 (the access date) from the vendor's own page and
read for the sentence quoted. Nothing here is a measurement. Where the transcript's own rows speak to the same point,
they are cited by row and the two are not reconciled.

- Qwen `thinking_budget` upper bound. URL: https://www.alibabacloud.com/help/en/model-studio/deep-thinking, accessed
  2026-10-22. Quote: "thinking_budget Parameter for the maximum number of Tokens supported in the Thinking Process.
  Applicable to Qwen3.8, Qwen3.7, Qwen3.6, Qwen3.5, Qwen3-VL, Qwen3, GLM, and Kimi series models." And, for the console's
  Deep Think debugging panel: "Value range 1~32768, default value 4000." The page states the 32768 bound for the console
  panel; it names no per-model API bound. In the transcript, the `qwen3.7-max` rows 5 and 6 (budget `0` and `-1`) were
  rejected with a body that names a bound of 262144; that is an observed rejection text, and it differs from the page's
  figure. This plan does not reconcile them.
- Anthropic `budget_tokens`. URL: https://platform.claude.com/docs/en/build-with-claude/extended-thinking, accessed
  2026-10-22. Quote: "budget_tokens must satisfy these constraints: Minimum of 1,024 tokens. The API rejects smaller
  values. Less than max_tokens." The page documents no other fixed upper bound. Quote on availability: "Extended thinking
  (thinking.type: "enabled" with budget_tokens) is deprecated on the Claude 4.6 models (requests using it still succeed).
  Claude 4.7 and later models do not support it and reject requests that use it, returning a 400 error." The transcript's
  `thinking_budget` rows are in the Anthropic per-model table.
- Gemini `thinkingBudget`: not applicable; no Gemini 2.5 id (the documented `thinkingBudget` generation) is in the final
  candidate set, so no bound was fetched. The Gemini `thinking_budget` rows in the transcript are the probe's own.
- DeepSeek silent coercions. URL: https://api-docs.deepseek.com/guides/thinking_mode, accessed 2026-10-22. Quote: "The
  mapping between the effort set by the user and the model's actual reasoning effort is as follows: Requested effort
  Actual mapped effort minimal low low low medium high high high xhigh high max max ultra max", that is minimal to low,
  low to low, medium to high, high to high, xhigh to high, max to max, ultra to max. The page also states that
  "Thinking mode is enabled by default, with the default effort being high".
- xAI silent coercion. URL: https://docs.x.ai/docs/guides/reasoning, accessed 2026-10-22. Quote: "xhigh" is available on
  grok-4.6 and later. On models that do not support it, such as grok-4.5, requests with "xhigh" are treated as "high".
  The same page lists the levels per model: grok-4.7 (low, medium, high default, xhigh), grok-4.6 (low, medium, high
  default, xhigh) and grok-4.5 (low, medium, high default), "cannot be disabled". The page names neither `grok-4.3` nor
  `grok-4.20-reasoning`. The transcript's Grok rows are in the per-model table above (for example `grok-4.5` accepted
  `xhigh` with 120 reasoning tokens, and `grok-4.20-reasoning` rejected every level with a 400).

---

# Live vendor effort probe

- date (UTC): 2026-10-22 14:18:09Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

- base URL: `https://api.openai.com/v1`
- chat endpoint: `POST https://api.openai.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_completion_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### gpt-5.4-nano

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 4 | 2895 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 1484 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 659 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 0 | 4 | 765 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 8 | 18 | 1169 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 6 | 16 | 1023 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 13 | 23 | 1332 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 612 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 262 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 602 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=0 max=-
- detector: field parsed (rejected)

### gpt-6.1-sol

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 5 | 15 | 4340 | accepted | - |
| 2 | top_level_effort | none | 400 | - | - | - | 586 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 3 | top_level_effort | minimal | 400 | - | - | - | 120 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 6 | 16 | 2383 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 8 | 18 | 2142 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 14 | 24 | 2361 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 11 | 21 | 1914 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 138 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 191 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 126 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=6 max=-
- detector: field parsed (rejected)

### gpt-6-sol

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 10 | 20 | 1203 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 1389 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 124 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 7 | 17 | 1105 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 8 | 18 | 1025 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 14 | 24 | 1025 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 14 | 24 | 1562 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 110 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 130 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 196 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=7 max=-
- detector: field parsed (rejected)

### gpt-6-luna

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 14 | 24 | 1315 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 1174 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 155 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 8 | 18 | 4935 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 13 | 23 | 1114 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 11 | 21 | 1006 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 8 | 18 | 1593 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 122 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 135 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 109 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=8 max=-
- detector: field parsed (rejected)

### gpt-6-astra

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 7 | 17 | 3779 | accepted | - |
| 2 | top_level_effort | none | 400 | - | - | - | 126 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 3 | top_level_effort | minimal | 400 | - | - | - | 121 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 4 | 14 | 3554 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 7 | 17 | 2413 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 11 | 21 | 4078 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 14 | 24 | 3306 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 216 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 117 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 141 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=4 max=-
- detector: field parsed (rejected)

### gpt-5.6-sol

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 4 | 902 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 988 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 127 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 0 | 4 | 1408 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 0 | 4 | 1011 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 0 | 4 | 909 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 0 | 4 | 957 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 136 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 130 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 236 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=0 max=-
- detector: field parsed (rejected)

### gpt-5.6-luna

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 13 | 23 | 1240 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 1421 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 141 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 13 | 23 | 975 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 9 | 19 | 1119 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 13 | 23 | 1034 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 14 | 24 | 916 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 139 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 165 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 135 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=13 max=-
- detector: field parsed (rejected)

### gpt-5.6-terra

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 9 | 19 | 1106 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 872 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 117 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 9 | 19 | 1133 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 13 | 23 | 882 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 12 | 22 | 1142 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 13 | 23 | 1369 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 113 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 129 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 137 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=9 max=-
- detector: field parsed (rejected)

### gpt-5.5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 13 | 23 | 1151 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 924 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 204 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 13 | 23 | 1248 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 14 | 24 | 1206 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 20 | 30 | 1449 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 41 | 51 | 1718 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 1776 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 122 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 152 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=13 max=-
- detector: field parsed (rejected)

### gpt-5.4

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 4 | 1434 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 781 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 106 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 4 | 14 | 1261 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 12 | 22 | 1117 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 15 | 25 | 1341 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 14 | 24 | 1018 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 151 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 126 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 126 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=4 max=-
- detector: field parsed (rejected)

### gpt-5.4-mini

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 10 | 1133 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 793 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 224 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 12 | 22 | 1029 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 13 | 23 | 1119 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 18 | 28 | 1181 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 50 | 60 | 1585 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 306 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 110 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 140 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=12 max=-
- detector: field parsed (rejected)

### gpt-5.2

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 4 | 885 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 4 | 804 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 201 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 6 | 16 | 1062 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 12 | 22 | 1383 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 26 | 36 | 1345 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 14 | 24 | 1133 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 136 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 111 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 253 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=6 max=-
- detector: field parsed (rejected)

### gpt-5.1

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 10 | 669 | accepted | - |
| 2 | top_level_effort | none | 200 | stop | 0 | 10 | 1584 | accepted | - |
| 3 | top_level_effort | minimal | 400 | - | - | - | 205 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'none', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 0 | 10 | 991 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 10 | 20 | 1958 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 112 | 122 | 2269 | accepted | - |
| 7 | top_level_effort | xhigh | 400 | - | - | - | 189 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'xhigh' with this model. Supported values are: 'none', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 8 | top_level_effort | max | 400 | - | - | - | 117 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'none', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 136 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'none', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 167 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=0 max=-
- detector: field parsed (rejected)

### gpt-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 64 | 74 | 2214 | accepted | - |
| 2 | top_level_effort | none | 400 | - | - | - | 161 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 3 | top_level_effort | minimal | 200 | stop | 0 | 10 | 1940 | accepted | - |
| 4 | top_level_effort | low | 200 | stop | 0 | 10 | 1528 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 384 | 394 | 6393 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 128 | 138 | 3578 | accepted | - |
| 7 | top_level_effort | xhigh | 400 | - | - | - | 151 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'xhigh' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 8 | top_level_effort | max | 400 | - | - | - | 115 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 110 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 186 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=0 max=-
- detector: field parsed (rejected)

### gpt-5-mini

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 10 | 1258 | accepted | - |
| 2 | top_level_effort | none | 400 | - | - | - | 170 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 3 | top_level_effort | minimal | 200 | stop | 0 | 10 | 699 | accepted | - |
| 4 | top_level_effort | low | 200 | stop | 0 | 10 | 937 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 64 | 74 | 1274 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 64 | 74 | 1861 | accepted | - |
| 7 | top_level_effort | xhigh | 400 | - | - | - | 146 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'xhigh' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 8 | top_level_effort | max | 400 | - | - | - | 120 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 179 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 176 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=0 max=-
- detector: field parsed (rejected)

### gpt-5-nano

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 128 | 138 | 6041 | accepted | - |
| 2 | top_level_effort | none | 400 | - | - | - | 281 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 3 | top_level_effort | minimal | 200 | stop | 0 | 10 | 3373 | accepted | - |
| 4 | top_level_effort | low | 200 | stop | 64 | 74 | 4032 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 64 | 74 | 4996 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 448 | 458 | 11263 | accepted | - |
| 7 | top_level_effort | xhigh | 400 | - | - | - | 122 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'xhigh' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 8 | top_level_effort | max | 400 | - | - | - | 125 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'max' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 125 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'bog...-51' with this model. Supported values are: 'minimal', 'low', 'medium', and 'high'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 133 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=64 max=-
- detector: field parsed (rejected)

### o3

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 0 | 19 | 1287 | accepted | - |
| 2 | top_level_effort | none | 400 | - | - | - | 164 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 3 | top_level_effort | minimal | 400 | - | - | - | 131 | rejected | {   "error": {     "message": "Unsupported value: 'reasoning_effort' does not support 'minimal' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "unsupported_value"   } } |
| 4 | top_level_effort | low | 200 | stop | 0 | 19 | 687 | accepted | - |
| 5 | top_level_effort | medium | 200 | stop | 64 | 83 | 1258 | accepted | - |
| 6 | top_level_effort | high | 200 | stop | 0 | 19 | 1575 | accepted | - |
| 7 | top_level_effort | xhigh | 200 | stop | 0 | 19 | 765 | accepted | - |
| 8 | top_level_effort | max | 400 | - | - | - | 127 | rejected | {   "error": {     "message": "Invalid value: 'max'. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "invalid_value"   } } |
| 9 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 137 | rejected | {   "error": {     "message": "Invalid value: 'bogus-effort-51'. Supported values are: 'low', 'medium', 'high', and 'xhigh'.",     "type": "invalid_request_error",     "param": "reasoning_effort",     "code": "invalid_value"   } } |
| 10 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 195 | rejected | {   "error": {     "message": "Unsupported value: 'temperature' does not support 0.5 with this model. Only the default (1) value is supported.",     "type": "invalid_request_error",     "param": "temperature",     "code": "unsupported_value"   } } |

- `top_level_effort`: field parsed: yes; effect: low=0 max=-
- detector: field parsed (rejected)

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:21:13Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

- base URL: `https://api.anthropic.com/v1`
- chat endpoint: `POST https://api.anthropic.com/v1/messages`; credential header `x-api-key` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### claude-haiku-5-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 116 | 119 | 3881 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 101 | 104 | 2359 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 62 | 65 | 921 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 102 | 105 | 1259 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 92 | 129 | 1145 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 137 | 140 | 1213 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 86 | 123 | 1286 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 108 | 145 | 1332 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 60 | 63 | 1023 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 71 | 74 | 1450 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 227 | 230 | 1824 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | 0 | 3 | 872 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 95 | 132 | 2269 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 174 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRa1ozCxVSqfq7TN71Y"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 154 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRa2RSS56yVXRYRAQgP"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 190 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRa39c19cMG981scKEg"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 139 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRa3vF7anLs1eLjH7DW"} |

- `output_config`: field parsed: yes; effect: low=101 max=137
- `output_config+adaptive`: field parsed: unknown; effect: low=86 max=227
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-opus-4-8

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 0 | 37 | 1505 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 0 | 37 | 945 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 0 | 37 | 720 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 0 | 37 | 885 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 0 | 11 | 1101 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 0 | 12 | 574 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 0 | 3 | 430 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 0 | 3 | 939 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 0 | 3 | 535 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 0 | 3 | 1021 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 41 | 44 | 943 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | 0 | 37 | 1370 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 0 | 3 | 555 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 315 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRaurb6pSChZNYmrapm"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 218 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRawPN1LGLhn4LdL7KV"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 225 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRax62yk481GBFAb3d4"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 181 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRay4ZWCtZmWNnFayN5"} |

- `output_config`: field parsed: yes; effect: low=0 max=0
- `output_config+adaptive`: field parsed: unknown; effect: low=0 max=41
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-fable-5-1

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 0 | 37 | 1500 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 0 | 37 | 1411 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 0 | 3 | 1127 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 0 | 3 | 1192 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 60 | 63 | 1617 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 36 | 39 | 1542 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 0 | 37 | 1435 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 0 | 37 | 1359 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 68 | 71 | 1759 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 123 | 126 | 2192 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 36 | 39 | 1390 | accepted | - |
| 12 | thinking_object | off | 400 | - | - | - | 255 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.disabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRcCXDTGJ83CHa6yrhN"} |
| 13 | thinking_object | auto | 200 | end_turn | 30 | 33 | 1433 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 126 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRcKjZ8UKLyb9Yo3avq"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 284 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRcLRUDXckHi2FwZtnT"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 200 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRcMWhkaKymCTbdig5F"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 162 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRcNMp7k8b6XB5fdo1U"} |

- `output_config`: field parsed: yes; effect: low=0 max=36
- `output_config+adaptive`: field parsed: unknown; effect: low=0 max=36
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-fable-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 19 | 22 | 3041 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 7 | 10 | 2594 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 7 | 10 | 3637 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 11 | 14 | 2616 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 19 | 22 | 3077 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 144 | 147 | 4532 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 7 | 10 | 3173 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 11 | 14 | 2966 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 11 | 14 | 3077 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 19 | 22 | 2922 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 149 | 152 | 4036 | accepted | - |
| 12 | thinking_object | off | 400 | - | - | - | 199 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.disabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRf1hfoN7uNZsxrRNey"} |
| 13 | thinking_object | auto | 200 | end_turn | 11 | 14 | 3175 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 144 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRfG48JsXtSqNNiE2JJ"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 163 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRfGko5sDgfizhHvjhE"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 136 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRfHRE9vYgkdoKAoziF"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 170 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRfHxUgJEaPbrfWpke2"} |

- `output_config`: field parsed: yes; effect: low=7 max=144
- `output_config+adaptive`: field parsed: unknown; effect: low=7 max=149
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-opus-5-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 10 | 13 | 821 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 8 | 11 | 810 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 8 | 11 | 821 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 10 | 13 | 1273 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 10 | 13 | 809 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 106 | 109 | 1717 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 170 | 173 | 3086 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 8 | 11 | 800 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 428 | 431 | 14767 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 10 | 13 | 866 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 106 | 109 | 1593 | accepted | - |
| 12 | thinking_object | off | 400 | - | - | - | 188 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.disabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRhKyLCPNAdzLoCQw7P"} |
| 13 | thinking_object | auto | 200 | end_turn | 8 | 11 | 849 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 303 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRhQDKjXaZwSWF3PDiA"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 261 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRhRVhr9LtUaG1H68Ca"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 159 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRhSefrxJSwTwWMQ82T"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 144 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRhTJbpSFznoJYzkSAY"} |

- `output_config`: field parsed: yes; effect: low=8 max=106
- `output_config+adaptive`: field parsed: unknown; effect: low=170 max=106
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-opus-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 12 | 15 | 924 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 11 | 14 | 2068 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 12 | 15 | 875 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 12 | 15 | 877 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 12 | 15 | 1090 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 63 | 66 | 1436 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 11 | 14 | 932 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 12 | 15 | 922 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 12 | 15 | 926 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 112 | 115 | 2869 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 129 | 132 | 2287 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | 0 | 3 | 1059 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 12 | 15 | 821 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 307 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRijL3edLdsp1RBGVvR"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 158 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRikTkqjk8VK4F4Wd2t"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 249 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRim1F1YUbQxVs4Hc67"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 122 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRin5jsjCwS1Z3NGGJj"} |

- `output_config`: field parsed: yes; effect: low=11 max=63
- `output_config+adaptive`: field parsed: unknown; effect: low=11 max=129
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-sonnet-5-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 0 | 3 | 1146 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 0 | 3 | 1290 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 0 | 3 | 1252 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 0 | 3 | 1305 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 29 | 32 | 1536 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 65 | 68 | 1636 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 0 | 3 | 1330 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 0 | 3 | 1227 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 0 | 3 | 1307 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 17 | 20 | 1349 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 66 | 69 | 1562 | accepted | - |
| 12 | thinking_object | off | 400 | - | - | - | 177 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"To turn thinking off on this model, send \"thinking\": {\"type\": \"between_tools\"} instead of {\"type\": \"disabled\"}. The model does not think before responding. The short updates it writes between tool calls come back as thinking blocks."},"request_id":"req_011CgHRjtfu1d2ce6sX9FkGg"} |
| 13 | thinking_object | auto | 200 | end_turn | 0 | 3 | 1270 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 367 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRjzkmCpD8JahqGWXEh"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 163 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRk2RxrrYNfYNGeHVib"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 149 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRk35AZjFgzgcMEGCxM"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 132 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRk3fdWXgqShMMxPCWU"} |

- `output_config`: field parsed: yes; effect: low=0 max=65
- `output_config+adaptive`: field parsed: unknown; effect: low=0 max=66
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-sonnet-5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 35 | 39 | 1133 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 0 | 3 | 656 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 0 | 37 | 1077 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 0 | 37 | 1078 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 35 | 38 | 1127 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 164 | 202 | 2398 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 0 | 3 | 655 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 0 | 37 | 1094 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 44 | 47 | 1035 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 47 | 50 | 1176 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 141 | 144 | 1819 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | 0 | 43 | 1038 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 0 | 37 | 1089 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 140 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRmC1xZUU62mZ7rrrAs"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 157 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRmCkcZ9LezZt8BYKbG"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 141 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRmDJbfuMWd3mLFm6wB"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 152 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRmDv49WRKetapuNnLx"} |

- `output_config`: field parsed: yes; effect: low=0 max=164
- `output_config+adaptive`: field parsed: unknown; effect: low=0 max=141
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-opus-4-7

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | 0 | 6 | 883 | accepted | - |
| 2 | output_config | low | 200 | end_turn | 0 | 6 | 1921 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | 0 | 6 | 1496 | accepted | - |
| 4 | output_config | high | 200 | end_turn | 0 | 6 | 1879 | accepted | - |
| 5 | output_config | xhigh | 200 | end_turn | 0 | 6 | 1180 | accepted | - |
| 6 | output_config | max | 200 | end_turn | 0 | 6 | 863 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 0 | 6 | 1352 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 0 | 6 | 819 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 0 | 6 | 1893 | accepted | - |
| 10 | output_config+adaptive | xhigh | 200 | end_turn | 0 | 6 | 1202 | accepted | - |
| 11 | output_config+adaptive | max | 200 | end_turn | 50 | 57 | 1095 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | 0 | 6 | 5629 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 0 | 6 | 1645 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 132 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRnr5hvqP8iKGA9AR79"} |
| 15 | thinking_budget | 1024 | 400 | - | - | - | 162 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."},"request_id":"req_011CgHRnrmso94d2i2VHGh3L"} |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 127 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRnsN72hhMp3gVVTKFV"} |
| 17 | output_config | high+temperature=0.5 | 400 | - | - | - | 137 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"`temperature` is deprecated for this model."},"request_id":"req_011CgHRnstMbgaKEuKnUnmba"} |

- `output_config`: field parsed: yes; effect: low=0 max=0
- `output_config+adaptive`: field parsed: unknown; effect: low=0 max=50
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=- 1023=-

### claude-opus-4-6

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | - | 41 | 2215 | accepted | - |
| 2 | output_config | low | 200 | end_turn | - | 5 | 1638 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | - | 5 | 1577 | accepted | - |
| 4 | output_config | high | 200 | end_turn | - | 41 | 1847 | accepted | - |
| 5 | output_config | xhigh | 400 | - | - | - | 145 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"This model does not support effort level 'xhigh'. Supported levels: high, low, max, medium."},"request_id":"req_011CgHRoReJbpch25MQU57bh"} |
| 6 | output_config | max | 200 | end_turn | - | 41 | 2141 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 0 | 5 | 1654 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 0 | 5 | 1691 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 46 | 53 | 3462 | accepted | - |
| 10 | output_config+adaptive | xhigh | 400 | - | - | - | 303 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"This model does not support effort level 'xhigh'. Supported levels: high, low, max, medium."},"request_id":"req_011CgHRp758fVooc3cyR2rrt"} |
| 11 | output_config+adaptive | max | 200 | end_turn | 48 | 55 | 2397 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | - | 41 | 2419 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 46 | 53 | 2319 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 122 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRpeLKtX5j4VFTxiXkM"} |
| 15 | thinking_budget | 1024 | 200 | end_turn | 39 | 45 | 3239 | accepted | - |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 201 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRptpiRMzao36c8gxtk"} |
| 17 | output_config | high+temperature=0.5 | 200 | end_turn | - | 41 | 2051 | accepted | - |

- `output_config`: field parsed: yes; effect: low=- max=-
- `output_config+adaptive`: field parsed: unknown; effect: low=0 max=48
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=39 1023=-

### claude-sonnet-4-6

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | end_turn | - | 42 | 1514 | accepted | - |
| 2 | output_config | low | 200 | end_turn | - | 42 | 953 | accepted | - |
| 3 | output_config | medium | 200 | end_turn | - | 42 | 6651 | accepted | - |
| 4 | output_config | high | 200 | end_turn | - | 42 | 1372 | accepted | - |
| 5 | output_config | xhigh | 400 | - | - | - | 154 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"This model does not support effort level 'xhigh'. Supported levels: high, low, max, medium."},"request_id":"req_011CgHRqqJfGUB1v9BeCxfe3"} |
| 6 | output_config | max | 200 | end_turn | - | 41 | 1385 | accepted | - |
| 7 | output_config+adaptive | low | 200 | end_turn | 14 | 20 | 1146 | accepted | - |
| 8 | output_config+adaptive | medium | 200 | end_turn | 46 | 52 | 2074 | accepted | - |
| 9 | output_config+adaptive | high | 200 | end_turn | 46 | 52 | 1596 | accepted | - |
| 10 | output_config+adaptive | xhigh | 400 | - | - | - | 316 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"This model does not support effort level 'xhigh'. Supported levels: high, low, max, medium."},"request_id":"req_011CgHRrJUnmtab1T8hk9bEo"} |
| 11 | output_config+adaptive | max | 200 | end_turn | 48 | 54 | 1358 | accepted | - |
| 12 | thinking_object | off | 200 | end_turn | - | 42 | 1436 | accepted | - |
| 13 | thinking_object | auto | 200 | end_turn | 46 | 52 | 1497 | accepted | - |
| 14 | output_config | bogus-effort-51 | 400 | - | - | - | 143 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"output_config.effort: Input should be 'low', 'medium', 'high', 'xhigh' or 'max'"},"request_id":"req_011CgHRreDBixFN1MRSHY4Em"} |
| 15 | thinking_budget | 1024 | 200 | end_turn | 39 | 45 | 1599 | accepted | - |
| 16 | thinking_budget | 1023 | 400 | - | - | - | 233 | rejected | {"type":"error","error":{"type":"invalid_request_error","message":"thinking.enabled.budget_tokens: Input should be greater than or equal to 1024"},"request_id":"req_011CgHRrmbSGrboNQ9NCgp4S"} |
| 17 | output_config | high+temperature=0.5 | 200 | end_turn | - | 42 | 2560 | accepted | - |

- `output_config`: field parsed: yes; effect: low=- max=-
- `output_config+adaptive`: field parsed: unknown; effect: low=14 max=48
- `thinking_object`: field parsed: unknown
- `thinking_budget`: field parsed: unknown; budget: 1024=39 1023=-

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:25:25Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

- base URL: `https://api.deepseek.com/v1`
- chat endpoint: `POST https://api.deepseek.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### deepseek-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 20 | 22 | 1147 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 20 | 22 | 952 | accepted | - |
| 3 | top_level_effort | low | 200 | stop | 24 | 26 | 771 | accepted | - |
| 4 | top_level_effort | medium | 200 | stop | 19 | 21 | 889 | accepted | - |
| 5 | top_level_effort | high | 200 | stop | 17 | 19 | 946 | accepted | - |
| 6 | top_level_effort | xhigh | 200 | stop | 18 | 20 | 795 | accepted | - |
| 7 | top_level_effort | max | 200 | stop | 22 | 24 | 706 | accepted | - |
| 8 | thinking_object | off | 200 | stop | - | 9 | 1168 | accepted | - |
| 9 | thinking_object | auto | 200 | stop | 17 | 19 | 739 | accepted | - |
| 10 | top_level_effort | bogus-effort-51 | 422 | - | - | - | 429 | rejected | {"error":{"message":"Failed to deserialize the JSON body into the target type: reasoning_effort: unknown variant `bogus-effort-51`, expected one of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `ultra`, `max` at line 1 column 197 (request_id: 6e01afc3-69a9-402d-aab8-9f38dbead1ca)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} |
| 11 | thinking_object | bogus-effort-51 | 422 | - | - | - | 427 | rejected | {"error":{"message":"Failed to deserialize the JSON body into the target type: thinking.type: unknown variant `bogus-effort-51`, expected one of `adaptive`, `enabled`, `disabled` at line 1 column 197 (request_id: 34f8bce9-182a-449a-a0b7-afc10f96b33d)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} |
| 12 | top_level_effort | high+temperature=0.5 | 200 | stop | 19 | 21 | 1100 | accepted | - |

- `top_level_effort`: field parsed: yes; effect: low=24 max=22
- `thinking_object`: field parsed: yes
- detector: field parsed (rejected)

### deepseek-v4-pro

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 35 | 37 | 1799 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 37 | 39 | 1267 | accepted | - |
| 3 | top_level_effort | low | 200 | stop | 25 | 27 | 1774 | accepted | - |
| 4 | top_level_effort | medium | 200 | stop | 36 | 38 | 1730 | accepted | - |
| 5 | top_level_effort | high | 200 | stop | 35 | 37 | 1518 | accepted | - |
| 6 | top_level_effort | xhigh | 200 | stop | 35 | 37 | 2010 | accepted | - |
| 7 | top_level_effort | max | 200 | stop | 39 | 41 | 1701 | accepted | - |
| 8 | thinking_object | off | 200 | stop | - | 52 | 1610 | accepted | - |
| 9 | thinking_object | auto | 200 | stop | 110 | 112 | 2524 | accepted | - |
| 10 | top_level_effort | bogus-effort-51 | 422 | - | - | - | 351 | rejected | {"error":{"message":"Failed to deserialize the JSON body into the target type: reasoning_effort: unknown variant `bogus-effort-51`, expected one of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `ultra`, `max` at line 1 column 198 (request_id: d964ecf1-38cc-4293-9b1c-6ba78eb0a5e8)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} |
| 11 | thinking_object | bogus-effort-51 | 422 | - | - | - | 424 | rejected | {"error":{"message":"Failed to deserialize the JSON body into the target type: thinking.type: unknown variant `bogus-effort-51`, expected one of `adaptive`, `enabled`, `disabled` at line 1 column 198 (request_id: acd28519-ad5b-4f9a-bec2-72a0c8aa29c5)","type":"invalid_request_error","param":null,"code":"invalid_request_error"}} |
| 12 | top_level_effort | high+temperature=0.5 | 200 | stop | 35 | 37 | 1673 | accepted | - |

- `top_level_effort`: field parsed: yes; effect: low=25 max=39
- `thinking_object`: field parsed: yes
- detector: field parsed (rejected)

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:16:46Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

- base URL: `https://api.moonshot.ai/v1`
- chat endpoint: `POST https://api.moonshot.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### kimi-k3

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 33 | 49 | 2981 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 46 | 62 | 3370 | accepted | - |
| 3 | thinking_object | minimal | 400 | - | - | - | 315 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: minimal","type":"invalid_request_error"}} |
| 4 | top_level_effort | low | 200 | stop | 47 | 63 | 4813 | accepted | - |
| 5 | thinking_object | low | 200 | stop | 40 | 56 | 4339 | accepted | - |
| 6 | top_level_effort | medium | 200 | stop | 48 | 64 | 3538 | accepted | - |
| 7 | thinking_object | medium | 400 | - | - | - | 297 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: medium","type":"invalid_request_error"}} |
| 8 | top_level_effort | high | 200 | stop | 35 | 51 | 3904 | accepted | - |
| 9 | thinking_object | high | 200 | stop | 48 | 64 | 4835 | accepted | - |
| 10 | top_level_effort | xhigh | 200 | stop | 51 | 67 | 3269 | accepted | - |
| 11 | thinking_object | xhigh | 400 | - | - | - | 342 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: xhigh","type":"invalid_request_error"}} |
| 12 | top_level_effort | max | 200 | stop | 86 | 102 | 5263 | accepted | - |
| 13 | thinking_object | max | 200 | stop | 116 | 132 | 5856 | accepted | - |
| 14 | thinking_object | off | 200 | stop | - | 11 | 2654 | accepted | - |
| 15 | thinking_object | auto | 200 | stop | 71 | 87 | 4742 | accepted | - |
| 16 | top_level_effort | bogus-effort-51 | 200 | stop | 43 | 59 | 3952 | accepted | - |
| 17 | thinking_object | bogus-effort-51 | 400 | - | - | - | 306 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: bogus-effort-51","type":"invalid_request_error"}} |
| 18 | thinking_object | high+temperature=0.5 | 400 | - | - | - | 310 | rejected | {"error":{"message":"invalid temperature: only 1 is allowed for this model","type":"invalid_request_error"}} |

- `top_level_effort`: field parsed: no; effect: low=47 max=86
- `thinking_object`: field parsed: yes; effect: low=40 max=116
- detector: field not parsed (silent)

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:25:53Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

- base URL: `https://api.moonshot.ai/v1`
- chat endpoint: `POST https://api.moonshot.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### kimi-k2.6

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 164 | 167 | 4006 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 195 | 198 | 4570 | accepted | - |
| 3 | thinking_object | minimal | 400 | - | - | - | 325 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: minimal","type":"invalid_request_error"}} |
| 4 | top_level_effort | low | 200 | stop | 128 | 131 | 3393 | accepted | - |
| 5 | thinking_object | low | 200 | stop | 141 | 144 | 3460 | accepted | - |
| 6 | top_level_effort | medium | 200 | stop | 416 | 419 | 6984 | accepted | - |
| 7 | thinking_object | medium | 400 | - | - | - | 251 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: medium","type":"invalid_request_error"}} |
| 8 | top_level_effort | high | 200 | stop | 378 | 381 | 6473 | accepted | - |
| 9 | thinking_object | high | 200 | stop | 620 | 623 | 13651 | accepted | - |
| 10 | top_level_effort | xhigh | 200 | stop | 472 | 475 | 9396 | accepted | - |
| 11 | thinking_object | xhigh | 400 | - | - | - | 314 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: xhigh","type":"invalid_request_error"}} |
| 12 | top_level_effort | max | 200 | stop | 327 | 330 | 6624 | accepted | - |
| 13 | thinking_object | max | 200 | stop | 141 | 144 | 3476 | accepted | - |
| 14 | thinking_object | off | 200 | stop | 1 | 4 | 666 | accepted | - |
| 15 | thinking_object | auto | 200 | stop | 140 | 143 | 3140 | accepted | - |
| 16 | top_level_effort | bogus-effort-51 | 200 | stop | 212 | 215 | 5045 | accepted | - |
| 17 | thinking_object | bogus-effort-51 | 400 | - | - | - | 256 | rejected | {"error":{"message":"Invalid request: invalid value for thinking effort level: bogus-effort-51","type":"invalid_request_error"}} |
| 18 | thinking_object | high+temperature=0.5 | 400 | - | - | - | 289 | rejected | {"error":{"message":"invalid temperature: only 1 is allowed for this model","type":"invalid_request_error"}} |

- `top_level_effort`: field parsed: no; effect: low=128 max=327
- `thinking_object`: field parsed: yes; effect: low=141 max=141
- detector: field not parsed (silent)

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:27:06Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

- base URL: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`
- chat endpoint: `POST https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### qwen3.8-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 53 | 59 | 2158 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 910 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 50 | 56 | 1689 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 7 | 1124 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 713 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-61981524-9d89-99fb-9fbd-969200de1054","request_id":"61981524-9d89-99fb-9fbd-969200de1054"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 384 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-f2ecb523-ec1c-92b1-a151-689b426bdfe2","request_id":"f2ecb523-ec1c-92b1-a151-689b426bdfe2"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 432 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-bfc813ce-37f9-94eb-944f-95061840c889","request_id":"bfc813ce-37f9-94eb-944f-95061840c889"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 51 | 57 | 1431 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.8-max

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 53 | 59 | 2396 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 853 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 47 | 53 | 2544 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 7 | 2913 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 509 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-50ec66eb-8e3f-9f1f-9b63-c976286df3e9","request_id":"50ec66eb-8e3f-9f1f-9b63-c976286df3e9"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 640 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-47a93a2e-eca4-9da1-adf7-373e616a8351","request_id":"47a93a2e-eca4-9da1-adf7-373e616a8351"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 781 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-23d5c1b0-5449-9712-9521-255e778ccb22","request_id":"23d5c1b0-5449-9712-9521-255e778ccb22"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 48 | 54 | 3264 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.7-plus

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 186 | 192 | 4101 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 1240 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 146 | 152 | 3486 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 12 | 1493 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 660 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-55cc8a28-f2bc-9ec3-ab57-01bc92fbe147","request_id":"55cc8a28-f2bc-9ec3-ab57-01bc92fbe147"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 507 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-56ff866a-3430-93f9-8758-2b3af138cff0","request_id":"56ff866a-3430-93f9-8758-2b3af138cff0"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 803 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-955339e1-16f3-9f0e-88d6-b42f19c8098e","request_id":"955339e1-16f3-9f0e-88d6-b42f19c8098e"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 792 | 833 | 14057 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.7-max

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 200 | 206 | 3076 | accepted | - |
| 2 | enable_thinking | off | - | - | - | - | 723 | error | error sending request |
| 3 | enable_thinking | auto | 200 | stop | 247 | 253 | 3609 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 7 | 1271 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 373 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-86f02285-318a-948b-bccd-0645bc99a600","request_id":"86f02285-318a-948b-bccd-0645bc99a600"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 675 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-ad80a060-24c0-92b8-b889-2e380adb05b5","request_id":"ad80a060-24c0-92b8-b889-2e380adb05b5"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 797 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-5cfa25d9-51d1-920c-a1f5-0e2c5e10b9e7","request_id":"5cfa25d9-51d1-920c-a1f5-0e2c5e10b9e7"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 540 | 546 | 7812 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.7-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 354 | 360 | 3853 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 718 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 823 | 829 | 9326 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 7 | 899 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 524 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 131072","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-857f3aa0-2240-9f9d-a11a-a64d9365cd65","request_id":"857f3aa0-2240-9f9d-a11a-a64d9365cd65"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 775 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 131072","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-27ff5a53-dc37-9ef4-89fc-86260c2e8c1e","request_id":"27ff5a53-dc37-9ef4-89fc-86260c2e8c1e"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 758 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-d602f557-bd51-99be-bad5-0a01db059ffc","request_id":"d602f557-bd51-99be-bad5-0a01db059ffc"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 516 | 522 | 5991 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.6-plus

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 1981 | 1989 | 34769 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 840 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 2645 | 2653 | 46014 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 9 | 1572 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 522 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 131072","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-8663bd1e-1acd-9af8-8461-b3bad9a90696","request_id":"8663bd1e-1acd-9af8-8461-b3bad9a90696"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 395 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 131072","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-4005a1c5-9946-92b3-9bcd-9f42d5c8e0e7","request_id":"4005a1c5-9946-92b3-9bcd-9f42d5c8e0e7"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 254 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_value"},"request_id":"chatcmpl-596a8094-d4a6-9fb1-91a6-43037be1557b"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 992 | 1000 | 17744 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.6-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 290 | 298 | 3190 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 1125 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 318 | 326 | 3489 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 9 | 889 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 319 | rejected | {"error":{"message":"The thinking_budget parameter must be a positive integer and not greater than 131072","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-3ecbce73-26db-9a47-b6fd-8e211329fb87","request_id":"3ecbce73-26db-9a47-b6fd-8e211329fb87"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 325 | rejected | {"error":{"message":"The thinking_budget parameter must be a positive integer and not greater than 131072","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-2a252802-357c-9afe-908b-5750f8ee4633","request_id":"2a252802-357c-9afe-908b-5750f8ee4633"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 331 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-b04f91f1-c250-9fdf-b027-38e68f2ab375","request_id":"b04f91f1-c250-9fdf-b027-38e68f2ab375"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 335 | 343 | 3468 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.5-plus

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 4903 | 4911 | 95209 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 2452 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 4699 | 4761 | 85213 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 9 | 2160 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 435 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-374cf44b-1d5c-930d-9c94-d0a199cbe86f","request_id":"374cf44b-1d5c-930d-9c94-d0a199cbe86f"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 714 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-68a06022-da45-9e96-9985-a45ae54c908a","request_id":"68a06022-da45-9e96-9985-a45ae54c908a"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 254 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_value"},"request_id":"chatcmpl-bb54a9fc-1814-9741-9569-ac270e2d2ac3"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 1652 | 1803 | 31168 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen3.5-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 3422 | 3430 | 20196 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 474 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 462 | 470 | 3364 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 9 | 614 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 499 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-d2868da8-caaf-9978-90c7-742eb9f98f95","request_id":"d2868da8-caaf-9978-90c7-742eb9f98f95"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 533 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-ca3708fb-0a80-9aee-8785-97407e7e4296","request_id":"ca3708fb-0a80-9aee-8785-97407e7e4296"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 252 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_value"},"request_id":"chatcmpl-a1b0c190-9218-9152-8f7d-8195ca0f2821"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 1948 | 1956 | 11762 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

### qwen-plus

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | - | 3 | 617 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 538 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 1047 | 1055 | 9746 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 9 | 571 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 428 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-6b68cd26-ed39-9942-a5ac-00048fd2d6c6","request_id":"6b68cd26-ed39-9942-a5ac-00048fd2d6c6"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 293 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-6a7ed9f8-be78-9a7e-997a-748e993c5bc6","request_id":"6a7ed9f8-be78-9a7e-997a-748e993c5bc6"} |
| 7 | detector | bogus-effort-51 | 200 | stop | - | 3 | 656 | accepted | - |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 1010 | 1018 | 11431 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field not parsed (silent)

### qwen-max

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | - | 3 | 496 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 520 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | - | 3 | 470 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | - | 3 | 495 | accepted | - |
| 5 | thinking_budget | 0 | 200 | stop | - | 3 | 518 | accepted | - |
| 6 | thinking_budget | -1 | 200 | stop | - | 3 | 491 | accepted | - |
| 7 | detector | bogus-effort-51 | 200 | stop | - | 3 | 532 | accepted | - |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | - | 3 | 536 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: no; budget: 1=- 0=-
- detector: field not parsed (silent)

### qwen-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | - | 11 | 415 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 403 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 336 | 344 | 2805 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 9 | 454 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 598 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-df4cbcc6-f7ab-92e6-b741-d9625b1bd0bb","request_id":"df4cbcc6-f7ab-92e6-b741-d9625b1bd0bb"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 620 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 81920","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-77fa94ae-916b-9dda-bd7e-bd3cc516b976","request_id":"77fa94ae-916b-9dda-bd7e-bd3cc516b976"} |
| 7 | detector | bogus-effort-51 | 200 | stop | - | 11 | 521 | accepted | - |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 269 | 277 | 2299 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field not parsed (silent)

### qwen-plus-latest

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | - | 3 | 748 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 715 | accepted | - |
| 3 | enable_thinking | auto | 400 | - | - | - | 443 | rejected | {"error":{"message":"parameter.enable_thinking only support stream call","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-e89e6c24-a793-9239-8ce3-e4d391278650","request_id":"e89e6c24-a793-9239-8ce3-e4d391278650"} |
| 4 | thinking_budget | 1 | 400 | - | - | - | 382 | rejected | {"error":{"message":"parameter.enable_thinking only support stream call","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-792310e2-afc7-95e8-a8bb-6feff9811495","request_id":"792310e2-afc7-95e8-a8bb-6feff9811495"} |
| 5 | thinking_budget | 0 | 400 | - | - | - | 680 | rejected | {"error":{"message":"parameter.enable_thinking only support stream call","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-acc599da-c79c-997d-9bab-76f61df802b8","request_id":"acc599da-c79c-997d-9bab-76f61df802b8"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 847 | rejected | {"error":{"message":"parameter.enable_thinking only support stream call","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-5135de83-a35a-9ef0-8bef-f688d5f68b8a","request_id":"5135de83-a35a-9ef0-8bef-f688d5f68b8a"} |
| 7 | detector | bogus-effort-51 | 200 | stop | - | 3 | 610 | accepted | - |
| 8 | enable_thinking | auto+temperature=0.5 | 400 | - | - | - | 312 | rejected | {"error":{"message":"parameter.enable_thinking only support stream call","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-a0a9243a-5d96-9915-a338-f20b529e9834","request_id":"a0a9243a-5d96-9915-a338-f20b529e9834"} |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=- 0=-
- detector: field not parsed (silent)

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:46:52Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

- base URL: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`
- chat endpoint: `POST https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### qwen3.7-max

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 400 | 406 | 5749 | accepted | - |
| 2 | enable_thinking | off | 200 | stop | - | 3 | 1264 | accepted | - |
| 3 | enable_thinking | auto | 200 | stop | 282 | 288 | 5030 | accepted | - |
| 4 | thinking_budget | 1 | 200 | stop | 1 | 15 | 2113 | accepted | - |
| 5 | thinking_budget | 0 | 400 | - | - | - | 723 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-c14b63dc-78d4-9a3e-921d-98d8da9301a2","request_id":"c14b63dc-78d4-9a3e-921d-98d8da9301a2"} |
| 6 | thinking_budget | -1 | 400 | - | - | - | 668 | rejected | {"error":{"message":"<400> InternalError.Algo.InvalidParameter: The thinking_budget parameter must be a positive integer and not greater than 262144","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-fa662ff5-c648-95cc-b02b-950bc5b58c7e","request_id":"fa662ff5-c648-95cc-b02b-950bc5b58c7e"} |
| 7 | detector | bogus-effort-51 | 400 | - | - | - | 349 | rejected | {"error":{"message":"'reasoning_effort' must be one of: 'none', 'minimal', 'low', 'medium', 'high', 'xhigh'","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"},"id":"chatcmpl-b86d615b-4b49-94a5-9b05-f10072a600ac","request_id":"b86d615b-4b49-94a5-9b05-f10072a600ac"} |
| 8 | enable_thinking | auto+temperature=0.5 | 200 | stop | 524 | 530 | 6830 | accepted | - |

- `enable_thinking`: field parsed: unknown
- `thinking_budget`: field parsed: yes; budget: 1=1 0=-
- detector: field parsed (rejected)

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:35:28Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

- base URL: `https://api.x.ai/v1`
- chat endpoint: `POST https://api.x.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### grok-4.3

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 327 | 1 | 3444 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 353 | 7 | 3251 | accepted | - |
| 3 | top_level_effort | low | 200 | stop | 270 | 62 | 2951 | accepted | - |
| 4 | top_level_effort | medium | 200 | stop | 877 | 7 | 7558 | accepted | - |
| 5 | top_level_effort | high | 200 | stop | 426 | 1 | 3639 | accepted | - |
| 6 | top_level_effort | xhigh | 200 | stop | 245 | 1 | 1916 | accepted | - |
| 7 | top_level_effort | max | 400 | - | - | - | 87 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 8 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 93 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 9 | top_level_effort | high+temperature=0.5 | 200 | stop | 230 | 1 | 1904 | accepted | - |

- `top_level_effort`: field parsed: yes; effect: low=270 max=-
- detector: field parsed (rejected)

### grok-4.6

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 189 | 1 | 2281 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 145 | 1 | 2220 | accepted | - |
| 3 | top_level_effort | low | 200 | stop | 154 | 1 | 1881 | accepted | - |
| 4 | top_level_effort | medium | 200 | stop | 126 | 1 | 3032 | accepted | - |
| 5 | top_level_effort | high | 200 | stop | 296 | 1 | 6574 | accepted | - |
| 6 | top_level_effort | xhigh | 200 | stop | 267 | 1 | 3339 | accepted | - |
| 7 | top_level_effort | max | 400 | - | - | - | 240 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 8 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 96 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 9 | top_level_effort | high+temperature=0.5 | 200 | stop | 263 | 1 | 4288 | accepted | - |

- `top_level_effort`: field parsed: yes; effect: low=154 max=-
- detector: field parsed (rejected)

### grok-4.5

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 130 | 1 | 1448 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 133 | 1 | 1332 | accepted | - |
| 3 | top_level_effort | low | 200 | stop | 128 | 1 | 1549 | accepted | - |
| 4 | top_level_effort | medium | 200 | stop | 132 | 1 | 1566 | accepted | - |
| 5 | top_level_effort | high | 200 | stop | 126 | 1 | 1677 | accepted | - |
| 6 | top_level_effort | xhigh | 200 | stop | 120 | 1 | 1472 | accepted | - |
| 7 | top_level_effort | max | 400 | - | - | - | 153 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 8 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 73 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 9 | top_level_effort | high+temperature=0.5 | 200 | stop | 116 | 1 | 1386 | accepted | - |

- `top_level_effort`: field parsed: yes; effect: low=128 max=-
- detector: field parsed (rejected)

### grok-4.20-reasoning

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 351 | 7 | 4504 | accepted | - |
| 2 | top_level_effort | minimal | 400 | - | - | - | 136 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 3 | top_level_effort | low | 400 | - | - | - | 91 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 4 | top_level_effort | medium | 400 | - | - | - | 91 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 5 | top_level_effort | high | 400 | - | - | - | 93 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 6 | top_level_effort | xhigh | 400 | - | - | - | 58 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 7 | top_level_effort | max | 400 | - | - | - | 90 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 8 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 91 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |
| 9 | top_level_effort | high+temperature=0.5 | 400 | - | - | - | 87 | rejected | {"code":"invalid-argument","error":"Model grok-4.20-reasoning does not support parameter reasoningEffort."} |

- `top_level_effort`: field parsed: yes; effect: low=- max=-
- detector: field parsed (rejected)

### grok-4.7

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | stop | 93 | 1 | 1518 | accepted | - |
| 2 | top_level_effort | minimal | 200 | stop | 96 | 1 | 1285 | accepted | - |
| 3 | top_level_effort | low | 200 | stop | 91 | 1 | 1111 | accepted | - |
| 4 | top_level_effort | medium | 200 | stop | 84 | 1 | 1176 | accepted | - |
| 5 | top_level_effort | high | 200 | stop | 105 | 1 | 1515 | accepted | - |
| 6 | top_level_effort | xhigh | 200 | stop | 99 | 1 | 1494 | accepted | - |
| 7 | top_level_effort | max | 400 | - | - | - | 69 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 8 | top_level_effort | bogus-effort-51 | 400 | - | - | - | 88 | rejected | {"code":"invalid-argument","error":"Invalid reasoning effort."} |
| 9 | top_level_effort | high+temperature=0.5 | 200 | stop | 77 | 1 | 978 | accepted | - |

- `top_level_effort`: field parsed: yes; effect: low=91 max=-
- detector: field parsed (rejected)

## Gemini

NOT PROBED: no `--models` candidate for this provider

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable

# Live vendor effort probe

- date (UTC): 2026-10-22 14:36:42Z
- commit: 32ce9dd0
- credential variables found (names only, never values): OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, MOONSHOT_API_KEY, DASHSCOPE_API_KEY, XAI_API_KEY, GEMINI_API_KEY
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."; every call capped at 2048 output tokens (numbers and verdicts are recorded; response content is never printed)

## OpenAI

NOT PROBED: no `--models` candidate for this provider

## Anthropic

NOT PROBED: no `--models` candidate for this provider

## DeepSeek

NOT PROBED: no `--models` candidate for this provider

## Kimi

NOT PROBED: no `--models` candidate for this provider

## Qwen

NOT PROBED: no `--models` candidate for this provider

## Grok

NOT PROBED: no `--models` candidate for this provider

## Gemini

- base URL: `https://generativelanguage.googleapis.com/v1beta`
- chat endpoint: `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`; credential header `x-goog-api-key` (name only, value never printed)
- every call carries `generationConfig.maxOutputTokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed

### gemini-4-argon

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 34 | 7 | 32243 | accepted | - |
| 2 | thinking_level | minimal | 400 | - | - | - | 257 | rejected | {   "error": {     "code": 400,     "message": "Thinking level MINIMAL is not supported for this model. Please retry with other thinking level.",     "status": "INVALID_ARGUMENT"   } }  |
| 3 | thinking_level | low | 200 | STOP | - | 7 | 12202 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 227 | 7 | 29810 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 415 | 7 | 36762 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 293 | 7 | 45183 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | - | 7 | 73709 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 41 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 160 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | - | 7 | 31248 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | - | 7 | 6980 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 267 | 7 | 28194 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=- high=415
- `thinking_budget`: field parsed: yes; budget: 128=- 1=-

### gemini-3.8-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 245 | 3 | 3202 | accepted | - |
| 2 | thinking_level | minimal | 400 | - | - | - | 254 | rejected | {   "error": {     "code": 400,     "message": "Thinking level MINIMAL is not supported for this model. Please retry with other thinking level.",     "status": "INVALID_ARGUMENT"   } }  |
| 3 | thinking_level | low | 200 | STOP | 34 | 3 | 1660 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 235 | 3 | 2928 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 232 | 3 | 3328 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 350 | 3 | 4338 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | 206 | 3 | 2877 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 42 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 301 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 36 | 3 | 3082 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 48 | 3 | 2079 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 187 | 3 | 2991 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=34 high=232
- `thinking_budget`: field parsed: yes; budget: 128=36 1=48

### gemini-3.7-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 243 | 2 | 3545 | accepted | - |
| 2 | thinking_level | minimal | 400 | - | - | - | 271 | rejected | {   "error": {     "code": 400,     "message": "Thinking level MINIMAL is not supported for this model. Please retry with other thinking level.",     "status": "INVALID_ARGUMENT"   } }  |
| 3 | thinking_level | low | 200 | STOP | 60 | 3 | 2192 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 200 | 2 | 2731 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 324 | 3 | 3556 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 180 | 3 | 2789 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | 48 | 3 | 2354 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 42 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 150 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 103 | 3 | 2272 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 34 | 3 | 1362 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 270 | 3 | 4258 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=60 high=324
- `thinking_budget`: field parsed: yes; budget: 128=103 1=34

### gemini-3.1-pro-preview

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 424 | 3 | 3890 | accepted | - |
| 2 | thinking_level | minimal | 400 | - | - | - | 304 | rejected | {   "error": {     "code": 400,     "message": "Thinking level MINIMAL is not supported for this model. Please retry with other thinking level.",     "status": "INVALID_ARGUMENT"   } }  |
| 3 | thinking_level | low | 200 | STOP | 322 | 3 | 3171 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 981 | 3 | 9160 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 498 | 3 | 5642 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 446 | 3 | 4410 | accepted | - |
| 7 | thinking_budget | off | 400 | - | - | - | 191 | rejected | {   "error": {     "code": 400,     "message": "Budget 0 is invalid. This model only works in thinking mode.",     "status": "INVALID_ARGUMENT"   } }  |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 45 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 165 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 430 | 3 | 4221 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 595 | 3 | 5558 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 665 | 3 | 6486 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=322 high=498
- `thinking_budget`: field parsed: yes; budget: 128=430 1=595

### gemini-pro-latest

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 399 | 3 | 3968 | accepted | - |
| 2 | thinking_level | minimal | 400 | - | - | - | 234 | rejected | {   "error": {     "code": 400,     "message": "Thinking level MINIMAL is not supported for this model. Please retry with other thinking level.",     "status": "INVALID_ARGUMENT"   } }  |
| 3 | thinking_level | low | 200 | STOP | 355 | 3 | 3653 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 521 | 3 | 5103 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 406 | 2 | 3791 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 571 | 3 | 6035 | accepted | - |
| 7 | thinking_budget | off | 400 | - | - | - | 305 | rejected | {   "error": {     "code": 400,     "message": "Budget 0 is invalid. This model only works in thinking mode.",     "status": "INVALID_ARGUMENT"   } }  |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 40 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 150 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 339 | 3 | 3438 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 517 | 3 | 4762 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 510 | 3 | 4757 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=355 high=406
- `thinking_budget`: field parsed: yes; budget: 128=339 1=517

### gemini-flash-latest

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 181 | 2 | 3231 | accepted | - |
| 2 | thinking_level | minimal | 400 | - | - | - | 205 | rejected | {   "error": {     "code": 400,     "message": "Thinking level MINIMAL is not supported for this model. Please retry with other thinking level.",     "status": "INVALID_ARGUMENT"   } }  |
| 3 | thinking_level | low | 200 | STOP | 188 | 3 | 2546 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 198 | 3 | 3388 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 259 | 3 | 3282 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 245 | 3 | 4166 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | 109 | 3 | 2597 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 41 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 116 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 36 | 3 | 1966 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 37 | 3 | 2405 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 230 | 3 | 3443 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=188 high=259
- `thinking_budget`: field parsed: yes; budget: 128=36 1=37

### gemini-flash-lite-latest

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | - | 50 | 1335 | accepted | - |
| 2 | thinking_level | minimal | 200 | STOP | - | 50 | 903 | accepted | - |
| 3 | thinking_level | low | 200 | STOP | 163 | 3 | 1484 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 223 | 3 | 1634 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 217 | 3 | 1949 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 228 | 3 | 1567 | accepted | - |
| 7 | thinking_budget | off | 400 | - | - | - | 229 | rejected | {   "error": {     "code": 400,     "message": "Request contains an invalid argument.",     "status": "INVALID_ARGUMENT"   } }  |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 42 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 188 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 163 | 3 | 1819 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 161 | 3 | 1487 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 364 | 3 | 2297 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=163 high=217
- `thinking_budget`: field parsed: yes; budget: 128=163 1=161

### gemini-3.1-flash-lite

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | - | 3 | 1020 | accepted | - |
| 2 | thinking_level | minimal | 200 | STOP | - | 3 | 702 | accepted | - |
| 3 | thinking_level | low | 200 | STOP | 143 | 3 | 1079 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 280 | 3 | 1395 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 370 | 3 | 1822 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 361 | 3 | 1654 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | - | 3 | 697 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 41 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 120 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | - | 3 | 594 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | - | 3 | 994 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 676 | 56 | 2568 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=143 high=370
- `thinking_budget`: field parsed: yes; budget: 128=- 1=-

### gemini-3-flash-preview

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 278 | 3 | 1731 | accepted | - |
| 2 | thinking_level | minimal | 200 | STOP | - | 3 | 731 | accepted | - |
| 3 | thinking_level | low | 200 | STOP | 236 | 3 | 1734 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 315 | 3 | 1943 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 288 | 3 | 1801 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 244 | 3 | 1746 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | - | 3 | 756 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 42 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 114 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 226 | 3 | 1604 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | - | 3 | 804 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 280 | 3 | 1723 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=236 high=288
- `thinking_budget`: field parsed: yes; budget: 128=226 1=-

### gemini-3.5-flash-lite

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | - | 50 | 1600 | accepted | - |
| 2 | thinking_level | minimal | 200 | STOP | - | 50 | 1436 | accepted | - |
| 3 | thinking_level | low | 200 | STOP | 75 | 3 | 1611 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 209 | 3 | 1702 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 328 | 3 | 2046 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 440 | 3 | 2451 | accepted | - |
| 7 | thinking_budget | off | 400 | - | - | - | 212 | rejected | {   "error": {     "code": 400,     "message": "Request contains an invalid argument.",     "status": "INVALID_ARGUMENT"   } }  |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 41 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 108 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 80 | 3 | 2133 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | 162 | 3 | 1544 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 304 | 3 | 1892 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=75 high=328
- `thinking_budget`: field parsed: yes; budget: 128=80 1=162

### gemini-3.6-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 165 | 3 | 1804 | accepted | - |
| 2 | thinking_level | minimal | 200 | STOP | - | 3 | 1226 | accepted | - |
| 3 | thinking_level | low | 200 | STOP | 155 | 2 | 1480 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 286 | 3 | 2253 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 181 | 2 | 2256 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 206 | 3 | 1629 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | - | 3 | 727 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 41 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 149 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 186 | 3 | 1536 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | - | 3 | 773 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 172 | 3 | 1540 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=155 high=181
- `thinking_budget`: field parsed: yes; budget: 128=186 1=-

### gemini-3.5-flash

| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |
|---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|
| 1 | default | - | 200 | STOP | 234 | 3 | 2185 | accepted | - |
| 2 | thinking_level | minimal | 200 | STOP | - | 3 | 1017 | accepted | - |
| 3 | thinking_level | low | 200 | STOP | 181 | 2 | 1600 | accepted | - |
| 4 | thinking_level | medium | 200 | STOP | 167 | 3 | 1469 | accepted | - |
| 5 | thinking_level | high | 200 | STOP | 207 | 3 | 1545 | accepted | - |
| 6 | thinking_level | HIGH | 200 | STOP | 180 | 3 | 1528 | accepted | - |
| 7 | thinking_budget | off | 200 | STOP | - | 3 | 908 | accepted | - |
| 8 | thinking_level | bogus-effort-51 | 400 | - | - | - | 46 | rejected | {   "error": {     "code": 400,     "message": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), \"bogus-effort-51\"",     "status": "INVALID_ARGUMENT",     "details": [       {         "@type": "type.googleapis.com/google.rpc.BadRequest",         "fieldViolations": [           {             "field": "[REDACTED]",             "description": "Invalid value at '[REDACTED]' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConf... [truncated, 594 total bytes] |
| 9 | thinking_budget | -5 | 400 | - | - | - | 106 | rejected | {   "error": {     "code": 400,     "message": "* [REDACTED].thinking_budget: thinking_budget must be in the range [-1, 65535]\n",     "status": "INVALID_ARGUMENT"   } }  |
| 10 | thinking_budget | 128 | 200 | STOP | 155 | 2 | 2830 | accepted | - |
| 11 | thinking_budget | 1 | 200 | STOP | - | 3 | 1021 | accepted | - |
| 12 | thinking_level | high+temperature=0.5 | 200 | STOP | 218 | 3 | 1564 | accepted | - |

- `thinking_level`: field parsed: yes; effect: low=181 high=207
- `thinking_budget`: field parsed: yes; budget: 128=155 1=-

## Ollama

SKIPPED (OLLAMA_BASE_URL not set)

## OpenAI-compatible

SKIPPED (OPENAI_COMPATIBLE_API_KEY not set)

---

1 of 9 providers probed (8 not configured or without candidates); 0 unreachable or not buildable
