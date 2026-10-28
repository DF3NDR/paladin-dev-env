# Phase 52 deferred items

Out-of-scope discoveries logged by plan executors (scope boundary: not caused by the plan's own changes,
not fixed in-plan). Plan 52-08 reads this file when it writes the handover.

## From plan 52-03 (CodeQL triage, 2026-10-28)

### D-52-03-01: News API key travels in the URL, and `reqwest` error text can echo the URL

- Where: `crates/paladin-content/src/adapters/input/news_api_fetcher.rs`, `build_news_api_url` (lines 64-72)
  appends `&apiKey={self.api_key}` to the request URL; `fetch_content_list` (line 138) sends it with
  `reqwest::blocking::Client::new()` and maps a send failure with
  `format!("Failed to fetch from News API: {}", e)` (line 146).
- Why it matters: `reqwest` 0.12.28's `Error` Display writes ` for url ({url})` when the error carries a URL
  (`reqwest-0.12.28/src/error.rs:268`), so a transport error string can contain the key. The client is also
  built with `Client::new()`, which follows redirects. `.github/instructions/security.instructions.md` asks for
  redaction before an error or log embeds a response/URL and for credential-carrying clients not to follow
  redirects. (Only the 0.12.28 source was checked; `Cargo.lock` also holds reqwest 0.13.4.)
- Not fixed here: CodeQL alert 54 itself (`rust/cleartext-transmission`) is a false positive because the URL
  scheme is a literal `https://`, and this plan's job was the register. The fix is a behaviour change to a
  third-party call (move the key to the `X-Api-Key` header, strip the URL from the mapped error, set
  `redirect::Policy::none()`) that needs its own test; it touches no public signature (`NewsApiFetcher` is
  `#[doc(hidden)]`). Registered row 54 names this item in its `why_dismissed` and `revisit_condition`.

### D-52-03-02: three advisory alerts with no governing exception

Open OSV-Scanner alerts with no row in `SECURITY-EXCEPTIONS.md` and no entry in `.cargo/audit.toml` or
`deny.toml`:

- alert 25, `RUSTSEC-2026-0221`: `event-listener` 5.4.1 in `Cargo.lock`, patched at >= 5.4.2, informational `unsound`.
- alert 24, `RUSTSEC-2026-0205`: `scc` 2.4.0 in `Cargo.lock`, patched at >= 3.8.4, informational `unsound`.
- alert 23, `GHSA-7gcf-g7xr-8hxj`: not identifiable offline (no record in the local RustSec database carries it).

An offline `cargo audit --no-fetch --db ~/.cargo/advisory-db` warns on the first two and exits without failing
on them, which is why CI has not flagged them. Fixing the first two is a `Cargo.lock` update
(`cargo update -p event-listener`, `cargo update -p scc`) that needs network access and a build, neither
available in this sandbox. Not dismissed and not registered: dismissing an ungoverned finding is the
maintainer's call.

### D-52-03-03: stale line anchors in two pre-existing register rows

`CODEQL-DISMISSALS.md` row 40 anchors `crates/paladin-memory/src/services/rag_retrieval_service.rs:106`, but the
alert now reports `:360` (the `log::debug!` in `RagRetrievalService::retrieve_context`); row 49 anchors
`src/bin/paladin-server.rs:387`, but the alert reports `:472` (the `info!` in `build_auth_config`). The
prose of both rows names the right statement and the guard only checks that the file exists, so nothing is
red. D-14 requires the six earlier rows to stay byte for byte, so they were not edited. A later sweep that is
allowed to touch them should refresh the anchors.
