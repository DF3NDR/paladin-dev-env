# Phase 29: Program Gates & Release - Context

**Gathered:** 2026-09-09
**Status:** Ready for planning
**Mode:** `--auto` — every decision below took the recommended option without a user prompt.
Each carries its rationale and the rejected alternative so a human can audit or reverse it.
Nothing here re-litigates a decision locked in Phases 22–28; those are carried under
"Prior-phase decisions that constrain this phase" in the canonical refs.

<domain>
## Phase Boundary

Phase 29 builds nothing new. It **proves the v0.10.0 program is complete and cuts the release**:

- **SHIP-01** — `MIGRATION.md` is closed: every §9 section filled, zero `TBD`, M-B-01…03 resolved
  with worked examples (already landed), the §9.2 register matching the `cargo semver-checks`
  allowlist exactly, §9.8 operator checklist written, and the file linked from the README (already
  true, `README.md:97`) and a new mdBook "Upgrading" page.
- **SHIP-02** — backward compatibility is *proven*: an integration test boots v0.10 with the v0.9
  sample config and asserts legacy behavior (every new subsystem off), and a golden diff of
  `openapi.json` restricted to the six pre-existing paths is empty.
- **SHIP-03** — the doc-08 verification protocol (`.project/v0.10.0/08-traceability-matrix.md`
  lines 98–108, ten steps) is run and its findings recorded: E2E-1/2/3 green, every FR has a
  passing test, no orphan behavior, ubiquitous-language names conform, BUG-01's old path is
  grep-absent with RED-then-GREEN visible in history.
- **SHIP-04** — v0.10.0 is releasable: all twelve publishable crates at `0.10.0`, changelogs
  finalized, `cargo publish --dry-run` green in dependency order, mdBook + rustdoc updated with no
  new broken intra-doc links, semver and MSRV CI jobs green on the release commit.

Out of scope: any behavioral change to shipped code (X-03), any rename of a public item, any
new capability, the actual `git tag` / crates.io publish (that is `make release` on `main`
after the PR merges, per ADR-0043/0044 — this phase makes the release commit *ready*).

</domain>

<decisions>
## Implementation Decisions

Decision numbering continues the house style (D-01 …). Reversibility ratings follow
`gsd-core/references/planner-reversibility.md`; unrated decisions are plainly reversible.
⚠ marks a decision the developer may want to overturn at plan review.

### MIGRATION.md closure & the Upgrading page (SHIP-01)

- **D-01: "No TBD" becomes a durable CI gate, not a one-time grep.** A new step in the
  `semver` job of `.github/workflows/ci.yml` (beside the existing allowlist set-equality step)
  fails when `grep -c 'TBD' MIGRATION.md` is non-zero. Today the file carries exactly three
  `TBD`s: the header note (line 7–8, describing the convention itself), §9.5 line 318 (the
  SHIP-02 boot-test claim) and §9.8 line 630 (the checklist). All three are closed by this
  phase; the header note is rewritten to describe the *closed* state. Rejected: recording a
  one-off grep in the audit — every prior phase's VERIFICATION had to recalibrate around
  "TBDs owned by a later phase", and a gate is what stops the pattern recurring in v0.11.
- **D-02: §9.8 is one ordered, copy-pasteable list, each step naming the concrete command or
  file, verified against the shipped tree.** Order per overview §9.8: back up state dirs/DBs →
  apply migrations (every migration in §9.4 runs automatically at adapter construction via
  `sqlx::migrate!`, so this step is "start the new binary once against a backup" plus the
  Postgres note) → update config (nothing required; §9.5's disabled-by-default claim, now
  test-backed by D-06) → adjust `terminationGracePeriodSeconds` ≥ 60 (M-B-02) → register custom
  evaluators if `EdgeCondition::Custom` is used (M-B-01) → deploy → verify. The "verify with
  `paladin-cli` health/graph-validate commands" clause is written against the commands that
  *actually exist* at HEAD — the researcher confirms the exact `paladin-cli` subcommand names
  (health, graph validate, `eval run`) and the checklist names only those; if no graph-validate
  command exists, the step names the nearest real check rather than inventing one. Rejected:
  per-subsystem sub-checklists — an operator upgrading reads one list top to bottom.
- **D-03: The mdBook "Upgrading" page is a new, hand-written
  `docs/src/api-reference/upgrading.md`, registered in `docs/src/SUMMARY.md` directly above
  the existing "Migration Guide" entry, linking `MIGRATION.md` by its repository URL.** It
  carries a two-paragraph orientation, the §9.1 behavioral-change table (M-B-01…04, one line
  each), a verbatim mirror of the §9.8 checklist, and the link. `docs/src/api-reference/
  migration-guide.md` (which stops at v0.5.0) gains a short "v0.10.0 (from v0.9.x)" section at
  the top pointing to the Upgrading page and `MIGRATION.md`, plus one line noting that 0.6–0.9
  changes are recorded in `CHANGELOG.md`. Rejected: `{{#include ../../../MIGRATION.md}}` —
  `preprocessor.links` is enabled so it would render, but `[output.linkcheck]` runs with
  `warning-policy = "error"` and MIGRATION.md's relative links to `crates/...` and `k8s/...`
  paths would fail the docs job; a hand-written page has no such exposure. Rejected: appending
  to `migration-guide.md` only — SHIP-01 names an "Upgrading" page and the existing guide is a
  historical v0.1→v0.5 document.
  — **Reversibility:** reversible — a docs page and a SUMMARY line.
- **D-04: The allowlist ↔ §9.2 CI check is tightened from crate-level to row-level.** The
  existing step compares *sorted unique crate names* (`/tmp/migration-deliberate-crates.txt`
  vs `/tmp/allowlist-crates.txt`), so nine `Y` rows and nine `[[entry]]` blocks already pass
  even if a row and an entry named different types. SHIP-01 says "matching exactly"; the step
  is extended to compare the set of `crate | type` pairs — each `[[entry]]`'s `migration_row`
  field (already present, e.g. `"paladin-ai-core | StopReason"`) against the §9.2 row's first
  two cells, with the type cell reduced to its first backtick-quoted identifier so the long
  parenthetical descriptions in rows like `StopReason` (defined in …) still match. Both
  directions, no wildcards, same awk/shellcheck-clean style as the existing step. Today's nine
  pairs are expected to match exactly; any mismatch found while writing the step is a finding
  for the audit (D-12), fixed in `MIGRATION.md` or the allowlist, never by loosening the check.
  Rejected: keeping crate-level plus a one-time manual audit — the claim would decay the day
  after release.
- **D-05: §9.2's 28 rows are audited row-by-row against `cargo semver-checks` output AND a
  public-API diff against v0.9.0, with the diff produced from the existing api-surface
  machinery.** The doc-08 step-6 "manual `cargo public-api`-style diff" is
  `git show v0.9.0:.project/current-exports.txt` diffed against the regenerated
  `.project/current-exports.txt` (`scripts/extract-public-api.sh`, 3936 items at 28-17; the
  `api-surface` CI job keeps it honest on nightly). Every pre-existing item whose signature
  changed must map to a §9.2 row; every §9.2 `N` row must correspond to a change semver-checks
  classifies as non-breaking; the eight "deliberate-zero" notes (types absent at v0.9.0) are
  confirmed by their absence from the v0.9.0 export. Rejected: adopting `cargo public-api` as a
  new tool — a new dependency in a release phase for a diff the tree already produces.

### Backward-compat proofs (SHIP-02)

- **D-06: The v0.9 sample config is a committed, frozen snapshot with provenance.**
  `git show v0.9.0:config.example.yml` (tag `v0.9.0` = `8b9bef89`) is committed verbatim as
  `tests/fixtures/config/v0.9.0-config.example.yml` with a leading comment block recording the
  tag, the blob SHA and the command that produced it. CI checks out with `fetch-depth: 1` and
  no tags, so reading the tag at test time is not an option. Rejected: a hand-trimmed
  "minimal v0.9 config" — the claim is about the *sample config a v0.9 operator actually has*.
  — **Reversibility:** reversible — a fixture file.
- **D-07: "Legacy behavior" is asserted at two levels, in one new root integration test target
  `tests/integration/v0_9_config_boot_test.rs` (registered as `[[test]] name =
  "v0_9_config_boot"`, no Docker, runs in the `test` job).** (1) *Config resolution:*
  `Settings::load_from_file` on the fixture succeeds, and every config struct added in v0.10
  (`agent_runtime`, `trace`, `web_server`, and the `Option<…>` fields for run queue/store/worker/
  stream, schedules, webhooks, assistants, waypoint store/retention, node cache, engine — the
  researcher enumerates the exact field list from `src/config/settings.rs:30-78` and the
  `src/config/*.rs` modules) equals its `Default` / is `None`, and every explicit enable flag
  (`graceful_shutdown` aside — it defaults `true` by M-B-02's recorded decision and the test
  asserts *that* value) resolves to "off". (2) *Behavioral:* the `paladin-server` app built from
  that `Settings` (the in-process pattern `tests/paladin_server_smoke.rs` already uses) mounts
  exactly the v0.9 route set — the six `/v1/agents…` paths plus `/health`, `/ready` and the
  docs routes — and no `/v1/runs`, `/v1/threads`, `/v1/assistants`, `/v1/schedules` or dev-ui
  route answers anything but 404. The test isolates itself from ambient `APP_*` environment
  variables (the loader's env-override layer is the one thing that could make CI diverge from a
  v0.9 operator's file) using the existing `serial_test` + scoped-env pattern from
  `tests/unit/settings_config_test.rs`. Rejected: config-resolution only — "boots with legacy
  behavior" is a behavioral claim, and Phase 27 designed the platform routes to be unmounted
  when their config is absent precisely so this test can pass by construction
  (27-DISCUSSION-LOG line 216).
- **D-08: The OpenAPI golden diff is path-restricted deep equality with `$ref` closure,
  `info.version` excluded.** `git show v0.9.0:crates/paladin-web/openapi.json` is committed as
  `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` (same provenance header convention as
  D-06, as a sibling `.md` note since JSON has no comments). A new
  `crates/paladin-web/tests/openapi_golden_v0_9.rs` loads `openapi_spec()` (the same function
  `openapi_matches_committed_baseline` in `crates/paladin-web/src/openapi.rs:264` uses), and for
  both documents: keeps only the six v0.9 `paths`, computes the transitive `$ref` closure of
  those operations into `components.schemas`, keeps `components.securitySchemes`, drops
  `info.version` (the bump regenerates it — `Makefile:614` documents exactly this), and asserts
  `serde_json::Value` equality with a readable diff on failure. Any inequality is a SHIP-02
  failure, not something to normalise away; the ONLY sanctioned normalisation is
  `info.version`. Rejected: whole-document diff with new paths stripped — a new shared schema
  referenced only by new paths would show as a spurious addition. — **Reversibility:**
  reversible — a test and a fixture.
- **D-09: Both compat tests are ordinary `cargo test --workspace` targets, so the `test` job
  gates them on every PR.** No new CI job. The 28-VALIDATION recalibration note (its literal
  `grep -c TBD` was unsatisfiable because of the §9.5 marker) is closed by D-01 + D-07 together:
  once the boot test exists, the §9.5 sentence is rewritten to cite it by target name.

### Program acceptance audit (SHIP-03)

- **D-10: The audit artefact is `.project/v0.10.0/09-program-acceptance-audit.md`, a new
  corpus document beside doc 08, with a pointer file in the phase directory.** The corpus is
  the program's source of truth and is not archived per milestone the way `.planning/phases/`
  is; doc 08 calls this "the post-implementation audit" and reserves the number. The phase dir
  gets `29-ACCEPTANCE-AUDIT.md` containing only the link and the verdict line, so the GSD
  verifier finds it. Structure mirrors doc-08's ten protocol steps, one `##` per step, each
  ending in a `Verdict:` line (`PASS` / `PASS with findings` / `FAIL`) and a findings list.
  — **Reversibility:** reversible — a document.
- **D-11: FR-to-test evidence is a per-FR table with named test anchors, seeded from artefacts
  that already exist, script-assisted and human-curated.** Every phase already carries the
  mapping: `2x-VERIFICATION.md` (must-have truths → tests), `2x-VALIDATION.md` (requirement →
  test id → command) and doc-08's own rows (e.g. the G-08 row lists seven anchors by
  `file#test_name`). The audit table has columns `FR | owning phase/plan | test anchor(s) |
  CI status`, filled by (a) a throwaway grep for `FR-\d+` mentions across `tests/`, `crates/*/
  src`, `crates/*/tests` and the phase VALIDATION files (run from the scratchpad, not committed
  as a script), then (b) curation. An FR with no anchor is a finding; the finding's disposition
  is either "anchor added" (a doc-comment citing the FR on an existing test — a comment, not a
  behavior change) or "test written" — the latter only when a genuine gap exists, recorded as a
  deviation. Rejected: per-phase summary only — doc-08 step 5 is per-FR.
- **D-12: Findings from every audit step are recorded, not fixed silently, and the fix set is
  bounded.** Allowed fixes in this phase: documentation, test additions, FR-citation comments,
  `MIGRATION.md`/allowlist corrections, CI gate tightening, changelog text. Anything requiring
  a production code change is an X-03 stop-and-flag item recorded in the audit with a proposed
  disposition (fix in v0.10.0 with a recorded decision, or defer to v0.10.1/v0.11 with a
  WINDOWS.md row) — the executor halts for that decision rather than making it.
- **D-13: Orphan-behavior scope is the set of integration/E2E test targets added since
  v0.9.0, plus the four `paladin-eval` scenarios.** Concretely: every `[[test]]` block present
  in HEAD's `Cargo.toml` and each crate's `Cargo.toml` but absent at `v0.9.0`, every file under
  `tests/integration/` and `crates/*/tests/` added since the tag, and `tests/evals.rs`'s
  registered scenarios. Each must trace to at least one FR (or to an X-rule / a BUG-0x fix,
  which count as owners). Unit tests inside `#[cfg(test)]` modules are out of scope — thousands
  of them, and doc-08 step 5's "orphan behavior" targets *behaviors*, which integration tests
  are the witnesses for. Rejected: every test in the tree.
- **D-14: Ubiquitous-language conformance is checked by table + grep, and deviations are
  FILED, never renamed this phase.** The audit carries a table of the twelve overview §4 terms
  (Battlefield, Dispatch, Superstep, Waypoint, Thread, Directive, Muster, Parley, Vanguard,
  Chronicle, Aegis, Vault) → the canonical Rust type/module that embodies each, plus a grep
  over public rustdoc and `docs/src/` for competing synonyms (`checkpoint` for Waypoint,
  `frontier` for Vanguard, `reducer` for Dispatch — the last two are sanctioned aliases per §4's
  own wording and are recorded as such). Known candidate: the `Frontier` type in
  `paladin-battalion::engine` alongside `compute_next_vanguard` — a naming split the audit
  records with disposition "accepted alias, rename deferred" (a public-type rename is an X-10
  break in a release-gate phase). Rejected: renaming now.
- **D-15: BUG-01 evidence is re-verified at Phase 29 HEAD and recorded with SHAs, not
  re-tested.** The audit cites RED `b2d05045` → GREEN `8d5ef333` (23-01-SUMMARY, doc-08's BUG-01
  row), re-runs `grep -rn "defaulting to true" crates/ src/` at HEAD (expected: zero matches),
  and names the four living tests (`unregistered_custom_condition_is_rejected_before_any_
  paladin_executes`, `unregistered_custom_edge_condition_fails_graph_validation`, `every_
  unregistered_custom_name_is_listed_sorted_and_deduped`, `registered_engine_evaluator_true_
  and_false_route_correctly`). BUG-02/03/04 are confirmed as pre-release engine fixes needing
  no §9.1/§9.2 entry (overview §7's own classification), per doc-08 step 4.
- **D-16: ⚠ The Phase 28 bench-overhead FAIL (PRD 07 acceptance 6: ≤3% superstep overhead with
  tracing on; measured +22.18% log sink, +18.46% composite, `28-BENCH-EVIDENCE.md`) is
  ACCEPTED for v0.10.0 as a documented deviation, with the bar re-scoped as a follow-up.**
  28-VERIFICATION's `human_verification` item 1 asks a maintainer to choose (a) accept, (b)
  re-scope to an I/O-bound superstep, or (c) optimise first. Recommended and auto-selected:
  (a) now + (b) as the recorded follow-up, because tracing sinks are opt-in (no sink → no
  overhead; `trace.state_values` defaults off, D-36) and the microbenchmark is all-Function-node
  (no LLM latency to amortise against), so no v0.9 workflow and no default v0.10 workflow pays
  it. Recorded as: a new `WINDOWS.md` row (kind `deviation`, Phase 28), a "Known limitations"
  line in `docs/src/operations/observability.md` (already partly present — the audit confirms
  the number is stated), a line in the root `CHANGELOG.md` `[0.10.0]` section, and a deferred
  idea for the optimisation. **The developer may overturn this at plan review** — choosing (c)
  turns it into an X-03 stop-and-flag production change and this phase would halt on it.
- **D-17: Judgment-tier prohibitions and other "human sign-off" items get a maintainer
  sign-off section in the audit, left unchecked by the executor.** 28-VERIFICATION item 2 (six
  judgment-tier safety/privacy prohibitions, LLM-verified non-authoritatively) and the M-B-04
  stop-and-flag confirmation (doc-08 step 7 says §9.1 holds M-B-01…03 "and nothing else — or
  any additional entry was raised as a stop-and-flag item with a recorded decision"; M-B-04 was
  added by ENG-08 in Phase 22 — the audit cites the Phase 22 CONTEXT/plan decision that added
  it and asks the maintainer to countersign) are listed as `- [ ]` checkboxes with the evidence
  beside each. The phase's UAT / `/gsd-verify-work` step is where the human ticks them.
  Rejected: the executor ticking them — the whole point of the tier is that an agent's verdict
  is non-authoritative.

### Version bump, changelogs & dry-run publish (SHIP-04)

- **D-18: The 0.10.0 bump lands on this feature branch, in this phase, without a tag —
  exactly the v0.9.0 precedent (PR #50 bumped twelve manifests, tag `v0.9.0` was cut on the
  merge commit).** Mechanism: `cargo release version 0.10.0 --execute --no-confirm --workspace`
  (the same command `make release` runs, `Makefile:611`), followed by `UPDATE_OPENAPI=1 cargo
  test -p paladin-web openapi_matches_committed_baseline` to regenerate the baseline whose
  `info.version` the bump invalidates (`Makefile:614-620`), and a grep that every
  `version = "0.9.0"` path-dependency pin in the manifests (`Cargo.toml:202,318` and the
  crates' inter-dependencies) moved to `0.10.0` — `cargo-release` handles these, the grep
  proves it. `Cargo.lock` is committed with the bump. The tag itself is cut from `main` after
  merge via `make release VERSION=0.10.0` / the tag-triggered `release.yml` (ADR-0043/0044:
  releases only from `main`; `verify-tag-source` enforces it). Rejected: leaving the bump to
  `make release` on `main` — SHIP-04 asks that "all workspace crates are at 0.10.0 … semver and
  MSRV CI jobs green on the release commit", which must be verifiable *in the PR's CI* before
  merge, not discovered on `main`. — **Reversibility:** costly — a version bump touches twelve
  manifests, the lockfile, the OpenAPI baseline and every changelog; reverting it is one revert
  commit but every dependent artefact must move together.
- **D-19: Changelogs: the script stamps, a human curates the root.** `make
  finalize-crate-changelogs VERSION=0.10.0` (`scripts/finalize-crate-changelogs.sh`) inserts a
  dated `## [0.10.0] - 2026-09-XX` section after each publishable package's `[Unreleased]`
  anchor (twelve files incl. the root; idempotent). The root `CHANGELOG.md`'s `[Unreleased]`
  body — already long and well-formed from Phases 22–28 — is *moved* under the dated heading
  and curated for a consumer: grouped Added/Changed/Fixed, one entry per subsystem with a
  `MIGRATION.md` §-link, the M-B-01…04 behavioral changes called out in a "Behavioral changes"
  sub-list first, the bench deviation (D-16) under "Known limitations". Per-crate changelogs
  keep whatever `[Unreleased]` content they have under the new heading (the script only inserts
  the heading; the executor moves the entries). `crates/paladin-eval/CHANGELOG.md`'s "Initial
  release" becomes its `[0.10.0]` section — its first published version (ADR-0048). The date is
  the bump commit's date; if the tag lands on a later day the date stands (v0.9.0's section is
  dated 2026-09-01, the day of both). `scripts/check-release-consistency.sh` (clauses 1–2:
  manifest and changelog-heading agreement) is run locally with `--tag v0.10.0` as the proof.
- **D-20: Dry-run publish evidence comes from `cargo publish --workspace --dry-run`, and `make
  publish-dry-run` is fixed to match; `scripts/publish-crates.sh` stays the real carrier.**
  Today's `publish-dry-run` target (`Makefile:552-565`) swallows every failure with `|| true`,
  addresses `paladin-core` by directory name (the package is `paladin-ai-core`, so that line can
  never succeed), omits `paladin-herald`, and its closing message points at a `docs/
  RELEASE_CHECKLIST.md` that does not exist. A per-crate `cargo publish --dry-run` loop also
  cannot be "green in dependency order" before anything is published, because each dependent's
  `paladin-ai-core = "0.10.0"` pin resolves against the registry, where 0.10.0 does not yet
  exist — which is exactly why the `|| true`s are there. `cargo publish --workspace --dry-run`
  (stable since cargo 1.90; the pinned toolchain is 1.97.1 — a release-time command, not
  MSRV-bound) packages the whole graph with intra-workspace resolution and verifies each crate
  in dependency order. The target is rewritten to that single command with no `|| true`, its
  message points at `docs/src/appendix/release-checklist.md`, and the release-checklist page's
  "Dry-Run Publish Validation" section lists the real twelve-crate order (adding `paladin-herald`
  and `paladin-eval`). **D-06 of Phase 20 is untouched:** the real publish stays the explicit
  per-crate loop in `scripts/publish-crates.sh` with registry-state detection and index polling;
  only the *dry-run evidence* command changes. The researcher confirms `--workspace --dry-run`
  behaves as described on 1.97.1 and whether `release.yml`'s `dry_run` dispatch input should
  call the same thing (out of scope to change unless it is a one-line swap).
  — **Reversibility:** reversible — a Makefile target and a docs section.
- **D-21: "The release commit" means both the PR head that carries the bump and the merge
  commit on `main` that gets tagged; evidence is recorded for both.** Pre-merge: the `ci`,
  `docs` and `feature-flags` workflow run IDs on the final feature-branch SHA (semver 11/11 vs
  0.9.0 — with the bump the tool reports `0.9.0 → 0.10.0`, minor, same allowlist; MSRV 1.88;
  coverage ≥ 82 %; api-surface; sdk-clients; e2e-platform-api; docs build + linkcheck). Post-
  merge: the same on the `main` merge commit, appended by the orchestrator after `/gsd-ship`,
  exactly as 27-CI-EVIDENCE / 28-CI-EVIDENCE did. The GSD phase closes on the pre-merge
  evidence; the milestone close (`/gsd-complete-milestone`) records the post-merge run and the
  tag.
- **D-22: Docs gates are the existing CI steps; no new doc tooling.** "No new broken intra-doc
  links" is `ci.yml:62-63` (`cargo doc --workspace --no-deps` with zero warnings tolerated) plus
  `docs.yml`'s `mdbook build` with `mdbook-linkcheck` at `warning-policy = "error"`,
  `check-doc-examples.sh` and `check-doc-config.sh`. The audit records their run IDs. The
  pre-existing `qdrant --all-features` rustdoc break (22-deferred-items item 1) is not "new"
  and stays deferred.

### Close-out hygiene: CI evidence, WINDOWS.md, doc sweep

- **D-23: `29-CI-EVIDENCE.md` mirrors the 27/28 form** — a local sweep table (every command
  this devcontainer can run: fmt, clippy `-D warnings` incl. `otel`/`dev-ui`/`web-server`
  feature sets, `cargo test --workspace`, `cargo test --test evals`, semver-checks 11/11,
  `cargo +1.88 check --workspace --all-features --all-targets`, `make security`,
  `check-release-consistency.sh --tag v0.10.0`, `cargo publish --workspace --dry-run`,
  `mdbook build docs/`), then the CI-run table per D-21.
- **D-24: `WINDOWS.md`'s 25 open rows are triaged with evidence in one dedicated plan; rows
  are never deleted, only moved to `fixed` or `waived` with a reason.** `/gsd-ship` blocks while
  `open_count > 0`, and a "releasable" verdict should not rest on an untriaged defect register.
  Buckets: (1) rows 2–19 predate the `v0.9.0` tag and shipped inside it as accepted debt →
  `waived`, reason cites `.planning/milestones/v0.9.0-MILESTONE-AUDIT.md` (status `tech_debt`,
  0 blockers); (2) Docker-gated `unrun-verify` rows 22, 26, 28 (Postgres/Redis Tier-2 suites
  never run locally) → `fixed` with the CI run ID of the `postgres-integration` /
  `redis-queue` / `redis-cache-integration` jobs that ran them green (27-CI-EVIDENCE records
  87/87 and 19/19); row 27 (coverage measured with a non-canonical invocation) → `fixed` with
  the canonical `coverage` job's figure from D-21's run; (3) documented design deviations 23,
  24, 25, 29–34 → `waived`, each reason pointing at the SUMMARY/REVIEW/ADR that recorded the
  decision — except any the audit (D-11/D-13) finds to be an unmet FR, which stays `open` as a
  finding; (4) one NEW row for the D-16 bench deviation. Every transition goes through
  `gsd-tools` (`query windows.*` handlers), never a direct edit. Rejected: leaving the register
  untouched — it would either block shipping or force a manual override at the worst moment.
- **D-25: The doc sweep is a bounded list, not a docs audit.** Exactly: the Upgrading page
  and `migration-guide.md` pointer (D-03); `docs/src/appendix/release-checklist.md`'s dry-run
  section and the Makefile message (D-20); the README MSRV badge / prerequisites line and
  `MIGRATION.md` §9.3 already agree on 1.88 (verified, `README.md:8,101`) — the audit records
  it; a one-line errata note in `.project/v0.10.0/00-program-overview.md` §4 reconciling its
  `Halt` mention in the Directive row with PRD 02 (23-DISCUSSION-LOG line 133 parked this for
  "Phase 29 doc sweep" — the researcher checks whether `NextStep::Halt` exists in the tree and
  the note states whichever is true); and the "Since: v0.10.0" markers on the Phase 22–28 mdBook
  pages are left as they are (they are correct). No new guides, no restructuring.

### Claude's Discretion

- Exact wording and layout of the Upgrading page and the curated root changelog.
- Whether the throwaway FR-grep (D-11) is worth keeping as `scripts/audit-fr-coverage.sh`
  for v0.11 — keep it only if it needs zero maintenance; otherwise leave it in the audit doc as
  a fenced command.
- File split of the audit document (one file per D-10 is the default; an appendix file for the
  per-FR table is acceptable if it exceeds ~600 lines).
- Order of plans; the natural waves are (1) SHIP-02 tests + D-04 gate tightening (mechanical,
  independent), (2) SHIP-03 audit + WINDOWS triage, (3) SHIP-01 MIGRATION closure + Upgrading
  page (depends on the boot test's target name), (4) SHIP-04 bump + changelogs + evidence
  (last, so every other commit lands under `0.9.0` and the bump is one clean commit).
- Whether the v0.9.0 openapi/config fixtures carry their provenance in a sibling `README.md`
  under `tests/fixtures/` instead of per-file headers.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition & requirements
- `.planning/ROADMAP.md` (line 717, "Phase 29: Program Gates & Release") — goal, the four
  success criteria, dependency on Phases 22–28.
- `.planning/REQUIREMENTS.md` (lines 286–308, SHIP-01…04; lines 8–16 for the X-10/X-11
  per-item versioning gate every requirement carries).
- `.project/v0.10.0/00-program-overview.md` — §3 X-03 (no behavioral change), X-08 (docs),
  X-10 (semver hygiene), X-11 (MSRV); §4 (the twelve ubiquitous-language terms, D-14); §5
  (program DoD items 1–7); §6 (E2E-1/2/3 text); §7 (BUG-01…04 classifications, D-15); §9
  (the required `MIGRATION.md` structure, the source for D-01/D-02).
- `.project/v0.10.0/08-traceability-matrix.md` lines 98–108 — **the ten-step verification
  protocol SHIP-03 executes**; the per-row test anchors (e.g. the G-08 row) are the seed for
  D-11's table.

### MIGRATION.md & the semver register
- `MIGRATION.md` (root, 638 lines) — §9.1 M-B-01…04 with worked examples; §9.2 (28 rows, 9
  marked `Y`); §9.3 (MSRV 1.88, every new dep/feature); §9.4 (seven tables, Citadel-unchanged
  statements); §9.5 line 318 (the SHIP-02 `TBD`); §9.6 (HTTP surface, pointer to golden diff);
  §9.7 (empty by design); §9.8 line 630 (the SHIP-01 `TBD`); header lines 7–8.
- `.cargo/semver-checks-allowlist.toml` — nine `[[entry]]` blocks with `migration_row` fields
  (the key D-04 matches on); header explains register-vs-suppression.
- `crates/paladin-core/Cargo.toml:64-67`, `crates/paladin-ports/Cargo.toml:45-47`,
  `crates/paladin-web/Cargo.toml:75-77`, `Cargo.toml:96-97` — the per-crate
  `[package.metadata.cargo-semver-checks.lints]` suppressions that actually silence the tool.
- `.github/workflows/ci.yml` — `msrv` job (251), `semver` job (303–357, eleven packages,
  `paladin-eval` deliberately excluded with comment at 330–338), allowlist set-equality step
  (359–400, extended by D-04, joined by D-01), `cargo doc` zero-warning step (62–63).
- `.planning/decisions/0048-paladin-eval-composition-crate.md` — why `paladin-eval` is outside
  the semver diff and inside the publish order; "checked against this ADR's reasoning, not
  re-derived".
- `.planning/phases/26-agent-runtime-enhancements/26-21-SUMMARY.md` lines 162–181 — the last
  full semver-checks sweep record (11/11, "assume minor"); the form D-23 repeats.
- `.project/current-exports.txt`, `scripts/extract-public-api.sh`, `scripts/check-api-surface.sh`,
  `scripts/normalize-api-bounds.py` — the api-surface baseline D-05 diffs against `v0.9.0`.

### Backward-compat proofs
- `config.example.yml` (HEAD) and `git show v0.9.0:config.example.yml` (tag `8b9bef89`) — the
  D-06 fixture source.
- `src/config/settings.rs` (`Settings::load_from_file` at 112; root fields 30–78 incl.
  `agent_runtime` 66, `trace` 72, `web_server` 78) and `src/config/{agent_runtime,trace,
  web_server,run_queue,run_store,run_worker,run_stream,schedules,webhooks,assistants,
  waypoint_store,waypoint_retention,node_cache,engine}.rs` — the structs D-07 asserts are
  inert.
- `tests/unit/settings_config_test.rs` (line 418+, `test_load_from_file_regression`) — the
  existing file-loading test and env-isolation pattern.
- `tests/paladin_server_smoke.rs` (`server_serves_openapi_spec_and_docs`, 285+) and
  `tests/web_server_e2e.rs` — the in-process server-boot pattern D-07's behavioral level reuses.
- `crates/paladin-web/src/openapi.rs` (`openapi_matches_committed_baseline`, 264–281) and
  `crates/paladin-web/openapi.json` (23 paths at HEAD; 6 at `v0.9.0`) — D-08's generator and
  the drift guard the bump must regenerate.
- `.planning/phases/27-platform-api/27-DISCUSSION-LOG.md` line 216 — "six new config structs
  all defaulting OFF … makes SHIP-02's boot test pass by construction".

### Acceptance audit inputs
- `tests/integration/e2e_crash_resume_test.rs`, `tests/integration/e2e_approval_gate_test.rs`,
  `tests/integration/e2e_muster_defer_order_test.rs`, `tests/helpers/e2e_fixtures.rs`,
  `tests/evals.rs` (+ `Cargo.toml` `[[test]]` blocks 379–432) — E2E-1/2/3 and their eval
  dogfood copies.
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-01-SUMMARY.md` — BUG-01
  RED `b2d05045` / GREEN `8d5ef333`, grep-absence statement (line 118).
- `.planning/phases/2{2,3,4,5,6,7,8}-*/2x-VERIFICATION.md` and `2x-VALIDATION.md` — the
  per-phase requirement→test mappings D-11 seeds from (read frontmatter + tables only).
- `.planning/phases/28-observability-tooling/28-VERIFICATION.md` (frontmatter
  `human_verification`, two items → D-16/D-17), `28-17-SUMMARY.md` lines 205–218 ("Open
  acceptance gaps a Phase 29 SHIP reviewer should see"), `28-BENCH-EVIDENCE.md` (the +22.18 % /
  +18.46 % measurement), `28-CI-EVIDENCE.md` and `27-CI-EVIDENCE.md` (the evidence-record
  form D-23 mirrors).
- `.planning/WINDOWS.md` — 25 open rows, `/gsd-ship` gate; D-24's triage input.
- `.planning/milestones/v0.9.0-MILESTONE-AUDIT.md` — the `tech_debt`/0-blocker record D-24
  cites for pre-v0.9.0 rows.

### Release mechanics
- `Makefile` — `release-check` (542), `publish-dry-run` (552–565, rewritten by D-20),
  `finalize-crate-changelogs` (567), `release` (576–620; the bump command at 611 and the
  OpenAPI regeneration note at 614–620), `openapi` (368).
- `scripts/finalize-crate-changelogs.sh`, `scripts/check-release-consistency.sh`,
  `scripts/publish-crates.sh` (D-06 carrier decision in its header — untouched),
  `scripts/extract-changelog-section.sh`, `scripts/check-changelogs.sh`.
- `.github/workflows/release.yml` — `verify-tag-source` (main-only), `create-release`
  (changelog section is the release body — a missing `## [0.10.0]` fails it), `dry_run` input.
- `.github/workflows/docs.yml` — mdbook 0.4.40 + mermaid + linkcheck 0.7.7, doc-example and
  doc-config checks.
- `docs/book.toml` (`[output.linkcheck] warning-policy = "error"` — why D-03 rejects include),
  `docs/src/SUMMARY.md` (line 70, Migration Guide entry — the Upgrading page goes above it),
  `docs/src/api-reference/migration-guide.md`, `docs/src/appendix/release-checklist.md`,
  `docs/src/appendix/release-automation.md`, `docs/src/appendix/release-recovery.md`.
- `.planning/decisions/0043-github-flow-trunk-and-trigger-surface.md`,
  `0044-branch-protection-posture.md` — releases from `main` only.
- `.planning/MILESTONES.md` and `.planning/PROJECT.md` lines 246–278 — the v0.9.0 release
  record (PR #50 bump, tag on merge `0b5d4106`, run `33542459191`) that D-18 replicates.
- `CHANGELOG.md` (root `[Unreleased]`, already populated) and `crates/*/CHANGELOG.md` (eleven;
  `paladin-eval`'s carries "Initial release").
- `README.md` lines 8 (MSRV badge 1.88) and 97 (the existing `MIGRATION.md` link).

### Prior-phase decisions that constrain this phase (do not re-open)
- 22.1: MSRV 1.88, single source `workspace.package.rust-version`, `resolver = "3"`.
- 22-04 / 26: allowlist register + per-crate suppressions; CI set-equality by crate
  (tightened, not replaced, by D-04).
- 23-01: BUG-01 fail-closed on both paths; RED before GREEN.
- 24: M-B-02 default `graceful_shutdown = true`, k8s `terminationGracePeriodSeconds: 60`.
- 26 (D-07…D-34): `StopReason`/`LlmRequest`/`GarrisonEntry` `#[non_exhaustive]`; M-B-03
  `tool_error_mode = FeedToModel` default.
- 27: every platform config struct defaults off; routes unmounted when absent; `paladin-eval`
  and the sdk-clients gate.
- 28 (D-34, D-39): E2E fixtures shared with the eval harness; `MIGRATION.md` rows filled per
  phase, "Phase 29 proves they are complete".
- ADR-0006: 82 % workspace line-coverage floor, single figure.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/paladin-web/src/openapi.rs::openapi_spec()` + `openapi_matches_committed_baseline` —
  the generator and the drift-guard pattern (env-var-driven regenerate) D-08's golden test
  sits beside.
- `Settings::load_from_file` (`src/config/settings.rs:112`) and the `config.test.yml`
  round-trip test — the loader D-07 exercises; every v0.10 config field is `#[serde(default)]`
  / `Option`, so a v0.9 file parses without edits.
- `tests/paladin_server_smoke.rs` — in-process `paladin-server` construction from a `Settings`;
  the route-set assertion in D-07 is a variation of its existing `/openapi.json` path check.
- `scripts/finalize-crate-changelogs.sh`, `scripts/check-release-consistency.sh` — both
  enumerate publishable packages from `cargo metadata`, never a hardcoded list; twelve
  packages including `paladin-eval` fall out automatically.
- `.github/workflows/ci.yml` allowlist step — the awk-over-§9.2 extraction D-04 extends
  (scoped between `## 9.2` and `## 9.3`, shellcheck-clean, tolerant of empty sets).
- `scripts/extract-public-api.sh` / `.project/current-exports.txt` — the public-API snapshot;
  `git show v0.9.0:.project/current-exports.txt` gives the baseline for D-05 with no new tool.
- Phase VERIFICATION/VALIDATION files and doc-08 rows — the FR→test anchors D-11 collates.
- `27-CI-EVIDENCE.md` / `28-CI-EVIDENCE.md` — the evidence-record shape.

### Established Patterns
- **Register + gate, both directions, no wildcards** (allowlist ↔ §9.2). D-01 and D-04 add
  gates in that style rather than one-off checks.
- **Evidence-based close-out**: every phase since 25 ends with a `CI-EVIDENCE.md` carrying run
  IDs; verifier notes list what is owed to Phase 29 explicitly (28-17-SUMMARY).
- **Frozen fixtures with provenance** (e.g. `docs/schemas/eval-scenario.schema.json` golden,
  `.project/current-exports.txt`) — D-06/D-08 follow it.
- **X-03 stop-and-flag**: any production behavior change discovered mid-audit halts for a
  decision (D-12), it is not a judgment call.
- **Release from `main` only; bump in a PR first** (v0.9.0) — D-18.
- **Per-phase `TBD` owner annotations** in `MIGRATION.md` — every remaining marker names
  SHIP-01/02; D-01 retires the convention by closing them.

### Integration Points
- `.github/workflows/ci.yml` `semver` job: two new/extended steps (D-01, D-04).
- `Cargo.toml` `[[test]]` registry: one new root target `v0_9_config_boot` (D-07); one new
  crate-level test file under `crates/paladin-web/tests/` (D-08).
- `tests/fixtures/config/` (new dir) and `crates/paladin-web/tests/fixtures/` (new dir).
- `docs/src/SUMMARY.md` + new `docs/src/api-reference/upgrading.md` (D-03).
- `Makefile` `publish-dry-run` (D-20).
- `MIGRATION.md` header, §9.5 line 318, §9.6 (add the golden-test pointer), §9.8 (D-01/D-02/D-09).
- `.project/v0.10.0/09-program-acceptance-audit.md` (new, D-10) and §4 errata (D-25).
- Twelve `Cargo.toml` `version` fields, `Cargo.lock`, `crates/paladin-web/openapi.json`,
  twelve `CHANGELOG.md` files (D-18/D-19).
- `.planning/WINDOWS.md` via `gsd-tools` handlers (D-24).

</code_context>

<specifics>
## Specific Ideas

- The v0.9.0 route set for D-07's behavioral assertion is exactly: `/v1/agents`,
  `/v1/agents/{id}`, `/v1/agents/{id}/execute`, `/v1/agents/{id}/execute/stream`,
  `/v1/agents/{id}/jobs`, `/v1/agents/{id}/jobs/{job_id}` (from `git show
  v0.9.0:crates/paladin-web/openapi.json`), plus the unversioned `/health`, `/ready`,
  `/openapi.json` and Swagger UI routes.
- The nine `Y` rows in §9.2 today: `paladin-ai-core` × `StopReason`, `BattalionError`,
  `PaladinError`, `GarrisonEntry`, `PaladinResult`; `paladin-ports` × `LlmError`, `LlmRequest`;
  `paladin-ai` × `Settings`; `paladin-web` × `require_authentication`. D-04's row-level check
  must pass on exactly these nine pairs at HEAD before any other change lands.
- `cargo semver-checks` after the bump reports `0.9.0 → 0.10.0` for each of the eleven
  baseline crates; because `0.x` minor bumps are treated as breaking-permitted by semver-checks
  only when lints are unsuppressed, the per-crate suppressions still carry the nine allowed
  changes and every other lint must stay clean — the audit records the post-bump run
  explicitly, not just the pre-bump one.
- The Makefile's `publish-dry-run` message references `docs/RELEASE_CHECKLIST.md`, which does
  not exist (`docs/src/appendix/release-checklist.md` does) — fix in the same edit (D-20).
- Root `CHANGELOG.md` `[Unreleased]` already contains the MSRV-1.88 entry, the M-B-01 entry
  and the fingerprint-bump entries; D-19 curates rather than rewrites.

</specifics>

<deferred>
## Deferred Ideas

- **`TraceDispatcher::emit` / `LogTraceSink` serialization optimisation** — the D-16
  follow-up if the re-scoped I/O-bound bar is also missed; not a v0.10.0 blocker.
- **`Frontier` → `Vanguard` rename** (or a documented alias) — D-14 files it; a public-type
  rename is an X-10 break.
- **Adopting `cargo public-api` as a first-class tool** — D-05 uses the existing api-surface
  snapshot; revisit if the snapshot's normaliser keeps needing patches.
- **Folding `migration-guide.md`'s v0.1–v0.5 history and `MIGRATION.md` into one versioned
  upgrade history** — D-03 adds a pointer only.
- **`release.yml` `dry_run` dispatch calling `cargo publish --workspace --dry-run`** — D-20
  leaves it unless it is a one-line swap.
- **`qdrant --all-features` rustdoc break** (22-deferred-items item 1) — pre-existing, not
  "new"; stays deferred.
- **Resolve-then-connect DNS-rebinding pinning for webhooks** (27 D-42), **LLM-call child
  spans**, **`FallbackHop.node_id` enrichment**, **live SSE attach in the inspector page**,
  **`trace.heartbeat_interval_secs` engine wiring**, **`DeltaMerged.field_changes[].dispatch`/
  `.writers` real values**, **`run_run_export` fired-edge derivation without a real graph**
  (WINDOWS #34) — Phase 28's own carry-forwards; recorded, not Phase 29 work.
- **FUT-01…05** (hand-polished SDKs, graphical IDE, multi-region HA, billing, multi-tenant
  RBAC) — named out of scope by the corpus.
- **A v0.11 `MIGRATION.md` scaffold** ("Upgrading from v0.10.0 to v0.11.0") — belongs to the
  next milestone's first phase, not this one.

### Reviewed Todos (not folded)
- **"Verify local `make coverage` reproduces CI's 82.39 % figure"**
  (`2026-08-13-verify-local-coverage-reproduction.md`, score 0.2) — related to WINDOWS.md row
  27 (coverage measured with a non-canonical invocation), which D-24 closes with the canonical
  CI figure; the local-reproduction question itself is a tooling task for any phase, not a
  SHIP gate.

</deferred>

---

*Phase: 29-program-gates-release*
*Context gathered: 2026-09-09*
