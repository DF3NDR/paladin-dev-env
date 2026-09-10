# v0.9.0 configuration fixtures — provenance

**Phase 29, plan 29-01 (SHIP-02, D-06/D-07, RESEARCH.md Pitfall 1, Open Question 1).**

Both files in this directory are frozen, byte-identical copies of blobs committed at the
`v0.9.0` tag. Neither is hand-trimmed, hand-edited, or copied from the current working tree.
Each was produced by redirecting `git show <tag>:<path>` output straight to disk — never by
copying the tree's current file and relabelling it.

CI checks this repository out with `fetch-depth: 1` and no tags, so these fixtures are
committed files the boot test reads directly. Reading the tag at test time is not an option.

## Source

- Tag: `v0.9.0`
- Tag commit: `8b9bef8998d251beff949f73b026674f4c46c73d`

## `v0.9.0-config.test.yml`

- Producing command: `git show v0.9.0:config.test.yml > tests/fixtures/config/v0.9.0-config.test.yml`
- Blob SHA: `e63d9f93582e0e06f2a1b40530fac94e846ffe32`
- Re-verify with: `git show v0.9.0:config.test.yml | git hash-object --stdin` and
  `git hash-object tests/fixtures/config/v0.9.0-config.test.yml` — both must print
  `e63d9f93582e0e06f2a1b40530fac94e846ffe32`.
- **This is the file `tests/integration/v0_9_config_boot_test.rs` loads.** At HEAD,
  `config.test.yml` (root) is byte-identical to this fixture PLUS the `trace:` and
  `web_server:` sections Phase 28 added — confirmed by direct diff before freezing. It is
  therefore authentically v0.9-shaped: the closest thing to "the sample config a v0.9
  operator actually has" that also parses cleanly through `Settings::load_from_file` at v0.10
  HEAD, following the existing `test_load_from_file_regression` precedent
  (`tests/unit/settings_config_test.rs`).

## `v0.9.0-config.example.yml`

- Producing command: `git show v0.9.0:config.example.yml > tests/fixtures/config/v0.9.0-config.example.yml`
- Blob SHA: `fecb9bd94278612b7246cc5ff03839bf80cdd249`
- Re-verify with: `git show v0.9.0:config.example.yml | git hash-object --stdin` and
  `git hash-object tests/fixtures/config/v0.9.0-config.example.yml` — both must print
  `fecb9bd94278612b7246cc5ff03839bf80cdd249`.
- **Documentation evidence only — this file does NOT parse through
  `Settings::load_from_file`, at either the `v0.9.0` tag or v0.10 HEAD.**
  `LlmProviderConfig::api_key` (`crates/paladin-llm/src/config/llm.rs`) is a required
  `String`, and this file's `ollama:` block has never carried one, at either tag. This is a
  **pre-existing defect that predates Phase 29 and is outside this phase's scope** — not a
  Phase 29 regression, and not something this plan fixes (X-03: no behavioral change).
  `v0.9.0-config.test.yml` above is therefore the file the boot test actually loads; this
  file is kept only as verbatim evidence of the sample config a real v0.9.0 operator had in
  front of them.

## Why frozen fixtures rather than reading the tag at test time

CI checks out with `fetch-depth: 1` and no tags (RESEARCH.md, verified against
`.github/workflows/ci.yml`), so `git show v0.9.0:...` is not available inside a CI test run.
Committing the exact blob bytes, with the SHAs above re-derivable by anyone with a full
clone, is the only option that is both hermetic and checkable.
