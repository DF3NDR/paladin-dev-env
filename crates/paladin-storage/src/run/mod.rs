//! Run storage adapters.
//!
//! Implementations of `paladin_ports::output::run_repository_port::RunRepositoryPort`,
//! mirroring `crate::waypoint`'s module layout (D-03).

use chrono::{DateTime, SubsecRound, Utc};

/// In-memory implementation, always available (no feature gate): used for
/// tests, local development, and this plan's tracer end-to-end test.
pub mod in_memory;

/// Shared `RunRepositoryPort` contract suite (D-04, D-17, D-18), mirroring
/// `crate::waypoint::contract_tests`. Plain module (not `#[cfg(test)]`) so
/// both unit tests inside each backend crate and future integration tests
/// can call it.
pub mod contract_tests;

/// SQLite implementation, behind the `sqlite` feature (D-03).
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// PostgreSQL implementation, behind the `postgres` feature (D-03, Tier 2).
#[cfg(feature = "postgres")]
pub mod postgres;

/// The sub-second precision the run store's persisted timestamps are
/// normalised to before crossing into PostgreSQL's `TIMESTAMPTZ` (six
/// digits: microseconds).
pub(crate) const STORAGE_TIMESTAMP_SUBSEC_DIGITS: u16 = 6;

/// The run store's persisted-timestamp precision contract.
///
/// PostgreSQL's `TIMESTAMPTZ` column type holds microsecond resolution, so a
/// `chrono::DateTime<Utc>` carrying nanosecond-precision sub-second digits
/// (as `Utc::now()` typically does) cannot survive an `insert`-then-`get`
/// round trip unchanged. Rather than let the driver or the server decide
/// whether the extra digits are rounded or truncated away -- a choice this
/// codebase does not control and should not depend on -- every timestamp
/// bound into a Postgres `TIMESTAMPTZ` column is normalised through this
/// function first, so the persisted value is always decided here.
///
/// Truncation is **toward zero**: a value 999 nanoseconds past a microsecond
/// boundary lands on that boundary, never rounds up to the next one.
/// `chrono::SubsecRound::round_subsecs` (the rounding twin of
/// `trunc_subsecs`) is deliberately not used -- rounding a timestamp forward
/// in time is a more surprising loss of information than truncating it, and
/// would make the persisted value depend on exactly where in its
/// sub-microsecond range the original value happened to fall.
///
/// The contract is "at least microsecond resolution on every backend,
/// exactly microsecond resolution on Postgres": SQLite (which stores
/// timestamps as RFC3339 text) and the in-memory adapter retain whatever
/// resolution the caller supplied, since neither backend's native storage
/// forces a narrower resolution the way Postgres's `TIMESTAMPTZ` does.
pub(crate) fn storage_timestamp(ts: DateTime<Utc>) -> DateTime<Utc> {
    ts.trunc_subsecs(STORAGE_TIMESTAMP_SUBSEC_DIGITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A value 999 nanoseconds past a microsecond boundary truncates down
    /// onto that boundary -- never up to the next one.
    #[test]
    fn storage_timestamp_truncates_sub_microsecond_digits_toward_zero() {
        let boundary = Utc::now().trunc_subsecs(STORAGE_TIMESTAMP_SUBSEC_DIGITS);
        let just_past_boundary = boundary + chrono::Duration::nanoseconds(999);

        let truncated = storage_timestamp(just_past_boundary);

        assert_eq!(
            truncated, boundary,
            "999ns past a microsecond boundary must truncate down onto it, not round up"
        );
    }

    /// A value already at microsecond resolution is unchanged by
    /// `storage_timestamp` (it is the identity at that resolution).
    #[test]
    fn storage_timestamp_is_identity_at_microsecond_resolution() {
        let already_microsecond_precision =
            Utc::now().trunc_subsecs(STORAGE_TIMESTAMP_SUBSEC_DIGITS);

        assert_eq!(
            storage_timestamp(already_microsecond_precision),
            already_microsecond_precision
        );
    }
}
