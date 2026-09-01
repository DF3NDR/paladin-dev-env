# Phase 22 — Deferred Items

Out-of-scope discoveries made while executing Phase 22 plans. None of these are fixed here —
scope boundary: only auto-fix issues directly caused by the current task's changes.

## 1. `paladin-memory`'s `qdrant` feature fails to build under `--all-features` (found in Plan 22-04, Task 3)

**Found during:** validating the new `semver` CI job (`cargo semver-checks check-release
--package paladin-ai --baseline-version 0.9.0`, run locally against this plan's HEAD).

**Symptom:** `cargo-semver-checks`'s default "enable every feature except unstable/nightly"
heuristic (used when no `--default-features`/`--only-explicit-features`/`--features` flag is
passed) builds `paladin-ai` with every feature on at once, including `qdrant`. That build fails
rustdoc generation with:

```
error[E0063]: missing field `memory` in initializer of `VectorParams`
   --> crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117:57
```

**Root cause:** `Cargo.toml`'s `[workspace.dependencies] qdrant-client = { version = "1.14" }` is
a caret range; `Cargo.lock` currently resolves it to `1.18.0`, and a `qdrant-client` release in
that range added a required `memory` field to `VectorParams` that `qdrant_adapter.rs`'s struct
literal does not set.

**Confirmed pre-existing and unrelated to Phase 22:** `cargo check -p paladin-memory --features
qdrant` (a plain compile, not a rustdoc build) succeeds today — the break is specific to the
combined all-features rustdoc build path `cargo-semver-checks` exercises by default, which no
existing CI job runs (`crate-isolation`'s `paladin-memory` leg uses `--all-features` too but only
`cargo build`/`cargo test`, not `cargo doc`; the `lint` job's `cargo doc --workspace --no-deps`
does not enable `qdrant` since that's a facade-level optional feature not in `paladin-ai`'s
default set). Phase 22 does not touch `crates/paladin-memory/src/sanctum/` at all.

**Workaround applied in this plan:** the new `semver` CI job passes `--default-features` to
`cargo semver-checks check-release` for every package, which sidesteps this break (verified
locally: all 11 publishable crates pass cleanly against the published `0.9.0` baseline under
`--default-features`). This is also arguably the more correct scope for the gate — it matches
the build a crates.io consumer gets by default.

**Recommended fix (deferred, not this phase):** either pin `qdrant-client` to an exact patch
version that predates the `VectorParams.memory` field, or update `qdrant_adapter.rs`'s struct
literal to set the new field (likely `..Default::default()` if `VectorParams` implements
`Default`, or an explicit `memory: None`/equivalent). Owner: whichever future phase next touches
the Sanctum/Qdrant adapter, or a dependency-maintenance pass.

**Impact if left unfixed:** none to Phase 22 itself. A future `cargo doc --all-features` or an
all-features rustdoc-based tool (e.g., docs.rs's own build, which uses default features unless
`[package.metadata.docs.rs] all-features = true` is set — not configured here) could hit the same
break; worth flagging to whoever owns the next `paladin-memory`/Qdrant-adjacent work.
