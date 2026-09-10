# Changelog

All notable changes to `paladin-eval` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

## [0.10.0] - 2026-09-10

### Added
- Initial release: the `.eval.yaml` scenario file format (`schema_version: "1"`,
  `graph_doc`/`registered` targets, scripted `llm` sequences and `match` rules,
  `interrupt_after_superstep`/`parley_responses`), golden-checked against
  `docs/schemas/eval-scenario.schema.json` (Phase 28, D-27/D-28).
- `ScenarioLlm`, a scripted `LlmPort` implementation with per-node routing,
  sequence-per-call scripts, prompt-substring match rules and request capture
  for failure rendering (D-30).
- The twelve OBS-FR-12 assertion evaluators (`final_state_field_equals`,
  `final_state_field_matches`, `field_json_path_equals`, `node_executed`,
  `node_not_executed`, `edge_fired`, `route_taken`, `run_status`,
  `total_tokens_max`, `supersteps_max`, `parley_raised`,
  `final_state_snapshot`) over `AssertionContext`, each with an
  `insta`-frozen actionable failure rendering (D-29).
- `ScenarioRunner`, the `eval_scenarios!` macro (`libtest-mimic`
  `harness = false` custom test target), `ScenarioPaladinPort`,
  `CapturingSink`, and the `interrupt_after_superstep` crash/resume driver
  (D-31, D-32).
- Gated live mode (`--live` + `PALADIN_EVAL_LIVE` + a configured provider
  credential, all three required) behind the crate's own `live` feature;
  content-bearing assertions are skipped under live mode unless a scenario
  opts in (D-35).
- Classified a composition crate under ADR-0031's own extracted-crate scope
  language — see `.planning/decisions/0048-paladin-eval-composition-crate.md`
  (ADR-0048) — and registered as the twelfth publishable crate in the
  release pipeline.

## [0.9.0] - 2026-09-01

Crate did not exist at this version — created in this same v0.10.0 cycle
(Phase 28). Listed here only so this file's version history reads
consistently against its sibling crates' changelogs, which all carry a
`0.9.0` entry.
