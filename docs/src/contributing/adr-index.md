# Architecture Decisions

The project's decision records live under `.planning/decisions/` in the repository. This page
indexes the ones that change what a crate consumer or operator sees.

| ADR | Title | Decision | Record |
|-----|-------|----------|--------|
| 0033 | One `cargo doc` bar — ratified, measured, and its residue | Precedence order settles the zero-`warning:` `cargo doc` bar as already-ratified, not newly contested | [0033-cargo-doc-warning-bar.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0033-cargo-doc-warning-bar.md) |
| 0037 | The agent route surface is `/v1` | Agent API served under `/v1`; `/health`, `/ready`, `/openapi.json`, `/docs` stay unversioned | [0037-agent-route-surface-v1.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0037-agent-route-surface-v1.md) |
| 0039 | HTTP-served agents carry no Garrison and no Arsenal — a permanent property of the topology | The absence is a permanent topology property, not planned/forward scope | [0039-http-topology-no-garrison-no-arsenal.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0039-http-topology-no-garrison-no-arsenal.md) |
| 0042 | LLM-native tool calling deferred as a future capability, with a named trigger and owner | Recorded as future capability improvement, not built | [0042-llm-native-tool-calling-deferred.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0042-llm-native-tool-calling-deferred.md) |
| 0047 | `docs/src/appendix/design-and-architecture.md` disposition — archived, Sentinel re-anchored, diagram clause withdrawn | Page recorded historical, superseded by `docs/src/architecture/` | [0047-architecture-appendix-disposition.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0047-architecture-appendix-disposition.md) |
| 0048 | `paladin-eval` as a published composition crate | Classified a composition crate; ADR-0031's default-build invariant does not apply | [0048-paladin-eval-composition-crate.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0048-paladin-eval-composition-crate.md) |
| 0049 | `Commissary` design, rename rationale, and rejected names | Re-ported under new vocabulary, never as `Quartermaster` | [0049-commissary-design-and-rename.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0049-commissary-design-and-rename.md) |
| 0050 | `Treasurer` reserved for cross-run spend governance | Role and scope reserved for Milestone 14; not built this cycle | [0050-treasurer-reservation.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0050-treasurer-reservation.md) |
| 0051 | Token-economy phases land as clean breaks inside the untagged v0.10.0 | X-03 superseded for Phases 31-33 only, on the operator's 2026-09-14 decision | [0051-token-economy-versioning-x03-supersession.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0051-token-economy-versioning-x03-supersession.md) |
