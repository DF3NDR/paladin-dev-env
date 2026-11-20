# Security Exception Register

This file is the authoritative **governance** register for every RustSec advisory this Paladin
workspace suppresses. It exists because neither `.cargo/audit.toml` nor `deny.toml` can hold
governance metadata: both express a suppression as a bare string inside an `[advisories] ignore`
array, and an array of strings can carry a trailing comment but never a queryable field. The four
fields a governed exception needs — a named owner, a review date, the affected scope, and a
concretely-stated compensating control — can only ever live as prose above the array. A prose
comment is not queryable and cannot be gated by a script. This register gives those fields a
structured home.

Two configuration files mirror this register mechanically, and neither is itself authoritative for
governance:

- **`.cargo/audit.toml`** is the authoritative *suppression* surface for `cargo audit`. Its
  `[advisories] ignore` array holds exactly the fourteen `vulnerability`-class rows below, and nothing
  else.
- **`deny.toml`** mirrors `.cargo/audit.toml`'s vulnerability class exactly, and adds one
  clearly-labelled `unmaintained` class of its own. `cargo audit` does not fail a build on an
  unmaintained (not-vulnerable) advisory; `cargo deny` does, and Milestone 10 Epic 4 FR-1 step 5
  authorises scoped `unmaintained`-class ignores with an explanatory comment.

**`scripts/check-advisory-register.sh`** (landing in this phase's plan 09-06) is the script that
enforces the relationship the paragraph above only asserts in prose. It reads all three files plus
`Cargo.lock` and fails non-zero if any of the following does not hold: the vulnerability-class IDs
in `deny.toml` and `.cargo/audit.toml` are set-equal; every ID suppressed in either TOML file has a
row below carrying all eleven fields non-empty, and every row below maps to a live suppression in
at least one of the two TOML files (a register row with no matching suppression is stale
bookkeeping, not documented risk, and fails the guard); and every suppressed advisory's named crate
still appears in `Cargo.lock` (the staleness check nothing previously performed — see the note
below on the four suppressions this register deliberately does not carry). Run it locally with
`make check-advisory-register`.

**Schema contract, stated plainly:** adding an advisory ID to either TOML file's `ignore` array
without adding a corresponding row here fails CI. Adding a row here with no live suppression in
either TOML file also fails CI — a row can only exist to govern a suppression that actually exists.

**This file is an exception register, not a GitHub-facing `SECURITY.md`.** There is no
`SECURITY.md` in this repository today. Adding one — for GitHub's private vulnerability-reporting
and security-advisory UI, aimed at a security researcher disclosing a *new* finding rather than a
developer auditing an *existing* suppression — is a separate deliverable for a different audience,
and is deliberately not addressed by this file.

## The twenty-three live suppressions

Fourteen vulnerability-class advisories (mirrored exactly between `.cargo/audit.toml` and `deny.toml`)
and nine unmaintained-class advisories (`deny.toml` only). This is the tree-verified live set, not
the fifteen the corpus's own record remembers — see the note immediately below the payload for why
four advisories that appear in `deny.toml`'s history are absent from this register on purpose.

*(Count corrected 2026-08-24 by the v0.8.0 milestone audit. This heading and paragraph read "ten"
and "five unmaintained-class" from the moment Phase 9 authored them (commit `a587e5a1`, plan
09-02), when both figures were true. Phase 15.1 added the eleventh row — `RUSTSEC-2026-0249`
(`smartstring`, transitive via `rxml`/`minidom` under `rust-s3`'s optional `s3` feature) — under
its own D-11, updating the machine-readable payload below but not this prose above it (commit
`d955998a`, plan 15.1-01). No gate was ever affected: `scripts/check-advisory-register.sh` parses
`deny.toml` and `.cargo/audit.toml` directly and never reads these words, and it reports "11
register row(s) checked against 11 deny.toml and 5 .cargo/audit.toml ignore entries; all clauses
satisfied", exit 0. Only the register's account of itself was stale.)*

*(Count updated 2026-10-19 at the Phase 50 seal: three vulnerability-class rows were added for `RUSTSEC-2026-0367/0368/0369` (`zip 3.0.0`, build-dependency of `utoipa-swagger-ui` under `paladin-web`), published 2026-10-09/10 with no upstream release that reaches the patched `zip >= 9`; the rows carry the build-time-only scope and the never-calls-`extract` compensating control.)*

*(Count updated 2026-11-04 at Phase 57 (REG-06, D-30 to D-32): a fourth vulnerability-class row was added for `RUSTSEC-2026-0429` (`zip 3.0.0`, `ZipArchive::extract` aborts the process on a symlink entry with a large declared size; published 2026-10-22, in the advisory-db 2026-11-02, and `make security` was red until this row). The earlier rows' revisit condition (an upstream release that reaches `zip >= 9`) was met on 2026-11-04 by `utoipa-swagger-ui` 10.0.2, which depends on `zip ^9.0` (build) but also requires `utoipa ^6.0.0`; the lift was not taken in v0.11.1 because it is the utoipa 6 migration, not a build-dependency bump. All four rows were re-dated to the D-31 review date and revisit condition, and the compensating control was re-measured from the 9.0.2 build script (`by_index` + `enclosed_name()`, no method-call use of the extraction API).)*

*(Twelfth row added 2026-10-14: `RUSTSEC-2024-0449` (`unsafe-libyaml`, the parser behind
`serde_yaml` 0.9 in `paladin-eval` and the facade's optional `cli` feature), an unmaintained-class
advisory that newly failed CI's `License & Dependency Policy` job on every branch. The heading and
the paragraph above were updated in the same commit, so this time the prose and the payload agree:
12 rows, 12 `deny.toml` entries, 5 `.cargo/audit.toml` entries.)*

*(Count updated 2026-11-20 on the z.ai adapter branch: seven rows were added because `make security`
and the required `Security Audit` and `License & Dependency Policy` checks were red on `main` and on
every pull request, and `protect-main-branch.json` carries no bypass. Five are vulnerability-class:
`RUSTSEC-2026-0457` and `-0462` (`rmcp 2.1.0`), `RUSTSEC-2026-0459` (`libsqlite3-sys 0.30.1`) and
`RUSTSEC-2026-0545` and `-0548` (`zip 3.0.0`). Two are unmaintained-class: `RUSTSEC-2026-0543`
(`chrono`) and `-0555` (`chrono-tz`). Every row's fix is a major- or minor-version migration owned by
the v0.12.0 milestone. One of them, `RUSTSEC-2026-0462`, is a live exposure with only a partial
compensating control; its row says so. The six `lettre` advisories published 2026-11-17 are not here
because they were fixed by a lockfile bump to 0.11.24. 23 rows, 23 `deny.toml` entries, 14
`.cargo/audit.toml` entries.)*

<!-- BEGIN MACHINE-READABLE REGISTER -->
```toml
[[exception]]
id = "RUSTSEC-2023-0071"
class = "vulnerability"
crate = "rsa"
path = "rsa -> sqlx-mysql -> sqlx -> workspace crates"
why_present = "Transitive dev/test dependency of sqlx-mysql, pulled into the workspace only through the sqlx mysql backend used by testcontainers-based integration tests."
why_not_fixable = "No fixed version of rsa is available upstream; sqlx-mysql has not yet upgraded its rsa dependency past the vulnerable 0.9.x range."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "dev/test dependency graph only, via testcontainers-based sqlx-mysql integration tests; absent from a release build's default dependency set"
compensating_control = "The Marvin timing side-channel is only reachable inside dev/test-scoped sqlx-mysql code paths exercised by CI, never by a shipped release binary."
revisit_condition = "sqlx-mysql upgrades its transitive rsa dependency past the vulnerable 0.9.x range"

[[exception]]
id = "RUSTSEC-2025-0111"
class = "vulnerability"
crate = "tokio-tar"
path = "tokio-tar -> testcontainers"
why_present = "Transitive dependency of testcontainers, used only by integration tests that provision ephemeral Docker containers."
why_not_fixable = "No fixed release of tokio-tar exists upstream, and testcontainers has not migrated away from the tokio-tar dependency."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "dev/test dependency graph only, via testcontainers integration tests; not present in a release build"
compensating_control = "tokio-tar's PAX header extraction runs only inside CI/test-runner processes against fixture archives this project authors itself, never against untrusted third-party archives."
revisit_condition = "testcontainers upgrades its transitive tokio-tar dependency, or drops tokio-tar entirely"

[[exception]]
id = "RUSTSEC-2026-0187"
class = "vulnerability"
crate = "lopdf"
path = "lopdf -> pdf-extract -> paladin-content"
why_present = "Transitive dependency of pdf-extract, which crates/paladin-content/Cargo.toml:41 declares as an unconditional (non-optional) dependency."
why_not_fixable = "Fixing requires pdf-extract >= 0.12, a breaking upgrade that itself introduces a new, currently-unpatched ttf-parser advisory."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "always in the dependency graph whenever paladin-content builds, because pdf-extract is unconditional rather than gated by the crate's empty pdf = [] feature"
compensating_control = "Paladin does not feed attacker-controlled PDF input to lopdf in a default build; no first-party code path today accepts externally-supplied PDF bytes at runtime."
revisit_condition = "pdf-extract ships a release depending on lopdf >= 0.42 without introducing a new advisory"

[[exception]]
id = "RUSTSEC-2026-0194"
class = "vulnerability"
crate = "quick-xml"
path = "quick-xml -> rust-s3/aws-creds -> paladin-storage (optional s3 feature)"
why_present = "Transitive dependency pulled in only when the optional s3 feature enables rust-s3/aws-creds for MinIO/S3-compatible storage."
why_not_fixable = "No release of rust-s3 depends on quick-xml >= 0.41 yet; the vulnerable pre-0.41 instance is pinned transitively through aws-creds."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "optional s3 feature path only, via rust-s3/aws-creds in paladin-storage; absent from a default-feature build"
compensating_control = "The XML this path parses is response bodies returned by an S3-compatible endpoint this project's own configuration names, not third-party or attacker-supplied documents."
revisit_condition = "rust-s3 (or aws-creds) bumps its quick-xml dependency to >= 0.41"

[[exception]]
id = "RUSTSEC-2026-0195"
class = "vulnerability"
crate = "quick-xml"
path = "quick-xml -> rust-s3/aws-creds -> paladin-storage (optional s3 feature)"
why_present = "Transitive dependency pulled in only when the optional s3 feature enables rust-s3/aws-creds for MinIO/S3-compatible storage; same dependency edge as RUSTSEC-2026-0194."
why_not_fixable = "No release of rust-s3 depends on quick-xml >= 0.41 yet; the vulnerable pre-0.41 instance is pinned transitively through aws-creds."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "optional s3 feature path only, via rust-s3/aws-creds in paladin-storage; absent from a default-feature build"
compensating_control = "The namespace-allocation path only processes XML returned by a configured S3-compatible endpoint at a project-controlled hostname, never an attacker-supplied document."
revisit_condition = "rust-s3 (or aws-creds) bumps its quick-xml dependency to >= 0.41"

[[exception]]
id = "RUSTSEC-2026-0367"
class = "vulnerability"
crate = "zip"
path = "zip 3.0.0 -> utoipa-swagger-ui 9.0.2 [build-dependencies] -> paladin-web (Swagger UI bundle, `vendored` feature)"
why_present = "utoipa-swagger-ui's build script unpacks the Swagger UI dist bundle with zip at build time so paladin-web can serve the OpenAPI UI; zip is a build-dependency only (insecure preservation of SUID/SGID bits during extraction)."
why_not_fixable = "Upstream condition met 2026-11-04 but not taken in v0.11.1: utoipa-swagger-ui 10.0.2 depends on zip ^9.0 (build) and utoipa ^6.0.0 (crates.io, checked 2026-11-04); the tree is on utoipa 5.5.0 / utoipa-axum 0.2 / utoipa-swagger-ui 9.0.2, so the lift is a utoipa 6 migration (derive macros, utoipa-axum 0.3, the committed crates/paladin-web/openapi.json snapshot) deferred to the next minor release, not a build-dependency bump; a [patch] override cannot bridge the zip 3 -> 9 API gap."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the next milestone, whichever is earlier"
scope = "build time only: zip runs inside utoipa-swagger-ui's build script and is linked into no shipped binary; with the `vendored` feature the archive it opens is the fixed bundle compiled into utoipa-swagger-ui-vendored, never a download or user-supplied file"
compensating_control = "The build script never calls ZipArchive::extract or ZipStreamReader::extract, the APIs all four advisories concern; it walks entries with by_index + enclosed_name() and writes each file itself, and its only input is the crate's own embedded bundle, so no attacker-controlled archive, symlink or mode bit reaches the vulnerable code."
revisit_condition = "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in a minor release"

[[exception]]
id = "RUSTSEC-2026-0368"
class = "vulnerability"
crate = "zip"
path = "zip 3.0.0 -> utoipa-swagger-ui 9.0.2 [build-dependencies] -> paladin-web (Swagger UI bundle, `vendored` feature)"
why_present = "utoipa-swagger-ui's build script unpacks the Swagger UI dist bundle with zip at build time so paladin-web can serve the OpenAPI UI; zip is a build-dependency only (ZipArchive::extract creates symlinks whose targets point outside the destination directory)."
why_not_fixable = "Upstream condition met 2026-11-04 but not taken in v0.11.1: utoipa-swagger-ui 10.0.2 depends on zip ^9.0 (build) and utoipa ^6.0.0 (crates.io, checked 2026-11-04); the tree is on utoipa 5.5.0 / utoipa-axum 0.2 / utoipa-swagger-ui 9.0.2, so the lift is a utoipa 6 migration (derive macros, utoipa-axum 0.3, the committed crates/paladin-web/openapi.json snapshot) deferred to the next minor release, not a build-dependency bump; a [patch] override cannot bridge the zip 3 -> 9 API gap."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the next milestone, whichever is earlier"
scope = "build time only: zip runs inside utoipa-swagger-ui's build script and is linked into no shipped binary; with the `vendored` feature the archive it opens is the fixed bundle compiled into utoipa-swagger-ui-vendored, never a download or user-supplied file"
compensating_control = "The build script never calls ZipArchive::extract or ZipStreamReader::extract, the APIs all four advisories concern; it walks entries with by_index + enclosed_name() and writes each file itself, and its only input is the crate's own embedded bundle, so no attacker-controlled archive, symlink or mode bit reaches the vulnerable code."
revisit_condition = "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in a minor release"

[[exception]]
id = "RUSTSEC-2026-0369"
class = "vulnerability"
crate = "zip"
path = "zip 3.0.0 -> utoipa-swagger-ui 9.0.2 [build-dependencies] -> paladin-web (Swagger UI bundle, `vendored` feature)"
why_present = "utoipa-swagger-ui's build script unpacks the Swagger UI dist bundle with zip at build time so paladin-web can serve the OpenAPI UI; zip is a build-dependency only (extract applies Unix permissions through symlinks without confining the target)."
why_not_fixable = "Upstream condition met 2026-11-04 but not taken in v0.11.1: utoipa-swagger-ui 10.0.2 depends on zip ^9.0 (build) and utoipa ^6.0.0 (crates.io, checked 2026-11-04); the tree is on utoipa 5.5.0 / utoipa-axum 0.2 / utoipa-swagger-ui 9.0.2, so the lift is a utoipa 6 migration (derive macros, utoipa-axum 0.3, the committed crates/paladin-web/openapi.json snapshot) deferred to the next minor release, not a build-dependency bump; a [patch] override cannot bridge the zip 3 -> 9 API gap."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the next milestone, whichever is earlier"
scope = "build time only: zip runs inside utoipa-swagger-ui's build script and is linked into no shipped binary; with the `vendored` feature the archive it opens is the fixed bundle compiled into utoipa-swagger-ui-vendored, never a download or user-supplied file"
compensating_control = "The build script never calls ZipArchive::extract or ZipStreamReader::extract, the APIs all four advisories concern; it walks entries with by_index + enclosed_name() and writes each file itself, and its only input is the crate's own embedded bundle, so no attacker-controlled archive, symlink or mode bit reaches the vulnerable code."
revisit_condition = "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in a minor release"

[[exception]]
id = "RUSTSEC-2026-0429"
class = "vulnerability"
crate = "zip"
path = "zip 3.0.0 -> utoipa-swagger-ui 9.0.2 [build-dependencies] -> paladin-web (Swagger UI bundle, `vendored` feature)"
why_present = "utoipa-swagger-ui's build script unpacks the Swagger UI dist bundle with zip at build time so paladin-web can serve the OpenAPI UI; zip is a build-dependency only (the zip extraction API sizes a symlink-target buffer from the entry's declared size before reading it, so a crafted archive aborts the process)."
why_not_fixable = "Upstream condition met 2026-11-04 but not taken in v0.11.1: utoipa-swagger-ui 10.0.2 depends on zip ^9.0 (build) and utoipa ^6.0.0 (crates.io, checked 2026-11-04); the tree is on utoipa 5.5.0 / utoipa-axum 0.2 / utoipa-swagger-ui 9.0.2, so the lift is a utoipa 6 migration (derive macros, utoipa-axum 0.3, the committed crates/paladin-web/openapi.json snapshot) deferred to the next minor release, not a build-dependency bump; a [patch] override cannot bridge the zip 3 -> 9 API gap."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the next milestone, whichever is earlier"
scope = "build time only: zip runs inside utoipa-swagger-ui's build script and is linked into no shipped binary; with the `vendored` feature the archive it opens is the fixed bundle compiled into utoipa-swagger-ui-vendored, never a download or user-supplied file"
compensating_control = "The build script never calls ZipArchive::extract or ZipStreamReader::extract, the APIs all four advisories concern; it walks entries with by_index + enclosed_name() and writes each file itself, and its only input is the crate's own embedded bundle, so no attacker-controlled archive, symlink or mode bit reaches the vulnerable code."
revisit_condition = "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in a minor release"

[[exception]]
id = "RUSTSEC-2026-0457"
class = "vulnerability"
crate = "rmcp"
path = "rmcp 2.1.0 (exact pin) -> paladin-ai (Arsenal MCP client engine; also a dev-dependency for the test MCP server)"
why_present = "rmcp is the official Model Context Protocol SDK behind the Arsenal MCP client (stdio and Streamable HTTP transports). The advisory is in rmcp's OAuth client, which decides whether an authorization-server metadata URL is internal by its host string without resolving it."
why_not_fixable = "The fix is rmcp >= 3.7.0. The workspace pins rmcp to exactly 2.1.0, and 2.x to 3.x is a major-version migration of the Arsenal MCP client and its test server, not a lockfile bump; it is scheduled as the first security item of the v0.12.0 milestone (.planning/todos/pending/2026-11-18-security-advisory-upgrade.md)."
owner = "DF3NDR"
review_date = "2026-12-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "not compiled: the workspace enables rmcp features client, server, transport-child-process, transport-streamable-http-client, transport-streamable-http-client-reqwest, reqwest and transport-streamable-http-server only; the `auth` feature that builds the OAuth client is off in every feature combination"
compensating_control = "rmcp's `auth` feature is not enabled anywhere in the workspace and its `oauth2` dependency is absent from Cargo.lock (measured 2026-11-20 with `cargo tree -e features -i rmcp --all-features`), so the OAuth discovery code the advisory concerns is not in any Paladin build; remote MCP servers authenticate with operator-supplied bearer or custom headers instead."
revisit_condition = "rmcp is upgraded to >= 3.9.0 in v0.12.0, or the `auth` feature is enabled before that upgrade (which makes this exception invalid and must block the change)"

[[exception]]
id = "RUSTSEC-2026-0462"
class = "vulnerability"
crate = "rmcp"
path = "rmcp 2.1.0 (exact pin) -> paladin-ai (Arsenal MCP client engine, Streamable HTTP reqwest transport)"
why_present = "The Arsenal MCP client reaches hosted MCP servers through rmcp's Streamable HTTP reqwest client, which reads non-streaming `application/json` and non-2xx response bodies into memory with no size limit."
why_not_fixable = "The fix is rmcp >= 3.9.0. The workspace pins rmcp to exactly 2.1.0, and 2.x to 3.x is a major-version migration of the Arsenal MCP client and its test server, not a lockfile bump; it is scheduled as the first security item of the v0.12.0 milestone (.planning/todos/pending/2026-11-18-security-advisory-upgrade.md)."
owner = "DF3NDR"
review_date = "2026-12-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "any process that connects the Arsenal to a remote (Streamable HTTP) MCP server; stdio MCP servers and builds that configure no remote MCP server are not affected. The affected transport is compiled in and used, so this is a live exposure, not a dormant one"
compensating_control = "Partial only. Paladin connects solely to MCP server endpoints the operator names in configuration (`arsenal.mcp_servers`), on the CLI or in embedding code; no paladin-web route and no model output selects an MCP server URL (checked 2026-11-20), so the unbounded read needs an operator-configured server that is itself malicious or compromised. The impact is memory exhaustion of the Paladin process (denial of service), not data exposure. No size cap is applied on Paladin's side; the upgrade is the fix."
revisit_condition = "rmcp is upgraded to >= 3.9.0 in v0.12.0; if that migration slips past the review date, a response-size guard on the MCP HTTP client is added instead of extending this exception"

[[exception]]
id = "RUSTSEC-2026-0459"
class = "vulnerability"
crate = "libsqlite3-sys"
path = "libsqlite3-sys 0.30.1 (bundled SQLite) -> sqlx-sqlite 0.8.6 -> sqlx 0.8 -> paladin-memory, paladin-storage, paladin-ai"
why_present = "sqlx's sqlite backend is the persistence engine for the Garrison (paladin-memory) and the repositories (paladin-storage); it builds SQLite from libsqlite3-sys's bundled source, which is older than the patched SQLite 3.54.0."
why_not_fixable = "The fix is libsqlite3-sys >= 0.39.0. sqlx-sqlite 0.8.x requires libsqlite3-sys ^0.30.1, so no 0.8 release reaches it; sqlx-sqlite 0.9.1 accepts < 0.41.0 (crates.io index, checked 2026-11-18), making the lift a sqlx 0.8 -> 0.9 minor-version migration across paladin-memory and paladin-storage, scheduled for the v0.12.0 milestone."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "processes that open a SQLite database file whose sibling `-wal` file a less trusted local party could have written; the advisory's vector is local and needs the victim process to open the crafted pair"
compensating_control = "Paladin opens only SQLite files at operator-configured paths (for example `garrison.path`) that Paladin itself creates and writes; no API, upload or model-driven path lets a caller supply a database or `-wal` file, so a crafted pair requires local write access to the operator's data directory, which already exposes the data directly."
revisit_condition = "sqlx is upgraded to 0.9.x with libsqlite3-sys >= 0.39.0 in v0.12.0"

[[exception]]
id = "RUSTSEC-2026-0545"
class = "vulnerability"
crate = "zip"
path = "zip 3.0.0 -> utoipa-swagger-ui 9.0.2 [build-dependencies] -> paladin-web (Swagger UI bundle, `vendored` feature)"
why_present = "utoipa-swagger-ui's build script unpacks the Swagger UI dist bundle with zip at build time so paladin-web can serve the OpenAPI UI; zip is a build-dependency only (the advisory is that WinZip AES entries can be returned without their authentication code being checked)."
why_not_fixable = "Same lift as RUSTSEC-2026-0367/-0368/-0369/-0429: utoipa-swagger-ui 10.0.2 depends on zip ^9.0 (build) and utoipa ^6.0.0, so reaching zip >= 9.0.0 is the utoipa 6 migration, scheduled for the v0.12.0 milestone, not a build-dependency bump."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "build time only, and not compiled: zip runs inside utoipa-swagger-ui's build script and is linked into no shipped binary, and its `aes-crypto` feature is off"
compensating_control = "zip's `aes-crypto` feature is not enabled (resolved features are the deflate family only, measured 2026-11-20 with `cargo tree -e features -i zip@3.0.0 --all-features`), so the AES decryption path the advisory concerns is not built; the build script opens only the crate's own embedded, unencrypted bundle."
revisit_condition = "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in v0.12.0"

[[exception]]
id = "RUSTSEC-2026-0548"
class = "vulnerability"
crate = "zip"
path = "zip 3.0.0 -> utoipa-swagger-ui 9.0.2 [build-dependencies] -> paladin-web (Swagger UI bundle, `vendored` feature)"
why_present = "utoipa-swagger-ui's build script unpacks the Swagger UI dist bundle with zip at build time so paladin-web can serve the OpenAPI UI; zip is a build-dependency only (the advisory is that ZipStreamReader extracts entries under local-header names that can differ from the central directory)."
why_not_fixable = "Same lift as RUSTSEC-2026-0367/-0368/-0369/-0429: utoipa-swagger-ui 10.0.2 depends on zip ^9.0 (build) and utoipa ^6.0.0, so reaching zip >= 9.0.5 is the utoipa 6 migration, scheduled for the v0.12.0 milestone, not a build-dependency bump."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "build time only: zip runs inside utoipa-swagger-ui's build script and is linked into no shipped binary; with the `vendored` feature the archive it opens is the fixed bundle compiled into utoipa-swagger-ui-vendored, never a download or user-supplied file"
compensating_control = "The 9.0.2 build script never uses ZipStreamReader or read_zipfile_from_stream, the APIs the advisory concerns (re-read 2026-11-20): it opens the bundle with ZipArchive and walks entries with by_index + enclosed_name(), which uses central-directory names, and its only input is the crate's own embedded bundle."
revisit_condition = "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in v0.12.0"

[[exception]]
id = "RUSTSEC-2026-0249"
class = "unmaintained"
crate = "smartstring"
path = "smartstring -> rxml -> minidom -> rust-s3 (optional s3 feature)"
why_present = "Transitive dependency pulled in only when the optional s3 feature enables rust-s3 for MinIO/S3-compatible storage, reached via the rxml/minidom XML stack rust-s3 depends on -- one hop further than RUSTSEC-2026-0194/-0195's quick-xml edge."
why_not_fixable = "rust-s3 is this project's S3/MinIO storage adapter and is not removable; no release of the rxml/minidom chain has dropped its smartstring dependency."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "optional s3 feature path only, via rust-s3 in paladin-storage; absent from a default-feature build"
compensating_control = "smartstring is rxml's internal string-interning dependency, not itself a parser of third-party input, and this advisory is an unmaintained notice rather than a known vulnerability, so there is no exploitable condition to compensate for today."
revisit_condition = "rxml (or minidom, or rust-s3) drops its smartstring dependency, or the advisory is upgraded from unmaintained to a vulnerability class, whichever comes first"

[[exception]]
id = "RUSTSEC-2021-0141"
class = "unmaintained"
crate = "dotenv"
path = "dotenv -> workspace crates (config loading)"
why_present = "dotenv is used for local .env file loading during development and testing across the workspace."
why_not_fixable = "dotenv itself is unmaintained upstream with no further releases; the maintained drop-in replacement is dotenvy, which this project has not yet migrated to."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "workspace-wide dependency used for local development .env file loading; not a network- or request-facing path"
compensating_control = "dotenv only parses local .env files a developer controls on their own machine; it is never used to parse untrusted or network-supplied input anywhere in this codebase."
revisit_condition = "the project migrates from dotenv to the maintained dotenvy replacement"

[[exception]]
id = "RUSTSEC-2025-0057"
class = "unmaintained"
crate = "fxhash"
path = "fxhash -> selectors/scraper -> paladin-content (optional web-scraping feature)"
why_present = "Transitive dependency of selectors, pulled in via scraper only when the optional web-scraping feature is enabled."
why_not_fixable = "fxhash is unmaintained upstream with no further releases; scraper has not migrated its selectors dependency away from it."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "optional web-scraping feature path only, via scraper/selectors in paladin-content"
compensating_control = "fxhash is used only for internal hashmap hashing inside selectors' CSS-selector matching engine, with no cryptographic role and no attacker-controlled key material."
revisit_condition = "scraper upgrades its selectors dependency to a version that drops fxhash"

[[exception]]
id = "RUSTSEC-2025-0119"
class = "unmaintained"
crate = "number_prefix"
path = "number_prefix -> indicatif -> workspace crates (progress-bar formatting)"
why_present = "Transitive dependency of indicatif, used for CLI progress-bar byte-size formatting."
why_not_fixable = "number_prefix is unmaintained upstream; indicatif has not dropped the dependency in a released version."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "CLI progress-bar display formatting only; never processes network or file input"
compensating_control = "number_prefix only formats already-known numeric byte counts for terminal display; it never parses external, untrusted, or attacker-supplied input of any kind."
revisit_condition = "indicatif ships a release that drops its number_prefix dependency"

[[exception]]
id = "RUSTSEC-2025-0134"
class = "unmaintained"
crate = "rustls-pemfile"
path = "rustls-pemfile -> tonic/testcontainers -> workspace crates"
why_present = "Transitive dependency pulled in via tonic and testcontainers for PEM certificate parsing in test and development contexts."
why_not_fixable = "rustls-pemfile is unmaintained upstream at the pinned version; neither tonic nor testcontainers has released an upgrade past it."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "dev/test certificate-handling path via tonic and testcontainers; not exercised by production TLS termination"
compensating_control = "The affected PEM parsing path runs only inside dev/test certificate fixtures generated by this project's own test harness, never against production TLS material at runtime."
revisit_condition = "tonic or testcontainers upgrades its transitive rustls-pemfile dependency"

[[exception]]
id = "RUSTSEC-2024-0436"
class = "unmaintained"
crate = "paste"
path = "paste -> utoipa -> paladin-web (OpenAPI generation)"
why_present = "Transitive dependency of utoipa, used for OpenAPI schema derive macros in paladin-web's Epic 6 API-documentation generation."
why_not_fixable = "paste is unmaintained upstream with no drop-in replacement utoipa has adopted in a released version."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "paladin-web's compile-time OpenAPI schema generation via utoipa's derive macros"
compensating_control = "paste is a compile-time proc-macro token-pasting helper with no runtime behaviour whatsoever; it cannot process any request-time or attacker-supplied input."
revisit_condition = "utoipa releases a version that no longer depends on paste"

[[exception]]
id = "RUSTSEC-2024-0449"
class = "unmaintained"
crate = "unsafe-libyaml"
path = "unsafe-libyaml -> serde_yaml 0.9 -> paladin-eval (scenario and graph-document YAML loaders) and the facade's optional cli feature (config loader, Paladin/Battalion templates, graph commands)"
why_present = "serde_yaml 0.9 is the YAML (de)serializer behind paladin-eval's scenario and graph-document loaders and behind the facade's optional cli feature (config loader, Paladin and Battalion templates, graph commands); unsafe-libyaml is serde_yaml's only parser backend."
why_not_fixable = "unsafe-libyaml's repository was archived on 2024-03-25 and serde_yaml 0.9.34 is itself published as deprecated, so there is no upgrade that drops the crate; the published serde_yaml forks carry the same c2rust-translated libyaml under other names. The advisory's own remedy is a different, safe-Rust parser (serde-saphyr), which is a migration of 53 call sites across 15 files rather than a lockfile bump; this exception defers that migration, it does not hide it."
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "YAML read from the local filesystem or from string literals by the CLI (off by default) and by the eval runner: operator-authored scenario, graph-document, config and template files; no network- or request-facing path in paladin-web or any LLM adapter parses YAML"
compensating_control = "Every serde_yaml call site parses a file path the operator names on the command line, a file in the checked-in eval corpus, or a literal; paladin-web and the LLM adapters never deserialise YAML, so no attacker-supplied bytes reach the parser, and the cli feature that pulls serde_yaml into the facade is not in the default feature set."
revisit_condition = "the workspace migrates serde_yaml to a safe-Rust YAML parser (serde-saphyr), or a maintained serde_yaml successor that does not depend on unsafe-libyaml is released"

[[exception]]
id = "RUSTSEC-2026-0543"
class = "unmaintained"
crate = "chrono"
path = "chrono 0.4 -> every workspace crate (workspace dependency) and transitively via sqlx, utoipa and others"
why_present = "chrono is the workspace's date-time type: Entity timestamps in paladin-core, serde formats, sqlx column mapping and the OpenAPI schema all use it."
why_not_fixable = "The advisory marks every chrono version unmaintained and lists no patched release; the remedy is a migration to jiff or time across every crate and the sqlx/utoipa integrations, which is a decision for the v0.12.0 milestone (.planning/todos/pending/2026-11-18-security-advisory-upgrade.md), not a bump."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "all builds: chrono is a direct dependency of every workspace crate"
compensating_control = "This advisory is an unmaintained notice, not a known vulnerability, so there is no exploitable condition to compensate for today; cargo-audit and cargo-deny keep scanning chrono for vulnerability-class advisories, which this exception does not suppress."
revisit_condition = "the v0.12.0 milestone decides between a governed long-term exception and a migration to jiff or time, or the advisory is upgraded from unmaintained to a vulnerability class, whichever comes first"

[[exception]]
id = "RUSTSEC-2026-0555"
class = "unmaintained"
crate = "chrono-tz"
path = "chrono-tz 0.10 -> paladin-storage -> paladin-ai, paladin-eval"
why_present = "paladin-storage uses chrono-tz for IANA time-zone handling alongside chrono."
why_not_fixable = "The advisory marks every chrono-tz version unmaintained and lists no patched release; replacing it follows the chrono decision (jiff carries its own time-zone database), scheduled for the v0.12.0 milestone."
owner = "DF3NDR"
review_date = "2027-03-31 or the first planning day of the v0.12.0 milestone, whichever is earlier"
scope = "builds that include paladin-storage"
compensating_control = "This advisory is an unmaintained notice, not a known vulnerability, so there is no exploitable condition to compensate for today; time-zone names reach chrono-tz from operator configuration and stored records, not from an unauthenticated request path."
revisit_condition = "the v0.12.0 milestone's chrono decision is taken, or the advisory is upgraded from unmaintained to a vulnerability class, whichever comes first"
```
<!-- END MACHINE-READABLE REGISTER -->

## What this register deliberately does not carry

**Four `deny.toml` history entries are absent from this register on purpose, not by oversight.**
`RUSTSEC-2022-0104` (`structopt`), `RUSTSEC-2021-0139` (`ansi_term`), `RUSTSEC-2024-0375` (`atty`)
and `RUSTSEC-2024-0370` (`proc-macro-error`) all justify their `deny.toml` entry as "via structopt"
or "via clap 2.x/structopt". Phase 8's clap v4 migration (`ADR-0023`) removed `structopt` from the
manifest entirely, and re-running `grep -c '^name = "<crate>"$' Cargo.lock` for all four this
session returns **0** for each — their parent crate is gone, so none of the four is reachable under
any feature combination. Attaching an owner and a review date to a suppression that suppresses
nothing is governance theatre: it presents a dead entry as governed live risk. These four are
deleted from `deny.toml` by this phase's plan 09-06, not backfilled here, and
`scripts/check-advisory-register.sh`'s `Cargo.lock` liveness clause makes this state permanently
unreachable going forward — a stale row here with a since-removed crate now fails the guard instead
of silently accumulating the way it did before this register existed.

**A note on what this register does not resolve.** `RUSTSEC-2026-0187`'s reachability above rests
on `crates/paladin-content/Cargo.toml:41` declaring `pdf-extract` unconditionally, while `:18`'s
`pdf = []` feature is empty and gates nothing — a live contradiction between a dependency that is
always compiled and a capability flag that enables nothing. This register treats that contradiction
as settled evidence that `lopdf` is reachable in the graph (so the suppression above is warranted
regardless of how the underlying capability question is answered), but it does not resolve the
contradiction itself. Whether PDF extraction is a supported capability of this project at all is
Phase 10 / HARD-06's subject — handed over here as a `file:line` finding, not answered.
