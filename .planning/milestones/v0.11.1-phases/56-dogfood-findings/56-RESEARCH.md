# Phase 56: Dogfood Findings - Research

**Researched:** 2026-11-02
**Domain:** Release-hardening tooling for a Rust workspace: a findings-log checker (bash + embedded python3), a config-loader round-trip test (`config` crate + `serde`), a published-crate first-run walk, and human-gated coverage and consumer-return checkpoints.
**Confidence:** HIGH for the tree-derived answers (every claim below was re-derived from the tree, from crates.io, or from a scratch run on 2026-11-02); MEDIUM for the Docker/coverage runbook details, which cannot be run in this sandbox.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Copied verbatim in substance from `56-CONTEXT.md` `## Implementation Decisions` (D-00a..D-17). The planner MUST honour all of them. The headline ones:

- **D-00a** Gates: 82 % workspace line coverage (ADR-0006); `make clean-code` (includes `make doc-check`), `make check-gates`, `make test-shell-guards`; `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional public-surface change); `make security`, `make check-migration-allowlist`, `make check-version-strings`; `make check-doc-config` for any fenced YAML touched in `docs/src`; `shellcheck`/`actionlint` for every script and workflow edit; the manual credential-handling review in `.github/instructions/security.instructions.md`; no `unwrap`/`expect`/`panic!` in library code; TDD (every fix watched failing first; a finding's reproduction becomes its regression test).
- **D-00b** Semver: every public change judged at `--release-type patch` against `0.11.0` for all twelve packages; milestone stays `v0.11.1`; allowlist stays zero entries; §11.2 stays header-only; a flagged change is reshaped to additive or dropped; only an S1 security fix with no additive shape may force `v0.12.0` (recorded at the point the break is known). Behavioural changes go in `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]`. No `#[deprecated]`.
- **D-00c** `MIGRATION.md` §9/§10 and every `[0.11.0]` changelog section are byte-frozen; logging stays on the `log` facade; if an ADR is needed it takes **0064** (`.planning/decisions/PROMOTION.md`) and advances that line in the same commit.
- **D-00d** Scope guards: no new ports, adapters or HTTP routes except for a security fix; stubs fenced, not finished; a finding that collides with an accepted contract (WINDOWS 64, 65, 66, 61; audit G1; FUT-01..13; `TokenUsage.total_tokens`) is logged with its reference and dispositioned `out`; provider measurement work only if a run hands over a captured body; OpenAI Responses API is out.
- **D-00e** Prove-then-lock: re-check every finding against the current tree before promotion; a stumble on published `0.11.0` that HEAD already fixes is recorded with the fixing phase, requirement and proving command, and not re-fixed. Locate items by content, never by line number.
- **D-01** Append-only dispositions: a disposition is a new dated bullet appended under the entry (`- **Disposition (2026-11-xx):** in v0.11.1 — fixed by …`); original lines stay byte-intact; an entry's disposition is its **last** `**Disposition…:**` bullet; new findings go below `<!-- append new findings below this line -->`, take the next free `DF-nn`, and use the entry format at the top of the file.
- **D-02** DF-01..DF-04 close as `in v0.11.1` with a pointer to the owning requirement + proving test/gate (not `duplicate of`); each is re-checked on HEAD first (D-00e); no live dogfood run is needed for these four.
- **D-03** "None reads `open`" is checked by a standalone offline script (name is the planner's, e.g. `scripts/check-findings-log.sh` + `make check-findings-log`) with a `tests/scripts/…_test.sh` fixture test wired into `make test-shell-guards`. It fails naming each id that is `open`, missing, or not one of `in v0.11.1` / `out (<reason>)` / `duplicate of DF-nn`; checks `duplicate of` targets exist and ids are unique and increasing. Run at phase verification and by Phase 58 SHIP-10. **Not** in `make check-gates` or CI.
- **D-04** Severity as written in the log (S1 security/isolation · S2 wrong result or crash · S3 misleading docs/config · S4 hygiene), graded at the worst credible deployment; a web3sec divergence that breaks the consumer's compile or tests is S2 by default; a muster gap with no behavioural effect is S3.
- **D-05** S1/S2: always `in v0.11.1` unless it collides with an accepted contract (`out` + reference) or does not reproduce on HEAD (`out (fixed by Phase N — <req>)` or `duplicate of`). S3/S4: `in v0.11.1` only when contained (docs/example change or single-file code change riding an existing plan); otherwise `out (re-homed: <todo file>)` with a pending todo in `.planning/todos/pending/`.
- **D-06** S1/S2 fixes land inline (a `--gaps` replan) by default; a decimal phase (`56.1`+) only when the fix needs its own design discussion. Every fix commit cites its `DF-nn`.
- **D-07** Three stages: **A** agent-runnable (DF-01..04 closure + checker; DOGF-03 round-trip; keyless half of DOGF-04), **B** human-gated checkpoints (keyed half of DOGF-04, DOGF-05, DOGF-02 triage), **C** fix plans written from the log. Stage A must not wait on Stage B. The phase closes only when Stage C is empty and the D-03 checker is green.
- **D-08** The round-trip test goes through the production loader (`Settings::load_from_file`), never raw `serde_yaml`; every other production path that reads an example section is also covered (enumerated in Answer 1 below).
- **D-09** "Every optional key present" comes from a committed full fixture plus a drift-guard test; the example itself is NOT uncommented. Dummy credentials are obviously fake (`test-key-not-a-secret`), never `${…}`. "Each section" = each top-level section present in `config.example.yml`.
- **D-10** "Silently ignored" is detected by key-path comparison of the input against the re-serialised `Settings`; map keys compared exactly and case-sensitively; deliberately-unserialised fields go in an explicit exemption list, one reason per row; a key the loader rejects fails the load; each rejected/ignored key is its own `DF-nn`; the test asserts zero unexplained keys with a `KNOWN_GAPS` table whose rows cite `DF-nn` and which fails when a row stops being a gap. `serde_ignored` is an allowed substitute only if re-serialisation is lossy in a way the exemption list cannot express (it is not — see Answer 2).
- **D-11** Fixture rows probe known bug shapes: mixed-case/punctuated map keys (`Acme-Prod`, model ids with `/` `.` `-`), numeric strings for decimal money (`"2.50"`), both `temperature` override shapes (`{fixed: …}`, `{range: […]}`) and `not_sent`, and an `api_keys` entry with a literal key.
- **D-12** `APP_…` environment overlays are out of this test.
- **D-13** The walk runs against crates.io `0.11.0` in a scratch crate outside the workspace (no `[patch]`, no workspace inheritance); steps follow the README in order; each command, exit code and redacted output goes in a phase evidence file; every stumble is a `DF-nn` triaged under D-00e.
- **D-14** The agent runs the keyless half; the maintainer runs the keyed half from a runbook (`checkpoint:human-action`); the key never enters the repo, a log or the evidence file; provider = the one the README Quick Example uses.
- **D-15** No web3sec return is simulated, inferred or reconstructed; triage starts only from the consumer's actual §6 return (SHA `70657919beadbd341a72be2d0107080017a803f0`, D-2 choice, gate totals vs 2,683 / 0 / 2, divergences); one `DF-nn` per divergence with **By:** web3sec.
- **D-16** If the return has not arrived when everything else is done, a `checkpoint:decision` asks the maintainer to hold the tag or re-home DOGF-02 with a named owner and a date (recorded in the requirement row and in todo `2026-10-14-web3sec-single-bump-to-v0-11-0`); never an agent default.
- **D-17** Coverage pass rule: whole-percent agreement with CI on the same commit, both ≥ 82 (figures truncate toward zero); the maintainer walks `docs/src/contributing/testing-guide.md` verbatim (`make services-up` → `make coverage`); every undocumented prerequisite is an S3 `DF-nn` + contained docs fix; the todo moves to `.planning/todos/completed/` with local figure, CI run id and commit.

### Claude's Discretion

- Script, make-target, fixture and evidence-file names (D-03, D-09, D-13), and how D-10's key-path walker is written (`serde_json::Value` walk, `serde_ignored`, or similar).
- Whether DOGF-03 is one test per section or one table-driven test, and where it lives (`src/config/` unit tests or a `tests/` integration test). The existing per-section `config.example.yml` tests in `web_server.rs`, `agents.rs`, `treasurer.rs`, `agent_runtime.rs` and `agent_host.rs` stay; the new test supplements them.
- Runbook layout for the Stage B checkpoints.
- How DF entries for one walk are grouped into plans for the Stage C replan.

### Deferred Ideas (OUT OF SCOPE)

- Environment-overlay (`APP_…`) round-trip (D-12).
- Turning `deny_unknown_fields` on across config sections (milestone-level call; a fix for one finding may be a targeted rename/alias only).
- Live probes of Ollama / OpenAI-compatible and walks on non-README providers (unless a run hands over a captured body).
- Reviewed, not folded: `2026-10-20-openai-responses-api-for-pro-tier` (conflicts with the 2026-10-27 milestone decision); a pro-tier 404 in a walk is logged `out (milestone decision 2026-10-27; todo 2026-10-20-…)`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DOGF-01 | Every `DF-nn` entry carries a disposition; none reads `open` | Answer 4 (checker parse rules, shell-gate shape, `make test-shell-guards` wiring); Answer 7 (proving commands for DF-01..04) |
| DOGF-02 | web3sec §6 return triaged, one entry per divergence, or DOGF-02 re-homed with owner + date | Answer 8 (no return exists in the tree as of 2026-11-02); §6 contract quoted; checkpoint:decision path |
| DOGF-03 | Each `config.example.yml` section round-trips through the loader with every optional key present, as a passing test; rejected/ignored keys logged | Answers 1, 2, 3 (production paths, re-serialisation faithfulness, `config` 0.15.23 key handling, proven by a scratch run against the real `Settings`) |
| DOGF-04 | First-run path walked on published `0.11.0` crates; every stumble logged | Answer 5 (crate names, README text, install route, runtime needs, crates.io reachable) + "Leads already reproduced" |
| DOGF-05 | Maintainer's local `make coverage` walk closes todo `2026-08-13-verify-local-coverage-reproduction` with the measured figure | Answer 6 (exact procedure, script, CI job, mismatches the runbook must handle) |
| DOGF-06 | Every finding promoted `in v0.11.1` at S1/S2 fixed and verified before the phase closes | Plan-shape guidance, semver/gate checklist, Validation Architecture |
</phase_requirements>

## Summary

This phase is a frame around unknown content, so the research is about making the frame correct, cheap to run and honest. Eight findings drive the plan:

1. **Only three production call sites read a config file**: `Settings::load_from_file` in `paladin-server` and in the `paladin` binary, and `Settings::new` in `paladin-cli arsenal`. `paladin-llm` has **no** production file loader (every `config::Config::builder()` there and in the `deserialize_wrapper_from_file` helpers sits after the first `#[cfg(test)]` in its file). So the DOGF-03 "each production path" requirement collapses to one real path (`Settings::load_from_file`) plus the `Settings::new` variant whose only difference is the `APP_` overlay (out of scope, D-12). Every top-level example section is a `Settings` field.
2. **Re-serialising `Settings` is faithful**. No config type has a custom `Serialize` impl, `flatten`, `untagged`, or `alias` on a key the example uses. Two fields are `skip_serializing` and need exemption rows: `llm.<provider>.api_key` and `treasurer.allowance.webhook.secret`. `serde_ignored` is **not** needed (not in `Cargo.lock`; it would pass `deny.toml`, but adds a package for no gain). Proven by a scratch run against the real `Settings`: the verbatim example drops exactly 8 `api_key` paths and nothing else; a full fixture drops exactly 9; planted typos are all caught.
3. **The `config` crate (locked 0.15.23, not 0.15.11 as CONTEXT says) preserves map keys exactly** from a YAML file: case, `.`, `/`, `-`, brackets, spaces. D-11's feared lowercasing/path-splitting does not happen on the file path. Two real behaviours worth pinning instead: an unquoted YAML decimal into a `String` field loses trailing zeros (`2.50` becomes `"2.5"`, `10.00` becomes `"10"`), and `yes` is a YAML-1.1 boolean.
4. **The README Quick Example uses `MockLlmAdapter`, so the entire README path is keyless.** The only keyed steps in the walk are `paladin-cli onboarding` and `setup-check`, plus the Quickstart's `OpenAIAdapter::from_env()` variant. D-14's "provider the README Quick Example uses" is therefore OpenAI (`OPENAI_API_KEY`, the README's "for real use" comment and onboarding's first option). Also: there is no `basic_paladin` command for a crates.io user; `cargo run --example` needs a repo checkout.
5. **`paladin-cli` ships in the `paladin-ai` package behind `required-features = ["cli"]`.** `cargo install paladin-ai --version 0.11.0 --bin paladin-cli` fails with exit 101 (`target paladin-cli … requires the features: cli`); reproduced here. Nothing in the README or `docs/src` documents any `cargo install` of the CLI.
6. **crates.io is reachable from this sandbox** (`cargo search`, `cargo add`, `cargo metadata`, `cargo install` and the JSON API all worked), so the keyless half is agent-runnable in principle. **Disk is the constraint**: the volume held 4.2 GB free after cleanup, and two scratch compiles of the dependency tree filled it to 0 MB during this research. A published-crate compile must use a roomy `CARGO_TARGET_DIR` or be handed to the maintainer.
7. **No Docker daemon here** (CLI 29.8.2 and Compose v5.6.0 are installed, but there is no `/var/run/docker.sock`), so DOGF-05 is not agent-runnable. The coverage runbook has three traps the testing guide does not mention (Answer 6): `make coverage` prints no percentage, `make services-up` exposes different host ports than `scripts/coverage.sh` probes, and CI pins `cargo-llvm-cov@0.8.7` while the guide says an unpinned `cargo install`.
8. **No web3sec return exists** anywhere in the tracked tree or `.planning/` after 2026-10-26; the only post-2026-10-26 web3sec commits are Paladin's own outbound hand-off (`ae6dcbb5a`, `18843ae2f`).

Several defects were found or reproduced while researching (see "Leads already found"). They are **leads for Stage A to reproduce and log**, not log entries; the log is the maintainer's record and D-00e still applies.

**Primary recommendation:** Build Stage A as five small plans in this order: (1) the findings-log checker with its fixture test, red against the current log; (2) the DF-01..DF-04 re-check and appended dispositions, turning the checker green; (3) the config round-trip test with a derived full fixture, a drift guard and a `KNOWN_GAPS` table; (4) the keyless published-crate walk with its evidence file; (5) the Stage B runbooks with `checkpoint:human-action` tasks and a final `checkpoint:decision` for DOGF-02. Write no Stage C plan until the log has entries to fix.

## Architectural Responsibility Map

This phase adds tooling and tests, not product capability. Tiers are the repo's real boundaries.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Findings-log parsing and the "no `open`" gate | Repo tooling (`scripts/`, `tests/scripts/`, `Makefile`) | CI (only `make test-shell-guards` runs the fixture test) | House shell-gate pattern; the checker is deliberately not in `check-gates`. |
| Config file load and key handling | Root crate library (`src/config/settings.rs`, `config` crate) | `paladin-llm` (types only, no loader) | `Settings::load_from_file` is the single production loader; `LlmConfig` reaches it as `Settings.llm`. |
| Config round-trip test and full fixture | Root crate `tests/` (integration test) + `tests/fixtures/config/` | — | Needs the real loader and the checked-in example; a top-level `tests/*.rs` file is auto-discovered (no `autotests = false`). |
| Server boot consumption of config | `src/bin/paladin-server.rs`, `src/infrastructure/web/agent_host.rs`, `facade_provisioner.rs`, `run_api_wiring.rs` | `paladin-web` | Where a parsed key is applied or silently unused. |
| First-run walk on published crates | Scratch crate outside the workspace (crates.io) | Maintainer machine (keyed half) | Must not use path deps, `[patch]` or the workspace. |
| Local coverage walk | Maintainer machine with Docker | CI `coverage` job (comparison figure) | No Docker daemon in agent environments. |
| web3sec return triage | Maintainer relay to the consumer | `.project/v0.11.1/02-dogfood-findings-log.md` | The consumer's workspace is not in this repo; nothing can be inferred. |

## Standard Stack

### Core

No new runtime dependency. Everything needed is already in the tree.

| Library / tool | Version | Purpose | Why standard |
|----------------|---------|---------|--------------|
| `config` | **0.15.23 locked** (manifest `0.15.11`) | The production loader under test | Already `Settings::load_from_file`'s engine [VERIFIED: Cargo.lock line `name = "config"`]. |
| `serde_json` | workspace `1.0` | `to_value(&settings)` and the key-path walk | Already a root dependency. |
| `serde` derive | workspace `1` | `Settings: Serialize` | Already derived. |
| bash + `python3` (stdlib only, `tomllib`/`re`) | python 3.13.16 here | Findings-log checker | House gate shape: `check-version-strings.sh`, `check-no-stub-panics.sh`. Stdlib only. |

### Supporting

| Library | Version | Purpose | When to use |
|---------|---------|---------|-------------|
| `serde_yaml` | `0.9` (locked `0.9.34+deprecated`) | Independent parse of the **input** fixture into `serde_json::Value` | Recommended as a `[dev-dependencies]` entry in the root crate. It is already in `Cargo.lock` (optional root dep for `cli`; unconditional in `paladin-eval`), so `make deny` is unaffected. It removes a blind spot: if the input paths came from the same `config` crate, a key mangled at parse time would look identical on both sides. Fallback if the planner wants zero manifest change: parse with `config` into `serde_json::Value` (works; keys preserved). |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Key-path diff | `serde_ignored` 0.1.14 | Works with `config::Config` as a `Deserializer`, MIT OR Apache-2.0, 63 M downloads, dtolnay [VERIFIED: crates.io API]. **Not in `Cargo.lock`** [VERIFIED: grep], so it would be a new package and `cargo-deny`/`cargo-audit` surface. It only reports keys unknown to a struct, not known-but-lost keys, and it cannot see inside the typed maps. The key-path diff needs two exemption rows and no new package. Do not add it. |
| `serde_json::to_value` of f32 | Value comparison | `temperature: 0.7` (f32) re-serialises as `0.699999988…`; comparing values would false-positive. Compare key paths only (D-10). |

**Installation:** none for the checker; for the optional input parser add to the root `Cargo.toml`:

```toml
[dev-dependencies]
serde_yaml = "0.9"
```

**Version verification** (run 2026-11-02):

```bash
grep -n -A2 'name = "config"' Cargo.lock        # 0.15.23
curl -sS https://crates.io/api/v1/crates/serde_ignored   # max 0.1.14, MIT OR Apache-2.0
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| serde_yaml | crates.io | 10 yrs (2016) | ~7.6M/wk | github.com/dtolnay/serde-yaml | OK | Approved (already in `Cargo.lock`; deprecated-upstream label is the `+deprecated` suffix, tool reports `deprecated: false`) |
| serde_ignored | crates.io | 9 yrs (2017) | ~1.5M/wk | github.com/dtolnay/serde-ignored | OK | **Not recommended** (new lock entry for no gain); passes the legitimacy check if the planner overrules |

Source: `gsd-tools query package-legitimacy check --ecosystem crates serde_ignored serde_yaml` returned `OK` for both [VERIFIED: gsd-tools seam, 2026-11-02]. No postinstall concept applies to crates.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                      .project/v0.11.1/02-dogfood-findings-log.md   (append-only; DF-01..04 + new)
                                   ^   ^    ^        |
   Stage A (agent, now)            |   |    |        v
 +--------------------------+      |   |    |   scripts/check-findings-log.sh  --> exit 0/1, one line per bad DF-nn
 | 56-01 checker + fixture  |------+   |    |        ^
 | 56-02 DF-01..04 dispose  |----------+    |        | run at verification + SHIP-10 (NOT check-gates)
 | 56-03 config round trip  |-- finds ->-----+
 | 56-04 keyless walk       |-- stumbles -->-+
 +--------------------------+                |
                                             |
   config.example.yml --> [full fixture + drift guard] --> Settings::load_from_file (config 0.15.23)
                                                                |
                                         serde_json::to_value(&settings) --> key paths
                                                                |
                         input key paths (serde_yaml) --diff--> MISSING  --> KNOWN_GAPS rows (cite DF-nn) | exemption rows

   Stage B (maintainer, human-gated)                     Stage C (agent, after A+B)
 +-----------------------------------+                  +-------------------------------+
 | keyed walk: onboarding/setup-check|--- stumbles ----->| plans written from the log:   |
 | local make coverage vs CI figure  |--- DF-nn  ------>| S1/S2 fixed + regression test |
 | web3sec §6 return triage          |                  | decimal 56.x only if design   |
 | decision: hold tag or re-home     |                  +-------------------------------+
 +-----------------------------------+                       phase closes only when empty
                                                             and the checker is green
```

### Recommended Project Structure

```
scripts/check-findings-log.sh              # bash + embedded python3, no arguments (name is the planner's)
tests/scripts/check-findings-log_test.sh   # executable; fixture trees under mktemp -d; auto-picked by test-shell-guards glob
tests/config_example_roundtrip.rs          # auto-discovered integration test (top-level tests/*.rs)
tests/fixtures/config/config.example.full.yml          # derived, hand-maintained
tests/fixtures/config/README.md            # add a provenance paragraph: derived, NOT a frozen tag copy
.planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md   # evidence file (D-13); redacted
.planning/phases/56-dogfood-findings/56-RUNBOOK-*.md        # Stage B runbooks
```

### Pattern 1: The shell-gate shape to copy (Answer 4)

Copy `scripts/check-version-strings.sh` (306 lines, the smaller of the three) and its test:

- `#!/usr/bin/env bash`, long header comment stating the rule, `set -euo pipefail`, `WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"`, a **test-only environment seam** (`VERSION_STRINGS_ROOT`, `STUB_PANICS_ROOT`) that relocates every input, python3 existence check, `REPORT=$(python3 - "${ROOT}" <<'PY' … PY)`; first output line is a status word (`OK`, `MISMATCH`, or a named failure), the rest are `path:line: …` details; the shell wrapper prints `✅`/`❌` and exits 0/1 [VERIFIED: read both scripts].
- **Never vacuous**: a named failure for "scanned zero" (`ZERO_FILES`, `NO_PACKAGES`), unreadable input (`UNREADABLE`), a stale exemption. The checker needs `ZERO_ENTRIES` (no `### DF-nn` heading found) and `UNREADABLE`.
- Output sorted so two runs on one tree are byte-identical.
- Script is `100755` in git; `make lint-shell` runs `shellcheck --severity=warning` over every tracked `*.sh` [VERIFIED: Makefile `lint-shell`].
- Test harness (`tests/scripts/check-no-stub-panics_test.sh`, 965 lines): `set -uo pipefail`, one `mktemp -d` scratch removed by a `trap`, `make_tree`/`fresh`/`plant`/`run_guard`/`assert_fire`/`assert_silent` helpers, failing cases first, then named failures, determinism, a no-mutation checksum of the real inputs, passing cases, the real tree, a final no-mutation assertion; prints `✅ N assertion(s) passed.` and exits 0, else `❌ F/N assertion(s) failed.` [VERIFIED: read].
- `make test-shell-guards` runs **every** `tests/scripts/*_test.sh` by glob (an empty glob is a named failure), invoking each file directly (`"$f"`), so the new test file **must be executable** and needs no Makefile edit. CI runs `make test-shell-guards` (`.github/workflows/ci.yml` step at line ~232) [VERIFIED: Makefile + ci.yml].

**Consequence for D-03 that the house pattern does not cover:** the house test ends with a "the real tree passes" assertion. The checker's real input (the findings log) legitimately holds `open` entries mid-phase, and the shell test runs in CI. So the real-log case must assert only "the script ran, parsed ≥ 4 entries, and did not report `ZERO_ENTRIES` or `UNREADABLE`", never exit 0. Every pass/fail assertion uses a fixture log through the seam (suggest `FINDINGS_LOG_PATH`).

#### Concrete parse rules for "last `**Disposition…:**` bullet per `### DF-nn` entry"

Measured against the real log: the format template at the top sits inside a ``` fence and contains a literal `### DF-nn — …` heading and a `- **Disposition:** open | in v0.11.1 | …` bullet; the pre-seeded bullets read `- **Disposition:** open → recommend **in v0.11.1**` (so the word `open` and the phrase `in v0.11.1` both appear in an `open` entry). The rules:

1. Read as UTF-8 (a non-decodable file is `UNREADABLE`). Walk lines; a line whose `strip()` starts with ``` toggles a fence flag; **ignore everything while in a fence.**
2. Entry heading (outside fences): `^### (DF-(\d+))\b`. The id number is the integer; the placeholder `DF-nn` cannot match `\d+`, so the template is ignored twice over. An entry body runs to the next line matching `^#{1,3} ` outside a fence, or EOF (the `<!-- append new findings below this line -->` marker may sit in the last body; it holds no bullet).
3. Disposition bullet: `^- \*\*Disposition(?: \(([^)]*)\))?:\*\*\s*(.*)$` (case-sensitive; the optional parenthetical is the dated-note form `(2026-11-03)`). Continuation: following lines indented by ≥ 2 spaces that do not start `- **` belong to the same bullet. **The last matching bullet in the entry wins.** No match is `MISSING`.
4. Classify the bullet value after stripping leading `*`, backticks and whitespace, by its first token(s):
   - `^open\b` → `OPEN` (fail). This is also what the unamended pre-seeded bullets read as.
   - `^in v0\.11\.1\b` → `IN`. Recommend also requiring a non-empty pointer after the token (` — fixed by …`, a phase/requirement or a commit) so a bare `in v0.11.1` is `NO_POINTER`.
   - `^out \((.+)\)` (greedy to the last `)`, so reasons containing parentheses work) with a non-blank reason → `OUT`; `out` without a parenthetical or with a blank one → `BAD_OUT`.
   - `^duplicate of (DF-\d+)\b` → `DUP`; the target must be an id seen in the file and must not be the entry itself, else `BAD_DUP_TARGET`.
   - anything else → `UNRECOGNISED`.
5. Ids: compare as integers; a repeat is `DUPLICATE_ID`; a value ≤ the previous heading's is `NOT_INCREASING`. Next free id today is **DF-05**.
6. Named failures first in the status line, in order: `UNREADABLE`, `ZERO_ENTRIES`, then `BAD` (any per-entry finding), then `OK`. Print `path:line: DF-nn: <STATUS>` per bad entry sorted by line. The OK line should print the entry count so the real-tree test can assert `N ≥ 4`.

### Pattern 2: Config round-trip test (Answers 1-3)

**What:** load the committed full fixture through `Settings::load_from_file`, `serde_json::to_value(&settings)`, collect key paths on both sides, assert `input ⊆ output ∪ exempt ∪ KNOWN_GAPS`, and assert every `KNOWN_GAPS` row is still a gap.

**Where:** a top-level `tests/config_example_roundtrip.rs` (auto-discovered; the root manifest has no `autotests = false` [VERIFIED: grep]; works under default features, no `web-server` needed; `CARGO_MANIFEST_DIR` makes paths cwd-independent). The existing unit-test precedents in `src/config/*.rs` stay.

**Example** (proven in a scratch crate against the real `paladin::config::Settings`; the key-path walker is the whole trick):

```rust
// Source: scratch probe run 2026-11-02 against paladin-ai path dep, config 0.15.23
use paladin::config::Settings;
use std::collections::BTreeSet;

fn paths(prefix: &str, v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::Object(m) => for (k, x) in m {
            let p = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
            out.insert(p.clone());
            paths(&p, x, out);
        },
        serde_json::Value::Array(a) => for (i, x) in a.iter().enumerate() {
            let p = format!("{prefix}[{i}]");
            out.insert(p.clone());
            paths(&p, x, out);
        },
        _ => {}
    }
}
// input:  serde_yaml::from_str::<serde_json::Value>(&text)      (independent parser)
// output: serde_json::to_value(&Settings::load_from_file(path)?) (production loader)
// missing = input_paths.difference(&output_paths)
```

Two details matter. (1) Join with `.` is fine for reporting but is lossy for keys that contain `.` (`gpt-5.5.1`, `ops.team`); build paths as a `Vec<String>` of segments and compare those, or use a separator that cannot appear (for example `\u{1f}`), so `a.b` as one key and `a`→`b` as two never collide. (2) **Never print `Settings` or the serialised value in an assertion message**: `ApiKeyConfig.key` serialises and Debug-prints in plaintext (see Pitfall 6). Failure messages name key paths only.

**Measured results on HEAD** (scratch probe, not committed):

| Input | Input paths | Output paths | Missing from output |
|-------|-------------|--------------|---------------------|
| Verbatim `config.example.yml` | 161 | 205 | 8: `llm.{openai,anthropic,deepseek,kimi,qwen,grok,gemini,openai-compatible}.api_key` (ollama carries none) |
| Derived full fixture (every commented block enabled, plus extras) | 246 | 295 | 9: the same plus `llm.ollama.api_key` |
| Full fixture + 7 planted typos (`trace.log_sinks`, `timeouts.defualt_seconds`, `llm.openai.defualt_model`, `http.auth.api_keys[0].tennant`, `agents[0].tempature`, `web_server.dev_ui.mermaid_urll`, wrong-case top-level `Server`) | 254 | 295 | all 7 reported, plus the api_key paths |
| Full fixture + `treasurer.pricing.<m>.cache_reads` typo | — | — | **load error**: ``unknown field `cache_reads`, expected one of `prompt`, `completion`, `cache_read`, `cache_write`, `reasoning` for key `treasurer.pricing.gpt-5.5.1` `` |

Keys `Acme-Prod`, `Qwen/qwen3.7-plus`, `gpt-5.5.1`, `Moonshot/Kimi-K3.1-preview`, `claude-3-5-sonnet-20241022` and `{range: [0.0, 1.0]}` / `{fixed: 1.0}` / `not_sent` / `{budget: 2048}` all round-tripped without loss. Add a **watched-red** unit case for the walker itself (an inline YAML with a typo key must be reported), so the test can fail.

**Fixture composition** (what the probe added to the example, as a checklist for the planner): `llm.kimi` `max_tokens`, `context_window`, `reasoning_effort`, `reasoning_fallback`, `models` with two ids (one with `/` and `.`); `llm.ollama.reasoning_wire`; an `api_key` on at least two providers; two `http.auth.api_keys` entries (literal fake keys, a tenant named `Acme-Prod`), `bearer_token {enabled: true, tenant}`, `cors_allow_origins`, `rate_limit.enabled: true`; `treasurer.pricing` with three ids including `/` and `.`, all five price axes on one row, `allowance` (`warn_at`, `webhook.url` and `webhook.secret`, `tenants`, `api_keys`), full `cadence`; `agent_runtime` non-empty `per_tool` maps, one guardrail rule (`on_match: {action: redact, replacement: …}`), `model_context_limits`, `model_fallback.providers`; `trace.otel.headers`; `agents[0]` with `max_loops`, `stop_words`, `reasoning_effort: {budget: 2048}`, `reasoning_fallback`. Do **not** put an empty `models: {}` under an `llm.<provider>` block: that field is `skip_serializing_if = "BTreeMap::is_empty"` and would read as dropped.

**Redis cadence variant:** `backend: { redis: { url_env: … } }` is a second shape; load it as a second small fixture, because `backend` cannot be both `in_process` and `redis`.

### Anti-Patterns to Avoid

- **Comparing values, not just key paths**: f32 re-serialisation and unquoted-decimal normalisation make value comparison noisy (Pitfalls 2, 3).
- **Printing `Settings` or its JSON in a failing assertion**: leaks `http.auth.api_keys[].key`.
- **Wiring the checker into `make check-gates` or CI**: the log legitimately holds `open` mid-phase (D-03).
- **A "real tree passes" assertion on the findings log in the shell test**: it would turn the CI shell-guard job red on every open finding.
- **Uncommenting blocks in the shipped example** to make the test easy: breaks Phase 54 D-04's "a verbatim copy fails closed" property (D-09).
- **Relying on PyYAML in the checker**: the sandbox's system python has it, a contributor's may not; house gates use stdlib only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parsing the example's YAML in the test | A line-based YAML reader | `serde_yaml` (dev-dep, already locked) or `config` → `serde_json::Value` | Anchors, flow maps `{fixed: 1.0}`, quoted keys. |
| Detecting dropped keys | A hand-written list of "known fields" | Diff of input vs `serde_json::to_value(&Settings)` key paths | The list drifts the moment a field is added; the diff cannot. |
| Shell-gate scaffolding | A new test harness | Copy `tests/scripts/check-no-stub-panics_test.sh`'s helper block | Determinism, no-mutation and never-vacuous checks are already encoded. |
| Coverage percentage | A new tool | The same LF/LH summation CI runs over `lcov.info` (Answer 6) | `cargo llvm-cov report --summary-only` reports a different scope (the guide's own CI comment explains). |
| Fetching the CI figure | Scraping the web UI | `gh run list/view` and the `coverage-summary` artifact | `gh` is authenticated in this environment. |

**Key insight:** every piece of machinery this phase needs already exists in the repo. The risk is not building too little but building something that turns a legitimately-open log or a legitimately-unserialised secret into a red build.

## Runtime State Inventory

Not a rename/refactor/migration phase. Omitted per the template.

## Answers to the Open Questions CONTEXT hands to research

### Answer 1 (D-08): every production path that reads a `config.example.yml` section

**Production call sites that read a config file** [VERIFIED: `git grep` + position of first `#[cfg(test)]` in each file]:

| # | Call site | What it reads | Notes |
|---|-----------|---------------|-------|
| P1 | `Settings::load_from_file(&config_path)` in `src/bin/paladin-server.rs` `run()` (path = `PALADIN_CONFIG`, else first CLI argument, else `config.yml`) | Whole file, format by extension (`.yml`/`.yaml` YAML, else TOML) | **The** production path. No env overlay. |
| P2 | `Settings::load_from_file(&opt.config)` in `src/main.rs` (the `paladin` binary, `--config`, default `config.yml`) | Same loader | Then `info!("Loaded configuration: {:?}", config)` (Lead L3). |
| P3 | `Settings::new()` in `src/application/cli/commands/arsenal.rs` (`paladin-cli arsenal`) | `File::with_name("config")` plus `Environment::with_prefix("APP")` plus optional `config.{APP_ENV}` | Same schema; only `arsenal` is used; overlay is out of scope (D-12). |

**Not production loaders** (all located after the file's first `#[cfg(test)]`): `deserialize_wrapper_from_file` in `src/config/treasurer.rs` (test mod from line 878) and `agent_runtime.rs` (1593); `Config::builder()` in `trace.rs` (288), `web_server.rs` (99), `agents.rs` (427), `crates/paladin-llm/src/config/llm.rs` (396) and `crates/paladin-llm/src/registry/mod.rs` (835); the `concat!(env!("CARGO_MANIFEST_DIR"), "/../../config.example.yml")` read in `llm.rs` ~758 (test). `paladin-llm` therefore has **no** file loader: `LlmConfig` is a type that arrives as `Settings.llm`. The CLI `agent`/`battalion`/`maneuver` commands parse their own per-agent YAML with `serde_yaml` (a different schema, `PaladinYamlConfig` etc.), never `config.example.yml`. The `run_store`/`run_queue`/`run_worker`/`run_stream`/`assistants`/`schedules`/`webhooks` configs are env-only (`apply_env_overrides`) and read no YAML, as the example's own banner says.

**Per top-level section** (`Settings` field, who parses it, who applies it in `paladin-server`):

| Example section | `Settings` field | Parsed by | Applied in the server boot path |
|-----------------|------------------|-----------|---------------------------------|
| `llm_type`, `llm_url`, `llm_api_key` | required `String`s | P1, P2, P3 | **Nothing** (schema-required only; the example says so) |
| `llm` | `Option<LlmConfig>` | P1-P3 | `agent_host::validate_config`/`build_agent` (`settings.llm`), `FacadeProvisioner::from_settings` (`with_llm_config(settings.llm.clone())`), `facade_provisioner` (`create_from_config`) |
| `max_file_size`, `sources` | required | P1-P3 | **Nothing** in `src/` outside `src/config` (git grep) |
| `server` | `ServerConfig` (required) | P1-P3 | bind address `agent_host.rs` |
| `timeouts` | `Option<AgentTimeoutsConfig>` | P1-P3 | `paladin-server.rs` (`settings.timeouts`) |
| `http` | `Option<WebHttpConfig>` | P1-P3 | `paladin-server.rs` → `build_auth_config(&http.auth)`, router layers |
| `agent_runtime` | `AgentRuntimeConfig` | P1-P3 | **Only `token_budget`** (`agent_host.rs`, `facade_provisioner.rs`); the other eleven middleware blocks have no consumer outside `src/config` (git grep `.agent_runtime`) |
| `treasurer` | `TreasurerConfig` (`deny_unknown_fields`) | P1-P3 | `get_treasurer_config()` validated at boot; pricing, allowance, cadence applied |
| `trace` | `TraceConfig` | P1-P3 | `run_api_wiring.rs` (`with_trace_config`, `persist`) |
| `web_server` | `WebServerConfig` | P1-P3 | **No server consumer found**: `web_server.dev_ui.mermaid_url` is read by nothing in `src/`; the dev-UI router is wired only in `examples/platform_api_client.rs` with a hard-coded URL (Lead L4) |
| `agents` | `Vec<AgentDefinition>` | P1-P3 | `agent_host::validate_config` + `build_agent` |

DOGF-03 asks only that each section survive the **loader**; the "applied" column is the source of Leads L4 and L6, which are semantic "accepted but unused" cases the round-trip cannot see.

### Answer 2 (D-10): is re-serialising `Settings` faithful?

Yes, with two exemptions [VERIFIED: grep of every serde attribute and `impl Serialize` in `src/config/` and `crates/paladin-llm/src/config/`; scratch run on the real types]:

- **No custom `Serialize` impls**, no `#[serde(flatten)]`, no `#[serde(untagged)]`, no `#[serde(alias)]` on a key the example uses. Enums use `rename_all = "snake_case"` or `lowercase` (`UserRole`), internally tagged `tag = "action"` (`GuardrailOnMatch`), or externally tagged unit/newtype variants (`CadenceBackend`: `in_process` / `{redis: {url_env}}`); all round-trip.
- **`rename`**: only `LlmConfig.openai_compatible` ↔ `"openai-compatible"` (with `alias = "openai_compatible"`). Serialisation writes the hyphenated form. The example uses the hyphenated form, so it round-trips; a fixture that used the snake_case alias would read as a drop. Keep the hyphenated spelling.
- **`skip_serializing` (exemption rows):**
  1. `llm.<provider>.api_key` for all nine providers (`LlmProviderConfig.api_key`, WR-04: never serialised so a config dump cannot emit a key). Reason for the row: "secret, deliberately unserialised".
  2. `treasurer.allowance.webhook.secret` (`AllowanceWebhookConfig.secret`, also redacted in `Debug`).
- **`skip_serializing_if` (not a drop unless the input is empty):** `LlmProviderConfig` `max_tokens`, `context_window`, `reasoning_effort`, `reasoning_fallback`, `reasoning_wire` (`Option::is_none`) and `models` (`BTreeMap::is_empty`). A fixture value that is present and non-empty serialises. Do not put `models: {}` in the fixture.
- **`Option::None` serialises as `null`**, so the output key set is a superset of the input's. That is why the check is one-directional (input ⊆ output).
- **Secret-bearing types that do serialise in plaintext**: `ApiKeyConfig.key`, `trace.otel.headers` values. The test must not print them.
- The reverse direction (output paths with no input path) is informational: on the verbatim example it lists 52 paths, of which the only one under a section the example claims to write in full is `agent_runtime.model_fallback.steps` (Lead L5). Everything else is a commented-by-design block (`treasurer.allowance`, `treasurer.cadence`, `http.auth.bearer_token.tenant`), an unset optional field, or a `Settings` field the example does not mention (D-09 says those are out of scope).

**Recommendation: key-path diff, no `serde_ignored`.** `serde_ignored` is not in `Cargo.lock` (grep), would pass `deny.toml` (MIT OR Apache-2.0; the allowlist has both), and is technically usable with `config::Config` (`Config` implements `Deserializer`, which is what `try_deserialize` calls). But it adds nothing the diff does not give and it cannot see lost-but-known fields.

### Answer 3 (D-11): what does `config` 0.15.23 do to map keys?

**Nothing, on the YAML file path** [VERIFIED: scratch crate, `config = "=0.15.23"`, both `File::from_str(.., Yaml)` and `File::new(path, Yaml)`, plus the real `Settings::load_from_file` on a full fixture]:

- Case is preserved: `Acme-Prod`, `Upper`, `Mixed.Case/Key-1`.
- `.` in a key is not split: `gpt-5.5.1`, `ops.team`, `Qwen/qwen3.7-plus` read back whole.
- `/`, `-`, `_`, `[0]`, and spaces in keys are preserved.
- Locked version is **0.15.23** (`Cargo.lock`), not the 0.15.11 in the manifest requirement or CONTEXT; the registry cache also holds 0.15.27, unused by this lock.

Behaviours that **do** change a value and belong in the fixture/pitfall list:

- **Unquoted YAML decimals into a `String` field are re-rendered**: `prompt: 2.50` → `"2.5"`, `completion: 10.00` → `"10"`, `cache_read: 0.250` → `"0.25"`. Numerically equal, textually not. The example quotes its prices (`"2.50"`); the fixture must too, and a note belongs near the example's price paragraph if an operator could drop the quotes.
- YAML 1.1 booleans: `yes` deserialises as `true` into a `bool`.
- Environment overlays (`Environment::with_prefix("APP")`) do lowercase/split on separators; that is P3 only and out of scope (D-12).

### Answer 4 (D-03): shell-gate shape and `test-shell-guards`

Covered in Pattern 1 above (shape, harness, parse rules, Makefile glob, CI step, executable bit, `lint-shell`).

### Answer 5 (D-13/D-14): published crates, README, install, runtime

- **Crate names** [VERIFIED: `cargo search paladin-ai`, crates.io API]: the umbrella is **`paladin-ai`** (lib name `paladin`), version `0.11.0` published 2026-10-26, not yanked, MSRV 1.88, edition 2024; siblings `paladin-llm`, `paladin-eval`, `paladin-ai-core`, `paladin-ports`, `paladin-battalion`, `paladin-memory`, `paladin-storage` and others at `0.11.0`.
- **README Quick Example** (identical at tag `v0.11.0` and HEAD) is a self-contained `main` using `paladin::MockLlmAdapter`, `PaladinBuilder`, `PaladinExecutionService`, `CircuitBreaker`: **no API key, no network**. "For real use: `Arc::new(OpenAIAdapter::from_env()?)`".
- **README dependency snippet.** At tag `v0.11.0` (what crates.io shows): `paladin-ai = "0.5"  # default features: ["llm-openai"]`. At HEAD: `paladin-ai = "0.11.0"`, with the same stale comment (the real default is `llm-openai`, `llm-anthropic`, `llm-deepseek`). The README has **no `cargo add` line**; `cargo add paladin-ai` resolves `0.11.0` with those three default features [VERIFIED: ran it]. A reader who pastes the tag's `"0.5"` gets `^0.5` (a 0.5.x crate), not 0.11. Also at the tag: `Current version: **0.8.0**`. These two are fixed at HEAD by FIRST-02, so on the walk they are `out (fixed by Phase 54 — FIRST-02)`.
- **Which package ships `paladin-cli`:** `paladin-ai` itself. `crates.io` `bin_names` = `paladin`, `paladin-cli`, `paladin-server`; each `[[bin]]` carries `required-features` (`cli`, `cli`, `web-server`). `cargo install paladin-ai --version 0.11.0 --bin paladin-cli` **fails**, exit 101: ``target `paladin-cli` in package `paladin-ai` requires the features: `cli` `` (reproduced, 2026-11-02). The working form is `cargo install paladin-ai --version 0.11.0 --features cli --bin paladin-cli` (add `--locked` to use the published lockfile) [ASSUMED: not compiled here because of disk; the failure message names the fix]. `grep` finds **no** `cargo install paladin-ai` anywhere in README or `docs/src`; the docs only say `cargo build --release --features cli --bin paladin-cli` from a checkout.
- **`basic_paladin`**: auto-discovered `examples/basic_paladin.rs`, uses `MockLlmAdapter` (keyless). It is in the published package (`include` lists `examples/**`) but a crates.io user cannot `cargo run --example` it without a checkout; the quickstart documents `git clone` + `cargo run --example basic_paladin`. For the walk, either copy the file out of the downloaded package (`~/.cargo/registry/src/*/paladin-ai-0.11.0/examples/basic_paladin.rs`) into the scratch crate's `examples/` (recommended: least compile, still the published source), or try `cargo install paladin-ai --version 0.11.0 --example basic_paladin` [ASSUMED: syntax and behaviour not run]. Decide at plan time; D-13's wording presumes a command that does not exist, so record that as the first stumble if it is one.
- **Runtime key needs:** the Quick Example and `basic_paladin` need none. The Quickstart (`docs/src/getting-started/quickstart.md`) uses `OpenAIAdapter::from_env()?` → `OPENAI_API_KEY`. `setup-check` reads `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY` (plus optional `REDIS_URL`, `QDRANT_URL`) and reports `Skip` when unset; it checks `.env` in the cwd (`Warn: Not found - run 'paladin onboarding' to create`). `onboarding` is an interactive `dialoguer` wizard (OpenAI / Anthropic / DeepSeek only) that writes a `.env` (and sample agent YAMLs) into the **current directory**.
- **With no key:** `setup-check` runs without a key and should report `Skip` rows (its unit test asserts `Skip` for a missing `OPENAI_API_KEY`); `onboarding` needs a TTY, so a non-interactive agent run is expected to fail or hang at the first prompt: that is itself the keyless observation to log (how clear is it?). Neither was executed here.
- **Walk docs mismatch to expect:** the CLI docs pages show `paladin onboarding` / `paladin setup-check` and sample output `Paladin CLI: v0.1.0`; the binary is `paladin-cli`.
- **Is crates.io reachable from the sandbox?** **Yes**, through the pre-configured proxy: `cargo search`, `cargo add`, `cargo metadata`, `cargo install` (download phase) and `curl https://crates.io/api/v1/…` all succeeded. So the keyless half is agent-runnable here, subject to disk (see Environment Availability).
- **Tag fetchable:** `git tag -l` shows local tag `v0.11.0`; `git show v0.11.0:README.md` works.

### Answer 6 (D-17): the local coverage procedure, exactly

- **Guide:** `docs/src/contributing/testing-guide.md` § "Test Coverage": (1) `rustup component add llvm-tools-preview`; (2) `cargo install cargo-llvm-cov --locked` (or `cargo binstall cargo-llvm-cov`); (3) `make services-up`; (4) `make coverage`; (5) optional `make coverage-html`.
- **`make coverage`** runs `bash scripts/coverage.sh`, which `exec`s: `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines "$FLOOR" -- --test-threads=1` with `FLOOR` defaulting to `82`, after probing Redis and the object store and exporting `TEST_*`, `REDIS_*`, `MINIO_*`, `USE_EXTERNAL_TEST_SERVICES=true`.
- **CI `coverage` job** (`.github/workflows/ci.yml`, job `coverage`): service containers `redis:7-alpine` on `6380:6379` and `rustfs/rustfs:1.0.0` on `9010:9000` (keys `testuser`/`testpass123`); toolchain `dtolnay/rust-toolchain@stable` with `llvm-tools-preview`; **`cargo-llvm-cov@0.8.7`** via `taiki-e/install-action`; then `bash scripts/coverage.sh` with `TEST_*` preset; then a python step sums `LF/LH/FNF/FNH` over `lcov.info` and writes `coverage-summary.txt` (artifact `coverage-summary`, 14 days).
- **Same command, same feature flags:** the runbook must name `scripts/coverage.sh` (the single source of truth) and the features `integration-tests,llm-all`.

**Three traps the guide does not state; the runbook must handle each and any that bites is a D-17 S3 finding:**

1. **`make coverage` prints no percentage.** `--lcov --output-path` writes a file and prints nothing (CI's own comment says so), yet the guide's "Reading the output" says "`make coverage` prints an LCOV summary". The local figure has to be computed from `lcov.info` with the same summation CI runs. Give the maintainer this exact snippet (taken from the CI step):

   ```bash
   python3 - <<'PY'
   lf = lh = 0
   for line in open("lcov.info"):
       if   line.startswith("LF:"): lf += int(line[3:])
       elif line.startswith("LH:"): lh += int(line[3:])
   print(f"Lines: {lh}/{lf} = {100*lh/lf:.2f}%  (truncated: {int(100*lh/lf)})")
   PY
   ```
2. **Port mismatch (hypothesis from reading, to be confirmed by the walk).** `make services-up` starts `docker/docker-compose.yml`: Redis on host `6379` with `--requirepass ${REDIS_PASSWORD:-changeme}` and RustFS on `9000/9001` with keys `paladin-dev`/`paladin-dev-secret`. `scripts/coverage.sh` probes Redis at the preset `TEST_REDIS_HOST`, then `redis:6379` (devcontainer), then `localhost:6380`, and the object store at the preset endpoint, then `rustfs:9000`, then `localhost:9010`. Ports 6380/9010 come from `docker/docker-compose.test.yml` and `.env.example`'s `TEST_*` block. On a bare host that ran only `make services-up` and has no sourced `.env`, neither `localhost:6380` nor `localhost:9010` answers, so the script's likeliest outcome is its `ERROR: Redis unreachable … Start services with 'make services-up'` exit; inside the devcontainer the `redis:6379` and `rustfs:9000` peers match. Also unverified: whether the password-protected compose Redis passes the script's unauthenticated `redis-cli ping` probe [ASSUMED]. The runbook should say: run the guide's steps verbatim, record what happens, and only then apply the documented workaround (`.env` from `.env.example`, or `docker compose -f docker/docker-compose.test.yml up -d redis-test rustfs-test`) as a logged stumble.
3. **Tool pin drift.** CI pins `cargo-llvm-cov@0.8.7`; the guide's `cargo install cargo-llvm-cov --locked` is unpinned, and CI's toolchain is `stable`. A figure that differs by a fraction of a percent after a tool or toolchain bump is a candidate cause to record, not a defect in the code. The CI summary also still prints `Scope: --workspace --features integration-tests` (the real scope is `integration-tests,llm-all`): a stale label, S4.

**Comparison target.** Per the todo's Phase 36.1 update, compare against the `coverage` job of the CI run **on the same commit** (not the historic 82.39 % at `e9e3267f9…`). Fetch it with `gh run list --workflow=ci.yml --commit <sha>` then `gh run download <id> -n coverage-summary` (or read the job summary). Pass rule is D-17: both figures truncate to the same whole percent and both ≥ 82.

### Answer 7 (D-02): proving tests for DF-01..DF-04, confirmed present on HEAD

All names below were confirmed by `git grep "fn <name>"` on HEAD. The two gates were **run** on HEAD in this session and are green. The cargo tests were not re-run here (a root-crate `--features web-server` build competes with the 4.2 GB of free disk); Stage A re-runs them, which is D-00e's re-check.

| DF | Owner | Proof on HEAD |
|----|-------|---------------|
| DF-01 | FIRST-01 (Phase 54) | `cargo test --lib --features web-server -- validate_refuses_an_unexpanded_placeholder_key validate_refuses_a_placeholder_key_while_auth_is_disabled validate_names_a_later_entry_whose_key_is_a_placeholder validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks` (`src/config/agents.rs`); `cargo test --bin paladin-server --features web-server build_auth_config_refuses_an_unexpanded_placeholder_key` (`src/bin/paladin-server.rs`); the log's own cited `yaml_env_placeholder_is_not_expanded` (`src/config/treasurer.rs`) still exists and pins "no expansion"; grep: `grep -nE '(^\s*\|[{,]\s*\|-\s+)key:\s*"?\$\{' config.example.yml` → exit 1; `test ! -e k8s/server/configmap.yaml`. Process-level proof: `54-FIRST-EVIDENCE.md` § "Boot refusal (SC1)" (real binary exits 1 with the `unexpanded '${…}' placeholder` message). |
| DF-02 | FIRST-02 / FIRST-03 (Phase 54) | `make check-version-strings` → `✅ OK: 128 file(s) scanned, every copyable version string equals '0.11.0'` (run today); `./tests/scripts/check-version-strings_test.sh`; `grep -n 'Current version' README.md` → `185:Current version: **0.11.0**.`; `54-FIRST-EVIDENCE.md` prove-then-lock table. |
| DF-03 | FIXES-01 (Phase 55) | `make check-no-stub-panics` → `✅ OK: scanned 624 file(s) (19 test-only), 0 finding(s)` (run today); `./tests/scripts/check-no-stub-panics_test.sh`; `test ! -e crates/paladin-content/src/services/content_filtering_service.rs` (the file the entry names was **deleted**, not fixed in place: "dead file deleted", 55-VERIFICATION). The entry's remedy ("remove or return `Err`") is met by removal. |
| DF-04 | TENANT-03..05, TENANT-07 (Phase 53) | `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404` (`crates/paladin-web/src/thread_controller.rs`); `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404`, `cross_tenant_fork_is_the_unknown_thread_404`, `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` (`src/application/services/run/http_surface_tests.rs`); `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` (`…/run/submission.rs`); `job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass` (`crates/paladin-web/src/agent_controller.rs`); `permits_thread_admits_all_and_only_the_latest_runs_tenant` (`crates/paladin-core/src/platform/container/principal.rs`). Evidence with watched-red runs: `53-TENANT-EVIDENCE.md`; verdict table: `53-VERIFICATION.md` (SC1-SC3 VERIFIED; TENANT-03/04/05/07 SATISFIED). |

The log says pre-seeded entries "need a real reproduction before promotion"; the owner's failing-first regression test, cited above, is that reproduction (D-02).

### Answer 8: any web3sec return in the tree?

**No.** `git grep -l 70657919` hits only planning records (MILESTONES, PROJECT, RETROSPECTIVE, ROADMAP, STATE, the v0.11.0 audit, 47-xx summaries and hand-offs). `git ls-files | grep -i web3sec|handoff|feedback` lists only Paladin-authored hand-offs (`47-`, `49-`, `50-`, `50.1-`) and `46-FEEDBACK-web3sec.md` (web3sec's 2026-10-24 findings, the source of the 2,683 / 0 / 2 figure). The only commits since 2026-10-26 that mention web3sec are Paladin's own outbound hand-off (`ae6dcbb5a`, `18843ae2f`). The folded todo's latest update is 2026-10-19. The §6 contract (quoted in `47-HANDOFF-web3sec.md`) asks for: the consumer SHA (must print `70657919beadbd341a72be2d0107080017a803f0`), the final D-2 choice, gate totals (`cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `check --workspace`, `test --workspace`) against the consumer's named pre-bump figure (2,592 / 0 / 2 earlier, 2,683 / 0 / 2 latest), and any divergence including real `use` lines the muster missed and any §10.8 step that did not read as written. **Do not invent one**: DOGF-02 starts as a Stage B checkpoint and ends in the D-16 `checkpoint:decision`.

## Common Pitfalls

### Pitfall 1: A "real tree passes" assertion on a log that is allowed to be open
**What goes wrong:** copying the house test verbatim adds a case that runs the checker on the real log and expects exit 0; CI's `make test-shell-guards` goes red the first time a finding is open.
**How to avoid:** all pass/fail assertions use fixture logs through the env seam; the real-log case asserts parseability only (≥ 4 entries, no `ZERO_ENTRIES`/`UNREADABLE`).
**Warning signs:** the test file mentions `02-dogfood-findings-log.md` in an `assert_silent`.

### Pitfall 2: Comparing values and tripping on f32 and decimal normalisation
**What goes wrong:** `temperature: 0.7` (f32) serialises as `0.699999988079071`; unquoted `2.50` becomes `"2.5"`.
**How to avoid:** key paths only; quote prices in the fixture.

### Pitfall 3: Joining key segments with `.`
**What goes wrong:** `treasurer.pricing.gpt-5.5.1` compares equal to a nested `gpt-5` → `5` → `1` path, so a split bug would hide.
**How to avoid:** compare segment vectors, or join with a separator that cannot occur in a key.

### Pitfall 4: Exemption lists that go stale
**What goes wrong:** an exempted key becomes serialised later (someone removes `skip_serializing`), and the exemption keeps passing.
**How to avoid:** treat exemption rows like `KNOWN_GAPS`: a row whose key now round-trips fails the test. This also pins WR-04 (a key must not start serialising by accident); keep a separate explicit assertion that the serialised output contains **no** `api_key` field.

### Pitfall 5: The drift guard parsing prose comments
**What goes wrong:** a naive "comment line that looks like `key:`" regex matched 55 lines in `config.example.yml`; 8 are prose (line "Live-verified:" at ~121, and the env-var banner at ~430-463 such as `# run_store: which durable Run backend…`). CONTEXT's "48 commented key lines" is the YAML-ish subset (~47: lines in the `kimi` block, `api_keys`, `bearer_token`, `pricing`, `allowance`, `cadence`).
**How to avoid:** stop the scan at the "Platform API" banner (everything after it is env-only prose), skip the single "Live-verified:" line via a one-row prose list with its reason, and compare leaf key names (or segment suffixes) rather than full paths because commented blocks inherit their parent from indentation.

### Pitfall 6: Secrets in Debug and Serialize output
**What goes wrong:** `ApiKeyConfig` derives plain `Debug` and `Serialize`, so `http.auth.api_keys[].key` prints and serialises in plaintext, unlike `LlmProviderConfig`, `AllowanceWebhookConfig`, `OtelConfig` and `NodeCacheConfig`, which redact. An assertion message that formats `Settings` leaks the fixture's fake key (harmless here) and, in the walk, a real one.
**How to avoid:** failure messages carry key paths only; the walk's evidence file redacts keys; see Lead L3.

### Pitfall 7: Walking from the repo or with a warm workspace
**What goes wrong:** running the walk inside `/home/user/paladin-dev-env` picks up path deps, the workspace `Cargo.lock` and `[patch]`, so a published-crate defect is masked (D-13).
**How to avoid:** a scratch crate under the scratchpad; `cargo add` with no `--path`; pass `--locked` to `cargo install`; record `cargo tree -i paladin-ai` showing the registry source.

### Pitfall 8: Disk exhaustion during a published-crate compile
**What goes wrong:** resolving the Quickstart manifest locks 419 packages; building them filled this sandbox's volume to 0 MB twice during research (two scratch targets), and an unwritable `target/` fails with misleading `couldn't create a temp dir` errors that are not product defects.
**How to avoid:** set `CARGO_TARGET_DIR` on a volume with ≥ 10 GB free, prefer `cargo check` for compile-only steps, delete the target afterwards, and never log a disk-full failure as a finding.

### Pitfall 9: Onboarding leaves a real key in `.env`
**What goes wrong:** `paladin-cli onboarding` writes the pasted key to `.env` in the **current directory** (plus sample YAMLs).
**How to avoid:** the maintainer runs it inside the scratch directory outside any git repo; the runbook ends with `shred`/`rm` of that `.env`; the pasted output is redacted before it enters the evidence file.

## Code Examples

### Findings-log checker core (parse rules as code)

```python
# Source: derived from the parse rules above + scripts/check-version-strings.sh shape
import re, sys
HEAD = re.compile(r'^### (DF-(\d+))\b')
BULLET = re.compile(r'^- \*\*Disposition(?: \(([^)]*)\))?:\*\*\s*(.*)$')
IN_RE  = re.compile(r'^in v0\.11\.1\b')
OUT_RE = re.compile(r'^out \((.+)\)')
DUP_RE = re.compile(r'^duplicate of (DF-\d+)\b')

def classify(value, ids, me):
    v = value.lstrip('*` ').strip()
    if re.match(r'^open\b', v):        return 'OPEN'
    if IN_RE.match(v):                 return 'IN'
    m = OUT_RE.match(v)
    if m: return 'OUT' if m.group(1).strip() else 'BAD_OUT'
    m = DUP_RE.match(v)
    if m: return 'DUP' if (m.group(1) in ids and m.group(1) != me) else 'BAD_DUP_TARGET'
    return 'UNRECOGNISED'
# walk lines; toggle `in_fence` on lines whose strip() startswith("```"); skip while in_fence;
# keep the LAST BULLET match per entry (carry continuation lines); two passes so `ids` is complete.
```

### Shell test case shape (house helpers)

```bash
# Source: tests/scripts/check-no-stub-panics_test.sh helpers (assert_fire / assert_silent)
fresh "open-entry"          # make_tree builds a fixture log with DF-01 disposed
plant "log.md" '- **Disposition:** open'   # appends to DF-01's last bullet position
assert_fire "${FX}" "an entry whose last Disposition bullet is open fails" "DF-01" "OPEN"
```

### Full-fixture drift guard sketch

```rust
// Collect leaf names from YAML-looking comment lines of config.example.yml (before the
// "Platform API" banner), minus PROSE_COMMENT_KEYS = [("Live-verified", "prose, not config")],
// then assert every leaf name occurs as a key somewhere in config.example.full.yml's parsed Value.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| One-off "grep for stale versions" sweeps | Offline gate scripts in `check-gates` + shell fixture tests | Phases 54-55 | The checker here follows that shape but is deliberately outside `check-gates`. |
| Unit tests on typed deserialisation only | Round-trip key-path diff through the real loader | This phase | Catches known-but-lost and unknown-and-ignored keys alike. |

**Deprecated/outdated:** `serde_yaml 0.9.34+deprecated` is upstream-deprecated but is the repo's existing YAML crate (CLI, `paladin-eval`) and is only used here as a dev-time input parser; no migration is in scope.

## Leads already found while researching (for Stage A to reproduce and log; none is a log entry yet)

Each carries its evidence and a proposed severity. D-00e applies: Stage A re-checks on HEAD and either reproduces or drops. Proposed dispositions follow D-05.

| Lead | What | Evidence | Proposed severity / disposition |
|------|------|----------|----------------------------------|
| **L1** | `docs/src/getting-started/installation.md` (HEAD) tells readers `paladin-llm = { version = "0.11.0", features = ["llm-openai"] }`; published `paladin-llm 0.11.0` has no such feature (its features are `anthropic, deepseek, gemini, grok, kimi, mock, ollama, openai, openai-compatible, openai-embeddings, qwen, vision`). | **Reproduced**: a scratch manifest with that line fails `cargo metadata`: "`paladin-llm` does not have that feature … available features: anthropic, deepseek, …". Fixing it to `"openai"` resolves (225 packages). The Quickstart's own manifest (`features = ["openai"]`) resolves. `check-version-strings` checks only the version token, so it cannot catch this. | S3, contained docs fix → `in v0.11.1`. Also `README.md`'s dependency-snippet comment says `default features: ["llm-openai"]` but the default is three providers (S4, same plan). |
| **L2** | `paladin-cli` cannot be installed from crates.io as one would try: `cargo install paladin-ai --version 0.11.0 --bin paladin-cli` exits 101 (needs `--features cli`), and no doc mentions `cargo install` of the CLI at all (docs say `cargo build … --features cli` from a checkout). | **Reproduced** 2026-11-02 (error text above). `git grep 'cargo install paladin-ai'` finds nothing. | S3, contained docs fix (add the install line with `--features cli --locked`) → `in v0.11.1`. |
| **L3** | `src/main.rs` (`paladin` binary) logs `info!("Loaded configuration: {:?}", config)`; `ApiKeyConfig` derives plain `Debug`, so `http.auth.api_keys[].key` is logged in plaintext at the default `debug` filter. | Read: `ApiKeyConfig` is `#[derive(Debug, Clone, Serialize, Deserialize)]`; the other secret-bearing config types implement a redacting `Debug`; the line is present at tag `v0.11.0`. **Not run**: needs a build with `--features cli`. | Credential in logs: grade at worst credible deployment per D-04 (likely **S1**; the `paladin` binary is not the server, so S2 is arguable). Needs a reproduction run; the fix is a redacting `Debug` (additive) or dropping the `{:?}`. Applies the security review in `security.instructions.md`. |
| **L4** | `web_server.dev_ui.mermaid_url` in the example is parsed but read by nothing in `src/`; `paladin-server` never mounts the dev-UI router (only `examples/platform_api_client.rs` does, with a hard-coded URL). The example documents it as live config. | `git grep mermaid_url|DevUiState -- src` finds no server consumer. | S3. Reproduce by booting `paladin-server` with a distinctive `mermaid_url`; if confirmed, a docs/example comment change is contained (`in v0.11.1`), wiring it is not (`out`, D-00d: no new routes). |
| **L5** | The example says `agent_runtime`'s "every field below is written out explicitly and is IDENTICAL to `AgentRuntimeConfig::default()`" but `model_fallback.steps` (D-15) exists in the type and is not written. | Reverse key-path diff on HEAD: `agent_runtime.model_fallback.steps` is the only output path under `agent_runtime` with no input path. | S3/S4, contained example edit; the DOGF-03 drift guard (reverse direction for `agent_runtime` only) would pin it. |
| **L6** | `Settings::validate()` (which validates `agent_runtime`, `trace`, `web_server`) has no caller outside `src/config/` in the tree; `paladin-server` validates only the treasurer and the run configs. So an invalid `trace.channel_capacity` or `agent_runtime.*` value may not stop boot. | `git grep -E '(trace|web_server|agent_runtime)\.validate\(\)|Settings::validate' -- src crates` finds no non-config caller. Consequence is **[ASSUMED]**: some of these may validate lazily elsewhere. | S2 if reproduced (a value the type calls invalid is accepted), otherwise drop. Reproduce with one bad `trace` value against `paladin-server`. |
| **L7** | Coverage walk traps (Answer 6): no percentage printed; compose-vs-probe port mismatch; unpinned `cargo-llvm-cov`; stale `Scope:` label in CI's summary; the guide's "Reading the output" claim. | Read of `scripts/coverage.sh`, `ci.yml`, `docker-compose*.yml`, `testing-guide.md`. | Runbook content; each one that bites on the maintainer's machine is an S3 DF + contained docs fix (D-17). |
| **L8** | `setup-check`'s Anthropic validator posts model `claude-3-haiku-20240307` and the checker only validates OpenAI, Anthropic and DeepSeek (nine providers ship). | Read of `src/application/cli/commands/setup_check.rs`. Whether that model id is still served is **[ASSUMED]: not checked**. | Watch during the keyed half; log only if it bites. |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli` succeeds (only the failing form was run) | Answer 5, L2 | The documented fix line would itself be wrong; Stage A must run it before writing it into docs. |
| A2 | `cargo install paladin-ai --version 0.11.0 --example basic_paladin` is a valid way to run a packaged example | Answer 5 | Fall back to copying the file from the registry source; no plan depends on it. |
| A3 | The password-protected compose Redis passes or fails `scripts/coverage.sh`'s unauthenticated `redis-cli ping` probe | Answer 6 trap 2 | Only affects the runbook's expected-failure wording. |
| A4 | `onboarding` fails or hangs without a TTY, and `setup-check` with no key reports `Skip` rows and exits 0 | Answer 5 | The keyless observations are what the walk records; wrong guesses are corrected by the run. |
| A5 | Consequence of L6 (no boot refusal for an invalid `trace`/`agent_runtime` value) | L6 | Lead is dropped at D-00e re-check if validation happens elsewhere. |
| A6 | `claude-3-haiku-20240307` may no longer be served | L8 | None unless the maintainer's Anthropic key is used. |
| A7 | Running the DF-01..04 cargo tests on HEAD passes (recorded passing in Phases 53-55; not re-run in this session) | Answer 7 | Stage A's re-check will show it; a failure would itself be a finding. |

## Open Questions

1. **Which `basic_paladin` form does the walk use?**
   - What we know: the README and Quickstart never give a crates.io-user command; `cargo run --example` needs a checkout.
   - What's unclear: whether D-13's "the `basic_paladin` example" means the repo's file or a published-crate equivalent.
   - Recommendation: copy the example from the registry copy of `paladin-ai-0.11.0` into the scratch crate; record the missing command as a stumble (S3 docs gap).
2. **Who compiles the published crates, given disk?**
   - What we know: 4.2 GB free in the sandbox; resolving the Quickstart manifest locks 419 packages; two scratch builds exhausted the volume.
   - Recommendation: the Stage A plan sets `CARGO_TARGET_DIR` to a large volume or limits itself to `cargo check` for the Quick Example; otherwise it hands the compile steps to the maintainer's runbook. Decide at plan time.
3. **Is the findings-log checker's pointer requirement (`in v0.11.1 — <pointer>`) too strict?**
   - D-03 lists only the keyword forms. Recommendation: require a non-empty pointer (`NO_POINTER`), since D-02 says each disposition names its evidence; the planner may relax this.
4. **Should the full fixture's secret-looking fake keys avoid `llm.*.api_key` entirely?**
   - They are exempt from the round-trip anyway; including them proves they load. Recommendation: include two, cover the rest via the exemption row.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | round-trip test, walk | ✓ | 1.97.1 (MSRV 1.88) | — |
| python3 (stdlib `re`, `tomllib`) | checker | ✓ | 3.13.16 | — |
| PyYAML | (not for the checker) | ✓ system python only | 6.0.1 | Do not depend on it |
| shellcheck | `make lint-shell` | ✓ | 0.9.0 | — |
| cargo-deny / cargo-audit | `make security` | ✓ | present in `~/.cargo/bin` | — |
| cargo-llvm-cov | DOGF-05 | ✗ | — | Maintainer machine only (CI pins 0.8.7) |
| Docker CLI + Compose | DOGF-05 | CLI 29.8.2, Compose v5.6.0 present | **no daemon** (`/var/run/docker.sock` absent) | None here: DOGF-05 is a human-gated checkpoint |
| crates.io (via proxy) | DOGF-04 keyless half | ✓ | `cargo search/add/metadata/install` and the JSON API all worked | — |
| `gh` (authenticated) | fetching the CI coverage figure and run id | ✓ | `gh run list` returned runs | — |
| Free disk | any published-crate compile | **Tight** | 4.2 GB free; 0 MB reached during research | Large `CARGO_TARGET_DIR`, `cargo check`, delete targets after |
| Real provider key | DOGF-04 keyed half | ✗ (must not be in agent env) | — | `checkpoint:human-action` |

**Missing dependencies with no fallback:** a Docker daemon (DOGF-05) and a provider key (keyed DOGF-04) are human-gated by design.
**Missing dependencies with fallback:** disk (use another volume or hand the compile to the maintainer).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (libtest) for the round-trip test; bash harness with `assert_fire`/`assert_silent` for the checker |
| Config file | root `Cargo.toml` (no `autotests = false`, so `tests/*.rs` is auto-discovered); `Makefile` `test-shell-guards` globs `tests/scripts/*_test.sh` |
| Quick run command | `./tests/scripts/check-findings-log_test.sh` and `cargo test --test config_example_roundtrip` |
| Full suite command | `cargo test` + `make test-shell-guards` + `make check-gates` + `make clean-code` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DOGF-01 | Checker rejects `open`/missing/unrecognised/bad-dup/non-increasing and accepts the valid forms; real log parses with ≥ 4 entries | shell fixture | `./tests/scripts/check-findings-log_test.sh` | ❌ Wave 0 |
| DOGF-01 | After 56-02 the real log has no `open` | script on real log | `make check-findings-log` (exit 0) | ❌ Wave 0 (target + script) |
| DOGF-01 | DF-01..04 proofs still hold | existing tests + gates | the commands in Answer 7 | ✅ (names confirmed; gates run green today) |
| DOGF-02 | Return triaged: one entry per divergence, SHA check, or re-homed with owner+date | manual + checker | `make check-findings-log` + `grep` of the requirement row and the todo for owner/date | ❌ checkpoint (human) |
| DOGF-03 | Every key in the full fixture survives the production loader (modulo exemptions/`KNOWN_GAPS`); walker detects a planted typo; `KNOWN_GAPS` rows are live | integration | `cargo test --test config_example_roundtrip` | ❌ Wave 0 |
| DOGF-03 | Drift guard: every example key (live or commented) is in the full fixture | integration | same test file | ❌ Wave 0 |
| DOGF-03 | Verbatim example still loads and stays fail-closed | existing | `cargo test --lib --features web-server config::agents` etc. | ✅ |
| DOGF-04 | Walk evidence exists, every stumble is a DF with disposition | manual (keyed half) + checker | `test -f …/56-FIRST-RUN-WALK.md` + `make check-findings-log` | ❌ Wave 0 (agent keyless half) |
| DOGF-05 | Todo closed with measured figure, CI run id, commit; whole-percent agreement | manual (Docker) | `test -f .planning/todos/completed/2026-08-13-verify-local-coverage-reproduction.md` and `grep` for the figure | ❌ checkpoint (human) |
| DOGF-06 | Every `in v0.11.1` S1/S2 entry names a fix and its regression test; fixes keep gates green | checker + per-fix tests + gates | `make check-findings-log`, the fix's own test, `make api-surface`, `make check-semver` for public changes | ❌ Stage C |

### Sampling Rate

- **Per task commit:** the quick commands for the files touched; `shellcheck --severity=warning` on any new `*.sh`.
- **Per wave merge:** `cargo test`, `make test-shell-guards`, `make check-gates`, `make clean-code`.
- **Phase gate:** full suite green, `make check-findings-log` exit 0, `make security`, `make api-surface`, then `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `scripts/check-findings-log.sh` + `Makefile` target (with `## ` help comment) — DOGF-01
- [ ] `tests/scripts/check-findings-log_test.sh` (executable) with fixtures: open, missing, unrecognised, bare `in v0.11.1`, `out` without reason, `duplicate of` missing target, duplicate id, non-increasing id, fenced template ignored, last-bullet-wins, zero entries, unreadable, determinism, no-mutation — DOGF-01
- [ ] `tests/config_example_roundtrip.rs` + `tests/fixtures/config/config.example.full.yml` (+ redis-cadence variant) + provenance paragraph in `tests/fixtures/config/README.md` — DOGF-03
- [ ] `[dev-dependencies] serde_yaml = "0.9"` in the root manifest (optional; fallback is `config`) — DOGF-03
- [ ] Stage B runbooks (keyed walk, coverage walk with the LF/LH snippet, web3sec triage) — DOGF-02/04/05

## Security Domain

`security_enforcement` is not set to `false` in `.planning/config.json` (the key is absent), so this section applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no auth surface is added); L3/L-walk touch API-key handling | existing `AuthConfig::validate` fail-closed rules |
| V3 Session Management | no | — |
| V4 Access Control | no (DF-04 is already closed; any new fix touching tenants reuses `RunReadScope::permits_thread`) | existing |
| V5 Input Validation | yes | the checker parses an untrusted-ish markdown file with anchored regexes and no `eval`/shell interpolation; the round-trip test parses fixture YAML with `serde_yaml`/`config` |
| V6 Cryptography | no | — |
| V7 Error Handling and Logging | **yes** | failure messages carry key paths, never values; `Settings` is never `{:?}`-printed or serialised into output; evidence files redact keys; `log` facade only (D-00c) |
| V8 Data Protection | yes | fixture credentials are fake (`test-key-not-a-secret`); the real key lives only in the maintainer's shell and a scratch `.env` outside any repo, deleted afterwards |
| V14 Configuration | yes | the verbatim example must stay fail-closed (D-09); `deny_unknown_fields` stays as is (deferred) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Credential leaks into a test assertion, evidence file or log | Information disclosure | Key-path-only messages; redaction before truncation (`crates/paladin-llm/src/redaction.rs` rule applies to anything echoing a response body); manual review per `security.instructions.md` |
| `Debug`-printing a config that holds literal API keys (L3) | Information disclosure | Redacting `Debug` for `ApiKeyConfig` (additive), or remove the `{:?}` |
| Walk against HEAD or with `[patch]` masks a published-crate defect | Repudiation / integrity of the evidence | Scratch crate, registry source only, `--locked`, record `cargo tree` provenance |
| Real provider key persisted by `onboarding`'s `.env` | Information disclosure | Scratch directory outside git, explicit cleanup step in the runbook |
| A fix for a config-handling finding flips every section to `deny_unknown_fields` | Tampering with deployed configs | Deferred by CONTEXT; fix is targeted alias/rename only |
| SSRF/redirect rules for any new outbound HTTP | Spoofing | No new outbound client is expected; if a security fix adds one, follow the webhook guard pattern in `security.instructions.md` |

## Project Constraints (from CLAUDE.md)

The planner verifies compliance with these (same authority as locked decisions):

- **TDD**: failing test first; coverage floor **82 % workspace line coverage** (ADR-0006), enforced by `cargo llvm-cov --fail-under-lines`; all public APIs have doc tests.
- **Dependencies flow inward only**: core → nothing; application/ports → core; adapters → core + ports. Never import infrastructure from core or application.
- **Ubiquitous language**: Medieval Military terms (Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest, …) consistently; `Entity<T>` is deliberately generic; "node" means only a WarGraph/Campaign vertex (ADR-0060). New docs, comments and runbooks for this phase use these terms where they name a product concept.
- **Before committing a parent task**: `cargo test` → `cargo fmt --check` → `cargo clippy` → `make api-surface` (intentional public-surface change gets `make api-surface-update` + CHANGELOG entry), then a conventional-commit message. Stop after each major task and wait for go-ahead.
- **Security**: `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new/modified code, plus the manual credential-handling review in `.github/instructions/security.instructions.md`. CodeQL is advisory-only; **do not reintroduce Snyk** or record a phase as blocked on it.
- **No `unwrap()`/`expect()`/`panic!` in library code**; prefer borrowing; keep iterators lazy. (Test code and the gate scripts are exempt; the round-trip test is test code.)
- Run `make clean-code` (fmt + clippy + check) before committing; `shellcheck`/`actionlint` for every script/workflow edit.
- Conventional-commit messages as single-line `-m` flags; every Stage C fix commit cites its `DF-nn` (D-06).

## Sources

### Primary (HIGH confidence)

- The repository tree at HEAD `99e71602c` and tag `v0.11.0`: `config.example.yml`, `src/config/*.rs`, `crates/paladin-llm/src/config/llm.rs`, `src/bin/paladin-server.rs`, `src/main.rs`, `src/application/cli/commands/{setup_check,onboarding,arsenal}.rs`, `scripts/{check-version-strings,check-no-stub-panics,coverage}.sh`, `tests/scripts/*_test.sh`, `Makefile`, `.github/workflows/ci.yml`, `docs/src/contributing/testing-guide.md`, `docs/src/getting-started/{quickstart,installation}.md`, `README.md`, `Cargo.toml`, `Cargo.lock`, `deny.toml`.
- Phase records: `53-TENANT-EVIDENCE.md`, `53-VERIFICATION.md`, `54-FIRST-EVIDENCE.md`, `55-VERIFICATION.md`; todos `2026-08-13-verify-local-coverage-reproduction`, `2026-10-14-web3sec-single-bump-to-v0-11-0`; `47-HANDOFF-web3sec.md` §6.
- Scratch runs (2026-11-02, scratchpad, binaries since deleted for disk): a `config = "=0.15.23"` key-handling probe; a probe linking the real `paladin::config::Settings` over the verbatim example, a derived full fixture, and typo variants; `cargo install paladin-ai --version 0.11.0 --bin paladin-cli`; `cargo metadata` on the installation and quickstart manifests.
- crates.io JSON API: `paladin-ai 0.11.0` (bin names, features, MSRV, publication date), `paladin-llm 0.11.0` (features), `serde_ignored` (licence, downloads).
- `gsd-tools query package-legitimacy check` for `serde_yaml` and `serde_ignored`.

### Secondary (MEDIUM confidence)

- None used.

### Tertiary (LOW confidence)

- Items tagged `[ASSUMED]` in the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. No new package; every tool was inspected or run.
- Architecture (loader paths, re-serialisation, key handling): HIGH. Derived by grep and verified by running the real `Settings`.
- Pitfalls: HIGH for items reproduced here (L1, L2, key handling, disk); MEDIUM for the coverage-walk traps (read, not run: no Docker).
- Leads L3, L4, L6: MEDIUM. Read from the tree; each needs a Stage A reproduction before it becomes a `DF-nn`.

**Research date:** 2026-11-02
**Valid until:** 2026-11-16 (the log, the tree and CI move quickly; re-check Answers 1, 6 and 7 against HEAD at plan time, and re-run the crates.io probes if anything is published before then).
