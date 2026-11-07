---
phase: 58-crate-release
reviewed: 2026-11-06T00:00:00Z
depth: standard
files_reviewed: 30
files_reviewed_list:
  - .devcontainer/Dockerfile.dev
  - .project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-notifications/CHANGELOG.md
  - crates/paladin-notifications/src/email_notification_adapter.rs
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/src/minio.rs
  - crates/paladin-storage/src/node_cache/redis.rs
  - crates/paladin-storage/src/redis.rs
  - crates/paladin-web/openapi.json
  - docs/src/api-reference/migration-guide.md
  - docs/src/api-reference/upgrading.md
  - docs/src/appendix/release-checklist.md
  - scripts/check-doc-models.allowlist
  - scripts/check-doc-models.sh
  - scripts/check-review-dispositions.sh
  - scripts/sdk-smoke/run.sh
  - src/application/cli/commands/onboarding.rs
  - src/application/cli/commands/setup_check.rs
  - src/config/agents.rs
  - src/config/file_storage.rs
  - src/config/node_cache.rs
  - src/config/queue.rs
  - src/config/settings.rs
  - tests/config_example_roundtrip.rs
  - tests/consumer_surface_muster.rs
  - tests/scripts/check-doc-models_test.sh
  - tests/scripts/check-review-dispositions_test.sh
findings:
  critical: 0
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 58: Code Review Report

**Reviewed:** 2026-11-06
**Depth:** standard
**Files Reviewed:** 30
**Status:** issues_found

## Summary

Reviewed the Phase 58 delta (`git diff 7832f280e HEAD`) for the listed files. Advisory and post-release: every item below is a follow-up, not a fix to v0.11.1.

No BLOCKER-class defect was found. The credential-handling changes are correct as far as they go:

- Each `skip_serializing` field has a load pin and a non-vacuous omission test.
- The hand-written `Debug` impls are guarded by drift tests.
- The Redis and Qdrant error texts no longer echo the URL.
- `write_env_file_at` really creates 0600 and narrows an older 0644 file.
- Both new gate scripts and their tests pass here (`check-doc-models.sh` reports `OK: 184 file(s) scanned ... allowlisted: 1`; the two shell test files pass 8 and 34 assertions).

The main concern is completeness. The hardening is described as covering "every credential field" of `Settings`, and it does not. Two of the gates are narrower than the intent stated in their own headers.

The real OpenAI key that `LIVE_API_TESTS_FIX.md` carried remains in git history. The 58-05 SUMMARY records it as rotated on 2026-11-05 with the history purge waived, so I did not raise it.

## Warnings

### WR-01: `Settings` Serialize claims to omit every credential, but `trace.otel.headers` still serialises in plaintext

**File:** `src/config/settings.rs:34-37` (doc claim); `src/config/trace.rs:139` (the leaking field)
**Issue:** The new `Settings` doc says "`Serialize` omits every credential field (WR-09)". `Settings.trace: TraceConfig` holds `otel: OtelConfig`, whose `headers: BTreeMap<String, String>` is documented in its own rustdoc as "Credential-shaped" (an `authorization` bearer token for the collector). It is redacted in `Debug` but has no `skip_serializing`, so `serde_json::to_string(&settings)` emits the bearer token. Neither `settings_serialisation_omits_every_credential_it_reaches` nor the `config_example_roundtrip` EXEMPT table covers it, so the tests cannot catch it. The same class exists on `RedisRunQueueConfig.connection_url` (`crates/paladin-storage/src/run_queue/redis.rs:280`, which embeds `redis://:password@host` and has a hand-written redacting `Debug` but a plain `Serialize`). That type is not reachable from `Settings`, so it is lower priority.
**Fix:** Add `#[serde(skip_serializing)]` to `OtelConfig.headers` (it already has `#[serde(default)]` at struct level, so loading is unchanged). Add the `trace.otel.headers` sentinel to `settings_serialisation_omits_every_credential_it_reaches` and an EXEMPT row to `tests/config_example_roundtrip.rs`. Either decide `RedisRunQueueConfig.connection_url` is in scope or soften the doc to name the fields actually covered. Add a table-driven test that walks every secret-shaped field name (`password|secret|token|key|headers|url`) under `Settings` so the next one is caught.

### WR-02: Lossy `Serialize` on public config types is a behaviour change shipped in a patch release

**File:** `crates/paladin-storage/src/minio.rs:55-62`, `crates/paladin-storage/src/redis.rs:48-50`, `crates/paladin-storage/src/node_cache/redis.rs:110-112`, `crates/paladin-notifications/src/email_notification_adapter.rs:67-69`
**Issue:** `MinioConfig.access_key`/`secret_key` and `EmailAdapterConfig.password` are `skip_serializing` with no serde default. `serde_json::from_str(&serde_json::to_string(&cfg)?)` now fails with `missing field` for any library consumer that persists or ships these configs through serde. The repo's own test had to be edited to insert the keys by hand. `cargo-semver-checks` cannot see this, and the CHANGELOGs say "No field, type or signature changes" while documenting the serde change, so the entries are accurate. The risk is that a library consumer upgrading 0.11.0 to 0.11.1 gets a runtime deserialisation failure, not a compile error.
**Fix:** Make sure `MIGRATION.md` M-B-50/51/52 (and the consumer-facing upgrading page) tell consumers in plain words that serialise-then-deserialise of these types no longer round-trips, and what to use instead. For future releases, treat a change in serde behaviour of a public type as a minor-bump item, or add an explicit CHANGELOG "Behaviour change" heading.

### WR-03: `.env` write is a truncate-then-write, and merge mode silently discards the existing file on a read error

**File:** `src/application/cli/commands/onboarding.rs:538-551, 568-579`
**Issue:** The new `write_env_file_at` opens with `.truncate(true)` and then writes, so a failure between the two (disk full, crash, or `set_permissions` returning `EPERM` on a file the user does not own) leaves the user's `.env`, which is what holds their other provider keys, empty. `set_permissions` is a hard `?` error where the old `std::fs::write` would have succeeded. Separately, merge mode reads the existing file with `read_to_string(".env").ok()`. Any read error (permission, non-UTF-8 content) becomes `None`, so the "merge" silently regenerates the file from only the newly entered keys and overwrites everything else. The truncate-before-write behaviour pre-dates the phase, but the new helper is where it can be fixed.
**Fix:** Write to a sibling temp file created with `OpenOptions::mode(0o600)`, `sync_all`, then `rename` over `.env`. That is atomic and makes the new file 0600 regardless of the old file's mode. In merge mode, propagate the read error (treat only `ErrorKind::NotFound` as "no existing file").

### WR-04: `check-doc-models.sh` scans a narrower set than "the material a new user copies"

**File:** `scripts/check-doc-models.sh:15-17, 119-128` (`discover`)
**Issue:** The header says the gate keeps retired ids out of copyable material. It scans only `README.md`, `docs/src/**/*.md` and `examples/**`. Remaining copyable sites that carry the retired id:

- `k8s/configmap.yaml:14` has `default_model: "gpt-4"` in a deployable manifest.
- `config.test.yml:19` also carries it.
- Public rustdoc, which is what docs.rs renders for crate users, has about 87 `///`/`//!` lines with `"gpt-4"`. One is a full YAML example at `src/config/agents.rs:382` (`model: "gpt-4"`).
- Per-crate READMEs and `config.example.yml` are not scanned. They are clean today, but nothing keeps them clean.

The original failure (Quickstart `.model("gpt-4")` giving `model_not_found`) is exactly the shape that rustdoc doc-tests and the k8s ConfigMap reproduce, and the gate stays green over them.
**Fix:** Extend `discover()` to `k8s/**/*.yaml`, `config*.yml`, `crates/*/README.md` and doc-comment lines of `crates/**/*.rs` and `src/**/*.rs` (limit the `.rs` scan to lines starting `///` or `//!`). Fix or allowlist the hits with reasons. Or state the narrower scope in the header and the Makefile help so the gate does not read as broader than it is.

### WR-05: `check-review-dispositions.sh` pointer heuristic still accepts pointer-shaped prose

**File:** `scripts/check-review-dispositions.sh:196-215` (`has_pointer`)
**Issue:** The 57-REVIEW WR-02 stated goal is that prose that merely looks pointer-shaped is not evidence. I extracted `has_pointer` and ran it. All of these return `True`:

- ``fixed by `and/or` `` and ``see `a/b` `` are accepted because any backticked span containing `x/y` counts as a repo path, with no extension check.
- ``covered by `max_retry_count` `` is accepted because any backticked snake_case token with two underscores counts as a test name.
- `verified manually, TLS-1 ok` is accepted because `TLS-1` is read as a requirement id (`TLS` is not in `NON_EVIDENCE_PREFIXES`).

A `closed` row with no real evidence therefore still passes the gate. The tests cover the new rejections (dates, bare numbers, `UTF-8`) but not these.
**Fix:** Require a backticked path to end in a known extension or to exist in the working tree (`os.path.exists(os.path.join(workspace, span))`). Require a backticked test name to appear as `fn <name>` somewhere in the repo, or to be `::`-joined. Replace the `NON_EVIDENCE_PREFIXES` deny list with a requirement-id allow list built from the ids in `REQUIREMENTS.md`. Add fixture rows for the cases above to `tests/scripts/check-review-dispositions_test.sh`.

## Info

### IN-01: `RedactedSecret` is copy-pasted twice inside one crate

**File:** `crates/paladin-storage/src/minio.rs:26-43` and `crates/paladin-storage/src/redis.rs:20-37`
**Issue:** The two definitions are identical. The doc comment justifies a private copy per crate (a shared helper would need a `lib.rs` change), but here both copies are in the same crate, and a third redaction convention sits in `node_cache/redis.rs`. A future change to the redaction rule (for example showing the length) has to be made in three places.
**Fix:** Add a single `pub(crate) mod redaction` in `paladin-storage` (one `mod` line in `lib.rs`) and use it from both files.

### IN-02: Files that cannot be decoded are skipped but still counted as scanned

**File:** `scripts/check-doc-models.sh:166-171`, `scripts/check-doc-models.sh:197-200`
**Issue:** An `OSError` or `UnicodeDecodeError` on a file executes `continue`, but the final `OK: <len(files)> file(s) scanned` uses `len(files)`, so the count includes skipped files. A copyable file that stopped being valid UTF-8 would drop out of the gate while the count looked unchanged. The header says a non-UTF-8 file is "skipped", so the behaviour is documented, but the count is misleading.
**Fix:** Track `scanned` and `skipped` separately and print `skipped: <n>` in the OK line, or warn on each skip.

### IN-03: SDK smoke builds the generated client with unpinned `npm install` and a `main`-only build check

**File:** `scripts/sdk-smoke/run.sh:53-60`
**Issue:** `npm install` in `target/sdk/typescript` resolves the client's devDependencies (`typescript`) fresh on every run. The smoke package itself uses `npm ci`, but the client build is not reproducible, so a new `typescript` release can turn the gate red with no repo change. The build check reads `package.json#main`, and a generator template that switches to `exports`/`module` only would fail with a misleading "was not built" error.
**Fix:** Pin `typescript` in the generator's package template, or install with `npm install --no-audit --no-fund typescript@<pinned>` first. Fall back to checking `exports["."]` and `types` when `main` is empty.

### IN-04: The Qdrant test's query-string case does not exercise a well-formed request

**File:** `src/application/cli/commands/setup_check.rs:88-90` (pre-existing URL build) and the new test at `:497-520`
**Issue:** `format!("{}/collections", qdrant_url.trim_end_matches('/'))` appends the path after any query string, so `http://127.0.0.1:1?api-key=PW` becomes `...?api-key=PW/collections`. The test still passes, because the connection is refused either way, but that URL shape is not a request a real server would receive, so the comment that "reqwest still echoes a credential carried in the query string" is only loosely demonstrated. Separately, `check_redis_connectivity` only checks the scheme, so its name and the "Connection failed" wording in `check_redis` overstate what is checked. Both are pre-existing.
**Fix:** Build the URL with `reqwest::Url::parse(qdrant_url)` and `url.join`/`path_segments_mut().push("collections")` so the query survives, and rename the Redis function to `check_redis_url_format`.

---

_Reviewed: 2026-11-06_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
