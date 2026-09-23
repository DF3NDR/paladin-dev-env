# Phase 22: Battlefield State & Superstep Engine - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-01
**Phase:** 22-battlefield-state-superstep-engine
**Mode:** `--auto` — Claude selected the recommended option for every question; no interactive prompts
**Areas discussed:** Waypoint backend placement & schema, Identity/fingerprint/legacy mapping, Program CI scaffolding (ENG-08), Contract suite & test infrastructure, Engine semantics defaults

---

## Waypoint backend placement & schema

| Option | Description | Selected |
|--------|-------------|----------|
| paladin-storage | Repositories/persistence/Citadel home; sqlx sqlite stack present; `postgres` feature addition localized | ✓ |
| paladin-memory | Owns Garrison sqlite adapter + migrations dir, but is conversation-memory scoped | |
| Split across crates | InMemory near engine tests, SQL adapters in storage — two homes for one port | |

**Auto-selection:** paladin-storage hosts all three backends (recommended default). PRD 01 §4 explicitly left "in `paladin-memory` or `paladin-storage`" open.

| Option | Description | Selected |
|--------|-------------|----------|
| TEXT serialized JSON | Debuggable, aligns with sqlx `json` feature | ✓ |
| BLOB | Marginally smaller, opaque in tooling | |

**Auto-selection:** TEXT (recommended default). PRD schema note says "payload BLOB/JSON" — either permitted.

---

## Identity, fingerprint & legacy mapping

| Option | Description | Selected |
|--------|-------------|----------|
| UUIDv7 | PRD-preferred, time-ordered; needs `v7` feature on existing workspace uuid 1.8 dep | ✓ |
| UUIDv4 | Works with current features, loses time-ordering | |

**Auto-selection:** UUIDv7 (recommended default). Feature addition on an existing dependency does not violate ENG-01's "no new core dependencies".

| Option | Description | Selected |
|--------|-------------|----------|
| blake3 canonical | Already a paladin-core dep; hash over deterministically-ordered node/edge/schema serialization | ✓ |
| sha2 | Also present; slower, no advantage here | |
| murmur3 | Non-cryptographic; collision posture weaker for a stored compatibility check | |

**Auto-selection:** blake3 canonical (recommended default).

| Option | Description | Selected |
|--------|-------------|----------|
| Name slug + uuid fallback | Human-readable NodeIds per PRD intent ("e.g. researcher"); deterministic | ✓ |
| Raw uuid strings | Deterministic but defeats the PRD's human-readable NodeId intent | |

**Auto-selection:** name slug with `{name-slug}-{short-uuid}` fallback on collision (recommended default).

---

## Program CI scaffolding (ENG-08)

| Option | Description | Selected |
|--------|-------------|----------|
| Published crates.io v0.9.0 baseline | Matches ENG-08 wording "vs the published v0.9.0 crates"; per-item allowlist in repo | ✓ |
| Git tag baseline | Simpler offline, but diverges from the requirement text | |

**Auto-selection:** published crates.io baseline (recommended default).

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated Rust 1.85 job | Pins MSRV toolchain, builds full workspace `--all-features` per X-11.1 | ✓ |
| cargo-msrv verify | Equivalent allowed by X-11.1; extra tool dependency in CI | |

**Auto-selection:** dedicated 1.85 job (recommended default). X-11.2 stop-and-flag applies if 1.85 proves unsatisfiable.

| Option | Description | Selected |
|--------|-------------|----------|
| Living doc, scoped TBD | Full §9 skeleton now; TBD only in later-epic sections; this phase's items filled | ✓ |
| Fill everything now | Impossible — later epics own M-B-02/03 resolutions and their §9 sections | |

**Auto-selection:** living doc with scoped TBD (recommended default; overview §9 defines MIGRATION.md as appended per epic).

---

## Contract suite & test infrastructure

| Option | Description | Selected |
|--------|-------------|----------|
| Generic async test fns | Clearer failure diagnostics, better IDE/type support | ✓ |
| `waypoint_port_contract_tests!` macro | Also PRD-sanctioned; noisier diagnostics | |

**Auto-selection:** generic fns (recommended default). PRD 01 ENG-FR-17 permits "a shared macro **or** generic test fn".

| Option | Description | Selected |
|--------|-------------|----------|
| docker-compose Tier 2 | PRD acceptance 4 names "the existing docker-compose integration target"; add postgres service | ✓ |
| testcontainers | Dep exists (0.24) but diverges from the PRD-named target | |

**Auto-selection:** docker-compose Tier 2 (recommended default).

| Option | Description | Selected |
|--------|-------------|----------|
| Seeded randomized scheduling repeat test | Shuffle spawn order / inject yields, ≥20 iterations, byte-identical assertion | ✓ |
| proptest strategies | Heavier machinery for the same guarantee | |

**Auto-selection:** seeded shuffle repeat test (recommended default).

---

## Engine semantics defaults

| Option | Description | Selected |
|--------|-------------|----------|
| Keep all PRD defaults | parallelism = vanguard size; `Strict` durability; limits 50/25; Arc-shared snapshot | ✓ |

**Auto-selection:** keep PRD defaults — these are stated in PRD 01 §3.5/ENG-FR-11 and were not re-opened.

---

## Claude's Discretion

- Module file layout in `crates/paladin-battalion/src/engine/` and core module splits
- Error message wording, internal data structures, bench harness details
- `WaypointSummary`/`ThreadSummary` field selection beyond PRD implications
- `NodeContext` shape for this phase
- Plan decomposition (respecting PRD §7 TDD ordering)

## Deferred Ideas

None — all later-epic capabilities are already roadmapped to Phases 23-28.
