---
phase: 46
slug: docs-currency-hygiene
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-10-24
---

# Phase 46 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — every one of the fourteen PLAN.md files (46-01 through 46-14) carries a `<threat_model>` block. Verification depth: ASVS L1 (grep-level presence checks against the implementation and phase artifacts), per the short-circuit rule for a plan-time register with `threats_open: 0`. Every SUMMARY.md `## Threat Flags` section reports "None" beyond the plan register. Threat ids shared by several plans (T-46-03, T-46-04 across 46-02/03/04; T-46-20 across 46-10/11; T-46-SC across six plans) are listed once.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| release evidence -> operator docs | the release appendix is what an operator trusts when deciding whether a crate can publish over OIDC | Trusted Publishing / Credential History rows |
| archived evidence -> milestone audit | `audit-milestone` reads `status` / `nyquist_compliant` as compliance facts | VALIDATION.md frontmatter |
| contributor manifest edit -> inward crates | a dependency added to `paladin-core` / `paladin-ports` crosses the hexagonal purity boundary | `[dependencies]` keys |
| local gate -> CI | CI runs each script itself; a gate missing from `ci.yml` is bypassed on every pull request | gate exit codes |
| Paladin public surface -> downstream consumer | an unpinned item can break web3sec's single bump without any in-tree signal | public types and fns |
| docs -> operator config | operators copy YAML from the Treasurer pages into deployments | config keys, secret names |
| operator webhook URL -> outbound HTTP | the notice webhook is a caller-chosen URL carrying a signature header | signed payload (SSRF-guarded) |
| stored document -> `BattalionConfig` / `ConclaveConfig` | consumer-persisted v0.10 JSON or YAML is read by v0.11 | removed-key names only |
| refusal message -> logs / operators | the deserialization error is printed and logged | key names, fixed text; never values |
| operator `aegis` policy -> Commander Maneuver path | a policy the pattern cannot honour is dropped | `warn!` naming the fields |
| semver gate -> published API | crate-wide allows can hide an accidental public removal | lint allow lists |
| MIGRATION.md register -> semver CI gate | the allowlist's set-equality is computed from the headings the awk window reads | 9.2 / 10.2 register rows |
| guide / release note / book page -> downstream consumer | web3sec follows section 10 and its citations to adapt | citation targets |
| comment edit -> shipped code | a retarget edit must not change a production line | `.rs` comment lines only |
| planning ledger -> next phase | STATE.md and the todo tell Phase 47 and web3sec what remains | open WINDOWS row ids |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-46-01 | Repudiation | `paladin-eval` Trusted Publishing row | low | mitigate | evidence class "proven by publish", run 35659477719 (46-01 SUMMARY Evidence) | closed |
| T-46-02 | Tampering | historical Credential History rows | low | mitigate | rc rows amended by parenthetical only; three historical markers counted (46-01) | closed |
| T-46-03 | Repudiation | VALIDATION.md compliance claims (22, 24, 28, 29, 30, 34, 36, 36.1) | medium | mitigate | flags set only under the step-4 rule; every row cites a HEAD re-run, seal or UAT pass; `## Gaps` reads None (46-02/03/04 SUMMARYs) | closed |
| T-46-04 | Tampering | tests / implementation edited to close a gap | medium | mitigate | reconcile-only mode; empty diff under `crates src tests benches examples`; one VALIDATION.md per commit (46-02/03/04) | closed |
| T-46-05 | Tampering | foreign crate entering `paladin-core` / `paladin-ports` | high | mitigate | `scripts/check-dependency-allowlist.sh` exact-key guard over `[dependencies]`, build and target tables; `tests/scripts/check-dependency-allowlist_test.sh` (15 assertions) | closed |
| T-46-06 | Tampering | guard silently re-pinned to bless drift | medium | mitigate | ADR-0015 dated amendment 2026-10-24 lists both sets; script header cites ADR-0015 and requires a same-commit note | closed |
| T-46-07 | Tampering / Repudiation | WINDOWS ledger corruption | medium | mitigate | sequential `gsd-tools windows` writes with before/after `status` counts (46-05 Ledger evidence) | closed |
| T-46-08 | Elevation of Privilege | gate bypassed in CI | medium | mitigate | `ci.yml` cargo-deny job runs `./scripts/check-dependency-allowlist.sh` and `make test-shell-guards`; `make check-gates` member | closed |
| T-46-09 | Denial of Service | consumer build broken by an unpinned surface change | low | mitigate | four compile-time pins in `tests/consumer_surface_muster.rs`, each shown red then green; `make check-consumer-surface` passes | closed |
| T-46-10 | Information Disclosure | docs instructing a secret into YAML | medium | mitigate | configuration.md names only `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`; secret row reads "never serialised, never echoed" | closed |
| T-46-11 | Spoofing / SSRF | overstated webhook-URL guard | medium | mitigate | treasurer.md restates the boot- and send-time address guard and names DNS rebinding as a documented limitation | closed |
| T-46-12 | Information Disclosure | `config.example.yml` carrying a real key | low | mitigate | only `${VAR}` placeholders and commented defaults; example-config tests pass | closed |
| T-46-13 | Elevation of Privilege | docs implying an admin bypass of allowances | low | mitigate | treasurer.md: "no role, admin keys included, bypasses an allowance" | closed |
| T-46-14 | Denial of Service / Tampering (by omission) | v0.10 bounded run loading as unbounded in v0.11 | high | mitigate | `legacy_keys.rs` refuses `timeout_seconds` / `retry_policy` / `error_strategy` direct and nested, naming key, `aegis.*` replacement and M-B-06; 15 tests | closed |
| T-46-15 | Information Disclosure | refusal echoing a key's value | medium | mitigate | message built from key names and fixed text; test `the_refusal_never_echoes_a_value` | closed |
| T-46-16 | Denial of Service | forward-compat break from refusing unknown keys | medium | mitigate | no `deny_unknown_fields`; unknown-key tests on both types | closed |
| T-46-17 | Repudiation | Maneuver ignoring `on_error` / `retry` silently | low | mitigate | `maneuver_ignored_aegis_policies` (commander.rs:315) drives one `warn!` per execution; unit test; rustdoc and M-B-11 state it | closed |
| T-46-18 | Tampering | crate-wide semver allows masking an accidental removal | medium | accept | dated removal condition 2026-10-24 (Phase 46, D-24 / WR-05) above both lint tables; see Accepted Risks AR-46-02 | closed |
| T-46-19 | Tampering | semver register gate after the MIGRATION.md move | high | mitigate | awk window `/^## (9\|10)\.2 /` byte-identical in `scripts/check-migration-allowlist.sh` and `ci.yml`; harness proves 10.2 read, 10.1 not | closed |
| T-46-20 | Denial of Service | CI placeholder-token step failing | low | mitigate | `grep -c TBD MIGRATION.md` prints 0 on HEAD | closed |
| T-46-21 | Repudiation | a row dropped or reworded in the move | medium | mitigate | 456/456 non-blank lines found verbatim in part 10; part 9 diff against v0.10.1 empty; 22 M-B and 26 DR ids preserved (46-10 Proof records) | closed |
| T-46-22 | Repudiation | guide misstating a timeout outcome under Absorb | medium | mitigate | G-3 in M-B-06, DR-9 and the `BattalionError` row state both outcomes (46-11) | closed |
| T-46-23 | Information Disclosure / integrity | stale vendor or call-count facts | low | mitigate | G-7 counts cited to `35fd839`; G-4 worded as web3sec's own dated statement | closed |
| T-46-24 | Repudiation | release-note citations wrong or history rewritten | low | mitigate | 188-line classification table drove every edit; history sections byte-identical to plan base (46-12) | closed |
| T-46-25 | Tampering | allowlist or manifest value edited while retargeting | medium | mitigate | value-line diff empty; `cargo metadata` parses; `make check-migration-allowlist` exits 0 | closed |
| T-46-26 | Repudiation | book Upgrading page diverging from the register | low | mitigate | page links rows by id; authoritative text stays in MIGRATION.md (D-05) | closed |
| T-46-27 | Information Disclosure / integrity | unverified vendor listing stated as fact | low | mitigate | DeepSeek note dated 2026-10-24 and attributed to the consumer's live check | closed |
| T-46-28 | Repudiation | open ledger rows hidden from Phase 47 | low | mitigate | STATE.md "Open WINDOWS rows at Phase 46 close" bullet lists the 13 open ids | closed |
| T-46-29 | Repudiation | code-comment / book citations wrong or v0.10 citation rewritten | low | mitigate | 26-line classification table; keep rows listed; recipe re-run prints only keeps (46-14) | closed |
| T-46-30 | Tampering | production `.rs` line changed while retargeting comments | medium | mitigate | comment-only diff filter empty for all thirteen files; `cargo check`, legacy guard and `make api-surface` unchanged | closed |
| T-46-SC | Tampering | package installs (supply chain) | low | accept | no dependency added or changed in any plan; guard uses python3 stdlib `tomllib`; see AR-46-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-46-01 | T-46-SC | No dependency was added or changed anywhere in the phase; the one new script uses only python3's standard library. Nothing to transfer or mitigate. | plan author (46-01, 05, 06, 08, 09, 10 PLAN threat models) | 2026-10-24 |
| AR-46-02 | T-46-18 | The crate-wide `cargo-semver-checks` allows in paladin-core and paladin-battalion cannot be removed until 0.11.0 is the published semver baseline; a dated removal condition is recorded above both lint tables and the residual risk is accepted until Phase 47 ships. | plan author (46-09 PLAN, D-24 / WR-05) | 2026-10-24 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-24 | 31 | 31 | 0 | gsd-secure-phase orchestrator (L1 grep-level, short-circuit: plan-time register, threats_open 0) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-24
