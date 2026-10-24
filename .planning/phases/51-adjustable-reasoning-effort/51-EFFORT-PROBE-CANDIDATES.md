# Phase 51 effort-probe candidates (D-02)

Draft for the owner. D-01 makes the `live_vendor_smoke --effort` probe the only source of seeded effort data, and D-02
says which models it runs on: every seeded entry with `reasoning: true`. The planner drafted the list below (62 ids) from
that rule plus the additions research Pitfall 1 recommends, each marked. At plan 51-10's checkpoint the owner strikes
or adds ids and answers the three questions below; only then does the live probe run. Nothing in this file touched the
network: the dry-run section is the harness's own offline output.

## D-02 candidate ids (drafted by the planner; the owner strikes or adds at plan 51-10's checkpoint)

Rule (D-02): every seeded entry with `reasoning: true`. Research Pitfall 1 found that the seeded `reasoning` flag
records "the default call returned reasoning text", not "can reason": every OpenAI row and nine of eleven Anthropic
rows read `false` (Chat Completions never returns reasoning text; Anthropic's thinking `display` defaults to omitted),
including web3sec's `claude-opus-4-8`. Applied literally the rule would leave those models with no measured effort
data, so every explicit effort on them would refuse (D-03). The planner therefore adds them, marked "addition", for the
owner to keep or strike. Column "web3sec" marks the three models web3sec runs (CONTEXT Specific Ideas: must be probed).

| id | vendor | source | web3sec | note |
|---|---|---|---|---|
| `claude-haiku-5-5` | anthropic | addition | -- | seeded `reasoning: false` (display omitted) |
| `claude-opus-4-8` | anthropic | addition | yes | thinking default-off per vendor docs: pairing question |
| `claude-fable-5-1` | anthropic | addition | -- | -- |
| `claude-fable-5` | anthropic | rule | -- | -- |
| `claude-opus-5-5` | anthropic | addition | -- | -- |
| `claude-opus-5` | anthropic | rule | -- | -- |
| `claude-sonnet-5-5` | anthropic | addition | -- | -- |
| `claude-sonnet-5` | anthropic | addition | -- | -- |
| `claude-opus-4-7` | anthropic | addition | -- | thinking default-off per vendor docs |
| `claude-opus-4-6` | anthropic | addition | -- | the only generation documented to accept `budget_tokens` |
| `claude-sonnet-4-6` | anthropic | addition | -- | `budget_tokens` documented |
| `gpt-5.4-nano` | openai | addition | -- | the adapter's default model |
| `gpt-6.1-sol` | openai | addition | -- | -- |
| `gpt-6-sol` | openai | addition | -- | -- |
| `gpt-6-luna` | openai | addition | -- | -- |
| `gpt-6-astra` | openai | addition | -- | vendor docs: `none` is HTTP 400 |
| `gpt-5.6-sol` | openai | addition | -- | -- |
| `gpt-5.6-luna` | openai | addition | -- | -- |
| `gpt-5.6-terra` | openai | addition | -- | -- |
| `gpt-5.5` | openai | addition | -- | -- |
| `gpt-5.4` | openai | addition | -- | -- |
| `gpt-5.4-mini` | openai | addition | -- | -- |
| `gpt-5.2` | openai | addition | -- | -- |
| `gpt-5.1` | openai | addition | -- | announced shutdown 2027-04-01 |
| `gpt-5` | openai | addition | -- | -- |
| `gpt-5-mini` | openai | addition | -- | -- |
| `gpt-5-nano` | openai | addition | -- | -- |
| `o3` | openai | addition | -- | reasoning series |
| `deepseek-flash` | deepseek | rule | -- | the adapter's default model |
| `deepseek-v4-pro` | deepseek | rule | yes | -- |
| `kimi-k2.6` | kimi | rule | -- | both wire forms probed |
| `kimi-k3` | kimi | rule | yes | both wire forms probed (research Open Question 1) |
| `qwen3.8-flash` | qwen | rule | -- | -- |
| `qwen3.8-max` | qwen | rule | -- | -- |
| `qwen3.7-plus` | qwen | rule | -- | -- |
| `qwen3.7-max` | qwen | rule | -- | -- |
| `qwen3.7-flash` | qwen | rule | -- | -- |
| `qwen3.6-plus` | qwen | rule | -- | -- |
| `qwen3.6-flash` | qwen | rule | -- | -- |
| `qwen3.5-plus` | qwen | rule | -- | -- |
| `qwen3.5-flash` | qwen | rule | -- | -- |
| `qwen-plus` | qwen | addition | -- | possible hybrid thinking model |
| `qwen-max` | qwen | addition | -- | possible hybrid thinking model |
| `qwen-flash` | qwen | addition | -- | possible hybrid thinking model |
| `qwen-plus-latest` | qwen | addition | -- | moving alias |
| `grok-4.3` | grok | rule | -- | not in xAI's effort docs |
| `grok-4.6` | grok | rule | -- | -- |
| `grok-4.5` | grok | rule | -- | vendor docs: `xhigh` treated as `high` |
| `grok-4.20-reasoning` | grok | rule | -- | not in xAI's effort docs |
| `grok-4.7` | grok | rule | -- | -- |
| `gemini-4-argon` | gemini | rule | -- | -- |
| `gemini-3.8-flash` | gemini | rule | -- | -- |
| `gemini-3.7-flash` | gemini | rule | -- | -- |
| `gemini-3.1-pro-preview` | gemini | rule | -- | preview |
| `gemini-pro-latest` | gemini | rule | -- | moving alias |
| `gemini-flash-latest` | gemini | rule | -- | moving alias |
| `gemini-flash-lite-latest` | gemini | rule | -- | moving alias |
| `gemini-3.1-flash-lite` | gemini | rule | -- | -- |
| `gemini-3-flash-preview` | gemini | rule | -- | preview |
| `gemini-3.5-flash-lite` | gemini | addition | -- | seeded `reasoning: false` |
| `gemini-3.6-flash` | gemini | addition | -- | seeded `reasoning: false` |
| `gemini-3.5-flash` | gemini | addition | -- | seeded `reasoning: false` |

Excluded by the planner (the owner may add any back; probing one measures its "does not reason" state):
- `gpt-4o-mini` (openai): a non-reasoning model family.
- `chat-latest` (openai): a moving alias whose effort support is true only on its probe date.
- `grok-4.20-non-reasoning` (grok): non-reasoning by name.
- Not seeded, so not candidates under D-02 (the owner may add): Gemini 2.5 ids (the documented `thinkingBudget`
  generation), `kimi-k2.7-code*`.
- Not probed at all: Ollama and OpenAI-compatible (no daemon or endpoint configured, D-02), unless the operator sets
  `OLLAMA_BASE_URL` / `OPENAI_COMPATIBLE_BASE_URL` for a D-10 check.


## Owner questions (answered at plan 51-10)

**Q1. Keep or strike the "addition" rows?** The rule-only list (seeded `reasoning: true`) leaves every OpenAI id and nine
of eleven Anthropic ids unprobed, including web3sec's `claude-opus-4-8`. Strike any addition you do not want to pay for;
add back any planner exclusion above.

**Q2. How is a seeded `reasoning: false` entry that is NOT probed treated?** Research Pitfall 1: the seeded flag records
"the default call returned reasoning text", not "can reason" (Chat Completions never returns reasoning text; Anthropic's
thinking `display` defaults to omitted).

- **(a) D-02 as written:** the "does not reason" state, `EffortSupport::NoReasoning`. `Off` is an exact no-op and every
  other level refuses. On any such model that does in fact reason, `Off` becomes a silent no-op, which is the failure
  mode this phase exists to prevent.
- **(b) Left `EffortSupport::Unmeasured`:** every explicit effort, `Off` included, refuses naming the
  `llm.<provider>.models.<model-id>.effort` override. Nothing is claimed that was not measured.

**Q3. Where does the probe run?** In this container on the keys already configured, after your go-ahead (spend estimate
below), or on a machine of the operator's choosing with the same command.

## Commands

Per vendor, the `--models` flag built from the table (the effort modes probe only the ids passed here; there is no
curated default):

| vendor | models | flag |
|---|---|---|
| anthropic | 11 | `--models anthropic=claude-haiku-5-5,claude-opus-4-8,claude-fable-5-1,claude-fable-5,claude-opus-5-5,claude-opus-5,claude-sonnet-5-5,claude-sonnet-5,claude-opus-4-7,claude-opus-4-6,claude-sonnet-4-6` |
| openai | 17 | `--models openai=gpt-5.4-nano,gpt-6.1-sol,gpt-6-sol,gpt-6-luna,gpt-6-astra,gpt-5.6-sol,gpt-5.6-luna,gpt-5.6-terra,gpt-5.5,gpt-5.4,gpt-5.4-mini,gpt-5.2,gpt-5.1,gpt-5,gpt-5-mini,gpt-5-nano,o3` |
| deepseek | 2 | `--models deepseek=deepseek-flash,deepseek-v4-pro` |
| kimi | 2 | `--models kimi=kimi-k2.6,kimi-k3` |
| qwen | 13 | `--models qwen=qwen3.8-flash,qwen3.8-max,qwen3.7-plus,qwen3.7-max,qwen3.7-flash,qwen3.6-plus,qwen3.6-flash,qwen3.5-plus,qwen3.5-flash,qwen-plus,qwen-max,qwen-flash,qwen-plus-latest` |
| grok | 5 | `--models grok=grok-4.3,grok-4.6,grok-4.5,grok-4.20-reasoning,grok-4.7` |
| gemini | 12 | `--models gemini=gemini-4-argon,gemini-3.8-flash,gemini-3.7-flash,gemini-3.1-pro-preview,gemini-pro-latest,gemini-flash-latest,gemini-flash-lite-latest,gemini-3.1-flash-lite,gemini-3-flash-preview,gemini-3.5-flash-lite,gemini-3.6-flash,gemini-3.5-flash` |

One combined live command (writes the raw transcript; the scrubbed transcript is `51-EFFORT-PROBE.md`, plan 51-10):

```bash
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models anthropic=claude-haiku-5-5,claude-opus-4-8,claude-fable-5-1,claude-fable-5,claude-opus-5-5,claude-opus-5,claude-sonnet-5-5,claude-sonnet-5,claude-opus-4-7,claude-opus-4-6,claude-sonnet-4-6 --models openai=gpt-5.4-nano,gpt-6.1-sol,gpt-6-sol,gpt-6-luna,gpt-6-astra,gpt-5.6-sol,gpt-5.6-luna,gpt-5.6-terra,gpt-5.5,gpt-5.4,gpt-5.4-mini,gpt-5.2,gpt-5.1,gpt-5,gpt-5-mini,gpt-5-nano,o3 --models deepseek=deepseek-flash,deepseek-v4-pro --models kimi=kimi-k2.6,kimi-k3 --models qwen=qwen3.8-flash,qwen3.8-max,qwen3.7-plus,qwen3.7-max,qwen3.7-flash,qwen3.6-plus,qwen3.6-flash,qwen3.5-plus,qwen3.5-flash,qwen-plus,qwen-max,qwen-flash,qwen-plus-latest --models grok=grok-4.3,grok-4.6,grok-4.5,grok-4.20-reasoning,grok-4.7 --models gemini=gemini-4-argon,gemini-3.8-flash,gemini-3.7-flash,gemini-3.1-pro-preview,gemini-pro-latest,gemini-flash-latest,gemini-flash-lite-latest,gemini-3.1-flash-lite,gemini-3-flash-preview,gemini-3.5-flash-lite,gemini-3.6-flash,gemini-3.5-flash > .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
```

The per-vendor alternative, one vendor at a time, appended in vendor order (the first run truncates, the rest append):

```bash
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models anthropic=claude-haiku-5-5,claude-opus-4-8,claude-fable-5-1,claude-fable-5,claude-opus-5-5,claude-opus-5,claude-sonnet-5-5,claude-sonnet-5,claude-opus-4-7,claude-opus-4-6,claude-sonnet-4-6 > .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models openai=gpt-5.4-nano,gpt-6.1-sol,gpt-6-sol,gpt-6-luna,gpt-6-astra,gpt-5.6-sol,gpt-5.6-luna,gpt-5.6-terra,gpt-5.5,gpt-5.4,gpt-5.4-mini,gpt-5.2,gpt-5.1,gpt-5,gpt-5-mini,gpt-5-nano,o3 >> .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models deepseek=deepseek-flash,deepseek-v4-pro >> .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models kimi=kimi-k2.6,kimi-k3 >> .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models qwen=qwen3.8-flash,qwen3.8-max,qwen3.7-plus,qwen3.7-max,qwen3.7-flash,qwen3.6-plus,qwen3.6-flash,qwen3.5-plus,qwen3.5-flash,qwen-plus,qwen-max,qwen-flash,qwen-plus-latest >> .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models grok=grok-4.3,grok-4.6,grok-4.5,grok-4.20-reasoning,grok-4.7 >> .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --models gemini=gemini-4-argon,gemini-3.8-flash,gemini-3.7-flash,gemini-3.1-pro-preview,gemini-pro-latest,gemini-flash-latest,gemini-flash-lite-latest,gemini-3.1-flash-lite,gemini-3-flash-preview,gemini-3.5-flash-lite,gemini-3.6-flash,gemini-3.5-flash >> .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md
```

Adapter-path smoke of the shipped adapters (plan 51-14, after the adapters carry effort data), e.g. one model:

```bash
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort-smoke --models deepseek=deepseek-v4-pro --max-tokens 2048
```

## Spend estimate

Calls per model per vendor are read from the harness's own plan (below). Every call is capped at 2,048 output tokens and
a 120 s client timeout; `finish = length` is recorded as data. Many calls end early (a rejected invalid value costs no
output), so the token figure is a ceiling, not a forecast.

| vendor | models | calls per model | calls | worst-case output tokens |
|---|---|---|---|---|
| anthropic | 11 | 17 | 187 | 382,976 |
| openai | 17 | 10 | 170 | 348,160 |
| deepseek | 2 | 12 | 24 | 49,152 |
| kimi | 2 | 18 | 36 | 73,728 |
| qwen | 13 | 8 | 104 | 212,992 |
| grok | 5 | 9 | 45 | 92,160 |
| gemini | 12 | 12 | 144 | 294,912 |
| **total** | **62** | | **710** | **1,454,080** |

About 710 raw calls (the planner's estimate in the plan was about 760). Worst-case wall time is calls x 120 s;
run one vendor at a time if the 8-minute `max` runs recorded in the handoff (Kimi, before the cap) are a concern.
Ollama and OpenAI-compatible are not in the table (no daemon or endpoint configured); set `OLLAMA_BASE_URL` or
`OPENAI_COMPATIBLE_BASE_URL` and pass `--models` for a D-10 check.

## Dry-run plan (offline)

The full stdout of the combined command with `--dry-run` added, run in the plan 51-02 worktree with every vendor key
(`MOONSHOT_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY`, `DASHSCOPE_API_KEY`, `XAI_API_KEY`, `GEMINI_API_KEY`) removed from the environment. It exited 0, made no network call and read no credential.

```text
cargo run -q -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort --dry-run --models anthropic=claude-haiku-5-5,claude-opus-4-8,claude-fable-5-1,claude-fable-5,claude-opus-5-5,claude-opus-5,claude-sonnet-5-5,claude-sonnet-5,claude-opus-4-7,claude-opus-4-6,claude-sonnet-4-6 --models openai=gpt-5.4-nano,gpt-6.1-sol,gpt-6-sol,gpt-6-luna,gpt-6-astra,gpt-5.6-sol,gpt-5.6-luna,gpt-5.6-terra,gpt-5.5,gpt-5.4,gpt-5.4-mini,gpt-5.2,gpt-5.1,gpt-5,gpt-5-mini,gpt-5-nano,o3 --models deepseek=deepseek-flash,deepseek-v4-pro --models kimi=kimi-k2.6,kimi-k3 --models qwen=qwen3.8-flash,qwen3.8-max,qwen3.7-plus,qwen3.7-max,qwen3.7-flash,qwen3.6-plus,qwen3.6-flash,qwen3.5-plus,qwen3.5-flash,qwen-plus,qwen-max,qwen-flash,qwen-plus-latest --models grok=grok-4.3,grok-4.6,grok-4.5,grok-4.20-reasoning,grok-4.7 --models gemini=gemini-4-argon,gemini-3.8-flash,gemini-3.7-flash,gemini-3.1-pro-preview,gemini-pro-latest,gemini-flash-latest,gemini-flash-lite-latest,gemini-3.1-flash-lite,gemini-3-flash-preview,gemini-3.5-flash-lite,gemini-3.6-flash,gemini-3.5-flash
```

```markdown
# Live vendor effort probe plan (--effort --dry-run)

- no network access was made and no credential was read
- prompt: "What is 17 multiplied by 23? Work it out, then answer with the number only."
- invalid value: `bogus-effort-51`; output cap 2048 tokens per call

## OpenAI

- base URL: `https://api.openai.com/v1`
- chat endpoint: `POST https://api.openai.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_completion_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `OPENAI_API_KEY`

### Candidate models

- `gpt-5.4-nano`: 10 calls
- `gpt-6.1-sol`: 10 calls
- `gpt-6-sol`: 10 calls
- `gpt-6-luna`: 10 calls
- `gpt-6-astra`: 10 calls
- `gpt-5.6-sol`: 10 calls
- `gpt-5.6-luna`: 10 calls
- `gpt-5.6-terra`: 10 calls
- `gpt-5.5`: 10 calls
- `gpt-5.4`: 10 calls
- `gpt-5.4-mini`: 10 calls
- `gpt-5.2`: 10 calls
- `gpt-5.1`: 10 calls
- `gpt-5`: 10 calls
- `gpt-5-mini`: 10 calls
- `gpt-5-nano`: 10 calls
- `o3`: 10 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | top_level_effort | none | Off | `{"reasoning_effort":"none"}` |
| 3 | top_level_effort | minimal | Level | `{"reasoning_effort":"minimal"}` |
| 4 | top_level_effort | low | Level | `{"reasoning_effort":"low"}` |
| 5 | top_level_effort | medium | Level | `{"reasoning_effort":"medium"}` |
| 6 | top_level_effort | high | Level | `{"reasoning_effort":"high"}` |
| 7 | top_level_effort | xhigh | Level | `{"reasoning_effort":"xhigh"}` |
| 8 | top_level_effort | max | Level | `{"reasoning_effort":"max"}` |
| 9 | top_level_effort | bogus-effort-51 | Invalid | `{"reasoning_effort":"bogus-effort-51"}` |
| 10 | top_level_effort | high+temperature=0.5 | Sampling | `{"reasoning_effort":"high","temperature":0.5}` |

## Anthropic

- base URL: `https://api.anthropic.com/v1`
- chat endpoint: `POST https://api.anthropic.com/v1/messages`; credential header `x-api-key` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `ANTHROPIC_API_KEY`

### Candidate models

- `claude-haiku-5-5`: 17 calls
- `claude-opus-4-8`: 17 calls
- `claude-fable-5-1`: 17 calls
- `claude-fable-5`: 17 calls
- `claude-opus-5-5`: 17 calls
- `claude-opus-5`: 17 calls
- `claude-sonnet-5-5`: 17 calls
- `claude-sonnet-5`: 17 calls
- `claude-opus-4-7`: 17 calls
- `claude-opus-4-6`: 17 calls
- `claude-sonnet-4-6`: 17 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | output_config | low | Level | `{"output_config":{"effort":"low"}}` |
| 3 | output_config | medium | Level | `{"output_config":{"effort":"medium"}}` |
| 4 | output_config | high | Level | `{"output_config":{"effort":"high"}}` |
| 5 | output_config | xhigh | Level | `{"output_config":{"effort":"xhigh"}}` |
| 6 | output_config | max | Level | `{"output_config":{"effort":"max"}}` |
| 7 | output_config+adaptive | low | Level | `{"output_config":{"effort":"low"},"thinking":{"type":"adaptive"}}` |
| 8 | output_config+adaptive | medium | Level | `{"output_config":{"effort":"medium"},"thinking":{"type":"adaptive"}}` |
| 9 | output_config+adaptive | high | Level | `{"output_config":{"effort":"high"},"thinking":{"type":"adaptive"}}` |
| 10 | output_config+adaptive | xhigh | Level | `{"output_config":{"effort":"xhigh"},"thinking":{"type":"adaptive"}}` |
| 11 | output_config+adaptive | max | Level | `{"output_config":{"effort":"max"},"thinking":{"type":"adaptive"}}` |
| 12 | thinking_object | off | Off | `{"thinking":{"type":"disabled"}}` |
| 13 | thinking_object | auto | Auto | `{"thinking":{"type":"adaptive"}}` |
| 14 | output_config | bogus-effort-51 | Invalid | `{"output_config":{"effort":"bogus-effort-51"}}` |
| 15 | thinking_budget | 1024 | Budget | `{"thinking":{"budget_tokens":1024,"type":"enabled"}}` |
| 16 | thinking_budget | 1023 | Budget | `{"thinking":{"budget_tokens":1023,"type":"enabled"}}` |
| 17 | output_config | high+temperature=0.5 | Sampling | `{"output_config":{"effort":"high"},"temperature":0.5}` |

## DeepSeek

- base URL: `https://api.deepseek.com/v1`
- chat endpoint: `POST https://api.deepseek.com/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `DEEPSEEK_API_KEY`

### Candidate models

- `deepseek-flash`: 12 calls
- `deepseek-v4-pro`: 12 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | top_level_effort | minimal | Level | `{"reasoning_effort":"minimal"}` |
| 3 | top_level_effort | low | Level | `{"reasoning_effort":"low"}` |
| 4 | top_level_effort | medium | Level | `{"reasoning_effort":"medium"}` |
| 5 | top_level_effort | high | Level | `{"reasoning_effort":"high"}` |
| 6 | top_level_effort | xhigh | Level | `{"reasoning_effort":"xhigh"}` |
| 7 | top_level_effort | max | Level | `{"reasoning_effort":"max"}` |
| 8 | thinking_object | off | Off | `{"thinking":{"type":"disabled"}}` |
| 9 | thinking_object | auto | Auto | `{"thinking":{"type":"enabled"}}` |
| 10 | top_level_effort | bogus-effort-51 | Invalid | `{"reasoning_effort":"bogus-effort-51"}` |
| 11 | thinking_object | bogus-effort-51 | Invalid | `{"thinking":{"type":"bogus-effort-51"}}` |
| 12 | top_level_effort | high+temperature=0.5 | Sampling | `{"reasoning_effort":"high","temperature":0.5}` |

## Kimi

- base URL: `https://api.moonshot.ai/v1`
- chat endpoint: `POST https://api.moonshot.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `MOONSHOT_API_KEY`

### Candidate models

- `kimi-k2.6`: 18 calls
- `kimi-k3`: 18 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | top_level_effort | minimal | Level | `{"reasoning_effort":"minimal"}` |
| 3 | thinking_object | minimal | Level | `{"thinking":{"effort":"minimal","type":"enabled"}}` |
| 4 | top_level_effort | low | Level | `{"reasoning_effort":"low"}` |
| 5 | thinking_object | low | Level | `{"thinking":{"effort":"low","type":"enabled"}}` |
| 6 | top_level_effort | medium | Level | `{"reasoning_effort":"medium"}` |
| 7 | thinking_object | medium | Level | `{"thinking":{"effort":"medium","type":"enabled"}}` |
| 8 | top_level_effort | high | Level | `{"reasoning_effort":"high"}` |
| 9 | thinking_object | high | Level | `{"thinking":{"effort":"high","type":"enabled"}}` |
| 10 | top_level_effort | xhigh | Level | `{"reasoning_effort":"xhigh"}` |
| 11 | thinking_object | xhigh | Level | `{"thinking":{"effort":"xhigh","type":"enabled"}}` |
| 12 | top_level_effort | max | Level | `{"reasoning_effort":"max"}` |
| 13 | thinking_object | max | Level | `{"thinking":{"effort":"max","type":"enabled"}}` |
| 14 | thinking_object | off | Off | `{"thinking":{"type":"disabled"}}` |
| 15 | thinking_object | auto | Auto | `{"thinking":{"type":"enabled"}}` |
| 16 | top_level_effort | bogus-effort-51 | Invalid | `{"reasoning_effort":"bogus-effort-51"}` |
| 17 | thinking_object | bogus-effort-51 | Invalid | `{"thinking":{"effort":"bogus-effort-51","type":"enabled"}}` |
| 18 | thinking_object | high+temperature=0.5 | Sampling | `{"temperature":0.5,"thinking":{"effort":"high","type":"enabled"}}` |

## Qwen

- base URL: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`
- chat endpoint: `POST https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `DASHSCOPE_API_KEY`

### Candidate models

- `qwen3.8-flash`: 8 calls
- `qwen3.8-max`: 8 calls
- `qwen3.7-plus`: 8 calls
- `qwen3.7-max`: 8 calls
- `qwen3.7-flash`: 8 calls
- `qwen3.6-plus`: 8 calls
- `qwen3.6-flash`: 8 calls
- `qwen3.5-plus`: 8 calls
- `qwen3.5-flash`: 8 calls
- `qwen-plus`: 8 calls
- `qwen-max`: 8 calls
- `qwen-flash`: 8 calls
- `qwen-plus-latest`: 8 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | enable_thinking | off | Off | `{"enable_thinking":false}` |
| 3 | enable_thinking | auto | Auto | `{"enable_thinking":true}` |
| 4 | thinking_budget | 1 | Budget | `{"enable_thinking":true,"thinking_budget":1}` |
| 5 | thinking_budget | 0 | Budget | `{"enable_thinking":true,"thinking_budget":0}` |
| 6 | thinking_budget | -1 | Invalid | `{"enable_thinking":true,"thinking_budget":-1}` |
| 7 | detector | bogus-effort-51 | Detector | `{"reasoning_effort":"bogus-effort-51"}` |
| 8 | enable_thinking | auto+temperature=0.5 | Sampling | `{"enable_thinking":true,"temperature":0.5}` |

## Grok

- base URL: `https://api.x.ai/v1`
- chat endpoint: `POST https://api.x.ai/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `XAI_API_KEY`

### Candidate models

- `grok-4.3`: 9 calls
- `grok-4.6`: 9 calls
- `grok-4.5`: 9 calls
- `grok-4.20-reasoning`: 9 calls
- `grok-4.7`: 9 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | top_level_effort | minimal | Level | `{"reasoning_effort":"minimal"}` |
| 3 | top_level_effort | low | Level | `{"reasoning_effort":"low"}` |
| 4 | top_level_effort | medium | Level | `{"reasoning_effort":"medium"}` |
| 5 | top_level_effort | high | Level | `{"reasoning_effort":"high"}` |
| 6 | top_level_effort | xhigh | Level | `{"reasoning_effort":"xhigh"}` |
| 7 | top_level_effort | max | Level | `{"reasoning_effort":"max"}` |
| 8 | top_level_effort | bogus-effort-51 | Invalid | `{"reasoning_effort":"bogus-effort-51"}` |
| 9 | top_level_effort | high+temperature=0.5 | Sampling | `{"reasoning_effort":"high","temperature":0.5}` |

## Gemini

- base URL: `https://generativelanguage.googleapis.com/v1beta`
- chat endpoint: `POST https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent`; credential header `x-goog-api-key` (name only, value never printed)
- every call carries `generationConfig.maxOutputTokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `GEMINI_API_KEY`

### Candidate models

- `gemini-4-argon`: 12 calls
- `gemini-3.8-flash`: 12 calls
- `gemini-3.7-flash`: 12 calls
- `gemini-3.1-pro-preview`: 12 calls
- `gemini-pro-latest`: 12 calls
- `gemini-flash-latest`: 12 calls
- `gemini-flash-lite-latest`: 12 calls
- `gemini-3.1-flash-lite`: 12 calls
- `gemini-3-flash-preview`: 12 calls
- `gemini-3.5-flash-lite`: 12 calls
- `gemini-3.6-flash`: 12 calls
- `gemini-3.5-flash`: 12 calls

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | thinking_level | minimal | Level | `{"generationConfig":{"thinkingConfig":{"thinkingLevel":"minimal"}}}` |
| 3 | thinking_level | low | Level | `{"generationConfig":{"thinkingConfig":{"thinkingLevel":"low"}}}` |
| 4 | thinking_level | medium | Level | `{"generationConfig":{"thinkingConfig":{"thinkingLevel":"medium"}}}` |
| 5 | thinking_level | high | Level | `{"generationConfig":{"thinkingConfig":{"thinkingLevel":"high"}}}` |
| 6 | thinking_level | HIGH | Level | `{"generationConfig":{"thinkingConfig":{"thinkingLevel":"HIGH"}}}` |
| 7 | thinking_budget | off | Off | `{"generationConfig":{"thinkingConfig":{"thinkingBudget":0}}}` |
| 8 | thinking_level | bogus-effort-51 | Invalid | `{"generationConfig":{"thinkingConfig":{"thinkingLevel":"bogus-effort-51"}}}` |
| 9 | thinking_budget | -5 | Invalid | `{"generationConfig":{"thinkingConfig":{"thinkingBudget":-5}}}` |
| 10 | thinking_budget | 128 | Budget | `{"generationConfig":{"thinkingConfig":{"thinkingBudget":128}}}` |
| 11 | thinking_budget | 1 | Budget | `{"generationConfig":{"thinkingConfig":{"thinkingBudget":1}}}` |
| 12 | thinking_level | high+temperature=0.5 | Sampling | `{"generationConfig":{"temperature":0.5,"thinkingConfig":{"thinkingLevel":"high"}}}` |

## Ollama

- base URL: `http://localhost:11434/v1`
- chat endpoint: `POST http://localhost:11434/v1/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `OLLAMA_BASE_URL`

### Candidate models

- none (pass `--models ollama=<id>[,<id>...]`; the effort modes probe no default)

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | top_level_effort | minimal | Level | `{"reasoning_effort":"minimal"}` |
| 3 | thinking_object | minimal | Level | `{"thinking":{"effort":"minimal","type":"enabled"}}` |
| 4 | top_level_effort | low | Level | `{"reasoning_effort":"low"}` |
| 5 | thinking_object | low | Level | `{"thinking":{"effort":"low","type":"enabled"}}` |
| 6 | top_level_effort | medium | Level | `{"reasoning_effort":"medium"}` |
| 7 | thinking_object | medium | Level | `{"thinking":{"effort":"medium","type":"enabled"}}` |
| 8 | top_level_effort | high | Level | `{"reasoning_effort":"high"}` |
| 9 | thinking_object | high | Level | `{"thinking":{"effort":"high","type":"enabled"}}` |
| 10 | top_level_effort | xhigh | Level | `{"reasoning_effort":"xhigh"}` |
| 11 | thinking_object | xhigh | Level | `{"thinking":{"effort":"xhigh","type":"enabled"}}` |
| 12 | top_level_effort | max | Level | `{"reasoning_effort":"max"}` |
| 13 | thinking_object | max | Level | `{"thinking":{"effort":"max","type":"enabled"}}` |
| 14 | thinking_object | off | Off | `{"thinking":{"type":"disabled"}}` |
| 15 | thinking_object | auto | Auto | `{"thinking":{"type":"enabled"}}` |
| 16 | top_level_effort | bogus-effort-51 | Invalid | `{"reasoning_effort":"bogus-effort-51"}` |
| 17 | thinking_object | bogus-effort-51 | Invalid | `{"thinking":{"effort":"bogus-effort-51","type":"enabled"}}` |
| 18 | enable_thinking | off | Off | `{"enable_thinking":false}` |
| 19 | enable_thinking | auto | Auto | `{"enable_thinking":true}` |
| 20 | thinking_budget | 1024 | Budget | `{"enable_thinking":true,"thinking_budget":1024}` |
| 21 | thinking_budget | -1 | Invalid | `{"enable_thinking":true,"thinking_budget":-1}` |
| 22 | top_level_effort | high+temperature=0.5 | Sampling | `{"reasoning_effort":"high","temperature":0.5}` |

## OpenAI-compatible

- base URL: `<OPENAI_COMPATIBLE_BASE_URL>`
- chat endpoint: `POST <OPENAI_COMPATIBLE_BASE_URL>/chat/completions`; credential header `Authorization` (name only, value never printed)
- every call carries `max_tokens` = 2048, a 120 s timeout, no acceptance extra; redirects are never followed
- credential variables required: `OPENAI_COMPATIBLE_API_KEY`, `OPENAI_COMPATIBLE_BASE_URL`

### Candidate models

- none (pass `--models openai-compatible=<id>[,<id>...]`; the effort modes probe no default)

### Planned calls (run once per candidate model)

| # | form | value | role | body extras |
|---|------|-------|------|-------------|
| 1 | default | - | Default | - |
| 2 | top_level_effort | minimal | Level | `{"reasoning_effort":"minimal"}` |
| 3 | thinking_object | minimal | Level | `{"thinking":{"effort":"minimal","type":"enabled"}}` |
| 4 | top_level_effort | low | Level | `{"reasoning_effort":"low"}` |
| 5 | thinking_object | low | Level | `{"thinking":{"effort":"low","type":"enabled"}}` |
| 6 | top_level_effort | medium | Level | `{"reasoning_effort":"medium"}` |
| 7 | thinking_object | medium | Level | `{"thinking":{"effort":"medium","type":"enabled"}}` |
| 8 | top_level_effort | high | Level | `{"reasoning_effort":"high"}` |
| 9 | thinking_object | high | Level | `{"thinking":{"effort":"high","type":"enabled"}}` |
| 10 | top_level_effort | xhigh | Level | `{"reasoning_effort":"xhigh"}` |
| 11 | thinking_object | xhigh | Level | `{"thinking":{"effort":"xhigh","type":"enabled"}}` |
| 12 | top_level_effort | max | Level | `{"reasoning_effort":"max"}` |
| 13 | thinking_object | max | Level | `{"thinking":{"effort":"max","type":"enabled"}}` |
| 14 | thinking_object | off | Off | `{"thinking":{"type":"disabled"}}` |
| 15 | thinking_object | auto | Auto | `{"thinking":{"type":"enabled"}}` |
| 16 | top_level_effort | bogus-effort-51 | Invalid | `{"reasoning_effort":"bogus-effort-51"}` |
| 17 | thinking_object | bogus-effort-51 | Invalid | `{"thinking":{"effort":"bogus-effort-51","type":"enabled"}}` |
| 18 | enable_thinking | off | Off | `{"enable_thinking":false}` |
| 19 | enable_thinking | auto | Auto | `{"enable_thinking":true}` |
| 20 | thinking_budget | 1024 | Budget | `{"enable_thinking":true,"thinking_budget":1024}` |
| 21 | thinking_budget | -1 | Invalid | `{"enable_thinking":true,"thinking_budget":-1}` |
| 22 | top_level_effort | high+temperature=0.5 | Sampling | `{"reasoning_effort":"high","temperature":0.5}` |

- total planned calls: 710
```

## Final candidate set

Recorded on 2026-10-22, before any live call, from the owner's answers at plan 51-10's checkpoint (Task 1). The
owner's words, verbatim:

- Option: "run here." (option `run-here`; this is also the go-ahead to spend.)
- Candidate set: confirmed as drafted. "that should be the ones that have been previously probed ... the models that
  are the currently active latest models available". Struck ids: none. Added ids: none. (The orchestrator checked the
  drafted table against the seeded registry's model ids before dispatch and found them identical.)
- D-02 treatment of an unprobed seeded `reasoning: false` entry: "D02 unmeasured" (`EffortSupport::Unmeasured`, the
  research Pitfall 1 alternative, not `NoReasoning`).

Delta against the draft above: drafted 62, struck 0, added 0, final 62. Every id in the draft table is probed, the
planner's "addition" rows included (all of Anthropic's and OpenAI's ids that were not seeded `reasoning: true`).

| vendor | drafted | struck | added | final | calls per model | calls |
|---|---|---|---|---|---|---|
| anthropic | 11 | 0 | 0 | 11 | 17 | 187 |
| openai | 17 | 0 | 0 | 17 | 10 | 170 |
| deepseek | 2 | 0 | 0 | 2 | 12 | 24 |
| kimi | 2 | 0 | 0 | 2 | 18 | 36 |
| qwen | 13 | 0 | 0 | 13 | 8 | 104 |
| grok | 5 | 0 | 0 | 5 | 9 | 45 |
| gemini | 12 | 0 | 0 | 12 | 12 | 144 |
| **total** | **62** | **0** | **0** | **62** | | **710** |

The per-vendor `--models` flags are exactly those in the Commands table above. Run order: `kimi=kimi-k3` alone first
(both wire forms), then OpenAI, Anthropic, DeepSeek, Kimi (both ids), Qwen, Grok, Gemini, one vendor at a time.
Ollama and OpenAI-compatible are not probed: no daemon or endpoint is configured in this environment.
