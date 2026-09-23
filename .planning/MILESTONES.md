# Milestones

## v0.10.0 Durable Agent Execution Runtime (Shipped: 2026-09-23)

**Phases completed:** 19 phases (22-37.1, including inserted 22.1, 36.1 and 37.1), 231 plans
(228 executed, 3 superseded — plans 37-09..37-11 were overtaken by Phase 37.1), 574 tasks
**Requirements:** 88/89 satisfied (ENG-01…08, CF-01…05, HITL-01…05, FT-01…06, RT-01…07,
PLAT-01…06, OBS-01…04, SHIP-01…04, SHIP-06, VOCAB-01…07, ACCT-01…05, PRIM-01…05, COMM-01…04,
CURR-01…21); SHIP-05 superseded by SHIP-06 (see *Known Gaps*)
**Timeline:** 2026-09-01 → 2026-09-23 (23 days, 1,678 commits since tag `v0.9.0`)
**Git range:** `495483ef` (milestone opened) → `6a08c293` (close)
**Closeout type:** override_closeout — 0 unverified phases (all 19 `VERIFICATION.md` files read
`passed`, `behavior_unverified: 0`); 1 phase the tooling reads as incomplete (Phase 37: 8 of 11
plans executed, 3 superseded without a SUMMARY, by design); 2 open artifacts acknowledged (both
`todos/pending/` files already dispositioned as deferred past v0.10.0 by Phase 36.1, CURR-20);
see STATE.md *Deferred Items*
**Audit:** `milestones/v0.10.0-MILESTONE-AUDIT.md` (status `tech_debt` — 88/89 requirements,
19/19 phases, 10/10 integration seams wired, 6/6 E2E flows, 0 gaps; debt items recorded with
owners)
**Tags:** `v0.10.0` on merge commit `1d4a9724` (2026-09-18; published 3 of 12 crates, all three
since yanked; GitHub Release kept, bannered, flipped to pre-release) and `v0.10.1` on merge commit
`f7dae267` (2026-09-21; release run `35659477719` green, all 12 publishable crates on crates.io
at `0.10.1` via Trusted Publishing). The milestone identity is `v0.10.0`; the crates a consumer
installs are `0.10.1`.

**Delivered:** Paladin is now a durable agent execution runtime rather than a pattern-oriented
orchestration library: a cyclic superstep engine over typed shared state that checkpoints every
superstep and resumes with zero re-execution, pauses indefinitely for a human, retries and falls
back per node, runs in the background behind a durable queue and a versioned HTTP platform API,
emits a machine-consumable trace stream with a regression harness to assert on it, accounts for
every token losslessly and rations context through one officer — with the whole surface
documented to currency and published to crates.io at one coherent version.

**Key accomplishments:**

- **A durable superstep engine, proven by crash-resume rather than described (Phases 22-24).**
  `WarGraph`/`WarEngine` run cycle-permitting graphs over a typed `Battlefield` with deterministic
  delta merge, persist exactly one `Waypoint` per superstep to in-memory, SQLite or Postgres
  stores that all pass one 13-function `WaypointPort` contract suite, and resume from any
  Waypoint (including a persisted `FrontierSnapshot` and intra-superstep `MusterProgress`) with
  zero re-execution. Control flow is node-driven (`Directive`/`NextStep`, Muster fan-out, nested
  Battalion subgraphs, LLM-evaluated edges behind a fail-closed registry that fixed BUG-01);
  Parley pauses a run into an `AwaitingInput` Waypoint, `resume_with` validates every response
  before writing anything, Chronicle replay/fork proves mainline immutability byte-for-byte, and
  cancellation drains a whole in-flight batch against one grace deadline. Two engine readiness
  defects (BUG-03 cycle-bootstrap starvation, BUG-04 resume-frontier loss) landed test-first in
  the inserted Phase 22.1, which also measured the MSRV at 1.88 instead of declaring it.

- **Fault tolerance and an agent runtime a consumer can shape (Phases 25-26).** A table-driven
  `Transience` taxonomy on every error enum, structured `NodeError`, per-node `Aegis`
  retry/timeout/typed handlers, a chain-composing `FallbackLlmAdapter` that never hops after a
  streamed chunk, and node result caching (in-memory + Redis) with a shared contract suite. On
  the agent side: an ordered `ExecutionMiddleware` chain proven byte-identical on an empty chain,
  twelve built-ins under one `AgentRuntimeConfig` (call/token/tool limits, guardrails, history
  trimming, summarization, Vault recall), the `Vault` long-term memory vocabulary and first
  adapter, and `ResponseFormat` structured output reaching the wire on four adapter paths.

- **A background-run platform and observability that other tools can consume (Phases 27-28).**
  `POST /v1/runs` persists, enqueues and executes on a `RunWorkerPool` with lease heartbeats,
  resume-not-restart redelivery and drain-on-shutdown, over `RunQueuePort` (in-memory + Redis
  ZSET/Lua) and `RunRepositoryPort` whose partial unique index *is* the 409 ThreadBusy invariant;
  `WarGraphDoc` gives graphs a schema-derived document form; versioned assistants, schedules and
  HMAC-signed webhooks with a table-tested SSRF guard at write and send time. A twelve-variant
  `TraceEvent` envelope with panic-isolated composite sinks (log, OTel, SSE), a `RunTracePort`
  with three adapters, Mermaid/DOT exporters frozen by golden files, and the new `paladin-eval`
  crate (scenario format, scripted `ScenarioLlm`, twelve evaluators).

- **Token economy without loss and without a second vocabulary (Phases 30-33).** ADR-0049/0050/0051
  fixed the two-officer model (`Commissary` kept, `Treasurer` reserved for Milestone 14) and the
  units-plain/roles-medieval naming rule; six-field `TokenUsage` now travels intact from the LLM
  port through `RunFinished`, both heralds and the HTTP edge (`from_total` deleted, ~99 literal
  sites migrated), with terminal-chunk streaming usage parity on every adapter; one counting
  contract (`TokenCounterPort::is_exact`) and one shared `resolve_context_window` replace two
  independent walks and the duplicate `paladin-memory` trait; RAG retrieval rations through
  `Commissary::dispense` with shed records and an omission marker — the last silent-truncation
  path in the tree, closed with a proptest and an ungated integration test.

- **Program gates that were re-sealed, not re-read (Phases 29, 33, 37, 37.1).** `MIGRATION.md`
  complete with a CI grep-gate against placeholder markers, a frozen v0.9 config that boots v0.10
  with every new subsystem inert, an OpenAPI golden diff over the six v0.9 routes, a 138-row
  per-FR evidence table, three program E2E scenarios plus their eval dogfood copies, the `semver`
  job comparing `crate | type` pairs, `WINDOWS.md` triaged to `open_count: 0` — and the full gate
  set re-run on the final commit three separate times (Phase 33, Phase 37, Phase 37.1).

- **Documentation brought to currency by inventory first, then closed by ID (Phases 34-36.1).**
  One read-only audit measured the debt (94 mdBook rows, 143 rustdoc rows — the carried "14
  unresolved links" was undersized 5.5×, 122 example rows) before anything was edited; every row
  was then closed by ID: the superstep-engine guide written with compile-verified `doc-examples`
  modules, the CLI appendix rebuilt from live `--help`, `cargo doc` taken from 65 warnings to
  zero and the `--all-features` bar from 77 errors to zero with no visibility widened, 14 new
  offline example programs, all 101 public-API entry points carrying a `# Examples` doctest
  behind a new gate, and every deferred register walked to `open_count: 0` before the tag.

- **The release itself — and the second one that finished it (Phases 37, 37.1).** `v0.10.0` was
  tagged on `main` by the documented flow and its pipeline published 3 of 12 crates before a
  publish-order defect the local dry-run gate was structurally blind to; Phase 37.1 fixed both
  defects (path-only workspace dev-dependencies; the `EPIPE` race in
  `create-or-reuse-release.sh`, with a 200,000-character regression case), built an offline
  `cargo metadata`-driven publish-order gate proven red on the broken tree and wired into
  `make check-gates` and CI, and released `v0.10.1` with all twelve crates registry-verified
  (`yanked: false`, `paladin-eval`'s first pipeline publish carrying `trustpub_data`) and the
  three orphans yanked by the maintainer with one register row each.

### Known Gaps

- **SHIP-05 — superseded, deliberately unticked.** "v0.10.0 is released" never became literally
  true: the `v0.10.0` tag's pipeline cannot complete forward because `release.yml` reads the
  `CRATES` order from the tag ref. The outcome it was written to secure is delivered by SHIP-06
  (`v0.10.1`). Recorded at source in `milestones/v0.10.0-REQUIREMENTS.md` (amend-at-source note
  dated 2026-09-22) and in `37-VERIFICATION.md`.
- **Phase 37 plans 37-09..37-11 — superseded, never executed.** Their scope (post-publish
  verification and milestone-close recording for `v0.10.0`) moved into Phase 37.1; the plan files
  carry dated supersession notes and no SUMMARY, which is why `init.manager` reads Phase 37 as
  incomplete although its verification passed.

**Known deferred items:** the audit's `tech_debt` register, with owners — the accepted tracing
overhead deviation (+22.18 % log sink / +18.46 % composite against PRD 07's ≤ 3 % bar; D-16,
`WINDOWS.md` row 35, candidate optimisation target `TraceDispatcher::emit`); the legacy
`Runnable::Agent` path emitting no SSE bus events or webhook deliveries (row 31) and the
single-tenant scoping of the run-inspection routes (row 32); the SSE `done` event reporting
`halted` for a caller-cancelled run whose persisted status is `Cancelled` (D-14); the seven
judgment-tier sign-off boxes in `.project/v0.10.0/09-program-acceptance-audit.md` §11 still
`- [ ]` on disk although 29-UAT recorded the pass; seven phases (22, 24, 29, 30, 34, 36, 36.1)
with `VALIDATION.md` at `status: draft` and Phase 28's at `nyquist_compliant: false` pending a
re-run; the terminal quay.io MinIO pin and its RustFS evaluation (FUT-10, todo); the user-owned
coverage-reproduction walkthrough carried since v0.8.0; the three roadmap-level v2 debt lines
(oversized service files, clone/lock contention, allowlist drift); and Milestone 14 (Treasurer),
reserved by ADR-0050 and not roadmapped. Also carried, from `security.instructions.md`: the
webhook SSRF guard does not pin the resolved address between check and connect (DNS rebinding is
a documented limitation).

### Release record — the two tags

**`v0.10.0` — tagged, only 3 of 12 crates published.** Annotated tag `v0.10.0` (object
`9282f4da38bb19f75ce3ed488c10454bdf254990`, tagger `Am0rfu5`, message "v0.10.0 Durable Agent
Execution Runtime") sits on `main` merge commit `1d4a9724cc219b85856a23012543458d62559e47`
(PR #55). `release.yml` run
[35404826303](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35404826303) published only
`paladin-ai-core`, `paladin-ports` and `paladin-herald` at `0.10.0` before `publish-crates` failed
deterministically at `paladin-battalion` (position 4 of 12). Two defects caused this:

1. **A response-size race in `scripts/create-or-reuse-release.sh:79`** — `printf '%s\n' "${raw}" |
   head -n1` under `set -o pipefail` takes an `EPIPE` when `head` closes the pipe before `printf`
   finishes writing a large response; the `v0.10.0` release body (46,274 bytes) crossed the
   threshold that `v0.9.0`'s smaller body (25,677 bytes) never had. This first blocked
   `Create Release` itself; the maintainer's manual retries eventually won the race.

2. **`paladin-battalion`'s two versioned workspace dev-dependencies** (`paladin-llm`,
   `paladin-storage`, introduced by Phases 23 and 28) named a registry `version` that could only
   resolve once those crates were already published — but `CRATES` publishes `paladin-battalion`
   at position 4, before `paladin-llm` (5) and `paladin-storage` (10). There is no true dependency
   cycle; this is an ordering defect.

**Why the local dry-run gate was green and blind.** `make publish-dry-run` (Local sweep row 29)
ran `cargo publish --workspace --dry-run`, which resolves sibling crates from a local workspace
overlay rather than the live registry index — so it is structurally incapable of seeing the
per-crate resolution order the real `cargo publish` loop depends on. The row was green and its
underlying command genuinely passed; it was simply blind to this class of defect. Phase 37.1
plan `37.1-02` built a gate that exercises the real per-crate resolution path and proved it red on
this exact tree before fixing it.

**`v0.10.1` — the release.** Annotated tag `v0.10.1` (object
`7593ab4dc1217ef23d6f8c14f8c8b817d42ec0ae`, tagger `Am0rfu5`, `2026-09-21T21:50:30Z`) sits on
`main` merge commit `f7dae2676580786e9541fefee8f787623b46c70f` (PR #56; `tree(merge) ==
tree(tick)`; two parents — `1d4a9724` the previous `main` tip, `f3fc061d` the maintainer's
re-confirmation tick). Both defects above were fixed (path-only dev-dependencies for
`paladin-battalion`; the `pipefail`-safe line-79 rewrite with a >64 KiB regression test) and a new
ordering gate was wired into `make check-gates` and CI. `release.yml` run
[35659477719](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35659477719), attempt 3:
`success`. (Attempt 1 failed with `403 Forbidden` publishing `paladin-eval` — a Trusted Publisher
environment-name mismatch, `crates.io` configured instead of `crates-io`; attempt 2 predated the
maintainer's fix; attempt 3, after the fix, published the remaining two crates and completed
12/12.)

All **12** publishable crates (`cargo metadata --format-version 1`, `publish != false` — the
live-derived count this phase used throughout, not copied from any document) are
**registry-verified** at `0.10.1`: one sparse-index row per crate, each carrying its checksum and
a `yanked: false` reading, count-asserted against the same live-derived figure — see
[`37.1-CI-EVIDENCE.md` § "Registry verification"](phases/37.1-v0-10-1-patch-release/37.1-CI-EVIDENCE.md).
Publishing went through **Trusted Publishing** with no standing registry credential at any point;
`paladin-eval`'s `0.10.1` entry is that pipeline's first publish of this crate and carries
non-null `trustpub_data` (`run_id 35659477719`, `sha` matching the tag's peeled commit) — the
observable proof of the Trusted Publisher link — contrasted against its earlier token-published
`0.0.1` bootstrap version's `trustpub_data: null` reading.

**Yank disposition.** The three orphaned `0.10.0` versions (`paladin-ai-core`, `paladin-ports`,
`paladin-herald`) were yanked by the maintainer, with their own credential, only after the
registry read every publishable crate at `0.10.1` and not-yanked — never by an agent. One
register row per crate (never a summarising row) is recorded in
[`docs/src/appendix/release-recovery.md` §5](../docs/src/appendix/release-recovery.md), reason
"not defective — orphaned partial publish of `v0.10.0`, superseded by `0.10.1`". Separately, the
`v0.10.0` GitHub Release object was kept (never deleted), bannered with a short dated notice
pointing at `v0.10.1`, and flipped to pre-release so `v0.10.1` reads as "Latest" — its existing
43,373-byte body preserved byte-for-byte below the notice, and the `v0.10.0` tag itself was never
moved, deleted or re-pointed (still `1d4a9724`).

**Requirement status.** `SHIP-06` ("v0.10.1 is released") is minted and satisfied. `SHIP-05`
("v0.10.0 is released") keeps its original text, is never ticked, and reads *superseded* — what
it literally said never became true, because `v0.10.0`'s own pipeline cannot complete forward
once tagged. See `REQUIREMENTS.md`.

---

## v0.9.0 Security Tooling (Shipped: 2026-09-01)

**Phases completed:** 4 phases (18-21), 25 plans
**Requirements:** 20/20 satisfied (SAST-01…04, PUB-01…05, PUBOPS-01…05, ARTIFACT-01…06)
**Timeline:** 2026-08-24 → 2026-09-01 (8 days, 240 commits)
**Git range:** `48ac11a5` → `3957d701`
**Closeout type:** override_closeout — 0 verification overrides (all 4 phases `passed`), 1 open
artifact acknowledged (the same user-owned coverage-reproduction todo carried from the v0.8.0
close); see STATE.md *Deferred Items*
**Audit:** `milestones/v0.9.0-MILESTONE-AUDIT.md` (status `tech_debt` — all requirements
satisfied, no critical blockers, 8 debt items recorded with owners)

- ~~**No git tag was cut**~~ (**Superseded 2026-09-01, hours after the close**: the user directed
  a release-number reconciliation — planning milestones and release versions now share one line —
  and **v0.9.0 was released for real** through the documented PR-merge flow: PR #50 bumped all
  twelve manifests to `0.9.0` and curated the changelog, tag `v0.9.0` was cut on merge commit
  `0b5d4106`, and release run `33542459191` completed fully green — all eleven crates published
  to crates.io at `0.9.0` via Trusted Publishing (registry-verified), stable GitHub release with
  binaries, digest-bound image and `SHA256SUMS`. **First stable release since 0.5.1**; 0.6.0,
  0.7.0 and 0.8.0 were never released, and the 0.8.1-rc line never graduates. The original
  rationale below was true at close time and is retained per the amend-at-source convention.)
  Original: the repository enforces main-only tags, this
  milestone closed on an unmerged branch (`chore/21-close`), and a `v*` tag push now triggers the
  release pipeline whose pre-publish gate would correctly refuse a tree whose manifests still read
  `0.8.0`. The milestone identity lives in this entry and the `milestones/v0.9.0-*` archives.

**Delivered:** The supply-chain posture this milestone existed to settle is settled: the Rust-SAST
question is answered with measured evidence rather than assumption, publishing to crates.io no
longer depends on any long-lived credential, a half-published release is now a recoverable state
with a written and rehearsed runbook, and a published release finally hands a consumer curated
notes, real binaries, a digest-pinned image and verifiable checksums — proven by the first
fully-green release run in this project's history.

**Key accomplishments:**

- **The Rust-SAST gap is closed by verdict, not by adoption.** CodeQL was proven to analyse all
  385 first-party `.rs` files (the exact distinction the Snyk failure blurred), then measured
  against a five-class planted-vulnerability probe across four independent measurement rounds —
  and **disqualified as a required-check-grade Rust SAST**, version-scoped to CodeQL `2.26.3` /
  `rust-queries` `0.1.40`: SQL injection, path traversal and regex injection never fired.
  `codeql.yml` is retained advisory-only for its one reliably-working class (hardcoded
  credentials) behind a schema-checked dismissal register, the probe fixture stays in the tree
  with a recorded re-run trigger, and every document that said "no Rust SAST" now states the
  measured, dated outcome (SAST-01…04).

- **The standing crates.io publish credential no longer exists.** `publish-crates` mints its
  token per run via GitHub OIDC (`id-token: write` on that job alone, under the protected
  `crates-io` environment), all eleven crates — `paladin-herald` reconciled into the set —
  published `0.8.1-rc.2` through the new path with crates.io's own `trustpub_data` as proof, and
  only then was the "Paladin" token revoked at the registry and the `CARGO_REGISTRY_TOKEN`
  secret deleted, in ratchet order, with an honest Revocation Ledger (PUB-01…05).

- **No release begins until its facts agree, and a half-published release is finishable.** A
  pre-publish consistency gate (tag ↔ eleven manifest versions ↔ eleven changelog sections ↔ the
  tagged SHA's recorded CI conclusion, every mismatch reported) structurally blocks
  `cargo publish`; `create-release` is create-or-reuse by tag so a same-tag re-run reaches the
  publish step; already-published is read from registry state with a bounded index poll instead
  of error-prose grep and `sleep 20`; a run that moves zero crates fails with a per-crate outcome
  table; and the stuck-halfway runbook with its yank policy was proven by two live rehearsals
  (v0.8.1-rc.3/rc.4) that found and fixed two real gate bugs (PUBOPS-01…05).

- **A release now hands a consumer something real.** The body is the curated `CHANGELOG.md`
  section byte-for-byte (a missing section fails the run — no git-log fallback), the three
  binaries actually build under the features their targets require with existence asserts before
  archiving, the container image is pinned in the body by its registry-issued `sha256:` digest,
  an aggregated `SHA256SUMS` ships with one-command verification instructions, the SBOM's
  root-package-only scope is stated, and the archived `create-release@v1` /
  `upload-release-asset@v1` actions and their `upload_url` plumbing are gone (ARTIFACT-01…06).

- **The whole artifact path was proven, not re-read — then human-confirmed.** Throwaway tag
  `v0.8.1-rc.5` (run `33436573814`) produced the first fully-green release run in this project's
  history — assets downloaded and checksum-verified, binaries executed, the digest confirmed, the
  body matching the changelog section — and every human-verification backstop the phase
  verifications declared was closed by recorded UAT before the close: the crates.io token
  revocation (operator, 2026-08-28), the out-of-band pull by immutable digest, and `paladin-cli`
  executed from the released archive (user, 2026-09-01).

**Known deferred items:** 5 debt items with owners in `milestones/v0.9.0-MILESTONE-AUDIT.md`
(CodeQL re-probe trigger, `workflow_dispatch` publish path untested, two pre-existing Phase 20
review findings, dead `upload_url` script output, Nyquist validation for Phases 18-21), plus the
carried coverage-reproduction todo surfaced in STATE.md *Deferred Items*.

---

## v0.8.0 Milestone 2-12 close-out & Provider Expansion (Shipped: 2026-08-24)

**Phases completed:** 14 phases (5-17, including inserted 15.1), 149 plans
**Requirements:** 65/65 satisfied (VERIFY-01…06, CLOSE-01…03, ARCH-01…07, DEBT-01…05,
SEC-01…05, HARD-01…07, FACADE-01…04, SUPPLY-01…03, ORCH-01…05, WEB-01…04, PIPE-01…05,
DEFER-01…03, DOCS-01…04, PROV-01…04)
**Timeline:** 2026-08-04 → 2026-08-24 (21 days, 1,014 commits)
**Git range:** `be2ff05` → `48ac11a5`
**Closeout type:** override_closeout — 0 verification overrides (all 14 phases `passed`), 1 open
artifact acknowledged; see STATE.md *Deferred Items*
**Audit:** `milestones/v0.8.0-MILESTONE-AUDIT.md` (status `tech_debt` — all requirements
satisfied, no critical blockers, 25 debt items recorded with owners)

**Delivered:** The four remaining ingest-derived milestone blocks are closed out and the planning
record now describes the shipped tree across all twelve historical milestones — every contested
position answered by an evidence-cited ADR, every verified defect fixed, the quality gates that
were only ever specified now built and measuring, and the first forward work beyond the ingest
shipped as six new LLM provider adapters.

**Key accomplishments:**

- **The record now matches the tree across twelve milestones.** Five as-shipped ledgers carry 554
  `REQ-*` rows with `file:line` verdicts — 118 for Milestone 2-3, 115 for Milestone 4-6, 86 for
  Milestone 7-8, 120 for Milestone 9-12 — replacing PRD paths that predate the workspace
  decomposition. Phases 5, 7, 10 and 13 touched zero `.rs` files, each boundary independently
  re-measured at close.

- **The quality gates Deferred-QA Epic 25 specified and nobody started now exist and run.** The
  `coverage`, `cli-tests`, `bench-check` and `actionlint` CI jobs are wired and green, with the 82%
  line-coverage floor single-sourced across `ci.yml`, the `Makefile` and ADR-0006. Two previously
  blind modules measure 94.21% and 96.90%.

- **Nine LLM providers ship where three did.** Kimi, Qwen, Grok, Ollama, Gemini and a generic
  operator-configured OpenAI-compatible adapter join OpenAI, Anthropic and DeepSeek — five on a
  shared extracted `CompatEngine`, Gemini on Google's own `generateContent` protocol. Live
  four-vendor testing found and closed four real defects, including Grok rejecting every request
  because the shared engine sent `presence_penalty` unconditionally.

- **Security governance became mechanical rather than asserted.** Four divergent RustSec exception
  sets collapsed to one register with an enforcing guard (`scripts/check-advisory-register.sh`),
  the duplicate `cargo audit` job that falsified a completed milestone's success metric was
  deleted, and every suppression carries an owner and a review date.

- **Branch protection went from nothing to enforced.** Three GitHub rulesets are applied and
  verified live — `main` protected with 44 required contexts and no bypass on the merge gate —
  after a 994-commit fast-forward reconciled a trunk that sat 921 commits behind an integration
  branch being used as `develop`.

- **Snyk was measured and removed rather than trusted.** A probe carrying four deliberate
  vulnerabilities returned 0 findings in Rust while the identical four in JavaScript returned 3.
  The mandate was unsatisfiable and had blocked verification in six plans. The resulting honest
  gap — no static taint analysis for first-party Rust — is now owned by Phase 18 in v0.9.0.

### Known Gaps

- **No merge-gating Rust SAST — settled by Phase 18, 2026-08-25.** CodeQL was measured and
  disqualified as a required-check-grade Rust SAST (version-scoped: CodeQL `2.26.3` /
  `rust-queries` `0.1.40` — 3 of 4 rule-aligned, source-wired classes never fired across four
  independent measurements; `385/385` file coverage held on every run). `.github/workflows/codeql.yml`
  is retained advisory-only, not promoted to a required check. `cargo-audit`/`cargo-deny` scan
  dependencies; clippy is a lint; the manual credential-handling review remains the primary
  control. Open item — owner Am0rfu5, revisit 2027-02-25 or on a qualifying CodeQL/`rust-queries`
  release, whichever is first. See
  `.planning/phases/18-rust-sast-evaluate-and-adopt-codeql/18-CODEQL-EVIDENCE.md`.

- **Local coverage reproduction unverified.** CI's 82.39% is confirmed (run `31727496744`); the
  documented local procedure has never been walked on a Docker-capable machine. Owner: repo
  maintainer.

- **Nyquist validation unreconciled** for all 14 phases — every `VALIDATION.md` reads
  `status: draft`, so `nyquist_compliant` is not authoritative (#2117). Phase 06 has none at all.
  A coverage TODO, not a compliance failure.

- **No git tag was cut.** v0.8.0 ships through the normal release process from `main`; this
  milestone closed on an unmerged branch, and the repository enforces main-only tags.

## v0.7.1 Milestone 1 close-out (Shipped: 2026-08-04)

**Phases completed:** 4 phases, 38 plans, 88 tasks
**Requirements:** 25/25 satisfied (RECON-01…08, GAP-01…07, QUAL-01…05, REL-01…05)
**Timeline:** 2026-07-30 → 2026-08-03 (5 days, 255 commits)
**Changes:** 213 files, +44,803 / −888
**Git range:** `b926336` → `be2ff05`
**Closeout type:** override_closeout — see Known Verification Overrides below
**Audit:** `milestones/v0.7.1-MILESTONE-AUDIT.md` (status `tech_debt` — all requirements
satisfied, no critical blockers, deferrals recorded with named owners)

**Delivered:** The planning record now describes the shipped v0.7.0 tree as it actually is —
every contested definition settled by an evidence-cited ADR, the residual Milestone-1
functionality finished, coverage and error-path testing made real and measured, and the release's
version, edition, dependency and documentation posture made coherent.

**Key accomplishments:**

- **Nine ADRs settle every contested definition** (`0001`–`0009`), each naming its chosen variant
  and the shipped code it was checked against — `BattalionConfig`, `BattalionResult`, Formation's
  minimum Paladin count, provider-aware temperature range, the `Herald` trait signature, the
  coverage gate, battalion cancellation, workspace version, and Rust edition. `.planning/decisions/`
  and `.planning/ledgers/` were stood up as new document classes to hold them.

- **The Phase 1 decisions were applied in code, not just recorded.** `ProviderCapabilities` gained
  `temperature_range: Option<(f32, f32)>` (making DeepSeek's 0.0–2.0 reachable through
  `PaladinBuilder`), Formation now constructs from a single Paladin, and the citadel placeholder was
  renamed `BattalionCheckpointConfig` across all consumers — including two the plan's own research
  had missed.

- **A real multi-byte panic was found and fixed behind a self-confirming test.**
  `TableHerald::truncate_text` sliced by byte index and panicked on multi-byte UTF-8; it now
  measures by Unicode scalar values, along with the two adjacent panic paths (`format_error`, and a
  `usize` underflow at sub-ellipsis widths) that shared the same defective helper.

- **Coverage was measured offline and gated on one number.** A fully offline
  `rustc -C instrument-coverage` → `llvm-profdata` → `llvm-cov` pipeline (no `cargo-llvm-cov`, no
  network, no Docker) measured 84.79%, which became ADR-0006's single 84% hard-fail floor; Phase 3
  reproduced it verbatim at 85.56% entry and 85.92% exit, closing 4 of 5 zero-coverage files.

- **Previously dead tests were compiled and run for the first time.** 25 `tests/unit/llm/` functions
  and 37 `tests/cli/` tests had never been wired into any test target; all were activated and fixed
  without deleting one. Four `#[ignore]`d Commander stubs became real error-path tests driven by a
  new `FaultyPaladinPort` harness.

- **The release was made coherent and provably green.** All twelve manifests converged on version
  0.7.0 and edition 2024, `cargo audit`/`cargo deny` verdicts were recorded to a provenance
  standard, and the gate suite was measured — 2,924 tests passing, 185 doc tests, all 47 example
  targets building across a four-invocation feature matrix.

**Notable process outcome:** three separate premature or incorrect completions were caught and
reverted rather than shipped — a RECON-07 checkbox flipped before its ADR existed, a stale
"22 examples" figure traced to a Milestone-1 report and corrected at five source locations, and an
OpenAPI baseline invalidated by the version bump. The record self-corrected in each case.

### Known Verification Overrides

**1 override.** Phase 1's `01-VERIFICATION.md` records `passed`, 5/5 must-haves, at
2026-07-31T16:46:51Z, but commit `be2ff05` (2026-08-03) later added `01-04-SUMMARY.md` — a
disposition record for a superseded plan — which pushed the phase directory past that timestamp.
`init.manager` therefore reports phase 1 as `verification_status: stale` /
`phase_complete: false`, which blocks `verified_closeout`.

The commit is documentation-only and states "No ADR, measurement, or source changes"; the passed
verdict stands on its own evidence. Accepted as an override rather than re-verified. See STATE.md
→ Deferred Items.

### Known Gaps and Deferred Work

No unsatisfied requirements. The following are recorded with named owners:

| Item | Owner |
|---|---|
| Herald not reachable from Campaign, Chain of Command, or the Commander router (WARN-01) | Unassigned — candidate for Phase 6 |
| Nyquist validation never reconciled — all 4 phases' `VALIDATION.md` read `status: draft` | `/gsd-validate-phase 1`–`4` |
| Multi-arch Docker build within 500 MB / 300 s (time measured 2946 s; size gate hard, last 86 MB) | Phase 15 / PIPE |
| Kubernetes smoke test within 30 s startup budget | Phase 15 / PIPE |
| Real readiness probes (`k8s/deployment.yaml` runs a placeholder sleep) | Phase 14 / WEB |
| CI observed running on a `release/**` push | Human release gate (D-03) |
| `src/bin/paladin-server.rs` at 0.00% coverage | Phase 5 / VERIFY-05 |
| `minio.rs` outside ADR-0006 default-feature scope | VERIFY-05 / PIPE-02 |
| Two absent bench targets (Paladin execution loop, Arsenal invocation) | No owner, per Phase 3 CONTEXT.md D-12 |
| CR-01 OpenAI adapter reads `user_prompt.context` instead of `.query` (pre-existing since `240eb1f`) | Deferred forward from plan 02-10 |

---
