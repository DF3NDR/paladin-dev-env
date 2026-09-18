# Phase 32: Unified Token Primitives - Context

**Gathered:** 2026-09-15
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected; every question resolved to the recommended option and logged in `32-DISCUSSION-LOG.md`)

<domain>
## Phase Boundary

Collapse two duplications in the token primitives to one each:

1. **One counting contract.** `TokenCounterPort`
   (`crates/paladin-ports/src/output/token_counter_port.rs`) gains
   `fn is_exact(&self) -> bool { false }`. `TiktokenCounter` returns `true`, `HeuristicTokenCounter`
   returns `false`. `Commissary::new` and `Commissary::from_port`
   (`crates/paladin-llm/src/services/commissary.rs`) drop the `is_exact_counter: bool` argument and
   read exactness from the port. The legacy fallible `garrison::TokenCounter` trait and
   `TokenCounterFactory` (`crates/paladin-memory/src/garrison/token_counter.rs`) are removed
   together with their three re-exports (`paladin-memory` `garrison/mod.rs:16`, `prelude.rs:10`,
   and the facade `src/infrastructure/adapters/garrison/mod.rs` at both line 9 and the
   `token_counter` compat sub-module at line 24).
2. **One window resolver.** A shared function in `paladin-llm` owns the precedence
   config table → provider capabilities → fallback, with an explicit strict mode that errors
   rather than defaults when the window is unknown. `HistoryTrimmer::resolve_limit`
   (`src/application/services/paladin/middleware/history.rs`) and `Commissary::new`'s inline
   `.or(fallback)` guard both become calls to it. Commissary resolves the same windows and
   HistoryTrimmer produces the same trims as before — proven, not assumed.

Plus the release bookkeeping every clean break in this milestone carries: `MIGRATION.md` §9.2
rows, row-matched `.cargo/semver-checks-allowlist.toml` entries, `CHANGELOG.md` `[0.10.0]`
entries, `make clean-code` and the 82 % coverage floor green (PRIM-05).

**Breaking, clean break, no shims** — governed by ADR-0051 (X-03 superseded for Phases 31-33
only). No forwarding constructor for `Commissary::new`, no `#[deprecated]` legacy trait, no
compatibility re-export of the removed names.

**Not in this phase:** any change to Commissary's dispensing behaviour or the windows it
resolves (PRD §4); a per-model override table for Commissary; a production caller for
Commissary and the RAG truncation path (Phase 33, COMM-01…03); re-sealing the Phase 29 release
gates (Phase 33, COMM-04); renaming any token type (Phase 30 D-17); pricing, `cost_estimate`,
allowances, pacing (Milestone 14); moving the resolver into `paladin-ports` (the roadmap and
PRD lock `paladin-llm`).

</domain>

<decisions>
## Implementation Decisions

### Shared resolver contract (PRIM-04)
- **D-01:** Strict mode is a **fallback-policy enum**, not a `bool`. Two variants: a lenient
  policy carrying a framework default (`Default(u32)` — always resolves; `HistoryTrimmer` passes
  `config.default_context_tokens`) and a strict policy carrying only the caller's optional
  fallback (`Strict { caller_fallback: Option<u32> }` — `Commissary` passes
  `config.fallback_context_tokens`). The precedence walk is the same for both: (1) config-table
  hit for `model`, (2) `capabilities.max_context_tokens`, (3) the policy's fallback. Strict
  errors when step 3 has nothing; the framework never invents a window (ADR-0010 refusal
  semantics, ADR-0049). This keeps Commissary's caller-supplied fallback working exactly as today
  (it is caller policy, not an invented window) while making "strict" an explicit, non-redundant
  type rather than a flag that can disagree with an `Option`. — **Reversibility:** costly — this
  is new public API in `paladin-llm` consumed by two services; reshaping it later is another
  §9.2 break.
- **D-02:** Placement is a **top-level `paladin_llm::window` module** (the PRD's own example
  path, `window::resolve_context_window`), not `services::window` and not a private helper in
  `commissary.rs`. The function takes `model: &str`, the optional per-model table
  (`Option<&HashMap<String, u32>>` or equivalent — planner's call), the provider capabilities
  (`&ProviderCapabilities` or just its `max_context_tokens`), and the policy from D-01; it
  returns `Result<ResolvedWindow, UnknownContextWindow>` where the resolved value carries **both
  the token count and which step produced it** (a `WindowSource` naming config table / provider
  capabilities / default / caller fallback). Carrying the source is what lets `HistoryTrimmer`
  keep Phase 26 D-14's debug log ("why did my history get trimmed at 8192") — its existing
  precedence tests assert on that source text. The facade `src/lib.rs` re-exports the resolver
  types next to the Commissary block (~line 200) so facade users see one surface.
- **D-03:** **`Commissary` gains no config table.** `CommissaryPlan` is unchanged; Commissary
  passes an empty/absent table so step 1 is a no-op and every window resolves identically to
  today (PRD §4: no behavioural drift). A per-model override for Commissary is a deferred idea.
- **D-04:** Commissary calls the resolver **once, in `new`**, replacing the inline
  `capabilities.max_context_tokens.or(config.fallback_context_tokens)` guard; the resolved
  `u32` is stored and `window()` returns it (the `ReservationExceedsWindow` check runs on the
  same value, unchanged). The resolver's unknown-window error maps to the existing
  `CommissaryError::UndeclaredContextWindow { provider }` — same variant, same `Display` text —
  so no test, rustdoc or mdBook wording drifts. `from_port` is unchanged apart from dropping
  `is_exact_counter` (D-08).
- **D-05:** `HistoryTrimmer::resolve_limit` becomes a thin call to the resolver with
  `Default(config.default_context_tokens)` and `&config.model_context_limits`; the local
  `LimitSource` enum is either deleted in favour of `WindowSource` or kept as a one-line mapping
  (planner's discretion), but the three logged source strings stay recognisable to the existing
  tests (`contains("provider")`/`("capabilities")`/`("default")`). Trim semantics
  (`KeepSystemAndRecent`, Phase 26 D-15) are untouched.

### Exactness on the port (PRIM-01, PRIM-02)
- **D-06:** `is_exact` is a **property of the adapter instance as constructed**, not of a
  `(text, model)` call: `true` means "`count` returns the model's own tokenizer tally";
  `false` means an approximation. Default `false`. The port's rustdoc states this and its
  doc test shows a bare impl inheriting `false`. — **Reversibility:** costly — flipping the
  default later silently changes the meaning of every external impl that relies on it.
- **D-07:** `TiktokenCounter::is_exact` returns **`true` unconditionally**, with rustdoc stating
  it is exact for the encoding resolved at `new(model)` and that `count`'s `model` argument is
  ignored (existing behaviour, documented at `token_counter.rs:~132`). `HeuristicTokenCounter`
  **relies on the trait default** (no override); its unit test asserts `false`, which doubles as
  the "defaulting to false" proof PRIM-01 asks for. The tiktoken test lives under the
  `content-processing` feature (that is where the type lives); it runs in CI via the
  `--all-features` clippy/check jobs and the `--all-features` test matrix rows (`ci.yml:~597`).
- **D-08:** `Commissary` **drops its private `is_exact_counter` field** and reads
  `self.counter.is_exact()` where `Stockpile::exact_tally` is set — one source of truth, no
  cached duplicate. Its `Debug` impl prints `counter.name()` and the live `is_exact()` in place
  of the field. `new` and `from_port` both lose the argument; the module docs' "Honesty clause"
  and rustdoc are rewritten to say exactness now comes from the port. Test doubles gain a
  configurable exactness where a test needs `true` (`MockCounter` in `commissary.rs:~610`;
  `AlwaysOneCounter`/`CountingTokenCounter` in `paladin_execution_service.rs:~6050/~6123` only if
  a test there needs it). — **Reversibility:** one-way — `Commissary::new`/`from_port` are a
  published signature; the downstream app's ~150 call sites migrate once, per ADR-0051.

### Legacy counter retirement (PRIM-03)
- **D-09:** The **`#[deprecated]` escape hatch is not used.** The scout found zero in-tree
  callers of `TokenCounter` / `TokenCounterFactory` outside `paladin-memory`'s own definition
  file and the three re-export sites (no hits in `src/`, `examples/`, `tests/`, `benches/`,
  `crates/doc-examples`). Both are removed outright in one commit, together with
  `TokenCounterFactory::for_model`, `supported_models` and `is_supported` (no callers).
  — **Reversibility:** one-way — public types removed from a published crate; ADR-0051 is the
  recorded authority.
- **D-10:** What survives on `TiktokenCounter`: `new(model) -> Result<Self, GarrisonError>`
  (construction stays fallible; `GarrisonError::TokenizationError` stays), its
  `impl TokenCounterPort` (now the only counting path — `count` does the BPE lookup and the
  per-string cache directly, no `Result` round-trip through a trait method), an inherent
  `model_name(&self) -> &str` accessor (kept: already used by its tests, cheap, useful in logs),
  and `is_exact() -> true` (D-07). No fallible inherent `count_tokens` is kept.
- **D-11:** The facade's backward-compatible `token_counter` sub-module
  (`src/infrastructure/adapters/garrison/mod.rs:~22-26`) and the top-level re-export at line 9
  are **kept, narrowed to `TiktokenCounter`** — the compat path still resolves for the one
  surviving type. `paladin-memory`'s `garrison/mod.rs:16` and `prelude.rs:10` likewise keep
  `TiktokenCounter` only.
- **D-12:** Doc sweep for the removed names and the dropped argument:
  `crates/paladin-memory/src/lib.rs` (feature table row + "Token Counter" narrative),
  `docs/src/architecture/crate-map.md:178`, `docs/src/user-guides/memory-management.md:159`,
  `commissary.rs` module docs, `docs/src/architecture/commissary.md` (the usage sketch must keep
  mirroring a real unit test — Phase 30 D-06 — and the "Honesty about exactness" section is
  rewritten), and a short "Token primitives" subsection in `docs/src/api-reference/upgrading.md`
  / `migration-guide.md` pointing at §9.2. Exit grep:
  `grep -rnE '\bTokenCounterFactory\b|garrison::TokenCounter\b|is_exact_counter' crates src docs examples benches`
  → empty (`.planning/` and `.project/` history are out of scope, per overview §5.4).

### Equivalence proof and release bookkeeping (PRIM-04, PRIM-05)
- **D-13:** The "equivalence snapshot" is **TDD-ordered fixture tests, not `insta`** (only
  `paladin-eval` pins insta; adding it to `paladin-llm` or the facade buys nothing). The first
  plan writes table-driven tests **against the pre-refactor code** and commits them green:
  Commissary windows for {capabilities hit, capabilities `None` + fallback hit, both `None` →
  `UndeclaredContextWindow`} and `allotted_tokens` for each, and HistoryTrimmer kept-sets for
  {config-table hit, capability hit, default} over a fixed history — all with distinct,
  non-round numbers (house style, Phase 31 `<specifics>`). The refactor commit(s) keep those
  tests byte-identical and green; the commit order (tests land before the refactor) is the
  snapshot. Separately, four precedence tests on the resolver itself cover: table beats
  capability, capability beats fallback, lenient default when both absent, strict refusal when
  both absent and no caller fallback.
- **D-14:** Semver lint discovery is **empirical (Phase 31 D-27), extended for feature gating**:
  the CI `semver` job runs `cargo semver-checks check-release --package <pkg> --default-features
  --baseline-version 0.9.0` (`ci.yml:~357`), and `TokenCounter`/`TokenCounterFactory` sit behind
  `content-processing` in both `paladin-memory` and `paladin-ai`, so the CI run **cannot observe
  their removal**. The executor runs the CI command for `paladin-ports`, `paladin-llm`,
  `paladin-memory`, `paladin-ai`, AND once more with `--features content-processing` for
  `paladin-memory` and `paladin-ai`, records every lint id that fires (expected:
  `trait_missing` + `struct_missing` for the legacy pair; `inherent_method_missing` or the
  method-parameter-count lint for `Commissary::new`/`from_port`; likely nothing for the
  defaulted `TokenCounterPort::is_exact`), and writes the allowlist entries with a
  `justification` that names the feature gate. Any lint that fires on the **default-features**
  run must also be added to that crate's `[package.metadata.cargo-semver-checks.lints]` table
  (`paladin-llm` has none today — add one; `paladin-memory` only if needed) so the CI job stays
  green.
  **Release-type trap (Phase 31 plan 31-07 finding):** every crate is already bumped to `0.10.0`, so
  a `--baseline-version 0.9.0` run treats the release as breaking-allowed and prints `0 checks, N
  skipped` while exiting 0 — pass `--release-type minor` on every local discovery run so the lints
  actually evaluate. The CI job's own command is left as it is; the allowlist rows are what the
  row-level gate checks.
- **D-15:** `MIGRATION.md` §9.2 gets **one row per `crate | Type` pair the row-level gate keys
  on** (Phase 29 D-04: the type cell reduces to its first backtick identifier):
  `paladin-llm | Commissary` (the `new`/`from_port` signature break — one row covers both
  methods), `paladin-memory | TokenCounter`, `paladin-memory | TokenCounterFactory`;
  `paladin-ai` rows only if a lint fires for the re-export removal. `paladin-ports |
  TokenCounterPort` gets a row only if a lint fires (it is an additive defaulted method); if
  nothing fires it is mentioned in the CHANGELOG only. Every row marked `Y` gets its
  `[[entry]]` in the **same commit**, following the `PaladinResult`/`LlmRequest` row templates.
- **D-16:** `CHANGELOG.md` `[0.10.0]` (Phase 31 D-28): a `### Changed` bullet for the
  `Commissary::new`/`from_port` signature and the shared resolver, a `### Removed` bullet for
  the legacy pair, an `### Added` bullet for `TokenCounterPort::is_exact`. Commit with
  `git commit` directly (the pre-commit hook runs workspace clippy; the GSD commit helper is
  known to time out — Phase 30 `<specifics>`), conventional scopes `feat(32)` / `refactor(32)`
  / `docs(32)`. The PRIM-05 gate evidence (`make clean-code`, coverage floor, the `semver` job's
  per-package run plus the row-level check, `cargo doc` zero-warning, `mdbook build docs/` or
  the docs CI job) goes in the last plan's SUMMARY, as Phase 31 did.

### Claude's Discretion
- Exact type and variant names (`WindowFallback`/`WindowPolicy`, `WindowSource`,
  `ResolvedWindow`, `UnknownContextWindow`) and whether the resolver takes
  `&ProviderCapabilities` or the bare `Option<u32>`.
- Whether `history.rs`'s `LimitSource` is deleted or kept as a mapping to `WindowSource` (D-05).
- Whether `TiktokenCounter`'s per-string cache stays an `RwLock<HashMap>` or is simplified while
  the trait is removed — behaviour (same counts, same cache hits) must not change.
- Test-double naming and where the configurable-exactness knob lives on them.
- Plan/wave granularity — natural waves: (1) equivalence fixtures against pre-refactor code +
  `is_exact` on the port and both adapters (PRIM-01); (2) the shared resolver + precedence tests,
  HistoryTrimmer and Commissary consuming it, `Commissary::new` break (PRIM-02, PRIM-04);
  (3) legacy removal + re-export narrowing + doc sweep (PRIM-03); (4) §9.2 rows, allowlist,
  CHANGELOG, upgrading page, gate evidence (PRIM-05).

### Folded Todos
- **Verify local `make coverage` reproduces CI's 82.39 % figure**
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.6) —
  folded as a verification note only, exactly as Phase 31 did: PRIM-05 already requires the
  coverage floor to be green, so the plan that records coverage evidence also records whether
  the local run reproduces the CI number. No new scope; the todo keeps its no-`resolves_phase`
  status.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone corpus and governing decisions
- `.project/Milestone_13-Token-Economy/Epic_3/prd-unify-token-primitives.md` — the source PRD:
  R1-R6, §4 out of scope (no Commissary behaviour change), §5 tests, §6 exit criteria, §7
  downstream impact (~150 call sites migrate once).
- `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` — §0 locked
  terms (no renames), §3 verified anchors (legacy counter row, `HistoryTrimmer` row), §4
  findings F2/F3 and decisions D-5/D-6, §5 clean-break policy, §7 out of scope.
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — clean break for
  Phases 31-33 only; every break still gets a §9.2 row + allowlist row; `0.10.0` untagged until
  Phase 33.
- `.planning/decisions/0049-commissary-design-and-rename.md` — the Commissary design record:
  `verify_fits` guard + `dispense` allocator, fail-loud / never-silent, the ADR-0010 "no
  invented window" refusal semantics D-01 preserves.
- `.planning/decisions/0050-treasurer-reservation.md` — what is reserved for Milestone 14 and
  therefore out of this phase.
- `.project/v0.10.0/00-program-overview.md` §3 X-10 (semver hygiene, still governing) and
  X-03 (superseded here).

### Planning record
- `.planning/ROADMAP.md` — Phase 32 entry (goal, depends-on, success criteria 1-5, ~line 839)
  and the 2026-09-14 extension footer (Epic 4 R3 folded into PRIM-04; Phase 33 keeps only the
  regression check).
- `.planning/REQUIREMENTS.md` — PRIM-01…05 (~line 405) and the 2026-09-14 extension record.
- `.planning/phases/31-lossless-token-accounting/31-CONTEXT.md` — D-27 (empirical lint
  discovery), D-28 (CHANGELOG `[0.10.0]`), `<specifics>` (distinct non-round test figures),
  the §9.2 row templates named in its canonical refs.
- `.planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-CONTEXT.md` — D-06
  (the mdBook usage sketch mirrors a real unit test), D-17 (no renames), D-18 (write
  "v0.10.0"), `<specifics>` on commit mechanics.
- `.planning/phases/29-program-gates-release/29-CONTEXT.md` D-04 — the row-level allowlist ↔
  §9.2 set-equality gate.
- `.planning/phases/26-agent-runtime-enhancements/26-11-PLAN.md` (D-13/D-14/D-15, lines ~23-28)
  — the infallible-port decision, HistoryTrimmer's three-step order with logged source, and
  trim semantics — history that D-05 preserves; do not edit.

### Migration register and semver tooling
- `MIGRATION.md` §9.2 (~lines 162-277) — row format; the `PaladinResult`, `LlmRequest` and
  `TokenUsage` rows are the templates; §9.5 (~371) already documents `HistoryTrimmerConfig`
  and `model_context_limits`.
- `.cargo/semver-checks-allowlist.toml` — `[[entry]]` schema; 16 entries today.
- `crates/paladin-ports/Cargo.toml:57` — the only per-crate `cargo-semver-checks.lints` table
  in the crates this phase touches (`paladin-llm` and `paladin-memory` have none; D-14).
- `.github/workflows/ci.yml` — `semver` job (~lines 271-360: `--default-features`, baseline
  `0.9.0`, `paladin-eval` excluded), row-level set-equality step (~361-430), `--all-features`
  clippy/check jobs (~60, ~269) and test-matrix rows (~597) where the `content-processing`
  tiktoken tests run.
- `CHANGELOG.md` — `[Unreleased]` (line 8) and `[0.10.0] - 2026-09-10` (line 15) sections.

### Code: the counting contract
- `crates/paladin-ports/src/output/token_counter_port.rs` — `TokenCounterPort` (`count`,
  `name`; module docs name the two adapters and the infallibility rationale).
- `crates/paladin-memory/src/token_counter/heuristic.rs` — `HeuristicTokenCounter`
  (`impl TokenCounterPort` at line 19).
- `crates/paladin-memory/src/garrison/token_counter.rs` (381 lines) — legacy `TokenCounter`
  trait (line 13), `TiktokenCounter` (line 49; `impl TokenCounterPort` at ~141 delegating to
  `count_tokens`, `name() == "tiktoken"`), `TokenCounterFactory` (line 165), tests using
  `model_name()`/`count_tokens()`.
- `crates/paladin-memory/src/garrison/mod.rs:13-16`, `crates/paladin-memory/src/prelude.rs:10`,
  `crates/paladin-memory/src/lib.rs:~27-40` (narrative + feature table),
  `src/infrastructure/adapters/garrison/mod.rs` (lines 9 and 22-26) — the re-export sites.
- `crates/paladin-memory/Cargo.toml:22` — `content-processing = ["dep:tiktoken-rs"]`;
  root `Cargo.toml:515` — the facade feature that forwards it.

### Code: the two resolvers being unified
- `crates/paladin-llm/src/services/commissary.rs` (1009 lines) — `CommissaryPlan.fallback_context_tokens`
  (~74), `CommissaryError::UndeclaredContextWindow` (~239), `struct Commissary` + `Debug`
  (~300-316), `new` (~338-380, the inline window guard at ~359), `from_port` (~389-401),
  `window()` (~408), `allotted_tokens` (~415), `exact_tally` set at ~547, `MockCounter` (~610),
  nine `Commissary::new(` test call sites (~632-995).
- `crates/paladin-llm/src/services/mod.rs` and `crates/paladin-llm/src/lib.rs` (module list
  ~53-172) — where the new `window` module registers; `src/lib.rs:~196-203` — the facade
  Commissary re-export block the resolver re-export joins.
- `src/application/services/paladin/middleware/history.rs` (622 lines) — module docs (D-14
  order), `LimitSource` (~58-83), `HistoryTrimmer`/`new` (~89-113), `resolve_limit` (~115-123),
  the three precedence tests (~297-333) and the trim tests that are the equivalence corpus.
- `src/config/agent_runtime.rs:~698-728` — `HistoryTrimmerConfig` (`default_context_tokens`,
  `model_context_limits: HashMap<String, u32>`); `~384` — the one production `HistoryTrimmer::new`
  site; `src/application/services/paladin/middleware/summarization.rs:~165` — the embedded
  trimmer construction.
- `crates/paladin-ports/src/output/llm_port.rs:~1170-1190` — `ProviderCapabilities.max_context_tokens: Option<u32>`.

### Docs being edited
- `docs/src/architecture/commissary.md` (~60-125: usage sketch with `/* is_exact_counter */`,
  "Honesty about exactness"), `docs/src/architecture/crate-map.md:178`,
  `docs/src/user-guides/memory-management.md:159`, `docs/src/api-reference/upgrading.md`,
  `docs/src/api-reference/migration-guide.md`; `docs/book.toml` (`warning-policy = "error"`),
  `.github/workflows/docs.yml` (pinned mdbook versions).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `HistoryTrimmer::resolve_limit` + `LimitSource` are the resolver in miniature: the three-step
  walk and the "name the source" pattern move to `paladin_llm::window` almost verbatim, with a
  fourth source (caller fallback) and the strict policy added.
- `history.rs`'s existing precedence tests (`limit_resolution_prefers_config_table`,
  `..._falls_back_to_provider_capabilities`, `..._falls_back_to_the_default`) and its
  `CapabilityOnlyLlmPort` test double are the template for the resolver's own precedence tests.
- `commissary.rs`'s `MockCounter` and the `commissary()` test helper (~631) are the seams for
  the Commissary equivalence fixtures and for a configurable `is_exact`.
- `TiktokenCounter`'s `impl TokenCounterPort` already contains the exact-count path; removing
  the legacy trait is mostly inlining `count_tokens` into `count`.
- `MIGRATION.md` §9.2 rows for `PaladinResult`, `LlmRequest`, `TokenUsage` and the
  `.cargo/semver-checks-allowlist.toml` entries from Phase 31 are the copy templates.

### Established Patterns
- Ports are small, synchronous, infallible where the operation is pure (`TokenCounterPort`,
  `GarrisonPort` shape) — `is_exact` follows: no `Result`, no `async`, a defaulted method.
- The facade's application layer already imports `paladin_llm::` in sixteen files
  (`resilience.rs`, `limits.rs`, `summarization.rs`, …), so `history.rs` consuming
  `paladin_llm::window` is the house pattern, not a new dependency edge.
- Feature-gated adapters are re-exported behind the same feature at every site
  (`#[cfg(feature = "content-processing")]` in `garrison/mod.rs` and the facade) — narrowing
  keeps the gates.
- Errors are per-module `thiserror` enums converted at boundaries — the resolver's error is
  its own small type, mapped into `CommissaryError::UndeclaredContextWindow` inside `new`.
- Semver-breaking rows and allowlist entries land in the same commit as the code that breaks
  (Phase 29 D-04, Phase 31 D-27).

### Integration Points
- `TokenCounterPort` (ports) ← `HeuristicTokenCounter`, `TiktokenCounter` (memory) ←
  `Commissary` (llm), `HistoryTrimmer`/`SummarizationMiddleware` (facade) — `is_exact` flows
  port → adapters → Commissary's `Stockpile::exact_tally`.
- `paladin_llm::window::resolve_context_window` ← `Commissary::new` (strict, caller fallback)
  and `HistoryTrimmer::resolve_limit` (lenient, config table + default) — the single shared
  seam PRIM-04 creates; `agent_runtime.rs:~384` and `summarization.rs:~165` are the production
  `HistoryTrimmer::new` sites whose behaviour must not change.
- Facade re-exports: `src/lib.rs:~200` (Commissary block) gains the resolver types;
  `src/infrastructure/adapters/garrison/mod.rs` loses two names at two sites.
- Release register: `MIGRATION.md` §9.2 ↔ `.cargo/semver-checks-allowlist.toml` ↔ per-crate
  `Cargo.toml` lint tables ↔ `ci.yml` `semver` job; `CHANGELOG.md` `[0.10.0]`.

</code_context>

<specifics>
## Specific Ideas

- Equivalence fixtures use distinct, non-round numbers (e.g. table `1_234`, capability `8_765`,
  default `4_321`, fallback `2_222`, reserve `321`) so a swapped step cannot pass by coincidence
  (Phase 31 house style).
- The resolver's strict-refusal test must assert the error names the provider (Commissary's
  `UndeclaredContextWindow { provider }` text is user-facing and unchanged).
- The `is_exact` port doc test: a bare `impl TokenCounterPort for AlwaysOne` (the existing doc
  example) plus `assert!(!counter.is_exact())` — the default proven on the port itself.
- Write "v0.10.0" in every new doc and row, never "v0.11.0" (Phase 30 D-18).
- Rustdoc for `TiktokenCounter::is_exact` says exact "for the encoding resolved at `new`";
  never claim exactness for arbitrary `model` strings passed to `count`.
- Semver discovery commands (D-14), run from the repo root with the CI pin
  `cargo-semver-checks@0.50.0`:
  `cargo semver-checks check-release --package paladin-memory --default-features --baseline-version 0.9.0 --release-type minor`
  and `cargo semver-checks check-release --package paladin-memory --features content-processing --baseline-version 0.9.0 --release-type minor`
  (same pair for `paladin-ai`; default-features only for `paladin-ports` and `paladin-llm`).
  `--release-type minor` is mandatory on every discovery run (D-14); without it the tool skips all
  lints for a 0.9.0 → 0.10.0 bump and reports nothing.

</specifics>

<deferred>
## Deferred Ideas

- A per-model context-window override table on `CommissaryPlan` (would make step 1 live for
  Commissary) — new capability, not this phase; backlog / Phase 33+ if a caller needs it.
- A production caller for `Commissary` and replacing RAG's `content.len() / 4` truncation —
  Phase 33 (COMM-01…03); re-sealing the Phase 29 release gates — Phase 33 (COMM-04).
- Placing the resolver in `paladin-ports` instead of `paladin-llm` (pure function over
  `ProviderCapabilities`) — considered, rejected for this phase because ROADMAP success
  criterion 4 and PRD R4 lock `paladin-llm`; revisit only if a `paladin-ports`-only consumer
  appears.
- `#[non_exhaustive]` on `CommissaryPlan` / `Commissary` — not needed while the milestone is a
  clean-break window; a later phase can decide.
- Treasurer, pricing, `cost_estimate`, allowances, pacing — Milestone 14 (ADR-0050).

### Reviewed Todos (not folded)
- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, score 0.9) —
  matched only on generic keywords ("test, crates, paladin"); it is an object-storage
  infrastructure evaluation with no overlap with token primitives. Folding it would violate the
  scope guardrail, so the mechanical ≥ 0.4 auto-fold rule was deliberately not applied. It stays
  pending with no `resolves_phase` tag, as its own text requires.

</deferred>

---

*Phase: 32-unified-token-primitives*
*Context gathered: 2026-09-15 via `--auto`*
