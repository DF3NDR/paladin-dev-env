# ADR-0015: `paladin-core` / `paladin-ports` dependency allowlist and the purity invariant

## Status

Accepted

**Date:** 2026-08-06

**Amended 2026-10-24 (Phase 46, D-20/D-21):** re-measured from the manifests' `[dependencies]` tables: `paladin-core` fourteen keys (unchanged from the measurement below); `paladin-ports` twelve -- the eleven listed below plus `log`, the logging facade `output/trace_sink_port.rs` uses for `log::error!`. `log` is not a provider SDK, transport client, storage driver or web framework, so invariant (i) holds. The two sets are now pinned by `scripts/check-dependency-allowlist.sh` (wired into `make check-gates` and CI's `cargo-deny` job); changing either set needs a dated note here in the same commit.

- `paladin-core` (14): `serde`, `serde_json`, `uuid`, `chrono`, `thiserror`, `async-trait`, `tokio`, `sha2`, `blake3`, `petgraph`, `murmur3`, `url`, `regex`, `futures`.
- `paladin-ports` (12): `paladin_core`, `async-trait`, `serde`, `thiserror`, `uuid`, `chrono`, `tokio`, `serde_json`, `futures`, `log`, `md5`, `mime_guess`.

## Context

`.project/Milestone_5-Workspace-Decomposition/Epic_1/prd-workspace-initialization-and-paladin-core-extraction.md`
Appendix B (`:321-334`) calls its list "the complete and exhaustive list of external crates that
`paladin-core` is allowed to depend on" — six crates: `serde`, `serde_json`, `uuid`, `chrono`,
`thiserror`, `async-trait`. That list is the stated enforcement mechanism for the crate's whole
hexagonal-purity argument: FR-6 (`:102`) states it directly, and SM-4 / FR-24 / FR-25 (`:283`,
`:133-134`) all measure purity against `cargo tree -p paladin-core` showing "only permitted deps".

Measured directly from `crates/paladin-core/Cargo.toml` `[dependencies]` (`:18-31`) during this
task: **fourteen** entries — the six PRD-permitted crates plus `tokio`, `sha2`, `blake3`,
`petgraph`, `murmur3`, `url`, `regex`, `futures`. Measured directly from
`crates/paladin-ports/Cargo.toml` `[dependencies]` (`:21-31`) during this task: **eleven**
entries — `paladin_core`, `async-trait`, `serde`, `thiserror`, `uuid`, `chrono`, `tokio`,
`serde_json`, `futures`, `md5`, `mime_guess`.

`.planning/intel/code-verification.md` records `paladin-ports` at ten dependencies (listing
`serde_json`, `futures`, `md5` as the extras beyond the base seven). That figure predates
`mime_guess = "2"` (`crates/paladin-ports/Cargo.toml:31`), which is present in the manifest today
and makes the correct count eleven, not ten.

A list wrong by eight entries in `paladin-core` and by four (or, per the stale intel figure, five)
in `paladin-ports` is unenforceable as written — a `cargo tree` check built against Appendix B's
literal text would fail on every default build. Underneath the wrong list, the thing Appendix B,
FR-24 and FR-25 were actually protecting — that `paladin-core` and `paladin-ports` stay free of
provider SDKs, transport clients, storage drivers and web frameworks — was never written down
separately from the six-crate enumeration. Rebaselining the count alone, without stating that
invariant, would leave the same unenforceable shape one measurement later.

## Decision

Three things, stated in this order and kept separate from one another:

**(i) The enforceable invariant.** `paladin-core` and `paladin-ports` may carry no provider SDK
(e.g. `async-openai`, an Anthropic or DeepSeek client), no transport client (e.g. `reqwest` used
for outbound network calls, `redis`), no storage driver (e.g. `sqlx`, `minio`), and no web
framework (e.g. `axum`, `actix-web`). This is what FR-24, FR-25 and SM-4 were actually measuring,
independent of any specific crate count, and it **holds today**: none of the fourteen
`paladin-core` entries or eleven `paladin-ports` entries is a provider SDK, transport client,
storage driver, or web framework.

**(ii) The measured current lists, as the new baseline.** `paladin-core`'s fourteen dependencies —
the PRD's original six (`serde`, `serde_json`, `uuid`, `chrono`, `thiserror`, `async-trait`) plus
eight extras: `tokio`, `sha2`, `blake3`, `petgraph`, `murmur3`, `url`, `regex`, `futures` — are
accepted as the baseline, not recorded as debt. Each extra is a general-purpose or domain-support
crate rather than infrastructure: `sha2`/`blake3`/`murmur3` are hashing primitives used by domain
identity/dedup logic, `petgraph` is a graph data structure backing `Campaign`'s DAG type, `url`
and `regex` are general-purpose parsing utilities, and `futures` supplies async combinators used
alongside `tokio`. `paladin-ports`'s eleven dependencies — the base seven (`paladin_core`,
`async-trait`, `serde`, `thiserror`, `uuid`, `chrono`, `tokio`) plus four extras: `serde_json`,
`futures`, `md5`, `mime_guess` — are accepted on the same terms; `md5` and `mime_guess` support
content-hashing and MIME-type detection in port-level value types, not infrastructure adapters.

**(iii) `tokio` in `paladin-core` gets its own written justification**, because it is an async
runtime inside a crate whose own package description
(`crates/paladin-core/Cargo.toml:9`) reads "zero infrastructure dependencies", and it is the one
entry a future purity review would reasonably challenge on that description's strength alone.
Read from the crate's own source, not asserted: `src/base/service/message_service.rs:20` imports
`tokio::sync::{RwLock, mpsc}` and uses `tokio::spawn` (`:397`, `:450`) to run the base
`MessageService`'s background worker pool — a domain-level service every platform messaging
service (Log, Notification, Event) extends, per the file's own header comment (`:1-14`).
`src/platform/container/task.rs` uses `tokio::time::sleep` (`:180`) in domain-level polling logic
and `tokio::fs::create_dir_all`/`tokio::fs::write` (`:355`, `:380`, `:428`, `:456`) for the
`Task` entity's async persistence helpers. `tokio` is therefore not an incidental transitive pull —
it is the async runtime the domain layer's own service and entity types are written against, and
removing it would require rewriting `MessageService` and `Task` off `async`/`await` entirely. It
is accepted as part of the baseline on that basis, not exempted from scrutiny.

## Considered Options

- State the intended six-crate target and treat the extra twelve (or thirteen, counting
  `paladin-ports`'s eleventh) as tracked debt — rejected. This manufactures twelve debt items
  nobody intends to pay down (none of the extras is infrastructure; there is no plan to remove
  `tokio`, `sha2`, or the others), and it still leaves the invariant Appendix B was protecting
  unstated separately from the list.
- Rebaseline the list to the measured fourteen/eleven without stating an invariant — rejected. This
  reproduces the exact unenforceable shape the PRD's six-crate list already has, one measurement
  later: the next dependency addition (a `paladin-core` `mime_guess`-equivalent, say) would again
  be either "exhaustive-list-violating" or silently ignored, with no stated principle to test it
  against.
- State the invariant separately from the list, rebaseline the list against measurement, and
  justify `tokio` explicitly — accepted. This is the only option that both makes the list
  enforceable again and preserves the purity property Appendix B, FR-24, and FR-25 actually cared
  about, independent of which specific crates happen to be present at any given measurement.

## Code Locations

- `crates/paladin-core/Cargo.toml:17-31` — the `[dependencies]` block, fourteen entries, re-counted
  during this task
- `crates/paladin-ports/Cargo.toml:20-31` — the `[dependencies]` block, eleven entries including
  `mime_guess = "2"` (`:31`), re-counted during this task
- `.project/Milestone_5-Workspace-Decomposition/Epic_1/prd-workspace-initialization-and-paladin-core-extraction.md:102`
  — FR-6, the six-crate-only requirement text
- `.project/Milestone_5-Workspace-Decomposition/Epic_1/prd-workspace-initialization-and-paladin-core-extraction.md:321-334`
  — Appendix B, the "complete and exhaustive" six-crate table this ADR rebaselines
- `.project/Milestone_5-Workspace-Decomposition/Epic_1/prd-workspace-initialization-and-paladin-core-extraction.md:133-134,283`
  — FR-24, FR-25, and SM-4, the purity clauses the allowlist exists to satisfy
- `.github/workflows/ci.yml:304` — the `crate-isolation` job, the build leg that exercises
  crate-boundary isolation today (re-grepped this task; the figure recorded elsewhere in this
  corpus as `:228` is stale)

## Code Conformance

conforms

The tree already satisfies the invariant stated in `## Decision` (i): no provider SDK, transport
client, storage driver, or web framework appears in either crate's dependency list. This ADR
changes the recorded baseline — replacing Appendix B's six-crate "complete and exhaustive" claim
with the measured fourteen/eleven and an invariant stated independently of the count — rather than
changing any code. No enforcement check is built by this phase; the `cargo tree`-based allowlist
check that would mechanically verify the invariant on every build is recorded below as a Phase 15
candidate, not implemented here.

## Downstream Consumers

- Phase 15 — receives the `cargo tree`-based dependency-allowlist check as a candidate to build
  against this ADR's invariant (D-10); not built in this phase.
- Plan 07-06 and plan 07-10 — the ledger rows for `REQ-paladin-core-dependency-allowlist-v1` and
  `REQ-paladin-core-dependency-allowlist-v2` cite this ADR as the recorded answer to ARCH-03(b).
- Plan 07-13 — adds this ADR's row to `.planning/decisions/PROMOTION.md`'s numbering index and
  advances the "Next free ADR number" line past 0015.
