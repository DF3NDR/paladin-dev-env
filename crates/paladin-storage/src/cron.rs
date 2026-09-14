/*
Cron Parsing for Run Schedules (D-38)

`parse_run_cron` accepts BOTH the standard 5-field crontab form
(`minute hour day month weekday`) and the 6-field form with seconds
(`sec minute hour day month weekday`) that `crates/paladin-storage/src/scheduler.rs`'s
`TokioCronSchedulerAdapter` already requires -- `croner`'s own
`.with_seconds_optional()` is what makes both forms parse through the SAME
`Cron` value. Timezone defaults to UTC (`RunSchedule::timezone` is never
empty by construction, see `run_schedule.rs`); an IANA name parses through
`chrono_tz::Tz: FromStr`.

`cron_field_count` is the ONE shared implementation `scheduler.rs`'s
`validate_cron_field_count` now delegates to (D-38) -- this module's own
field-count check accepts 5 OR 6, while `scheduler.rs`'s caller still
requires exactly 6 for its own (unchanged, X-03) stricter contract. Sharing
the counting primitive, not the acceptance predicate, is what keeps that
adapter's existing tests passing unchanged.

Every `croner`/`chrono_tz` error is wrapped into this module's own typed
`CronParseError` (X-06) at the boundary -- `croner::errors::CronError` is a
plain `std::error::Error`, not a `paladin-storage` type, so callers never see
it directly.
*/

use std::str::FromStr;

use chrono::{DateTime, Utc};
use croner::Cron;
use thiserror::Error;

/// The number of whitespace-separated fields the DEFAULT (unmodified)
/// `tokio-cron-scheduler`-backed [`crate::scheduler::TokioCronSchedulerAdapter`]
/// requires: `sec min hour day month weekday`. Re-exported here so
/// `scheduler.rs`'s `validate_cron_field_count` can keep its own six-field
/// requirement while sharing this module's counting primitive (D-38, X-03 --
/// the adapter's contract itself is unchanged).
pub const CRON_FIELD_COUNT: usize = 6;

/// Count the whitespace-separated fields in a cron expression. Shared,
/// unit-testable free function -- `scheduler.rs`'s `validate_cron_field_count`
/// delegates to this rather than re-deriving `split_whitespace().count()`
/// (D-38).
pub fn cron_field_count(expr: &str) -> usize {
    expr.split_whitespace().count()
}

/// Errors [`parse_run_cron`]/[`RunCron::next_after`] can return (X-06 --
/// structured, never a bare `String`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CronParseError {
    /// The cron expression did not have 5 (standard crontab) or 6 (with
    /// leading seconds) whitespace-separated fields.
    #[error(
        "cron expression must have 5 (minute hour day month weekday) or 6 \
         (sec minute hour day month weekday) fields, found {found}"
    )]
    FieldCount {
        /// The number of fields actually found.
        found: usize,
    },
    /// The timezone name was not a recognized IANA timezone.
    #[error("unknown IANA timezone: {name:?}")]
    UnknownTimezone {
        /// The rejected timezone name.
        name: String,
    },
    /// The cron expression's fields did not parse (an out-of-range value, an
    /// illegal character, or another `croner`-reported structural problem).
    #[error("invalid cron expression: {message}")]
    Invalid {
        /// `croner`'s own error message.
        message: String,
    },
    /// `croner` could not find a next occurrence within its own search
    /// bound (an unsatisfiable pattern, e.g. `31 2 *` in a month with no
    /// 31st combined with a day-of-week that never lands on it).
    #[error("no next occurrence exists for this cron expression")]
    NoNextOccurrence,
}

/// A parsed, timezone-aware run cron schedule (D-38).
///
/// Construct through [`parse_run_cron`]; compute the next fire time in UTC
/// through [`RunCron::next_after`].
#[derive(Debug)]
pub struct RunCron {
    cron: Cron,
    tz: chrono_tz::Tz,
}

impl RunCron {
    /// Compute the next occurrence strictly after `after` (exclusive --
    /// matches `find_next_occurrence`'s `inclusive = false`), in the
    /// schedule's own timezone, converted back to UTC.
    ///
    /// # Errors
    ///
    /// Returns [`CronParseError::NoNextOccurrence`] if `croner` cannot find
    /// one within its own search bound, or [`CronParseError::Invalid`] for
    /// any other `croner`-reported failure evaluating the pattern.
    pub fn next_after(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, CronParseError> {
        let after_in_tz = after.with_timezone(&self.tz);
        let next_in_tz = self
            .cron
            .find_next_occurrence(&after_in_tz, false)
            .map_err(|e| match e {
                croner::errors::CronError::TimeSearchLimitExceeded => {
                    CronParseError::NoNextOccurrence
                }
                other => CronParseError::Invalid {
                    message: other.to_string(),
                },
            })?;
        Ok(next_in_tz.with_timezone(&Utc))
    }
}

/// Parse a run-schedule cron expression and IANA timezone name into a
/// [`RunCron`] (D-38).
///
/// Accepts both the standard 5-field crontab form (`minute hour day month
/// weekday`) and the 6-field form with leading seconds (`sec minute hour day
/// month weekday`) -- `croner`'s own `.with_seconds_optional()` parses both
/// through the same value. `timezone` is any string `chrono_tz::Tz` parses
/// (e.g. `"UTC"`, `"Europe/Berlin"`); `RunSchedule::timezone` defaults to
/// `"UTC"` so this is never called with an empty string in practice.
///
/// # Errors
///
/// Returns [`CronParseError::FieldCount`] if `expr` does not have 5 or 6
/// whitespace-separated fields, [`CronParseError::UnknownTimezone`] if
/// `timezone` is not a recognized IANA name, or [`CronParseError::Invalid`]
/// if `croner` itself rejects the expression (an out-of-range field value or
/// an illegal character).
pub fn parse_run_cron(expr: &str, timezone: &str) -> Result<RunCron, CronParseError> {
    let found = cron_field_count(expr);
    if found != 5 && found != 6 {
        return Err(CronParseError::FieldCount { found });
    }

    let tz = chrono_tz::Tz::from_str(timezone).map_err(|_| CronParseError::UnknownTimezone {
        name: timezone.to_string(),
    })?;

    let cron = Cron::new(expr).with_seconds_optional().parse().map_err(
        |e: croner::errors::CronError| CronParseError::Invalid {
            message: e.to_string(),
        },
    )?;

    Ok(RunCron { cron, tz })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn cron_field_count_counts_whitespace_separated_fields() {
        assert_eq!(cron_field_count("* * * * *"), 5);
        assert_eq!(cron_field_count("0 * * * * *"), 6);
        assert_eq!(cron_field_count("* * *"), 3);
    }

    #[test]
    fn cron_field_count_ignores_irregular_whitespace() {
        assert_eq!(cron_field_count("0\t0   9 * * *"), 6);
    }

    #[test]
    fn parse_run_cron_accepts_five_field_form() {
        assert!(parse_run_cron("*/5 * * * *", "UTC").is_ok());
    }

    #[test]
    fn parse_run_cron_accepts_six_field_form() {
        assert!(parse_run_cron("0 */5 * * * *", "UTC").is_ok());
    }

    #[test]
    fn five_and_six_field_forms_yield_the_same_next_instants() {
        let five = parse_run_cron("*/5 * * * *", "UTC").unwrap();
        let six = parse_run_cron("0 */5 * * * *", "UTC").unwrap();
        let now = Utc::now();
        assert_eq!(five.next_after(now).unwrap(), six.next_after(now).unwrap());
    }

    #[test]
    fn parse_run_cron_accepts_iana_timezone() {
        assert!(parse_run_cron("0 9 * * *", "Europe/Berlin").is_ok());
    }

    #[test]
    fn parse_run_cron_rejects_unknown_timezone() {
        let err = parse_run_cron("0 9 * * *", "Mars/Olympus").unwrap_err();
        match err {
            CronParseError::UnknownTimezone { name } => assert_eq!(name, "Mars/Olympus"),
            other => panic!("expected UnknownTimezone, got {other:?}"),
        }
    }

    #[test]
    fn parse_run_cron_rejects_wrong_field_count() {
        let err = parse_run_cron("* * *", "UTC").unwrap_err();
        match err {
            CronParseError::FieldCount { found } => assert_eq!(found, 3),
            other => panic!("expected FieldCount, got {other:?}"),
        }
    }

    #[test]
    fn parse_run_cron_rejects_out_of_range_field() {
        let err = parse_run_cron("61 * * * *", "UTC").unwrap_err();
        assert!(matches!(err, CronParseError::Invalid { .. }));
    }

    #[test]
    fn next_after_advances_from_a_known_instant() {
        let cron = parse_run_cron("0 0 * * *", "UTC").unwrap(); // daily at midnight
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let next = cron.next_after(start).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap());
    }

    #[test]
    fn next_after_is_timezone_aware() {
        // 09:00 Europe/Berlin (UTC+1 in January) is 08:00 UTC.
        let cron = parse_run_cron("0 9 * * *", "Europe/Berlin").unwrap();
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let next = cron.next_after(start).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 1, 1, 8, 0, 0).unwrap());
    }
}
