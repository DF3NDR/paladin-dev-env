# Phase 56 deferred items

Out-of-scope discoveries logged by executors; not fixed in the plan that found them.

## 56-02: `make audit` fails on RUSTSEC-2026-0429 (zip 3.0.0)

- Found by: `make security` run at the end of plan 56-02 (2026-11-02). `Cargo.lock` is byte-identical
  to the plan-start commit, so the plan did not introduce it; the RustSec advisory was published
  2026-10-22 and is newly visible in the audit database.
- Advisory: `zip` 3.0.0, "ZipArchive::extract aborts the process on a symlink entry with a large
  declared size", severity 7.5, solution `>=9.0.0`.
- Path: `zip` is a build-dependency of `utoipa-swagger-ui` 9.0.2, pulled in by `paladin-web`
  (build time only; the crate does not extract untrusted archives at runtime).
- Needs: an owner decision (bump `utoipa-swagger-ui`, or an explicit `.cargo/audit.toml` allowlist
  entry with a reason, per `.github/instructions/security.instructions.md`). Not fixed here.
