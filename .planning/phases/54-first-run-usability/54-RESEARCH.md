# Phase 54: First-Run Usability - Research

**Researched:** 2026-10-31
**Domain:** Config-load validation (Rust), offline shell release gates (bash + embedded python3), k8s/Docker/docs currency, changelog/ADR hygiene
**Confidence:** HIGH (every finding is from reading or running the tree at `4400fb3a2`; the few `[ASSUMED]` items are listed in the Assumptions Log)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (locked by the milestone open, Phases 52-53 and the roadmap — not re-asked)

- **D-00a:** Gates as Phase 53 D-00a: 82 % workspace line coverage (ADR-0006), `make clean-code`,
  `make check-gates`, `make test-shell-guards`, `make api-surface` (+ `make api-surface-update`
  and a CHANGELOG entry for an intentional public-surface change), `make security`,
  `make check-migration-allowlist`, `make check-doc-config` for any fenced YAML touched in
  `docs/src`, `shellcheck`/`actionlint` for every script and workflow edit, and the manual
  credential-handling review in `.github/instructions/security.instructions.md`. No
  `unwrap`/`expect`/`panic!` in library code. TDD: every new behaviour below is watched failing
  first.
- **D-00b:** Semver as Phase 52 D-01/D-03: `--release-type patch` against `0.11.0` for all twelve
  packages. FIRST-01 is a **behavioral** change (a config that loaded now refuses), not a Rust API
  break: it gets `MIGRATION.md` §11.1 and §11.5 entries and a `CHANGELOG.md` `[Unreleased]` entry,
  never a §11.2 row. The allowlist stays at zero entries; the milestone stays `v0.11.1`.
- **D-00c:** Frozen records: `MIGRATION.md` §9/§10, the root `CHANGELOG.md` `[0.11.0]` section and
  every crate changelog's published `[0.11.0]` section are byte-frozen. Corrections go in new
  `[Unreleased]` entries, which `finalize-crate-changelogs.sh` and `make release` stamp as
  `[0.11.1]` in Phase 58. Logging stays on the `log` facade.
- **D-00d:** ADRs: no new ADR is expected. ADR-0056 is amended in place with a dated note, the
  house pattern used for ADR-0015 and ADR-0059. If a planner finds one is needed, it takes the next
  free number from `.planning/decisions/PROMOTION.md` (currently **0064**) and advances the line in
  the same commit.
- **D-00e:** Milestone decisions of 2026-10-27, as the roadmap scope notes restate them:
  - `${VAR}` in HTTP auth keys is refused at boot. There is no expansion path and no new
    secret-handling path, including a `key_env` field.
  - The LLM `llm.<provider>.api_key: "${NAME}"` indirection (`paladin-llm`
    `resolve_secret_placeholder`, exact form only) is a separate, working mechanism and is not
    touched.
  - B10: "the server attaches no node cache" is documented, not wired.
  - B12: the cache-key version is kept and documented.
  - Version strings are held to the exact workspace manifest version, so they read `0.11.0` now
    and move with Phase 58's bump.
- **D-00f:** **Prove-then-lock** (the Phase 53 D-00d pattern). Every enumerated item is re-checked
  against the tree before it is changed; only what is still stale is changed, and an item that is
  already closed is recorded in the phase evidence file with the command that proves it. Facts
  from this session's scout (2026-10-31) that the plans must re-confirm:
  - The `src/application/services/treasurer/tests.rs` module doc already cites ADR-0057
    (check-only). ADR-0056's lines on the reservation are still stale.
  - `.project/current-exports.txt` was regenerated in plan 53-06 (`6b73e11b0`, 2026-10-31) and
    holds no literal `UnsupportedParameter` or `check_reasoning` line.
  - Line numbers in the inventory have drifted. The dangling link is now at `CHANGELOG.md:~3408`,
    not 3396, and the `ci.yml` "all ten" comment is at `~318`, not 277. Locate items by content,
    never by line number.

#### Placeholder refusal and honest examples (FIRST-01; B1, C3, DF-01, WINDOWS row 62)

- **D-01:** **The refusal rule: any key value containing `${` is refused.** This is broader than
  the exact `${NAME}` grammar the LLM resolver recognises. A partly templated value such as
  `pre-${VAR}` is just as unexpanded, a generated secret never contains `${`, and failing closed is
  the point. A bare `$VAR` with no braces is accepted, because `$` can occur in a real key.
  `[auto] Detection rule — Q: "Exact ${NAME} only, or any '${'?" → "Any value containing '${'" (recommended)`
- **D-02:** **Where the check runs and what its message says.**
  - It lives in `AuthConfig::validate` (`src/config/agents.rs`), for each `api_keys` entry. It runs
    after `name` is validated, so the message can use the `http.auth.api_keys[<name>]` form, and
    before the tenant and duplicate checks, so two identical placeholders report as a placeholder,
    not as a duplicate.
  - The message names the entry and says the server does not expand environment variables in
    `http.auth.api_keys`, so the literal key must be set. It **never prints the value or the
    variable name**: the house rule is never to print a key value.
  - The check also applies when `enabled: false`, following the existing rule that a disabled
    section's keys take effect the moment auth is re-enabled.
  - `bearer_token` has no secret field and is not touched.
  `[auto] Message — Q: "Echo the variable name to help the operator?" → "No — name the entry only" (recommended)`
- **D-03:** **Tests, each watched failing first.**
  - Table rows in `agents.rs` tests: an exact `${X}`, an embedded `pre${X}post`, an unterminated
    `${X`, `$X` accepted, refusal with auth disabled, refusal on the second entry, and a check that
    the message contains neither the value nor the variable name.
  - A `paladin-server.rs` boot test (the shape of the existing "auth enabled with no API keys …
    must refuse" test near line 815) proving a placeholder key refuses before any listener binds.
  - A test that the shipped `config.example.yml` `http` section deserializes and passes
    `AuthConfig::validate`, so the example cannot fail its own new rule.
  - The existing `treasurer.rs::yaml_env_placeholder_is_not_expanded` pin (on the webhook
    `secret`) stays as it is.
- **D-04:** **`config.example.yml`: commented-out entries with an empty key.**
  - The `api_keys` block becomes `api_keys: []`, with the two example entries kept below it as
    comments. Each commented entry shows `key: ""` and a note: paste a literal secret, for example
    from `openssl rand -hex 32`; `${VAR}` is not expanded here. An entry uncommented without being
    edited is refused as empty, so no shape of the example ever loads a placeholder.
  - `auth.enabled` stays `true`, the secure default. A verbatim copy fails closed with the existing
    named "authentication is enabled but no credentials are configured" error until a key is
    pasted, or until auth is disabled for development as already documented.
  - The comment states exactly which fields resolve `"${NAME}"`: the `llm.<provider>.api_key`
    blocks. It also states which do not: `http.auth.api_keys` and the treasurer webhook `secret`,
    which uses `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`.
  `[auto] Example shape — Q: "Commented entries, empty-key entries, or a dummy literal?" → "Commented entries showing key: \"\"" (recommended)`
  — **Reversibility:** reversible.
- **D-05:** **`k8s/server`: deliver the server config as a Secret.** The server reads exactly one
  file (`Settings::load_from_file`, no env layer), and a config that holds literal API keys is a
  secret.
  - `config.yml` moves into `k8s/server/secret.yaml.example`, as a `stringData` key of the existing
    `paladin-server-secrets` Secret. Its `api_keys` entries carry `key: ""`, so an unfilled template
    is refused at boot like `REPLACE_ME`, but never loads.
  - `deployment.yaml` and `worker-deployment.yaml` mount `/etc/paladin/config.yml` from that Secret
    (using `items`), and the ConfigMap `configmap.yaml` is retired.
  - The dead `PALADIN_API_KEY_CI`/`PALADIN_API_KEY_APP` env injections and the
    `api-key-ci`/`api-key-app` Secret keys are removed.
  - Only working mechanisms remain: the LLM key through `OPENAI_API_KEY` from the Secret (the
    researcher confirms which field `paladin-server` actually reads, `llm.openai.api_key: "${NAME}"`
    or the adapter's env fallback; the legacy top-level `llm_api_key: "${…}"` line goes if nothing
    reads it), and the `APP_RUN_*_URL_ENV` indirection, which is unchanged.
  - `k8s/README.md` and every `docs/src` page that names `configmap.yaml` follow. The root
    `k8s/configmap.yaml` and `k8s/deployment.yaml`, which the CI e2e kind cluster applies, are not
    restructured.
  - A `CHANGELOG.md` `[Unreleased]` entry and a `MIGRATION.md` §11.5 note tell anyone who deployed
    the old manifests that their `${…}` keys now refuse to boot.
  `[auto] k8s delivery — Q: "Keep ConfigMap with keys commented (fails closed), initContainer envsubst, or config-as-Secret?" → "Config as a Secret" (recommended)`
  — **Reversibility:** reversible: manifests only, but operators re-apply, hence the MIGRATION
  note.

#### Version-string gate and the currency sweep (FIRST-02, FIRST-03, FIRST-04; C1, C2, C4, DF-02)

- **D-06:** **One source of truth, exact equality.** The wanted version is the root `[package]
  version` in `Cargo.toml` (`paladin-ai`, `0.11.0`), read offline with `tomllib`, the
  `check-crate-names.sh` pattern. Every matched string must equal it exactly, with an optional
  leading `v` for k8s labels. There is no `0.11` short form; pins are written in full as
  `"0.11.0"`. Lockstep agreement between manifests stays the job of
  `check-release-consistency.sh`.
- **D-07:** **The gate matches four copyable pattern classes, not every version-looking string.**
  The scout found 220 version mentions across 44 `docs/src` files, mostly history. The classes:
  1. Cargo dependency pins whose crate name, or `package =` value, is a workspace package name
     taken from the manifests. Every form counts: `name = "X"`, `name = { version = "X", … }` and
     `alias = { package = "<ws>", version = "X" }`, including inside code fences and `//!` lines.
     A pin of a non-workspace crate (`paladin-custom-llm = "0.1"`) is never matched.
  2. A `Current version: X` line.
  3. In `k8s/`, the `version: vX` labels and `image:` tags of first-party images (`paladin-server`,
     `paladin`). Third-party images and non-version tags such as `paladin:test` are not matched.
  4. `info.version` in `crates/paladin-web/openapi.json`.

  Files scanned: the root `README.md`, `crates/*/README.md` (published on crates.io), everything
  under `docs/src`, everything under `k8s/`, and that `openapi.json`.
  `[auto] Gate scope — Q: "Pattern classes or every X.Y.Z string?" → "Copyable pattern classes" (recommended)`
- **D-08:** **Exemptions are a committed per-file list inside the script, one reason per entry.**
  The list starts with `docs/src/api-reference/migration-guide.md` (historical 0.4 rows, exempt
  per FIRST-02) and `docs/src/contributing/architecture-decisions.md` (an ADR example recording a
  0.5-era decision). The researcher confirms each candidate by reading it, and nothing else is
  exempted without a reason. The regression test proves both sides: a stale pin in an exempt file
  passes, and the same content in a scanned file fails. Inline exemption markers are not used.
  `[auto] Exemptions — Q: "Per-file list or inline markers?" → "Per-file list with reasons" (recommended)`
- **D-09:** **The sweep is defined by the gate, not by the inventory's line list.**
  - FIRST-02's enumerated lines are the minimum. Every hit the gate reports is fixed. Known hits
    beyond the list: `getting-started/quickstart.md`, `installation.md`,
    `user-guides/paladin-agents.md`, `battalion-patterns.md` and `maneuver-flow-dsl.md`; the
    remaining `crate-map.md` and `feature-flags.md` pins; the `k8s/server` labels and image tags;
    and `k8s/deployment.yaml`'s `v0.1.0` label.
  - The swept tree and the gate land so that `make check-gates` is never red on the branch: sweep
    first, or in the same commit.
- **D-10:** **The documented image build produces the manifest's tag.** `make docker-build-server`
  tags `paladin-server:latest` today, while the manifests pull `:0.10.0` or `:0.6.0`, so a copied
  manifest can never find the image. The target additionally tags `paladin-server:<workspace
  version>`, keeping `:latest`, and the `k8s/server` manifests reference the version tag, which
  the gate then holds.
  `[auto] Image tags — Q: "Version-tag in manifests + build target, or :latest?" → "Version tag, build target tags it" (recommended)`
  — **Reversibility:** reversible.
- **D-11:** **Gate shape.**
  - `scripts/check-version-strings.sh` is offline, takes no argument and runs in about a second,
    in bash with embedded `python3` like its siblings. It reports every finding before exiting
    (`file:line: found 'X', want 'Y'`), and scanning zero files is a named failure, never a pass.
  - A `make check-version-strings` target is added to the `check-gates` prerequisite list, and a
    CI step calls the script in the same job as the other `check-gates` scripts (`ci.yml` ~139-184).
  - `tests/scripts/check-version-strings_test.sh`, with fixtures, is auto-discovered by
    `make test-shell-guards`, which already runs in CI. The script is `shellcheck`-clean.
  - SC3's red proof is the harness's stale-fixture case, plus one local run against the real tree
    with a deliberately stale string, captured in the phase evidence file. No scratch PR is needed:
    the harness itself runs in CI.
- **D-12:** **The gate only checks; it never rewrites.**
  - There is no `--write` mode, and `make release` is unchanged. After a lockstep bump the gate
    lists every string that must move, and Phase 58's bump PR edits them.
  - A `make release` run that skipped the edit pushes a head on which CI goes red, and
    `release.yml`'s CI-conclusion clause (check-release-consistency clause 3) then blocks publishing
    from that tag.
  - Templating the labels (Kustomize or Helm, C4's "consider") is rejected in favour of literal
    strings held by the gate.
  `[auto] Bump path — Q: "Check-only, or add a rewrite mode now?" → "Check-only; rewrite deferred" (recommended)`

#### Changelog, ADR and count corrections (FIRST-05, FIRST-06, FIRST-07; C5-C8)

- **D-13:** **The crate changelog correction goes in a new `[Unreleased]` entry.** Neither
  `crates/paladin-memory/CHANGELOG.md` nor `crates/paladin-content/CHANGELOG.md` has a
  `## [Unreleased]` anchor today. Each gains one, carrying an entry that corrects the `[0.11.0]`
  "No functional change" line: `0.11.0` changed the persisted entity key `paladin.node` →
  `paladin.data` one way (cite `MIGRATION.md` §10.1 M-B-12, and reuse its upgrade advice: back up,
  upgrade readers before writers). The `[0.11.0]` text stays untouched, and
  `finalize-crate-changelogs.sh` stamps the anchor's content as `[0.11.1]` in Phase 58.
- **D-14:** **ADR-0056 is amended in place.**
  - The sentences that say Phase 42 closes the over-admission race with a reservation (the
    Downstream Consumers bullet near line 217, and the lines near 70 and 213) are corrected to
    cite ADR-0057: the boundary is check-only, the race is accepted, and WINDOWS row 65 holds it.
  - A dated "Amended 2026-10-31 (Phase 54, FIRST-06)" note names what changed. The PROMOTION line
    is unchanged.
  - The `treasurer/tests.rs` doc comment is proved already correct (D-00f) and left alone unless a
    stale sentence remains.
- **D-15:** **Crate counts in comments become count-free wording.** At `release.toml` ~41/47,
  `scripts/finalize-crate-changelogs.sh` ~8 and `ci.yml` "all ten" ~318, the wording becomes
  "every per-crate `CHANGELOG.md`" or "every publishable package". The scripts enumerate through
  `cargo metadata`, so a number in a comment can only go stale. Where a number is genuinely
  load-bearing, the measured count is written (eleven crate changelogs, twelve publishable
  packages).
  `[auto] Counts — Q: "Correct the number or drop it?" → "Count-free wording" (recommended)`
- **D-16:** **`release.toml` ~49 records the `cargo-release` used for the v0.11.0 bump.** That is
  `1.1.7`, installed with `--locked` in plan 47-01 and cited from `47-CI-EVIDENCE.md` rows 13-14
  (dated), replacing the Phase 20 `1.1.2` observation. Nothing is re-measured here.
- **D-17:** **The dangling footer link is deleted, not retargeted.** `[Unreleased]:
  https://github.com/jamatulli/paladin/compare/v0.2.0...HEAD` is removed. The historical `[0.2.0]`
  and `[0.1.0]` definitions stay, and no compare links are added for other versions: none of 0.3
  to 0.11 has one, and a per-release link is exactly the drift this phase removes. The researcher
  checks that `make release`'s perl substitution and `extract-changelog-section.sh` do not read
  the definition.
  `[auto] Footer link — Q: "Delete or retarget to DF3NDR compare v0.11.0...HEAD?" → "Delete" (recommended)`

#### API surface snapshot and the node-cache guide (FIRST-08, FIRST-09; C10, B10, B12)

- **D-18:** **The snapshot is checked first and regenerated only if it has drifted.**
  - Run `make api-surface` on the pinned `nightly-2026-09-20` (installed in this container). If it
    passes, FIRST-08 is met by evidence. If it reports drift, run `make api-surface-update` and
    commit. That is the committed copy catching up, not a surface change, and the CHANGELOG entry
    says so only if the diff adds items.
  - The `UnsupportedParameter` / `check_reasoning` clause is checked with `grep` against the
    resulting file. The facade reaches `paladin_ports` items partly through glob re-exports
    (`<<…::*>>`), so if those names cannot appear by name, the evidence says so and the
    requirement and inventory wording are reconciled.
  - The snapshot is **never hand-edited**. Later phases that move the surface refresh it again,
    per the roadmap note.
- **D-19:** **The node-cache guide is the cache section of
  `docs/src/user-guides/fault-tolerance.md` (Aegis), and it gains three things:**
  1. A plain statement that `paladin-server` attaches no node cache. `NodeCacheConfig` and
     `APP_NODE_CACHE_*` take effect only in a library composition root that builds a backend: the
     scout found no server or wiring file that reads `NodeCacheConfig`. The PACE-04 stampede lock
     attaches only through the library builder `WarEngine::with_cadence`.
  2. The key format and its version, `NODE_CACHE_KEY_VERSION = "k1"`
     (`crates/paladin-battalion/src/engine/cache_key.rs`): what the key covers, and that it stays
     `k1` under B12.
  3. The `key_prefix` rotation, or clearing the cache, after a change to a provider-level
     `llm.<provider>.reasoning_effort`, because the provider default is not in the key.

  `reasoning-effort.md` and `getting-started/configuration.md` link to this section instead of
  restating it. There is no code change.

### Claude's Discretion

- Exact error wording (D-02), within the house message shape and the never-print rule.
- The script's internals: the python/bash split, regexes, report format details and the exemption
  list's layout (D-07, D-08, D-11).
- The Secret's exact layout: the same `paladin-server-secrets` Secret or a second one, `items`
  versus `subPath`, and whether `configmap.yaml` is deleted or left as a pointer comment file
  (D-05). Deleting it is the default.
- The evidence file name. One `54-FIRST-EVIDENCE.md` holding the prove-then-lock results, the red
  run and the API-surface result is acceptable.
- Plan and commit grouping, and whether WINDOWS row 62 is closed in this phase's final plan or
  handed to Phase 57 with an evidence pointer. The row text must cite the D-03 boot test and the
  example changes.

### Deferred Ideas (OUT OF SCOPE)

- **A rewrite mode for the version gate** (`--write`, or a `make bump-version-strings` target
  that Phase 58 or `make release` calls): D-12 keeps this phase check-only.
- **Refuse `${…}` in other credential fields that are not expanded** (the treasurer allowance
  webhook `secret`, and any other secret-shaped config field): the same trap class, but new
  behaviour beyond FIRST-01. A candidate for Phase 55 or a dogfood finding.
- **`${VAR}` expansion or a `key_env` indirection for HTTP auth keys**: out by the 2026-10-27
  milestone decision; revisit only if dogfooding asks.
- **A boot `warn!` when `node_cache.enabled: true` on `paladin-server`**, which attaches no cache:
  the PACE-07 horizon ("node cache wired into the server").
- **Templated k8s labels and tags** (Kustomize or Helm): rejected for now in favour of literal
  strings plus the gate. Revisit if the manifests multiply.
- **Archival `.project/Milestone_*` `Node<T>` wording (C11)**: out per REQUIREMENTS.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FIRST-01 | `http.auth.api_keys[].key` with an unexpanded `${…}` is refused at config load naming the key; `config.example.yml` and `k8s/server/configmap.yaml` stop implying expansion | Insertion point and order in `AuthConfig::validate` (Pitfall 1, Code Example 1); test locations and shapes (D-03 answer); the Docker first-run path that FIRST-01 also breaks (Finding F1); the real LLM-key mechanism (D-05 answer) |
| FIRST-02 | Every install snippet / version line a user copies reads the current release | Exact gate hit list: 49 non-exempt hits in 12 files plus 1 judgement call (Version Sweep Inventory); exemption recommendation per file |
| FIRST-03 | `check-version-strings` guard in `make check-gates` and CI | Gate design, regexes, seam, harness conventions, CI job and Makefile wiring (Architecture Patterns 2-3) |
| FIRST-04 | `k8s/server` `version:` labels read the current release and are covered by the guard | Class 3 regex (anchored, not `apiVersion`), image tag and `imagePullPolicy` analysis (D-10 answer) |
| FIRST-05 | memory/content `[0.11.1]` entries correct the "No functional change" wording | How the 0.11.0 finalize treated anchors; what the two crates actually changed (Finding F6); Phase 58 anchor debt for seven other crates |
| FIRST-06 | tests.rs doc comment and ADR-0056 cite ADR-0057 (check-only) | tests.rs already correct; the stale ADR-0056 spots by content (Finding F5) |
| FIRST-07 | Stale counts/versions corrected | Every count comment found, which are stale, which are not; cargo-release 1.1.7 confirmed; the footer link is read by nothing (Finding F7) |
| FIRST-08 | `.project/current-exports.txt` regenerated on the pinned nightly, carrying `UnsupportedParameter`/`check_reasoning` rows | `make api-surface` was run: unchanged (4325 items). The by-name clause cannot hold: the facade snapshot does not re-export `paladin_ports` (Finding F4) |
| FIRST-09 | Node-cache guide: server attaches no cache, key version, `key_prefix` rotation | Current text of the three pages; what is missing where (Finding F8) |
</phase_requirements>

## Summary

Phase 54 is a documentation-and-guard phase with one small behavioural change. The behavioural change (FIRST-01) is a ~10-line addition to `AuthConfig::validate` plus tests. The rest is (a) removing every loadable placeholder credential from copyable material, (b) a new offline gate `check-version-strings` plus a one-time sweep of 49 stale strings, and (c) six independent corrections of stale records. All tooling the phase needs is already in the repo and already installed in this container (python 3.13 with `tomllib` and PyYAML, `shellcheck` 0.9.0, `cargo-release` 1.1.7, `cargo-public-api` 0.52.0, `nightly-2026-09-20`). No external package is installed, so there is no package-legitimacy exposure.

Research found **three places where CONTEXT.md's locked decisions are incomplete or point at something that does not exist as described**. The planner must handle them: (F1) the Docker first-run path (`Dockerfile.server` bakes `config.example.yml` as the runtime config; `docker/docker-compose.server.yml`, the `Dockerfile.server` header and `http-service-host.md` all tell users to pass `PALADIN_API_KEY_CI` as an env var) stops booting once FIRST-01 and D-04 land, and nothing in CONTEXT covers it; (F4) the FIRST-08 / SC5 clause that `UnsupportedParameter` and `check_reasoning` rows appear in `.project/current-exports.txt` cannot be satisfied by any regeneration, because the facade snapshot does not re-export `paladin_ports` (the snapshot is already current: `make api-surface` exits 0, "API surface unchanged", 4325 items); (F2) the legacy top-level `llm_api_key` cannot be deleted from the k8s config, because it is a required (non-`Option`, no serde default) field of `Settings`; it must stay as `""`. Smaller deviations: `architecture-decisions.md:645` is a copyable adapter-author `Cargo.toml`, not a historical record, so it should be fixed rather than exempted; the k8s README quick start applies the whole `k8s/server/` directory, which also applies the worker Deployment that needs a second Secret.

**Primary recommendation:** Land FIRST-01 (code + tests) first; do the sweep and the honest-examples edits (config.example.yml, k8s Secret move, Docker/docs snippets) before the gate lands, then add the gate, harness, Makefile and CI step in one plan that depends on the sweep plans; keep all edits of `CHANGELOG.md`, `MIGRATION.md`, `ci.yml`, `Makefile` and `k8s/server/*` serialised across plans; close with an evidence file that records the prove-then-lock results, including the F4 reconciliation.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Refuse `${…}` API keys at config load | Config / application layer (`src/config/agents.rs`, facade crate) | Server composition root (`paladin-server.rs::build_auth_config` calls `validate`) | `AuthConfig::validate` is the single validation owner; the bin only propagates the error before binding a listener |
| Honest example config | Documentation / shipped artefacts (`config.example.yml`) | Config tests (`agent_host.rs`, `agents.rs`) | The example must not fail its own validator, so a test, not review, guards it |
| k8s config delivery | Deployment manifests (`k8s/server/`) | Docs (`k8s/README.md`, `http-service-host.md`) | Server reads exactly one file via `Settings::load_from_file`; a Secret volume is the only delivery that keeps literal keys out of a ConfigMap |
| Version-string currency | CI / release tooling (`scripts/check-version-strings.sh`) | Makefile (`check-gates`), `ci.yml` `cargo-deny` job | Same tier as the nine sibling gates; offline, check-only |
| Image tag consistency | Build tooling (`Makefile docker-build-server`) | `k8s/server/*.yaml` image lines | Gate holds the manifest tag; the Make target must produce it |
| Changelog / ADR / count corrections | Planning + repo records | Release scripts (read-only consumers) | Corrections go forward in `[Unreleased]`; scripts only read `## [` headings |
| API-surface snapshot | Release tooling (`make api-surface`) | `.project/current-exports.txt` | Tracks the facade crate only; `paladin_ports` is guarded by cargo-semver-checks instead |
| Node-cache guide | mdBook docs (`fault-tolerance.md`) | `reasoning-effort.md`, `configuration.md` link in | One authoritative section, others link |

## Standard Stack

No new external library. Everything is in-repo tooling.

### Core
| Tool | Version (this container) | Purpose | Why Standard |
|------|--------------------------|---------|--------------|
| bash + embedded `python3` | python 3.13.16 (`tomllib` stdlib) | `check-version-strings.sh` | Every sibling gate is bash + a `python3 - <<'PY'` block [VERIFIED: codebase grep, `scripts/check-crate-names.sh`] |
| PyYAML | 6.0.1 | `check-doc-config.sh` (already used) and ad-hoc k8s manifest parse in verification | Already required by `make check-doc-config` [VERIFIED: tool run] |
| shellcheck | 0.9.0 | Lint new script and harness | Present locally; also run by pre-commit via `shellcheck-py` [VERIFIED: tool run, `.pre-commit-config.yaml`] |
| actionlint | not installed locally | Lint the `ci.yml` edit | CI downloads v1.7.12 itself (`ci.yml` actionlint job); cannot be run here [VERIFIED: `command -v`, `ci.yml`] |
| cargo-public-api | 0.52.0 on `nightly-2026-09-20` | `make api-surface` | Pinned by Makefile `PUBLIC_API_TOOLCHAIN` [VERIFIED: tool run] |
| cargo-release | 1.1.7 | Version recorded in `release.toml` | `cargo release --version` prints `cargo-release 1.1.7` here [VERIFIED: tool run] |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `serde_json` / `config` crate (already deps) | The D-03 example-config test | Follow the `web_server.rs::example_config_still_loads_and_validates` wrapper-struct precedent |
| `git ls-files -s` | Confirm new script and test are committed `100755` | The Makefile runs `"$f"` directly, CI only `chmod +x scripts/*.sh`, not `tests/scripts/` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Literal strings + gate | Kustomize / Helm templating | Rejected by D-12 and Deferred |
| `${` substring refusal | Exact `${NAME}` grammar | Rejected by D-01 (partly templated values would load) |
| Secret-delivered config | ConfigMap with commented keys; initContainer envsubst | Rejected by D-05 |

**Installation:** none.

**Version verification:** n/a (no package install). Tool versions above were read from the container on 2026-10-31.

## Package Legitimacy Audit

No external packages are installed or added by this phase (no `Cargo.toml` dependency change, no `npm`/`pip` install). The audit is therefore empty by construction.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| none | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                     FIRST-01: config load path (paladin-server)
 config.yml / Secret volume
        |
        v
 Settings::load_from_file(PALADIN_CONFIG | argv[1] | config.yml)   (no ${VAR} expansion, no APP_ layer)
        |
        v
 treasurer validate -> run store -> ledger -> cadence -> build_agent_registry_with_cadence
        |                                                 (LLM key: llm.<p>.api_key "${NAME}" | env fallback)
        v
 build_auth_config(&http.auth)
        |-- AuthConfig::validate()  <-- NEW: entry.key contains "${"  => Err("http.auth.api_keys[<name>]: ...")
        |        (order: empty key #i -> name valid #i -> reserved name #i -> **placeholder [name]** -> tenant -> reserved tenant -> dup name -> dup key)
        |-- no credentials  => Err("authentication is enabled but no credentials are configured ...")
        v
 ... run API wiring ...  ->  TcpListener::bind(...)   (only reached if every step above passed)

                     FIRST-03: version-string gate
 Cargo.toml [package].version  --tomllib-->  WANT
 manifests (root + crates/*)   --tomllib-->  workspace package names
 README.md, crates/*/README.md, docs/src/**, k8s/**, openapi.json
        |  (skip exempt files; skip non-text)
        v
 class 1 pins | class 2 "Current version" | class 3 k8s labels+images | class 4 openapi info.version
        v
 findings: "file:line: found 'X', want 'Y'" (all reported)  ->  exit 1 ; zero files scanned => named failure
        |
        +--> make check-version-strings (member of check-gates) ; ci.yml step in the `cargo-deny` job
             ("License & Dependency Policy", a required check in .github/rulesets/protect-main-branch.json)
```

### Recommended Project Structure (files this phase touches)
```
scripts/check-version-strings.sh                   # new, mode 100755
tests/scripts/check-version-strings_test.sh        # new, mode 100755 (Makefile runs it directly)
src/config/agents.rs                               # validate() + table tests + example-config test
src/bin/paladin-server.rs                          # one boot-refusal unit test beside the "no credentials" test
config.example.yml                                 # http.auth block + header comments
k8s/server/{secret.yaml.example,deployment.yaml,worker-deployment.yaml}  # configmap.yaml deleted
k8s/deployment.yaml                                # v0.1.0 label only
k8s/README.md                                      # server section, scaling notes, quick start
Dockerfile.server, docker/docker-compose.server.yml, docs/src/deployment-topologies/http-service-host.md   # F1
Makefile (check-version-strings, check-gates, docker-build-server, "nine members" comment)
.github/workflows/ci.yml (one step in cargo-deny job; "all ten" comment)
README.md, docs/src/{appendix,api-reference,getting-started,user-guides,contributing}/...   # sweep
CHANGELOG.md, MIGRATION.md (§11.1 M-B-29, §11.5), crates/paladin-{memory,content}/CHANGELOG.md
.planning/decisions/0056-allowance-admission-model.md, release.toml, scripts/finalize-crate-changelogs.sh
docs/src/user-guides/{fault-tolerance,reasoning-effort}.md, docs/src/getting-started/configuration.md
.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md, .planning/WINDOWS.md row 62
```

### Pattern 1: A new refusal in `AuthConfig::validate`
**What:** Add the `${` check after the reserved-name block, before the `tenant` block. Message uses `[{name}]`, never prints the value or any variable name.
**When to use:** FIRST-01.
**Example:** see Code Example 1.

### Pattern 2: Offline gate script (house shape)
**What:** `set -euo pipefail`; `WORKSPACE_ROOT` from `BASH_SOURCE`; an environment seam that defaults to the real tree; `python3 - … <<'PY'` produces a report whose first line is a status token (`OK` / `MISMATCH` / `ZERO_FILES`); bash prints the report and exits 1 on anything but `OK`. [VERIFIED: codebase grep, `scripts/check-crate-names.sh`, `scripts/check-dependency-allowlist.sh`]
**Seam convention:** siblings take no argument and override inputs by environment variable (`CORE_MANIFEST`, `PORTS_MANIFEST`); the harness points them at a `mktemp -d` tree. Use one variable, e.g. `VERSION_STRINGS_ROOT` (default `WORKSPACE_ROOT`), read for `Cargo.toml`, every scanned path, and the exempt-list lookup (the exempt list stores root-relative paths, so a fixture tree can recreate `docs/src/api-reference/migration-guide.md`).
**Per-class sanity (recommendation, `[ASSUMED]` as a design choice):** besides "zero files scanned", fail with a named status when a class matches nothing in the whole tree (class 1, 2, 3 and 4 each have at least one real match today), so a regex that silently rots cannot turn the gate vacuous.

### Pattern 3: Harness (`tests/scripts/*_test.sh`)
**What:** `set -uo pipefail`; `SCRATCH="$(mktemp -d …)"` with a `trap` cleanup; `FAILED`/`ASSERTIONS` counters; `assert_fire` (non-zero exit AND every needle present in output) and `assert_silent`; failing cases first, passing cases next, the real tree last; a closing `cksum` check that the real tree was not mutated. Fixtures are built inline under the scratch dir (only `check-publish-order` has a committed `fixtures/` dir). [VERIFIED: codebase grep, `check-dependency-allowlist_test.sh`, `check-workflow-triggers_test.sh`]
**Required cases (D-08 + D-11):** stale pin of each class fails with `file:line`; stale pin in the exempt `migration-guide.md` passes; identical content in a scanned file fails; non-workspace crate pin passes; `paladin-ai-core` alias form checked; prose line with `package = "paladin-ai-core"` but no version passes; `apiVersion: apps/v1` is not read as a label; `paladin:test` and a third-party image are not matched; zero scanned files fails; correct tree passes; real tree passes.

### Pattern 4: Secret-mounted config (D-05)
Layout recommended (Claude's Discretion area): keep the existing `paladin-server-secrets` Secret, add a `config.yml` `stringData` key, drop `api-key-ci`/`api-key-app`, keep `openai-api-key`. Mount with `items` and a restrictive `defaultMode`:
```yaml
volumes:
  - name: config
    secret:
      secretName: paladin-server-secrets
      defaultMode: 0440      # config.yml now holds literal API keys; fsGroup 65532 keeps it readable
      items:
        - key: config.yml
          path: config.yml
```
The container mount stays `mountPath: /etc/paladin`, `readOnly: true`. `items` projects only `config.yml`, so `openai-api-key` is not exposed as a file. Both Deployments (`deployment.yaml` line ~100, `worker-deployment.yaml` line ~145) currently reference ConfigMap `paladin-server-config` and both change. [ASSUMED: `defaultMode` octal syntax and `fsGroup` interplay are standard Kubernetes behaviour, not re-verified in this session; no `kubectl` is available here.]

### Anti-Patterns to Avoid
- **Exact-grammar refusal** (`^\$\{[A-Z0-9_]+\}$`): `pre-${VAR}` would load. D-01 says substring.
- **Echoing the variable name** in the error (D-02).
- **Hand-editing `.project/current-exports.txt`** (D-18).
- **Locating items by line number**: the inventory's numbers have drifted (D-00f).
- **Editing frozen records** (`[0.11.0]` sections, MIGRATION §9/§10): corrections go in `[Unreleased]`.
- **Using the word the CI placeholder grep forbids in `MIGRATION.md`**: CI's "Verify MIGRATION.md carries no unfilled placeholder marker" step fails on the four-letter token `TBD` anywhere in `MIGRATION.md` (ci.yml, `cargo-deny` job). Do not write that token in the new §11 entries.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Reading the workspace version / package names | A grep for `version =` in Cargo.toml | `tomllib` on the root `[package]` and each `crates/*/Cargo.toml` `[package].name` | Root manifest also has `[[bin]]`, `[[test]]` tables with their own `name`; `paladin-core` dir's package is `paladin-ai-core` [VERIFIED: manifests] |
| Parsing `openapi.json` | regex | `json.load(...)["info"]["version"]` | Exact |
| Enumerating publishable packages | A hardcoded list | `cargo metadata` (used by the release scripts) or manifests via `tomllib` | A hardcoded count is exactly the drift D-15 removes |
| Changelog section stamping | Manual heading edits at release time | `scripts/finalize-crate-changelogs.sh` (rule 2 inserts the dated heading under the `## [Unreleased]` anchor) | Existing, tested |
| Env-var expansion for API keys | Any expander | Nothing: out by decision D-00e | No new secret-handling path |

**Key insight:** every piece of this phase is "extend an existing house mechanism by one entry" (one validator branch, one gate script cloned from a sibling, one Makefile prerequisite, one CI step). The risk is in the inventory of what else copies the same strings, not in the mechanisms.

## Findings That Answer the Orchestrator's Questions (and Where CONTEXT Needs Amending)

### D-05: which LLM-key field does `paladin-server` read? [VERIFIED: codebase grep]
- Agents are built through `LlmProviderFactory` over `settings.llm` (`src/infrastructure/web/agent_host.rs` pass 2 and `facade_provisioner.rs:390`, `create_from_config(settings.llm.as_ref().unwrap_or(&empty), …)`). Provider availability is `LlmProviderFactory::is_configured(llm, provider)`, judged over the `llm:` section **and** the environment.
- Layering per provider (`crates/paladin-llm/src/config/layers.rs` module doc): the `llm.<provider>` block, then the provider's legacy env vars (`OPENAI_API_KEY` for openai, `bridge.rs`/`provider_factory.rs`), then the adapter default. An exact `"${NAME}"` in `api_key` resolves from `NAME`; an empty/omitted `api_key` falls back to `OPENAI_API_KEY`.
- The legacy top-level `llm_api_key` is read by **nothing** (only `settings.rs` declares it, `user_config.rs` has a test literal). **But it cannot be deleted:** `Settings.llm_type`, `llm_url` and `llm_api_key` are plain `String` fields with no `#[serde(default)]`, so the config fails to deserialize without them. `config.example.yml` already sets `llm_api_key: ""`. **Action:** in the k8s Secret-borne config replace `llm_api_key: "${OPENAI_API_KEY}"` with `llm_api_key: ""` (keep `llm_type`, `llm_url`, `max_file_size`, `sources`, `server`).
- Recommended k8s config: keep the `OPENAI_API_KEY` env injection (secretKeyRef, unchanged) and add `llm: { openai: { api_key: "${OPENAI_API_KEY}" } }` for explicitness, or omit the `llm:` block and rely on the env fallback; both work. Recommend the explicit block (matches `config.example.yml`, makes the mechanism visible to a reader). The `APP_RUN_*_URL_ENV` indirection in the worker Deployment is untouched.
- **Docs and files naming the retired objects** (must follow the move): `k8s/README.md` lines ~10, 25, 165 (`server/configmap.yaml`) and the worker/deployment volumes; `k8s/server/deployment.yaml` env (`PALADIN_API_KEY_CI`/`_APP`) and volume; `k8s/server/worker-deployment.yaml` volume (lines ~145-147); `k8s/server/secret.yaml.example` header and keys; `docs/src/deployment-topologies/http-service-host.md` line ~151 ("Deployment + Service + ConfigMap"). `docs/src/deployment/kubernetes.md` lines 35/405/501 name the **root** `k8s/configmap.yaml`, which is not restructured. No script, Makefile target or workflow references `server/configmap.yaml` or `paladin-server-config` [VERIFIED: grep over `k8s docs/src README.md Makefile .github scripts tests docker`].
- **k8s README quick start pitfall:** the documented sequence ends with `kubectl apply -f k8s/server/secret.yaml -f k8s/server/` (README and `http-service-host.md`). The directory apply also applies `worker-deployment.yaml`, whose pods need a second Secret `paladin-secrets` (`run-db-url`, `redis-url`) plus Postgres and Redis; the README itself says the server needs "no Redis or object store". Applying the directory leaves worker pods in `CreateContainerConfigError`. To make the quick start a clean copy-paste (CONTEXT Specifics), apply `deployment.yaml` and `service.yaml` explicitly. `kubectl apply -f <dir>` ignores `*.yaml.example`, so the Secret template is not applied by the directory form.

### D-03: tests that load `config.example.yml`, and the boot-test shape [VERIFIED: codebase grep]
| File:test | Reads `http.auth`? | Cross-checks `treasurer.allowance.api_keys` vs `http.auth.api_keys`? |
|---|---|---|
| `src/infrastructure/web/agent_host.rs::the_shipped_example_config_builds_every_agent_it_describes` (~2297) | No: deserializes `llm` + `agents` only | No |
| `src/config/web_server.rs::example_config_still_loads_and_validates` (~151) | No: `trace` + `web_server` only | No |
| `src/config/treasurer.rs::example_config_treasurer_block_round_trips_to_default` (~939) | No: `treasurer` block only; the example's `allowance:` block is fully commented out | No |
| `src/config/agent_runtime.rs::example_config_agent_runtime_block_round_trips_to_default` (~1809) | No | No |
| `crates/paladin-llm/src/config/llm.rs::example_config_llm_section_loads_unchanged` (~760) | No: `llm:` section text only | No |

So `api_keys: []` with commented entries keeps all five green. (Two doc comments, `web_server.rs` ~150-160 and `agent_runtime.rs` ~1622, claim `Settings::load_from_file("config.example.yml")` fails on the keyless `ollama` block; `llm.rs` says that was fixed. The new D-03 example test should follow the wrapper-struct precedent, not `Settings::load_from_file`, so it does not depend on whichever is true; that doc comment is stale and can be left.)
- **New example test shape:** a local `#[derive(Deserialize)] struct Example { #[serde(default)] http: WebHttpConfig }` over `config::File::new("config.example.yml", Yaml)` (cwd is the package root for the root crate's unit tests, as the existing tests rely on), then `assert!(example.http.auth.validate().is_ok())`. Because `api_keys: []` and `enabled: true` with no bearer *passes `AuthConfig::validate`* (no-credentials is refused by `build_auth_config`, not by `validate`), also assert `example.http.auth.api_keys.is_empty()` so the test documents that a verbatim copy fails closed later. Optional extra proof of D-04's "uncommented unedited is refused as empty": read the file text, uncomment the commented entry block, parse, and assert `validate()` returns the empty-key error.
- **Boot-refusal test:** the existing test is `build_auth_config_fails_closed_when_enabled_with_no_credentials` at `src/bin/paladin-server.rs:~800-815`, in the bin's `#[cfg(test)]` module (uses `api_key(name)` helper, `AuthConfig { enabled, api_keys, bearer_token }`, calls `build_auth_config(&cfg)`). Mirror it: build a config whose key is `${PALADIN_API_KEY_CI}`, assert `Err`, that the message starts `http.auth.api_keys[ci-runner]`, and contains neither the value nor `PALADIN_API_KEY_CI`. Ordering evidence for "before any listener binds": `build_auth_config` is called at `paladin-server.rs:~145` and `TcpListener::bind` at `~304`; `build_auth_config` calls `cfg.validate()?` as its first line [VERIFIED: file read]. Run with `cargo test --features web-server --bin paladin-server build_auth_config` (the bin has `required-features = ["web-server"]`).
- **Process-level proof (optional, recommended for the evidence file):** SC1 says the server "exits at config load". The unit test proves the function; one captured run of the built binary against a temp config with `agents: []` and a `${…}` key (exit status 1, stderr `paladin-server failed to start: http.auth.api_keys[…]`) would prove the exit. `agents: []` reaches the auth step without needing an LLM key (agent registry is built first, line ~130). `[ASSUMED]` not executed in this session (it needs a `--features web-server` binary build).

### F1 (new, not in CONTEXT): the Docker first-run path breaks and must be handled
`Dockerfile.server` does `COPY config.example.yml /etc/paladin/config.yml` and sets `PALADIN_CONFIG` to it. Today the example's `${PALADIN_API_KEY_CI}` loads as the literal string, so `docker run -e PALADIN_API_KEY_CI=sk-…` "works" (and silently accepts the placeholder text as a valid key, which is the bug). After FIRST-01 plus D-04 the baked config has `api_keys: []` with auth enabled, so the container refuses to start with "authentication is enabled but no credentials are configured", and the env var is not read by anything. Everything user-facing that says otherwise must change in this phase (it is "copy from the docs" territory, the phase goal):
- `docs/src/deployment-topologies/http-service-host.md`: the config sample line ~82 (`key: "${PALADIN_API_KEY_CI}"`), "Running it" binary snippet ~137 (`PALADIN_API_KEY_CI=sk-… cargo run …`), Docker snippet ~146, and the Kubernetes paragraph ~150.
- `docker/docker-compose.server.yml`: header comment, `PALADIN_API_KEY_CI/_APP` env passthrough, and `image: paladin-server:0.6.0` (stale version, see Sweep); the file already has a commented `volumes: ../config.yml:/etc/paladin/config.yml:ro`. Recommended: make the config mount the documented, uncommented path (copy `config.example.yml` to `config.yml`, paste a literal key) and delete the dead env lines.
- `Dockerfile.server` header comment (lines 7-9: `paladin-server:0.6.0`, `-e PALADIN_API_KEY_CI=…`).
- Decide whether the baked default config should stay `config.example.yml` (then the image boots only with a mounted config; acceptable and honest) or not; the minimal choice is to keep it and document the mount.
- `tests/fixtures/config/v0.9.0-config.example.yml` also carries `${PALADIN_API_KEY_CI}`; it is a frozen documentation-only fixture that no test loads (`tests/fixtures/config/README.md`, MIGRATION §9) and is outside the gate's scope. Leave it.
[VERIFIED: codebase grep and file reads]

### F2/F3: `llm_api_key` and `architecture-decisions.md`
See the D-05 answer above (F2: `llm_api_key` must stay as `""`) and the Exemption recommendations in the Version Sweep Inventory below (F3: fix `architecture-decisions.md:645`, do not exempt it).

### F4: FIRST-08's by-name clause cannot hold [VERIFIED: tool run + grep]
- `make api-surface` was run on `nightly-2026-09-20`: **exit 0, "API surface unchanged", 4325 items** (twice). The committed `.project/current-exports.txt` (generated 2026-10-31 13:09 UTC, 8640 lines, last touched by `6b73e11b0`) is current, so FIRST-08's regeneration is met by evidence and `make api-surface-update` is not needed.
- `rg -c "UnsupportedParameter|check_reasoning" .project/current-exports.txt` returns no match. This is structural, not lag: `LlmError::UnsupportedParameter` and `LlmPort::check_reasoning` live in `paladin-ports` (`crates/paladin-ports/src/output/llm_port.rs`); `src/lib.rs` ~175 states the facade deliberately does **not** re-export `paladin_ports` (consumers import it directly); and the snapshot tracks only the `paladin` facade package. The snapshot lists foreign items only where a facade item mentions them in a signature (e.g. `dyn paladin_ports::output::llm_port::LlmPort`). There is no `pub use paladin::ports…` line at all.
- Precedent: WINDOWS row 59 (Phase 40) recorded the identical tool-design limitation (`RunReadScope`/`TenantId`) and was `waived`.
- **Action:** record this in the evidence file, and reconcile the wording of FIRST-08, ROADMAP SC5 and inventory C10 (for example "regenerated and current; the `paladin-ports` rows are not representable in the facade snapshot, that crate's surface is guarded by `make check-semver`"). Do not hand-edit the snapshot, and do not add a `pub use` just to make a name appear (new public surface, out of scope).

### F5: ADR-0056 gaps (D-14) [VERIFIED: file read]
`src/application/services/treasurer/tests.rs` module doc (lines ~12-22) already says "ADR-0057 supersedes ADR-0056's earlier plan to reserve at the boundary" and cites WINDOWS row 65: **closed, leave alone**, record the proving `rg -n ADR-0057 src/application/services/treasurer/tests.rs`.
ADR-0056 (`.planning/decisions/0056-allowance-admission-model.md`) stale spots, located by content:
1. Context bullet "ADR-0052 fixes where Phase 42 halts…" (~22-23): fine as written (ADR-0052 still fixes the location); optional.
2. D-01 text "…and Phase 42's reservations reuse the same `window_for` function" (~57): stale; ADR-0057 writes no reservation.
3. D-05 (~69-71): "…is accepted here and is closed by Phase 42's reservation at the superstep boundary (ADR-0052), which adds a `reserve` beside this check" : stale, the core claim.
4. Alternatives bullet "Reusing `spend()`…" (~120): "ignores reservations once Phase 42 places them": stale.
5. Alternatives bullet "Reserve a minimum hold at admission…" (~122-123): "pulls Phase 42's settle/release attachment forward": stale.
6. Tests paragraph (~212-213): "…not covered by a prevention test; Phase 42 closes the first.": stale.
7. Downstream Consumers, Phase 42 bullet (~217-223): body still says "closes the D-05 over-admission race by reserving at the superstep boundary", but a dated note **already follows it** ("Dated note, 2026-10-06 (plan 42-01, ADR-0057): superseded …"). FIRST-06 asks that the line no longer say it; fix the body text and keep the existing dated note.
Add the "Amended 2026-10-31 (Phase 54, FIRST-06)" paragraph under `## Status` in the house position (see ADR-0015/0059, line 9 of those files: a bold `**Amended …:**` paragraph directly after the Status block). PROMOTION line unchanged.

### F6 (D-13): how the 0.11.0 finalize handled anchors; Phase 58 debt [VERIFIED: git + file reads]
- Commit `310829733` ("stamp 0.11.0 changelogs") ran `finalize-crate-changelogs.sh`; at that time every changelog had a `## [Unreleased]` anchor, and the commit message records "R7=a: the now-empty [Unreleased] anchor removed from every changelog" and "EMPTY=a: one lockstep line under paladin-content, paladin-memory, paladin-notifications". So the anchor is consumed per release (script rule 2 inserts the dated heading under it; the empty anchor was then removed by hand).
- Today: `## [Unreleased]` exists only in the root `CHANGELOG.md`, `paladin-core` and `paladin-web` (Phase 53 added entries). **Nine** crate changelogs have none: battalion, content, eval, herald, llm, memory, notifications, ports, storage. This phase adds anchors to memory and content, leaving **seven** (battalion, eval, herald, llm, notifications, ports, storage) that need an anchor or a `## [0.11.1]` section before Phase 58's finalize (rule 3 is a named failure; `check-release-consistency.sh` requires a `## [X.Y.Z]` heading in all twelve changelogs). `check-changelogs.sh` does not require the anchor.
- **What memory and content actually changed in 0.11.0** (so the correction is accurate): `git diff v0.10.1 v0.11.0`: `paladin-memory` rewrote `citadel/file_citadel.rs` (84 lines; the `FileCitadel` adapter now writes `paladin.data`, reads `paladin.node` or `paladin.data`, never both; `pre_rename_citadel_snapshot_loads_through_file_citadel` is the proof) plus a test-only edit in `memory_extraction_service.rs`. `paladin-content` changed only test lines (`content_item.node.node` to `content_item.entity.data`, added `cost`/`reasoning` in test literals) and has no persistence adapter of its own; the persisted-key change reaches it through the `paladin-ai-core` `ContentItem` type it builds. Recommended wording: memory: "`FileCitadel` writes the Paladin payload under `paladin.data` (v0.10 wrote `paladin.node`); one-way; see MIGRATION.md §10.1 M-B-12". Content: say its own code is unchanged but the `ContentItem` JSON it produces now uses `entity`/`data` keys one-way (cite M-B-12); do not claim the crate persists anything. Both reuse M-B-12's advice: back up, upgrade readers before writers. `paladin-notifications` also carries "No functional change" and was not named by FIRST-05; its `email_notification_adapter.rs` was not part of the persisted-key change, so leave it.
- M-B-12 exact text is at `MIGRATION.md` line ~130 (§10.1, frozen); the root CHANGELOG `[0.11.0]` section carries the entry-level wording to cite. The new MIGRATION §11.1 row for FIRST-01 is **M-B-29** (last used: M-B-28).

### F7 (D-15, D-16, D-17): counts, cargo-release, footer [VERIFIED: grep + tool run]
True counts today: 12 publishable packages (root `paladin-ai` + 11 crates: `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`), 13 workspace packages (adds unpublished `paladin-doc-examples`), 12 `CHANGELOG.md` files (11 crate + root), `publish-crates.sh`'s built-in `CRATES` array has 12 entries.
| File (locate by content) | Text | Status | Recommendation |
|---|---|---|---|
| `release.toml` "extends to the ten per-crate CHANGELOG.md files" and "list of ten distinct target files" | ten | stale (eleven crate changelogs) | count-free: "every per-crate `CHANGELOG.md`" |
| `scripts/finalize-crate-changelogs.sh` header "an eleven-file manual chore: today all ten crate changelogs sit at …" | eleven/ten, plus a stale "today" claim | stale | rewrite count-free, drop "today" |
| `.github/workflows/ci.yml` MSRV comment "all ten crate manifests inherit it" | ten | stale | "every workspace manifest" |
| `release.toml` "`cargo-release 1.1.2`" | version | stale | `1.1.7` (installed here, `cargo release --version` = 1.1.7; evidence `47-CI-EVIDENCE.md` rows 13-14, dated 2026-10-24/25) |
| `scripts/check-release-consistency.sh` ~595 hint text "hand-editing eleven files" | eleven | stale (twelve changelogs) | count-free (not in FIRST-07's list; optional) |
| `scripts/publish-crates.sh` ~114 "built-in eleven-crate dependency order" | eleven | stale (array has twelve) | count-free (optional) |
| `scripts/finalize-release-body.sh` ~267 "not the eleven-crate workspace" | eleven | arguably true (root + eleven crates) | leave, or count-free |
| `docs/src/appendix/release-automation.md` ~220 "The eleven per-crate changelogs" | eleven | true (eleven crate changelogs) | leave |
| `Makefile` ~239 "the nine members of check-gates" | nine | becomes ten when this phase adds a member | update to ten or count-free |
| `ci.yml` twelve/"all twelve" comments (~374, 389, 418), `Makefile` ~641 | twelve | correct | leave |
D-17: the footer link is read by nothing. `make release` no longer has any perl substitution (the recipe is `cargo release version … --execute`, openapi regen, `finalize-crate-changelogs.sh`, `check-release-consistency.sh`, commit/tag/push). `extract-changelog-section.sh` and `check-release-consistency.sh` match `## [X.Y.Z]` headings only; link-reference definitions (`[Unreleased]: …`) start with `[`, not `## [`. Only the three footer lines exist (`[Unreleased]`, `[0.2.0]`, `[0.1.0]`, at `CHANGELOG.md` ~3408-3410). Deleting the `[Unreleased]:` line is safe. (The `## [Unreleased]` heading then renders as plain text in markdown; that is the intended result.)

### F8 (D-19): node cache [VERIFIED: grep + file reads]
- `NodeCacheConfig` readers: only `src/config/node_cache.rs`, its re-export in `src/config/mod.rs`, doc mentions in `agent_runtime.rs`, `examples/node_result_cache.rs`, and `paladin-storage` (backend types). `Settings` has no `node_cache` field; `rg node_cache|NodeCache src` outside `config/` returns nothing; neither `src/bin/paladin-server.rs`, `infrastructure/web/*` nor `paladin-web` references it. Confirmed: **no server or wiring file reads `NodeCacheConfig`**.
- The stampede lock: `WarEngine::with_cadence(port, lock_ttl)` (`crates/paladin-battalion/src/engine/mod.rs:~2144`) engages only for a node with a cache policy on an engine that also has `with_node_cache` (~2072); its rustdoc already says "The server attaches no node cache today, so it attaches no lock either; this is a library builder." Note `paladin-server.rs` calls `FacadeProvisioner::with_cadence` (rate pacing), a different method; do not conflate.
- `NODE_CACHE_KEY_VERSION = "k1"` at `crates/paladin-battalion/src/engine/cache_key.rs:69`, key rendered `{k1}:{graph fingerprint}:{node id length}:{node id}:{digest}` (module doc ~50); the module doc (~44-47) already says the key stays `k1` under REASON-01 and that a provider-level default effort is not in the key ("needs a new cache namespace or a version bump"). Pin test at ~583.
- Current page state: `getting-started/configuration.md` ~668-672 (`lock_ttl_secs`) **already** says "`paladin-server` attaches no node cache, so the key has no effect on the shipped server today" (add a link to the Aegis section; do not restate). `reasoning-effort.md` "The node cache" (~577-583) already explains the provider-default gap and says "start with a fresh node-cache namespace (change the `key_prefix`, see [Aegis](fault-tolerance.md)) or clear the cache", but the link target has no such text. `fault-tolerance.md` "Node Caching" (~328-380) has the key-composition paragraph and `NodeCacheConfig` table but **none of** the three D-19 items: no "server attaches no cache" statement, no key format or version, no `key_prefix` rotation. So the new text goes in `fault-tolerance.md` only, with an anchor heading the other two pages can link to (for example a `### Cache key version and rotation` sub-heading), and `reasoning-effort.md` / `configuration.md` get a link, not a restatement. Also note `fault-tolerance.md` key-composition paragraph lists "model, system prompt, temperature, `max_loops`, stop words" and omits reasoning effort/fallback; fix it while there (the module doc says they are included when set).

### D-10: image and build path [VERIFIED: file reads]
`Dockerfile.server` builds `paladin-server` (stage 2 `debian:12-slim`, `ENTRYPOINT paladin-server`, `PALADIN_CONFIG=/etc/paladin/config.yml`). `Makefile` `docker-build-server` runs `docker build -f Dockerfile.server -t paladin-server:latest .` (~line 522). Manifests: `k8s/server/deployment.yaml` `image: paladin-server:0.6.0`, `worker-deployment.yaml` `image: paladin-server:0.10.0`, both `imagePullPolicy: IfNotPresent`. With an explicit `IfNotPresent` and a version tag, a locally built or `kind load`-ed image is used without a pull (the default policy becomes `Always` only for `:latest` when no policy is given, which these manifests avoid by stating one). So tagging the build `paladin-server:<workspace version>` plus keeping `:latest` and pointing the manifests at the version tag works for a locally loaded image. Implementation: derive the version in the Make recipe from the root manifest (for example `$(shell python3 -c 'import tomllib;print(tomllib.load(open("Cargo.toml","rb"))["package"]["version"])')` or `cargo metadata`), and use a second `-t paladin-server:$(VERSION)`; test with `make -n docker-build-server` (no docker daemon needed for a dry run). `k8s/deployment.yaml` (root) uses `paladin:test` and `imagePullPolicy: Never` for the CI kind job (`ci.yml` ~1868 `sed 's/paladin:latest/paladin:test/g'`); only its `version: v0.1.0` labels change, which `kubectl`/CI do not read.

### D-11: CI layout [VERIFIED: file read]
Gate scripts run as steps of the **`cargo-deny` job** (display name "License & Dependency Policy", `ci.yml` ~125-222), after "Make scripts executable" (`chmod +x scripts/*.sh`), in this order: check-changelogs, check-crate-names, check-advisory-register, check-workflow-suppressions, check-workflow-triggers, check-codeql-dismissals, check-publish-order, check-dependency-allowlist (commented "CI runs each gate script itself and does not call `make check-gates`"), check-migration-allowlist, the MIGRATION.md placeholder-token grep, then "Run shell-guard regression tests" (`make test-shell-guards`, ~218). Add the new step after "Check core/ports dependency allowlist (ADR-0015)" with a comment in the same style. That job is a required status check (`.github/rulesets/protect-main-branch.json` ~147; `docs/src/appendix/branch-protection.md` ~99), so a new **step** blocks merges with no register change. There is no register of CI steps; `check-workflow-triggers.sh` reads workflow triggers and the `branching-model.md` policy table, which are unaffected by a new step. A new *job* would instead need ruleset and policy-table edits: do not add one. The new test file must be committed `100755` (Makefile `test-shell-guards` executes `"$f"` directly; CI's chmod covers only `scripts/*.sh`).

## Version Sweep Inventory (D-07 / D-08 / D-09) [VERIFIED: own scan, 2026-10-31]
Method: a python scan over `README.md`, `crates/*/README.md`, `docs/src/**`, `k8s/**`, `crates/paladin-web/openapi.json` for the four classes, with workspace names read from the manifests (13: `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-doc-examples`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`; note the directory `crates/paladin-core` is package **`paladin-ai-core`**, and `crate-map.md:227` uses the local alias key `paladin-core = { package = "paladin-ai-core", version = … }`). 130 files scanned.

| File | Hits | Wanted | Notes |
|---|---|---|---|
| `README.md` | 2 | `0.11.0` | line ~108 `paladin-ai = "0.5"`; ~183 `Current version: **0.8.0**` (bold markers: the class-2 regex must tolerate `**`, backticks and a leading `v`) |
| `docs/src/appendix/sentinel.md` | 1 | `0.11.0` | line ~66 `"0.5"` |
| `docs/src/api-reference/crate-map.md` | 7 | `0.11.0` | 190, 212, 227 (alias form), 228-231; line ~223 is prose naming `package = "paladin-ai-core"` with **no version**: the gate must not flag it |
| `docs/src/api-reference/feature-flags.md` | 18 | `0.11.0` | includes `//!` lines inside a rust fence (~362, 391) |
| `docs/src/getting-started/installation.md` | 7 | `0.11.0` | `paladin-ai-core = "0.10.0"` etc.; aligned `=` with extra spaces |
| `docs/src/getting-started/quickstart.md` | 3 | `0.11.0` | `paladin-ai    = "0.10.0"` (aligned) |
| `docs/src/user-guides/{paladin-agents,battalion-patterns,maneuver-flow-dsl}.md` | 1 each | `0.11.0` | |
| `docs/src/contributing/architecture-decisions.md` | 1 | see below | line ~645 |
| `k8s/server/deployment.yaml` | 3 | `v0.11.0` labels (2), `0.11.0` image | `image: paladin-server:0.6.0` |
| `k8s/server/worker-deployment.yaml` | 3 | same | labels `v0.10.0`, image `paladin-server:0.10.0` |
| `k8s/deployment.yaml` (root) | 2 labels | `v0.11.0` | `image: paladin:test` is a non-version tag and not matched |
| `docs/src/api-reference/migration-guide.md` | 21 | exempt | all `"0.4"` pins |
| `crates/*/README.md` | 0 | — | none carry pins today; the glob must still be scanned (D-07) |
| `crates/paladin-web/openapi.json` | 0 | — | `info.version` is already `0.11.0` (line 14) |
Total: **49 hits to fix** in 12 files (excluding `architecture-decisions.md`) plus the architecture-decisions line; 21 exempt.

**Exemption recommendations (D-08, by reading each file):**
- `docs/src/api-reference/migration-guide.md`: **exempt, whole file.** Its own header says the historical guide "stops at v0.5.0"; every pin is a `"0.4"` row of the 0.4 upgrade (`paladin-ai = "0.4"`, features). Reason string: "historical 0.4 upgrade rows".
- `docs/src/contributing/architecture-decisions.md`: **do not exempt; fix the one line.** It is not an ADR record: line ~641-648 is the `## Publishing Adapters` section's copyable `Cargo.toml` for an *adapter author's crate* (`name = "paladin-custom-llm"`, `paladin-ai = { version = "0.5", default-features = false }`). A reader copies it today and gets a stale pin. Change `"0.5"` to `"0.11.0"`. Its `version = "0.1.0"` is the adapter's own `[package]` version and `paladin-custom-llm = "0.1"` (~663, a `//!` line) is a non-workspace crate, so neither is matched. This deviates from CONTEXT D-08's pre-listed second exemption; the planner should keep the exemption list at one entry unless the user disagrees.
- No other file needs an exemption. `upgrading.md`, `stable-api.md`, `release-automation.md` and `release-recovery.md` mention versions only in prose or placeholders.

**Residuals outside the locked gate scope (flag, do not silently widen):**
- `docker/docker-compose.server.yml` (`image: paladin-server:0.6.0`) and `Dockerfile.server` (header `paladin-server:0.6.0`) carry stale first-party tags a user copies. D-07 scopes class 3 to `k8s/`. Fix the strings in this phase (they are part of F1) and, as a cheap recommendation, add `docker/docker-compose.server.yml` and `Dockerfile.server` to the gate's class-3 scan; that widens D-07 slightly, so treat it as an Open Question.
- `docs/src/deployment/docker.md` (~91, 515, 551-566, 664) and `docs/src/deployment/kubernetes.md` (~206) show `ghcr.io/your-org/paladin:v0.8.0` (an illustrative registry); `docs/src/operations/monitoring.md` and `appendix/sanctum-deployment.md` use `paladin:latest`. Class 3 as locked is `k8s/` only, so none of these trips the gate. They are stale version examples; note them in the evidence file as known residuals unless the user widens the gate.

## Common Pitfalls

### Pitfall 1: Wrong insertion point in `validate`
**What goes wrong:** Putting the `${` check before the name check gives the `#index` form and loses the entry name; putting it after the tenant check lets a missing tenant mask it, and after the duplicate check makes two identical placeholders report "duplicate".
**Current order (read from the file):** empty key (`#i`), name valid (`#i`), reserved name (`#i`), tenant empty, tenant valid, reserved tenant, duplicate name, duplicate key. **Insert immediately after the reserved-name block.**
**How to avoid:** table tests in this order of cases: exact `${X}`, `pre${X}post`, unterminated `${X`, `$X` accepted, `enabled: false` still refused, refusal on the second entry (index preserved as `[name]`, not `#1`), two identical placeholders report as placeholder, message contains neither the value nor `X`. Mind `format!` escaping: a literal `${` in a format string is written `${{`.
**Warning signs:** a test expects `[#0]` for the placeholder case; the message contains `PALADIN_API_KEY_CI`.

### Pitfall 2: The refusal also hits `key: ""` copies
`config.example.yml`'s commented entries use `key: ""`; an uncommented, unedited entry fails the existing empty-key check first (`http.auth.api_keys[#0]: 'key' must not be empty`). That is intended (D-04) and is a different message from the placeholder one; do not "fix" it.

### Pitfall 3: Doc comments that now lie
`ApiKeyConfig::key` rustdoc says the key "should come from an environment variable / secret in practice", `AuthConfig::validate` rustdoc lists the checks, and `config.example.yml` says "Put real keys in env/secrets, not in VCS" and "API keys are read from the ENVIRONMENT, never from this file" (header lines 3-4, which is false for `http.auth.api_keys`). Update all three (rustdoc changes do not alter the API snapshot) and document the placeholder rule in `validate`'s rustdoc.

### Pitfall 4: Gate regex false positives and negatives
`version:` in k8s must be anchored (`^\s*version:`) so `apiVersion: apps/v1` is never read; `image:` tags must look like a version before comparing (`paladin:test` and `:latest` are ignored); class-1 prose such as `package = "paladin-ai-core"` without a `version =` in the same inline table must not be reported; a `[dependencies.paladin-ai]` table form with `version = "…"` on the next line is not matched by an inline-table regex (none exist today; add a test that documents the choice rather than silently missing it); `Current version: **0.8.0**` needs `*`/backtick/`v` stripping; non-UTF-8 files under `docs/src/assets` (SVG is text, PNG is not) must be skipped, not crash; sort the report so Phase 58's checklist is stable.

### Pitfall 5: Gate red on the branch
D-09: the sweep must land before or with the gate. If the gate plan is ordered ahead of the sweep, `make check-gates` and the CI step are red. Make the gate plan depend on every sweep plan (README/docs, k8s, Docker docs).

### Pitfall 6: Serialised shared files
`CHANGELOG.md [Unreleased]` (FIRST-01 entry, k8s move entry, gate entry, footer deletion), `MIGRATION.md` §11.1/§11.5, `ci.yml` (D-11 step and D-15 comment), `Makefile` (target, `check-gates`, `docker-build-server`, "nine" comment), and `k8s/server/*` (D-05 move, D-09/D-10 tags) are each touched by more than one requirement. Put each file's edits in one plan or order the plans, so parallel waves cannot conflict.

### Pitfall 7: Exec bit and `shellcheck`
New `scripts/check-version-strings.sh` and `tests/scripts/check-version-strings_test.sh` must be committed `100755`. Run `shellcheck` on both (0.9.0 is available locally). `actionlint` cannot run locally; keep the `ci.yml` step a copy of its neighbours.

### Pitfall 8: Phase 58 interaction
The gate is not part of `make release-check`, and `make release` bumps manifests, regenerates `openapi.json`, finalizes changelogs and commits via `git add -u`. Phase 58 must therefore edit the gated strings (README pin, `Current version`, docs pins, k8s labels and image tags) **in the same commit** as the manifest bump, or CI on that commit is red and `release.yml`'s CI-conclusion clause blocks the tag (D-12). Whether `cargo release version --execute` tolerates a dirty working tree (strings edited before the bump) is `[ASSUMED]` unverified; Phase 58 should check it. Also remember the seven changelogs that need an anchor (F6).

## Code Examples

### 1. The validator branch (FIRST-01)
```rust
// Source: src/config/agents.rs AuthConfig::validate, inserted after the reserved-name check
// D-01/D-02: any '${' is an unexpanded placeholder; the server expands nothing here. The message
// names the entry and never the value or the variable.
if entry.key.contains("${") {
    return Err(format!(
        "http.auth.api_keys[{name}]: 'key' is an unexpanded '${{…}}' placeholder -- the server \
         does not expand environment variables in http.auth.api_keys; set the literal key value"
    ));
}
```

### 2. Boot-refusal test shape (D-03), beside `build_auth_config_fails_closed_when_enabled_with_no_credentials`
```rust
// Source: pattern of src/bin/paladin-server.rs tests (api_key helper, build_auth_config)
#[test]
fn build_auth_config_refuses_an_unexpanded_placeholder_key() {
    let mut key = api_key("ci-runner");
    key.key = "${PALADIN_API_KEY_CI}".to_string();
    let cfg = AuthConfig {
        enabled: true,
        api_keys: vec![key],
        bearer_token: BearerTokenAuthConfig { enabled: false, tenant: None },
    };
    let err = match build_auth_config(&cfg) {
        Ok(_) => panic!("a placeholder key must refuse to boot"),
        Err(e) => e.to_string(),
    };
    assert!(err.starts_with("http.auth.api_keys[ci-runner]"), "{err}");
    assert!(!err.contains("PALADIN_API_KEY_CI"), "never echo the variable: {err}");
}
```
(Test code may use `panic!`; the repo's no-`unwrap`/`panic!` rule is for library code.)

### 3. Example-config test (D-03)
```rust
// Source: pattern of src/config/web_server.rs::example_config_still_loads_and_validates
#[derive(serde::Deserialize)]
struct Example { #[serde(default)] http: WebHttpConfig }
let example: Example = config::Config::builder()
    .add_source(config::File::new("config.example.yml", config::FileFormat::Yaml))
    .build().unwrap_or_else(|e| panic!("config.example.yml must build: {e}"))
    .try_deserialize().unwrap_or_else(|e| panic!("its http section must deserialize: {e}"));
assert!(example.http.auth.validate().is_ok());
assert!(example.http.auth.api_keys.is_empty()); // a verbatim copy fails closed in build_auth_config
```

### 4. Gate core (python block sketch)
```python
# Source: scripts/check-crate-names.sh pattern (tomllib, status token, collect-then-report)
want = tomllib.load(open(f"{root}/Cargo.toml","rb"))["package"]["version"]   # "0.11.0"
ws = {tomllib.load(open(p,"rb"))["package"]["name"] for p in manifests}      # incl. paladin-ai-core
# class 1: inline pins; alias form needs version in the same {...}
PIN = re.compile(r'(?<![\w-])(?P<k>[A-Za-z0-9_-]+)\s*=\s*(?:"(?P<v>[^"]*)"|\{(?P<t>[^}]*)\})')
# for each match: pkg = k if k in ws else (package="…" inside t if in ws) ; v = bare string or version="…" inside t
# report f"{rel}:{lineno}: found '{v}', want '{want}'" when pkg in ws and v != want
# class 3: ^\s*version:\s*"?v?(\d+\.\d+\.\d+)"?  and  image:\s*(\S*/)?(paladin-server|paladin):v?(\d+\.\d+\.\d+)
```

### 5. k8s Secret template shape (D-05)
```yaml
# k8s/server/secret.yaml.example  (config.yml moves here; api keys are LITERAL, nothing expands them)
apiVersion: v1
kind: Secret
metadata: { name: paladin-server-secrets, namespace: paladin, labels: { app: paladin-server } }
type: Opaque
stringData:
  openai-api-key: "REPLACE_ME"   # injected as OPENAI_API_KEY; referenced below as "${OPENAI_API_KEY}" (llm.openai only)
  config.yml: |
    llm_type: "openai"
    llm_url: "https://api.openai.com/v1"
    llm_api_key: ""              # legacy, required by the schema, read by nothing
    max_file_size: 10485760
    sources: []
    llm: { default_provider: "openai", openai: { api_key: "${OPENAI_API_KEY}" } }
    # ... server/timeouts/http (auth.enabled: true, api_keys with key: "" and tenant) / agents as today
```
Parse-check it with PyYAML (`yaml.safe_load` on the Secret, then `yaml.safe_load` on `data["stringData"]["config.yml"]`), since no `kubectl` is available.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Examples carry `${PALADIN_API_KEY_*}` and claim load-time expansion | Empty/commented keys, refusal at boot | This phase | Placeholder can no longer authenticate as a credential |
| One-off currency sweeps (Phase 46) | A gate in `check-gates` and CI | This phase | Stale strings turn CI red |
| k8s config in a ConfigMap | Config in a Secret | This phase | Literal keys never live in a ConfigMap |
| `cargo-release 1.1.2` recorded | `1.1.7` | v0.11.0 (47-01) | Comment catches up |

**Deprecated/outdated:** the `[Unreleased]:` compare link (v0.2.0...HEAD), the `PALADIN_API_KEY_CI/_APP` env injections, the `api-key-ci`/`api-key-app` Secret keys.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `defaultMode: 0440` with `fsGroup: 65532` lets the non-root container read the mounted Secret file; octal YAML form is accepted by `kubectl` | Pattern 4 | Pod could not read `config.yml`; fix is `0444` or `0400`; no `kubectl` here to test |
| A2 | A process-level run of `paladin-server` with `agents: []` and a `${…}` key exits 1 before binding (not executed) | D-03 answer | The optional evidence capture needs a different minimal config |
| A3 | `cargo release version --execute` may refuse or accept a dirty working tree when Phase 58 edits strings first | Pitfall 8 | Phase 58 sequencing changes (edit strings after the bump, same commit) |
| A4 | Adding a "class matched nothing" named failure per class is a good gate design | Pattern 2 | None beyond extra test cases; drop if the user wants D-11's exact wording only |
| A5 | Widening class 3 to `docker/docker-compose.server.yml` and `Dockerfile.server` is acceptable | Sweep residuals | Needs the user's OK, it enlarges D-07 |
| A6 | Count comments outside FIRST-07's list (`check-release-consistency.sh` hint text, `publish-crates.sh`) are in scope for the count-free sweep | F7 | Planner may skip them; no requirement fails |

## Open Questions

1. **Docker first-run path (F1).**
   - What we know: the baked config is `config.example.yml`; compose and docs pass `PALADIN_API_KEY_CI` env; after this phase that boots into the "no credentials" refusal.
   - What's unclear: whether the user wants the image to stay example-baked (document the mount) or ship no default config.
   - Recommendation: keep the image as is, document the config mount in compose, `Dockerfile.server` header and `http-service-host.md`; add the files to the plan's sweep.
2. **FIRST-08 wording.**
   - What we know: the by-name rows cannot appear; the snapshot is current.
   - Recommendation: reconcile REQUIREMENTS FIRST-08, ROADMAP SC5 and inventory C10 in the final plan, citing WINDOWS row 59 as precedent.
3. **Exemption list size and gate scope.**
   - Recommendation: one exemption (`migration-guide.md`), fix `architecture-decisions.md:645`; optionally add `docker/docker-compose.server.yml` and `Dockerfile.server` to class 3.
4. **WINDOWS row 62:** close in this phase's final plan (all evidence is produced here) rather than handing to Phase 57; the row text must cite the D-03 boot test, the example changes and the F1 Docker changes. `.planning/WINDOWS.md` row 62 currently says "Owner: Phase 46 docs currency" and `open`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| python3 + `tomllib` | gate script | ✓ | 3.13.16 | — |
| PyYAML | `check-doc-config`, manifest parse-check | ✓ | 6.0.1 | — |
| shellcheck | script + harness lint | ✓ | 0.9.0 | — |
| actionlint | `ci.yml` edit | ✗ locally | — | CI job `actionlint` (v1.7.12) is the check; keep the step a copy of its neighbours |
| nightly-2026-09-20 + cargo-public-api 0.52.0 | `make api-surface` | ✓ | | — |
| cargo-release | D-16 version | ✓ | 1.1.7 | — |
| docker CLI | `docker-build-server` | CLI present (29.8.2), daemon not verified | | Use `make -n docker-build-server` for the recipe check |
| kubectl / kind / kubeconform | k8s manifest checks | ✗ | — | PyYAML parse plus assertions on volume and secret names |
| mdbook | docs build | ✗ | — | `make check-doc-config` for YAML fences; read links by hand |

**Missing dependencies with no fallback:** none. **With fallback:** actionlint, kubectl, mdbook, docker daemon (see table).

## Validation Architecture

Nyquist validation is enabled (`.planning/config.json` `workflow.nyquist_validation: true`).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust unit tests in `src/config/agents.rs`, bin tests in `src/bin/paladin-server.rs`); bash harnesses under `tests/scripts/*_test.sh` |
| Config file | none (workspace `Cargo.toml`); `Makefile` targets |
| Quick run command | `cargo test --lib config::agents::` ; `cargo test --features web-server --bin paladin-server build_auth_config` ; `./tests/scripts/check-version-strings_test.sh` |
| Full suite command | `cargo test` ; `make test-shell-guards` ; `make check-gates` ; `make clean-code` ; `make api-surface` ; `make security` ; `make check-doc-config` ; `make check-migration-allowlist` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FIRST-01 | `${` key refused naming the entry, never the value/variable; `$X` accepted; also when disabled; table rows | unit | `cargo test --lib config::agents::` | ❌ Wave 0 (new rows in existing module) |
| FIRST-01 | boot path refuses before bind | unit (bin) | `cargo test --features web-server --bin paladin-server build_auth_config_refuses` | ❌ Wave 0 |
| FIRST-01 | shipped example passes its own validator and carries no placeholder | unit | `cargo test --lib config::agents::example` | ❌ Wave 0 |
| FIRST-01 | no `${…}` key and no expansion claim in examples/k8s/docs | grep gate | `! rg -n 'key:\s*["'\'']?\$\{' config.example.yml k8s docs/src && ! rg -n 'expanded by the server' k8s config.example.yml` | n/a |
| FIRST-01 | k8s Secret template parses; both Deployments mount it; no configmap or `PALADIN_API_KEY_*` left | script | PyYAML parse + `rg -n 'paladin-server-config\|PALADIN_API_KEY_\|api-key-' k8s` returns nothing | ❌ (ad-hoc, record in evidence) |
| FIRST-02 | sweep complete | gate | `./scripts/check-version-strings.sh` exits 0 on the real tree | ❌ Wave 0 |
| FIRST-03 | gate red on a stale string, green when current, exempt file passes, zero-scan fails | bash harness | `./tests/scripts/check-version-strings_test.sh` (and via `make test-shell-guards`) | ❌ Wave 0 |
| FIRST-03 | wired in Makefile and CI | grep | `make -n check-gates \| rg check-version-strings` ; `rg -n check-version-strings .github/workflows/ci.yml Makefile` | n/a |
| FIRST-03 | red proof on the real tree | manual, recorded | edit one string (for example README pin to `"0.5"`), run `make check-version-strings`, expect exit 1 and `README.md:<n>: found '0.5', want '0.11.0'`, then `git checkout -- README.md` | n/a |
| FIRST-04 | k8s labels and image tags equal `v0.11.0`/`0.11.0` | gate | same script; `rg -n 'version: v\|image: paladin' k8s/server` | via gate |
| FIRST-04 | `make docker-build-server` also tags the version | dry run | `make -n docker-build-server` shows both `-t paladin-server:latest` and `-t paladin-server:0.11.0` | n/a |
| FIRST-05 | `[Unreleased]` anchors with corrective entries; `[0.11.0]` text untouched | grep + diff | `rg -n '^## \[Unreleased\]' crates/paladin-memory/CHANGELOG.md crates/paladin-content/CHANGELOG.md` ; `git diff -U0 -- crates/paladin-{memory,content}/CHANGELOG.md` shows additions only above `## [0.11.0]` ; `make check-changelogs` | n/a |
| FIRST-06 | ADR-0056 cites ADR-0057, no "Phase 42 closes/reserves" claim; tests.rs already cites it | grep | `rg -n 'reserv' .planning/decisions/0056-allowance-admission-model.md` (each hit reads as superseded) ; `rg -n 'ADR-0057' src/application/services/treasurer/tests.rs` | n/a |
| FIRST-07 | no stale counts; version line `1.1.7`; footer link gone | grep | `! rg -n 'ten (per-crate\|crate)\|all ten' release.toml scripts/finalize-crate-changelogs.sh .github/workflows/ci.yml` ; `rg -n 'cargo-release 1.1.7' release.toml` ; `! rg -n '^\[Unreleased\]:' CHANGELOG.md` | n/a |
| FIRST-07 | footer deletion breaks no script | script tests | `make test-shell-guards` (extract-changelog-section, finalize, release-consistency harnesses) | ✅ |
| FIRST-08 | snapshot current | gate | `make api-surface` (exit 0, "API surface unchanged") | ✅ |
| FIRST-08 | by-name clause reconciled | evidence | `rg -c 'UnsupportedParameter\|check_reasoning' .project/current-exports.txt` (0) plus the reconciled requirement text | n/a |
| FIRST-09 | cache section states the three things; links not restatements | grep | `rg -n 'attaches no node cache' docs/src/user-guides/fault-tolerance.md` ; `rg -n 'NODE_CACHE_KEY_VERSION\|k1' docs/src/user-guides/fault-tolerance.md` ; `rg -n 'key_prefix' docs/src/user-guides/fault-tolerance.md` ; `make check-doc-config` | ❌ Wave 0 (text) |

### Sampling Rate
- **Per task commit:** the quick-run command for the touched area (`cargo test --lib config::agents::`, the bin test, or the harness) plus `shellcheck` on any script touched.
- **Per wave merge:** `make test-shell-guards` and `make check-gates`; `cargo test` when Rust changed.
- **Phase gate:** full set in D-00a (including `make api-surface`, `make security`, `make clean-code`, coverage floor) green before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/scripts/check-version-strings_test.sh` (mode 100755) covering the cases in Pattern 3
- [ ] `scripts/check-version-strings.sh` (mode 100755)
- [ ] New test rows in `src/config/agents.rs` and `src/bin/paladin-server.rs`
- [ ] `54-FIRST-EVIDENCE.md` skeleton: prove-then-lock rows, the red run, the API-surface result, the F4 reconciliation
- Framework install: none.

## Security Domain

`security_enforcement` is not set to `false` in `.planning/config.json`, so this section applies.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Static API keys validated fail-closed at config load (`AuthConfig::validate`); a placeholder can never be a credential |
| V3 Session Management | no | No session change; the bearer-token store is untouched |
| V4 Access Control | yes (indirect) | Tenant mapping untouched; Phase 53 owns it. The new check runs before the tenant checks and does not alter them |
| V5 Input Validation | yes | Config-load validation (`validate`); table-tested; no value echoed |
| V6 Cryptography | no | No crypto change; no new secret-handling path (D-00e) |
| V7 Error Handling and Logging | yes | Message names the entry, never the key value or variable; backend text only to `log::error!` |
| V14 Configuration | yes | Secrets delivered via a Secret volume (`defaultMode` restrictive), not a ConfigMap; shipped examples fail closed |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| An unexpanded `${VAR}` literal accepted as the API key (any caller who knows the example string authenticates) | Spoofing / Elevation | Refuse any key containing `${` at load (FIRST-01); remove loadable placeholders from every example |
| Key value or variable name leaked into logs or errors | Information disclosure | Message names the `http.auth.api_keys[<name>]` entry only; test asserts neither appears; review per `.github/instructions/security.instructions.md` |
| Literal API keys in a world-readable ConfigMap | Information disclosure | Config moves to a Secret mounted with `defaultMode: 0440`, `items` projecting only `config.yml` |
| Docker first run silently accepting the placeholder as a credential | Spoofing | F1: the baked example now has no loadable key; docs say to mount a config with a literal key |
| Gate that scans nothing and passes | Repudiation (false assurance) | Zero-files and per-class zero-match named failures (the same "scanned nothing" lesson as Snyk/CodeQL in the security instructions) |
Known gap carried forward: `ApiKeyConfig` derives `Debug` and would print its `key` if a caller `Debug`-formats a config; the treasurer webhook `secret` placeholder is not refused (Deferred Ideas). Neither is in this phase.

## Sources

### Primary (HIGH confidence): repository reads and tool runs on 2026-10-31
- `src/config/agents.rs` (`AuthConfig::validate`, `ApiKeyConfig`, `WebHttpConfig`, tests), `src/bin/paladin-server.rs` (boot order, `build_auth_config`, tests), `src/config/settings.rs` (required legacy fields, `load_from_file`), `src/infrastructure/web/agent_host.rs` and `facade_provisioner.rs` (LLM key path), `crates/paladin-llm/src/config/{layers,bridge}.rs`
- `config.example.yml`, `Dockerfile.server`, `docker/docker-compose.server.yml`, `k8s/**`, `k8s/README.md`, `docs/src/deployment-topologies/http-service-host.md`, `docs/src/deployment/kubernetes.md`
- `scripts/check-crate-names.sh`, `check-dependency-allowlist.sh`, `finalize-crate-changelogs.sh`, `extract-changelog-section.sh`, `check-api-surface.sh`; `tests/scripts/*_test.sh`; `Makefile`; `.github/workflows/ci.yml`; `.github/rulesets/protect-main-branch.json`; `release.toml`
- `CHANGELOG.md`, per-crate changelogs, `MIGRATION.md` §10.1 M-B-12 and §11, `.planning/decisions/0056-…md`, `0057-…md`, `.planning/WINDOWS.md` rows 59, 62, 65, `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md` rows 13-14
- Git: commit `310829733` (0.11.0 changelog stamp), `git diff v0.10.1 v0.11.0` for `paladin-memory` and `paladin-content`
- Tool runs: `make api-surface` (exit 0, unchanged, 4325 items), `cargo release --version` (1.1.7), `shellcheck --version` (0.9.0), the four-class scan script (130 files, 73 raw hits, 49 to fix + 21 exempt + 1 judgement + prose/non-version lines excluded)

### Secondary (MEDIUM confidence)
- none

### Tertiary (LOW confidence)
- Kubernetes Secret volume semantics (`defaultMode`, `fsGroup`, `items`), from training knowledge: tagged `[ASSUMED]` (A1)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new dependency; versions read from the container
- Architecture: HIGH, every insertion point and ordering was read from source
- Pitfalls: HIGH for code/gate/script pitfalls; MEDIUM for Kubernetes Secret mode details (A1) and the Phase 58 dirty-tree behaviour (A3)

**Research date:** 2026-10-31
**Valid until:** 2026-11-14 (the tree moves each phase; re-run the four-class scan and `make api-surface` at plan time, per prove-then-lock)
