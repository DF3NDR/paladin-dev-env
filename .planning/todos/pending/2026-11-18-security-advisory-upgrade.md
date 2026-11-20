---
created: 2026-11-18T00:00:00Z
title: "`make security` is red: upgrade rmcp, sqlx, utoipa (zip) and disposition chrono"
area: security
severity: blocker
files:
  - Cargo.toml
  - Cargo.lock
  - crates/paladin-web/Cargo.toml
  - crates/paladin-storage/Cargo.toml
  - crates/paladin-memory/Cargo.toml
  - .cargo/audit.toml
  - deny.toml
  - SECURITY-EXCEPTIONS.md
owner: v0.12.0 milestone
dispositioned_by: quick task 261117-t5c follow-up (owner decision 2026-11-18, security upgrade is part of v0.12.0)
---

## Problem

`make security` fails as of 2026-11-18 (measured with `cargo audit` on branch
`claude/stoic-wright-60wtxw`). None of it was introduced by the z.ai work. The `lettre` advisories
(RUSTSEC-2026-0560 to -0565) were fixed on that branch by a lockfile bump to 0.11.24. The owner
first chose not to add interim exceptions. That changed on 2026-11-20: `protect-main-branch.json` has
no bypass and both checks are required, so nothing could merge. Seven governed exceptions were added
(`SECURITY-EXCEPTIONS.md`, `.cargo/audit.toml`, `deny.toml`) and `make security` is green again. They
are interim: every step below must also delete the exception rows it makes obsolete.

| Crate | Advisories | Enters via | Fixed in |
|---|---|---|---|
| `rmcp` 2.1.0 | RUSTSEC-2026-0457 (OAuth client SSRF filter checks host strings, not resolved addresses), RUSTSEC-2026-0462 (unbounded response body buffering in Streamable HTTP clients) | root crate, exact pin `=2.1.0` (Arsenal MCP client) | >= 3.9.0 (latest seen 3.11.0) |
| `libsqlite3-sys` 0.30.1 | RUSTSEC-2026-0459 (C SQLite CVE) | `sqlx` 0.8.6 (`sqlx-sqlite` caps it at ^0.30.1) | >= 0.39.0; `sqlx-sqlite` 0.9.1 accepts `<0.41.0` |
| `zip` 3.0.0 | RUSTSEC-2026-0545 (AES entries returned without checking the authentication code), RUSTSEC-2026-0548 (`ZipStreamReader` local-header names) | build-dependency of `utoipa-swagger-ui` 9 (paladin-web) | >= 9.0.5; `utoipa-swagger-ui` 10.0.2 needs `utoipa` ^6 |
| `chrono` 0.4.44, `chrono-tz` 0.10.4 | RUSTSEC-2026-0543, RUSTSEC-2026-0555 (unmaintained; fails `cargo-deny` only) | whole workspace | no patched version |

## What to do

1. Migrate `rmcp` 2.1.0 to >= 3.9.0. Both advisories are in code paths Paladin uses (Streamable
   HTTP client with auth), so this is the highest-priority item.
2. Bump `sqlx` 0.8 to 0.9.1 and confirm the lockfile resolves `libsqlite3-sys` >= 0.39.0.
3. Do the `utoipa` 5 to 6 and `utoipa-swagger-ui` 9 to 10 migration. That lifts `zip` to 9.x, which
   also retires the four existing `zip` 3.0.0 exceptions (RUSTSEC-2026-0367, -0368, -0369, -0429) in
   `.cargo/audit.toml`, `deny.toml` and `SECURITY-EXCEPTIONS.md`.
4. Decide `chrono` / `chrono-tz`: a governed unmaintained-crate exception (owner, review date,
   compensating control, mirrored in `deny.toml`) or a replacement date library.
5. `cargo audit` also warns (does not fail) on unsound `event-listener` 5.4.1 (RUSTSEC-2026-0221),
   `rustix` 1.1.4 (RUSTSEC-2026-0486) and `scc` 2.4.0 (RUSTSEC-2026-0205). Check whether lockfile
   bumps clear them.
6. Remove the interim exceptions as each fix lands: `RUSTSEC-2026-0457`, `-0462` (rmcp), `-0459`
   (libsqlite3-sys), `-0545`, `-0548` (zip), and `-0543`, `-0555` (chrono, chrono-tz) if step 4 chooses
   a migration. `RUSTSEC-2026-0462` is a live exposure with only a partial control, so it goes first.
7. Finish with `make security` green and `scripts/check-advisory-register.sh` passing.
