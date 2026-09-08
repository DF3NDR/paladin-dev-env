# paladin-eval

Deterministic evaluation harness for Paladin agent graphs.

## Purpose

`paladin-eval` provides a serde scenario file format (`.eval.yaml`), a scripted `LlmPort`
implementation (`ScenarioLlm`) that makes a run deterministic without touching a real
provider, and an assertion library evaluated over a captured trace record and the final
`Battlefield`. It is a composition-tier tool crate: it depends downward on `paladin-core`,
`paladin-ports`, `paladin-battalion`, `paladin-llm` (`mock` feature) and `paladin-storage`
(`sqlite` feature), and never on the facade `paladin-ai` — see ADR-0047 for the
classification.

## Status

Published (`publish = true`) so a downstream team can add it as a `[dev-dependencies]`
entry for their own agent-graph test suites (OBS-04).

## Scenario file format

A `.eval.yaml` (or `.eval.json`) file carries `schema_version: "1"`, a `target`
(`graph_doc` path or a `registered` constructor name), an optional `store`, an `llm`
script and a list of `cases`, each with `assertions`. See
`docs/schemas/eval-scenario.schema.json` for the machine-readable schema and the crate's
own rustdoc for a worked example.
