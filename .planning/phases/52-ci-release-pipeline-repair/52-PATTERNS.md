# Phase 52: CI & Release Pipeline Repair - Pattern Map

**Mapped:** 2026-10-28
**Files analyzed:** 22 (new/modified)
**Analogs found:** 21 / 22 (all in-repo; no Rust is added)

All paths are relative to `/home/user/paladin-dev-env`. Line numbers are as read on 2026-10-28.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.github/workflows/ci.yml` (semver step, `on.pull_request.types`, e2e/dry-run `if:`, gate-script step) | config (workflow) | event-driven | itself: `cargo-deny` job steps (114-170), `benchmark-regression-signal` `if:` (2075) | exact |
| `scripts/check-migration-allowlist.sh` (window to `11.2`/`11.3`, header rewrite) | utility (offline gate) | batch / transform | itself, line 94 | exact |
| `scripts/create-or-reuse-release.sh` (`jq --rawfile`, size guard call) | utility (release script) | request-response | itself, `_cor_build_payload` (lines 99-120) | exact |
| `scripts/finalize-release-body.sh` (guard after `compose_release_body`) | utility (release script) | transform / file-I/O | itself, lines 296-330 and 470-486 | exact |
| `scripts/<shared size helper>` (new, sourced by both release scripts) | utility (library) | transform | `scripts/check-publish-order.sh` sourcing `publish-crates.sh` (line 131) | role-match |
| `tests/scripts/create-or-reuse-release_test.sh` (new D-10/D-11 assertions) | test | request-response (stubbed `gh`) | itself, Case 8 (330-355) and Case 9 (357-372) | exact |
| `tests/scripts/finalize-release-body_test.sh` (cap/idempotency cases) | test | transform | itself, Case 12 (489-500) and Case 18 (695) | exact |
| `tests/scripts/check-migration-allowlist_test.sh` (window to §11.2) | test | batch | itself, `write_migration_two_parts` (61-80) | exact |
| `.cargo/semver-checks-allowlist.toml` (zero `[[entry]]`, new header) | config (register mirror) | batch | itself, header lines 1-30 ("EMPTY AS OF THIS PLAN") | exact |
| `Cargo.toml`, `crates/{paladin-core,paladin-battalion,paladin-llm,paladin-ports,paladin-web}/Cargo.toml` (delete lint tables + notes) | config | n/a (deletion) | the tables themselves | exact |
| `MIGRATION.md` (new `# 11.` block above `# 10.`) | docs (register) | n/a | `MIGRATION.md` `# 10.` block, lines 5-346 | exact |
| `CODEQL-DISMISSALS.md` (new `[[dismissal]]` rows) | docs (register) | n/a | existing rows 28 and 40-49 (lines 52-120) | exact |
| `52-SEMVER-RED-EVIDENCE.md`, `52-CI-EVIDENCE.md` (new) | docs (evidence) | n/a | `.planning/milestones/v0.11.0-phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-MUSTER-RED-EVIDENCE.md` | exact |
| `docs/src/appendix/release-automation.md` (label act; publish-order paragraph) | docs | n/a | itself: `## Canonical Publish Order` (74), `**Packaging note:**` (101), `## Operator Guide` (108) | exact |
| `docs/src/appendix/release-checklist.md`, `docs/src/appendix/branch-protection.md`, `docs/src/deployment/cicd.md`, `docs/src/contributing/branching-model.md` | docs | n/a | `cicd.md` job table rows (lines 60-85) | exact |
| `Makefile` line ~213 (`check-migration-allowlist` help text) | config | n/a | itself | exact |
| `.github/rulesets/protect-main-branch.json` (optional record edit) | config | n/a | itself; validated by `check-workflow-triggers.sh` clause 3 | exact |

## Pattern Assignments

### `tests/scripts/create-or-reuse-release_test.sh` new assertion (test, request-response)

**Analog:** same file. A new assertion follows the per-case shape: a `DIRn` under `${SCRATCH}`, `write_gh_stub`, status fixture files, then `assert_silent` / `assert_fire` / `assert_call_count`, with ad hoc `ASSERTIONS`/`FAILED` accumulation for payload checks.

**Harness skeleton to copy** (lines 28-48):
```bash
set -uo pipefail
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/create-or-reuse-release.sh"
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/create-or-reuse-release-test.XXXXXX")"
cleanup() { rm -rf "${SCRATCH}"; }
trap cleanup EXIT
FAILED=0
ASSERTIONS=0
MUTATION_WATCH_PATHS=(scripts .github/workflows)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"
```

**Stub contract** (lines 63-130). The stub writes the POST stdin to `${dir}/captured_payload` (`cat - > "${SCRATCH_DIR}/captured_payload"`), logs `LOOKUP`/`CREATE` to `call_log`, and reads `lookup_status`, `lookup_body`, `create_status`, `create_body`. `run_script` (lines 143-148) forwards the environment, so the limit env var is set with `PALADIN_RELEASE_BODY_LIMIT=500000 run_script ...` or inline in a variant of `run_script`.

**Large-input precedent** (Case 9, lines 357-372). Size comes from a named variable and the input is generated at test time:
```bash
DIR9="${SCRATCH}/case9"
mkdir -p "${DIR9}"
write_gh_stub "${DIR9}"
PADDING_SIZE=200000
PADDING="$(head -c "${PADDING_SIZE}" /dev/zero | tr '\0' 'a')"
echo "200" > "${DIR9}/lookup_status"
...
assert_silent "${DIR9}" "lookup 200 with a response body larger than the pipe buffer (${PADDING_SIZE}+ chars) reuses the release" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR9}" "large-body lookup 200 makes no create call" 0
```

**Payload-content assertion precedent** (Case 8, lines 330-355). Uses `ASSERTIONS=$((ASSERTIONS + 1))`, then `PASS:`/`FAIL:` echo, `FAILED=$((FAILED + 1))`, and dumps with `sed 's/^/  | /'`. Add the new payload checks in this shape:
```bash
ASSERTIONS=$((ASSERTIONS + 1))
if [ "$(jq -r '.body | length' "${DIRX}/captured_payload")" -gt 131072 ]; then
    echo "PASS: ..."
else
    echo "FAIL: ..."
    FAILED=$((FAILED + 1))
fi
```
Also assert byte-identity for the under-cap case with `jq -j .body "${DIRX}/captured_payload" | cmp - "${BODY_FILE}"`.

**Tail to keep last:** the real-tree mutation check and summary (lines 376-405) stay at the end. Insert new cases before the `# --- Missing --tag` block (around line 374).

**Constraints from RESEARCH:**
- D-10 must raise the limit above the body size, otherwise D-11's default 125,000 cap truncates it (Discrepancy 4).
- Generate the body with `awk`, no committed fixture.
- Record red-first against the unmodified script (exit 126, "Argument list too long", zero CREATE calls).
- Cover multi-byte text and a single huge line with no newline.

---

### `tests/scripts/finalize-release-body_test.sh` (test, transform)

**Analog:** same file, Case 12 (lines 489-500) and Case 13 (502-517).

Pure composition uses the `compose_to` helper through `FINALIZE_RELEASE_BODY_LIB_ONLY=1`:
```bash
compose_to case12 run_a.md "${CURATED1}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" "450"
RUN_A="${LAST_COMPOSE_FILE}"
assert_cmp "case12: compose #1 and #2 over identical inputs are byte-identical" "${RUN_A}" "${RUN_B}"
```
The round trip through the stubbed `gh` uses `run_finalize` plus the `current_body` fixture (Case 13). It is the pattern for "a second finalize run over its own output stays byte-identical" with a truncation footer present. Case 18 (line 695) is the second idempotency precedent.

**Where the guard attaches in the script:** `compose_release_body` (296-330) builds the body, and `finalize_release_body_main` (lines 470-486) calls `"${GH}" release edit "${TAG}" --notes-file "${OUTPUT}"`. The size check goes between those two calls. The curated text is everything before `MARKER='<!-- paladin:release-artifacts -->'` (line 118).

---

### `tests/scripts/check-migration-allowlist_test.sh` (test, batch)

**Analog:** same file. Fixture writers to retarget (lines 50-80):
```bash
write_migration_two_parts() {   # ~line 62: writes ## 10.1/10.2/10.3 then ## 9.2/9.3
    ...
        echo "## 10.2 Rust API changes (compile-affecting; the X-10 register)"
        echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
        echo "| --- | --- | --- | --- | --- | --- |"
        printf '%s\n' "${body10}"
        echo "## 10.3 Toolchain & dependencies"
```
Other helpers: `migration_row CRATE TYPE MARKER` (line ~83), `allowlist_entry CRATE TYPE`, and `NO_MANIFESTS` (an empty `MANIFEST_ROOT` for cases that are not about the crate-wide mirror), and `run_guard MIGRATION ALLOWLIST [MANIFEST_ROOT]`.

**Edit:** the comment at line 61 says "the guard's `(9|10)` window", so rewrite it. The fixtures become `## 11.1`/`## 11.2`/`## 11.3` with the old `## 10.2`/`## 9.2` blocks following, so the new assertions can show that `Y` rows under §10.2 and §9.2 are NOT read, a `Y` row under §11.2 with no entry fires, and the window ends at `## 11.3` (RESEARCH Pitfall 3).

---

### `scripts/check-migration-allowlist.sh` (utility, batch)

**Analog:** same file. The only logic change is line 94:
```bash
awk '/^## (9|10)\.2 /{flag=1; next} /^## (9|10)\.3 /{flag=0} flag' "${MIGRATION_FILE}" \
```
becomes `/^## 11\.2 /` and `/^## 11\.3 /`. Update the two `echo` lines and the `::error::` text that name §9.2/§10.2.

**Header rewrite** (lines 1-12). Remove "copied VERBATIM from ci.yml ... Do not 'improve' the parsing". State that this is the single implementation CI calls. Also remove "LOCAL-ONLY EXTENSION (not in the ci.yml step)" (lines 31-37), since the extension is now enforced in CI. The `MIGRATION_FILE` / `ALLOWLIST_FILE` / `MANIFEST_ROOT` override seam (lines 49-53) stays unchanged:
```bash
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATION_FILE="${MIGRATION_FILE:-${WORKSPACE_ROOT}/MIGRATION.md}"
ALLOWLIST_FILE="${ALLOWLIST_FILE:-${WORKSPACE_ROOT}/.cargo/semver-checks-allowlist.toml}"
MANIFEST_ROOT="${MANIFEST_ROOT:-${WORKSPACE_ROOT}}"
```

---

### `.github/workflows/ci.yml` gate-script step (config, event-driven)

**Analog:** the `cargo-deny` ("License & Dependency Policy") job, lines 114-170. It is a required job with no job-level `if:`, and it already runs `chmod +x scripts/*.sh`. Copy the step shape:
```yaml
      - name: Check publish order
        run: ./scripts/check-publish-order.sh

      - name: Check core/ports dependency allowlist (ADR-0015)
        run: ./scripts/check-dependency-allowlist.sh
```
New step, placed beside them, preceded by a short comment in the same style (phase/decision reference, why it is here rather than in the advisory `semver` job):
```yaml
      - name: Check semver allowlist is set-equal to the MIGRATION.md §11.2 register
        run: ./scripts/check-migration-allowlist.sh
```
**Delete** the inline step at 419-490 ("Verify allowlist is set-equal to the MIGRATION.md §9.2 and §10.2 registers"). **Keep** the `TBD`-marker step (492-513) as is. Consider moving it beside the new step (RESEARCH Pattern 2). Its comment says never to spell the four-letter token in prose.

---

### `.github/workflows/ci.yml` job-level `if:` with label condition (config, event-driven)

**Analog (existing `if:` forms):** line 2015 `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` (e2e), line 2075 `if: github.event_name == 'pull_request' || github.event_name == 'workflow_dispatch'` (benchmark signal), and line 2156 (publish-dry-run, the same as e2e).

**Edit for `e2e-tests`** (lines 2010-2016). Remove the two stale comment lines ("use this under normal behavior", "use this for troubleshooting e2e tests") and the commented-out `if:`. Use the RESEARCH Code Example 2 form:
```yaml
    if: >-
      (github.event_name == 'push' && github.ref == 'refs/heads/main') ||
      (github.event_name == 'pull_request' &&
       contains(github.event.pull_request.labels.*.name, 'release'))
```
**Edit for `publish-dry-run`** (line ~2152): delete its `if:` line, and fix the header comment "(main branch only)".

**Trigger edit** (lines 22-23):
```yaml
  pull_request:
    branches: [ main, 'release/**' ]
    types: [ opened, synchronize, reopened, labeled ]
```
The label name is used only inside the `contains()` expression, never inside a `run:` (workflow-injection rule). `check-workflow-triggers.sh` compares trigger names and the push branch filter only, so `types:` does not trip it. Mirror the change in the `ci.yml` row of the trigger-policy table in `docs/src/contributing/branching-model.md` (one pipe row, no merged cells). The concurrency comment at lines 27-30 claims push and PR share a group. RESEARCH Discrepancy 6 says they do not, so fix or soften it.

---

### `.github/workflows/ci.yml` semver step (config)

**Analog:** RESEARCH Code Example 1 is the verified command shape, copied into the existing `semver` job at ~381-400. Other things to match: job `name:` becomes `Semver Checks (vs v0.11.0)`, the package array uses published names (`paladin-ai-core`, not `paladin-core`) and adds `paladin-eval` for twelve, and `--default-features` stays. Parse the check count with a tolerant `[1-9][0-9]* checks:` pattern after stripping ANSI.

---

### `scripts/create-or-reuse-release.sh` (utility, request-response)

**Analog:** same file. Change is confined to `_cor_build_payload` (lines 99-120). Current:
```bash
        body_text="$(cat "${body_file}")"
    ...
    jq -n \
        --arg tag_name "${tag}" \
        --arg name "${name}" \
        --arg body "${body_text}" \
```
Target (RESEARCH Code Example 3):
```bash
    jq -n \
        --arg tag_name "${tag}" \
        --arg name "${name}" \
        --rawfile body "${body_file:-/dev/null}" \
        --argjson draft false \
        --argjson prerelease "${prerelease}" \
        '{tag_name: $tag_name, name: $name, body: $body, draft: $draft, prerelease: $prerelease}'
```
Keep the `"ERROR: --body-file not found"` branch (lines 104-108). Update the doc comment at lines 21-26 and 95-98, which say `jq -n --arg`. Payload still goes to `gh` through `printf '%s' "${payload}" | "${GH}" api ... --input -` (a builtin plus stdin, so it is not an argv path). `PAYLOAD=$(...)` is bash-internal.

**Error/usage conventions to copy for the new limit variable:** `echo "ERROR: ..." >&2; return 1` inside `create_or_reuse_release_main`. Log sizes only, never body text or headers (credential-review rule in `.github/instructions/security.instructions.md`).

**Sourcing seam** (lines 245-247): the `*_LIB_ONLY` guard at the bottom stays:
```bash
if [ "${CREATE_OR_REUSE_RELEASE_LIB_ONLY:-0}" != "1" ]; then
    create_or_reuse_release_main "$@"
fi
```

---

### `scripts/<shared size helper>` (utility, transform)

**Analog:** `scripts/check-publish-order.sh` reads another script's data through its sourcing seam instead of keeping a second copy (lines 125-131):
```bash
    # shellcheck source=scripts/publish-crates.sh
    PUBLISH_CRATES_LIB_ONLY=1 source "${PUBLISH_CRATES_SCRIPT}"
    if [ "${#CRATES[@]}" -eq 0 ]; then
        echo "ERROR: CRATES_EMPTY -- ..." >&2
        return 1
    fi
```
Copy these conventions for the new helper:
- Per-script `<NAME>_LIB_ONLY` guard at the bottom (every script in `scripts/` has one).
- Resolve the path from `WORKSPACE_ROOT`, never the caller's cwd.
- A missing helper is a named failure.
- A `# shellcheck source=` directive.
- `python3 - <<'PY'` for character counting (not `wc -m`).
- Inputs stay data: pass paths, not text.

---

### `.cargo/semver-checks-allowlist.toml` (config, register mirror)

**Analog:** same file, header lines 1-30. It already has an "EMPTY AS OF THIS PLAN (22-04)" paragraph whose wording is the template for the emptied state. Rewrite lines 1-10 ("Source of truth: MIGRATION.md §9.2 and §10.2 ... both read by the gate") to name §11.2 only. Replace the "EMPTY AS OF" paragraph, or reuse it, to say zero `[[entry]]` and that Phase 53/55 add an entry in the same commit as the §11.2 row. Remove all 85 `[[entry]]` blocks. Keep the "Schema per entry" comment block (from line 29).

---

### `MIGRATION.md` `# 11.` block (docs, register)

**Analog:** `MIGRATION.md` `# 10.` block. Line 1 `# Upgrading Paladin`, line 3 the scope note ("has two parts, newest first" becomes three), line 5 `# 10. Upgrading from v0.10.1 to v0.11.0`. The skeleton is `## 10.1 Behavioral changes` (11), `## 10.2 Rust API changes (compile-affecting; the X-10 register)` (67), `## 10.3 Toolchain & dependencies` (210), `10.4 Persistence & schema migrations` (217), `10.5 Configuration & environment` (232), `10.6 HTTP API` (278), `10.7 Deprecations` (339), `10.8 Upgrade checklist` (346).

**Table header to copy exactly** (from §10.2, under line 67):
```
| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |
| --- | --- | --- | --- | --- | --- |
```
Place the new `# 11. Upgrading from v0.11.0 to v0.11.1` part between the scope note and `# 10.`. It must contain a `## 11.3 ...` heading so the awk window ends (Pitfall 3), so give it the full 11.1-11.8 skeleton. §11.2 has the header and separator, with no data rows. Never write the four-letter placeholder token anywhere in `MIGRATION.md`, since the CI marker step fails on any occurrence. §9/§10 bodies are byte-frozen. Only the scope note (line 3) changes outside the new block.

---

### `CODEQL-DISMISSALS.md` `[[dismissal]]` row (docs, register)

**Analog:** rows 40 and 49 are the full 11-field shape (alert 28 has the same fields, with `dismissed_reason = "used in tests"` and `review_date = "2026-12-31"`). Row template, from alert 40 (lines ~66-75):
```toml
[[dismissal]]
alert_number = 40
rule_id = "rust/cleartext-logging"
path = "crates/paladin-memory/src/services/rag_retrieval_service.rs:106"
why_present = "..."
why_dismissed = "..."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-03-31"
scope = "..."
compensating_control = "..."
revisit_condition = "... -- whichever comes first"
```
Fields (all eleven mandatory, non-empty): `alert_number, rule_id, path, why_present, why_dismissed, dismissed_reason, owner, review_date, scope, compensating_control, revisit_condition`. For a test-fixture literal, copy the alert 28 prose style. Cite the enclosing `#[cfg(test)]` block and state that the literal never leaves the compilation unit. For a heuristic false positive, copy alerts 45-47's style: name the over-approximation and the exact source lines inspected.

**Three places to edit per addition:**
1. The TOML rows inside the fenced block between `<!-- BEGIN MACHINE-READABLE REGISTER -->` and `<!-- END MACHINE-READABLE REGISTER -->`.
2. `Declared dismissals: 6` (line ~48).
3. The heading `## The 6 live dismissals` (line ~46).

Add a header sentence for D-14: Phase 52 rows share `review_date = "2027-06-30"`, while rows 28/40/45-49 keep their dates (`2026-12-31` / `2027-03-31`). Verify with `make check-codeql-dismissals` and `tests/scripts/check-codeql-dismissals_test.sh`. The 24-alert list needs maintainer-run `gh api` (sandbox gets 403), so the rows depend on a `checkpoint:human-action`. The reconciliation command is in the file header:
```bash
gh api "/repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=dismissed&per_page=100" --paginate
```
(use `state=open` for the open-alert pull).

---

### `52-SEMVER-RED-EVIDENCE.md` / `52-CI-EVIDENCE.md` (docs, evidence)

**Analog:** `.planning/milestones/v0.11.0-phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-MUSTER-RED-EVIDENCE.md`.

Shape:
- YAML frontmatter: `phase`, `slug`, `status`, `created`.
- H1 "Phase N — ...: proven-red evidence".
- A blockquote stating why the proof exists ("A guard that cannot fail proves nothing") and that no mutated source is committed.
- `Tree: <sha>` line (plus the git tree object).
- An environment line (e.g. "Every cargo invocation ran with `CARGO_INCREMENTAL=0`").
- A numbered list of the conditions that count as proof, "checked by counts and presence, never by diagnostic order".
- One `## Experiment N — ...` section per mutation, containing the scripted mutation, its transcript and the tree restoration.

Mutation script conventions worth copying:
- Dirty-baseline guard (`DIRTY-BASELINE`).
- `trap 'git checkout -- <files>' EXIT` installed before the first edit.
- A mutation-applied guard that aborts with `MUTATION-NOT-APPLIED` instead of passing vacuously.
- The script lives outside the repo.

Phase 52 specifics (RESEARCH): the verified break is `crates/paladin-ports/src/output/spend_scope.rs` line 230, `pub fn current_attempt_spend` to `fn`. Expected: lint `function_missing`, exit 100, `N checks: X pass, 1 fail`. Do not break `paladin-herald` items (`#[doc(hidden)]`, undetectable). Record the per-package `223 checks: 223 pass, 31 skip` line, the 28-line allow-table count, the CodeQL triage table, and the issue #41 comment text.

---

### `docs/src/appendix/release-automation.md` and sibling docs (docs)

**Analog:** same file. Anchors: `## Canonical Publish Order` (line 74), `**Packaging note:**` (101), `## Operator Guide: Cutting a Release` (108). Insert the dated D-16 paragraph before the Packaging note, and add the `release` label step to the Operator Guide. Evidence pointers: `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-01-PLAN.md` (row `A-SC3`, ~line 107) and `47-08-SUMMARY.md`.

`docs/src/deployment/cicd.md` job-table row to rewrite (line 65), pipe table `| id | Name | description | Required/Advisory |`:
```
| `semver` | Semver Checks (vs v0.9.0) | `cargo-semver-checks` for every publishable crate against the published v0.9.0 baseline | Advisory |
```
Also update the `e2e-tests` row (~83) and the `publish-dry-run` row (~85). `release-checklist.md` lines ~66-69 still say `semver` excludes `paladin-eval`. `branch-protection.md` holds the count "44" and the ruleset-update procedure (id `20868126`). ADR-0048 and `.project/` texts are historical and stay unedited.

---

## Shared Patterns

### Offline, named-failure gate script
**Source:** `scripts/check-migration-allowlist.sh`, `scripts/check-publish-order.sh`
**Apply to:** the new size helper and every edited gate script.
- `set -euo pipefail`.
- `WORKSPACE_ROOT` derived from `BASH_SOURCE[0]`.
- Env-var override seams for fixtures.
- Missing input is a named `ERROR:` plus a non-zero exit, never a silent pass.
- Scratch via `mktemp -d`, never a fixed `/tmp` path.
- `*_LIB_ONLY` sourcing seam at the bottom.
- Report all violations before exiting.

### Regression-harness skeleton
**Source:** `tests/scripts/create-or-reuse-release_test.sh` lines 28-48 and 376-405
**Apply to:** all three edited `_test.sh` files.
- `set -uo pipefail` (no `-e`).
- `SCRATCH=$(mktemp -d ...)` plus a `trap cleanup EXIT`.
- `FAILED`/`ASSERTIONS` counters, with `PASS (...)`/`FAIL:` lines.
- A final git-status mutation check over `scripts .github/workflows`.
- Exit via `❌ N/M assertion(s) failed` or `✅ N assertion(s) passed`.
- `make test-shell-guards` discovers `tests/scripts/*_test.sh` by glob, so no registration is needed.

### Stubbed `gh` through `GH_BIN`
**Source:** `tests/scripts/create-or-reuse-release_test.sh` `write_gh_stub` (lines 63-130), and the `finalize-release-body_test.sh` stub, which serves `release view --json body -q .body` from a `current_body` fixture and copies `release edit --notes-file` back into it.
**Apply to:** any release-script assertion. Never call the real `gh` or the network.

### Release body is data, never shell text
**Source:** `scripts/create-or-reuse-release.sh` header (lines 21-26) and `_cor_build_payload`
**Apply to:** the payload builder and the truncation helper.
- File path in, `jq --rawfile`, `gh api --input -` on stdin.
- No `echo "$body"` into a command line.
- Keep Case 8's literal-`EOF`/backtick/`$(...)` assertion green.

### Workflow edits
**Source:** the `cargo-deny` job comments in `ci.yml` (lines 114-170) and `.github/instructions/security.instructions.md`
**Apply to:** every `ci.yml` edit.
- Each added step gets a short comment citing its phase/decision.
- Never `${{ ... }}` inside `run:`.
- No inline advisory suppressions (`check-workflow-suppressions`).
- Any `on:` change is mirrored in the trigger-policy register (`check-workflow-triggers`).
- Every condition must be `actionlint`-clean (CI `Workflow Lint`; the sandbox cannot run it).

### Governed register + count line
**Source:** `CODEQL-DISMISSALS.md`, `MIGRATION.md §10.2` with `.cargo/semver-checks-allowlist.toml`
**Apply to:** `CODEQL-DISMISSALS.md`, `MIGRATION.md`, the allowlist.
A row and its machine-readable mirror change in the same commit, and counts and headings are updated with them.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Truncation / fit logic inside the shared size helper (line-boundary cut, fence balancing, sentinel footer, idempotent re-fit) | utility | transform | No existing script truncates text by character count. Use RESEARCH Pattern 5 (steps 1-6 and the finalize interplay). Only the sourcing and harness conventions above have analogs. |

## Metadata

**Analog search scope:** `scripts/`, `tests/scripts/`, `.github/workflows/ci.yml`, `Makefile`, `MIGRATION.md`, `CODEQL-DISMISSALS.md`, `.cargo/semver-checks-allowlist.toml`, `docs/src/appendix/`, `docs/src/deployment/`, `.planning/milestones/v0.11.0-phases/49-*`.
**Files scanned:** about 20 read in full or in targeted ranges; the rest located by grep.
**Not read in full (planner should open them):** `scripts/finalize-release-body.sh` lines 76-295 (helper functions), the `compose_to`/`run_finalize` helpers and the stub in `finalize-release-body_test.sh`, the six `Cargo.toml` lint tables (line starts in RESEARCH Discrepancy 1), `check-workflow-triggers.sh`, and `branching-model.md`.
**Pattern extraction date:** 2026-10-28
