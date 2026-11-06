# Phase 54: First-Run Usability - Context

**Gathered:** 2026-10-31
**Status:** Ready for planning
**Mode:** `--auto` (recommended option selected for every question, single pass; no SPEC.md for
this phase, so the roadmap's five success criteria and FIRST-01..09 are the requirement of record)

<domain>
## Phase Boundary

A new user who copies from `README.md`, `docs/src`, `config.example.yml` or `k8s/` gets a working,
honest first run:

- An `http.auth.api_keys[].key` holding an unexpanded `${…}` placeholder is refused at config load
  with an error that names the entry.
- No shipped example loads a placeholder as a credential, and no example says the server expands
  one.
- Every install snippet, `Current version` line, `k8s/` `version:` label, first-party image tag and
  `openapi.json` version reads the workspace version, and a `check-version-strings` gate in
  `make check-gates` and CI keeps them that way.
- These nits are corrected: the `paladin-memory`/`paladin-content` changelog entries, the
  ADR-0056 reservation claim, the stale crate counts, the `cargo-release` version line, the
  dangling `[Unreleased]` link, the committed API-surface snapshot, and the node-cache guide.

Requirements FIRST-01..09 (inventory rows B1, B10, B12, C1-C8, C10; DF-01, DF-02; WINDOWS row 62).
Out of scope: any `${VAR}` expansion path, new ports, adapters or HTTP routes, wiring a node cache
into the server, and the lockstep bump to `0.11.1` (Phase 58).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by the milestone open, Phases 52-53 and the roadmap — not re-asked)

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

### Placeholder refusal and honest examples (FIRST-01; B1, C3, DF-01, WINDOWS row 62)

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

### Version-string gate and the currency sweep (FIRST-02, FIRST-03, FIRST-04; C1, C2, C4, DF-02)

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

### Changelog, ADR and count corrections (FIRST-05, FIRST-06, FIRST-07; C5-C8)

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

### API surface snapshot and the node-cache guide (FIRST-08, FIRST-09; C10, B10, B12)

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements of record
- `.planning/ROADMAP.md` "Phase 54: First-Run Usability": the goal, five success criteria and
  scope notes (exact workspace version, snapshot refreshed again by later phases, `${VAR}`
  expansion out).
- `.planning/REQUIREMENTS.md` "First-Run Usability (FIRST)": FIRST-01..09 text, and the "Out of
  Scope" table (C11 archival docs out).
- `.planning/PROJECT.md` "Current Milestone: v0.11.1 Dogfood Cleanup": decisions at open (B1
  refuse-at-boot, B10 document, B12 keep and document) and the scope guards.

### Phase input of record
- `.project/v0.11.1/01-carried-gaps-inventory.md` §B rows B1, B10, B12 and §C rows C1-C8 and C10
  (C11 out): the sources and dispositions.
- `.project/v0.11.1/02-dogfood-findings-log.md` DF-01 and DF-02: repro commands; the
  "Surfaces worth deliberately exercising" list.
- `.project/v0.11.1/03-suggestions-and-scoping.md`: the decision table rows for B1, B10 and B12.
- `.planning/WINDOWS.md` row 62: the closing condition this phase satisfies.

### Governing decisions and frozen records
- `.planning/phases/53-tenant-isolation-closure/53-CONTEXT.md` D-00a, D-00b, D-00e: gates, semver
  and frozen records, carried forward as D-00a..c.
- `.planning/phases/52-ci-release-pipeline-repair/52-CONTEXT.md` D-01, D-03, D-06: the
  patch-level semver gate, the §11.2 register, and how the gate scripts are wired into CI.
- `MIGRATION.md` §11.1, §11.5: the behavioral and configuration entries for FIRST-01 and the k8s
  move (D-00b, D-05). §10.1 M-B-12 is the persisted-key change FIRST-05 cites. §9 and §10 are
  frozen.
- `.planning/decisions/0056-allowance-admission-model.md`: the lines amended under D-14.
- `.planning/decisions/0057-mid-run-halt-contract.md`: the check-only boundary ADR-0056 must cite.
- `.planning/decisions/0058-rate-pacing-cadence.md`: the stampede lock (library builder only)
  behind D-19.
- `.planning/decisions/PROMOTION.md`: next free ADR number 0064; no new ADR expected.
- `.github/instructions/security.instructions.md`: the manual credential review that governs the
  D-02 message (never print a key value).

### v0.11.0 phase evidence
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md`: rows 13-14
  (`cargo-release 1.1.7`, D-16) and D-00d (the stale k8s labels and counts).
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-REVIEW.md`: WR-01 (crate
  changelog wording) and WR-02 (the dangling link).
- `.planning/milestones/v0.11.0-phases/42-mid-run-halt-sse-terminal-status/deferred-items.md`:
  42-02 (the ADR-0056 reservation claim, C6).
- `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/deferred-items.md`: WR-04
  and IN-02 (the cache key and provider default effort, B12).
- `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md`: G4 (the snapshot lag, C10) and the Phase 43
  tech-debt line (the stampede lock, B10).

### Code and files this phase edits
- `src/config/agents.rs`: `AuthConfig::validate` and its table tests (D-01..D-03).
- `src/bin/paladin-server.rs`: the boot-refusal test harness near the "auth enabled with no API
  keys" test (D-03); `config_path` and `Settings::load_from_file` are read-only context for D-05.
- `src/config/treasurer.rs`: the `yaml_env_placeholder_is_not_expanded` pin, read-only.
- `crates/paladin-llm/src/config/layers.rs`: `resolve_secret_placeholder` / `placeholder_name`,
  the exact `${NAME}` mechanism (read-only; it is what the D-04 comment describes).
- `config.example.yml`: the http.auth block (~170-198), the LLM `${NAME}` header comment (~16-22)
  and the treasurer webhook note (~308) (D-04). Every test that loads this file must stay green:
  `src/infrastructure/web/agent_host.rs`, `src/config/web_server.rs`, `src/config/treasurer.rs`,
  `src/config/agent_runtime.rs`, `crates/paladin-llm/src/config/llm.rs`.
- `k8s/server/configmap.yaml`, `secret.yaml.example`, `deployment.yaml`,
  `worker-deployment.yaml`, and `k8s/README.md` (D-05, D-09, D-10). `k8s/deployment.yaml` gets
  its label only.
- `README.md` (~108, ~183), `docs/src/appendix/sentinel.md`,
  `docs/src/api-reference/crate-map.md`, `docs/src/api-reference/feature-flags.md`,
  `docs/src/getting-started/quickstart.md`, `docs/src/getting-started/installation.md`, and
  `docs/src/user-guides/{paladin-agents,battalion-patterns,maneuver-flow-dsl}.md` (D-09).
- `scripts/check-version-strings.sh` (new), `tests/scripts/check-version-strings_test.sh` and
  fixtures (new), `Makefile` (`check-gates`, `check-version-strings`, `docker-build-server`), and
  `.github/workflows/ci.yml` (one step) (D-10, D-11). Siblings to copy:
  `scripts/check-crate-names.sh` and `scripts/check-dependency-allowlist.sh`.
- `crates/paladin-web/openapi.json`: `info.version`, read by the gate; it is regenerated only by
  `make openapi` or the release recipe.
- `crates/paladin-memory/CHANGELOG.md` and `crates/paladin-content/CHANGELOG.md` (D-13).
- `.planning/decisions/0056-allowance-admission-model.md` and
  `src/application/services/treasurer/tests.rs` (D-14).
- `release.toml`, `scripts/finalize-crate-changelogs.sh` and `.github/workflows/ci.yml` comments
  (D-15, D-16); the `CHANGELOG.md` footer (D-17).
- `.project/current-exports.txt` and `scripts/extract-public-api.sh` / `check-api-surface.sh`
  (D-18).
- `docs/src/user-guides/fault-tolerance.md` (the cache section),
  `docs/src/user-guides/reasoning-effort.md` "The node cache",
  `docs/src/getting-started/configuration.md` `lock_ttl_secs`, and
  `crates/paladin-battalion/src/engine/cache_key.rs` (`NODE_CACHE_KEY_VERSION`, read-only)
  (D-19).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AuthConfig::validate`: its fail-closed, first-offender-in-declaration-order shape and its
  message forms (`http.auth.api_keys[#<index>]` before the name is usable, `[<name>]` after).
  Phase 53's reserved-literal rows are the newest example of adding a refusal; D-02 slots in
  beside them.
- The `paladin-server.rs` boot test for "auth enabled with no credentials", the model for D-03's
  boot refusal test.
- `resolve_secret_placeholder` (`paladin-llm`): the exact `${NAME}` grammar, including the test
  `only_an_exact_placeholder_is_a_placeholder`. Reference only; it stays `pub(crate)`, and D-01
  deliberately uses a broader rule for refusal.
- `scripts/check-crate-names.sh`: offline `tomllib` manifest parsing, set and report style, and
  named failures. The template for `check-version-strings.sh`.
- `tests/scripts/*_test.sh` with `tests/scripts/fixtures/`: the regression-test pattern that
  `make test-shell-guards` auto-discovers (a zero-match glob is itself a failure), run in CI at
  `ci.yml` ~218.
- `scripts/check-release-consistency.sh`: enumerates publishable packages via `cargo metadata`.
  It is the lockstep guard; D-06 does not duplicate it.

### Established Patterns
- Operator messages name the config entry and never a secret. Backend text goes only to
  `log::error!`.
- Gates are offline, take no argument, finish in about a second, report every finding before
  exiting, and treat "scanned nothing" as a named failure. Each is a Makefile target in
  `check-gates` plus its own CI step. They are checks, not fixers.
- Measured, not assumed (prove-then-lock; D-00f). Evidence files record the command and its
  output.
- The shipped `config.example.yml` must not refuse its own `llm`/`agents` sections
  (T-50-10-03, `agent_host.rs`). D-03 extends that to the new auth rule.
- Frozen published records are corrected forward in `[Unreleased]`, never edited in place.
- An ADR is amended in place with a dated note, and the PROMOTION line advances only for a new
  number.

### Integration Points
- The server reads exactly one file: `PALADIN_CONFIG`, else `argv[1]`, else `config.yml`, through
  `Settings::load_from_file`, with no `APP_` environment layer at that point. That is why D-05 puts
  the whole config into a Secret rather than layering it.
- The `make release` recipe runs `release-check` (no `check-gates`) before the bump and
  `check-release-consistency.sh` after it. `release.yml` refuses to publish a tag whose commit
  lacks a green `ci.yml` run, which is where a stale string blocks a release (D-12).
- `make docker-build-server` (`Makefile` ~522) is the build `k8s/README.md` documents; D-10 makes
  its tag match the manifests.
- Phase 57's review sweep and Phase 58's pre-tag audit read this phase's VERIFICATION and evidence.
  Phase 58's lockstep bump is the gate's first real red-then-green cycle.

</code_context>

<specifics>
## Specific Ideas

- The FIRST-01 message reads like its neighbours in `AuthConfig::validate`, for example
  `http.auth.api_keys[ci-runner]: 'key' is an unexpanded '${…}' placeholder -- the server does
  not expand environment variables in http.auth.api_keys; set the literal key value`. The wording
  is the planner's; the shape is not.
- The gate's report is one line per finding, so Phase 58's bump PR can work through it as a
  checklist.
- **For Phase 58 (not done here):** only `paladin-memory` and `paladin-content` gain a
  `## [Unreleased]` anchor in this phase. `finalize-crate-changelogs.sh` rule 3 records a named
  failure for any changelog that has no anchor and no section for the target version, so the
  other nine crate changelogs need an anchor, or a section, before the `0.11.1` finalize. The
  researcher confirms how the `0.11.0` finalize handled this.
- The `k8s/README.md` quick start must still work as a copy-paste sequence after D-05 and D-10:
  build the image, copy and fill the Secret template, `kubectl apply`, then probe `/health`.

</specifics>

<deferred>
## Deferred Ideas

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

### Reviewed Todos (not folded)
`[auto] todo.match-phase returned the same four matches at score 0.6 as Phase 53. The auto-fold
rule (score ≥ 0.4) was overridden on the Phase 53 precedent: every match is keyword-only
("make, docs, src, guide, yml"), and folding any of them would widen the phase against the
milestone scope guard.`
- `2026-08-13-verify-local-coverage-reproduction.md`: assigned to Phase 56
  (`resolves_phase: 56`).
- `2026-10-13-review-content-data-orchestration-base-items.md`: the base items in
  `paladin-core`; belongs to Phase 55 (G3).
- `2026-10-14-web3sec-single-bump-to-v0-11-0.md`: the downstream consumer's own step.
- `2026-10-20-openai-responses-api-for-pro-tier.md`: ruled out at milestone open.

</deferred>

---

*Phase: 54-first-run-usability*
*Context gathered: 2026-10-31*
