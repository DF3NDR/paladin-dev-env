# Phase 56: first-run walk on published 0.11.0 (DOGF-04, D-13, D-14)

Keyless half, run by the agent (plan 56-06) on 2026-11-02. Everything up to the first provider call, with
`OPENAI_API_KEY`, `ANTHROPIC_API_KEY` and `DEEPSEEK_API_KEY` unset in every command (`env -u ...`). The keyed
half is the maintainer's (plan 56-08, `56-RUNBOOK-KEYED-WALK.md`). Plan-start SHA: `a39a338dd`.

## Provenance

Scratch crate outside any git repository, no path dependency, no `[patch]`, no workspace.

```
$ WALK=$(mktemp -d "$SCRATCHPAD/paladin-walk.XXXXXX")     # scratch root (session scratchpad, not under the repo)
$ git -C "$WALK" rev-parse --show-toplevel
fatal: not a git repository (or any of the parent directories): .git
rev-parse exit=128
$ rustc -V ; cargo -V
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
$ cd dogfood-walk && cargo metadata --format-version 1 | python3 -I (print name, version, source of package paladin-ai)
paladin-ai 0.11.0 registry+https://github.com/rust-lang/crates.io-index
workspace_root = <scratch>/paladin-walk.XXXXXX/dogfood-walk      (the scratch crate itself; no outer workspace)
$ grep -c 'path = ' dogfood-walk/Cargo.toml
0
```

`dogfood-walk/Cargo.toml` `[dependencies]` is exactly `paladin-ai = "0.11.0"` and
`tokio = { version = "1", features = ["full"] }` (from `cargo add`). `cargo tree -p paladin-ai --depth 1` shows
`paladin-ai v0.11.0` with `paladin-ai-core`, `paladin-battalion`, `paladin-herald`, `paladin-llm`, `paladin-memory`,
`paladin-ports`, `paladin-storage` all `v0.11.0` (registry, no path suffix); `cargo fetch` locked 419 packages.
Published text walked: `registry/src/index.crates.io-*/paladin-ai-0.11.0/README.md`, byte-identical to
`git show v0.11.0:README.md` (`diff` printed nothing).

## Published README vs HEAD

`diff -u` of the published README (what crates.io shows) against HEAD `README.md`, limited to the three sections
walked. Quick Example: identical. Getting Started and Project Status differ only where Phase 54 (FIRST-01/02) fixed
the published text:

```
 [dependencies]
-paladin-ai = "0.5"                      # default features: ["llm-openai"]
+paladin-ai = "0.11.0"                  # default features: ["llm-openai"]
...
-Current version: **0.8.0**.
+Current version: **0.11.0**.
```

(The same diff also shows the "Running agents behind an HTTP API" paragraph reworded by FIRST-01; not on the
first-run path walked here.) HEAD's Getting Started comment still reads `# default features: ["llm-openai"]` while the
root manifest's `default` is `["llm-openai", "llm-anthropic", "llm-deepseek"]` (S4, logged as DF-11). Neither the
published README nor HEAD's has a `cargo add` or `cargo install` line.

## Step table

| # | step | command | exit | output excerpt | stumble → DF |
|---|------|---------|------|----------------|--------------|
| 1 | scratch crate + `cargo add paladin-ai@0.11.0` | `cargo new dogfood-walk && cargo add paladin-ai@0.11.0 && cargo add tokio@1 --features full && cargo fetch` | 0 / 0 / 0 / 0 | `Locking 419 packages to latest Rust 1.97.0 compatible versions`; `cargo metadata` source line in Provenance (`paladin-ai 0.11.0 registry+https://github.com/rust-lang/crates.io-index`). The README offers a `[dependencies]` snippet, not a `cargo add` line, on tag and on HEAD alike; the snippet is a complete way to add the crate, so its absence is not a stumble (the snippet's own content is row 2) | no stumble |
| 2 | README `[dependencies]` snippet as published, resolved as written | `cargo new snippet-as-written`; `[dependencies]` replaced verbatim by the published block (`paladin-ai = "0.5"  # default features: ["llm-openai"]`, `tokio = { version = "1", features = ["full"] }`); `cargo metadata --format-version 1` | 0 | resolved `paladin-ai 0.5.1 registry+https://github.com/rust-lang/crates.io-index` (`Adding paladin-ai v0.5.1 (available: v0.11.0)`); published `Current version: **0.8.0**`. HEAD pins `0.11.0` and `0.11.0` (FIRST-02), so this is DF-02 as published. The `default features` comment names one of three defaults (HEAD too) | DF-10 (duplicate of DF-02), DF-11 |
| 3 | README Quick Example (`MockLlmAdapter`, no key) | `env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY CARGO_TARGET_DIR=$WALK/target CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo run` (block copied verbatim from the published README into `src/main.rs`) | 0 | `Finished dev profile [unoptimized] target(s) in 1m 22s` then `Hello from Paladin!` | no stumble |

## Disk ledger

| point | free on `/` |
|-------|-------------|
| plan start (before any build) | 5513 MB |
| after freeing `target/debug/examples` and `target/debug/incremental` (both were already empty, 4 KB each, so nothing to free) | 5513 MB |
| after the step-3 scratch build (`$WALK/target` present) | 4338 MB |
