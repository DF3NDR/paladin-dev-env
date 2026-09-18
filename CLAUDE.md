# Paladin — Claude Code Project Guide

Paladin is a **Rust-based enterprise multi-agent orchestration framework** built with
**Hexagonal Architecture** (Ports & Adapters) and **Domain-Driven Design**. This file is
loaded automatically every session and is the single source of truth for how Claude works
in this repo.

## Imported instructions

The detailed conventions live in the existing `.github/` instruction files and are imported
below so they apply to every session (one source of truth shared with GitHub Copilot):

@.github/copilot-instructions.md
@.github/instructions/rust.instructions.md
@.github/instructions/security.instructions.md

## Workspace layout

This is a Cargo **workspace** with crates under `crates/`:

| Crate | Responsibility |
|-------|----------------|
| `paladin-core` | Core domain primitives (`Node<T>`, base types, platform container) |
| `paladin-ports` | Application-layer port traits (input/output interfaces) |
| `paladin-battalion` | Multi-agent orchestration (Formation, Phalanx, Campaign, Chain of Command) |
| `paladin-llm` | LLM provider adapters (OpenAI, DeepSeek, Anthropic) |
| `paladin-memory` | Garrison memory adapters (in-memory, sqlite) |
| `paladin-storage` | Repositories / persistence (mysql, sqlite, Citadel) |
| `paladin-content` | Content ingestion & processing |
| `paladin-notifications` | Notification adapters |
| `paladin-web` | Web / HTTP surface |

Other key directories: `docs/` (architecture & guides), `notes/` (design notes),
`project/` (PRDs & task lists), `examples/`, `benches/`, `migrations/`, `k8s/`, `docker/`.

## Quick command reference

```bash
make help                 # List all make targets
cargo build               # Build workspace
cargo test                # Unit tests
make test-all             # Unit + integration
make test-integration-docker  # Integration with Docker services (Redis, MinIO)
cargo test -p paladin-battalion   # Test a single crate

make clean-code           # fmt + clippy + check (run before committing)
cargo fmt --check         # Verify formatting
cargo clippy -- -D warnings   # Lint, warnings as errors
make audit                # cargo-audit for vulnerable deps
make deny                 # cargo-deny (licenses/bans/advisories)

make dev                  # Start all dev services
make services-up          # Services only (Redis, MinIO)
make health               # Service status
```

## Working agreements

- **TDD (Red-Green-Refactor)** — write the failing test first. Coverage floor: **82% workspace
  line coverage** (single number, no separate unit/integration targets — superseded 2026-08-13,
  see ADR-0006), gated by `cargo llvm-cov --fail-under-lines` in CI's `coverage` job. All public
  APIs need doc tests.
- **Dependencies flow inward only**: core → (nothing); application/ports → core; infrastructure
  adapters → core + ports. Never import infrastructure from core or application.
- **Ubiquitous language**: always use the Medieval Military terms (Paladin, Battalion, Garrison,
  Arsenal, Citadel, Herald, Quest, …) consistently in code, docs, and comments.
- **Before committing a parent task**: `cargo test` → `cargo fmt --check` → `cargo clippy` →
  `make api-surface` (an intentional public-surface change is refreshed with
  `make api-surface-update` plus a CHANGELOG entry), then conventional-commit message. Stop after
  each major task and wait for go-ahead.
- **Security**: run `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings`
  on new/modified code, plus the manual credential-handling review in the imported
  `security.instructions.md`. `codeql.yml` runs Rust static analysis **advisory-only** — CodeQL
  was evaluated and disqualified as a required-check-grade Rust SAST at CodeQL `2.26.3`
  (2026-08-25); it does not gate a merge, and the manual credential-handling review stays the
  primary control. Snyk was evaluated and removed — it has no Rust coverage; do not
  reintroduce a Snyk step or record a phase as blocked on one.
- Avoid `unwrap()`/`expect()` and `panic!` in library code — return `Result`. Prefer borrowing
  over cloning; keep iterators lazy until you need a collection.

## Slash commands

Project workflow commands live in `.claude/commands/` (converted from `.github/prompts/`):

- `/create-prd` — generate a Product Requirements Document into `project/`
- `/generate-tasks` — turn a PRD/requirements into a `tasks-*.md` checklist in `project/`
- `/process-tasks` — drive a task list to completion following the Rust completion protocol

## MCP servers

GitHub and Context7 (live library docs) are connected via the claude.ai account and available
now. `.mcp.json` holds a documented, project-scoped scaffold for additional servers — see the
comments in that file before enabling (they require `node`/`uv` at runtime).
