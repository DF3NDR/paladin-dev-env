//! Epoch-aligned tumbling windows for allowance periods (ALLOW-01, D-01).
//!
//! A window is a pure function of one instant and one period: it spans Unix-epoch seconds from
//! `floor(now / P) * P` (inclusive) to that plus `P` (exclusive). The caller supplies `now` from
//! the ledger's store clock (`TreasuryLedgerPort::store_now`) -- this module never reads a clock
//! itself, so every worker and web process derives the same boundaries from the one
//! authoritative clock.
//!
//! Windows tumble, they do not slide: a tenant can spend up to twice an allowance across one
//! boundary, by design.

use chrono::{DateTime, Utc};

/// The half-open window `[start, end)` of `period_secs` seconds, aligned to the Unix epoch, that
/// contains `now` (truncated to whole seconds).
///
/// Returns `None` for a zero period or when an instant would fall outside the representable
/// range -- never a panic.
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use paladin::application::services::treasurer::window_for;
///
/// let now = Utc.with_ymd_and_hms(2026, 10, 3, 13, 30, 0).single().ok_or("bad instant")?;
/// let (start, end) = window_for(now, 86_400).ok_or("no window")?;
/// assert_eq!(start, Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).single().ok_or("bad instant")?);
/// assert_eq!(end, Utc.with_ymd_and_hms(2026, 10, 4, 0, 0, 0).single().ok_or("bad instant")?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn window_for(now: DateTime<Utc>, period_secs: u64) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let period = i64::try_from(period_secs).ok().filter(|p| *p > 0)?;
    let start_secs = now.timestamp().div_euclid(period).checked_mul(period)?;
    let end_secs = start_secs.checked_add(period)?;
    Some((
        DateTime::from_timestamp(start_secs, 0)?,
        DateTime::from_timestamp(end_secs, 0)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 3, h, m, s)
            .single()
            .expect("valid instant")
    }

    #[test]
    fn window_contains_now_and_is_epoch_aligned() {
        let now = at(13, 30, 45);
        for period in [60_u64, 3_600, 86_400, 7 * 86_400] {
            let (start, end) = window_for(now, period).expect("window");
            assert!(start <= now && now < end, "{period}");
            assert_eq!(
                (end - start).num_seconds(),
                i64::try_from(period).unwrap(),
                "{period}"
            );
            assert_eq!(
                start.timestamp().rem_euclid(i64::try_from(period).unwrap()),
                0
            );
        }
        let (start, end) = window_for(now, 86_400).expect("window");
        assert_eq!(start, at(0, 0, 0));
        assert_eq!(
            end,
            Utc.with_ymd_and_hms(2026, 10, 4, 0, 0, 0).single().unwrap()
        );
    }

    #[test]
    fn instant_at_window_end_belongs_to_the_next_window() {
        let (_, end) = window_for(at(13, 0, 0), 3_600).expect("window");
        assert_eq!(end, at(14, 0, 0));
        let (next_start, next_end) = window_for(end, 3_600).expect("next window");
        assert_eq!(next_start, end);
        assert_eq!(next_end, at(15, 0, 0));
        // One second before the boundary is still in the first window.
        let (start, _) = window_for(at(13, 59, 59), 3_600).expect("window");
        assert_eq!(start, at(13, 0, 0));
    }

    #[test]
    fn zero_period_is_none() {
        assert_eq!(window_for(at(1, 2, 3), 0), None);
        assert_eq!(window_for(at(1, 2, 3), u64::MAX), None);
    }
}
