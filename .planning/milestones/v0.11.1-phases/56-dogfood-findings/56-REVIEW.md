---
phase: 56-dogfood-findings
reviewed: 2026-11-04T00:00:00Z
depth: standard
files_reviewed: 35
files_reviewed_list: see 56-10 code-review scope (35 files from the 14 SUMMARYs)
findings:
  critical: 0
  warning: 9
  info: 4
  total: 13
status: issues_found
---


# Phase 56: Code Review Report

## Summary

The four security and correctness fixes hold up under review.

- **DF-07, redacting `Debug`:** `Settings`, `ApiKeyConfig`, `QueueConfig`, `FileStorageConfig`, `EmailAdapterConfig` and `OtelConfig` all redact their credentials. I checked the other nested types in `Settings` for credential-bearing fields and found none with a derived `Debug`. `lettre::Credentials` has an empty `Debug`, so `EmailNotificationAdapter`'s derived `Debug` is safe.
- **DF-19, `provider_probe`:** redaction runs before truncation (`diagnostic_excerpt` calls `redact_credentials` and then `bounded_excerpt`). One client is used, with `Policy::none()` and a test that proves a 307 is not followed.
- **DF-08/DF-26, `load_settings`:** overlays are applied before `validate()`. The treasurer refusal still comes first.
- **`src/main.rs`:** the whole-`Settings` log line is gone.

No BLOCKER-class defect found in the shipped code paths. The warnings are:

- the findings-log checker has confirmed false-pass holes;
- `#[serial]` isolation is incomplete for `set_var`;
- some credential paths survive beside the fixed ones;
- some docs are wrong.

## Warnings

### WR-01: Checker silently drops a malformed `### DF-` heading (confirmed false pass)

**File:** `scripts/check-findings-log.sh:91,149-158`
**Issue:** `HEAD_RE = ^### (DF-(\d+))\b` requires a word boundary after the digits. `### DF-02a title` (or `DF-7_x`) fails `HEAD_RE`, but still matches `ANY_HEAD_RE`, so the `else` branch sets `current = None`. The entry is not counted, not reported, and its `- **Disposition:** open` bullet is ignored. I built a fixture with that heading and an `open` disposition; the script printed OK and exited 0. This breaks the "never vacuous" contract the header advertises (DOGF-01).
**Fix:** treat any `### DF-` heading that fails the strict form as a named failure.
```python
LOOSE_HEAD_RE = re.compile(r"^### DF-")
...
head = HEAD_RE.match(line)
if head: ...
elif LOOSE_HEAD_RE.match(line):
    malformed.append((lineno, line))   # emit BAD_HEADING and fail
```

### WR-02: Checker accepts vacuous "final" dispositions (confirmed)

**File:** `scripts/check-findings-log.sh:97,203,214-221`
**Issue:** I confirmed three false passes with fixtures:
- `out () (x)` passes. `OUT_REASON_RE = ^\((.+)\)` is greedy and unanchored at the end, so `.+` swallows `) (x` and the empty first pair is accepted.
- `DF-03: duplicate of DF-04` together with `DF-04: duplicate of DF-03` passes. Nothing detects a duplicate cycle or a duplicate pointing at another duplicate, so a finding can be "closed" with no real disposition.
- `in v0.11.1 — TBD` passes. The pointer check only requires some non-punctuation text, so "TBD", "none" and "todo" count as evidence.

**Fix:**
- Anchor the reason regex, e.g. `^\(([^()]*(?:\([^()]*\)[^()]*)*)\)\s*$`, or require the reason to start with a non-space character before the first `)`.
- Follow `duplicate of` chains and fail on a cycle or on a target that is itself a `duplicate of`.
- Optionally reject a pointer in a small stop-list (TBD/none/todo/tbc).

### WR-03: `setup-check` echoes the raw `REDIS_URL` on a format error

**File:** `src/application/cli/commands/setup_check.rs:74-77`
**Issue:** `check_redis_connectivity` returns `Invalid Redis URL format: {redis_url}`, and the caller wraps it into the printed Fail line. A `REDIS_URL` with a typo'd scheme, for example `redis:/:s3cret@host`, `valkey://:pw@host` or `http://u:pw@host`, prints its password to the terminal and to CI logs under `--quiet`. This is the exact "credential reaches an error message" class DF-07 targets. `check_qdrant_connectivity` has a similar, lower-confidence exposure: it interpolates `QDRANT_URL` into a reqwest error, and `reqwest::Error` Display includes the URL.
**Fix:** do not echo the value.
```rust
Err(CliError::configuration(
    "Invalid Redis URL format: expected a redis:// or rediss:// URL (value not shown)".to_string(),
))
```
For Qdrant, call `e.without_url()` on the reqwest error before formatting it.

### WR-04: Onboarding writes `.env` (live API keys) with default permissions

**File:** `src/application/cli/commands/onboarding.rs:548`
**Issue:** `std::fs::write(".env", content)` creates the file with mode 0666 & ~umask, normally 0644, so it is world-readable on a shared host. The file holds provider API keys. The "Overwrite" path also truncates an existing `.env` with no backup.
**Fix:** on unix, create the file with `OpenOptions::new().write(true).create(true).truncate(true).mode(0o600)` (via `std::os::unix::fs::OpenOptionsExt`). If the file already exists, also `set_permissions` to 0600.

### WR-05: Onboarding reports "All API keys validated successfully!" after failures

**File:** `src/application/cli/commands/onboarding.rs:318-339`
**Issue:** the loop only warns on a failed validation, and then line 338 unconditionally prints `formatter.success("All API keys validated successfully!")`. A rejected key (for instance the DF-19 404 case this phase fixes) is followed by a success banner, which is misleading output on a credential check.
**Fix:** count failures in the loop. Print the success message only when the count is 0, otherwise print a warning that says how many keys failed.

### WR-06: `#[serial]` does not protect against concurrent env readers (unsafe `set_var` in edition 2024)

**File:** `src/bin/paladin-server.rs:866-893` (and the non-serial tests, e.g. `default_config_spawns_no_run_services` at ~1673)
**Issue:** the SAFETY comments claim "every test that sets or reads these prefixes is `#[serial]`". But `#[serial]` only excludes other `#[serial]` tests. Non-serial tests run concurrently on other harness threads. `default_config_spawns_no_run_services` calls `build_run_api(&Settings::default(), ...)`, and the `build_auth_config_*` and `server_wires_*` tests run `apply_env_overrides`-style env reads. A `setenv` racing a `getenv` is the memory-unsafety the 2024 `unsafe` marker exists for. Separately, `with_overlay_env` clears only the three overlay prefixes. `load_settings` also reads `APP_TREASURER_*` (via `get_treasurer_config`), so a developer environment with `APP_TREASURER_CURRENCY` set makes `load_settings_accepts_the_shipped_example_config` and `..._keeps_the_treasurer_refusal_first` fail or flake.
**Fix:** mark every test that reads env as `#[serial]`, or move the env-mutating tests to their own integration-test binary. Add `APP_TREASURER_` to `OVERLAY_PREFIXES`.

### WR-07: Round-trip test contradicts its own "no APP_* env" claim

**File:** `tests/config_example_roundtrip.rs:21,349-367`
**Issue:** the module doc says the file "sets, reads and depends on no `APP_*` environment variable (D-12)". But `full_fixture_round_trips_through_settings_new` calls `Settings::new()`, which applies `Environment::with_prefix("APP")` and reads `APP_ENV` to pick an extra `config.{env}` file. The test therefore depends on the developer or CI environment. A stray `APP_*` variable of the wrong type fails deserialisation. Its `set_current_dir` is also process-global while the other tests run concurrently; they use absolute paths, so that is safe today but fragile.
**Fix:** run this test in a cleared-`APP_*` guard (as in `paladin-server.rs`), or correct the doc claim. Keep `#[serial]`.

### WR-08: Docs name an environment variable that does not exist, and the rename to `paladin-cli` is partial

**File:** `docs/src/appendix/cli-setup-check.md:367`, `docs/src/appendix/cli-council.md`, `cli-muster.md`, `cli-usage.md`
**Issue:**
- The troubleshooting step "Increase timeout: `PALADIN_REQUEST_TIMEOUT=10000 paladin-cli setup-check`" is not read anywhere in `src/` or `crates/`. `probe_client()` hard-codes 10 s. The phase touched this exact line and kept the fiction.
- The sample output on lines 103-110 (`Connected [342ms]`, `Models: ...`, "Invalid API key format") is not what the command prints.
- The rename covers only `setup-check` and `onboarding`. About 147 invocations of `paladin council|muster|agent|features` remain. The `paladin` binary (`src/main.rs`) accepts only `--config`, so those commands cannot run as written, and pages now mix both names.

**Fix:** delete the timeout step (or implement the variable). Finish the `paladin` to `paladin-cli` rename across `docs/src/appendix/cli-*.md`, or record the remainder as a logged finding.

### WR-09: DF-07 redacts `Debug` only; `Serialize` still emits every secret in plaintext

**File:** `src/config/settings.rs:34`, `src/config/agents.rs:96,111`, `src/config/queue.rs`, `src/config/file_storage.rs`, `crates/paladin-notifications/src/email_notification_adapter.rs:66`
**Issue:** `Settings`, `ApiKeyConfig.key`, `llm_api_key`, `redis_password`, `minio_*_key` and the SMTP `password` all derive `Serialize` with no `skip_serializing`. The round-trip test's own header admits `ApiKeyConfig.key` "serialises in plaintext". `LlmProviderConfig.api_key` and `AllowanceWebhookConfig.secret` already use `skip_serializing`, so the house pattern is inconsistent. I found no current call site in `src/` that serialises `Settings` outward, so this is a latent leak, not an active one.
**Fix:** document the invariant that `Settings` must never be serialised outward, and add a guard test or lint. Or apply `#[serde(skip_serializing)]` to these fields, if the round-trip test's key-path diff can tolerate extending `EXEMPT`.

## Info

### IN-01: Failed config load is printed with `{:?}`

**File:** `src/main.rs:54`
**Issue:** `eprintln!("Failed to load configuration: {:?}", e)` prints the `config`-crate error. For a type mismatch, that error echoes the offending value (`invalid type: string "...", expected u16 for key ...`). If the mis-typed key is a credential field (for example an unquoted numeric `redis_password`), the value appears on stderr.
**Fix:** print `{}` and rely on the key path, or accept the residual risk and note it.

### IN-02: `std::env::vars()` panics on a non-UTF-8 variable

**File:** `src/bin/paladin-server.rs:851`
**Issue:** `overlay_vars()` uses `std::env::vars()`, which panics if any process variable is not valid UTF-8.
**Fix:** use `vars_os()` and filter on the lossy name.

### IN-03: Unparsable overlay values are silently ignored at boot

**File:** `src/bin/paladin-server.rs:126-143`
**Issue:** the doc notes "an unset or unparsable variable leaves the file value". `PALADIN_TRACE_CHANNEL_CAPACITY=10O24` therefore boots on the file value with no warning, which weakens DF-26's intent of "the judged values are the running values".
**Fix:** log a `warn!` naming the variable (never the value) when an overlay variable is present but fails to parse. This needs a change in the `apply_env_overrides` impls.

### IN-04: Checker does not validate the disposition date

**File:** `scripts/check-findings-log.sh:93`
**Issue:** the `(?: \(([^)]*)\))?` group captures anything, e.g. `**Disposition (soon):**`, and the usage text says the bullet is dated. This is cosmetic, since the date is never used.
**Fix:** require `\d{4}-\d{2}-\d{2}` if dated bullets are meant to be enforced.

_Reviewed: 2026-11-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
