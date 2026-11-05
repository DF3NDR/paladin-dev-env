# Phase 58: Crate Release - Pattern Map

**Mapped:** 2026-11-05
**Files analyzed:** 27 (new or modified; release mechanics and documents, one Rust test, one shell checker)
**Analogs found:** 25 / 27 (2 partial, see "No Analog Found")

This is a release phase. Almost every file is a document or a script, so the analogs are the Phase 47
(`v0.11.0`) artifacts under `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/`, the existing
crate changelog anchors, and the existing gate scripts with their `tests/scripts/*_test.sh` harnesses.
Line numbers below were read on 2026-11-05. Do not invent Rust source analogs for documents.

Hard rules that apply to every file in this map:

- Nothing under `src/` or `crates/*/src/` changes (CONTEXT domain boundary). The one Rust file touched is a
  top-level test, `tests/consumer_surface_muster.rs`.
- Every `.planning/` file ends in exactly one newline and has no trailing whitespace. The required CI
  context `pre-commit run --all-files` is red today on `57-VALIDATION.md` for this reason (RESEARCH Pitfall 1).
- The `sk-proj-...` literal is never quoted. Name file and line only (D-08).
- A blanket `0.11.0` to `0.11.1` replacement is wrong. `0.11.0` stays in `scripts/check-semver.sh:86`,
  `Makefile:271-277`, `ci.yml` job name `Semver Checks (vs v0.11.0)`, the ruleset context name, frozen
  CHANGELOG/MIGRATION history and the devcontainer `vhs=0.11.0` pin (RESEARCH Anti-Patterns).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml`, `crates/*/Cargo.toml` (13 manifests), `Cargo.lock` | config | batch (tool-driven bump) | Phase 47 bump (`release.toml` + `cargo release version`) | exact |
| 52 gate-listed strings (docs pins, README, `k8s/**`, `crates/paladin-web/openapi.json`) | config/docs | transform (gate-driven sed) | `scripts/check-version-strings.sh` output at Phase 47 bump | exact |
| `CHANGELOG.md` (root) + `crates/*/CHANGELOG.md` x11 (stamp) | docs | batch | `scripts/finalize-crate-changelogs.sh` run at Phase 47 | exact |
| `crates/paladin-{battalion,eval,herald}/CHANGELOG.md` (new `[Unreleased]` anchor + entries) | docs | transform (git log to entries) | `crates/paladin-ports/CHANGELOG.md` (anchor shape) and `crates/paladin-herald/CHANGELOG.md` `[0.11.0]` entry style | exact |
| `MIGRATION.md` section 11.8 `### Library consumers: v0.11.0 → v0.11.1` | docs | request-response (static) | `MIGRATION.md` section 10.8 `### Library consumers: v0.10.1 → v0.11.0` (line 523) | exact |
| `docs/src/api-reference/migration-guide.md:4`, `upgrading.md` (stale prose, gate-invisible) | docs | transform | `docs/src/api-reference/upgrading.md` v0.10.1 to v0.11.0 entry | role-match |
| `scripts/check-review-dispositions.sh` (WR-02 fix) | utility (gate script) | transform | itself, lines 140-150 and 355-372 | exact |
| `tests/scripts/check-review-dispositions_test.sh` + `tests/scripts/fixtures/check-review-dispositions/<case>/` (near-miss fixtures) | test | batch | itself; sibling `tests/scripts/check-version-strings_test.sh` | exact |
| `scripts/sdk-smoke/run.sh` (SDK-smoke fix, conditional) | utility (CI script) | file-I/O | itself (`scripts/sdk-smoke/` siblings `lib-boot.sh`, `smoke.ts`) | exact |
| `scripts/check-doc-models.sh` + Makefile target + `tests/scripts/check-doc-models_test.sh` (optional `gpt-4` gate) | utility (gate script) + test | transform | `scripts/check-version-strings.sh` + `tests/scripts/check-version-strings_test.sh`; Makefile `check-version-strings` (line 225) | role-match |
| `gpt-4` literal sweep across `docs/src/**`, `README.md`, `examples/**` | docs | transform | todo `2026-11-03-docs-deprecated-gpt-4-model-literal.md` (its own scope list) | exact |
| `tests/consumer_surface_muster.rs` (three Cadence `use` pins) | test | compile-time | itself, `use paladin::...` block lines 47-61 | exact |
| `.devcontainer/Dockerfile.dev` (cargo-llvm-cov 0.8.7) | config | batch | `.github/workflows/ci.yml:1489` `tool: cargo-llvm-cov@0.8.7`; `.devcontainer/Dockerfile` `vhs` pin style | role-match |
| `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md` (scrub lines 49 and 53) | docs | transform | `docs/src/deployment/production.md` and `k8s/secret.yaml.example` placeholder forms (`sk-proj-staging-...`) | role-match |
| `.planning/phases/57-register-hygiene-close-out/57-VALIDATION.md` (EOF fix) | docs | transform | none needed; `end-of-file-fixer` semantics | exact |
| `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md` | docs (register) | CRUD (rows) | `57-REVIEW-DISPOSITIONS.md` (row form) and `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` `tech_debt` register | role-match |
| `.planning/todos/pending/*.md` (re-home todos: CR-01, WR-01, D-52-03-01, 54 WR-04, FIXES-04, OSV 23-25, baseline rename; `recheck_by` added to the 7 existing) | docs | CRUD | `.planning/todos/completed/2026-10-26-record-v0-11-0-coverage-lines-figure.md` (has `recheck_by`); `2026-11-04-muster-pin-cadence-consumer-imports.md` (frontmatter) | exact |
| `.planning/phases/58-crate-release/58-ACCEPTANCE-AUDIT.md` | docs | request-response | `47-ACCEPTANCE-AUDIT.md` | exact |
| `.planning/phases/58-crate-release/58-CI-EVIDENCE.md` | docs | event-driven (append per push/run) | `47-CI-EVIDENCE.md` | exact |
| `.planning/phases/58-crate-release/58-HANDOFF-web3sec.md` | docs | request-response | `47-HANDOFF-web3sec.md` (shortened, no return contract) | role-match |
| `.planning/phases/58-crate-release/58-READY-TO-CLOSE.md` | docs | request-response | `47-READY-TO-CLOSE.md` | exact |
| `.planning/v0.11.1-MILESTONE-AUDIT.md` (scoped to Phases 52-57) | docs | transform | `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` (frontmatter, `superseded_requirements`) | exact |
| Registry/TP probe script in the phase directory (read-only, not a gate) | utility | request-response | probe in `47-CI-EVIDENCE.md` lines 689-700 (sparse index) and RESEARCH Code Examples | exact |
| Release PR retitle + body (`gh api` REST) | config (GitHub state) | request-response | PR #59 retitle in `47-CI-EVIDENCE.md` "Plan 47-05" (line 254) | exact |
| Merge + tag hand-off text | docs | request-response | `47-CI-EVIDENCE.md` "Plan 47-08" (line 559) and `47-READY-TO-CLOSE.md` | exact |
| `MILESTONES.md` `v0.11.1` entry (`chore/58-close`) | docs | CRUD | the `v0.11.0` entry in `MILESTONES.md` | exact |
| `REQUIREMENTS.md` SHIP-08/09/10 ticks, `STATE.md`, `ROADMAP.md` | docs | CRUD via `gsd-tools` | Phase 47 post-tag wave (handlers only, never direct edits) | exact |
| `WINDOWS.md` | docs | read-only | must still read `open_count: 0`; gains no row (D-09) | n/a |

## Pattern Assignments

### Bump commit 1: manifests, lockfile, 52 strings, `openapi.json` (config, batch)

**Analog:** Phase 47 bump with `release.toml`; RESEARCH "Pattern 1" reproduction.

Order inside the commit (RESEARCH Discretion, confirmed):

```bash
# 1. clean tree first
cargo release version 0.11.1 --execute --no-confirm --workspace   # modifies 13 manifests + Cargo.lock, no commit/tag/push
# 2. let the gate enumerate the work (RESEARCH Pattern 1, measured: 52 findings -> OK)
./scripts/check-version-strings.sh 2>&1 \
  | grep -E "found '0\.11\.0'" \
  | sed -E 's/^([^:]+):([0-9]+): .*/\1 \2/' \
  | while read -r f l; do sed -i "${l}s/0\.11\.0/0.11.1/g" "$f"; done
# 3. regenerate openapi (diff must be the version string only, D-14)
make openapi
# 4. gate green on this commit
make check-gates
```

Expected gate summary after the edit: `OK: 128 file(s) scanned ... '0.11.1' (class 1: 42; 2: 1; 3: 8; 4: 1; exempt: 1)`.
`crates/doc-examples` (`publish = false`) is hand-touched if the tool skips it. The grep-zero check is scoped to
manifests, pins and the gate's four file classes only.

**Do not** touch `.project/current-exports.txt` (zero `0.11.0` matches; API-surface baseline must not move).

---

### Bump commit 2: changelog stamping (docs, batch)

**Analog:** `scripts/finalize-crate-changelogs.sh` as run at Phase 47; Makefile target at line 668-674.

```makefile
finalize-crate-changelogs: ## Stamp a dated section into every publishable package's changelog (VERSION=x.y.z required)
	@./scripts/finalize-crate-changelogs.sh --version "$(VERSION)"
```

```bash
make finalize-crate-changelogs VERSION=0.11.1   # needs a `## [Unreleased]` anchor in all 12 files first
make check-changelogs
make check-release-consistency RELEASE_TAG=v0.11.1
```

Pattern notes: the script inserts `## [0.11.1] - <date>` immediately after `## [Unreleased]`, leaving an empty
anchor above. The Makefile target passes no `--date`; to stamp another day call
`./scripts/finalize-crate-changelogs.sh --version 0.11.1 --date YYYY-MM-DD` or redate by a `docs(release)`
commit (D-16). A file with no anchor and no target section is left untouched with a named failure (D-00q).

---

### New `[Unreleased]` anchors for `paladin-battalion`, `paladin-eval`, `paladin-herald` (docs, transform)

**Analog anchor shape:** `crates/paladin-ports/CHANGELOG.md` lines 1-13. Eight crates already carry
`## [Unreleased]` at line 8 (`CHANGELOG.md`, content, core, llm, memory, notifications, ports, storage, web).

```markdown
# Changelog

All notable changes to `paladin-ports` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- <entry>
```

**Today's state of the three:** `crates/paladin-herald/CHANGELOG.md` starts at `## [0.11.0] - 2026-10-26` on line 8
(no anchor). Insert `## [Unreleased]` immediately above that line, before the `[0.11.0]` section.

**Entry content comes from `git log v0.11.0..HEAD -- crates/<crate>/`** (the `v0.11.0` tag is present locally).
RESEARCH measured counts: `paladin-herald` 0 commits (the "no functional change" line is legitimate),
`paladin-battalion` 2 (`79cd06bf7` test-only cross-reference; `2d3cf119a` removal of crate-wide semver allows),
`paladin-eval` 1 (`f70f03deb` docs: cancelled-run note). Entry style to copy is the `paladin-herald` `[0.11.0]` block:
`- Phase NN (topic, REQ-ID): sentence naming the observable change.` under `### Added` / `### Changed`.
The other eight changelogs are sanity-checked against their own logs so none contradicts the root (47-REVIEW WR-01).

---

### `MIGRATION.md` section 11.8 `### Library consumers: v0.11.0 → v0.11.1` (docs)

**Analog:** `MIGRATION.md:523` `### Library consumers: v0.10.1 → v0.11.0`. Insert after the operator checklist that
ends the section 11.8 block (the checklist runs from line 133, items 1-12 plus the trailing `RunWorkerPool` paragraph).

Opening shape to mirror (lines 523-529):

```markdown
### Library consumers: v0.10.1 → v0.11.0

This subsection is for a library consumer that depends on the facade `paladin-ai` (plus
`paladin-ports` / `paladin-memory`) by path or submodule and moves from the `v0.10.1` tag to
`v0.11.0`. The operator checklist above is separate and does not apply to a consumer that only
links the libraries. ...
```

Required content (D-10): one paragraph on measured evidence; no Rust signature, type or serialized schema
changed; section 11.2 has no row and `.cargo/semver-checks-allowlist.toml` has no `[[entry]]`; 
`scripts/check-semver.sh` on the release head reads `0 failure(s)` for 12 packages at `--release-type patch`
(name head SHA and date); the only consumer-visible note is `get_job` unrouted-but-not-deprecated (section 11.7).
Open Question 4 in RESEARCH: section 11.1 holds 25 behavioural rows (M-B-23 to M-B-47) that a library consumer can
observe (for example M-B-30 `RepositoryError::QueryError` instead of a panic), so word it as a compile-time
statement plus a pointer list, not a bare "without adaptation". Sections 9 and 10 are frozen (D-00n).

---

### `scripts/check-review-dispositions.sh` WR-02 fix and fixtures (utility, transform) and its test (test, batch)

**Analog:** the script itself plus its harness `tests/scripts/check-review-dispositions_test.sh`.

Pointer regexes to tighten (script lines 143-147):

```python
COMMIT_RE = re.compile(r"\b(?=[0-9a-f]*[0-9])[0-9a-f]{7,40}\b")
TEST_RE = re.compile(r"\b[a-z][a-z0-9]*(?:_[a-z0-9]+){2,}\b")
REQ_RE = re.compile(r"\b([A-Z]{2,})-[0-9]+\b")
FINDING_PREFIXES = ("WR", "CR", "IN")  # a pointer back at a finding is not evidence
PATH_RE = re.compile(r"[A-Za-z0-9_.\-]+(?:/[A-Za-z0-9_.\-]+)+")
```

Failure site that must fire on non-evidence (script lines 365-368):

```python
    if disp == "closed" and not has_pointer(evidence):
        fail("CLOSED_NO_POINTER", phase, ident,
             "no commit, test name, requirement id, repo path or WINDOWS row in Evidence")
```

**Test harness pattern** (test lines 1-50): each case is a committed fixture directory
`tests/scripts/fixtures/check-review-dispositions/<case>/` holding `archive/<NN>-<slug>/<NN>-REVIEW.md`,
`register.md` and an optional `args`; every fixture is a copy of `ok` with exactly one thing changed; failing
cases are asserted first ("prove it fails"), then passing cases; each run's last line must match

```bash
SUMMARY_RE='^checked [0-9]+ finding\(s\) across [0-9]+ review file\(s\); [0-9]+ failure\(s\)$'
```

New near-miss cases (RESEARCH Wave 0): dates, `ADR-0063`, `SHA-256`, `UTF-8`, `and/or/else`, `not_a_thing`; each must
fire `CLOSED_NO_POINTER`. RED first, then implement, then run the real archive (expects
`checked 51 finding(s) across 13 review file(s); 0 failure(s)`) and `make test-shell-guards` (22 fixtures plus new).
Real-archive path in the harness is `.planning/milestones/v0.11.0-phases`.

---

### `tests/consumer_surface_muster.rs` Cadence pins (test, compile-time)

**Analog:** the file itself. Import block to extend (lines 47-61):

```rust
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::aegis::{Aegis, ErrorHandlerSpec, TimeoutPolicy};
...
```

Add the three paths named by the folded todo (`2026-11-04-muster-pin-cadence-consumer-imports.md`):
`paladin::infrastructure::cadence::build_cadence`, `paladin::infrastructure::cadence::with_cadence`,
`paladin::config::treasurer::CadenceConfig` (use `CadenceConfig::default()`). Follow the file's existing
pattern for exercising each import (read the body of the file before writing; it is outside the excerpt above).
Gate: `make check-consumer-surface`; needs `llm-anthropic` and `llm-deepseek` features (RESEARCH Pitfall 10).
No public-surface change, so `.project/current-exports.txt` does not move. Conventional type: `test`.

---

### Optional `scripts/check-doc-models.sh` plus Makefile target and harness (utility + test)

**Analog:** `scripts/check-version-strings.sh` (offline, python3 >= 3.11 file scanner with `file:line` output),
Makefile wiring at line 225-226:

```makefile
check-version-strings: ## Verify every copyable version string (README, docs/src, k8s/, openapi.json) equals the workspace version (FIRST-03)
	@./scripts/check-version-strings.sh
```

Harness analog: `tests/scripts/check-version-strings_test.sh` (looped automatically by `make test-shell-guards`, so
a new `tests/scripts/check-doc-models_test.sh` is picked up with no list edit). If the gate lands, add it to
`check-gates` (Makefile line 281) only if the planner decides so; include an allowlist for deliberate
deprecated-id pages. Replacement model name follows the todo (`gpt-4o-mini`, assumption A5). Whether the gate lands
is the planner's call (CONTEXT Folded Todos).

---

### `scripts/sdk-smoke/run.sh` (utility, file-I/O; conditional on diagnosis)

**Analog:** itself. Directory holds `run.sh`, `lib-boot.sh`, `smoke.ts`, `smoke.py`, `smoke-http.sh`, `package.json`.
RESEARCH Pitfall 3: the TypeScript half fails at `npm install --no-save ../../target/sdk/typescript` with
`sh: 1: tsc: not found` (exit 127) on `ubuntu-26.04`. Candidate minimal fix stays inside `scripts/`: build the
generated client explicitly (`npm install --ignore-scripts && npm run build` in the generated package) and install
it into the smoke package with `--ignore-scripts`. Not reproducible without Docker; only a CI run proves it.
Read `run.sh` before editing (not read in this mapping).

---

### `58-ACCEPTANCE-AUDIT.md` (docs, request-response)

**Analog:** `47-ACCEPTANCE-AUDIT.md` (69 lines). Shape: title `# vX re-seal and Tag Sign-off`, a preamble of 2-3
sentences, a "Head SHA this section's gates were measured on" paragraph (SHA-R plus the code-head and the
`git diff --name-only <SHA-H> <SHA-R> -- . ':!.planning' ':!.project'` empty-diff proof), then the eight-row table:

```markdown
| # | Gate | Command | SHA | Result | Evidence |
|---|------|---------|-----|--------|----------|
| 1 | `MIGRATION.md` no-TBD marker + ... allowlist set-equality | `grep -c TBD MIGRATION.md` then `make check-migration-allowlist` | `d82d67ca` | `COUNT=0`; ... | `47-CI-EVIDENCE.md` Local sweep row N |
| 2 | `v0_9_config_boot` frozen compat target | `cargo test --features web-server --test v0_9_config_boot` | ... |
| 3 | OpenAPI golden diff | `cargo test -p paladin-web --test openapi_golden_v0_9` | ... |
| 4 | `cargo semver-checks` | ... | ... |
| 5 | MSRV floor, toolchain 1.88 | CI-attributed | ... |
| 6 | `make publish-dry-run`, twelve crates | ... | ... |
| 7 | `CHANGELOG.md` completeness and release-body size | `make check-gates` + hard-assertion greps | ... |
| 8 | Publish-order gate | `make check-publish-order` | ... |
```

Adaptations for 58 (D-04, RESEARCH re-seal table): row 4 is now `make check-semver` (a real check vs `0.11.0` at
patch, 12 crates, `SEMVER_PRUNE_LOCAL_CACHE=1`); row 3 uses `cargo test -p paladin-web --lib openapi_matches_committed_baseline`
(also `v0_9`-golden as Phase 47 did); row 7 reads `[0.11.1]`. The single tick box (line 67 in the analog) must
be written unticked, exact text per D-04:

```markdown
- [ ] **The `v0.11.1` tag may be cut** — evidence: this section in full, plus `.planning/phases/58-crate-release/58-CI-EVIDENCE.md` ...
```

Only the maintainer ticks it (D-00a). Note the analog's "not run locally -- CI-attributed" wording for MSRV
(1.88 toolchain absent) and the `Coverage` job (cargo-llvm-cov absent).

---

### `58-CI-EVIDENCE.md` (docs, event-driven append)

**Analog:** `47-CI-EVIDENCE.md` (1280 lines). Section skeleton to copy:

```
# Phase NN vX Crate Release — CI Evidence Record
## Provenance
## Local sweep                      (table, one row per command with SHA and result)
## CI-run table                     (per-push subsections: "Plan NN — push of SHA-An", "every run on SHA-An")
### CI-evidence-only entries (D-12)
## Registry verification (D-00h)
### Pre-tag baseline
### Pre-tag guard
### Post-publish verification
## Findings carried forward (D-00d)  (dated "### YYYY-MM-DD — <id>: <title>" entries)
```

Registry probe command and assertions (analog lines 689-700, 786-800; RESEARCH Code Examples), with `V=0.11.1`:

```bash
UA='paladin-release-verify (github.com/DF3NDR/paladin-dev-env)'
for c in $(cargo metadata --format-version 1 --no-deps | jq -r '.packages[]|select(.publish==null)|.name' | LC_ALL=C sort); do
  curl -sS -A "$UA" "https://index.crates.io/$(sparse_path "$c")" \
   | jq -r --arg v 0.11.1 'select(.vers==$v)|[.name,.vers,.cksum,.yanked]|@tsv'
  curl -sS -A "$UA" "https://crates.io/api/v1/crates/$c/0.11.1" | jq -c '.version|{num,yanked,trustpub_data}'
done
```

Assertions: rows = 12 = N; N > 0; every row `vers` `0.11.1`, `yanked` `false`, 64-hex `cksum`; exactly one `0.11.1`
line per crate; `trustpub_data.sha` equals the merge commit; `trustpub_data.run_id` equals the `release.yml` run.
Include the pre-tag baseline table (all twelve at `0.11.0`, no `0.11.1` row, read 2026-11-05), the keyless
scratch-crate smoke (D-13), the release-body size and footer read (first live `release-body-fit.sh` run, D-00r),
and the SDK-smoke diagnosis, `treasury_spend` flake, and pre-commit EOF findings as dated findings entries.
Semver evidence from CI without log access: `GET /repos/DF3NDR/paladin-dev-env/check-runs/<job_id>/annotations`
expecting twelve `223 checks` lines. File names the shallow-clone caveat (`Shallow clone: commit counts come from
GitHub`, analog line 934) if `git log v0.11.0..HEAD` ever fails.

---

### `58-HANDOFF-web3sec.md` (docs, request-response)

**Analog:** `47-HANDOFF-web3sec.md` (150 lines), shortened. Keep only the outbound parts: tag, merge SHA,
release run id, twelve-crate registry table, pointers to MIGRATION section 11.8's consumer subsection and the
operator steps `M-B-36` (key rotation) and `M-B-38` (environment variables), and the Cadence muster pin. Drop the
return-contract section entirely (D-12). Read the analog's first 40 lines for tone and section order.

---

### `58-READY-TO-CLOSE.md` (docs, request-response)

**Analog:** `47-READY-TO-CLOSE.md` (151 lines). Lists the `/gsd-audit-milestone` and `/gsd-complete-milestone v0.11.1`
prerequisites for the maintainer. The agent does not run complete-milestone (D-00l).

---

### `58-AUDIT-DISPOSITIONS.md` and re-home todos (docs, CRUD)

**Register row analog:** `.planning/phases/57-register-hygiene-close-out/57-REVIEW-DISPOSITIONS.md` (row form
checked by `scripts/check-review-dispositions.sh`: Disposition is one of `closed | waived | open`, Evidence needs a
checkable pointer, `waived` needs an owner). 58 uses its own three verdicts (D-06): `fixed` (pointer to commit
or evidence), `waived` (reason), `re-homed` (owner, date, todo path). Suggested columns:
`id | source | verdict | pointer / reason / owner+date | todo path`. Do not feed this file to
`check-review-dispositions.sh` (different verdict vocabulary); validate with the RESEARCH Wave 0 throwaway
verifier (every row has exactly one verdict; re-homed rows cite an existing todo with `owner` and `recheck_by`).

**Milestone audit analog:** `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` frontmatter:

```yaml
milestone: v0.11.0
status: tech_debt
previous_audit: "... — gaps_found (...); superseded by this re-audit"
scores:
  requirements: 63/66
  phases: 15/15
gaps:
  requirements: []
superseded_requirements:
  - id: "ADOPT-01"
    status: "superseded"
```

For `.planning/v0.11.1-MILESTONE-AUDIT.md`: scope explicitly to Phases 52-57, list SHIP-08/09/10 as owned by
Phase 58 and proven by the post-tag registry read, use a `re_homed_requirements:` key for FIXES-04 mirroring
`superseded_requirements:`, and set `tech_debt: []` citing only register rows (RESEARCH Pitfall 6; the
audit-scope choice is Open Question 5 for the maintainer).

**Todo frontmatter analog:** `.planning/todos/pending/2026-11-04-muster-pin-cadence-consumer-imports.md`:

```yaml
---
created: 2026-11-04T00:00:00Z
title: <title>
area: downstream-consumer
severity: minor
files:
  - <path>
owner: repo maintainer
dispositioned_by: Phase 56 plan 56-10 (DF-36)
---
```

Add `recheck_by: YYYY-MM-DD` as in `.planning/todos/completed/2026-10-26-record-v0-11-0-coverage-lines-figure.md`.
None of the seven pending todos has it today. The date is not known (Open Question 3): ask the maintainer, do not
invent one. Code items take `owner: v0.12.0 Phase 1`. Sections below the frontmatter follow `## Problem` /
`## What is already true`. The pending files today: openai-responses-api, setup-check-first-run-output,
unquoted-treasurer-price, devcontainer-llvm-cov, docs-gpt-4, muster-pin-cadence, reasoning-budget-hint. The three
folded ones (llvm-cov, gpt-4, muster pin) move to `completed/` through the proper handler once their commits land.

---

### `.project/.../LIVE_API_TESTS_FIX.md` scrub (docs)

**Analog (placeholder forms already accepted by the tree):** `sk-proj-staging-...` in `docs/src/deployment/production.md`
and `k8s/secret.yaml.example`. Scrub both key-shaped lines (49 partial with tail fragment, 53 full literal; RESEARCH
Pitfall 8), not just line 53. Proof, never echoing a match:

```bash
git grep -nE 'sk-proj-[A-Za-z0-9_-]{16,}'   # expect no match except documented placeholders; record counts and file:line only
```

`gitleaks` did not flag the literal, so the regex scan is the proof, plus the 56-08 key-shape scanner. Commit
message names file and lines only. Commit type `docs`.

---

### Release PR retitle, label, merge and tag hand-offs (config, request-response)

**Analog:** `47-CI-EVIDENCE.md` "Plan 47-05 — push of SHA-A0 and the PR #59 retitle" (line 254) and "Plan 47-08"
(line 559). PR writes go through `gh api` REST; GraphQL is blocked; use the "hand the exact command over" fallback
for a read-only token (`gh auth status` reported `GH_TOKEN` invalid, writes untested). Tag commands (D-00c):

```bash
git fetch origin && git tag -a v0.11.1 <merge-sha> -m "v0.11.1" && git push origin v0.11.1
```

Preconditions the hand-off must state: `ci.yml` on the merge SHA concluded `success` as a whole run, which includes
the non-required SDK-smoke job (`check-release-consistency` clause 3, RESEARCH Pitfall 2). Reword D-05's "accept a
red non-required job" option to "fix or hold the tag". The `release` label step is one checkpoint, applied after the
bump push (D-03).

---

### `.devcontainer/Dockerfile.dev` cargo-llvm-cov (config)

**Analog:** `.github/workflows/ci.yml:1489` `tool: cargo-llvm-cov@0.8.7` (keep the pin equal to CI; crates.io
latest is 0.9.1, do not upgrade). Shape: `RUN cargo install --locked --version 0.8.7 cargo-llvm-cov`. Read both
`.devcontainer/Dockerfile.dev` and `.devcontainer/Dockerfile` before choosing where the line goes (the latter
carries the `vhs=0.11.0` pin that must NOT be bumped). Commit type `chore`; no coverage run is claimed.

## Shared Patterns

### Gate scripts with named failures and shell harnesses
**Source:** `scripts/check-*.sh` with `tests/scripts/*_test.sh`, wired by Makefile `check-gates` (line 281) and
`make test-shell-guards` (loops `tests/scripts/*_test.sh`).
**Apply to:** the WR-02 fix, the optional `check-doc-models` gate, the register-completeness verifier.
Prove-it-fails fixtures first, then pass cases, summary line asserted by regex.

### Conventional commit per build-wave item, bump set last
**Source:** CONTEXT D-14, D-17. Types: `test` (muster pin), `docs` (gpt-4 sweep, scrub, anchors, register, todos),
`chore` (devcontainer), `fix` (WR-02), `ci` (SDK smoke), then `chore(release)` x2 and `docs(release)` x1 as the
last three commits. Commit trailers per the harness attribution reminder.

### Red-gate policy
**Source:** D-00j. From the first re-seal command onward, a red gate is recorded (command, SHA, failing output) and
the agent halts at a blocking checkpoint, fixing nothing. Only a pure infrastructure flake is re-run once, with both
run IDs recorded. RESEARCH Pitfall 5: `treasury_spend_prints_one_row_per_currency` is a test-level flake, not
infrastructure; decide its rule before the re-seal.

### Maintainer-only acts and resume files
**Source:** D-00a, D-00b, D-00i. Agent writes exact commands and stops; resume file names the exact resume condition;
no parked watchers.

### Sparse-index registry proof
**Source:** `47-CI-EVIDENCE.md` lines 689-800 and RESEARCH Code Examples. Mandatory `User-Agent`; publishable set
derived from `cargo metadata`, never hard-coded; twelve names, `publish-crates.sh` `CRATES` order is different from
the sorted list.

### Status mutations through handlers
**Source:** CONTEXT "Established Patterns". ROADMAP, STATE and REQUIREMENTS change only through `gsd-tools`
handlers; SHIP ticks happen post-tag after the 12/12 registry table. Only the sign-off box is maintainer-ticked.

### Disk and tool hygiene
**Source:** RESEARCH Pitfall 10 and Environment Availability. About 5.9 GB free with a 17 GB `target/`; set
`SEMVER_PRUNE_LOCAL_CACHE=1`, `CARGO_INCREMENTAL=0`, delete `target/debug/incremental` before the re-seal. No Docker
and no `cargo-llvm-cov` locally, so those rows are CI-attributed.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `58-AUDIT-DISPOSITIONS.md` register verdict vocabulary (`fixed / waived / re-homed`) | docs | CRUD | No prior register uses these three verdicts; `57-REVIEW-DISPOSITIONS.md` uses `closed / waived / open`, and the v0.11.0 audit carried a `tech_debt` list instead. Borrow row form only. |
| Registry/TP probe script as a committed file in the phase directory | utility | request-response | Phase 47 kept the probe as a command block inside `47-CI-EVIDENCE.md`, not a script. Use that block and the RESEARCH Code Example. |

## Metadata

**Analog search scope:** `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/`, `.planning/todos/`,
`.planning/milestones/v0.11.0-MILESTONE-AUDIT.md`, `scripts/`, `tests/scripts/`, `Makefile`,
`crates/*/CHANGELOG.md`, `MIGRATION.md`, `tests/consumer_surface_muster.rs`, `scripts/sdk-smoke/`.
**Files scanned:** about 30 read or grepped (47-ACCEPTANCE-AUDIT, 47-CI-EVIDENCE headings and probe sections,
47-HANDOFF and 47-READY-TO-CLOSE heads, MIGRATION 11.8 and 10.8, two crate changelogs, the review-dispositions
script and harness, Makefile gate targets, one pending todo).
**Not read in full (planner should read before editing):** `scripts/sdk-smoke/run.sh`,
`tests/consumer_surface_muster.rs` body, `.devcontainer/Dockerfile*`, `57-REVIEW-DISPOSITIONS.md`,
`47-HANDOFF-web3sec.md` and `47-READY-TO-CLOSE.md` bodies, `MILESTONES.md`.
**Pattern extraction date:** 2026-11-05
