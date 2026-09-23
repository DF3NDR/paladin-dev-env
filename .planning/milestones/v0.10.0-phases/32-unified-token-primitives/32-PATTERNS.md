# Phase 32: Unified Token Primitives - Pattern Map

**Mapped:** 2026-09-15
**Files analyzed:** 9 (1 new, 8 modified)
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|---------------|
| `crates/paladin-llm/src/window.rs` (new; `paladin_llm::window`) | utility (pure resolver) | transform | `src/application/services/paladin/middleware/history.rs` `resolve_limit`/`LimitSource` (facade) + `crates/paladin-llm/src/services/commissary.rs` `CommissaryError` (thiserror enum shape) | role-match (composite: precedence logic from one, module/error shape from the other) |
| `crates/paladin-ports/src/output/token_counter_port.rs` | service port (trait) | transform | itself (adding a defaulted method to an existing trait) | exact — this is the file being edited, its own doc test is the template |
| `crates/paladin-memory/src/token_counter/heuristic.rs` | service (adapter, CRUD-less pure fn) | transform | `token_counter_port.rs` doc test + `garrison/token_counter.rs`'s existing `TokenCounterPort` impl tests | exact |
| `crates/paladin-memory/src/garrison/token_counter.rs` | service (adapter) | transform | `token_counter_port.rs` (trait it implements); its own existing `impl TokenCounterPort for TiktokenCounter` block | exact |
| `crates/paladin-llm/src/services/commissary.rs` | service | request-response (guard) / CRUD-less allocator | itself; `window.rs` is the new analog for the window-resolution slice it drops | exact (self-modification) |
| `src/application/services/paladin/middleware/history.rs` | middleware | transform | itself; consumes `window.rs` for `resolve_limit`'s body | exact (self-modification) |
| `crates/paladin-memory/src/garrison/mod.rs`, `prelude.rs`, `src/infrastructure/adapters/garrison/mod.rs` | re-export / facade | — | themselves (narrowing an existing `pub use` line) | exact |
| `MIGRATION.md` §9.2 / `.cargo/semver-checks-allowlist.toml` / `CHANGELOG.md` | config / docs | batch (release bookkeeping) | Phase 31's `PaladinResult`/`LlmRequest`/`TokenUsage` rows and matching allowlist `[[entry]]`/CHANGELOG bullets | exact — explicit templates named in CONTEXT.md D-15/D-16 |
| `docs/src/architecture/commissary.md` + `crate-map.md`/`memory-management.md`/`upgrading.md`/`migration-guide.md` | docs | — | their own current prose (being edited in place) | exact |

## Pattern Assignments

### `crates/paladin-llm/src/window.rs` (new module, transform)

**Analogs:** `src/application/services/paladin/middleware/history.rs` (precedence walk shape) +
`crates/paladin-llm/src/services/commissary.rs` (module-doc style, thiserror enum style, where it
sits in `lib.rs`'s module list and the facade re-export block).

**Precedence-walk pattern to generalize** (`history.rs` lines 60-123, read in full):
```rust
/// Which of D-14's three resolution steps produced a limit -- named so the
/// debug log can say exactly which one, rather than just the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LimitSource {
    /// `config.model_context_limits.get(model)` had an entry.
    ConfigTable,
    /// The constructor's `llm_port.get_capabilities().max_context_tokens`
    /// had a value.
    ProviderCapabilities,
    /// Neither of the above -- `config.default_context_tokens`.
    Default,
}

impl LimitSource {
    fn as_str(self) -> &'static str {
        match self {
            LimitSource::ConfigTable => "model_context_limits config table",
            LimitSource::ProviderCapabilities => {
                "provider capabilities (get_capabilities().max_context_tokens)"
            }
            LimitSource::Default => "default_context_tokens",
        }
    }
}

/// D-14's three-step resolution order, returning both the resolved
/// limit and which step produced it.
fn resolve_limit(&self, model: &str) -> (u32, LimitSource) {
    if let Some(&limit) = self.config.model_context_limits.get(model) {
        return (limit, LimitSource::ConfigTable);
    }
    if let Some(max_context_tokens) = self.llm_port.get_capabilities().max_context_tokens {
        return (max_context_tokens, LimitSource::ProviderCapabilities);
    }
    (self.config.default_context_tokens, LimitSource::Default)
}
```
The new `window::resolve_context_window` generalizes this exact shape by inserting a fourth,
policy-carried branch (`WindowFallbackPolicy::Strict { caller_fallback }`) instead of always
falling through to `config.default_context_tokens`. Keep the doc-comment convention of naming
*why* the source is tracked ("why did my history get trimmed at 8192") — copy that sentence style
into `WindowSource`'s own doc comments so `HistoryTrimmer`'s module docs (lines 1-45, own file,
D-14 cross-reference) stay accurate once `resolve_limit`'s body becomes a call-through.

**Module-doc and thiserror-enum style to copy** (`commissary.rs` lines 1-40, read in full — module
doc opens with a one-line role statement, then a "Two/Three responsibilities" bulleted breakdown,
then a **bolded rule name** paragraph, e.g. `**Honesty clause.**`; error enums are `thiserror`,
one `#[error(...)]` per variant, matching the house pattern already required by
`security.instructions.md`/`rust.instructions.md`):
```rust
//! The Commissary — the officer who issues rations under scarcity.
//! ...
//! **Honesty clause.** Not every model has an exact tokenizer available offline
//! (`claude-*`, `deepseek-*`). ...
```
`window.rs`'s own module doc should open the same way: one-line role statement ("the shared
context-window precedence resolver every caller of `ProviderCapabilities` should call through"),
then the three/four-step precedence list, then a **bolded rule** paragraph for the strict-vs-lenient
distinction (mirrors ADR-0010 refusal semantics D-01 cites).

**Error type shape to copy** — `UnknownContextWindow` should follow the same one-`#[error(...)]`-
per-variant `thiserror` convention already used throughout `commissary.rs`'s `CommissaryError`
(not read in full here, but its Display text for `UndeclaredContextWindow` is quoted verbatim in
RESEARCH.md's Existing Code Insights and MUST be preserved byte-for-byte when `Commissary::new`
maps the new error into it — do not change the wording, only the source of the value).

**Registration points** (verified locations, not yet edited):
- `crates/paladin-llm/src/lib.rs` module list (~lines 53-172) gains `pub mod window;` alongside
  the existing `pub mod services;` — follow the existing one-line-per-module style there.
- Facade `src/lib.rs` (~line 200, the existing Commissary re-export block) gains the resolver's
  public types (`WindowFallbackPolicy`/`WindowSource`/`ResolvedWindow`/`UnknownContextWindow` or
  whatever names are chosen) in the same `pub use paladin_llm::...` block style already used for
  `Commissary`/`CommissaryError`/`CommissaryPlan`.

---

### `crates/paladin-ports/src/output/token_counter_port.rs` (port trait, transform)

**Analog:** itself — the file's own existing doc test is the exact template for `is_exact`'s proof
(full file read, 77 lines).

**Current trait** (lines 67-76):
```rust
pub trait TokenCounterPort: Send + Sync {
    /// Counts the (approximate or exact) number of tokens `text` would
    /// occupy for `model`. Never fails: an adapter that does not recognise
    /// `model` falls back to its own approximation internally.
    fn count(&self, text: &str, model: &str) -> u32;

    /// A stable, adapter-identifying name (e.g. `"heuristic"`, `"tiktoken"`),
    /// usable in a debug log naming which counter produced a given count.
    fn name(&self) -> &str;
}
```

**Doc test to extend** (lines 47-66 — the `AlwaysOne` bare impl, already the exact shape D-06's
proof needs; only the trailing assertion is new):
```rust
/// ```
/// use paladin_ports::output::token_counter_port::TokenCounterPort;
///
/// struct AlwaysOne;
///
/// impl TokenCounterPort for AlwaysOne {
///     fn count(&self, _text: &str, _model: &str) -> u32 {
///         1
///     }
///
///     fn name(&self) -> &str {
///         "always-one"
///     }
///     // No `is_exact` override -- inherits the trait default.
/// }
///
/// let counter = AlwaysOne;
/// assert_eq!(counter.count("hello, world", "any-model"), 1);
/// assert_eq!(counter.count("", "an-unrecognised-model"), 1);
/// assert_eq!(counter.name(), "always-one");
/// assert!(!counter.is_exact(), "the trait default is false");
/// ```
```

**Method doc-comment convention** (mirror `count`/`name`'s own style — one-line summary,
contract in a `# Contract` bullet list at the trait level, lines 29-44):
```rust
/// Whether this adapter's `count` returns the model's own tokenizer's exact
/// tally, as constructed -- not a property of any individual `(text, model)`
/// call. Defaults to `false` (approximate); an exact adapter overrides it to
/// `true`.
fn is_exact(&self) -> bool {
    false
}
```

---

### `crates/paladin-memory/src/token_counter/heuristic.rs` (adapter, transform)

**Analog:** `token_counter_port.rs`'s doc test pattern (above) plus the file's own existing
`impl TokenCounterPort` block (not modified except adding one test).

**Pattern:** rely on the trait default — add NO `is_exact` override, add one test in the file's
existing `#[cfg(test)] mod tests` block asserting the default:
```rust
#[test]
fn is_exact_defaults_to_false() {
    let counter = HeuristicTokenCounter;
    assert!(!counter.is_exact());
}
```
Match the existing test module's naming convention in this file (snake_case, asserts on a single
behavior per test, no setup helper needed for this zero-field struct).

---

### `crates/paladin-memory/src/garrison/token_counter.rs` (adapter removal + narrowing, transform)

**Analog:** the file's own existing `impl TokenCounterPort for TiktokenCounter` tests, lines
342-370 (`tiktoken_counter_implements_the_port`,
`tiktoken_counter_falls_back_inside_the_adapter_for_an_unknown_model`) — these are the reusable
templates for the new `is_exact() == true` test; copy their structure (construct via
`TiktokenCounter::new(model)`, assert on the `TokenCounterPort` surface, not the removed trait).

**Removal targets** (confirmed by RESEARCH.md's full-file read): `trait TokenCounter` (line 13,
`count_tokens`/`model_name`), `impl TokenCounter for TiktokenCounter` (line 102), `struct
TokenCounterFactory` (line 165, `for_model`/`supported_models`/`is_supported`) — all deleted
outright, no `#[deprecated]` shim (D-09).

**What survives, with `is_exact` added** (the existing `impl TokenCounterPort for TiktokenCounter`
at line 141 — read its current body before editing; per D-10 it currently delegates to
`count_tokens` and must instead inline the BPE-lookup-plus-cache logic directly, then add):
```rust
fn is_exact(&self) -> bool {
    true
}
```
with rustdoc stating exactness holds "for the encoding resolved at `new(model)`" and that
`count`'s own `model` argument is ignored (per D-07, `token_counter.rs:~132`'s existing rustdoc
wording — reuse its phrasing, don't invent new wording).

---

### `crates/paladin-llm/src/services/commissary.rs` (self-modification, request-response/allocator)

**Analog:** itself. Module doc (lines 1-40, read in full above) needs its **Honesty clause**
paragraph (lines 32-39) rewritten — it currently says "the caller supplies `is_exact_counter`
explicitly at construction"; D-08 requires this rewritten to say exactness now comes from
`self.counter.is_exact()`, one source of truth, no cached duplicate.

**Struct field removal** (per RESEARCH.md's confirmed read, lines 300-306):
```rust
pub struct Commissary {
    counter: Arc<dyn TokenCounterPort>,
    capabilities: ProviderCapabilities,
    provider: String,
    config: CommissaryPlan,
    is_exact_counter: bool,       // DROP (D-08)
}
```

**Window-guard replacement** (current inline guard at lines 359-364, confirmed):
```rust
let window = capabilities
    .max_context_tokens
    .or(config.fallback_context_tokens)
    .ok_or_else(|| CommissaryError::UndeclaredContextWindow { provider: provider.clone() })?;
```
becomes one call to `window::resolve_context_window(..., WindowFallbackPolicy::Strict {
caller_fallback: config.fallback_context_tokens })`, mapping `Err(UnknownContextWindow { .. })` to
the SAME `CommissaryError::UndeclaredContextWindow { provider: provider.clone() }` variant with
unchanged Display text (D-04) — do not let the resolver's own `model` field leak into the mapped
error.

**`Debug` impl** (line 308-317) — replace the `is_exact_counter` field print with a live
`self.counter.is_exact()` call, matching the existing style of printing `counter.name()`
alongside it (read the current `Debug` body before editing to match its exact formatting macro
usage).

**Test-double seam** (`MockCounter`, lines 607-618, and the `commissary()`/`counter()`/
`capabilities_with_window()` helpers, lines 620-640) — this is the analog for adding a
configurable-exactness knob and for D-13's equivalence fixtures. Reuse these helpers verbatim in
the new equivalence tests rather than writing new construction helpers.

---

### `src/application/services/paladin/middleware/history.rs` (self-modification, transform)

**Analog:** itself, plus the new `window.rs` it now calls into.

**`resolve_limit` becomes a thin wrapper** (current body at lines 115-123, quoted above) — replace
the three-step `if`/`if`/fallthrough body with one call to
`window::resolve_context_window(model, Some(&self.config.model_context_limits),
&self.llm_port.get_capabilities(), WindowFallbackPolicy::Default(self.config.default_context_tokens))`,
then map `ResolvedWindow.source` back to `LimitSource` (or delete `LimitSource` and use
`WindowSource` directly — Claude's Discretion, D-05) while preserving the exact substrings the
three existing precedence tests assert on: `LimitSource::as_str()`'s current wording (lines
73-82, quoted above) contains `"config"`, `"provider"`/`"capabilities"`, `"default"` — whatever
replaces it must keep those substrings verbatim or the tests at lines ~296-333 (not re-read here,
per RESEARCH.md's confirmed locations) fail on the string assertion even though the resolved `u32`
is correct.

**Module doc convention** (lines 1-45, read in full) — the "D-14" cross-reference paragraph (lines
5-11) must be updated to point at the shared resolver's module docs instead of re-describing the
three-step order inline, matching this file's own established pattern of citing the authoritative
source (`Doc 05 RT-FR-08/10/11/12, D-14, D-15`) rather than duplicating prose.

**Imports to add** (matching this file's existing import-block style at lines 47-58):
```rust
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, warn};

use crate::application::services::paladin::error::PaladinError;
use crate::config::agent_runtime::HistoryTrimmerConfig;
use crate::core::platform::container::garrison::GarrisonEntry;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::token_counter_port::TokenCounterPort;

use super::{ExecutionMiddleware, MiddlewareFlow, ModelCallContext};
```
Add `use paladin_llm::window::{self, WindowFallbackPolicy};` (or equivalent) grouped with the
other external-crate imports (`paladin_ports::...`), matching this file's existing alphabetical/
grouped-by-crate ordering (std → external crates → crate-internal → `super::`).

---

### Re-export narrowing (`crates/paladin-memory/src/garrison/mod.rs:16`, `prelude.rs:10`,
`src/infrastructure/adapters/garrison/mod.rs:9,22-26`)

**Analog:** the files' own current lines (confirmed by RESEARCH.md's read):

| File | Current | Target |
|------|---------|--------|
| `crates/paladin-memory/src/garrison/mod.rs:16` | `pub use token_counter::{TiktokenCounter, TokenCounter, TokenCounterFactory};` | `pub use token_counter::TiktokenCounter;` |
| `crates/paladin-memory/src/prelude.rs:10` | `pub use crate::garrison::{TiktokenCounter, TokenCounter, TokenCounterFactory};` | `pub use crate::garrison::TiktokenCounter;` |
| `src/infrastructure/adapters/garrison/mod.rs:9` | `pub use paladin_memory::garrison::{TiktokenCounter, TokenCounter, TokenCounterFactory};` | `pub use paladin_memory::garrison::TiktokenCounter;` |
| `src/infrastructure/adapters/garrison/mod.rs:22-26` (`token_counter` compat sub-module) | re-exports all three | re-exports `TiktokenCounter` only |

Mechanical narrowing — no structural pattern change, keep the surrounding `#[cfg(feature =
"content-processing")]` gate exactly as-is at every site (D-11, Pitfall 3 in RESEARCH.md).

---

### Release bookkeeping (`MIGRATION.md` §9.2, `.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md`)

**Analog:** Phase 31's own rows — `PaladinResult`, `LlmRequest`, `TokenUsage` in `MIGRATION.md`
§9.2 (lines ~162-277) and their matching `.cargo/semver-checks-allowlist.toml` `[[entry]]` blocks.
Copy the row/entry shape exactly (not re-read this session — RESEARCH.md's Sources section
confirms these are the named templates); new rows needed: `paladin-llm | Commissary`,
`paladin-memory | TokenCounter`, `paladin-memory | TokenCounterFactory` (D-15), each landing in
the same commit as its allowlist `[[entry]]`.

`CHANGELOG.md` `[0.10.0]`: one `### Changed` bullet (Commissary signature + shared resolver), one
`### Removed` bullet (legacy pair), one `### Added` bullet (`TokenCounterPort::is_exact`) — mirror
Phase 31's ACCT-scoped bullet wording style (terse, names the type, names the phase requirement
ID) rather than prose paragraphs.

## Shared Patterns

### thiserror per-module error enums
**Source:** `crates/paladin-llm/src/services/commissary.rs` (`CommissaryError`, not fully quoted
here but its Display text is authoritative per RESEARCH.md) and the general house convention in
`.github/instructions/rust.instructions.md` / `security.instructions.md`.
**Apply to:** `window.rs`'s `UnknownContextWindow` error type — one `#[derive(Debug,
thiserror::Error, ...)]`, one `#[error("...")]` string, converted at the `Commissary::new`
boundary into the existing `CommissaryError` variant (never propagated raw across the crate
boundary into the facade).

### Doc-comment "named source" convention
**Source:** `history.rs`'s `LimitSource`/`resolve_limit` (lines 60-123, quoted above) — every
resolved value is paired with an enum naming which step produced it, and the module doc explains
*why* (debuggability: "why did my history get trimmed at 8192").
**Apply to:** `window.rs`'s `WindowSource` and `ResolvedWindow` — keep the same "the source is not
decorative, it answers an operator's question" doc-comment framing.

### Infallible, synchronous port methods
**Source:** `token_counter_port.rs`'s own module doc (lines 10-18, quoted above): "Counting a
string's length is a pure, local, CPU-bound computation -- it never needs `.await` and it never
needs a `Result`."
**Apply to:** `is_exact(&self) -> bool` — no `Result`, no `async`, matching `count`/`name`'s
existing signatures exactly.

### Feature-gated re-export narrowing without removing the gate
**Source:** existing `#[cfg(feature = "content-processing")]` placement at every one of the three
re-export sites (verified, not re-quoted — see table above).
**Apply to:** all four narrowing edits (D-11) — change only the item list inside the `pub use`,
never the surrounding `#[cfg(...)]`.

## No Analog Found

None. Every file in this phase's scope is either a self-modification of an existing, fully-read
file, or a new module (`window.rs`) with two strong analogs (the precedence-walk shape from
`history.rs`, the module-doc/error-enum style from `commissary.rs`) that together cover its full
shape. RESEARCH.md's own Architecture Patterns section additionally provides a complete design
sketch for `window.rs` (types, function signature, and a working implementation) that this
PATTERNS.md defers to rather than duplicating in full — see RESEARCH.md "Pattern 1: Fallback-policy
enum, not a bool (D-01)".

## Metadata

**Analog search scope:** `crates/paladin-ports/src/output/`, `crates/paladin-memory/src/`,
`crates/paladin-llm/src/services/`, `src/application/services/paladin/middleware/`,
`src/infrastructure/adapters/garrison/`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`,
`CHANGELOG.md`, `docs/src/architecture/`.
**Files scanned:** 9 target files (all read in full or targeted this session or in the upstream
RESEARCH.md session, per its Sources §Primary list) + 3 release-bookkeeping template files
(referenced, not re-read — RESEARCH.md already confirms their exact template rows).
**Pattern extraction date:** 2026-09-15
