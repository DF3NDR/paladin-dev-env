//! Shared embedded SQL migrator for `paladin-memory` (D-17, D-23, RESEARCH Pitfall 4).
//!
//! This crate has **exactly one** `migrations/` directory
//! (`crates/paladin-memory/migrations/`) and **exactly one** embedded-migrator macro
//! invocation: [`MIGRATOR`]. `SqliteGarrison` (this plan, 26-07) and, from plan
//! 26-09, `SqliteVault` both call this one static rather than declaring their own,
//! because `sqlx` tracks applied migrations in a single `_sqlx_migrations` table
//! keyed by version and checksum -- a second `migrations/` directory reusing a
//! version number could not be reconciled against the first. Numbering (`001`,
//! `002`, `003`, ...) is therefore a single shared sequence across every table this
//! crate persists, regardless of which adapter owns that table.
//!
//! Migrations are embedded in the compiled binary at build time (the macro reads
//! the directory during compilation, not at runtime), so a deployment no longer
//! depends on a `migrations/` directory being present relative to the process's
//! current working directory -- the fragile pattern this module replaces
//! (`sqlite_garrison.rs`'s prior `Migrator::new("./migrations")` runtime call).

use sqlx::SqlitePool;
use sqlx::migrate::MigrateError;

/// The crate's one embedded migrator, compiled from `crates/paladin-memory/migrations/`.
///
/// Running it against a fresh database creates every table this crate persists.
/// Running it against a database that already has some migrations recorded in its
/// `_sqlx_migrations` table applies only the ones still pending, and is safe to call
/// more than once against the same database -- `sqlx::migrate::Migrator` tracks
/// applied versions itself (idempotent construction, per the `SqliteWaypointStore`
/// precedent at `crates/paladin-storage/src/waypoint/sqlite.rs`).
///
/// The literal argument is `"./migrations"`, not `"migrations"`: `sqlx`'s macro
/// resolves either form identically relative to `CARGO_MANIFEST_DIR`, but rejects a
/// single-path-segment literal at compile time with "paths relative to the current
/// file's directory are not currently supported" (a footgun-avoidance check in
/// `sqlx-macros-core`, not an actual file-relative resolution difference).
pub(crate) static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Runs [`MIGRATOR`] against `pool`, applying any pending migrations.
///
/// # Errors
///
/// Returns [`MigrateError`] if a migration fails to apply -- for example, a
/// checksum mismatch against an already-applied migration (someone edited a
/// migration file after it shipped), or a SQL error in a pending one.
pub(crate) async fn run_migrations(pool: &SqlitePool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

#[cfg(test)]
mod tests {
    /// Recursively counts occurrences of `needle` across every `.rs` file under `dir`.
    fn count_matches(dir: &std::path::Path, needle: &str) -> usize {
        let mut count = 0;
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                count += count_matches(&path, needle);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let contents = std::fs::read_to_string(&path).expect("read source file");
                count += contents.matches(needle).count();
            }
        }
        count
    }

    /// Source-level guard for the crate's own invariant (D-17, D-23, RESEARCH Pitfall
    /// 4): exactly one embedded-migrator macro invocation exists in this crate, here
    /// in `migrations.rs`. A second call site would create a second, unreconcilable
    /// `_sqlx_migrations` tracking sequence.
    ///
    /// The search needle is assembled from parts at runtime rather than written as a
    /// literal substring in this file, so this guard's own source text is never
    /// counted as a second match.
    #[test]
    fn migrator_is_declared_once_in_the_crate() {
        let needle = ["sqlx", "::", "migrate", "!"].concat();
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let occurrences = count_matches(&src_dir, &needle);
        assert_eq!(
            occurrences, 1,
            "expected exactly one embedded-migrator macro invocation in the crate, found {occurrences}"
        );
    }
}
