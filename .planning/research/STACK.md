# Stack Research

**Domain:** Rust enterprise multi-agent orchestration framework — output-side spend governance
(pricing/cost, allowances, spend ledger, rate pacing) plus one infra swap (RustFS for MinIO) and
one perf fix (trace serialization). Milestone v0.11.0 "Treasurer Spend Governance," phases starting
at Phase 38.

**Researched:** 2026-09-24
**Confidence:** HIGH for crate identity/version/license (verified against crates.io/GitHub source,
cross-checked against the workspace's own `Cargo.toml`/`Cargo.lock`); MEDIUM for the RustFS S3
API-parity claim (no compatibility-matrix page could be fetched directly — `docs.rustfs.com` and
`lib.rs` are blocked by this environment's egress proxy; verified instead via GitHub README/release
notes and Docker Hub); MEDIUM for `governor`'s exact MSRV (not declared in its `Cargo.toml` — infer
low risk from wide adoption, verify empirically in CI's existing MSRV job before merging).

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `rust_decimal` | 1.43.0 (MIT) | Parse operator-entered per-model unit prices (config/YAML) and do the price × token-count multiplication without binary-float rounding error | The only new dependency this milestone strictly needs for money math. Binary `f64` cannot exactly represent decimal fractions like `0.000002` (a realistic per-token USD price) — multiplying it by a token count and summing across a campaign accumulates drift, which is the wrong failure mode for a spend *ledger*. `rust_decimal` is the de facto standard for this in the Rust ecosystem (paupino/rust-decimal), pure-Rust, `no_std`-capable, MSRV well under the workspace's 1.88 floor (measured ~1.67 as of a recent release; current MSRV not independently re-verified for 1.43.0 but the crate has never required anything close to 1.88). **Scope it narrowly**: used only at the config-parse / price-computation boundary — not as the ledger's persisted column type (see "What NOT to Use"). |
| Fixed-point `i64` micro-USD (or nano-USD) integer | n/a — plain Rust, no crate | The **persisted** unit for the spend ledger (new port + SQLite/Postgres adapters) and for the value actually summed/compared during allowance enforcement | This is not a new dependency — it is the architectural decision the money-representation question resolves to. `sqlx` **deliberately does not support `rust_decimal`/`bigdecimal` for SQLite** (the sqlx maintainers rejected it specifically because SQLite's `NUMERIC` affinity silently backs onto `f64`, which they judged worse than no support at all — see Sources). Since `paladin-storage`'s SQLite adapter is the workspace's always-on, unconditional backend (not feature-gated behind `sqlite` the way MySQL/Postgres are optional), any ledger design that leans on `sqlx`'s `rust_decimal` feature only works for two of the three backends and needs a separate TEXT-string round-trip for SQLite — extra code, extra bug surface, and a real risk of Postgres/SQLite drift. A plain `i64` (BIGINT/INTEGER) column is native to `sqlx` on **every** backend with zero extra feature flags, mirrors the house convention that token counts (`TokenUsage`) are already plain integers (the VOCAB-01…07 "units plain" rule, ADR-0049/0050), and is the same design real billing systems use (Stripe: integer cents) for exactly this reason. Quantize `Decimal` → `i64` once, at the boundary where a run's cost is computed; never carry `Decimal` into a persisted row. |
| `governor` | 0.10.4 (MIT) | In-process rate pacing / back-off against provider 429s, keyed per provider+model | GCRA-based (leaky-bucket-equivalent), lock-free (64-bit CAS state, "10× faster than mutex-based" per its own docs), async-aware (`until_ready()` resolves a `Delay`-style future), and its default feature set already includes `dashmap` — a concurrent keyed-limiter map, which is exactly the "per provider, per model" pacing shape R4 needs with no extra crate. It is **in-process only** — confirmed no Redis/distributed backend exists or is planned; every replica keeps its own bucket, so N workers each individually rate-limit at the configured rate (N× the intended ceiling in aggregate). That gap is real and is why the Redis-shared tier below exists as a *separate* mechanism, not a `governor` feature. |
| (no new crate) — hand-rolled Redis Lua GCRA/lock, via the already-declared `redis` crate | `redis` stays pinned at `0.32.2` (already a workspace dependency, `script` feature already enabled) | Cross-worker shared rate-pacing state and the distributed cache-stampede lock (both explicitly named in this milestone's R4/FUT-09 scope) | The workspace already has a proven, in-tree precedent for exactly this shape: `crates/paladin-storage/src/run_queue/redis.rs` implements the Redis-backed `RunQueuePort` as four atomic `redis::Script` (`EVAL`) calls — claim/extend/ack/nack — each reading the server's own `TIME` command so no client clock is trusted, with `Script::invoke_async`'s built-in `NOSCRIPT` reload handling relied on rather than hand-rolled. A GCRA token-bucket-per-key script (state = last-update timestamp + token count in a Redis HASH or a pair of keys, updated atomically) and a `SET key token NX PX ttl` + compare-and-delete Lua unlock script for the stampede lock are the same idiom, reusing the identical `redis::Script`/`ConnectionManager` machinery already wired for the queue adapter. This needs **zero new dependencies and zero new Cargo features** — `redis`'s `script` feature is already on. Adding a third-party crate (`redis-rate`, `limitador`, `rslock`/`redlock-rs`) here would introduce a second idiom for "atomic Redis coordination" next to the one already proven and reviewed in this codebase, for no capability gain at the single-Redis-instance scale this workspace runs at (true multi-Redis-node Redlock quorum is not in scope — R4 asks for pacing shared *across workers of one deployment*, not cross-datacenter consensus). |
| `httpdate` | 1.0.3 (MIT) — **already resolved transitively** (via `reqwest`/hyper), promote to a direct dependency | Parse the `Retry-After` response header's HTTP-date form (RFC 7231 §7.1.3) when a provider sends a date instead of a delay-in-seconds, feeding the 429 back-off R4 requires | `LlmError::RateLimitExceeded` today is a bare unit variant — no adapter in `crates/paladin-llm` currently reads or carries a `Retry-After` value at all (`grep` confirms zero hits workspace-wide). Implementing R4 means teaching each adapter's `map_http_status`/429 path to read the header. The delay-seconds form (`Retry-After: 120`) is a trivial `str::parse::<u64>()` — no crate needed — but the header also legally carries an HTTP-date, and `httpdate` is already sitting in `Cargo.lock` at `1.0.3` as a transitive dependency of the existing `reqwest`/`hyper` stack. Promoting an already-resolved transitive dependency to a direct one adds **zero new packages to the graph** — the exact "no new package, no version-drift risk" justification this workspace's own `Cargo.toml` already uses for `croner`, `hmac`, and `schemars` (see comments there). Parse-seconds-first, fall back to `httpdate::parse_http_date` only if the seconds parse fails. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rust_decimal` `serde-with-str` feature | bundled with 1.43.0 | Deserialize operator-authored price-table entries (e.g. `prompt_price_per_1k: "0.0025"`) from the existing `config`-crate-loaded YAML as exact decimals, not via a float intermediate | Enable on the `rust_decimal` dependency wherever the price-table config struct lives. Do **not** enable `serde-float` (round-trips through `f64`, reintroducing the exact problem this dependency exists to avoid) or the arbitrary-precision JSON features (unneeded weight; this workspace's config path is YAML via the `config`/`serde_yaml` crates already in the tree, not arbitrary-precision JSON numbers). |
| `governor`'s `dashmap` default feature | bundled with 0.10.4 | Per-key (provider+model, or tenant+key) in-process limiter state without a hand-rolled `HashMap<K, RateLimiter>` + `Mutex`/`RwLock` | Default-on; no extra Cargo line needed beyond the base `governor` dependency. |
| `rust-s3` | already pinned `0.35.1` (0.37.2 is current upstream) | The existing MinIO/S3 adapter's HTTP client — likely **reusable as-is** against a RustFS backend, not necessarily replaced | See the RustFS section below: RustFS's S3 API is a wire-protocol match, so the *first* thing to try is pointing the existing `MinioAdapter`'s `endpoint`/credentials config at a RustFS container, before writing a second adapter. A version bump to 0.37.2 is optional housekeeping, independent of this milestone, and should be its own change if taken (two minor version jumps, not evaluated here for breaking changes). |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo bench --bench engine_benchmarks -- bench_superstep_cost` | Re-measure `TraceDispatcher`/`LogTraceSink` overhead after the D-16 fix | This exact invocation and fixture (`bench_superstep_cost_sink_variants`, `build_width_graph(8)`) already produced the +22.18%/+18.46% baseline in `28-BENCH-EVIDENCE.md` — reuse it unchanged so the before/after numbers are comparable, not a new synthetic fixture. |
| `cargo deny check` (`make deny` / `make security`) | License gate for every new dependency above | `rust_decimal` (MIT), `governor` (MIT), `httpdate` (MIT) all fall inside the existing permissive allow-list in `deny.toml` (`MIT`, `Apache-2.0`, `BSD-2/3-Clause`, `ISC`, `Zlib`, plus the named exceptions) with **no new exception entries required**. |

## Tracing overhead (D-16, row 35) — no new crate recommended as the primary fix

**Root cause, from `28-BENCH-EVIDENCE.md`'s own analysis (confirmed by reading
`LogTraceSink::write_trace_line` and `TraceDispatcher::emit` directly):**

1. `LogTraceSink::write_trace_line` calls `serde_json::to_string(value)` **unconditionally**, every
   call, before ever reaching the `log::info!("{json}")` macro invocation. The `json` binding is a
   captured-by-reference format argument, not a closure — `log`'s own `log!` macro only skips the
   *macro body* when the target/level is filtered, but the `json` value referenced inside the
   `format_args!` must already exist, so the serialization cost is paid even when
   `RUST_LOG=paladin::trace=off` would otherwise make the line a no-op. This is a genuine,
   zero-dependency bug-shaped inefficiency, not an inherent cost of the design.
2. `serde_json::to_string` allocates a fresh `String` on every call — one allocation per trace
   record, and an 8-node superstep emits 20+ records (the bench evidence doc's own count).
3. `TraceDispatcher::emit` separately pays a `Mutex<VecDeque>`-guarded push plus an
   `AtomicU64::fetch_add` and a `try_send` doorbell wakeup — smaller in absolute terms per the
   evidence doc, but compounds at this call volume against a ~110µs baseline superstep.

**Recommended fix, in priority order, with no new dependency for the first two:**

| Fix | Mechanism | Expected effect |
|-----|-----------|------------------|
| Guard serialization with `log::log_enabled!` | `if log::log_enabled!(target: "paladin::trace", log::Level::Info) { serialize + log::info! }` in `write_trace_line` | Makes `trace.log_sink: false` / `RUST_LOG=…=off` genuinely free (currently it is not) — a correctness fix as much as a perf one. Does **not** move the default-on benchmark number, since that config has the sink enabled. |
| Reuse a buffer with `serde_json::to_writer` instead of `to_string` | A `thread_local!`-scoped or pooled `Vec<u8>`, cleared and reused per call, written via `serde_json::to_writer(&mut buf, &record)`, then passed to `log::info!` as `%s` over `std::str::from_utf8(&buf)` | Removes the one-allocation-per-event cost `to_string()` pays every call — directly targets the evidence doc's own named suspect ("serialization is the likely dominant cost"). No new dependency; a pure `serde_json`-API change. |
| Re-derive the ≤3% bar against an I/O-bound fixture (PRD 07's own "path 1", not a code change) | N/A | The evidence doc itself flags that ~110µs is an unrealistically small baseline for a *real* Paladin node (which waits on network I/O); the same fixed per-record dispatcher cost is a much smaller fraction of a realistic run. This is a measurement-methodology fix, not a stack change, but is cheaper than any code change and should be tried before further engineering. |

**Explicitly evaluated and NOT recommended:** replacing `serde_json` with a SIMD serializer
(`sonic-rs` 0.2.x/0.1.4, Apache-2.0/MIT) at this call site. It is meaningfully faster than
`serde_json` for both parsing and serialization, but (a) it leans on `unsafe` internals gated by CPU
SIMD extension availability, which cuts against this workspace's house rule to avoid `unsafe` unless
required and fully documented, applied here transitively rather than in first-party code; (b) it
does not support a `to_writer`-style streaming API as of the versions checked, so it cannot combine
with the buffer-reuse fix above; (c) `serde_json` is used at 30+ call sites across
`paladin-battalion` (Waypoint/Battlefield serialization, cache-key hashing, golden-tested fingerprint
bytes) and at least one existing test (`log_sink_writes_one_json_line_per_record`) asserts the
**byte-order position** of specific JSON keys in `LogTraceSink`'s output — a workspace-wide or even
single-call-site serializer swap risks silently changing key ordering or float formatting in a way
those tests (and any external consumer parsing the trace-log JSON) would notice. If the two
no-new-dependency fixes above still leave the measured overhead above whatever bar Phase 38+
ultimately adopts, re-evaluate `sonic-rs` narrowly scoped to `LogTraceSink` only, behind its own
measured before/after `cargo bench` run — do not adopt speculatively.

## RustFS (FUT-10) — replacing the terminal quay.io MinIO pin

| Question | Finding | Confidence |
|----------|---------|------------|
| Current release | **1.0.0 GA, tagged `1.0.0`, released 2026-09-16** — 8 days before this research. Pin the exact tag (`rustfs/rustfs:1.0.0`), never `:latest` — this mirrors the house pattern already used for the terminal MinIO pin (`quay.io/minio/minio:RELEASE.2025-09-07T…`) and for `mc` (checksum-verified release asset), and the project's own recheck-by note for this exact todo already anticipated pinning discipline. | HIGH (Docker Hub + GitHub release tag both confirm) |
| License | Apache-2.0 — inside `deny.toml`'s allow-list already, no exception needed. | HIGH |
| Multi-arch images | `linux/amd64` and `linux/arm64` both built (project's own `docker-buildx.sh`), matching the existing multi-arch Docker image requirement this workspace already has for its own release image. | MEDIUM (from README, not independently pulled/inspected) |
| Health check | S3 API exposes `GET /health` on the same port as the S3 API (9000 by default) — the project's own `docker-compose.yml` healthcheck curls this plus the console port (9001) on a 30s interval / 40s start period. Directly reusable for `docker/docker-compose.test.yml`'s existing MinIO healthcheck shape. | HIGH |
| S3 API surface vs what `paladin-storage` calls | `crates/paladin-storage/src/minio.rs` uses `rust-s3`'s `Bucket::new`/`Bucket::create`, `put_object_with_content_type`, `get_object`, `presign_put`, `presign_get` — standard SigV4-presigned single-part PUT/GET plus bucket creation. RustFS's README claims "broad S3 API compatibility for supported features" with a dedicated compatibility matrix, but that matrix page (`docs.rustfs.com`) could not be fetched directly in this environment (proxy-blocked) to confirm presigned-URL and bucket-creation parity item-by-item. **This is the one open item the todo's own "adapter-parity integration tests" step exists to close** — the recommendation below is designed around not needing to trust the matrix. | MEDIUM — do not treat as settled; verify empirically |
| Admin/bootstrap CLI (the `mc`-equivalent) | RustFS ships its own S3-compatible client, `rc`, distributed via Nix and (per Docker Hub) as a separate `rustfs/rc` image — this is the direct swap-in for whatever `docker/docker-compose.test.yml`/CI currently uses `mc` for (bucket creation, policy bootstrap). | MEDIUM (name and image confirmed; exact command surface not independently verified against the specific `mc` invocations this repo's compose/CI files use — check those against `rc --help` before wiring in) |

**Recommended integration shape, informed by the "no new Rust crate" possibility the todo itself
left open as a question ("RustFS's S3 API surface coverage versus what `paladin-storage` actually
calls"):**

1. **Try reuse before rewrite.** Since RustFS is wire-compatible S3 (not a different SDK), the
   cheapest first step is pointing the *existing* `rust-s3`-based `MinioAdapter`
   (`crates/paladin-storage/src/minio.rs`) at a RustFS container via its existing `endpoint`/`region`/
   credentials config — no new adapter file, no new Cargo feature. Only fall back to the todo's
   originally-sketched second adapter (new file, new feature flag, adapter-parity contract suite) if
   the existing contract-test suite the `s3` feature already has fails against RustFS — e.g. if
   presigned-URL signature validation or a specific header RustFS doesn't yet implement breaks a
   real call. This test *is* the compatibility matrix, run for real against this repo's actual
   call pattern instead of trusted from a doc page.
2. **If a second adapter does prove necessary**, it still needs no new *crate* — `rust-s3` remains
   the client; only the `endpoint`/adapter selection logic and the Cargo feature flag are new,
   consistent with the "SQL-backed repository adapters" pattern (`sqlite`/`mysql`/`postgres` features
   sharing one `sqlx` dependency) already used four times over in this same crate.
3. **`mc` → `rc`** in `docker/docker-compose.test.yml`/`docker/docker-compose.yml`/CI bucket-bootstrap
   steps and in `k8s/minio.yaml` if that manifest is moved too (the todo leaves this as an open
   question — the PRD context above scopes only "dev/test stack, CI jobs and k8s smoke test," which
   reads as yes for the smoke test specifically, not necessarily the production reference manifest).

## Installation

```bash
# Wherever the Treasurer's price-table / cost-computation logic lands (new module or crate — this
# research does not prescribe the crate boundary, only the dependency):
cargo add rust_decimal@1.43 --no-default-features --features "std,serde,serde-with-str,macros"

# governor for in-process 429 pacing (dashmap-keyed limiter is a default feature, no extra flag):
cargo add governor@0.10

# Promote the already-transitively-resolved httpdate to a direct dependency for Retry-After parsing:
cargo add httpdate@1.0.3

# No `cargo add` needed for: the Redis-shared GCRA/lock (reuses the already-declared `redis` crate
# and its already-enabled `script` feature), the ledger's `i64` persisted column (plain sqlx/Rust),
# or RustFS (a container image + compose/CI/k8s config change, not a Cargo dependency at all unless
# the reuse-first adapter test in the RustFS section above fails).
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|--------------------------|
| `i64` fixed-point micro/nano-USD as the ledger's persisted column | `rust_decimal::Decimal` via `sqlx`'s `rust_decimal` feature | Only if the Postgres/MySQL-only nature of that `sqlx` feature is acceptable and SQLite support is dropped from the ledger port entirely — not viable here, since SQLite is this workspace's always-on default backend and the `RunRepositoryPort` pattern this milestone explicitly follows (R5) requires all three backends to share one contract. |
| Hand-rolled Redis Lua GCRA/lock via the existing `redis::Script` idiom | `redis-rate` / `limitador` / `rslock`/`redlock-rs` crates | If true multi-node Redis quorum locking (Redlock across independent Redis masters, not one shared instance) becomes a requirement, `rslock` is the closest maintained Redlock implementation and would be worth a fresh evaluation then — it is not warranted for this milestone's single-Redis-instance scope, and would introduce a second "atomic Redis coordination" idiom next to the one already proven in `run_queue/redis.rs`. |
| `serde_json::to_writer` + reused buffer for `LogTraceSink` | `sonic-rs` | If the two no-new-dependency fixes are implemented, re-measured with the same `bench_superstep_cost_sink_variants` fixture, and still fail whatever bar Phase 38+ adopts — see the tracing-overhead section above for the full caveat list before reaching for it. |
| Reuse the existing `rust-s3`-based adapter against RustFS | A hand-written RustFS-native client (RustFS also plans/has a native Rust SDK per its broader ecosystem) | Only if `rust-s3`'s SigV4 presigning or a specific call this workspace makes genuinely fails RustFS's implementation — verify with the adapter-parity contract-test run before assuming a native SDK is needed. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|--------------|
| `f64`/binary float for anything **summed or compared** in the spend ledger or allowance logic | Binary floating point cannot exactly represent most decimal fractions; multiplying a per-token USD price by a token count and accumulating across a campaign compounds rounding error — exactly the failure mode a spend *ledger* (an accounting artifact someone will reconcile against an invoice) cannot tolerate. The *existing* `ExecutionMetadata.cost_estimate: Option<f64>` field can stay `f64` at the **display/API boundary** (it is not persisted, not summed, not compared for allowance decisions) — but no additive/comparison logic should read it as authoritative; derive it from the `i64` ledger value, never the reverse. | `i64` fixed-point for persisted/summed values; `rust_decimal::Decimal` only at the config-parse and per-run-computation boundary, immediately quantized to `i64` before it touches the ledger. |
| `sqlx`'s `rust_decimal` (or `bigdecimal`) feature | Deliberately unsupported for SQLite by the `sqlx` maintainers themselves (SQLite's `NUMERIC` affinity is `f64`-backed; they judged a working-but-lossy mapping worse than none) — enabling it only covers Postgres/MySQL, splitting this workspace's three-backend ledger port into two different persisted representations. Also carries a known (closed, but illustrative of the coupling) build-break history requiring `bigdecimal` to be enabled alongside `rust_decimal` even when `bigdecimal` types are never used. | Plain `i64`/`BIGINT` column, native on every `sqlx` backend with no extra feature. |
| `redis-rate`, `limitador`, or any other off-the-shelf Redis-rate-limiting crate | Lower download/maintenance signal than `governor` itself, and — more importantly — this workspace already has a reviewed, tested, in-tree idiom for atomic Redis coordination (`redis::Script` + server-`TIME`-sourced clock, D-08) that a new crate would sit awkwardly next to rather than reuse. | Hand-rolled Lua GCRA script via the existing `redis::Script`/`ConnectionManager` pattern in `paladin-storage`. |
| `sonic-rs` / `simd-json` as a workspace-wide or reflexive `serde_json` replacement | Heavy `unsafe` internals gated on CPU SIMD extensions, immature streaming/`to_writer` support, and a 30+-call-site blast radius across `paladin-battalion` including at least one test asserting JSON key byte-order — see the tracing-overhead section for the full reasoning. | The two no-new-dependency fixes (enablement guard + buffer reuse) first; re-evaluate narrowly only if still insufficient. |
| `:latest` (or any floating) tag for the RustFS Docker image | This whole FUT-10 item exists **because** a floating/untagged community image disappeared out from under CI with no warning (Docker Hub deleted the MinIO community repos, 2026-09-12) — repeating that pattern with a different vendor defeats the point of the swap. | The exact `1.0.0` GA tag (or a later exact tag as RustFS releases, re-pinned deliberately each time — never automatically). |

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|------------------|-------|
| `rust_decimal 1.43.0` | Rust 1.88 (workspace MSRV) | MSRV comfortably clears 1.88 — a recent release measured at ~1.67; no conflict expected, but run the existing MSRV CI job on the actual dependency addition rather than trusting this figure, per this workspace's own "measured, not declared" MSRV discipline (`Cargo.toml`'s own MSRV comment). |
| `governor 0.10.4` | Rust 1.88, `tokio` 1.x (workspace's existing async runtime) | No explicit `rust-version` field found in its manifest; widely deployed at current stable Rust with no known 1.88-specific issue found in this research — verify with the workspace's MSRV job rather than treating this as settled (this is the one MEDIUM-confidence version claim in this document). |
| `httpdate 1.0.3` | Already resolved in this exact version in `Cargo.lock` (transitive via `reqwest`/hyper) | Promoting it to a direct dependency changes nothing in the resolved graph — verify with `cargo tree -i httpdate` after the change shows no second version introduced, mirroring how this workspace already verified `schemars`' promotion the same way. |
| `redis 0.32.2` (unchanged) | Its `script` feature, already enabled in `paladin-storage`'s `[dependencies]` | No version bump needed for either the Redis-shared pacing or the distributed lock — both are new Lua script text and new Rust call sites against the crate/feature set already in the tree. |
| `rust-s3 0.35.1` (unchanged, pending the reuse-first RustFS step) | RustFS 1.0.0's S3 API, *if* SigV4 presigning and bucket-create parity hold under the adapter-parity contract test | Not independently confirmed in this research (see the RustFS table's MEDIUM-confidence row) — this is exactly what Phase 38+'s own contract-test step should settle before committing to either the reuse path or the second-adapter path. |

## Sources

- This workspace's own `Cargo.toml` (workspace deps, MSRV comment, `croner`/`hmac`/`schemars`
  "no new package" promotion precedent), `crates/paladin-storage/Cargo.toml` and
  `crates/paladin-storage/src/run_queue/redis.rs` (the Lua lease-script precedent), and
  `crates/paladin-storage/src/minio.rs` (the existing `rust-s3` call surface) — read directly.
- `deny.toml` (license allow-list) — read directly.
- `crates/paladin-core/src/platform/container/herald.rs` (current `ExecutionMetadata.cost_estimate:
  Option<f64>` shape and its "reserved for the Treasurer" doc comments) — read directly.
- `crates/paladin-llm/src/error.rs` (confirms `LlmError::RateLimitExceeded` is a bare unit variant
  today, no `Retry-After` capture anywhere in `crates/paladin-llm`) — read directly.
- `src/infrastructure/telemetry/log_sink.rs` and
  `.planning/milestones/v0.10.0-phases/28-observability-tooling/28-BENCH-EVIDENCE.md` (the measured
  +22.18%/+18.46% tracing overhead, its own root-cause analysis, and its own two suggested paths
  forward) — read directly.
- [SQLx `sqlite` type-mapping rationale for rejecting `rust_decimal`/`bigdecimal`](https://github.com/launchbadge/sqlx) and [`NUMERIC` and `DECIMAL` support for SQLite discussion](https://github.com/launchbadge/sqlx/issues/2887) — web search, confidence HIGH (sqlx maintainers' own stated design rationale).
- [`rust_decimal`/`bigdecimal` sqlx feature coupling issue #2549](https://github.com/launchbadge/sqlx/issues/2549) — closed; cited as illustrative of the feature's coupling cost, not as a currently-open bug.
- [`paupino/rust-decimal` `Cargo.toml`](https://github.com/paupino/rust-decimal) (feature names: `serde-with-str`, `serde-float`, `db-postgres`, etc.) and crates.io metadata (`1.43.0`, MIT) — fetched directly.
- [`boinkor-net/governor`](https://github.com/boinkor-net/governor) (GCRA description, lock-free performance claim, default feature set) and crates.io metadata (`0.10.4`, MIT) — fetched directly.
- Distributed-limiting gap in `governor` (in-process only, N replicas → N× quota) — web search, cross-referenced against the crate's own architecture (no Redis backend in its dependency tree).
- [`rustfs/rustfs` GitHub repository](https://github.com/rustfs/rustfs) (README: Apache-2.0, multi-arch `linux/amd64`/`linux/arm64`, `rc` CLI) and its [`1.0.0` release tag](https://github.com/rustfs/rustfs/releases/tag/1.0.0) (GA, 2026-09-16) — fetched directly; `docs.rustfs.com` and `lib.rs` were proxy-blocked in this environment and could not be independently fetched, which is why the S3-compatibility-matrix claim is MEDIUM confidence rather than HIGH.
- `rustfs/rustfs` `docker-compose.yml` (ports 9000/9001, `/health` healthcheck, env var names) — fetched directly from the GitHub repo.
- `httpdate 1.0.3` presence — verified directly against this workspace's own `Cargo.lock`.
- [`sonic-rs`](https://github.com/cloudwego/sonic-rs) (SIMD JSON, `unsafe`-heavy, streaming-API gap) — web search, cross-referenced with its own README's stated caveats.

---
*Stack research for: v0.11.0 Treasurer Spend Governance*
*Researched: 2026-09-24*
