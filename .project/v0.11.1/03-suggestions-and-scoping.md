# v0.11.1 — Suggestions and scoping

Observations and recommendations for shaping the milestone, written 2026-10-27 before `/gsd-new-milestone`.
Everything here is a suggestion; the inventory in `01-carried-gaps-inventory.md` is the evidence.

## 1. The milestone has a real backlog already — it does not need to wait for dogfood reports

Reading the registers, v0.11.0 left **eight security / tenant-isolation items from one review (Phase 40)
that were never dispositioned**, one reachable `todo!()` panic in a published crate, one config-example trap
that loads a placeholder as an API key, and a README that says 0.8.0. None of those needs a dogfood run to
justify. Suggest the first code phase of v0.11.1 is fixed before any finding arrives, and the dogfood
findings feed phases two onward.

## 2. Suggested themes (candidate phases), in order

1. **Release-process and CI repair** (inventory D1–D7). Do this first: re-pin semver to 0.11.0 and drop the
   crate-wide allows (their own dated notes say "remove once 0.11.0 is the baseline" — it is), wire
   `check-migration-allowlist` into CI, run the `main`-only jobs on PRs, fix the release-body argv cap with
   `jq --rawfile`. Every later phase then runs under a semver gate that actually checks something, and the
   0.11.1 release itself cannot repeat the v0.11.0 condensation scramble.
2. **Tenant isolation closure** (A1–A5, A8). One phase, one shared visibility gate for `/threads/*`, the
   service-layer tenant check on cancel, the error-body redaction on the run controller, the boot-time
   assertion that a configured treasurer is actually attached. This is the only S1 cluster and it is bounded.
3. **First-run usability** (B1/C3 placeholder expansion-or-refusal, C1/C2 version strings with a gate, C4
   k8s labels, C5 crate changelog corrections in the 0.11.1 entries, C6–C8 doc nits, D6 close issue #41).
   Cheap, visible, and exactly what a new user trips on.
4. **Correctness small fixes** (B2 `todo!()` removal, B3 Failed-vs-Halted mapping, B5 `RunStatusValue::
   Cancelled`, B6 opus-4-8 seed, E1/E2 hermetic and non-flaky tests, E3–E5, E7 lint and stub cleanup).
5. **Dogfood findings** — one or more phases scoped from `02-dogfood-findings-log.md` once the maintainer's
   runs and the web3sec return (`47-HANDOFF-web3sec.md` §6) are in. Leave room: this is the milestone's
   stated purpose, and its content is unknown today.
6. **Register hygiene close-out** (F1–F5): validate-phase on the nine draft phases, SUMMARY frontmatter,
   WINDOWS dispositions (56, 68, 69, 79), archive the three resolved debug sessions. Cheap, and it means
   v0.11.1 can audit `passed` rather than `tech_debt` for the first time since v0.9.0.

## 3. Decisions to make before roadmapping

These change the shape of the milestone, so they belong in the `/gsd-new-milestone` conversation, not in a
phase's discuss step.

| Decision | Options | My recommendation |
|---|---|---|
| `${VAR}` in HTTP auth keys (B1) | implement expansion for auth keys · refuse unexpanded `${…}` at boot and fix the examples | **Refuse at boot.** Smaller change, no new secret-handling path, and the examples become honest. Expansion can come later if dogfooding wants it. |
| OpenAI Responses API (G2) | in as the one feature item · out, stays a todo | **Decide by dogfood need.** If the maintainer or web3sec actually needs a pro-tier model in the window, bring it in fenced as "the one feature". Otherwise it is the first item of the next feature milestone. |
| Content/data orchestration base items (G3, B2) | review-and-fence (keep, document as unfinished, make every stub return an error) · remove · finish | **Review-and-fence.** The owner already said they are not dormant; v0.11.1's job is to make sure nothing in them panics or misleads. Finishing is out of scope by the milestone's own definition. |
| Unpriced-model surface (B4) | warn at boot / admission · refuse workflow assistants like agent routes · leave | **Warn.** A boot-time list of "models reachable by a metered principal with no pricing row" is cheap and does not change an accepted contract. |
| Stampede lock composition (B10) | document "server attaches no node cache" · wire a node cache into the server | **Document** unless a dogfood run wants the cache; wiring it is feature work. |
| Node-cache key and provider default effort (B12) | bump the cache key version and fold the default in · keep rotating `key_prefix` | **Keep, document** — unless dogfooding shows a stale-cache wrong result, which would make it a bug. |
| Pre-Phase-41 unattributed schedules (A7) | boot-time `warn!` · backfill migration · PATCH route | **Warn now**; backfill only if a dogfood deployment actually has such rows. |
| `cargo test --workspace --all-features` in CI (D10) | add the job · keep per-feature matrix | CI-cost call; the single-feature clippy leftovers (E3) suggest the matrix still has value. The eight rustdoc allows are a separate, in-scope hygiene item either way. |
| LLM-judge prohibition verdicts (F5) | bounded human read of the ~40 Phase 40/42 items · accept | A **bounded read of the Phase 40 subset only** (they overlap the S1 cluster). |

## 4. Exit criteria worth stating up front

A cleanup milestone is easy to leave half-done because nothing in it is a headline. Suggest the roadmap
states these as the milestone's definition of done:

- `WINDOWS.md` `open_count: 0` at the tag (every row fixed, waived with a reason, or re-homed with an owner and
  date) — the v0.10.0 Phase 36.1 bar, which v0.11.0 did not meet.
- Milestone audit status `passed`, not `tech_debt`.
- Zero undispositioned review warnings across all phases 38–51 (today: Phase 40 WR-01..04).
- The 0.11.1 release runs end-to-end through `release.yml` with **no manual condensation step** (D4 fixed)
  and a semver job that reports a non-zero check count (D1 fixed).
- `README.md`, `docs/src` install snippets, `k8s/` labels and `openapi.json` all read 0.11.1, enforced by a
  gate, not a sweep.
- Every dogfood finding in `02-dogfood-findings-log.md` has a disposition (in, out with a reason, or
  duplicate); none is `open` at the tag.

## 5. Scope guards

- **No new ports, no new adapters, no new HTTP routes** except where a security fix needs one (A1 may need a
  tenant column on threads; that is a migration, not a feature). Audit G1 (aggregate spend route) is
  explicitly out.
- **No MIGRATION.md section 11 breaks unless forced by a security fix.** v0.11.1 should be a patch a
  consumer takes without an adaptation; the web3sec single-bump rule argues for it strongly. If A1 forces a
  persisted-shape change, record it as the one deliberate break and say so in the handoff.
- **Stubs are fenced, not finished.** A stub that panics (`todo!()`, `unimplemented!()` outside `#[cfg(test)]`)
  is a bug and gets an `Err`; a stub that returns a placeholder silently (`planning_service.rs:417`) is worth
  one look; a stub that is simply absent functionality stays on the FUT register.
- **Accepted contracts stay accepted** (inventory §H). A dogfood report that collides with one is logged with
  the ADR reference and dispositioned `out`, not quietly reopened.
- **Measurement work is not cleanup.** Probing Ollama / OpenAI-compatible, capturing quota bodies, unifying
  retry counters (C9) are real gaps but need live keys and a probe plan; they belong to a provider-parity
  follow-up unless a dogfood run on that provider hands over a captured body for free.

## 6. Things I would watch for during dogfooding (likely-bug shapes)

Not findings — places where the registers suggest the next bug will be.

- **The agent-kind run path.** It has no Waypoint (row 64), its halt-vs-failed mapping has one known edge
  (B3), and its two wiring tests cannot run without network (E1). It is the least-exercised of the two run
  kinds and the one `/v1/agents/{id}/execute` users hit first.
- **Reasoning-effort refusals** (B7). Owner-accepted, but "every seeded Anthropic model refuses `Budget(n)`
  until you declare a span" is exactly the kind of first-run wall a dogfooder reports. Watch whether the
  refusal message names the config key.
- **Config loading in general.** The `${VAR}` finding (B1) and the removed-v0.10-keys fix (Phase 44 WR-03,
  done) both came from the same place: the loader accepts things the examples imply it rejects, or vice
  versa. A quick pass that feeds each `config.example.yml` section through the loader with every optional
  key present would likely find more.
- **The `paladin-cli onboarding` / `setup-check` commands.** Their live tests are all `#[ignore]`; nothing in
  CI runs them against a real key.
- **Anything with "unprofiled" in the trace** (`paladin.unprofiled_model`, `paladin.usage_unreconciled`):
  these are the designed signals that a provider or model is outside the measured set, and a dogfood run on
  a non-OpenAI provider is the first time most of them fire in anger.

## 7. Naming note

The milestone is `v0.11.1` under semver 0.x: a patch bump. If a security fix in theme 2 forces a persisted
or public-surface break, the right number is `v0.12.0`, not a quietly breaking `0.11.1`. Decide at the point
the break is known, not at the tag.
