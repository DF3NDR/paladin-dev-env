# Phase 56: Dogfood Findings - Pattern Map

**Mapped:** 2026-11-02
**Files analyzed:** 12 (new or modified)
**Analogs found:** 9 / 12 (3 are content/evidence files with only format templates)

Line numbers below are from 2026-11-02 reads. Per D-00e, locate by content, not number.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `scripts/check-findings-log.sh` | utility (offline gate) | batch / transform (parse markdown) | `scripts/check-version-strings.sh` | exact (shape) |
| `tests/scripts/check-findings-log_test.sh` | test (shell fixture harness) | batch | `tests/scripts/check-version-strings_test.sh` (also `check-no-stub-panics_test.sh`) | exact |
| `Makefile` (`check-findings-log` target) | config | request-response | `check-version-strings` target (Makefile ~224-226) | exact |
| `tests/config_example_roundtrip.rs` | test (integration) | file-I/O + transform | `src/config/web_server.rs` example test (~145-177); `tests/consumer_surface_muster.rs` for top-level layout | role-match |
| `tests/fixtures/config/config.example.full.yml` | config fixture | file-I/O | `tests/fixtures/config/v0.9.0-config.example.yml` (provenance only) and root `config.example.yml` (content) | role-match |
| `tests/fixtures/config/README.md` (provenance paragraph) | docs | n/a | existing sections of the same README | exact |
| `.project/v0.11.1/02-dogfood-findings-log.md` (appended entries) | docs / log | append-only | its own entry format (lines 11-22) and DF-01..DF-04 | exact |
| `56-FIRST-RUN-WALK.md` | evidence | n/a | `.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md` | role-match |
| `56-RUNBOOK-*.md` (coverage, web3sec, keyed walk) | docs / runbook | n/a | `54-FIRST-EVIDENCE.md`; RESEARCH Answer 6 and Answer 8 | partial |
| `.planning/todos/completed/2026-08-13-verify-local-coverage-reproduction.md` (moved from pending) | todo | n/a | `.planning/todos/completed/2026-10-26-record-v0-11-0-coverage-lines-figure.md` | exact (move convention) |
| `.planning/todos/completed/` or `pending/` for web3sec todo | todo | n/a | same folder | exact |
| Stage C fix files | n/a | n/a | none yet (written from the log) | n/a |

## Pattern Assignments

### `scripts/check-findings-log.sh` (utility, batch/transform)

**Analog:** `scripts/check-version-strings.sh` (306 lines). `check-no-stub-panics.sh` is the same shape.

**Header / seam / preflight pattern** (lines 1-8 comment style, 66-75):
```bash
#!/usr/bin/env bash
# check-version-strings.sh
#
# Offline, check-only gate that ... (Phase 54, FIRST-03, D-06..D-12). ...
# Test-only seam: VERSION_STRINGS_ROOT relocates every input ...
# Usage:  ./scripts/check-version-strings.sh
# Exit:   0 when ..., 1 otherwise.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${VERSION_STRINGS_ROOT:-${WORKSPACE_ROOT}}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
```
New script: seam name `FINDINGS_LOG_PATH`, default `${WORKSPACE_ROOT}/.project/v0.11.1/02-dogfood-findings-log.md`. No arguments. stdlib python only (no PyYAML).

**Named-failure / status-line pattern** (lines ~255-285):
```python
if not texts:
    named.append(("ZERO_FILES",
                  f"ZERO_FILES: scanned 0 file(s) under {root}; a broken scan set "
                  "is a named failure, never a pass."))
...
if named or findings:
    print(named[0][0] if named else "MISMATCH")
    for _, detail in named:
        print(detail)
    for path, lineno, found in findings:
        print(f"{path}:{lineno}: found '{found}', want '{want}'")
    sys.exit(0)
print("OK")
print(f"OK: {len(texts)} file(s) scanned, ...")
```
Map to: `UNREADABLE`, `ZERO_ENTRIES`, `BAD`, `OK`. The OK line must print the entry count (the real-log test asserts N >= 4).

**Shell wrapper tail** (lines ~288-306):
```bash
STATUS_LINE=$(head -n1 <<<"${REPORT}")
DETAIL=$(tail -n +2 <<<"${REPORT}")
if [ "${STATUS_LINE}" = "OK" ]; then
    echo "✅ ${DETAIL}"; exit 0
else
    echo "❌ Version-string check failed (${STATUS_LINE})"
    echo ""; echo "${DETAIL}"; ...
    exit 1
fi
```
`REPORT=$(python3 - "${ROOT}" <<'PY' ... PY)`. Output sorted so two runs are byte-identical. The file must be `100755`; `make lint-shell` runs shellcheck over every tracked `*.sh`.

**Parse rules** (RESEARCH Pattern 1, "Concrete parse rules"). Follow exactly:
- Ignore fenced regions (toggle on a line whose `strip()` starts with three backticks). The template `### DF-nn` heading and its `Disposition: open | in v0.11.1 | …` line sit inside a fence at log lines 12-22.
- Heading `^### (DF-(\d+))\b`. Body runs to the next `^#{1,3} ` outside a fence, or EOF.
- Bullet `^- \*\*Disposition(?: \(([^)]*)\))?:\*\*\s*(.*)$`. The last match wins; none gives `MISSING`.
- Classify: `^open\b` is OPEN (pre-seeded `open → recommend **in v0.11.1**` must read OPEN); `^in v0\.11\.1\b` plus a non-empty pointer, else `NO_POINTER`; `^out \((.+)\)` with a non-blank reason, else `BAD_OUT`; `^duplicate of (DF-\d+)` whose target exists and is not self, else `BAD_DUP_TARGET`; anything else `UNRECOGNISED`.
- Ids are integers: `DUPLICATE_ID` and `NOT_INCREASING`. The next free id is DF-05.
- Per-bad-entry line format: `path:line: DF-nn: <STATUS>`.

---

### `tests/scripts/check-findings-log_test.sh` (test, batch)

**Analog:** `tests/scripts/check-version-strings_test.sh` (helpers `make_tree`, `fresh`, `plant`, `run_guard`, `assert_fire`, `assert_silent`, no-mutation checksums).

**Preamble pattern** (lines 20-45):
```bash
set -uo pipefail
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-version-strings.sh"
if [ ! -f "${GUARD}" ]; then echo "ERROR: guard script not found at ${GUARD}" >&2; exit 1; fi
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-version-strings-test.XXXXXX")"
cleanup() { rm -rf "${SCRATCH}"; }
trap cleanup EXIT
FAILED=0
ASSERTIONS=0
```

**Runner / assert pattern** (lines ~107-140):
```bash
run_guard() {
    LAST_OUTPUT="$(VERSION_STRINGS_ROOT="$1" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}
assert_fire() {  # non-zero exit AND every NEEDLE appears in the output
    local root="$1" desc="$2"; shift 2
    ASSERTIONS=$((ASSERTIONS + 1)); run_guard "${root}"
    if [ "${LAST_STATUS}" -eq 0 ]; then echo "FAIL: expected non-zero exit for: ${desc} (got 0)" ...
```
Swap the env var to `FINDINGS_LOG_PATH="$1"` (a file, not a root dir). Ordering: failing cases first (open, missing, `out` with no reason, duplicate target missing, duplicate id, decreasing id, bare `in v0.11.1`, last-bullet-wins both ways, fenced template ignored), then named failures (`ZERO_ENTRIES`, `UNREADABLE`), then determinism, then passing cases (a dated-note disposition appended after `open`), then the real log.

**Tail pattern** (final lines):
```bash
if [ "${FAILED}" -eq 0 ]; then echo "✅ ${ASSERTIONS} assertion(s) passed."; exit 0
else echo "❌ ${FAILED}/${ASSERTIONS} assertion(s) failed."; exit 1; fi
```

**Deviation from the analog (D-03, RESEARCH):** the analog ends with a "real tree passes" assertion. Do NOT copy that. The real log legitimately holds `open` mid-phase and CI runs this test. Assert only that the script parsed >= 4 entries and printed neither `ZERO_ENTRIES` nor `UNREADABLE`. Keep the "real file not mutated" checksum case, on the log file. The file must be `chmod +x`. `make test-shell-guards` picks it up by the `tests/scripts/*_test.sh` glob, so no Makefile edit is needed for the test.

---

### `Makefile` (`check-findings-log` target)

**Analog:** `check-version-strings` target.
```make
.PHONY: check-version-strings
check-version-strings: ## Verify every copyable version string (README, docs/src, k8s/, openapi.json) equals the workspace version (FIRST-03)
	@./scripts/check-version-strings.sh
```
Add the same shape for `check-findings-log`. **Do NOT add it to `check-gates:`** (line ~272). Use the comment style of `check-release-consistency` (Makefile ~233-238, "Deliberately NOT part of check-gates: ...") to say why.

---

### `tests/config_example_roundtrip.rs` (test, file-I/O + transform)

**Analogs:**
- Production-loader precedent: `src/config/web_server.rs` ~145-177 and `src/config/agents.rs` ~1083-1102 (`Settings::load_from_file("config.example.yml")`).
- Layout precedent: top-level `tests/*.rs` is auto-discovered (no `autotests = false`). `tests/consumer_surface_muster.rs` is a top-level, module-doc-heavy test with a header explaining its rule.
- Loader under test, `src/config/settings.rs` ~121:
```rust
pub fn load_from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
    let format = if filename.ends_with(".yml") || filename.ends_with(".yaml") { FileFormat::Yaml } else { FileFormat::Toml };
    let cfg = Config::builder().add_source(File::new(filename, format)).build()?;
    Ok(cfg.try_deserialize()?)
}
```

**Core pattern** (RESEARCH Pattern 2, proven by a scratch run):
```rust
use paladin::config::Settings;
fn paths(prefix: &str, v: &serde_json::Value, out: &mut BTreeSet<String>) { /* Object -> k, Array -> [i], recurse */ }
// input:  serde_yaml::from_str::<serde_json::Value>(&text)       (independent parser; dev-dep serde_yaml = "0.9", already in Cargo.lock)
// output: serde_json::to_value(&Settings::load_from_file(path)?) (production loader)
// missing = input_paths.difference(&output_paths)
```
Rules:
- Resolve paths with `env!("CARGO_MANIFEST_DIR")`, as `llm.rs` does with `concat!(env!("CARGO_MANIFEST_DIR"), "/../../config.example.yml")`.
- Build paths as `Vec<String>` segments, not `.`-joined strings. Keys such as `gpt-5.5.1` contain dots.
- Compare key paths only, never values. `f32` temperature and unquoted-decimal normalisation make value comparison noisy.
- Never print `Settings` or the serialised JSON in an assertion message. `ApiKeyConfig.key` serialises and Debug-prints in plaintext.
- Exemption table, one reason per row: `llm.<provider>.api_key` (x9, `skip_serializing`, WR-04) and `treasurer.allowance.webhook.secret`.
- `KNOWN_GAPS` table: each row cites its `DF-nn`, and a row whose key now round-trips fails the test.
- Include a watched-red walker case (inline YAML with a typo key must be reported).
- Drift guard: fail when a key path in `config.example.yml` (live or commented-out) is missing from the full fixture.
- Load the Redis-cadence variant (`backend: { redis: { url_env } }`) as a second small fixture.
- Keep `openai-compatible` hyphenated in the fixture. Do not put `models: {}` under a provider.
- Measured expectation on HEAD: verbatim example drops exactly 8 `api_key` paths; the full fixture drops exactly 9.

**Error handling:** existing tests use `unwrap_or_else(|e| panic!(...))`. That is acceptable in test code. Library code stays `unwrap`-free.

---

### `tests/fixtures/config/config.example.full.yml` (fixture)

**Analog (content):** root `config.example.yml` with every commented-out optional block enabled. Do not uncomment the shipped example (D-09, Phase 54 D-04 "verbatim copy fails closed").
**Fixture composition checklist:** RESEARCH Pattern 2 "Fixture composition". The key items:
- Provider `max_tokens`, `context_window`, `reasoning_*`, and `models` with `/` and `.` ids.
- Two `http.auth.api_keys` with fake literal keys (`test-key-not-a-secret`, never `${…}`) and a tenant `Acme-Prod`.
- `treasurer.pricing` ids `Qwen/qwen3.7-plus` and `gpt-5.5.1`; money values quoted (`"2.50"`); `allowance` and `cadence`.
- `agent_runtime` per_tool, guardrail rule and model_fallback; `trace.otel.headers`; an agent with `reasoning_effort: {budget: 2048}`.
- Both temperature shapes: `{fixed: …}` and `{range: […]}`, plus `not_sent`.

---

### `tests/fixtures/config/README.md` (provenance paragraph)

**Analog:** the same file's existing sections. Each fixture gets: source, producing command, blob SHA, re-verify command. See "v0.9.0-config.example.yml" at lines ~34-40 ("Producing command: `git show v0.9.0:... > ...`", "Blob SHA", "Re-verify with").
**Deviation:** the new fixture is derived and hand-maintained, NOT a frozen tag copy. The README's current opening claims "Both files ... are frozen, byte-identical copies". Add a clearly separate section so that statement stays true. Record the derivation (from the root `config.example.yml`, which sections were enabled and which extras added), the drift-guard test name, and that no blob SHA is pinned.

---

### `.project/v0.11.1/02-dogfood-findings-log.md` (append entries)

**Analog:** its own entry format (lines 11-22) and DF-01..DF-04 (lines 49-93).
```
### DF-nn — <one-line title>
- **Found:** <date> · **By:** <maintainer | web3sec | other> · **Surface:** <cli | server | library | docs | ci | k8s>
- **Version:** 0.11.0 (crate / tag / commit if different)
- **What happened:** <observed behavior, verbatim error text in a code block>
- **Expected:** ...
- **Repro:** ...
- **Severity:** S1 security/isolation · S2 wrong result or crash · S3 misleading docs/config · S4 hygiene
- **Related:** <WINDOWS row, review id, inventory row, or none>
- **Disposition:** open | in v0.11.1 | out (reason) | duplicate of DF-nn
```
**Append rules (D-01):**
- Never edit an existing line. Existing bullets read `- **Disposition:** open → recommend **in v0.11.1**` and stay byte-intact.
- Add a dated bullet under each entry: `- **Disposition (2026-11-xx):** in v0.11.1 — fixed by FIRST-01 (Phase 54); proof: <test/gate>`.
- New entries go below `<!-- append new findings below this line -->` (line 95), starting at DF-05.
- Pointers per D-02 / RESEARCH Answer 7: DF-01 -> FIRST-01 (`validate_refuses_an_unexpanded_placeholder_key` etc.); DF-02 -> FIRST-02/03 (`make check-version-strings`); DF-03 -> FIXES-01 (`make check-no-stub-panics`; file deleted); DF-04 -> TENANT-03..05/07 (`every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404` etc.).
- The disposition value must match the checker's grammar exactly (`in v0.11.1 — <non-empty pointer>`).

---

### `56-FIRST-RUN-WALK.md` and `56-RUNBOOK-*.md` (evidence / runbook)

**Analog:** `.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md` (evidence with command, exit code, output, prove-then-lock table). Read it at plan time for its section layout. No excerpt was taken here.
**Content sources for the runbooks:**
- Keyless walk: RESEARCH Answer 5. Scratch crate outside the workspace; `cargo add paladin-ai`; `cargo install paladin-ai --version 0.11.0 --features cli --bin paladin-cli` (the form without `--features cli` exits 101); copy `basic_paladin.rs` out of the registry source; `onboarding` needs a TTY.
- Coverage: RESEARCH Answer 6. Run `make services-up` then `make coverage`, which calls `scripts/coverage.sh`; compute the figure from `lcov.info` with the LF/LH python snippet; compare against the CI `coverage` job on the same commit via `gh run download <id> -n coverage-summary`. Port-mismatch trap: `make services-up` exposes 6379/9000, while `coverage.sh` probes 6380/9010 unless preset.
- web3sec: RESEARCH Answer 8. Quote the §6 contract; SHA must print `70657919beadbd341a72be2d0107080017a803f0`; compare against 2,683 / 0 / 2; never fabricate a return; end with a `checkpoint:decision`.
- Secrets never enter the evidence file (redacted). Quote verbatim error text in code blocks.

---

### Todo moves

**Analog:** `.planning/todos/completed/2026-10-26-record-v0-11-0-coverage-lines-figure.md` (the convention for a closed todo).
- `git mv .planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md .planning/todos/completed/` after the walk. Add the measured local figure, CI run id and commit (D-17).
- `2026-10-14-web3sec-single-bump-to-v0-11-0.md` moves only when DOGF-02 closes. If re-homed (D-16), update it in place with owner and date and leave it in `pending/`.
- Any S3/S4 `out (re-homed: <todo>)` disposition needs a new file in `.planning/todos/pending/`, copying the front-matter shape of the 2026-08-13 todo (`created`, `title`, `area`, `severity`, `files`, `owner`).

---

## Shared Patterns

### Offline, never-vacuous shell gates
**Source:** `scripts/check-version-strings.sh` and `tests/scripts/check-version-strings_test.sh`
**Apply to:** the checker and its test
Rules:
- No args and a test-only env seam.
- A named failure for "scanned zero".
- Sorted, deterministic output.
- A no-mutation checksum in the test.
- shellcheck-clean.
- Executable bit set.

### Production loader, never raw serde
**Source:** `src/config/settings.rs` `Settings::load_from_file`
**Apply to:** the round-trip test. There are only three production readers (RESEARCH Answer 1): `paladin-server`, the `paladin` binary, and `paladin-cli arsenal` (`Settings::new`, `APP_` overlay out of scope). `paladin-llm` has no production file loader.

### Secrets in test output
**Source:** `.github/instructions/security.instructions.md`
**Apply to:** the round-trip test and the evidence files
Never print `Settings`, `Debug`-format a config, or paste a key into an evidence file. Use obviously fake literals.

### Append-only log with last-bullet-wins dispositions
**Source:** `02-dogfood-findings-log.md` header ("Append; never rewrite an earlier entry")
**Apply to:** every log edit in the phase

### Gates before commit
**Source:** CLAUDE.md and D-00a
`make clean-code`, `make check-gates`, `make test-shell-guards`, `make api-surface`, `make security`, shellcheck on every script. Every fix commit cites its `DF-nn`.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Stage C fix files | varies | varies | Unknown until findings are logged (D-06/D-07). |
| `56-RUNBOOK-*.md` | runbook | n/a | No prior runbook in the phase tree. Use `54-FIRST-EVIDENCE.md` for layout only. |
| Key-path walker (`paths()`) | utility | transform | No existing Value-walker in the repo. Use the RESEARCH Pattern 2 snippet. |

## Metadata

**Analog search scope:** `scripts/`, `tests/scripts/`, `tests/` (top level), `tests/fixtures/config/`, `src/config/`, `Makefile`, `.planning/todos/`, `.planning/phases/54-first-run-usability/`
**Files read:** `check-version-strings.sh` (head and tail), `check-version-strings_test.sh` (head and tail), `Makefile` (gate targets), `tests/fixtures/config/README.md` (head), the findings log (head and index), `consumer_surface_muster.rs` (head), `settings.rs` and `web_server.rs` (targeted excerpts), plus CONTEXT and RESEARCH (first 417 of 677 lines; Pitfalls and later sections not read).
**Pattern extraction date:** 2026-11-02
