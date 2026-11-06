# Handoff: Paladin `v0.11.1` is released

**From:** Paladin (`DF3NDR/paladin-dev-env`), the framework maintainer
**To:** the Web3 Security Paladin ("web3sec") consumer agent
**Date:** 2026-11-06
**Purpose:** tell you what shipped and where to read what changes for you. It is a notice, not a request.

`v0.11.1` is a patch release on top of `v0.11.0`, the Dogfood Cleanup milestone. Everything below is public at the
`v0.11.1` tag; you need no access to Paladin's planning files.

---

## 1. TL;DR

- **Tag:** `v0.11.1` (annotated tag object `65b13ed08e4e0be06bc0db390e5f682748db6941`), cut 2026-11-06.
- **Tagged commit (full merge SHA):** `251303bcd143c5529c737afe8a2c61b05688f018`, the merge commit of Paladin's
  release pull request #64 (a merge commit, not a squash) and the tip of `main` when the tag was cut.
- **Release run:** `42304251278`, <https://github.com/DF3NDR/paladin-dev-env/actions/runs/42304251278>
  (`release.yml`, triggered by the tag, concluded `success`, including its `Publish to crates.io` job).
- **Registry:** all **12** publishable crates are on crates.io at `0.11.1`, none yanked, through Trusted Publishing:
  the registry's own record for each version names repository `DF3NDR/paladin-dev-env`, run `42304251278` and the
  merge commit above.
- **GitHub Release:** <https://github.com/DF3NDR/paladin-dev-env/releases/tag/v0.11.1> (not a draft, not a
  pre-release, ten assets).

## 2. Registry

One unauthenticated read per crate of the crates.io sparse index (the only header sent is a `User-Agent`), taken
2026-11-06 at 22:40Z and re-read at 22:47Z. The crate names are the crates.io names, so the core crate appears as
published, `paladin-ai-core`. The count (12) is derived from `cargo metadata` over the workspace members that are
publishable, not a typed list.

| Crate | Version | Yanked |
|---|---|---|
| paladin-ai | 0.11.1 | false |
| paladin-ai-core | 0.11.1 | false |
| paladin-battalion | 0.11.1 | false |
| paladin-content | 0.11.1 | false |
| paladin-eval | 0.11.1 | false |
| paladin-herald | 0.11.1 | false |
| paladin-llm | 0.11.1 | false |
| paladin-memory | 0.11.1 | false |
| paladin-notifications | 0.11.1 | false |
| paladin-ports | 0.11.1 | false |
| paladin-storage | 0.11.1 | false |
| paladin-web | 0.11.1 | false |

You consume Paladin through the git submodule and path dependencies, not through crates.io, so the registry is
background for you; the tag is what you pin. The facade is `paladin-ai` (its Rust crate name is `paladin`).

## 3. What changes for you

Pointers only; they are the adaptation spec and are not restated here. Both are at the tag.

- **`MIGRATION.md`, section 11.8, subsection `Library consumers: v0.11.0 → v0.11.1`**
  (<https://github.com/DF3NDR/paladin-dev-env/blob/v0.11.1/MIGRATION.md#118-upgrade-checklist>). Its first
  sentence, as committed: "A library consumer moves from `0.11.0` to `0.11.1` without changing source: no public
  Rust signature or type changed, and no serialized schema changed except that the credential fields of nine config
  types are no longer written when those types are serialised (`M-B-50`, `M-B-51`; loading is unchanged)". The
  subsection then lists the run-time changes a library consumer can observe (section 11.1).
- **Two operator steps in the same section touch a deployment**, and apply if you also run the `paladin` or
  `paladin-server` binaries:
  - **`M-B-36`, credentials an old `paladin --config` log line held.** v0.9.0 through v0.11.0 logged the whole
    configuration at startup; rotate the credentials it carried and delete the collected `Loaded configuration:`
    lines. `paladin-server` never logged its configuration.
  - **`M-B-38`, trace, agent-runtime and web-server environment variables now applied.** `paladin-server` used to
    ignore `PALADIN_TRACE_*`, `APP_AGENT_RUNTIME_*` and `APP_WEB_SERVER_*`; it now applies each that is set and
    refuses boot on an invalid value. List them before upgrading and remove any you do not want applied.

  Both are numbered steps of the section 11.8 operator checklist; neither applies to code that only links the
  libraries.

## 4. The surface that is held for you

`tests/consumer_surface_muster.rs` (<https://github.com/DF3NDR/paladin-dev-env/blob/v0.11.1/tests/consumer_surface_muster.rs>),
run by `make check-consumer-surface`, is the compile-only list of the surface a library consumer imports. It now
also pins `build_cadence`, `with_cadence` and `CadenceConfig` (test `cadence_consumer_imports_pin`), so a change to
those imports is a red test in Paladin rather than a surprise in your build.
