# Phase 46 deferred items (out of scope for the plan that found them)

## Found during 46-05 (2026-10-24)

- `make security` / `cargo deny check advisories` fails on **RUSTSEC-2026-0386**
  (`tokio::io::unix::AsyncFd` safe constructors violate I/O safety; `unsound`
  advisory; fixed-in/deprecation starts at tokio 1.51.6 / 1.53.3 / 1.54.0). The
  advisory DB fetch from the sandbox works. `bans`, `licenses` and `sources` are
  ok. Plan 46-05 changed no `Cargo.toml` or `Cargo.lock`, so this is a pre-existing
  condition (a newly published advisory against the locked `tokio`), not caused by
  the plan. `cargo audit` also lists four unmaintained-crate warnings (`dotenv`
  RUSTSEC-2021-0141, `fxhash` RUSTSEC-2025-0057, `number_prefix`
  RUSTSEC-2025-0119, `paste` RUSTSEC-2024-0436), all warnings, not errors.
  Needs a dependency-bump or an explicit `.cargo/audit.toml` / `deny.toml`
  exception with a reason by the maintainer; CI's Security Audit job is the
  authority. Not recorded in `.planning/WINDOWS.md` so plan 46-05's
  "open unchanged" ledger assertion stays true.
