---
created: 2026-11-02T00:00:00Z
title: An unquoted treasurer price with 19 significant digits loses digits on load
area: config
severity: minor
files:
  - src/config/treasurer.rs
  - src/config/settings.rs
  - tests/config_example_roundtrip.rs
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 56 plan 56-02 (DF-06)
---

## Problem

`treasurer.pricing.<id>.prompt` (and the other price axes, and the allowance `amount` /
`lifetime`) are `String` fields documented as DECIMAL STRINGS. When an operator writes the value
unquoted in YAML, the `config` crate (0.15.23) reads it as a number and re-renders it as a string
on the way into the `String` field. Credible prices survive with the same decimal value
(`2.50` loads as `"2.5"`, `10.00` as `"10"`, `0.075` and `15` unchanged), but a value with 19
significant digits does not:

```
treasurer.pricing.probe.prompt: 1234567890.123456789  ->  loaded as "1234567890.1234567"
```

Measured once in plan 56-02 (not committed as a test). The cause is the number being held as an
`f64` between the YAML parse and the `String` field.

## Why it is parked (S4, DF-06)

No credible per-1M-token price has more than about 10 significant digits, and the example already
tells the operator to write prices as quoted decimal strings (`"2.50"`). A fix would be a custom
deserializer for the price fields, or a loader change, which is more than a contained docs or
example edit (D-05).

## When picked up

Either give the price and allowance string fields a deserializer that rejects an unquoted number
whose decimal value would change, or state in the `treasurer:` comment of `config.example.yml`
that an unquoted price is read as a number and must be quoted above 15 significant digits. The
quoted path is already pinned by `quoted_money_strings_arrive_byte_identical` and the credible
unquoted path by `unquoted_credible_prices_keep_their_decimal_value` in
`tests/config_example_roundtrip.rs`.

## Update 2026-11-05 (Phase 58 pre-tag audit, R-64)

Acknowledged and kept open by the Phase 58 pre-tag audit (`58-AUDIT-DISPOSITIONS.md` R-64). Owner: repo maintainer. Re-check by 2026-11-09, the maintainer's `DATE` recorded in `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`.
