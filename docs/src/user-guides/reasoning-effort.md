# Reasoning Effort

This page explains how to tell a model how hard to think, what Paladin does when the model cannot
do what you asked, and exactly what each provider and model was measured to accept. The mapping
table below is checked against the built adapters by a test (`registry::reasoning_page_tests`), so
a row cannot say a model accepts an effort that its adapter refuses.

## Mapping table

The cells below are read from the adapters, not typed by hand. `exact` means the effort reaches the
vendor as asked. `refused` means the request fails before any HTTP call with
`LlmError::UnsupportedParameter`. `refused (coerced)` means the vendor accepts the value but
silently runs it as another, so Paladin refuses it under every fallback. `budget <min>-<max>` is
the span of reasoning-token budgets the model accepts. `verified on` is the date of the live probe
the row comes from; `unmeasured` means the model was never probed, so every explicit effort on it
is refused until an operator declares what it takes.

### Declared wire per provider

| Provider | Model | Wire field | Verified on |
|----------|-------|------------|-------------|
| deepseek | (declared wire) | `reasoning_effort` | 2026-10-22 |

### Effort per model

| Provider | Model | Off | Auto | Minimal | Low | Medium | High | XHigh | Max | Budget | Verified on |
|----------|-------|-----|------|---------|-----|--------|------|-------|-----|--------|-------------|
| deepseek | `deepseek-flash` | exact | exact | refused (coerced) | exact | refused (coerced) | exact | refused (coerced) | exact | refused | 2026-10-22 |
| deepseek | `deepseek-v4-pro` | exact | exact | refused (coerced) | exact | refused (coerced) | exact | refused (coerced) | exact | refused | 2026-10-22 |

DeepSeek documents that `minimal` runs as `low`, `medium` and `xhigh` as `high`, so those three are
refused rather than sent: a request for `medium` that quietly ran `high` would change cost and
latency without telling anyone.
