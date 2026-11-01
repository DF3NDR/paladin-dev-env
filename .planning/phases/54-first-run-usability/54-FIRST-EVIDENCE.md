# Phase 54 First-Run Evidence (plan 54-09)

Recorded 2026-11-01 on branch `claude/laughing-dirac-e0h2ax`. Phase head when the rows below were
re-run: `027891467d03cae84529e6ec61ae4d0bdf23ea49` (the 54-08 completion commit; no code changed after
it, only this file and planning records). The workspace version is `0.11.0` (root `Cargo.toml`).

"Before" cells are copied from the plan SUMMARYs (54-01 .. 54-08), which measured them on each plan's
own start SHA before any edit (D-00f). "Head" cells were re-run for this file.

## Prove-then-lock (D-00f)

| Item | Command | Before (from the plan SUMMARY) | Head output | Disposition |
|------|---------|--------------------------------|-------------|-------------|
| FIRST-01 refusal exists | `grep -n 'unexpanded' src/config/agents.rs` | 54-01: printed nothing (exit 1) at `75f1606650de` | hits at l.91, 162, 218, 223 (rustdoc, validator branch, message) and the test section at l.980 | changed in 54-01 |
| FIRST-01 example carries no loadable key | `grep -nE '(^\s*\|[{,]\s*\|-\s+)key:\s*"?\$\{' config.example.yml` | 54-01: two hits, l.182 (`ci-runner`) and l.186 (`frontend`) | no output (exit 1) | changed in 54-01 |
| FIRST-01 retired ConfigMap and its claim | `test ! -e k8s/server/configmap.yaml`; `grep -rn 'expanded by the server' k8s config.example.yml` | 54-06: `configmap.yaml:4,8,36,40` carried the expansion claim, the ConfigMap name and two `key: "${...}"` entries | file absent; grep prints nothing (exit 1); `grep -rnE 'key:\s*"?\$\{' k8s` also exit 1 | changed in 54-06 (file retired, so its expansion-claiming header went with it) |
| FIRST-01 per-caller key env vars | `grep -rn 'PALADIN_API_KEY_' docker Dockerfile.server docs/src k8s` | 54-02: `docker-compose.server.yml:6,21,22`, `Dockerfile.server:9`, `http-service-host.md:137,146`; 54-06: `k8s/server/deployment.yaml:66-75` | no output (exit 1) | changed in 54-02 and 54-06 |
| FIRST-02 stale pins | `grep -nE '"0\.(5\|10\.0)"\|0\.8\.0\|v0\.10\.0'` over README, `docs/src/getting-started`, crate-map, feature-flags, sentinel, `docs/src/user-guides`, `architecture-decisions.md` | 54-03: README l.108 (`"0.5"`), l.183 (`0.8.0`), quickstart l.24-26, installation l.48-78, crate-map l.12 plus 8 pins, feature-flags 18 pins, sentinel l.66; 54-04: five pins in four user and contributor guides, five banners | three hits only: `**Since:** v0.10.0` on `eval-harness.md:3`, `graph-visualization.md:3`, `superstep-engine.md:3`. These are historical "introduced in" statements, kept by design (54-04); the gate's classes do not cover them | changed in 54-03 and 54-04 |
| FIRST-02 deployment-guide image tags | `grep -nE 'paladin:v?[0-9]\|tag: "v?[0-9]' docs/src/deployment/docker.md docs/src/deployment/kubernetes.md` | 54-04: `docker.md` l.91, 105, 515, 549-553, 566, 664 (`v0.8.0`, `v0.8`, `v0`); `kubernetes.md` l.206, l.518 | no output (exit 1) | changed in 54-04 (Task 3) |
| FIRST-02 README version line | `grep -n 'Current version' README.md` | 54-03: `Current version: 0.8.0` at l.183 | `185:Current version: **0.11.0**.` | changed in 54-03 |
| FIRST-03 gate exists and is a `check-gates` member | `make -n check-gates \| grep -c check-version-strings`; `grep -n 'FIRST-03' .github/workflows/ci.yml` | 54-07: `0` before the plan | `1`; `ci.yml:177` (comment) and `:181` (step `Check user-facing version strings (FIRST-03)`); the gate's own red and green runs are below | changed in 54-07 |
| FIRST-04 k8s labels and tags | `grep -nE 'version: v\|image: paladin' k8s/server/*.yaml k8s/deployment.yaml`; `make -n docker-build-server \| grep -o '\-t [^ ]*'` | 54-06: labels `v0.10.0` (worker), `v0.6.0` (server), `v0.1.0` (root); images `paladin-server:0.10.0`, `0.6.0`; one `-t paladin-server:latest` | six `version: v0.11.0` labels, `paladin-server:0.11.0` twice, `paladin:test` unchanged; `-t paladin-server:latest` and `-t paladin-server:0.11.0` | changed in 54-06 |
| FIRST-05 memory and content correction | `grep -n '^## \[' crates/paladin-memory/CHANGELOG.md crates/paladin-content/CHANGELOG.md` (first three each) | 54-05: no `## [Unreleased]` in either; `[0.11.0]` held only "No functional change" | each has `## [Unreleased]` (l.8), then `## [0.11.0] - 2026-10-26` (l.14); both carry a `Correction to the [0.11.0] entry:` bullet; `[0.11.0]` sections untouched | changed in 54-05 |
| FIRST-06 ADR-0056 | `grep -c 'ADR-0057' .planning/decisions/0056-allowance-admission-model.md`; `grep -n 'reserv' ...` | 54-05: stale "reservation" passages at l.57, 70, 120, 122, 217-219; `ADR-0057` not cited | `ADR-0057` appears 8 times; the remaining `reserv` hits (l.13, 14, 30, 38, 128, 131, 133, 233) are the amendment, the rejected-alternative text and "writes no reservation" statements | changed in 54-05 |
| FIRST-07 stale counts | `grep -nE 'ten per-crate\|list of ten\|nine members\|all ten\|eleven-file\|eleven files\|eleven-crate dependency' release.toml scripts/finalize-crate-changelogs.sh scripts/check-release-consistency.sh scripts/publish-crates.sh Makefile .github/workflows/ci.yml` | 54-07: Makefile l.243 ("nine members"), `ci.yml` l.325 ("all ten"); 54-08: `release.toml:41,47`, `finalize-crate-changelogs.sh:8`, `check-release-consistency.sh:595`, `publish-crates.sh:114` | no output (exit 1) | changed in 54-07 and 54-08 |
| FIRST-07 `cargo-release` version | `grep -n 'cargo-release 1' release.toml` | 54-08: l.49 said `1.1.2` | `49:# lockstep bump ran on \`cargo-release 1.1.7\`, installed with \`--locked\` in plan` | changed in 54-08 |
| FIRST-07 dangling footer link | `grep -n '^\[' CHANGELOG.md \| tail -3` | 54-08: `[Unreleased]: https://github.com/jamatulli/paladin/compare/v0.2.0...HEAD` at l.3413 | last definitions are `[0.2.0]` (l.3413) and `[0.1.0]` (l.3414); no `[Unreleased]:` definition | changed in 54-08 |
| FIRST-08 snapshot | `make api-surface` | RESEARCH measured `exit 0`, `API surface unchanged`, 4325 items | see `## API surface (FIRST-08)` (added with the reconciliation) | already closed (53-06); wording reconciled in 54-09 |
| FIRST-09 node-cache guide | `grep -n '^### Node cache on the shipped server\|^### Cache key version and rotation' docs/src/user-guides/fault-tolerance.md` | 54-05: only `key_prefix` hits at l.364 and l.376; `grep -rn 'NodeCacheConfig' src/bin src/infrastructure/web crates/paladin-web/src` empty | `381:### Node cache on the shipped server`, `394:### Cache key version and rotation` | changed in 54-05 |
| Scout fact: `treasurer/tests.rs` cites ADR-0057 | `grep -n 'ADR-0057' src/application/services/treasurer/tests.rs` | 54-05: hits at l.17 and l.19 before any edit | `17://! check is check-only too (ADR-0057 supersedes ADR-0056's earlier plan to reserve at the`, `19://! ADR-0056 (plan 41-09) and ADR-0057 record the race and \`.planning/WINDOWS.md\` row 65 holds` | already closed (file left untouched) |
| Scout fact: `current-exports.txt` already regenerated | `git log -1 --format='%h %cs' -- .project/current-exports.txt` | 53-06 regenerated it | `6b73e11b0 2026-10-31` (`feat(53-06): refuse to boot a configured Treasurer without the engine-factory path (TENANT-08)`) | already closed |

### Line drift recorded

The requirement text cites stale line numbers; the measured ones are:

- the dangling `[Unreleased]` compare link sat at `CHANGELOG.md:3413` when 54-08 started (the CONTEXT note
  said about 3408 and the requirement said 3396);
- the `ci.yml` count comment that named the crate manifests ("all ten") sat at l.325 when 54-07 started
  (not l.277 or l.318); the Makefile counterpart ("nine members") sat at l.243.

### Manual-only row: the k8s quick-start read (VALIDATION)

There is no cluster here (`kubectl`, `kind` and a Docker daemon are all absent), so the quick start was read
against the tree rather than applied:

| Check | Command | Output |
|-------|---------|--------|
| The image tag the manifests pull is the one the build makes | `make -n docker-build-server \| grep -o '\-t [^ ]*'` | `-t paladin-server:latest`, `-t paladin-server:0.11.0`; both Deployments read `image: paladin-server:0.11.0` |
| The Secret template carries `config.yml` | PyYAML load of `k8s/server/secret.yaml.example` | `stringData` keys are `openai-api-key` and `config.yml`; the latter parses and is run unfilled in the SC1 section below |
| Every path `k8s/README.md`'s server quick start passes to `kubectl apply -f` exists, apart from the operator-made copy | `test -f` on `k8s/namespace.yaml`, `k8s/server/deployment.yaml`, `k8s/server/service.yaml`, `k8s/server/worker-deployment.yaml`, `k8s/server/secret.yaml.example` | all five FOUND; `k8s/server/secret.yaml` is absent, as intended (it is the gitignored copy the operator makes with the `cp` on README l.20) |

What this does not show: the pod's ability to read the Secret volume file (see `## Known residuals`).

## Boot refusal (SC1)

Process-level proof on the real binary, built with `cargo build --features web-server --bin paladin-server`
(`Finished dev profile`). `paladin-server` reads one file (`PALADIN_CONFIG`) and runs the agent registry
build before `build_auth_config`, which runs before `TcpListener::bind`. Both configs live in a `mktemp -d`
directory under the session scratchpad and are not committed. Only the placeholder text and the obviously
dummy `sk-dummy-not-a-real-key` appear anywhere.

### (a) A key that is the placeholder for `PALADIN_API_KEY_CI`

Config: the required top-level fields, `agents: []`, `server.port: 18080`, and `http.auth` enabled with one
entry (`name: "ci-runner"`, `role: "admin"`, `tenant: "platform-ops"`) whose `key` is the `${...}` placeholder
text for `PALADIN_API_KEY_CI`.

Command: `PALADIN_CONFIG=<file> timeout 60 ./target/debug/paladin-server`

- Exit status: `1`
- The `failed to start` line, verbatim:

```
[2026-11-01T01:16:34Z ERROR paladin_server] paladin-server failed to start: http.auth.api_keys[ci-runner]: 'key' is an unexpanded '${…}' placeholder -- the server does not expand environment variables in http.auth.api_keys; set the literal key value
```

- `grep -c PALADIN_API_KEY_CI` on stderr: `0` (the variable name is not echoed). `grep -ci listening`: `0`.
- No listener bound: the run ended in about a second against a 60 s `timeout`, and the log stops at the
  circuit-breaker lines that precede the auth step.

### (b) The Secret template's own `config.yml`, unfilled

Config: `stringData['config.yml']` of `k8s/server/secret.yaml.example`, extracted with PyYAML, with only
`server.port` changed from 8080 to 18081. `OPENAI_API_KEY=sk-dummy-not-a-real-key` in the environment, so the
`llm.openai.api_key: "${OPENAI_API_KEY}"` line (the working exact-name mechanism) resolves and the run gets as
far as the HTTP auth step.

- Exit status: `1`
- The `failed to start` line, verbatim:

```
[2026-11-01T01:16:34Z ERROR paladin_server] paladin-server failed to start: http.auth.api_keys[#0]: 'key' must not be empty
```

- `grep -ci listening` on stderr: `0`.
- Before that line the log shows the agent execution service being built (`Creating PaladinExecutionService`,
  `Attaching execution middleware ... token_budget`), which confirms the failure is the auth step and not an
  earlier unrelated one (RESEARCH A2). Neither run needed an adjusted field, so there is no A2 deviation.

An unfilled template therefore fails closed instead of loading a placeholder, and a key copied from the old
v0.11.0 example stops boot instead of authenticating with its own placeholder text.

No string matching `sk-[A-Za-z0-9]{20,}` was written to the scratch files or to this file.

## Gate red run (SC3)

Real tree, not a fixture. The README install pin (`README.md:108`) was set to `"0.5"` with `sed -i`, the gate
run, the file restored with `git checkout -- README.md`, and the gate re-run.

Red, through the make target (`make check-version-strings`):

```
🔍 Checking copyable version strings against the workspace version ...
❌ Version-string check failed (MISMATCH)

README.md:108: found '0.5', want '0.11.0'
...
make: *** [Makefile:226: check-version-strings] Error 1
```

`make` reports its own status 2 for a failing recipe. The script itself, run directly in the same state
(`./scripts/check-version-strings.sh`), prints the same `README.md:108: found '0.5', want '0.11.0'` line and
exits `1`.

Restore: `git checkout -- README.md`; `git status --porcelain README.md` printed nothing.

Green, after the restore:

```
🔍 Checking copyable version strings against the workspace version ...
✅ OK: 128 file(s) scanned, every copyable version string equals '0.11.0' (class 1 Cargo pins: 42; class 2 Current version lines: 1; class 3 k8s labels and image tags: 8; class 4 openapi.json info.version: 1; exempt files: 1)
```

Script exit `0`; `make check-version-strings` exit `0`. `git status --porcelain README.md` was empty after both
runs, so nothing stale is committed. The orchestrator's own run after 54-07 produced the same two outputs.

## API surface (FIRST-08)

Re-run for this file on the phase head, toolchain `nightly-2026-09-20` (`rustup toolchain list` shows it;
`cargo +nightly-2026-09-20 public-api --version` prints `cargo-public-api 0.52.0`).

`make api-surface` (runs `scripts/check-api-surface.sh .project/current-exports.txt`), output with colour codes
removed:

```
Checking public API surface (toolchain: nightly-2026-09-20)...
🔍 Checking API surface for changes...
Extracting public API surface using cargo-public-api (toolchain: nightly-2026-09-20)...
✅ API surface extracted to /tmp/tmp.38fpSVSuhi (4325 items)
✅ API surface unchanged
exit=0
```

The committed snapshot is current, so no regeneration was needed and the PD-09-2 contingency did not fire. The
check only compares a fresh extraction against `.project/current-exports.txt`; it writes nothing, and only
`make api-surface-update` writes the file. `sha256sum` of `.project/current-exports.txt` and `src/lib.rs` taken
before the run and re-checked after it both read `OK`, and `git diff --stat 027891467 -- src/lib.rs
.project/current-exports.txt` is empty: the snapshot was not hand-edited and no `pub use` was added.

The requirement text asks for the snapshot to carry `UnsupportedParameter` and `check_reasoning` rows. It
cannot, for a structural reason:

| Command | Output |
|---------|--------|
| `grep -c 'UnsupportedParameter\|check_reasoning' .project/current-exports.txt` | `0` |
| `grep -n 'paladin_ports' src/lib.rs` | `175:// (e.g. \`paladin_ports::\`, \`paladin_memory::\`, \`paladin_battalion::\`, etc.).` (the facade states that other crates are reached by their full paths; it has no `pub use paladin_ports`) |
| `grep -n 'UnsupportedParameter\|fn check_reasoning' crates/paladin-ports/src/output/llm_port.rs` | `LlmError::UnsupportedParameter` at l.609 (the variant) and `fn check_reasoning(` at l.2587 (the `LlmPort` method); further hits are rustdoc and tests |

Both items live in `paladin-ports`. The snapshot is extracted from the `paladin-ai` facade only, and the facade
does not re-export that crate, so no regeneration on any toolchain can list them. This is the same tool-design
limitation WINDOWS row 59 waived for `RunReadScope` and `TenantId` in Phase 40 (the facade lists a re-exported
module as one `pub use` line and never its items). The `paladin-ports` surface is guarded by `make check-semver`
(`cargo-semver-checks` for every publishable crate against the published 0.11.0 baseline; it builds and needs
crates.io, so it is not run here). The wording in REQUIREMENTS.md FIRST-08, ROADMAP.md Phase 54 success
criterion 5 and the carried-gaps inventory row C10 was reconciled with a dated note citing row 59 and
`make check-semver`; each keeps its ID, checkbox and original text.

## WINDOWS row 62 closing evidence

Row 62 (phase 41, `config.example.yml`) said the config loader expands no `${VAR}` in
`http.auth.api_keys[].key` and that the shipped examples loaded the placeholder as the key. Closing tests,
re-run on the phase head before the row was marked fixed (T-54-27):

| Command | Result line |
|---------|-------------|
| `cargo test --lib config::agents::tests::validate_refuses_an_unexpanded_placeholder_key` | `test config::agents::tests::validate_refuses_an_unexpanded_placeholder_key ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1377 filtered out; finished in 0.00s` |
| `cargo test --lib config::agents::tests::shipped_example` | `shipped_example_http_auth_validates_and_configures_no_key ... ok`, `shipped_example_api_key_entries_uncommented_are_refused_as_empty ... ok`; `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1376 filtered out; finished in 0.00s` |
| `cargo test --features web-server --bin paladin-server build_auth_config_refuses_an_unexpanded_placeholder_key` | `test tests::build_auth_config_refuses_an_unexpanded_placeholder_key ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s` |

Together with the SC1 section (the real binary refusing both a placeholder key and the unfilled Secret template),
the `config.example.yml` change (`api_keys: []`), the k8s Secret move (`configmap.yaml` retired) and the
Docker and compose change (a mounted literal-key config), every clause of the row's closing condition is met. No
expansion path was added (D-00e). Ledger counters before this change: `open_count: 16`, `waived_count: 39`,
`fixed_count: 32`, `total_count: 87`.

Ledger counters after `node .claude/gsd-core/bin/gsd-tools.cjs windows fixed 62` and the hand-edit of the reason
cell (table row and JSON mirror, no `|` inside; the 53-07 precedent): `open_count: 15` (was 16, minus 1),
`waived_count: 39` (unchanged), `fixed_count: 33` (was 32, plus 1), `total_count: 87` (unchanged).
`gsd-tools windows status` loads the ledger with no count mismatch, and `git diff` of `.planning/WINDOWS.md`
touches only the frontmatter counters, `last_updated`, and row 62 (table row and its JSON entry). The row's
reason cell cites `validate_refuses_an_unexpanded_placeholder_key`,
`build_auth_config_refuses_an_unexpanded_placeholder_key`, the `shipped_example_*` tests, the
`config.example.yml` change, the k8s Secret move and the Docker and compose change.

## Carry-forward to Phase 58

Measured on the head with
`for f in crates/*/CHANGELOG.md; do grep -qE '^## \[(Unreleased|0\.11\.1)\]' "$f" || echo "$f"; done`:

```
crates/paladin-battalion/CHANGELOG.md
crates/paladin-eval/CHANGELOG.md
crates/paladin-herald/CHANGELOG.md
crates/paladin-llm/CHANGELOG.md
crates/paladin-notifications/CHANGELOG.md
crates/paladin-ports/CHANGELOG.md
crates/paladin-storage/CHANGELOG.md
```

Seven of the eleven `crates/*/CHANGELOG.md` files, the set RESEARCH F6 expected. The root `CHANGELOG.md`,
`paladin-memory` and `paladin-content` carry `## [Unreleased]` (the last two since 54-05); the other crates
not listed carry one too.

What Phase 58 inherits:

1. **Finalize rule 3.** `scripts/finalize-crate-changelogs.sh` leaves a file with no anchor and no section for
   the target version untouched and records it as a named failure (it never guesses an insertion point).
   Each of the seven needs a `## [Unreleased]` anchor or a `## [0.11.1]` section before the 0.11.1 finalize.
   `check-release-consistency.sh` independently requires a `## [X.Y.Z]` heading in all twelve publishable
   changelogs (the root plus eleven crates).
2. **The version gate goes red at the bump, on purpose.** `scripts/check-version-strings.sh` holds the
   copyable strings to the root manifest version, so once the lockstep bump moves `Cargo.toml` to `0.11.1`
   the gate lists every string still reading `0.11.0`: today 52 of them, namely 42 class-1 Cargo pins, 1
   `Current version:` line (README), 8 class-3 k8s labels and image tags (six `version: v0.11.0` labels and
   two `paladin-server:0.11.0` images), and 1 `info.version` in `crates/paladin-web/openapi.json`. The bump
   commit must carry those edits, so that the gate is green on the release commit.
3. **Unverified: a dirty tree under `cargo release version --execute`** (RESEARCH A3). Whether the tool
   refuses or accepts a working tree that already holds the string edits was not tested. If it refuses, the
   plan's fallback is to run the bump first and edit the strings after it, in the same commit.
4. `make check-semver` (the guard for the `paladin-ports` surface named in the API-surface section) builds
   and needs crates.io, so Phase 58's pre-tag audit is where it is run.

STATE.md carries one blocker line pointing at this section.
