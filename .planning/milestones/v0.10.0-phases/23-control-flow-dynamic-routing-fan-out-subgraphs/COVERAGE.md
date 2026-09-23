# Phase 23 — API Coverage Decision

No external API integration: Phase 23 extends the in-tree `WarEngine` superstep scheduler
(`crates/paladin-battalion/src/engine/`) and the legacy `CampaignExecutionService` with routing,
fan-out and subgraph control flow; the only outward-facing call it makes is through the
already-shipped `paladin-ports::output::llm_port::LlmPort` abstraction (CF-05), reusing the
existing provider adapters without adding, changing, or newly integrating any provider SDK,
HTTP endpoint, or external service.

The deterministic detector (`api-coverage.cjs`) returned `{"detected": false, "signals": []}`
against this phase's CONTEXT.md + ROADMAP scope on 2026-09-03; this declaration is recorded so
the `api-coverage.verify-pre` seal gate has an explicit, reasoned artifact rather than a
re-derivation from PLAN.md prose.
