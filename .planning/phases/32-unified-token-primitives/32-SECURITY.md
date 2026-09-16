---
phase: 32
slug: unified-token-primitives
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-09-16
verified: 2026-09-16
---

# Phase 32 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 32 unifies the token-counting and context-window primitives: `TokenCounterPort` gains a
defaulted `is_exact` signal, `Commissary` reads exactness live from the injected port instead of
a caller-supplied bool, the legacy fallible `garrison::TokenCounter`/`TokenCounterFactory` pair is
deleted outright, and both `Commissary` and `HistoryTrimmer` resolve their window through the
single shared `paladin_llm::window::resolve_context_window`. No file touched by this phase handles
an API key, an external response body or an outbound HTTP client, and no crate manifest gained a
dependency.

The register below was authored at plan time (all five PLAN files carry a `<threat_model>` block)
and is verified here at ASVS L1 grep depth per the workflow short-circuit rule (`threats_open: 0`,
plan-time register, `asvs_level: 1`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| counter adapter → `Commissary` → `Stockpile` consumer | an exactness CLAIM crosses here; a consumer sizes its own safety margin on it, so a false claim is acted on, not merely displayed | `is_exact` bool (integrity-sensitive) |
| caller-supplied `model` string → `TokenCounterPort::count` / tiktoken encode path | untrusted-shaped input; the infallible, never-re-resolve contract must survive the inlining | model name string (untrusted) |
| caller-supplied `model` string → resolver config-table lookup | untrusted-shaped input used only as a `HashMap` key | model name string (untrusted) |
| provider-declared `max_context_tokens` → the budget every downstream guard enforces | a wrong, absent or invented window silently changes how much prompt material is admitted or refused | context window size (integrity-sensitive) |
| provider-declared window → prompt-budget guard and history-trim budget | the resolved number decides admission/refusal; a wrong source changes enforcement, not just reporting | context window + source label |
| resolver error → `CommissaryError` Display text read by an operator | user-facing message quoted in tests, rustdoc and the mdBook page; rewording is an observable contract change | error text (operator-facing) |
| downstream crate → `paladin-memory`'s public re-export surface | published names are removed; a consumer still importing them must fail to compile rather than silently resolve something else | public API names |
| discovery-tool output → the published migration register | a guessed lint id makes the register describe a break that did not happen, or miss one that did | `cargo semver-checks` lint ids |
| this repository's register → a downstream upgrader's build | the register and CHANGELOG are the only warning a consumer gets before their build breaks | MIGRATION.md §9.2 rows, allowlist entries |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-32-01 | Spoofing | `TokenCounterPort::is_exact` implementations | medium | mitigate | Trait default is `false` (`crates/paladin-ports/src/output/token_counter_port.rs:96-98`), proven by the port's doc test (line 75). Only `TiktokenCounter` overrides to `true` (`crates/paladin-memory/src/garrison/token_counter.rs:134-136`) with rustdoc scoping the claim to the encoding resolved at `new(model)`. `HeuristicTokenCounter` writes no override; test `heuristic_is_exact_reports_false_through_the_trait_default` asserts the default. | closed |
| T-32-02 | Tampering | `Commissary`'s removed cached exactness field | medium | mitigate | `struct Commissary` has no exactness field (fields: `counter`, `capabilities`, `provider`, `config`, `resolved_window`). `Stockpile.exact_tally` is read live from `self.counter.is_exact()` (`crates/paladin-llm/src/services/commissary.rs:556`). Tests `exact_tally_true_is_read_from_an_exact_injected_port` and `exact_tally_false_is_read_from_an_approximate_injected_port` assert both directions. `git grep is_exact_counter` over code returns only the two migration doc pages. | closed |
| T-32-03 | Information Disclosure | `Commissary`'s `Debug` impl | low | mitigate | `impl Debug for Commissary` prints `provider`, `capabilities`, `config`, `counter.name()` and `counter.is_exact()` only. `CommissaryPlan` fields are all numeric limits plus `truncation_marker` and `model_hint` — no secret. Phase diff scanned for credential-shaped literals (API-key, bearer, secret, password, token patterns): none added. | closed |
| T-32-04 | Denial of Service | `TiktokenCounter::count` with an unrecognised model string | low | accept | Unchanged by this phase: `count` ignores its `_model` argument and delegates to the encoding loaded at `new`; test `tiktoken_counter_falls_back_inside_the_adapter_for_an_unknown_model` present (line 266). No `unwrap`/`expect`/`panic!` outside rustdoc examples and the test module. See Accepted Risks AR-32-01. | closed |
| T-32-05 | Tampering | `resolve_context_window`'s terminal step | high | mitigate | `WindowFallbackPolicy::Strict { caller_fallback: Option<u32> }` (`crates/paladin-llm/src/window.rs:36-49`) is a type, not a bool flag; the strict-with-`None` arm returns `Err(UnknownContextWindow)` (line 180-182). No hardcoded window constant exists in the module. Test `precedence_strict_policy_with_no_fallback_returns_the_error` asserts the refusal. | closed |
| T-32-06 | Repudiation | the resolved window's reported source | medium | mitigate | `WindowSource::as_str` is an exhaustive four-arm match with no wildcard (`window.rs:98-106`); `WindowSource::ALL` (line 76) is walked by `source_label_invariant_walks_every_variant` asserting distinct, non-empty labels. Every `Ok(ResolvedWindow)` carries its source. | closed |
| T-32-07 | Denial of Service | the config-table lookup with a hostile model string | low | accept | Lookup is `Option<&HashMap<String, u32>>` keyed by the borrowed string (`window.rs:151`); no parsing, regex, I/O or size-proportional allocation. Resolver is synchronous and pure. See Accepted Risks AR-32-02. | closed |
| T-32-08 | Information Disclosure | `UnknownContextWindow`'s Display text and new rustdoc | low | mitigate | `#[error("no context window could be resolved for model '{model}'")]` (`window.rs:125`) embeds only the caller-supplied model name. Phase diff scan found no credential-shaped literal in new rustdoc. | closed |
| T-32-09 | Denial of Service | the inlined `count` path in the tiktoken adapter | medium | mitigate | `impl TokenCounterPort for TiktokenCounter::count` (`token_counter.rs:101-124`) performs cache read → `self.bpe.encode_with_special_tokens` → bounded cache write with the 1000-entry ceiling carried over (line 117); no `get_bpe_from_model` on the count path. Unknown-model test stays green; port infallibility clause preserved. | closed |
| T-32-10 | Tampering | removal of a published counting contract | medium | mitigate | `garrison/token_counter.rs` contains no `pub trait` and no `TokenCounterFactory`; names are deleted, not aliased. `git grep` over `crates src docs/src examples benches tests` hits only the two reader-facing migration pages. MIGRATION.md §9.2 rows for `paladin-memory | TokenCounter` and `paladin-memory | TokenCounterFactory` exist with matching `.cargo/semver-checks-allowlist.toml` entries (`trait_missing`, `struct_missing`). | closed |
| T-32-11 | Tampering | the four narrowed re-export sites | low | mitigate | Every `pub use ...TiktokenCounter` in `crates/paladin-memory/src/garrison/mod.rs`, `prelude.rs`, and both sites in `src/infrastructure/adapters/garrison/mod.rs` sits directly under an intact `#[cfg(feature = "content-processing")]`. VERIFICATION.md records `cargo check --workspace --all-features --all-targets` green; SUMMARY records the default-features check green. | closed |
| T-32-12 | Information Disclosure | rewritten rustdoc, the crate map and the memory-management guide | low | mitigate | Phase diff over `docs/src` and touched crates scanned for credential-shaped literals: none added. Doc examples use only type names, feature names and numeric literals. | closed |
| T-32-13 | Tampering | the window `Commissary` enforces after the swap | high | mitigate | `Commissary::new` calls `resolve_context_window(..., None config table, ..., WindowFallbackPolicy::Strict { caller_fallback: config.fallback_context_tokens })` once (`commissary.rs:367-375`) and stores `resolved_window`. Equivalence fixture `window_and_allowance_equivalence_snapshot_pre_resolver` (line 1022), committed green pre-resolver in `0e7434ba`, asserts identical resolved numbers across all three cases. | closed |
| T-32-14 | Repudiation | the error an operator sees when no window can be resolved | medium | mitigate | The resolver error is mapped via `.map_err(|_unknown| CommissaryError::UndeclaredContextWindow { .. })` (`commissary.rs:375`) into the pre-existing variant; the `CommissaryError` enum still carries its five original variants with unchanged `#[error]` text. Test at line 844 and the equivalence fixture both match on `UndeclaredContextWindow`. | closed |
| T-32-15 | Repudiation | the trim source an operator reads in the debug line | medium | mitigate | `HistoryTrimmer::resolve_limit` returns `(u32, WindowSource)` (`src/application/services/paladin/middleware/history.rs:97-114`) and logs `source.as_str()` (line 151). `git grep LimitSource -- src crates` returns nothing — the duplicate enum is deleted. Tests `limit_resolution_prefers_the_config_table`, `..._falls_back_to_provider_capabilities`, `..._falls_back_to_the_default` assert the preserved `config`/`provider|capabilities`/`default` substrings. | closed |
| T-32-16 | Denial of Service | a panic introduced while handling the always-`Ok` lenient result | medium | mitigate | `resolve_limit` matches the `Result` with an explicit `Err` arm that returns the configured default (`src/application/services/paladin/middleware/history.rs:99-113`). Grep for `unwrap()`/`expect(`/`panic!` outside the `#[cfg(test)]` module of `commissary.rs` and `history.rs`: none. `cargo clippy --workspace --all-targets --all-features -- -D warnings` recorded green. | closed |
| T-32-17 | Repudiation | the semver discovery runs | high | mitigate | 32-05-SUMMARY records six `cargo semver-checks check-release` commands verbatim, all carrying `--release-type minor` (13 occurrences of the flag in the file), each with a captured `Checked [...] N checks` line (194–196 checks, never zero). The only lint ids written to rows/allowlist (`trait_missing`, `struct_missing`) appear in the captured Run 5 output. | closed |
| T-32-18 | Tampering | a feature-gated break invisible to CI | high | mitigate | Runs 5 and 6 are `--features content-processing` passes for `paladin-memory` and `paladin-ai`. Run 5 fires `trait_missing`/`struct_missing`; both allowlist justifications state plainly that CI's `--default-features` semver job cannot observe the removal. No `[package.metadata.cargo-semver-checks.lints]` allow was added for the gated case, so a future default-features occurrence still surfaces. | closed |
| T-32-19 | Tampering | the register-to-allowlist set equality | medium | mitigate | The CI row-level set-equality script (copied verbatim from `ci.yml`) was re-run locally in 32-05 and independently re-run in 32-VERIFICATION.md: 15 register pairs = 15 allowlist pairs in both directions, exit 0. Rows and entries landed in the same commit (`54598f1a`). The `paladin-llm | Commissary` row is `N/A` by design (no lint exists for inherent-method arity) and is correctly excluded from the scan. Re-audit 2026-09-16: re-run through `make check-migration-allowlist` (`scripts/check-migration-allowlist.sh`, a verbatim offline mirror of the CI step that landed in `93ddd678` after the first audit): 15 register pairs = 15 allowlist pairs, exit 0. | closed |
| T-32-20 | Information Disclosure | new register, CHANGELOG, doc and Cargo.toml text | low | mitigate | Phase diff over MIGRATION.md, CHANGELOG.md, docs and manifests scanned for credential-shaped literals: none. 32-05-SUMMARY records the manual credential-handling review verdict explicitly (no API key, response body or HTTP client touched; the three checks are not engaged). | closed |
| T-32-21 | Denial of Service | the coverage gate reported without being run | medium | mitigate | 32-05-SUMMARY records a measured local run of `scripts/coverage.sh` (the CI-equivalent path, `--fail-under-lines 82`) with the `cargo llvm-cov report --summary-only` line reproduced verbatim: 90.25% lines against the 82% floor. The MinIO/docker blocker is named and explained (services already reachable). 32-VERIFICATION.md accepts this figure rather than claiming an independent re-run. | closed |
| T-32-SC | Tampering | npm/pip/cargo installs (supply chain; listed in all five plans) | low | accept | `git diff 791dca94^..HEAD` over the root and every crate `Cargo.toml` adds no dependency line and no non-comment line; `Cargo.lock` is unchanged across the phase. `make security` (cargo-audit + cargo-deny) recorded green in 32-05 (`advisories ok, bans ok, licenses ok, sources ok`). See Accepted Risks AR-32-03. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-32-01 | T-32-04 | An unrecognised model string passed to `TiktokenCounter::count` is never looked up: the encoding was resolved fallibly at `new(model)` and `count` delegates to it unconditionally. The worst case is an approximate-but-labelled-exact tally for a mismatched model, which is the documented scope of the `is_exact` claim, not a crash or hang. Pre-existing behaviour carried over unchanged and covered by the existing unknown-model test. | plan 32-01 threat model, verified 2026-09-16 | 2026-09-16 |
| AR-32-02 | T-32-07 | The config-table lookup is a plain `HashMap<String, u32>::get` on the borrowed caller string. No parsing, regex, I/O or attacker-proportional allocation exists, and the resolver is synchronous. Any string, including empty or adversarial input, is safe by construction; no mitigation beyond the type is needed. | plan 32-02 threat model, verified 2026-09-16 | 2026-09-16 |
| AR-32-03 | T-32-SC | Not applicable: no `Cargo.toml` in the workspace gained a dependency during Phase 32 and `Cargo.lock` is byte-identical across the phase's commit range. `make security` passed at the phase gate. The `qdrant-client` version-drift note in 32-05 is confined to `cargo-semver-checks`' own from-scratch resolution and does not affect the locked workspace build. | plan 32-01..05 threat models, verified 2026-09-16 | 2026-09-16 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-16 | 22 | 22 | 0 | /gsd-secure-phase (orchestrator, L1 grep-depth short-circuit; auditor not spawned per workflow rule) |
| 2026-09-16 (re-audit) | 22 | 22 | 0 | /gsd-secure-phase (State A re-audit; L1 grep-depth short-circuit; auditor not spawned per workflow rule) |

**Audit notes (2026-09-16):**

- All five PLAN files carry a `<threat_model>` block; `register_authored_at_plan_time: true`.
- SUMMARY threat flags: 32-01, 32-02, 32-04 and 32-05 each record "None — no new surface outside the register". **32-03-SUMMARY.md has no `## Threat Flags` section at all.** This is an artifact-completeness gap, not a threat: plan 32-03's touched surface (the inlined `count` path, the deleted trait/factory, the four narrowed re-exports, the rewritten docs) is exactly T-32-09..T-32-12, and each was verified directly against the tree above. No unregistered surface was found.
- T-32-SC appears in all five plans with identical disposition; it is recorded once here.
- No implementation file was modified by this audit.
- The two findings in 32-REVIEW.md (WR-01, IN-01) are stale line-number citations in `docs/src/architecture/commissary.md` and have no security relevance.

**Audit notes (2026-09-16, re-audit):**

- Commits landed after the first audit: `16a3f87d` (this file), `93ddd678` (Nyquist validation
  tests) and `7891aed5` (32-VALIDATION.md). `93ddd678` touched only `Makefile`,
  `scripts/check-migration-allowlist.sh` and `tests/scripts/check-migration-allowlist_test.sh` —
  no crate source, manifest or lockfile changed, so no register row's implementation moved.
- The new script is an offline guard that reads two tracked files and writes only to a
  `mktemp -d` directory; it adds no network, credential or external-input surface. It is now
  the local re-run path for T-32-19 and was executed green in this audit.
- Every code-level mitigation (T-32-01..T-32-16) was re-grepped against the current tree and
  matched the cited lines; the `HistoryTrimmer` citations in T-32-15/T-32-16 were previously
  unqualified and now name the facade path `src/application/services/paladin/middleware/history.rs`.
- `Cargo.lock` and every `Cargo.toml` remain free of added dependency lines across
  `791dca94^..HEAD`; the credential-shaped-literal scan over the phase diff is still empty.
- No implementation file was modified by this audit.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-16
