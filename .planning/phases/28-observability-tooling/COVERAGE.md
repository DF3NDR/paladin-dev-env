# API Coverage — Phase 28 (Observability & Tooling)

No external API integration: the only outbound surface is an operator-configured OTLP collector
endpoint reached through the `opentelemetry-otlp` exporter *library* (D-12) — a wire-protocol
exporter with no third-party capability surface to enumerate; everything else this phase builds
(trace model, storage adapters, Mermaid/DOT exporters, the `dev-ui` page, the `paladin-eval`
harness) is first-party, in-process code against the workspace's own ports.

The deterministic detector was run at plan time over the ROADMAP Phase 28 section and returned
`{"detected": false, "signals": []}`. The scope was re-read by hand and the verdict confirmed:
`opentelemetry-otlp` is a dependency, not an integrated service with `search`/`play`/`list`-style
capabilities; the `--live` eval mode (D-35) reuses the already-integrated `paladin-llm`
`provider_factory` under ADR-0012 and adds no new provider surface.
