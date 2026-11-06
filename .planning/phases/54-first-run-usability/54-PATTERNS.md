# Phase 54: First-Run Usability - Pattern Map

**Mapped:** 2026-10-31
**Files analyzed:** 14 new/modified code-or-gate files (plus doc-only sweeps listed at the end)
**Analogs found:** 14 / 14

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `scripts/check-version-strings.sh` | gate script (bash + embedded python) | batch / transform | `scripts/check-crate-names.sh` | exact |
| `tests/scripts/check-version-strings_test.sh` | test harness | batch | `tests/scripts/check-dependency-allowlist_test.sh` | exact |
| `tests/scripts/fixtures/check-version-strings/*` | fixtures | file-I/O | `tests/scripts/fixtures/check-publish-order/` (`ls` only; the dependency-allowlist harness instead builds fixtures under `mktemp -d`) | role-match |
| `src/config/agents.rs` (`AuthConfig::validate` + tests) | config validator | request-response (fail-closed) | same file, Phase 53 reserved-literal rows | exact |
| `src/bin/paladin-server.rs` (boot test) | test | request-response | `build_auth_config_fails_closed_when_enabled_with_no_credentials` (line 799) | exact |
| shipped-example-validates test (location planner's choice; `agent_host.rs` already hosts the T-50-10-03 example-loading test) | test | transform | `agents.rs` tests plus the `agent_host.rs` example loader | role-match |
| `Makefile` (`check-version-strings`, `check-gates`, `docker-build-server`) | build config | batch | `check-crate-names` target (Makefile 192-194), `check-gates` (264) | exact |
| `.github/workflows/ci.yml` (one step, comment edits) | CI config | batch | `check-dependency-allowlist` step (~170-175) | exact |
| `.planning/decisions/0056-allowance-admission-model.md` | ADR amendment | doc | ADR-0015 / ADR-0059 line 9 | exact |
| `MIGRATION.md` §11.1 + §11.5 rows | doc | doc | M-B-27/M-B-28 and the Phase 53 §11.5 bullet | exact |
| `crates/paladin-memory/CHANGELOG.md`, `crates/paladin-content/CHANGELOG.md` | changelog | doc | `crates/paladin-web/CHANGELOG.md` `## [Unreleased]` | exact |
| root `CHANGELOG.md` `[Unreleased]` entry | changelog | doc | the `[Unreleased]` block at CHANGELOG.md line 8 | exact |
| `config.example.yml`, `k8s/server/*` | config / manifests | n/a | no code analog (see No Analog Found) | none |
| `docs/src/user-guides/fault-tolerance.md` and other docs | doc | n/a | none needed | n/a |

## Pattern Assignments

### `scripts/check-version-strings.sh` (gate, batch)

**Analog:** `scripts/check-crate-names.sh` (144 lines)

**Header and prelude** (lines 1-48): a block comment (purpose, offline rationale, `Usage:`, `Exit:`), then:
```bash
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
...
if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
fi

echo "🔍 Checking ... ."
```

**Embedded-python core** (lines 50-98). Read manifests with `tomllib`, never grep, and emit a status line followed by detail:
```bash
REPORT=$(python3 - "${WORKSPACE_ROOT}" ... <<'PY'
import glob, os, sys, tomllib
manifest_paths = [os.path.join(workspace_root, "Cargo.toml")]
manifest_paths += sorted(glob.glob(os.path.join(workspace_root, "crates", "*", "Cargo.toml")))
...
    with open(manifest_path, "rb") as fh:
        manifest = tomllib.load(fh)
    package = manifest.get("package", {})
...
if not tree_names:
    print("ZERO_CRATES")
    print("FAIL: zero publishable crates discovered ... a broken glob ..., not success.")
    sys.exit(0)
```
For D-06/D-07, the wanted version is the root `[package].version`, and the workspace package names come from the same loop. "Scanned zero files" must print a named status (the `ZERO_CRATES` shape) and fail.

**Report/exit tail** (lines 126-144):
```bash
STATUS_LINE=$(head -n1 <<<"${REPORT}")
DETAIL=$(tail -n +2 <<<"${REPORT}")
if [ "${STATUS_LINE}" = "OK" ]; then echo "✅ ${DETAIL}"; exit 0
else echo "❌ ... check failed"; echo "${DETAIL}"; ...; exit 1; fi
```
The python side prints `OK` or `MISMATCH` and always exits 0. Bash owns the exit code. D-11 asks for one `file:line: found 'X', want 'Y'` line per finding, all reported before exit, so the python `MISMATCH` branch must collect every finding and not stop at the first. Print a remediation hint block like lines 137-142.

Differences to build in: the script takes no argument (sibling convention). The per-file exemption list lives inside the script, one reason per entry (D-08). There is no `--write` mode (D-12).

---

### `tests/scripts/check-version-strings_test.sh` (test harness)

**Analog:** `tests/scripts/check-dependency-allowlist_test.sh` (230 lines). `make test-shell-guards` auto-discovers `tests/scripts/*_test.sh` by glob (Makefile ~272).

**Skeleton to copy** (lines 21-67): `set -uo pipefail` (no `-e`), `WORKSPACE_ROOT`, `GUARD`, a `mktemp -d` scratch with `trap cleanup EXIT`, `FAILED`/`ASSERTIONS` counters, and `cksum` before/after sums of the real inputs. `run_guard` points the script at fixtures through env vars:
```bash
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-dependency-allowlist-test.XXXXXX")"
cleanup() { rm -rf "${SCRATCH}"; }
trap cleanup EXIT
run_guard() {
    LAST_OUTPUT="$(CORE_MANIFEST="${core}" PORTS_MANIFEST="${ports}" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}
```
`assert_fire` (lines 71-92) requires a non-zero exit and every needle in the output. `assert_silent` (95-106) requires exit 0.

**Order** (lines 108-220): failing cases first ("prove it fails"), then passing cases, then the real tree, then the no-mutation checksum assertion. Include a determinism case (lines 176-185) and an ordering case (160-174).

Gap for the planner: the sibling script reads `CORE_MANIFEST`/`PORTS_MANIFEST` overrides, and `check-crate-names.sh` has no override at all. `check-version-strings.sh` must expose a test-only root override, for example `VERSION_STRINGS_ROOT`, so the harness can build a fake tree under `SCRATCH`. This is still compatible with "takes no argument". Harness cases required by D-08, D-11 and D-07:
- A stale pin in an exempt file passes, and the same content in a scanned file fails.
- A non-workspace crate pin (`paladin-custom-llm = "0.1"`) is not matched.
- An `alias = { package = "<ws>", version = "X" }` pin is matched.
- A third-party image tag and `paladin:test` are not matched.
- A zero-files-scanned tree fails with a named message.
- All findings are reported in one run.

---

### `src/config/agents.rs`: `AuthConfig::validate` (D-01, D-02)

**Analog:** same function, lines 188-245. Insert the new refusal inside the per-entry loop **after** the reserved-name check (line 213) and **before** `if entry.tenant.is_empty()` (line 215), so that the message can use the `[{name}]` form and identical placeholders report as a placeholder, not a duplicate.

Existing shape to copy (lines 203-213):
```rust
let name = entry.name.as_str();
// TENANT-09, D-13: exact, case-sensitive; the key value is never interpolated.
if RESERVED_PRINCIPAL_LITERALS.contains(&name)
    || RESERVED_KEY_NAME_LITERALS.contains(&name)
{
    return Err(format!(
        "http.auth.api_keys[#{index}]: 'name' is the reserved literal '{name}' -- \
         open-access, unattributed and anonymous name ..."
    ));
}
```
New row follows that form: `if entry.key.contains("${") { return Err(format!("http.auth.api_keys[{name}]: 'key' is an unexpanded '${{…}}' placeholder -- the server does not expand environment variables in http.auth.api_keys; set the literal key value")); }`

Pitfalls:
- `format!` needs `${{` to print a literal `${`, since braces are escaped.
- Never interpolate `entry.key` or the variable name.
- The empty-key check at line 195 runs first, so `key: ""` still reports as empty.
- `validate` runs regardless of `enabled`; `bearer_token` is untouched.

### `src/config/agents.rs` tests (D-03)

**Analog:** the table test at lines 839-880 (`validate_rejects_reserved_literals_in_every_position`) and the helpers `auth_with_keys`, `api_key(name, key, tenant)`, `validation_error(&cfg)`.
```rust
let mut cases: Vec<(AuthConfig, String)> = Vec::new();
cases.push((
    auth_with_keys(vec![api_key("ok", "sk-ok", "acme"), api_key(name, "sk-secret-value", "acme")]),
    "http.auth.api_keys[#1]: 'name'".to_string(),
));
for (cfg, prefix) in cases {
    let err = validation_error(&cfg);
    assert!(err.starts_with(&prefix), "expected a message starting {prefix:?}, got: {err}");
    assert!(err.contains("reserved literal"), "...: {err}");
    assert!(!err.contains("sk-secret-value") && !err.contains("sk-ok"), "never print the key value: {err}");
}
```
Rows to add: exact `${X}`, embedded `pre${X}post`, unterminated `${X`, `$X` accepted (`is_ok`), auth `enabled: false` still refused (model on `validate_checks_api_keys_even_when_auth_is_disabled`, line 818), refusal on the second entry (the message names `[<name>]`, not `#1`), and the neither-value-nor-variable-name assertion. Use a distinctive variable name such as `SK_PLACEHOLDER_VAR` and assert `!err.contains(...)`. A duplicate-placeholder row is worth adding: two identical `${X}` values must report "placeholder", not "duplicate 'key'". Model the duplicate message assertion on lines 717-737.

---

### `src/bin/paladin-server.rs` boot test (D-03)

**Analog:** lines 798-818:
```rust
#[test]
fn build_auth_config_fails_closed_when_enabled_with_no_credentials() {
    install_capturing_logger();
    let cfg = AuthConfig { enabled: true, api_keys: vec![], bearer_token: BearerTokenAuthConfig { enabled: false, tenant: None } };
    let result = build_auth_config(&cfg);
    assert!(result.is_err(), "authentication enabled with no API keys ... must refuse to start, ...");
}
```
The new test builds the same `AuthConfig` with one key whose value is `${X}`, with the `api_key("name")` helper used at line 824 (set `.key` and `.tenant`). It asserts that the boot path refuses before any listener binds. Planner: check whether `build_auth_config` itself calls `AuthConfig::validate`. If it does not, the test must go through the real boot step that does (read `build_auth_config` and `main`). Also assert that the error text contains the entry name and not the value.

---

### `Makefile`

**Analog:** target block at 192-194:
```make
.PHONY: check-crate-names
check-crate-names: ## Guard against crates.io package-name collisions (allow-list)
	@./scripts/check-crate-names.sh
```
Add `check-version-strings` the same way. Append it to the prerequisite list at line 264:
`check-gates: check-changelogs check-crate-names ... check-dependency-allowlist ## Run all offline release-gate guards`.

The comments at lines 239-243 say "the nine members of check-gates"; a count in a comment is exactly the staleness D-15 removes, so reword it to count-free wording when adding a tenth.

`docker-build-server` (lines 521-524) is currently:
```make
@$(DOCKER) build -f Dockerfile.server -t paladin-server:latest .
```
D-10 adds a second `-t paladin-server:<workspace version>`. Derive the version offline in the recipe, for example from the root `Cargo.toml`. Check how the Makefile already derives the version (grep `VERSION`) before inventing one.

### `.github/workflows/ci.yml`

**Analog:** lines 166-175. The `cargo-deny` job runs each gate as its own step; it does not call `make check-gates`:
```yaml
      # Phase 46 D-21 (CURR-25): ... CI runs each gate script
      # itself and does not call `make check-gates`, so the gate needs its own step.
      - name: Check core/ports dependency allowlist (ADR-0015)
        run: ./scripts/check-dependency-allowlist.sh
```
The same job's earlier steps run `chmod +x scripts/*.sh` (line ~136), so the new script is executable there, but commit it with mode 755 anyway. Add a "Check version strings" step beside these. The "all ten" comment (~318) becomes count-free (D-15). Run `actionlint` after the edit.

---

### ADR-0056 amendment (D-14)

**Analog:** ADR-0015 line 9 and ADR-0059 line 9. The dated note is a bold lead-in paragraph placed immediately after the header block, at line 9:
```markdown
**Amended 2026-10-24 (Phase 46, D-20/D-21):** re-measured from the manifests' ... changing either set needs a dated note here in the same commit.
```
Use `**Amended 2026-10-31 (Phase 54, FIRST-06):** ...`. State what changed and cite ADR-0057 (check-only boundary, race accepted, WINDOWS row 65).

ADR-0059's wording is the model for a supersession: "The earlier text of this ADR that records ... describes the Phase 44 state and is superseded on this point only."

ADR-0056 already carries an inline dated-note style at the Downstream Consumers bullet (~line 217): `*Dated note, 2026-10-06 (plan 42-01, ADR-0057):* superseded -- ...`. The stale text at ~70, ~213 ("Phase 42 closes the first") and ~217 should be corrected to cite ADR-0057, in the same commit as the top note. The PROMOTION line is unchanged.

### `MIGRATION.md` (D-00b, D-05)

**§11.1 row shape** (table `| ID | Change | Who is affected | Required user action |`, line 11-12). The newest row is `M-B-28`, so the new row is `M-B-29`. Copy the form of M-B-27:
`| M-B-27 | **<bold headline> (REQ-ID, Phase N plan NN, D-xx).** <what the behaviour was and now is, with exact messages> | <who> | <action> |`
Check the latest ID in the file before assigning (`grep -n '^| M-B-' MIGRATION.md | tail`).

**§11.5 shape** (line 39+, a bullet list, not a table):
```markdown
- **Reserved principal literals are refused at config load** (TENANT-09, Phase 53 plan 53-06, D-13). `http.auth.api_keys[].name` may not be ... A configuration that uses one now fails at load with a message naming the entry (...) and never printing the key value. Ac...
```
Closing line at ~43: "A later v0.11.1 change of this kind adds its entry here in the same commit." Add the FIRST-01 bullet and the k8s config-as-Secret bullet above that sentence. FIRST-01 is behavioral, so it gets no §11.2 row and the allowlist stays at zero. §9 and §10 are frozen.

### Crate changelogs (D-13)

**Analog:** `crates/paladin-web/CHANGELOG.md` lines 8-20:
```markdown
## [Unreleased]

### Added

- `JobStore::create_owned` ... (TENANT-07, Phase 53 plan 53-05): ...

### Security

- Scoped job polls (TENANT-07, ...; see `MIGRATION.md` section 11.1 `M-B-26`): ...
```
`paladin-memory` and `paladin-content` have no anchor today. Their layout is `# Changelog`, a blurb, then `## [0.11.0] - 2026-10-26` with the text `No functional change; version bumped in lockstep with the workspace.` (memory file, lines 8-10). Insert `## [Unreleased]` above `## [0.11.0]`, with a `### Changed` or `### Fixed` entry correcting that line. Cite `MIGRATION.md` §10.1 M-B-12 (`paladin.node` to `paladin.data`) and its upgrade advice. Leave the `[0.11.0]` text byte-identical. The root `CHANGELOG.md` `[Unreleased]` (line 8) uses `### Added` / `### Security` subsections in the same shape.

---

## Shared Patterns

### Never print a secret
**Source:** `src/config/agents.rs` lines 204 and 239-243 (`"(the value is never printed)"`), and `.github/instructions/security.instructions.md`.
**Apply to:** the new validate row, its tests, the boot test and every k8s or example comment. Assert `!err.contains(<value>)` in every refusal test.

### Offline, no-argument gates that report everything
**Source:** `scripts/check-crate-names.sh` (status-line protocol, named zero-scan failure, exit owned by bash).
**Apply to:** `check-version-strings.sh`.

### Prove-fail-first harness
**Source:** `tests/scripts/check-dependency-allowlist_test.sh` (fire cases, then silent cases, then the real tree, then the no-mutation checksum).
**Apply to:** `check-version-strings_test.sh`.

### Dated in-place amendment, no edit of frozen records
**Source:** ADR-0015/0059 line 9 (ADRs); crate `## [Unreleased]` (changelogs); MIGRATION §11.x (never §9/§10).
**Apply to:** every correction in this phase.

### Locate by content, not line number
Per D-00f the inventory line numbers have drifted. Use `grep` for the dangling `[Unreleased]:` link, the "all ten" comment, the `release.toml` counts and the `1.1.2` observation.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| `k8s/server/secret.yaml.example` (config as a `stringData` key), `deployment.yaml` and `worker-deployment.yaml` (mount via `items`) | manifests | `k8s/server/` currently mounts a ConfigMap named `paladin-server-config` (`deployment.yaml` lines 77 and 100-102, with `PALADIN_CONFIG=/etc/paladin/config.yml` at line 59). No existing manifest mounts config from a Secret. The planner should adapt the existing `config` volume to `secret: { secretName: paladin-server-secrets, items: [...] }` and delete `configmap.yaml`. Use RESEARCH.md for the layout. |
| `config.example.yml` `api_keys: []` plus commented entries | example config | Nothing to copy. D-04 gives the shape. Confirm it against the example-loading tests listed in CONTEXT (`agent_host.rs`, `web_server.rs`, `treasurer.rs`, `agent_runtime.rs`, `paladin-llm` `llm.rs`). |
| `docs/src` version sweeps and the fault-tolerance cache section | docs | Content edits driven by the gate's own output (D-09). |
| FIRST-08 snapshot check | n/a | Run `make api-surface`. The snapshot is never hand-edited. |

## Metadata

**Analog search scope:** `scripts/`, `tests/scripts/`, `src/config/agents.rs`, `src/bin/paladin-server.rs`, `Makefile`, `.github/workflows/ci.yml`, `.planning/decisions/`, `MIGRATION.md`, `crates/*/CHANGELOG.md`, `CHANGELOG.md`, `k8s/server/`
**Files read in full or in part:** about 12
**Not verified here (planner or executor to check):** whether `build_auth_config` calls `AuthConfig::validate`; how the Makefile derives the workspace version for D-10; the exact ADR-0056 line numbers for the stale reservation claim; the highest existing `M-B-` ID.
**Pattern extraction date:** 2026-10-31
