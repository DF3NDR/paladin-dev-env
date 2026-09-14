# `crates/paladin-web/tests/fixtures/` — provenance record

JSON has no comment syntax, so this sibling file carries the provenance for each frozen fixture
in this directory (SHIP-02, D-08).

## `openapi-v0.9.0.json`

- **Source tag:** `v0.9.0`
- **Tag commit:** `8b9bef8998d251beff949f73b026674f4c46c73d`
- **Producing command:** `git show v0.9.0:crates/paladin-web/openapi.json`
- **Blob SHA:** `f9d22f27f57f8da0957d8bc28c662a546ee6b0a6`

Re-derive and verify with:

```bash
git show v0.9.0:crates/paladin-web/openapi.json | git hash-object --stdin
git hash-object crates/paladin-web/tests/fixtures/openapi-v0.9.0.json
# both must print f9d22f27f57f8da0957d8bc28c662a546ee6b0a6
```

This file is a **historical record that is never regenerated**. It is the exact bytes of the
`v0.9.0` tag's `crates/paladin-web/openapi.json` blob — not a copy of the current
`crates/paladin-web/openapi.json`, and not produced by regenerating the spec at HEAD. This is why
`crates/paladin-web/tests/openapi_golden_v0_9.rs` deliberately offers no `UPDATE_OPENAPI`-style
regeneration escape hatch: there is nothing to regenerate a frozen historical baseline from.
