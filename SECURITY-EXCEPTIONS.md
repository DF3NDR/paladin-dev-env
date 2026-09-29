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
  `[advisories] ignore` array holds exactly the five `vulnerability`-class rows below, and nothing
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

## The eleven live suppressions

Five vulnerability-class advisories (mirrored exactly between `.cargo/audit.toml` and `deny.toml`)
and six unmaintained-class advisories (`deny.toml` only). This is the tree-verified live set, not
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
