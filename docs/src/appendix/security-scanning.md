# Dependency Security & License Compliance

This document describes Paladin's supply-chain security tooling: vulnerability
scanning, license compliance, the exception process, and Software Bill of
Materials (SBOM) generation. It is part of **Milestone 10 — CI Hardening and
Release Automation, Epic 2**.

## Tooling Overview

| Concern | Tool | Where it runs | Config / source of truth |
|---------|------|---------------|--------------------------|
| Known vulnerabilities (RustSec) | `cargo audit` | CI (`security-audit` job) + local | `.cargo/audit.toml` |
| Known vulnerabilities (OSV DB) | OSV-Scanner | CI (`osv-scanner` job, PR annotations) | `Cargo.lock` |
| License compliance + bans + duplicates | `cargo deny` | CI (`cargo-deny` job) + local | `deny.toml` |
| Software Bill of Materials | `cargo cyclonedx` | Release pipeline | `Cargo.lock` |

## Running the Checks Locally

```bash
# Vulnerability advisories (reads exceptions from .cargo/audit.toml)
cargo audit

# License policy, bans, duplicate versions, advisories (reads deny.toml)
cargo deny check

# Both at once
make security

# Generate a CycloneDX SBOM for the workspace
make sbom
```

Install the tools once with:

```bash
cargo install --locked cargo-audit cargo-deny cargo-cyclonedx
```

## License Policy

`deny.toml` enforces a **permissive-only** allow-list:

- Allowed (core): `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`.
- Allowed (additional permissive, each justified in `deny.toml`): `Unicode-3.0`,
  `0BSD`, `CC0-1.0`, `CDLA-Permissive-2.0`.
- Strong copyleft licenses (`GPL-*`, `AGPL-*`, `LGPL-*`) are **not** allowed.
- Weak/file-level copyleft (`MPL-2.0`) is **not** in the global allow-list; it is
  granted only via narrowly-scoped per-crate `[[licenses.exceptions]]` entries so
  the global policy stays permissive-only.

If a required dependency uses a license outside this set, do **not** disable the
license check. Instead, either:

1. Add the specific SPDX license id to `deny.toml`'s `[licenses].allow` list with
   a comment justifying it (for genuinely permissive licenses), or
2. Add a narrowly-scoped `[[licenses.exceptions]]` entry granting a specific
   license to a specific crate (preferred for weak copyleft like `MPL-2.0`), or
3. Add a `[[licenses.clarify]]` entry for a specific crate when its license
   metadata is ambiguous.

## Advisory Exception Process

Some advisories cannot be remediated immediately (typically transitive or
dev/test-only dependencies with no upstream fix). Exceptions are recorded in
**two synchronized files**:

- `.cargo/audit.toml` — auto-discovered by `cargo audit`.
- `deny.toml` (`[advisories].ignore`) — used by `cargo deny`.

Each exception **must** include a comment stating:

1. The advisory ID (e.g. `RUSTSEC-2023-0071`).
2. The affected crate and why it is in the tree (e.g. transitive dev dependency
   of `sqlx-mysql`).
3. Why it is not yet fixable (no upstream patch available).
4. A revisit condition (e.g. "revisit when sqlx upgrades rsa").

When adding or removing an exception, update **both** files so the two scanners
do not contradict each other.

Current tracked exceptions (the full `.cargo/audit.toml` `[advisories].ignore` array):

- `RUSTSEC-2023-0071` — RSA timing side-channel via `rsa 0.9.x` (transitive
  dev/test dep of `sqlx-mysql`; no upstream fix).
- `RUSTSEC-2025-0111` — `tokio-tar` path traversal (transitive dev/test dep of
  `testcontainers`; no upstream fix).
- `RUSTSEC-2026-0187` — stack overflow in `lopdf` via deeply nested PDF objects
  (transitive via `pdf-extract`, an unconditional dependency of `paladin-content`;
  reachability is gated by whether the facade's optional `paladin-content` dependency
  is enabled, ADR-0032. Fix requires a breaking `pdf-extract` >= 0.12 jump; deferred).
- `RUSTSEC-2026-0194` — `quick-xml` quadratic attribute parsing (DoS); the remaining
  < 0.41 instance is transitive via `rust-s3`/`aws-creds` (optional `s3` feature); no
  `rust-s3` release uses `quick-xml` >= 0.41 yet.
- `RUSTSEC-2026-0195` — `quick-xml` unbounded namespace allocation (DoS); same
  transitive path and revisit condition as `RUSTSEC-2026-0194`.

## OSV-Scanner Policy

OSV-Scanner runs on pull requests and reports findings as **PR annotations**
(via SARIF upload). It is currently **annotate-only (non-blocking)** to avoid
contradicting the `cargo audit` gate while the annotation signal level is
assessed. It may be promoted to a blocking gate later (see PRD Open Question 1).

## Snyk Evaluation & Decision

**Decision: evaluated and removed (2026-08-18).** Do not reintroduce a Snyk scan
step, and do not record a phase as blocked on one.

Snyk was evaluated against the combined coverage of `cargo audit` (RustSec),
OSV-Scanner (OSV database), and `cargo deny` (licenses + bans + duplicates), and
measured directly against this workspace rather than assumed:

- **Snyk Code (SAST)** ingests `.rs` files but has no meaningful Rust rules. A probe
  carrying a hardcoded credential, command injection via `sh -c`, path traversal and
  SQL injection returned **0 findings**. The same four probes in JavaScript returned
  3 findings (HIGH/MEDIUM/LOW), confirming the scanner and credentials worked — the
  zero-Rust-findings result is a genuine coverage gap, not a broken evaluation.
- **Snyk Open Source (SCA)** has no Cargo support; `snyk test` exits
  `SNYK-CLI-0008 — no supported target files` on this workspace.

A "clean" Snyk result on this workspace means *nothing was analysed*, not *the code
is clean* — worse than no scan at all, because it reads as assurance the project does
not have.

**Rationale:** The existing three tools (`cargo audit`, OSV-Scanner, `cargo deny`)
already cover advisories and license compliance with no external account, no secret
management, and fully version-controlled policy (`.cargo/audit.toml`, `deny.toml`).
Snyk provides zero incremental Rust coverage on this workspace, so its
account/secret-management overhead (`SNYK_TOKEN`) is not justified.

**Standing instruction:** Snyk was evaluated and removed; it is not reintroduced, and
no phase should be recorded as blocked pending a Snyk step.

## Known Gap: No Rust SAST

CodeQL was evaluated as a Rust-capable SAST candidate and **disqualified** as a
required-check-grade Rust SAST at the tested version — CodeQL CLI `2.26.3`,
`rust-queries` `0.1.40`, `security-extended` query suite, evaluated 2026-08-25.
`.github/workflows/codeql.yml` is **retained, advisory-only**: it runs on every
push/PR/schedule and reports findings in the code-scanning UI, but it is not pinned
in any ruleset and does not gate a merge.

Measured, not assumed: across four independent fixture measurements, SQL injection,
path traversal and regex injection built from a `reqwest` remote source never fired
under any tested condition; only `rust/hard-coded-cryptographic-value` fired
reliably, and it carries a real false-positive cost on this codebase's own code
(alert #28, a test-fixture literal, not a leaked secret). Coverage is not the gap —
`analysed_rs_files` read 100% of the file denominator on every run — the gap is a
measured detection gap in the rule classes that matter for credential-handling code.

There is still no static taint analysis of first-party Rust that gates a merge.
`cargo-audit` and `cargo-deny` scan dependencies; `clippy` is a lint. The manual
credential-handling review (response bodies redacted before truncation, no API key
interpolated into logs, HTTP clients carrying a credential header never following
redirects) remains the **primary control** for credential-handling code — this is
stated plainly rather than letting CodeQL's retained advisory scan read as coverage
it does not provide.

## SBOM

Every GitHub release attaches a CycloneDX SBOM
(`paladin-<version>.cdx.json`) generated from the locked dependency graph by the
`sbom` job in `.github/workflows/release.yml`. Generate the SBOMs locally with
`make sbom`, which runs `cargo cyclonedx --all --format json` and writes one
`<crate>.cdx.json` next to each workspace crate's manifest (the root package's
`paladin-ai.cdx.json` is the primary deliverable). These generated files are
git-ignored.
