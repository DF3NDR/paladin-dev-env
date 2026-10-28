---
phase: 52
slug: ci-release-pipeline-repair
status: verified
created: 2026-10-28
---

# Phase 52: CI and release pipeline repair, evidence record

> Measured facts for the phase's CI gates. Each section records the command, the tool versions, the tree and the
> output that was actually observed, so a later reader can repeat the measurement rather than trust a summary.

## D-02 allow-table measurement

Plan 52-05, Task 1. Purpose: prove that the 28 crate-wide `cargo-semver-checks` allow lines (six manifests) were
redundant at the published `0.11.0` baseline, by removing every table and then running the gate over all twelve
published packages.

- HEAD SHA when measured (manifests edited in the working tree, uncommitted): `95659216ab4a514c3c615194638130f9d9bd9944`
- `cargo semver-checks --version`: `cargo-semver-checks 0.50.0`
- `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Baseline: published `0.11.0` from crates.io, `--release-type patch` (fixed by flag; every manifest equals the baseline version).
- Per-package command, run through `scripts/check-semver.sh --package <pkg>` with `SEMVER_PRUNE_LOCAL_CACHE=1`:

```text
cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch
```

The twelve packages are the `CRATES` array of `scripts/publish-crates.sh`. The script was invoked once per package
rather than once for all twelve; the loop body, the command and the check-line parsing are the same, and the
per-package form let free disk space be read after every package (the sandbox had about 575 MB free).

| Package | Check line | Exit |
|---|---|---|
| `paladin-ai-core` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-ports` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-herald` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-battalion` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-llm` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-memory` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-web` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-notifications` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-content` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-storage` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-eval` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-ai` | 223 checks: 223 pass, 31 skip | 0 |

Every package printed `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`.

Lints that fired: none.

### Removed allow lines

| Manifest | Package | `= "allow"` lines removed |
|---|---|---|
| `Cargo.toml` | `paladin-ai` | 1 |
| `crates/paladin-core/Cargo.toml` | `paladin-ai-core` | 11 |
| `crates/paladin-battalion/Cargo.toml` | `paladin-battalion` | 2 |
| `crates/paladin-llm/Cargo.toml` | `paladin-llm` | 4 |
| `crates/paladin-ports/Cargo.toml` | `paladin-ports` | 6 |
| `crates/paladin-web/Cargo.toml` | `paladin-web` | 4 |

Total 1 + 11 + 2 + 4 + 6 + 4 = 28. The removed text is the `[package.metadata.cargo-semver-checks.lints]` table, any
comment lines inside it, and the contiguous comment block above it that justified the allows. `git diff -U0` showed
only comment lines, the six table headers, six separating blank lines and the 28 allow lines removed, and no added line.

### Disk and one retry

- Free space before the first package and after each of the first eleven: 575 MB (volume `/dev/vda`, 99% used).
- The first attempt at `paladin-web` failed with exit 101. This was not a lint: the rustdoc build of the `0.11.0`
  baseline ran out of space (`No space left on device`, a linker bus error in `rust-embed-impl`, then a failed
  `rmeta` write), so no check line was produced. The stale `0.9.0` baseline directory
  `target/semver-checks/registry-paladin_web-0_9_0-*` (about 571 MB, no longer used by any gate) was removed once,
  which raised free space to 1.2 GB. The retry of `paladin-web` then produced the line recorded above, with no
  `No space left` text in its log. Free space after each of the remaining packages: 1.2 GB.

### Known limit

RESEARCH Pitfall 9 (the coupling of this measurement to the exact toolchain and its rustdoc JSON format) is the
known limit of this measurement: a different `rustc` or `cargo-semver-checks` version can change the lint set that
runs or what it reports, so the result above is exact for `cargo-semver-checks 0.50.0` on `rustc 1.97.1` only.
