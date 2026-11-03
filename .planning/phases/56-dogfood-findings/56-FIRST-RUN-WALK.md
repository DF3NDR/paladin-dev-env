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
The other scratch crates (`snippet-as-written`, `quickstart-as-written`, `installation-as-written`) are resolution-only
(`cargo metadata`, no build) and live beside it under the same scratch root; their `Cargo.toml` files carry no
`path = ` entry either. `cargo install` used `--root $WALK/cli` and `--locked` for the working form (the published
lockfile). Published text walked: `registry/src/index.crates.io-*/paladin-ai-0.11.0/README.md`, byte-identical to
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

Rows follow the D-13 order. Every row has its command, exit code and an output excerpt; the last column names the
DF the stumble became, or 'no stumble'. Keys: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY` unset in
every command (`env -u ...`); scratch builds use `CARGO_TARGET_DIR=$WALK/target CARGO_PROFILE_DEV_DEBUG=0
CARGO_INCREMENTAL=0` as a command prefix only.

| # | step | command | exit | output excerpt | stumble → DF |
|---|------|---------|------|----------------|--------------|
| 1 | scratch crate + `cargo add paladin-ai@0.11.0` | `cargo new dogfood-walk && cargo add paladin-ai@0.11.0 && cargo add tokio@1 --features full && cargo fetch` | 0 / 0 / 0 / 0 | `Locking 419 packages to latest Rust 1.97.0 compatible versions`; `cargo metadata` source line in Provenance (`paladin-ai 0.11.0 registry+https://github.com/rust-lang/crates.io-index`). The README offers a `[dependencies]` snippet, not a `cargo add` line, on the tag and on HEAD alike; the snippet is a complete way to add the crate, so the absence of a `cargo add` line is not a stumble (the snippet's own content is row 2) | no stumble |
| 2 | README `[dependencies]` snippet as published, resolved as written | `cargo new snippet-as-written`; `[dependencies]` replaced verbatim by the published block (`paladin-ai = "0.5"  # default features: ["llm-openai"]`, `tokio = ...`); `cargo metadata --format-version 1` | 0 | resolved `paladin-ai 0.5.1 registry+https://github.com/rust-lang/crates.io-index` (`Adding paladin-ai v0.5.1 (available: v0.11.0)`); published `Current version: **0.8.0**`. HEAD pins `0.11.0` in both places (FIRST-02), so this is DF-02 as published. The `default features` comment names one of three defaults (HEAD too) | DF-10 (duplicate of DF-02), DF-11 |
| 3 | README Quick Example (`MockLlmAdapter`, no key) | `cargo run` with the scratch prefix (block copied verbatim from the published README into `src/main.rs`) | 0 | `Finished dev profile [unoptimized] target(s) in 1m 22s`, then `Hello from Paladin!` | no stumble |
| 4 | `basic_paladin` from the published package source | `cp ~/.cargo/registry/src/*/paladin-ai-0.11.0/examples/basic_paladin.rs dogfood-walk/examples/`; `cargo run --example basic_paladin` (scratch prefix). Then `cargo add paladin-ports@0.11.0` and the same command | 101, then 0 | error[E0433]: cannot find module or crate `paladin_ports` in this scope, at `examples/basic_paladin.rs:22:5`; after adding `paladin-ports`, the example prints the Herald result and `✨ Example completed successfully!`. No doc gives a crates.io user a command for it (the Quickstart's route is `git clone`) | DF-12 |
| 5 | Quickstart with no key | tag Quickstart `[dependencies]` resolved as written (`cargo metadata`, in `quickstart-as-written`): `paladin-ai 0.10.1`, `paladin-llm 0.10.1`; then `cargo add paladin-ports@0.11.0` and `cargo add paladin-llm@0.11.0 --features openai`, tag `main` in `src/bin/quickstart.rs`, `env -u OPENAI_API_KEY cargo run --bin quickstart` | 0 (resolution), 1 (run) | the tag Quickstart pins `0.10.0` (resolves to 0.10.1); the walk used `0.11.0`. Run: `Error: "OPENAI_API_KEY environment variable not set"`. The error names the variable, so the missing-key message is not a stumble | DF-13 (duplicate of DF-02) for the version pin; the key error is no stumble |
| 5b | lead L1: installation page's `[dependencies]` block | `cargo metadata` on the block of `docs/src/getting-started/installation.md` (HEAD line 57; versions as HEAD, `0.11.0`) in `installation-as-written`; then the same with `features = ["openai"]` | 101, then 0 | package `installation-as-written` depends on `paladin-llm` with feature `llm-openai` but `paladin-llm` does not have that feature (help: available features: anthropic, deepseek, default, gemini, grok, kimi, mock, ollama, openai, ...); with `openai`: resolves, all seven `paladin-*` at `0.11.0`. The tag's block fails the same way | DF-14 |
| 6 | free disk: delete the scratch build | `rm -rf $WALK/target`; `df -BM --output=avail /` | 0 | 4327 MB → 5476 MB free | no stumble |
| 7 | lead L2: `cargo install` of `paladin-cli` | (a) `cargo install paladin-ai --version 0.11.0 --bin paladin-cli --root $WALK/cli`; (b) `CARGO_TARGET_DIR=$WALK/install-target cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root $WALK/cli`, then `rm -rf $WALK/install-target` | (a) 101, (b) 0 | (a) target `paladin-cli` in package `paladin-ai` requires the features: `cli` (Consider enabling them by passing, e.g., `--features="cli"`). (b) Finished `release` profile [optimized] target(s) in 5m 12s, Installed package `paladin-ai v0.11.0` (executable `paladin-cli`). A1 proven | DF-15 |
| 8 | `paladin-cli --version` and `--help` | `$WALK/cli/bin/paladin-cli --version`; `... --help` | 0 / 0 | `paladin 0.11.0`; `--help` lists 13 subcommands (agent, battalion, arsenal, maneuver, onboarding, setup-check, features, muster, eval, graph, run, treasury, council). The version string is `paladin`, not `paladin-cli` (the cause of DF-17); the docs show no `--version` sample | DF-17 (cause); no separate stumble |
| 9 | `setup-check`, keyless | in an empty `$WALK/run`, with the three keys, `REDIS_URL` and `QDRANT_URL` unset: `paladin-cli setup-check`, `... --verbose`, `... --quiet` | 2 / 2 / 2 | see Keyless observations: bare `Service:` and `LLM Provider:` headings with no rows; `Passed: 2`, `Warnings: 1`; section order differs run to run (five runs); exit 2 is the documented warning exit | DF-16 |
| 10 | `onboarding` with no TTY | `timeout 20 paladin-cli onboarding < /dev/null` in `$WALK/run` | 2 | welcome banner, then `Error: User input error: IO error: not a terminal`. It fails at once with a clear message and does not hang; no file written (`find . -mindepth 1` printed nothing) | no stumble |
| 11 | the docs' command as printed (`paladin onboarding`, `paladin setup-check`) | `env -i PATH=$WALK/cli/bin /bin/bash -c 'paladin onboarding; paladin setup-check; command -v paladin-cli'` | 127 / 127 | `paladin: command not found` twice; `command -v paladin-cli` finds the installed binary. Six CLI appendix pages and the binary's own messages name `paladin` | DF-17 |

## Keyless observations

Quoted verbatim from the runs above (provider keys unset).

**Quickstart missing-key error** (step 5; tag `main` with `OpenAIAdapter::from_env()?`, no `OPENAI_API_KEY`):

```
Error: "OPENAI_API_KEY environment variable not set"
```

It names the variable. The message is Debug-formatted (quoted), which is how `fn main() -> Result<(), Box<dyn Error>>`
prints a `String` error; not logged.

**`setup-check`, default** (step 9, exit 2; run in an empty directory, the three provider keys, `REDIS_URL` and
`QDRANT_URL` unset; this run happened to print `Service`, `LLM Provider`, `Environment`, `System`):

```
┌═══════════════════════┐
│ Paladin Setup Check │
└═══════════════════════┘



Service:

LLM Provider:

Environment:
  ⚠ .env file
⚠   Not found - run 'paladin onboarding' to create

System:
  ✓ Paladin CLI v0.11.0
  ✓ Rust Toolchain (rustc 1.97.0 (2d8144b78 2026-07-07))

┌═══════════┐
│ Summary │
└═══════════┘
  Passed: 2
  Warnings: 1

⚠ Setup check completed with warnings

┌────────────────────────────────────────────────────────────┐
│ Next Steps:                                                │
│ 1. Run 'paladin onboarding' to configure missing providers │
│ 2. Verify environment variables in your .env file          │
│ 3. Run 'paladin setup-check' again to verify               │
└────────────────────────────────────────────────────────────┘
exit=2
```

**`setup-check --verbose`** (step 9, exit 2; section rows only, banner, summary and Next Steps box as above;
Summary adds `Skipped: 5`):

```
LLM Provider:
  ○ OpenAI
ℹ   OPENAI_API_KEY not configured
  ○ Anthropic
ℹ   ANTHROPIC_API_KEY not configured
  ○ DeepSeek
ℹ   DEEPSEEK_API_KEY not configured

Service:
  ○ Redis
ℹ   REDIS_URL not configured
  ○ Qdrant
ℹ   QDRANT_URL not configured

Environment:
  ⚠ .env file
⚠   Not found - run 'paladin onboarding' to create

System:
  ✓ Paladin CLI v0.11.0
ℹ   All checks passed
  ✓ Rust Toolchain (rustc 1.97.0 (2d8144b78 2026-07-07))
ℹ   All checks passed
```

`--quiet` printed the same rows as the default, with `System` first and `LLM Provider` last (exit 2). Section order
across five runs of the same command, headings only: `Service LLM Environment System`; `LLM Service Environment System`
(`--verbose`); `System Service Environment LLM` (`--quiet`); `System LLM Service Environment`;
`LLM Environment System Service`. No key was reported present when none was set. The exit code (2) agrees with the
rows (one warning) and with the documented exit-code table.

**`onboarding`, no TTY** (step 10, `< /dev/null`, `timeout 20`; the box-drawing banner lines elided):

```
 Welcome
 Welcome to Paladin! 🛡️
✓ Let's get you set up with a multi-agent orchestration framework.

This wizard will help you:
  • Configure LLM provider API keys
  • Set up your environment file
  • Validate API connectivity
  • Generate sample configurations


Error: User input error: IO error: not a terminal

exit=2
```

No `.env` and no YAML was written (the directory was empty afterwards). It neither hangs nor prompts; this replaces
research assumption A4 ("fails or hangs"): it fails at once and says why.

**Lead outcomes.** L1 reproduced on the published `0.11.0` crates (DF-14). L2 reproduced (DF-15) and the working form
`cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli` is proven (assumption A1).

## Disk ledger

Plan PD-06-2. The scratch root no longer exists (`test ! -e "$WALK"` printed `scratch removed`). Scratch builds used `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0` and a throwaway `CARGO_TARGET_DIR`;
nothing was exported into a shell that builds the workspace, and no workspace `cargo build` or `cargo test` ran.

| point | free on `/` |
|-------|-------------|
| plan start (before any build) | 5513 MB |
| after freeing `target/debug/examples` and `target/debug/incremental` (both were already empty, 4 KB each, so nothing to free) | 5513 MB |
| after the step-3 scratch build (`$WALK/target` present) | 4338 MB |
| after the step-4 and step-5 builds (same `$WALK/target`, incremental) | 4327 MB |
| step 6: after `rm -rf $WALK/target` | 5476 MB |
| step 7(a): resolve, download and fail (no build) | 5476 MB |
| step 7(b): during the release install (`install-target` present) | 4316 MB at the end of the build |
| after `rm -rf $WALK/install-target` | 5455 MB |
| end: after `rm -rf $WALK` (scratch root, every scratch crate, `cli/` install root) | 5476 MB |

No step hit `No space left on device`; no step is marked 'not run: disk'.

## Findings-log check

`./scripts/check-findings-log.sh` after the last entry of this plan (exit 1, expected; exit status is not asserted
because contained docs entries stay `open` until plan 56-07 and DF-07/DF-08 are Stage C):

```
DF-07: OPEN   DF-08: OPEN   DF-11: OPEN   DF-12: OPEN   DF-14: OPEN   DF-15: OPEN   DF-17: OPEN
parsed 17 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

DF-10 and DF-13 parse as `duplicate of DF-02`; DF-16 parses as `out (re-homed: ...)`. `git diff --numstat` for the log
over this plan shows 0 deleted lines.

## Contained fixes

Plan 56-07 (docs only, plan-start SHA `15392e9aa`), 2026-11-02. Proofs that need crates.io run in a `mktemp -d`
scratch crate outside the repo and are resolution-only (`cargo metadata`, no compile); the scratch is deleted
afterwards. Columns: `DF | file(s) | change | proof command | result`.

| DF | file(s) | change | proof command | result |
|----|---------|--------|---------------|--------|
| DF-14 (L1) | `docs/src/getting-started/installation.md` | `paladin-llm` line: `features = ["llm-openai"]` becomes `features = ["openai"]`. No other `paladin-llm` dependency line in `docs/src` or `README.md` used an `llm-` feature (`docs/src/getting-started/quickstart.md:26` and `docs/src/api-reference/crate-map.md:230` already say `openai`) | scratch crate whose `[dependencies]` is the corrected block extracted from the page: `cargo metadata --format-version 1 > /dev/null`; then the same block with `features = ["llm-openai"]` | corrected block: exit 0, `Locking 225 packages`, `paladin-ai-core`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-ports`, `paladin-storage` all `0.11.0`. Old block: exit 101, `package \`installation-old\` depends on \`paladin-llm\` with feature \`llm-openai\` but \`paladin-llm\` does not have that feature.` |
| DF-15 (L2) | `docs/src/getting-started/installation.md`, `docs/src/appendix/cli-onboarding.md` | adds `cargo install paladin-ai --features cli --locked --bin paladin-cli` in a fenced `bash` block, with a sentence that the `cli` feature is required and `--locked` uses the published lockfile. The line has no `--version`, so it cannot go stale (PD-07-1) | plan 56-06 step 7(b) ran the same line with `--version 0.11.0 --root <scratch>`; step 7(a) without `--features cli` failed; `grep -rn 'cargo install paladin-ai --features cli --locked --bin paladin-cli' docs/src/getting-started/installation.md docs/src/appendix/cli-onboarding.md` | 7(b) exit 0 (`Installed package paladin-ai v0.11.0 (executable paladin-cli)`, A1 proven); 7(a) exit 101 (`requires the features: cli`). grep prints 2 lines (`installation.md:141`, `cli-onboarding.md:22`). The `--version`-less form was not itself re-run here: it differs from the proven run only by resolving the latest published `paladin-ai`, which is `0.11.0` |
| DF-11 (S4) | `README.md`, `docs/src/api-reference/crate-map.md` | the comment after `paladin-ai = "0.11.0"` now reads `# default features: ["llm-openai", "llm-anthropic", "llm-deepseek"]` | `python3 -c "import tomllib;print(tomllib.load(open('Cargo.toml','rb'))['features']['default'])"` and `grep -n 'default features' README.md docs/src/api-reference/crate-map.md` | `['llm-openai', 'llm-anthropic', 'llm-deepseek']`; both comments list those three names in that order. `make check-version-strings` exit 0; `make check-doc-config` exit 0 (161 YAML blocks, 0 failed) |
| DF-17 | `docs/src/appendix/cli-onboarding.md`, `cli-setup-check.md`, `cli-muster.md`, `cli-council.md`, `cli-configuration.md`, and `cli-usage.md` (the sixth page DF-17 names; deviation, see the 56-07 SUMMARY) | every command line, inline command, `$ ` transcript line, link text and heading naming the onboarding or setup-check subcommand changed from `paladin <sub>` to `paladin-cli <sub>`; the two `cli-usage.md` anchors follow the renamed headings (`#paladin-cli-onboarding`, `#paladin-cli-setup-check`). The sample version output needed no change (row 8: the docs show no `paladin-cli --version` sample). The binary's own strings (`setup_check.rs`, `onboarding.rs`, `env.rs`) are a code change (P10) and stay in the re-homed todo `.planning/todos/pending/2026-11-02-setup-check-first-run-output.md` | `grep -hE '(^\|[^-[:alnum:]_])paladin (onboarding\|setup-check)' <the six pages> \| grep -c .` and `grep -rnE 'paladin-cli (onboarding\|setup-check)' <the six pages> \| wc -l` | 0 remaining bare-`paladin` invocations; 38 lines name `paladin-cli`. Row 11 showed `paladin onboarding` exits 127 and `command -v paladin-cli` finds the binary, so the corrected name is the one that resolves |
| DF-12 | `docs/src/getting-started/quickstart.md` | after the "Running the Built-in Examples" block: `cargo run --example` needs a checkout; a crates.io user copies the example's source from the downloaded package's `examples/` into their own crate, adds `paladin-ports` (and `tokio`), and runs it | row 4 of the step table: copy of `~/.cargo/registry/src/*/paladin-ai-0.11.0/examples/basic_paladin.rs` failed with exit 101 (`E0433 paladin_ports`); after `cargo add paladin-ports@0.11.0` it exited 0 (`Example completed successfully!`). `make check-version-strings` exit 0 (the note carries no version pin) | the documented steps are the ones row 4 ran |

### Gate runs after Task 3

```
$ make check-version-strings
OK: 128 file(s) scanned, every copyable version string equals '0.11.0' (class 1 Cargo pins: 42; ...)
$ make check-doc-config
Results: 161 YAML block(s) checked, 0 failed
$ make check-changelogs
11 publishable crate(s) checked, all have a CHANGELOG.md.
```

`CHANGELOG.md` `[Unreleased]` gained one `### Documentation` bullet citing DF-11, DF-12, DF-14, DF-15 and DF-17;
nothing under `[0.11.0]` and nothing in `MIGRATION.md` changed.

## Keyed half (maintainer)

Status: run 2026-11-03 by the maintainer (one real OpenAI key; an Anthropic key for K8) on the published `0.11.0` crates;
the redacted output reached the agent through chat, relayed verbatim by the orchestrator, not through the runbook file
(D-14: the agent never ran a keyed step and never held a key). Plan 56-08 Task 3 scanned the relayed text, the runbook
and this file for credentials before anything was committed (0 hits), then copied the output into the runbook slots K1..K9
and here. The maintainer ran `onboarding` and `setup-check` from `$WALK/my-paladin-agent`, so `.env` and `examples/`
landed there rather than in `$WALK`: an operator slip, not a finding. The local account name in the K6 listing is
replaced with `<user>`.

| step | command | exit | excerpt | stumble → DF |
|------|---------|------|---------|--------------|
| K1 | `rustc -V; cargo -V; date; uname` | not run as a block | not captured; derived from K6/K7: rustc 1.97.1 (8bab26f4f 2026-07-14), Linux, 2026-11-03 | no stumble (not captured) |
| K2 | Quickstart `cargo run` as written (`.model("gpt-4")`), then with `"gpt-4o-mini"` | 1, then 0 | `The model gpt-4 has been deprecated ... "code": "model_not_found"` then `Output : Hello! How can I assist you today?` / `Tokens : 32` / `Time   : 1390ms` | DF-18 |
| K3 | `RUST_LOG=info cargo run 2>&1 \| redact \| grep -E "unprofiled_model\|usage_unreconciled"` | grep 1 (no match) | `none`. The Quickstart installs no tracing subscriber, so no log lines exist and this is not evidence either way | no stumble |
| K4 | `cargo install paladin-ai` (bare), then `cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root "$WALK/cli"` | bare: not captured, nothing installed; runbook line: 0 | bare: 4m 12s compile, then `warning: none of the package's binaries are available for install using the selected features` / `bin "paladin-cli" requires the features: \`cli\``. Runbook line: `Installed package \`paladin-ai v0.11.0\` (executable \`paladin-cli\`)`, 8m 04s | DF-15 (second reproduction; the docs line is already fixed by 56-07) |
| K5 | `"$WALK/cli/bin/paladin-cli" --version` | not echoed (output printed) | `paladin 0.11.0` | DF-17 |
| K6 | `"$WALK/cli/bin/paladin-cli" onboarding` (real TTY, run from `$WALK/my-paladin-agent`) | not echoed (ran to the end) | `API Validation` ... `OpenAI ✓` / `All API keys validated successfully!`; `Created .env file`; four sample files in `examples/`; `Setup Complete!`; Next Steps box says `Run 'paladin setup-check'`, `'paladin features'`. No overwrite prompt (fresh directory) | DF-17 (Next Steps strings) |
| K7 | `paladin-cli setup-check`, then `... --verbose` (OpenAI key only, after K6) | 0, 0 | default: `LLM Provider:` first, a bare `Service:` heading, `Passed: 4`, `All checks passed!`; verbose: `System:` first, `○ Anthropic ... not configured`, `Passed: 4`, `Skipped: 4`. The `.env` warning is gone after K6, so the exit is 0 as the runbook predicted | DF-16 (bare heading, order differs between the two runs) |
| K8 | `paladin-cli setup-check --verbose` with a valid `ANTHROPIC_API_KEY` as well | 1 | `✗ Anthropic` / `API validation failed: Paladin execution error: Anthropic API validation failed (404 Not Found)`; `Passed: 4`, `Failed: 1`, `Skipped: 3` | DF-19 |
| K9 | stumble list and `## Cleanup` | n/a | eight bullets (below); last line `cleaned` | no stumble (cleanup confirmed) |

Not findings, noted as evidence only: K3 `none` (no subscriber, see the row); `onboarding` run from a subdirectory (above);
`cargo install` built `openssl-sys`, so system OpenSSL development headers are needed (present on the maintainer host;
not reproduced as a failure); K1 not captured. A pro-tier OpenAI 404 did not occur. K4 exited 0 on the runbook line, so the
`--features cli --locked` install is proven a second time; L2 (DF-15) was already closed by plan 56-07, so no docs change
was needed here.

Triage of K9 against HEAD dc190dba6 and the log (DF-01..DF-17): two new entries, DF-18 (S3, `out`, re-homed to
`.planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md`) and DF-19 (S2, `open → promote`, Stage C).
DF-15, DF-16 and DF-17 were already final; the keyed run reproduced all three and reopens none.

### Redacted output, as pasted

#### K1 — environment

```
not captured; the K1 command block (rustc -V; cargo -V; date; uname) was not run as a block.
derived from K7/K6: rustc 1.97.1 (8bab26f4f 2026-07-14), Linux host, run date 2026-11-03.
```

#### K2 — Quickstart run

```
Dependency block actually used (crates at 0.11.0, `openai` feature):

[package]
name = "my-paladin-agent"
version = "0.1.0"
edition = "2024"

[dependencies]
paladin-ai    = "0.11.0"
paladin-ports = "0.11.0"
paladin-llm   = { version = "0.11.0", features = ["openai"] }
tokio         = { version = "1", features = ["full"] }

Run 1, Quickstart as written (`.model("gpt-4")`):

my-paladin-agent$ cargo run 2>&1 | redact | tail -40; echo "exit=${PIPESTATUS[0]}"
   ... (compile lines elided) ...
   Compiling paladin-ai v0.11.0
   Compiling my-paladin-agent v0.1.0 (/tmp/tmp.X5778tt5fi/my-paladin-agent)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 09s
     Running `/tmp/tmp.X5778tt5fi/target/debug/my-paladin-agent`
Error: LlmFailure { transience: Permanent, status: None, provider: None, message: "Model not available: {\n    \"error\": {\n        \"message\": \"The model `gpt-4` has been deprecated, learn more here: https://platform.openai.com/docs/deprecations\",\n        \"type\": \"invalid_request_error\",\n        \"param\": null,\n        \"code\": \"model_not_found\"\n    }\n}\n" }
exit=1

Run 2, after `sed -i 's/"gpt-4"/"gpt-4o-mini"/' src/main.rs`:

cargo run 2>&1 | redact | tail -20; echo "exit=${PIPESTATUS[0]}"
   Compiling my-paladin-agent v0.1.0 (/tmp/tmp.X5778tt5fi/my-paladin-agent)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.54s
     Running `/tmp/tmp.X5778tt5fi/target/debug/my-paladin-agent`
Output : Hello! How can I assist you today?
Tokens : 32
Time   : 1390ms
exit=0
```

#### K3 — trace watch-list matches

```
 RUST_LOG=info cargo run 2>&1 | redact | grep -E 'unprofiled_model|usage_unreconciled'; echo "grep_exit=${PIPESTATUS[2]}"
grep_exit=1

Result: `none`. The Quickstart `main.rs` installs no tracing subscriber, so no log lines were
printed and this result is not evidence either way. No `tracing-subscriber` was added.
```

#### K4 — CLI install

```
Pre-run (NOT the runbook line; the bare command a new user types):

cargo install paladin-ai
    ... Finished `release` profile [optimized] target(s) in 4m 12s
warning: none of the package's binaries are available for install using the selected features
  bin "paladin" requires the features: `cli`
  bin "paladin-cli" requires the features: `cli`
  bin "paladin-server" requires the features: `web-server`
  ...
Consider enabling some of the needed features by passing, e.g., `--features="cli"`
(exit code not captured; nothing installed; resolved an unlocked, newer dependency set, e.g.
reqwest 0.13.5, tokio 1.54.1, rand 0.10.3, and compiled clean.)

Runbook line:

cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root "$WALK/cli" 2>&1 | redact | tail -20; echo "exit=${PIPESTATUS[0]}"
   Compiling argon2 v0.5.3
   Compiling paladin-herald v0.11.0
   Compiling toml v0.8.23
   Compiling hmac v0.12.1
   Compiling md5 v0.8.0
   Compiling urlencoding v2.1.3
   Compiling dotenv v0.15.0
   Compiling tokio-rustls v0.26.4
   Compiling rustls-platform-verifier v0.7.0
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.13.4
   Compiling sqlx v0.8.6
   Compiling paladin-storage v0.11.0
   Compiling paladin-memory v0.11.0
   Compiling paladin-eval v0.11.0
   Compiling paladin-ai v0.11.0
    Finished `release` profile [optimized] target(s) in 8m 04s
  Installing /tmp/tmp.X5778tt5fi/cli/bin/paladin-cli
   Installed package `paladin-ai v0.11.0` (executable `paladin-cli`)
warning: be sure to add `/tmp/tmp.X5778tt5fi/cli/bin` to your PATH to be able to run the installed binaries
exit=0
```

#### K5 — CLI version

```
"$WALK/cli/bin/paladin-cli" --version
paladin 0.11.0
```

#### K6 — onboarding

```
"$WALK/cli/bin/paladin-cli" onboarding

━━ Welcome ━━
┌═══════════════════════════════┐
│ Welcome to Paladin! 🛡️ │
└═══════════════════════════════┘

✓ Let's get you set up with a multi-agent orchestration framework.

This wizard will help you:
  • Configure LLM provider API keys
  • Set up your environment file
  • Validate API connectivity
  • Generate sample configurations

Ready to begin? yes

━━ Provider Selection ━━
┌══════════════════════════┐
│ LLM Provider Selection │
└══════════════════════════┘

Select one or more LLM providers to configure:

Select providers (use Space to select, Enter to confirm): OpenAI
✓ Selected providers: OpenAI

━━ API Key Input ━━
┌═════════════════════════┐
│ API Key Configuration │
└═════════════════════════┘

Enter your OpenAI API key:
OPENAI_API_KEY: [hidden]

✓ Configured 1 API key(s)

━━ API Validation ━━
┌══════════════════┐
│ API Validation │
└══════════════════┘

Testing API connectivity...

⠏ OpenAI ✓
✓ All API keys validated successfully!

━━ Environment File ━━
┌══════════════════════════┐
│ Environment File Setup │
└══════════════════════════┘

Creating new .env file...
✓ Created .env file

━━ Sample Configurations ━━
┌═════════════════════════┐
│ Sample Configurations │
└═════════════════════════┘

Create sample configuration files? yes
⠏ Sample files created
  • examples/basic_paladin.yaml
  • examples/formation.yaml
  • examples/phalanx.yaml
  • examples/paladin_with_rag.yaml

━━ Summary ━━
┌════════════════════════┐
│ Setup Complete! 🎉 │
└════════════════════════┘

✓ Your Paladin environment is ready!

Completed:
  ✓ Configured 1 LLM provider(s)
  ✓ Created/updated .env file
  ✓ Generated sample configurations

┌───────────────────────────────────────────────────────────┐
│ Next Steps:                                               │
│ 1. Run 'paladin setup-check' to verify your configuration │
│ 2. Try the examples in the examples/ directory            │
│ 3. Run 'paladin features' to explore available commands   │
│ 4. Read the docs at https://github.com/DF3NDR/paladin     │
└───────────────────────────────────────────────────────────┘

No overwrite/skip/merge prompt appeared (fresh directory: "Creating new .env file"). Listing
(taken at `$WALK`, before realising the wizard ran in the subdirectory; `.env` and `examples/`
are under `my-paladin-agent/`):

ls -la "$WALK" "$WALK/examples" 2>&1 | redact
ls: cannot access '/tmp/tmp.X5778tt5fi/examples': No such file or directory
/tmp/tmp.X5778tt5fi:
total 280
drwx------  5 <user> <user>   4096 Nov  3 12:52 .
drwxrwxrwt 59 root      root      266240 Nov  3 13:03 ..
drwxrwxr-x  3 <user> <user>   4096 Nov  3 13:00 cli
drwxrwxr-x  5 <user> <user>   4096 Nov  3 13:02 my-paladin-agent
drwxrwxr-x  4 <user> <user>   4096 Nov  3 12:52 target
```

#### K7 — setup-check

```
"$WALK/cli/bin/paladin-cli" setup-check 2>&1 | redact; echo "exit=${PIPESTATUS[0]}"
"$WALK/cli/bin/paladin-cli" setup-check --verbose 2>&1 | redact; echo "exit=${PIPESTATUS[0]}"
┌═══════════════════════┐
│ Paladin Setup Check │
└═══════════════════════┘

LLM Provider:
  ✓ OpenAI

Environment:
  ✓ .env file

Service:

System:
  ✓ Paladin CLI v0.11.0
  ✓ Rust Toolchain (rustc 1.97.1 (8bab26f4f 2026-07-14))

┌═══════════┐
│ Summary │
└═══════════┘
  Passed: 4

✓ All checks passed! ✨

exit=0
┌═══════════════════════┐
│ Paladin Setup Check │
└═══════════════════════┘

System:
  ✓ Paladin CLI v0.11.0
ℹ   All checks passed
  ✓ Rust Toolchain (rustc 1.97.1 (8bab26f4f 2026-07-14))
ℹ   All checks passed

LLM Provider:
  ✓ OpenAI
ℹ   All checks passed
  ○ Anthropic
ℹ   ANTHROPIC_API_KEY not configured
  ○ DeepSeek
ℹ   DEEPSEEK_API_KEY not configured

Environment:
  ✓ .env file
ℹ   All checks passed

Service:
  ○ Redis
ℹ   REDIS_URL not configured
  ○ Qdrant
ℹ   QDRANT_URL not configured

┌═══════════┐
│ Summary │
└═══════════┘
  Passed: 4
  Skipped: 4

✓ All checks passed! ✨

exit=0
```

#### K8 — Anthropic row (optional)

```
 "$WALK/cli/bin/paladin-cli" setup-check --verbose 2>&1 | redact | sed -n '/LLM Provider/,/Environment/p'; echo "exit=${PIPESTATUS[0]}"
LLM Provider:
  ✓ OpenAI
ℹ   All checks passed
  ✗ Anthropic
✗   API validation failed: Paladin execution error: Anthropic API validation failed (404 Not Found)
  ○ DeepSeek
ℹ   DEEPSEEK_API_KEY not configured

Service:
  ○ Redis
ℹ   REDIS_URL not configured
  ○ Qdrant
ℹ   QDRANT_URL not configured

┌═══════════┐
│ Summary │
└═══════════┘
  Passed: 4
  Failed: 1
  Skipped: 3

✗ Setup check failed - please address the errors above

┌────────────────────────────────────────────────────────────┐
│ Next Steps:                                                │
│ 1. Run 'paladin onboarding' to configure missing providers │
│ 2. Verify environment variables in your .env file          │
│ 3. Run 'paladin setup-check' again to verify               │
└────────────────────────────────────────────────────────────┘
exit=1
```

#### K9 — stumbles, and the cleanup line

```
- `cargo install paladin-ai` with no `--features cli`: 4m12s compile, nothing installed, warning names the feature. (DF-15, docs fixed in 56-07; additional reproduction on the maintainer's host.)
- Quickstart `.model("gpt-4")` fails: `model_not_found`, "The model `gpt-4` has been deprecated". Succeeds with `gpt-4o-mini` (32 tokens, 1390 ms). Same literal on HEAD `docs/src/getting-started/quickstart.md:53`; `"gpt-4"` appears ~109 times across 10 pages in `docs/src`, `examples/`, `README.md`. NEW.
- `setup-check --verbose` and (by the same literal) `onboarding` fail a valid Anthropic key with `Anthropic API validation failed (404 Not Found)`; probe hard-codes `claude-3-haiku-20240307`; message does not name the model. NEW (lead L8).
- `paladin-cli --version` prints `paladin 0.11.0`; onboarding and setup-check "Next Steps" boxes say `paladin setup-check` / `paladin onboarding` / `paladin features`. (DF-17 in-binary strings, re-homed.)
- `setup-check` default run shows a bare `Service:` heading; section order differs between the default and `--verbose` runs (`LLM Provider` first vs `System` first). (DF-16, re-homed.)
- `onboarding` was run from `$WALK/my-paladin-agent` rather than `$WALK`, so `.env`/`examples/` landed in the subdirectory. Operator slip, not a finding.
- `cargo install` built `openssl-sys`, so system OpenSSL dev headers are required; present on this host. Borderline; not reproduced as a failure.
- K1 command block not captured (see K1).

Cleanup (run as given, `.env` path adjusted to `my-paladin-agent/.env`; `unset` of both keys):

cleaned
```
