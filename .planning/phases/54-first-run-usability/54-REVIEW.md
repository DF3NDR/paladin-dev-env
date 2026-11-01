---
phase: 54-first-run-usability
reviewed: 2026-11-01T02:00:00Z
depth: standard
files_reviewed: 43
files_reviewed_list:
  - .github/workflows/ci.yml
  - .project/v0.11.1/01-carried-gaps-inventory.md
  - CHANGELOG.md
  - Dockerfile.server
  - MIGRATION.md
  - Makefile
  - README.md
  - config.example.yml
  - crates/paladin-content/CHANGELOG.md
  - crates/paladin-memory/CHANGELOG.md
  - docker/docker-compose.server.yml
  - docs/src/api-reference/crate-map.md
  - docs/src/api-reference/feature-flags.md
  - docs/src/appendix/sentinel.md
  - docs/src/contributing/architecture-decisions.md
  - docs/src/deployment-topologies/http-service-host.md
  - docs/src/deployment/docker.md
  - docs/src/deployment/kubernetes.md
  - docs/src/getting-started/configuration.md
  - docs/src/getting-started/installation.md
  - docs/src/getting-started/quickstart.md
  - docs/src/user-guides/agent-orchestrator-bridge.md
  - docs/src/user-guides/battalion-patterns.md
  - docs/src/user-guides/content-processing.md
  - docs/src/user-guides/fault-tolerance.md
  - docs/src/user-guides/maneuver-flow-dsl.md
  - docs/src/user-guides/orchestration.md
  - docs/src/user-guides/paladin-agents.md
  - docs/src/user-guides/reasoning-effort.md
  - docs/src/user-guides/superstep-engine.md
  - k8s/README.md
  - k8s/deployment.yaml
  - k8s/server/deployment.yaml
  - k8s/server/secret.yaml.example
  - k8s/server/worker-deployment.yaml
  - release.toml
  - scripts/check-release-consistency.sh
  - scripts/check-version-strings.sh
  - scripts/finalize-crate-changelogs.sh
  - scripts/publish-crates.sh
  - src/bin/paladin-server.rs
  - src/config/agents.rs
  - tests/scripts/check-version-strings_test.sh
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 54: Code Review Report

**Reviewed:** 2026-11-01T02:00:00Z
**Depth:** standard
**Files Reviewed:** 43 (plus the deleted `k8s/server/configmap.yaml`, read from the diff)
**Status:** issues_found

## Summary

The core of the phase holds up under test. The `${` refusal in `AuthConfig::validate` is correct, ordered before the tenant and duplicate checks, independent of `enabled`, and never prints the value or the variable name. I ran the 36 `config::agents` unit tests (all pass). I also ran the prebuilt `paladin-server` binary against the Secret's embedded `config.yml`:

- Unfilled, it refuses with `http.auth.api_keys[#0]: 'key' must not be empty`.
- With a `${PALADIN_API_KEY_CI}` key, it refuses with the placeholder message.
- With literal keys, it boots.

`scripts/check-version-strings.sh` passes on the real tree (128 files, all four classes non-zero). Its 37-assertion harness passes. The k8s Secret mount is sound: `items` projects only `config.yml`, `0440` plus `fsGroup: 65532` is readable, and `0440` is valid YAML-octal for `defaultMode`.

The defects are at the seams, not in the new validators:

- **The new gate is not wired into the release flow.** `make release` will produce a tagged commit that the gate then rejects (CR-01).
- **The "no example loads a placeholder as a credential" and "an unfilled template fails closed" claims are only partly true.**
  - The Secret template still ships `REPLACE_ME` as a live provider key (WR-02).
  - The same literal-placeholder trap remains open for the Treasurer webhook secret (WR-04).
  - Only one of the shipped example configs is pinned by a test (WR-04).
- **Copy-paste docs have wrong feature names** that the version gate cannot see (WR-01).
- **The compose first-run path has a bind-mount trap** (WR-03).

## Critical Issues

### CR-01: `make release` commits and tags a tree that the new version-string gate (and so the release gate) rejects

**File:** `Makefile:664-731` (`release` target, `git add -u` at line 723); `scripts/check-version-strings.sh`; `.github/workflows/ci.yml:181`
**Issue:** The new gate holds 128 files (README, `docs/src`, `k8s/`, `openapi.json`) to the root `Cargo.toml` version. `make release VERSION=x.y.z` bumps `Cargo.toml` and regenerates `openapi.json`, then runs `git add -u`, commits, tags and pushes. Nothing in that sequence rewrites the README `Current version`, the ~40 doc pins, or the `k8s/**` labels and image tags. Nothing runs `check-version-strings.sh` before the tag.

Consequences on the next bump (for example 0.11.1):
1. The release commit goes red on CI's new "Check user-facing version strings" step.
2. `release.yml`'s `check-release-consistency` job needs the tagged SHA's `ci.yml` run to be green (`CI_MISMATCH`). The publish job `needs: [test, create-release, check-release-consistency]` (`release.yml:605`), so the publish is blocked after an immutable tag is already pushed. That is exactly the situation `docs/src/appendix/release-recovery.md` exists to clean up.

The Makefile comment at line 525 says the version "moves with the version bump". That is only true of `Cargo.toml`, `docker-build-server` tags and `openapi.json`, not the manifests or docs the gate scans. No document (`release-automation.md`, `MIGRATION.md`, the script header) tells the releaser to hand-edit the strings first. The gate script also deliberately has no write mode (D-12), so there is no supported way to do it.

**Fix:** Add two steps to `release`, between the changelog finalisation (line 710) and `git add -u`.
1. A rewrite step, separate from the check-only gate: for example `scripts/bump-version-strings.sh --from <old> --to $(VERSION)`, which edits only the exact strings the gate matches.
2. A pre-tag check, mirroring how `check-release-consistency.sh` is run:
```make
	@echo "$(CYAN)Verifying user-facing version strings for v$(VERSION) before tagging...$(NC)"
	@./scripts/check-version-strings.sh
```
If a rewrite step is out of scope, the second step alone still turns a post-tag failure into a pre-tag one. Either way, document the manual step in `docs/src/appendix/release-automation.md`.

## Warnings

### WR-01: Copy-paste Cargo snippets name features that do not exist (the gate checks versions only)

**File:** `docs/src/api-reference/crate-map.md:201` and `docs/src/getting-started/installation.md:57`
**Issue:** The phase's stated goal (FIRST-03) is that a pasted snippet resolves to the crate the docs describe. Two snippets whose version lines were edited in this phase still fail `cargo` resolution:
- `crate-map.md:201` enables `"storage-sqlite"` on `paladin-ai`. The root `Cargo.toml` has no such feature. Lines 542-543 say SQLite repositories are always on, and only `storage-mysql`, `storage-postgres` and `storage` exist.
- `installation.md:57` writes `paladin-llm = { version = "0.11.0", features = ["llm-openai"] }`. `crates/paladin-llm/Cargo.toml` features are `openai`, `anthropic` and so on. `llm-openai` is the facade (`paladin-ai`) spelling. `quickstart.md:26` and `crate-map.md:230` use the correct `["openai"]`.

Cargo fails both with "does not have feature". The gate reports `OK` because it compares version tokens only.

**Fix:**
```toml
# crate-map.md: drop the line (SQLite repositories are always available)
paladin-ai = { version = "0.11.0", features = ["llm-anthropic", "llm-deepseek", "qdrant", "notifications"] }
# installation.md:57
paladin-llm = { version = "0.11.0", features = ["openai"] }
```
Consider extending the gate (or the doc-example crate) to verify that every `features = [...]` entry on a workspace pin names a real feature of that crate.

### WR-02: `k8s/server/secret.yaml.example` still loads a placeholder as a live credential, contradicting its own "fails closed" claim

**File:** `k8s/server/secret.yaml.example:31` (claim at line 15)
**Issue:** The header says "An unfilled template fails closed". That is true for the HTTP API keys (`key: ""`), but `openai-api-key: "REPLACE_ME"` is a non-empty placeholder that the server treats as a real provider key. I verified with the unfilled-provider variant of the embedded config and literal API keys:
- `OPENAI_API_KEY=REPLACE_ME` boots, logs `listening ... serving 1 agent(s)`, and passes the `/ready` probe. `REPLACE_ME` is then sent to the provider as the bearer.
- `OPENAI_API_KEY=""` is refused at boot with `llm.openai.api_key is the placeholder '${OPENAI_API_KEY}', but environment variable OPENAI_API_KEY is not set`.

So a half-filled Secret (API keys filled, provider key forgotten) reports healthy and fails only on the first agent call. This is the class of defect the phase set out to remove ("no example loads a placeholder as a credential").

**Fix:**
```yaml
stringData:
  openai-api-key: "" # paste the provider key; empty is refused at boot
```
Update the comment at line 15 to say both the API keys and the provider key are refused while blank.

### WR-03: The compose first-run path creates a directory named `config.yml` and then crash-loops

**File:** `docker/docker-compose.server.yml:28` (`- ../config.yml:/etc/paladin/config.yml:ro`, with `restart: unless-stopped`)
**Issue:** The file now bind-mounts `../config.yml` unconditionally. The short volume syntax creates a missing host source as a root-owned directory. A user who runs step 2 of the header recipe without step 1 (`cp config.example.yml config.yml`) gets a `config.yml/` directory in the repo root. The server then fails to read its config and `restart: unless-stopped` restarts it forever. The later `cp config.example.yml config.yml` then copies the file inside that directory rather than replacing it, so the problem persists. Before this phase the mount was commented out, so this trap is new.

**Fix:** Use the long form with `create_host_path: false`, so compose errors instead of creating the directory:
```yaml
    volumes:
      - type: bind
        source: ../config.yml
        target: /etc/paladin/config.yml
        read_only: true
        bind:
          create_host_path: false
```
Alternatively, change `restart` to `on-failure:3` so a refused boot is visible rather than looping.

### WR-04: The "no example loads a placeholder" guarantee is pinned for one file only; the Treasurer webhook secret has the same trap

**File:** `src/config/agents.rs:1080-1165` (tests); `src/config/treasurer.rs:1491-1503`; `k8s/server/secret.yaml.example:34-90`; `config.example.yml:318`
**Issue:**
1. The new tests read only `config.example.yml`. The k8s Secret's embedded `config.yml` (a second full shipped config, hand-copied), the `http-service-host.md` snippet, and the frozen `tests/fixtures/config/v0.9.0-config.example.yml` (still `key: "${PALADIN_API_KEY_CI}"`) are not covered. Nothing proves the Secret's embedded config still parses against the schema (I confirmed by hand that it currently does). Nothing stops a future edit from reintroducing `${...}` in `k8s/` or the docs.
2. `treasurer.allowance.webhook.secret` is the same trap and stays open. `yaml_env_placeholder_is_not_expanded` pins that a `${ALLOWANCE_WEBHOOK_SECRET}` value loads literally as the HMAC signing secret. A guessable signing secret lets anyone forge `X-Paladin-Signature`. The phase documents "set it via `APP_TREASURER_...`" but, unlike `api_keys`, does not refuse a `${`.

**Fix:**
- Add a test (or extend `check-version-strings.sh`'s sibling gates) that extracts the `config.yml` block from `k8s/server/secret.yaml.example`, loads it through `config::File`, and asserts both `AuthConfig::validate` and that no `key`/`secret` field contains `${`.
- Apply the same `contains("${")` refusal to the Treasurer webhook secret in its `validate`, with a name-only message, or record the deliberate deferral in the register.

### WR-05: `check-version-strings.sh` exits with an unreadable traceback on Python older than 3.11

**File:** `scripts/check-version-strings.sh:73-86`
**Issue:** The script checks only that `python3` exists, then runs `import tomllib` inside the heredoc. On Python 3.9 or 3.10 (default on some developer machines, and on older CI images) this raises `ModuleNotFoundError`. Under `set -e` the command substitution aborts the script with a raw traceback and no pointer to the cause. The exit is non-zero, so it fails closed, but the header and the Makefile guard (`WORKSPACE_VERSION` check) both promise a diagnostic. The Makefile `docker-build-server` guard does say "python3 with tomllib is required"; this script does not.

**Fix:**
```bash
if ! python3 -c 'import tomllib' >/dev/null 2>&1; then
    echo "ERROR: python3 >= 3.11 (tomllib) is required for Cargo.toml parsing." >&2
    exit 1
fi
```

## Info

### IN-01: MIGRATION operator checklist mixes "after" and "before" steps out of order

**File:** `MIGRATION.md:113-114`
**Issue:** Steps 1-3 are "After upgrading, look in the boot log...". Step 4 is "Before upgrading, search your config for `${`". Step 5 (Kubernetes) tells the operator to apply the Deployment before the image exists ("build or push `paladin-server:<workspace version>`" comes after the apply). An operator following the list in order will upgrade first and only then learn to pre-check, which is too late because the server refuses to boot.
**Fix:** Move the "before upgrading" step to the top of the list (renumber), and in step 5 put the image build or push before `kubectl apply`.

### IN-02: Worker pods mount the full config, including literal admin API keys they never use

**File:** `k8s/server/worker-deployment.yaml:145-154`
**Issue:** The worker Deployment mounts the same `paladin-server-secrets` `config.yml`, which now carries the literal `ci-runner` (admin) and `frontend` keys. `k8s/README.md` states workers never serve the auth-gated `/v1` routes. Before this phase the worker's copy held only placeholders. Each extra mounted copy of an admin credential widens the blast radius for no functional benefit.
**Fix:** Give workers a separate Secret key (for example `worker-config.yml`) with `http.auth.enabled: false` or an empty key set, as long as the worker path validates that way. Otherwise, note the exposure in `k8s/README.md`.

### IN-03: Gate parsers are brittle in a few edge shapes

**File:** `scripts/check-version-strings.sh:185` (`CURRENT_RE`), `:191-194` (`IMAGE_RE`), `:201` (`splitlines`)
**Issue:**
- `Current version: [0.11.0](url)` or `Current version: 0.11.0,` yields the token `[0.11.0](url)` or `0.11.0,`. That is a false positive.
- `**Current version**: 0.11.0` (colon outside the emphasis) is not matched, a false negative.
- A `paladin-server:0.11.0@sha256:...` image tag is captured with the digest and reported as a mismatch.
- `str.splitlines()` also splits on `\x0b`, `\x0c`, `\x1c-\x1e`, `\x85`, U+2028 and U+2029, so reported `file:line` numbers drift from editors and `grep -n` if a scanned file ever contains one. No current file does (checked).
**Fix:** Use `re.split(r'\r?\n', text)` for line numbering. Tighten `CURRENT_RE` to `v?(\d+\.\d+\.\d+[\w.\-+]*)` after emphasis stripping. Cut the image tag at `@`. None of these is exercised by the harness today.

### IN-04: Duplicate comment in `ci.yml`, and a placeholder literal `X.Y.Z` in copy-paste manifests

**File:** `.github/workflows/ci.yml:173-180`; `docs/src/deployment/kubernetes.md:206,518`
**Issue:** The new step's comment repeats the preceding step's "CI runs each gate script itself and does not call `make check-gates`" sentence verbatim, with the two blocks adjacent. Separately, the Docker and Kubernetes doc snippets now carry the literal `X.Y.Z` as an image tag, which is not a pullable tag. That is acceptable for an `your-org` placeholder, but the snippet at `kubernetes.md:206` is a full Deployment a reader may paste.
**Fix:** Keep the one-line comment on the new step only. Add a `# replace X.Y.Z with the release tag` comment next to the two literals.

---

_Reviewed: 2026-11-01T02:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
