# Phase 56 Runbook: keyed first-run walk (DOGF-04, D-13, D-14)

Stage B runbook. It needs one real OpenAI API key and a machine that can compile Rust against
crates.io. The agent has already run (or will run, in plan 56-06) the keyless half. This is the
half that needs a key. Paste slots `K1`..`K9` at the end are read by plan 56-08.

## Purpose and decision

- **Requirement:** DOGF-04. Walk the first-run path on the published `0.11.0` crates and log every
  stumble.
- **Decision D-13:** the walk uses the crates.io `0.11.0` packages in a scratch crate outside any
  repository. Never a path dependency, never `[patch]`, never a workspace checkout. Steps follow
  the README and the Quickstart in order.
- **Decision D-14:** the keyed half is yours. The key never enters the repo, a log, a slot or a file
  that survives the walk. The provider is OpenAI, the one the README Quick Example names in its
  "for real use" line (`OpenAIAdapter::from_env()?`) and onboarding's first option. An extra
  provider key (Anthropic, DeepSeek) is optional and handled the same way.

## Before you start

You need a Rust toolchain, network access to crates.io, and a few GB of free disk. Run these in one
shell and keep that shell open for the whole walk, because the key lives only in its environment.

1. Make the scratch directory and keep every build output inside it:

   ```bash
   WALK=$(mktemp -d) && cd "$WALK"
   export CARGO_TARGET_DIR="$WALK/target"
   ```

2. Check that the scratch directory is outside any git repository. This command must print
   nothing and the message must NOT appear:

   ```bash
   if git -C "$WALK" rev-parse --show-toplevel 2>/dev/null; then echo "STOP: $WALK is inside a git repository"; fi
   ```

   If `STOP` appears, `rm -rf "$WALK"` and start again somewhere else.
3. Enter the key with no echo, so it is never in shell history, a file or a command line:

   ```bash
   read -rs OPENAI_API_KEY && export OPENAI_API_KEY
   ```

   Press Enter once after pasting. Nothing is displayed. Then run `echo` to get a fresh line.
4. Define the redaction filter used by every paste (see `## Redaction`):

   ```bash
   redact() { sed -E 's/(sk-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+/\1REDACTED/g; s/(API_KEY=)[^[:space:]]+/\1REDACTED/g'; }
   ```

5. Record the environment into slot `K1`:

   ```bash
   { rustc -V; cargo -V; date -u +%FT%TZ; uname -srm; } | redact
   ```

## Steps

Run every step in `$WALK` unless it says otherwise. Pipe the output of each step through `redact`
before pasting it into the named slot. Paste verbatim; do not tidy.

1. **K2: the Quickstart, as written.**

   ```bash
   cargo new my-paladin-agent && cd my-paladin-agent
   ```

   Print the Quickstart exactly as it was at the release tag. Use a checkout you already have,
   read-only, and do not `cd` into it:

   ```bash
   git -C /path/to/paladin-dev-env show v0.11.0:docs/src/getting-started/quickstart.md
   ```

   Copy its `[dependencies]` block into `Cargo.toml` and its `main.rs` into `src/main.rs`, exactly
   as printed. If the printed block names a version other than `0.11.0` (the research found the
   tag's Quickstart may still say `0.10.0`), that is a stumble for `K9`: use `0.11.0` for the three
   `paladin-*` crates so the walk stays on the published release, and say you changed it. Then:

   ```bash
   cargo run 2>&1 | redact | tail -40; echo "exit=${PIPESTATUS[0]}"
   ```

   Paste the output and the exit code into `K2`, plus the dependency block you actually used.
2. **K3: trace watch-list.** The scoping notes ask whether `paladin.unprofiled_model` or
   `paladin.usage_unreconciled` fire on the README provider. Run the same binary again:

   ```bash
   RUST_LOG=info cargo run 2>&1 | redact | grep -E 'unprofiled_model|usage_unreconciled'
   ```

   Paste each match, or the word `none`. The Quickstart `main.rs` may install no tracing
   subscriber, in which case `RUST_LOG` has no effect and `none` proves nothing. Say in `K3`
   whether the program printed any log lines at all. If you add a `tracing-subscriber` call to see
   the events, that is a change to the walk: name it in `K3`.
3. **K4: install the CLI.** The CLI ships inside the `paladin-ai` package behind the `cli` feature.

   ```bash
   cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root "$WALK/cli" 2>&1 | redact | tail -20; echo "exit=${PIPESTATUS[0]}"
   ```

   Paste the last 20 lines and the exit code. If the README or docs gave you a different install
   command, try that one first, paste its failure, and then use the line above. That sequence is
   itself a stumble for `K9`.
4. **K5: version.**

   ```bash
   "$WALK/cli/bin/paladin-cli" --version
   ```
5. **K6: onboarding.** Run in `$WALK` (it writes into the current directory; see traps):

   ```bash
   "$WALK/cli/bin/paladin-cli" onboarding
   ```

   Choose OpenAI. At the hidden password prompt, paste the key. Paste the prompts and everything
   printed into `K6`, redacted. Do not `cat` the `.env` it writes. List what it created instead:

   ```bash
   ls -la "$WALK" "$WALK/examples" 2>&1 | redact
   ```

   Onboarding validates the key against the provider, so this step makes a real network call.
6. **K7: setup-check.**

   ```bash
   "$WALK/cli/bin/paladin-cli" setup-check 2>&1 | redact; echo "exit=${PIPESTATUS[0]}"
   "$WALK/cli/bin/paladin-cli" setup-check --verbose 2>&1 | redact; echo "exit=${PIPESTATUS[0]}"
   ```

   Paste both outputs and both exit codes.
7. **K8 (optional): Anthropic row.** If you have an Anthropic key, enter it the same way
   (`read -rs ANTHROPIC_API_KEY && export ANTHROPIC_API_KEY`), rerun
   `"$WALK/cli/bin/paladin-cli" setup-check 2>&1 | redact`, and paste the Anthropic row. Note
   whether the validator's hard-coded model `claude-3-haiku-20240307` (research lead L8) still
   answers or comes back as a model-not-found error. If you have no Anthropic key, write
   `skipped` in `K8`.
8. **K9: stumbles.** One bullet per step that did not read as written, for example an error
   message, a doc step that is missing, docs saying `paladin onboarding` where the binary is
   `paladin-cli`, or a reasoning-effort refusal that does not name its config key. Quote the
   verbatim text in a code block under the bullet, redacted.

## Traps to record

- **Onboarding writes into the current directory.** It creates `.env` (with your key in it) and an
  `examples/` directory of sample agent YAML files in `$WALK`. That is why the walk runs in a
  scratch directory and why `## Cleanup` deletes `.env` first. If `.env` already exists onboarding
  asks overwrite, skip or merge; record which prompt you saw.
- **A pro-tier OpenAI model returning 404** is logged as a finding and dispositioned
  `out (milestone decision 2026-10-27; todo 2026-10-20-openai-responses-api-for-pro-tier)`, per
  the Deferred section of `56-CONTEXT.md`. Do not work around it.
- **Disk.** A release build of the CLI needs several GB. `CARGO_TARGET_DIR="$WALK/target"` (set in
  `## Before you start`) puts it inside the scratch directory so it is removed with it. If you
  open a new shell mid-walk, set it again.

## Redaction

Run before pasting anything, on everything:

```bash
sed -E 's/(sk-[A-Za-z0-9_-]{4})[A-Za-z0-9_-]+/\1REDACTED/g'
```

(the `redact` function above is this plus a second expression for `*_API_KEY=` assignments).

- Never paste `.env`. Never `cat` it.
- Keys never go on a command line. Use `read -rs` for every key.
- Never put a key in a file you keep, or in any paste slot.
- Before you paste, scan the text once for `sk-` followed by 20 or more key characters. If you
  find one, redact by hand. The agent scans every slot again before committing and stops the
  plan on any hit.

## Cleanup

Do this as soon as the last step is done, even if the walk failed part-way:

```bash
shred -u "$WALK/.env" 2>/dev/null || rm -f "$WALK/.env"
unset OPENAI_API_KEY ANTHROPIC_API_KEY
cd / && rm -rf "$WALK"
test ! -e "$WALK" && echo cleaned
```

The `shred` line falls back to `rm -f "$WALK/.env"` where `shred` is not available. Paste the last
line (`cleaned`) into `K9`. If you opened extra shells that exported a key, close them.

## Verified by the keyless half

<!-- filled by plan 56-06 with the commands the agent ran keyless, and any correction to the steps above -->

Filled 2026-11-02 by plan 56-06. Evidence, provenance and the step table are in `56-FIRST-RUN-WALK.md`. Every command
below ran with `OPENAI_API_KEY`, `ANTHROPIC_API_KEY` and `DEEPSEEK_API_KEY` unset, in a scratch directory outside any
git repository, against crates.io `0.11.0` (`cargo metadata`: `paladin-ai 0.11.0
registry+https://github.com/rust-lang/crates.io-index`).

Commands run and their exit codes:

| walk step | command (keys unset) | exit |
|-----------|----------------------|------|
| 1 | `cargo new dogfood-walk`, `cargo add paladin-ai@0.11.0`, `cargo add tokio@1 --features full`, `cargo fetch` | 0 |
| 2 | README `[dependencies]` block as published, `cargo metadata` (resolves `paladin-ai 0.5.1`) | 0 |
| 3 | README Quick Example, `cargo run` (prints `Hello from Paladin!`) | 0 |
| 4 | `basic_paladin.rs` copied from the downloaded package, `cargo run --example basic_paladin` | 101 (needs `paladin-ports`), 0 once added |
| 5 | tag Quickstart block as written, `cargo metadata` (resolves `0.10.1`) | 0 |
| 5 | `cargo add paladin-ports@0.11.0`, `cargo add paladin-llm@0.11.0 --features openai`, tag `main` as `src/bin/quickstart.rs`, `cargo run --bin quickstart` | 1 (`Error: "OPENAI_API_KEY environment variable not set"`) |
| 5b | installation page block with `llm-openai`, `cargo metadata` | 101 |
| 5b | the same block with `openai`, `cargo metadata` | 0 |
| 7 | `cargo install paladin-ai --version 0.11.0 --bin paladin-cli --root <scratch>/cli` | 101 |
| 7 | `cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root <scratch>/cli` | 0 |
| 8 | `paladin-cli --version` (`paladin 0.11.0`), `paladin-cli --help` | 0, 0 |
| 9 | `paladin-cli setup-check`, `... --verbose`, `... --quiet`, three provider keys, `REDIS_URL`, `QDRANT_URL` unset, empty directory | 2, 2, 2 |
| 10 | `timeout 20 paladin-cli onboarding < /dev/null` | 2 (`Error: User input error: IO error: not a terminal`) |
| 11 | `paladin onboarding` and `paladin setup-check` with only the install's `bin` on `PATH` | 127, 127 |

As a line: step 1 exit=0; 2 exit=0; 3 exit=0; 4 exit=101 then exit=0; 5 exit=1; 5b exit=101 then exit=0; 7 exit=101 then
exit=0; 8 exit=0; 9 exit=2; 10 exit=2; 11 exit=127.

**The K4 install line is proven.** `cargo install paladin-ai --version 0.11.0 --features cli --locked --bin
paladin-cli --root "$WALK/cli"` exited 0 in 5m 12s (release profile) and produced a working `paladin-cli`. Assumption
A1 holds. It used about 1.2 GB of scratch target (free space 5476 MB to 4316 MB).

Not run: nothing was skipped for disk. Not run keylessly, because they need the key: the Quickstart's real provider
call (K2 with a key), the trace watch-list (K3), `onboarding` through its prompts and the key validation (K6),
`setup-check` with a configured key (K7) and the Anthropic row (K8).

Corrections to the steps above, as dated notes (the steps themselves are unchanged):

- **2026-11-02, K2.** The tag's Quickstart does not only "may" say `0.10.0`; it does. Its block is `paladin-ai`,
  `paladin-ports` at `"0.10.0"` and `paladin-llm = { version = "0.10.0", features = ["openai"] }`, which resolves to
  `0.10.1`. Use `0.11.0` as the step says and name the change in K2/K9. Logged as DF-13 (duplicate of DF-02); do not
  log it again. With `0.11.0` the program compiled and, with no key, stopped at `OPENAI_API_KEY environment variable
  not set`, so the keyed run is the first one that reaches the provider.
- **2026-11-02, K3.** The Quickstart `main.rs` installs no tracing subscriber (`use` list: builder, execution service,
  circuit breaker, `LlmPort`, `OpenAIAdapter`), so `RUST_LOG` has no effect and `none` proves nothing, as the step
  already warns. Say so in K3.
- **2026-11-02, K4.** The step says that if the docs gave a different install command, try it first. No doc gives a
  `cargo install` line for the CLI (DF-15). The line a reader would try, without `--features cli`, exits 101 with
  "target paladin-cli in package paladin-ai requires the features: cli". Both are logged; K9 needs no new
  entry for them.
- **2026-11-02, K6/K7.** The CLI names itself `paladin`: its Next Steps box and warnings say `paladin onboarding`
  and `paladin setup-check`, and `paladin-cli --version` prints `paladin 0.11.0`. This is DF-17, already logged. For
  K7: the section order of `setup-check` changes from run to run (DF-16), so two pastes of the same output can differ
  in order only. With no `.env` the exit code is 2 (documented: warnings); after a successful onboarding in the same
  directory the `.env` warning should go and the exit may become 0.
- **2026-11-02, K6.** Without a TTY `onboarding` prints its welcome banner and exits 2 with `Error: User input error:
  IO error: not a terminal`, and writes nothing. Run it in a real terminal.
- **2026-11-02, K9.** Already logged by this walk, no need to log again: DF-10, DF-11, DF-12, DF-13, DF-14, DF-15,
  DF-16, DF-17. Log only what is new from the keyed steps.

## Paste slots

Fill these in place, after redaction. Each has an HTML comment marking where to paste, then an
empty fenced block.

### K1 — environment

<!-- paste below this line; redacted -->
```
not captured; the K1 command block (rustc -V; cargo -V; date; uname) was not run as a block.
derived from K7/K6: rustc 1.97.1 (8bab26f4f 2026-07-14), Linux host, run date 2026-11-03.
```

### K2 — Quickstart run

<!-- paste below this line; redacted -->
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

### K3 — trace watch-list matches

<!-- paste below this line; redacted -->
```
 RUST_LOG=info cargo run 2>&1 | redact | grep -E 'unprofiled_model|usage_unreconciled'; echo "grep_exit=${PIPESTATUS[2]}"
grep_exit=1

Result: `none`. The Quickstart `main.rs` installs no tracing subscriber, so no log lines were
printed and this result is not evidence either way. No `tracing-subscriber` was added.
```

### K4 — CLI install

<!-- paste below this line; redacted -->
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

### K5 — CLI version

<!-- paste below this line; redacted -->
```
"$WALK/cli/bin/paladin-cli" --version
paladin 0.11.0
```

### K6 — onboarding

<!-- paste below this line; redacted -->
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

### K7 — setup-check

<!-- paste below this line; redacted -->
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

### K8 — Anthropic row (optional)

<!-- paste below this line; redacted -->
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

### K9 — stumbles, and the cleanup line

<!-- paste below this line; redacted -->
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

## Triage checklist

For plan 56-08. Start only from the filled slots.

- [ ] Scan every slot with the secret pattern from `## Redaction` (`sk-[A-Za-z0-9_-]{20,}`, and
  any `API_KEY=` line with a value other than `REDACTED`) **before committing anything**. A hit
  stops the plan and goes back to the maintainer. It is never committed, and the maintainer
  should rotate that key.
- [ ] Log each `K9` stumble and each watch-list hit in `K3` as a DF with `**By:** maintainer`.
  Grade it under D-04, re-check it on HEAD (D-00e) and disposition it under D-05. Entries must
  parse under `make check-findings-log`; the next free id is the one after the last
  `### DF-nn` in `.project/v0.11.1/02-dogfood-findings-log.md`.
- [ ] Copy the redacted outputs into `56-FIRST-RUN-WALK.md` under `## Keyed half (maintainer)`.
