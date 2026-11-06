# Phase 56: Dogfood Findings - Context

**Gathered:** 2026-11-02
**Status:** Ready for planning
**Mode:** `--auto` (every gray area auto-resolved to the recommended option; each choice is logged inline)

<domain>
## Phase Boundary

This phase is a **fixed frame around unknown content**. It finds, logs, dispositions and (for S1/S2)
fixes the defects v0.11.0 shows when used for real: on the published `0.11.0` crates, in the
maintainer's own runs and in the web3sec consumer's return. The log is
`.project/v0.11.1/02-dogfood-findings-log.md`.

It delivers six things (DOGF-01…06):

1. A disposition on every `DF-nn` entry, including the pre-seeded DF-01…DF-04.
2. The web3sec return triaged into the log, one entry per divergence, or DOGF-02 re-homed with an
   owner and a date.
3. A passing test that feeds each `config.example.yml` section through the real loader with every
   optional key present. Every key the loader rejects or silently ignores becomes a finding.
4. The first-run path walked on the published `0.11.0` crates, with every stumble logged.
5. The maintainer's local `make coverage` walk, which closes todo
   `2026-08-13-verify-local-coverage-reproduction` with the measured figure.
6. Every finding promoted `in v0.11.1` at S1/S2 fixed and verified before the phase closes.

**What it does not deliver:** new capabilities, new ports, adapters or HTTP routes (unless a
security fix needs one), finished stubs, provider measurement work, or anything that reopens an
accepted contract (inventory §H).

**Scout fact (2026-11-02):** the log holds only DF-01…DF-04, and no web3sec return is anywhere
in the tree. Every owner of DF-01…DF-04 is checked off in `REQUIREMENTS.md`: FIRST-01,
FIRST-02/03, FIXES-01, TENANT-03…05 and TENANT-07, all done in Phases 53-55.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by the milestone open, Phases 52-55 and the roadmap — not re-asked)

- **D-00a:** **Gates.** As Phase 55 D-00a:
  - 82 % workspace line coverage (ADR-0006).
  - `make clean-code` (includes `make doc-check`), `make check-gates`, `make test-shell-guards`.
  - `make api-surface`. An intentional public-surface change also gets `make api-surface-update`
    and a CHANGELOG entry.
  - `make security`, `make check-migration-allowlist`, `make check-version-strings`.
  - `make check-doc-config` for any fenced YAML touched in `docs/src`.
  - `shellcheck`/`actionlint` for every script and workflow edit.
  - The manual credential-handling review in `.github/instructions/security.instructions.md`.
  - No `unwrap`/`expect`/`panic!` in library code.
  - TDD: every fix is watched failing first, and a finding's reproduction becomes its regression
    test.
- **D-00b:** **Semver.** As Phase 53 D-02 and Phase 55 D-00b:
  - Every public change is judged at `--release-type patch` against `0.11.0` for all twelve
    packages. The milestone stays `v0.11.1`, the allowlist stays at zero entries and §11.2 stays
    header-only.
  - A change the gate flags is reshaped to additive or dropped.
  - Only an **S1 security fix with no additive shape** may take the forced-break branch: a §11.2
    row plus an allowlist entry in the same commit, and the milestone becomes `v0.12.0`. That call
    is recorded at the point the break is known (scoping §7), not at the tag.
  - Behavioural changes go in `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]`.
  - No `#[deprecated]` attribute is added anywhere.
- **D-00c:** **Frozen records and ADRs.**
  - `MIGRATION.md` §9/§10, the root `CHANGELOG.md` `[0.11.0]` section and every crate changelog's
    `[0.11.0]` section are byte-frozen.
  - Logging stays on the `log` facade.
  - No new ADR is expected. If one is needed, it takes the next free number from
    `.planning/decisions/PROMOTION.md` (currently **0064**) and advances that line in the same
    commit.
- **D-00d:** **Scope guards** (scoping §5, inventory §H):
  - No new ports, adapters or HTTP routes except for a security fix. Stubs are fenced, not
    finished.
  - A finding that collides with an accepted contract is logged with its ADR or row reference and
    dispositioned `out`, not reopened. The contracts are: WINDOWS 64 (agent-kind run, no
    Waypoint), 65 (ADR-0057 check-only boundary), 66 (streamed-call overshoot), 61 (OBS-05 waiver),
    audit G1 (no aggregate HTTP spend route), FUT-01…13, and `TokenUsage.total_tokens`.
  - Provider measurement work (Ollama / OpenAI-compatible probes, quota-body capture, C9) enters
    only if a run hands over a captured body.
  - The OpenAI Responses API is out (milestone decision 2026-10-27).
- **D-00e:** **Prove-then-lock** (Phase 53 D-00d through Phase 55 D-00e). A finding is re-checked
  against the current tree before it is promoted. A stumble seen on published `0.11.0` that HEAD
  already fixes is recorded with the fixing phase, its requirement and the proving command. It is
  not re-fixed. Line numbers have drifted, so locate items by content, never by line number.

### Findings-log mechanics and the DF-01…DF-04 dispositions (DOGF-01)

- **D-01:** **Append-only dispositions.** The log's own rule is "append; never rewrite an earlier
  entry (add a dated note)", so a disposition is a new dated bullet appended under the entry, for
  example `- **Disposition (2026-11-xx):** in v0.11.1 — fixed by …`. The original lines, including
  `open → recommend …`, stay byte-intact. An entry's **disposition is its last
  `**Disposition…:**` bullet.** New findings are appended below the
  `<!-- append new findings below this line -->` marker, take the next free `DF-nn`, and use the
  entry format at the top of the file.
  `[auto] Log edits — Q: "Edit the Disposition line in place or append a dated note?" → "Append a dated note; last Disposition bullet wins" (recommended)`
- **D-02:** **DF-01…DF-04 close as `in v0.11.1` with a pointer to the owner's evidence**, not as
  `duplicate of`, since there is no other DF entry they duplicate. Each pointer names the owning
  requirement, the phase, and the regression test or gate that proves the fix:
  - DF-01 → FIRST-01 (Phase 54)
  - DF-02 → FIRST-02/03 and the `check-version-strings` gate (Phase 54)
  - DF-03 → FIXES-01 and the stub-panic gate (Phase 55)
  - DF-04 → TENANT-03…05 and TENANT-07 (Phase 53)

  Each is re-checked first under D-00e, by running the named test or gate on HEAD. The log says
  pre-seeded entries "need a real reproduction before promotion". That requirement is met by the
  owner's failing-first regression test, which is cited, so no live dogfood run is needed for
  these four.
  `[auto] Pre-seeded closure — Q: "How do DF-01…04 close?" → "in v0.11.1, pointer to owning requirement + proving test" (recommended)`
- **D-03:** **"None reads `open`" is checked by a script, not by eye. The script is not in
  `make check-gates`.**
  - A small offline checker (name is the planner's, e.g. `scripts/check-findings-log.sh` with a
    `make check-findings-log` target) parses each `### DF-nn` entry's last disposition bullet. It
    fails, naming each id, when any is `open`, missing, or not one of `in v0.11.1` / `out (<reason>)`
    / `duplicate of DF-nn`.
  - It also checks `duplicate of DF-nn` targets exist, and that ids are unique and increasing.
  - It is run at phase verification and by Phase 58's SHIP-10.
  - It is **not** wired into `make check-gates` or CI, because the log legitimately holds `open`
    entries mid-phase. A CI gate would turn every new finding into a red build for unrelated PRs.
  - It gets the house shell-guard test (`tests/scripts/…_test.sh` with pass/fail fixtures, wired
    into `make test-shell-guards`), so the checker itself is proven.
  `[auto] Open-check — Q: "CI gate, phase-close script, or manual read?" → "Standalone script + make target, run at verification and SHIP-10; not in check-gates" (recommended)`
  — **Reversibility:** reversible.

### Triage policy — severity, promotion and where fixes land (DOGF-01, DOGF-06)

- **D-04:** **Severity scale as written in the log** (S1 security/isolation · S2 wrong result or
  crash · S3 misleading docs/config · S4 hygiene). The triager assigns severity at logging time and
  states it in the entry. A finding that is S1 if shipped as written (the DF-01 shape) is graded at
  its worst credible deployment.
  - A web3sec divergence that breaks the consumer's compile or tests is **S2 by default**: the
    milestone's promise is a patch a consumer takes without adaptation.
  - A muster gap with no behavioural effect is S3.
- **D-05:** **Disposition rule by severity.**
  - **S1/S2:** always `in v0.11.1` (DOGF-06), unless it collides with an accepted contract (D-00d,
    then `out` with the reference) or does not reproduce on HEAD (D-00e, then `out (fixed by
    Phase N — <requirement>)` or `duplicate of`).
  - **S3/S4:** `in v0.11.1` only when the fix is contained. That means a docs or example change, or
    a single-file code change, that rides an existing plan in this phase without its own design
    decision. Otherwise `out (re-homed: <todo file>)`, with a pending todo filed in
    `.planning/todos/pending/` for the next milestone, so nothing is dropped.
  `[auto] S3/S4 policy — Q: "Fix every S3/S4, or only contained ones?" → "Contained ones in; the rest out with a filed todo" (recommended)`
- **D-06:** **Where S1/S2 fixes land.** By default a fix is a plan inside Phase 56, added by a
  `--gaps` replan once the findings are logged. A **decimal phase (`56.1`+, via
  `/gsd-phase --insert`)** is inserted only when the fix needs its own design discussion: a choice
  between behaviours, a semver or ADR question, or work across more than one crate's public
  surface. Decimal phases inherit DOGF-06 and do not change the requirement mapping (roadmap scope
  note). Every fix commit cites its `DF-nn` in the message, so the roadmap row, the fix commit and
  the log stay cross-referenced.
  `[auto] Fix placement — Q: "Inline plans or decimal phases?" → "Inline by default; decimal only for fixes needing their own design call" (recommended)`
- **D-07:** **Plan shape: a frame of three stages.**
  - **Stage A** — agent-runnable now, no external input: DOGF-01's DF-01…04 closure and the
    checker; DOGF-03's config round-trip; the keyless half of DOGF-04 (D-14).
  - **Stage B** — human-gated checkpoints (`checkpoint:human-action` / `checkpoint:human-verify`):
    the keyed half of DOGF-04, DOGF-05, and the DOGF-02 return triage. Each has a written runbook
    with exact commands and a place to paste output.
  - **Stage C** — fix plans written from the log after A and B have produced findings (D-06).

  Stage A must not wait on Stage B. The phase closes only when Stage C is empty: every promoted
  S1/S2 is fixed and verified, and the D-03 checker is green.
  `[auto] Phase shape — Q: "Plan everything up front or stage it?" → "Three stages: agent-runnable, human-gated, fixes-from-log" (recommended)`

### Config round-trip test design (DOGF-03)

- **D-08:** **The test goes through the production loader, never raw `serde_yaml`.** The config
  crate's own handling is where this class of bug lives (scoping §6 "config loading in general";
  B1 and Phase 44 WR-03 came from it). That handling covers source merging, key handling and
  `try_deserialize`.
  - The test loads via `Settings::load_from_file`.
  - The research step also enumerates every *other* path the server and CLI use to read
    `config.example.yml` sections, such as `paladin-llm`'s own `config/llm.rs` loader and the
    `deserialize_wrapper_from_file` wrappers in `treasurer.rs` and `agent_runtime.rs`. Each
    section is tested through each production path that reads it.
  `[auto] Loader path — Q: "Production loader or serde_yaml directly?" → "Production loader(s) only" (recommended)`
- **D-09:** **"Every optional key present" comes from a committed full fixture plus a drift guard.
  The example itself is not uncommented.** The shipped example deliberately leaves blocks
  commented out, including treasurer `pricing`/`allowance`, `cadence`, `api_keys` and the
  provider model overrides. Turning them on in the example would change what a verbatim copy does.
  - A fixture (path is the planner's, e.g. `tests/fixtures/config/config.example.full.yml`) is
    `config.example.yml` with every commented-out optional key enabled and every optional key the
    `Settings` tree accepts for those sections given a value.
  - Dummy credentials are obviously fake, e.g. `test-key-not-a-secret`, never `${…}`.
  - A **drift guard test** fails when a key path appears in `config.example.yml` (live or
    commented) and not in the full fixture, so a key added to the example later cannot escape the
    round-trip.
  - "Each section" means each top-level section present in `config.example.yml`. Settings fields
    the example does not mention are out of scope here.
  `[auto] Optional keys — Q: "Uncomment in the example, a full fixture, or text-munge the comments in-test?" → "Committed full fixture + drift guard against the example" (recommended)`
  — **Reversibility:** reversible.
- **D-10:** **"Silently ignored" is detected by key-path comparison. Known gaps are pinned to DF
  ids in an allowlist.**
  - The test loads the fixture and re-serialises the resulting `Settings` (`Settings` derives
    `Serialize`), then compares key-path sets. Every key path in the input must appear in the
    output. An input key missing from the output was dropped by the loader.
  - Map keys are compared exactly, case-sensitively.
  - Fields that are deliberately `skip_serializing` (secrets, for example) go in a small explicit
    exemption list, one reason per row.
  - If research finds re-serialisation lossy in a way the exemption list cannot express, the
    researcher may substitute `serde_ignored`. It is a new dev-dependency and must pass
    `make deny`. The observable rule is unchanged.
  - A key the loader **rejects** fails the load, so it shows up as a test failure naming the key.
  - Each rejected or ignored key is logged as its own `DF-nn` (DOGF-03). The test then asserts
    **zero unexplained keys**: any remaining gap sits in a `KNOWN_GAPS` table where every row cites
    its `DF-nn`. Fixing the finding deletes the row, and a row whose key now round-trips fails the
    test, so the table cannot go stale.
  `[auto] Ignored-key detection — Q: "How is a silently-ignored key detected?" → "Re-serialise and diff key paths; KNOWN_GAPS rows cite DF ids" (recommended)`
- **D-11:** **Fixture rows that probe known bug shapes.** The full fixture deliberately carries:
  - **Mixed-case and punctuated map keys** where the YAML uses user-chosen keys: treasurer
    `allowance.tenants`/`api_keys` names, `pricing` model ids with `/`, `.` and `-`, and provider
    `models:` overrides. The config crate's key handling is a known risk for exactly these.
  - Numeric strings for decimal money fields (`"2.50"`).
  - Both `temperature` override shapes (`{fixed: …}`, `{range: […]}`) and `not_sent`.
  - An `api_keys` entry with a literal key.

  Whatever reads back different is a finding under D-10.
- **D-12:** **Environment overlays (`APP_…`) are out of this test.** DOGF-03 is about file
  round-trip. An env-overlay surprise seen during the DOGF-04 walk is logged there.

### First-run walk on published 0.11.0 (DOGF-04)

- **D-13:** **The walk runs against crates.io `0.11.0`, never path dependencies or HEAD.** It runs
  in a scratch crate outside the workspace, with no `[patch]` and no workspace inheritance.
  - The steps follow the README verbatim, in this order: README Quick Example → `cargo add` (or
    the README's `Cargo.toml` snippet as written) → the `basic_paladin` example → install
    `paladin-cli` from the published package → `paladin-cli onboarding` → `paladin-cli setup-check`.
  - Each command, its exit code and its output (secrets redacted) go into a phase evidence file
    (name is the planner's, e.g. `56-FIRST-RUN-WALK.md`).
  - Every stumble is logged as a `DF-nn`, then triaged under D-00e. For example, the 0.8.0 version
    line becomes `out (fixed by Phase 54 — FIRST-02)`.
- **D-14:** **The agent runs the keyless half; the maintainer runs the keyed half from a runbook.**
  - The agent does everything up to the first provider call: resolve, compile, run until the
    missing-key error, `onboarding`/`setup-check` with no key. It logs those stumbles, including
    how clear the missing-key message is.
  - The real-key half is a Stage B `checkpoint:human-action`. The maintainer runs the same commands
    with one real provider key and pastes the redacted output. The key never enters the repo, a
    log or the evidence file.
  - **Provider:** the one the README Quick Example uses, so the walk matches the docs verbatim.
    Any extra provider the maintainer has a key for is optional and logged the same way.
  `[auto] Walk split — Q: "Who runs which half, and on which provider?" → "Agent keyless half; maintainer keyed half on the README's provider" (recommended)`

### web3sec return (DOGF-02)

- **D-15:** **No return is simulated, inferred or reconstructed.** Triage starts only from the
  consumer's actual §6 return in `47-HANDOFF-web3sec.md`: commit SHA, D-2 choice, gate totals
  against their own named pre-bump figure, and divergences.
  - Each divergence, including any real `use` line the muster missed and any §10.8 step that did
    not read as written, becomes one `DF-nn` with **By:** web3sec.
  - A gate-total regression against web3sec's latest figure (2,683 / 0 / 2) is its own entry.
  - The SHA is checked against `70657919beadbd341a72be2d0107080017a803f0`.
- **D-16:** **If the return has not arrived when everything else is done, ask the maintainer.**
  This is a `checkpoint:decision`, never an agent default. The maintainer either holds the tag or
  re-homes DOGF-02 with a **named owner and a date**, recorded in the requirement row and in todo
  `2026-10-14-web3sec-single-bump-to-v0-11-0`. SHIP-10 admits only fixed, waived-with-reason or
  re-homed-with-owner-and-date. A late return is appended to the log whenever it arrives, even
  after the phase closes, and triaged under D-04/D-05.
  `[auto] Missing return — Q: "Block the phase, or decision checkpoint at the end?" → "Never block Stages A/C; maintainer decision checkpoint at close" (recommended)`

### Local coverage walk (DOGF-05)

- **D-17:** **Pass rule: whole-percent agreement with CI on the same commit.**
  - The maintainer runs `docs/src/contributing/testing-guide.md`'s procedure verbatim
    (`make services-up` → `make coverage`) on a Docker-capable machine, at a commit whose CI
    `coverage` job figure is also recorded.
  - Agreement means the local and CI line-coverage figures truncate to the same whole percent and
    both are ≥ 82 (the todo's own bar).
  - Every undocumented prerequisite, missing tool or step that did not read as written is an S3
    `DF-nn` and a contained docs fix under D-05.
  - The todo moves to `.planning/todos/completed/` with the measured local figure, the CI run id
    and the commit.
  - This is a Stage B `checkpoint:human-action` with a runbook. No agent environment here has
    Docker.

### Claude's Discretion

- Script, make-target, fixture and evidence-file names (D-03, D-09, D-13), and how D-10's key-path
  walker is written. A `serde_json::Value` walk, `serde_ignored`, or similar are all acceptable.
- Whether DOGF-03 is one test per section or one table-driven test, and where it lives
  (`src/config/` unit tests or a `tests/` integration test). The existing per-section
  `config.example.yml` tests in `web_server.rs`, `agents.rs`, `treasurer.rs`, `agent_runtime.rs`
  and `agent_host.rs` stay; the new test supplements them.
- Runbook layout for the Stage B checkpoints.
- How DF entries for one walk are grouped into plans for the Stage C replan.

### Folded Todos

- **`2026-08-13-verify-local-coverage-reproduction`** — "Verify local make coverage reproduces
  CI's 82.39% figure" (score 0.9). The testing guide's local coverage procedure has never been
  walked by a human on a Docker machine. This is DOGF-05 exactly (inventory G4,
  `resolves_phase: 56`), and it closes under D-17.
- **`2026-10-14-web3sec-single-bump-to-v0-11-0`** — "web3sec moves once, straight to v0.11.0"
  (score 0.6). It waits on the consumer's §6 return. This is DOGF-02 exactly (inventory G1). The
  return is triaged under D-15, or the todo is re-homed under D-16.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md` § "Phase 56: Dogfood Findings" — goal, five success criteria, scope notes
  (fixed frame, decimal phases inherit DOGF-06, human-gated DOGF-04/05, §H collisions → `out`)
- `.planning/REQUIREMENTS.md` § "Dogfood Findings (DOGF)" — DOGF-01…06 wording and traceability rows
- `.planning/PROJECT.md` § "Current Milestone" — milestone decisions of 2026-10-27 (Responses API out,
  `${VAR}` auth keys refused at boot, stubs fenced not finished)

### The findings log and its milestone inputs
- `.project/v0.11.1/02-dogfood-findings-log.md` — THE log: entry format, severity scale, append-only
  rule, "surfaces worth deliberately exercising", DF-01…DF-04
- `.project/v0.11.1/01-carried-gaps-inventory.md` § G (G1 web3sec, G4 local coverage) and § H
  (accepted contracts — the `out` list)
- `.project/v0.11.1/03-suggestions-and-scoping.md` § 5 (scope guards), § 6 (likely-bug shapes:
  agent-kind run path, reasoning-effort refusals, config loading in general, onboarding/setup-check,
  "unprofiled" trace signals), § 7 (patch vs `v0.12.0` naming)
- `.project/v0.11.1/00-README.md` — how the three milestone input files relate

### web3sec return contract (DOGF-02)
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-HANDOFF-web3sec.md` § 6 — return
  contract (SHA `70657919…`, D-2 choice, gate totals vs 2,683 / 0 / 2, divergences)
- `MIGRATION.md` § 10.8 — "Library consumers: v0.10.1 → v0.11.0" (the steps a divergence is measured against; frozen)
- `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` — folded todo, status history

### Local coverage walk (DOGF-05)
- `docs/src/contributing/testing-guide.md` § "Test Coverage" / Code Coverage — the procedure walked
- `.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md` — folded todo, CI baseline record
- `.planning/decisions/0006-coverage-gate.md` — ADR-0006, the 82 % floor
- `Makefile` (`coverage`, `services-up` targets) and `.github/workflows/ci.yml` `coverage` job — locate by name

### Config round-trip (DOGF-03)
- `config.example.yml` — the sections under test (`llm_*`, `llm`, `max_file_size`, `sources`, `server`,
  `timeouts`, `http`, `agent_runtime`, `treasurer`, `trace`, `web_server`, `agents`)
- `src/config/settings.rs` — `Settings`, `Settings::load_from_file` (production loader)
- `tests/fixtures/config/README.md` — fixture provenance conventions (frozen-copy discipline)

### Prior phase decisions this phase builds on
- `.planning/phases/53-tenant-isolation-closure/53-CONTEXT.md` — D-00d prove-then-lock; D-02 version decision
- `.planning/phases/53-tenant-isolation-closure/53-VERSION-DECISION.md` — `v0.11.1` patch call
- `.planning/phases/54-first-run-usability/54-CONTEXT.md` — FIRST-01 refusal rule, example shape (DF-01/DF-02 owners)
- `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-CONTEXT.md` — D-00a gates; FIXES-01 stub-panic gate (DF-03 owner)
- `.planning/WINDOWS.md` — cross-phase defect register; Phase 57 sweeps rows minted here
- `.planning/decisions/PROMOTION.md` — next free ADR number (0064)

### Security review
- `.github/instructions/security.instructions.md` — manual credential-handling review (any fix touching a key, a response body, or the walk's redaction)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Per-section `config.example.yml` tests already exist.** These are the precedents for loading
  the example:
  - `src/config/web_server.rs` (~145-177) and `src/config/agents.rs` (~1083-1102) load through
    `Settings::load_from_file("config.example.yml")`.
  - `src/config/treasurer.rs` (~901, ~941) and `src/config/agent_runtime.rs` (~1626, ~1811) use a
    local `deserialize_wrapper_from_file` helper.
  - `crates/paladin-llm/src/config/llm.rs` (~758) reads the example via
    `concat!(env!("CARGO_MANIFEST_DIR"), "/../../config.example.yml")`.
  - `src/infrastructure/web/agent_host.rs` (~2293) checks the example cannot refuse itself.
- **The house shell-gate shape:** `scripts/check-version-strings.sh`, `scripts/check-crate-names.sh`
  and the Phase 55 stub-panic gate. Each is bash with embedded `python3`, takes no argument, runs
  in about a second, reports `file:line`, treats "scanned zero" as a named failure, and has a
  `tests/scripts/*_test.sh` fixture test. D-03's checker copies this shape.
- `tests/fixtures/config/` holds config fixtures with a provenance README. D-09's full fixture goes
  here with its own provenance note: it is derived and hand-maintained, not a frozen tag copy.
- `examples/basic_paladin.rs` is the in-tree twin of the README walk. D-13 walks the *published*
  equivalent, not this file.
- `src/bin/paladin-cli.rs` holds the `onboarding` and `setup-check` commands. Their live tests are
  `#[ignore]` ("Requires valid API key").

### Established Patterns
- **Unknown keys are silently accepted almost everywhere.** In `src/config/`, only `treasurer.rs`
  uses `#[serde(deny_unknown_fields)]`. Every other section deserialises with serde's default and
  drops unknown keys without a word. This is why D-10 detects by key-path diff and does not rely on
  load errors.
- `Settings` derives `Serialize`, which D-10's re-serialisation depends on. The researcher must
  check every nested config type also serialises its fields faithfully: no lossy custom
  `Serialize`, and `skip_serializing` fields are listed.
- Loader: `config` crate `0.15.11` (`Config::builder().add_source(File::new(..)).build()?
  .try_deserialize()`). Its key handling for user-chosen map keys (case, `.` in keys) is the D-11
  probe target. The researcher confirms the actual 0.15 behaviour.
- `config.example.yml` deliberately ships commented-out blocks, most of the 48 commented key
  lines: provider `max_tokens`/`context_window`/`models` overrides, `http.auth.api_keys` entries,
  treasurer `pricing`/`allowance`/`cadence`. The example's "verbatim copy fails closed" property
  (Phase 54 D-04) must stay true, which is why D-09 does not uncomment it.

### Integration Points
- New DF entries go in `.project/v0.11.1/02-dogfood-findings-log.md`, below the append marker.
- Fix plans (Stage C) touch whatever the findings name. Each fix adds a regression test (D-00a),
  a `CHANGELOG.md` `[Unreleased]` entry where user-visible, and `MIGRATION.md` §11.1 for a
  behavioural change.
- Phase 57 (Register Hygiene) depends on this phase. Any `WINDOWS.md` row minted or closed here
  must be stated in the SUMMARY so Phase 57's `open_count: 0` sweep includes it.
- Phase 58's SHIP-10 reads the log through D-03's checker.

</code_context>

<specifics>
## Specific Ideas

- D-11's fixture should deliberately include a tenant name such as `Acme-Prod` and a pricing model
  id such as `Qwen/qwen3.7-plus` or `gpt-5.5.1`. If either reads back lowercased or split at the
  dot, that is a high-value finding: tenant names and model ids are matched exactly downstream
  (Phase 53 D-00f reserved names are case-sensitive).
- Watch-list from scoping §6, to be exercised during the DOGF-04 walk and logged if it bites:
  - Does a reasoning-effort refusal name the config key?
  - Does `paladin.unprofiled_model` / `paladin.usage_unreconciled` fire on the README provider?
  - How clear is the missing-key error from `basic_paladin`?
- Each `DF-nn` from the walk quotes the verbatim error text in a code block, as the entry format
  asks, with keys redacted.

</specifics>

<deferred>
## Deferred Ideas

- **Environment-overlay (`APP_…`) round-trip.** A natural sibling of DOGF-03, but not in its
  wording (D-12). Note it for a later config-hardening pass if the walk turns up an overlay
  surprise.
- **Turning `deny_unknown_fields` on across config sections.** If DOGF-03 finds silently-ignored
  keys, the *fix* for a given finding may be a targeted rename or alias. Flipping every section to
  `deny_unknown_fields` is a behavioural change: configs that loaded would refuse. That is a
  milestone-level call, not something to slip into a fix plan.
- **Live probes of Ollama / OpenAI-compatible, and walks on non-README providers.** These are
  provider-parity follow-up work (scoping §5) unless a run hands over a captured body.

### Reviewed Todos (not folded)
- **`2026-10-20-openai-responses-api-for-pro-tier`** — "Add OpenAI Responses API support so the
  pro tier is usable" (score 0.6). The auto-fold rule (score ≥ 0.4) would have folded it. It is
  **not folded** because it conflicts with a locked milestone decision (2026-10-27: "OpenAI
  Responses API out") and is feature work by shape (inventory G2). It stays pending as the first
  item of the next feature milestone. If a DOGF-04 walk hits a pro-tier 404, log it as a `DF-nn`
  and disposition it `out (milestone decision 2026-10-27; todo 2026-10-20-…)`.

</deferred>

---

*Phase: 56-dogfood-findings*
*Context gathered: 2026-11-02*
