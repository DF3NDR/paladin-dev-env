//! # Provider rate-limit response headers -> typed hints (PACE-01)
//!
//! One shared, pure, bounded parser that turns the headers of a provider's `429 Too Many
//! Requests` into a [`RateLimitHints`]: an explicit retry delay (`Retry-After` in both RFC 7231
//! forms, then the optional `retry-after-ms`) and, per quota dimension, the limit, the remaining
//! allowance and the time until it resets.
//!
//! ## Why it runs at the HTTP edge, before the body is consumed
//!
//! Every adapter reads its error body with `response.text().await`, which consumes the response
//! and with it the headers. The adapter therefore snapshots the headers **first** and passes the
//! parsed result down to [`crate::http_status::map_http_status_with_hints`]. This module names no
//! HTTP-client type: the caller hands in a lookup closure (`name -> Option<&str>`), so it is not
//! feature-gated and `--no-default-features` builds keep compiling.
//!
//! ## Precedence
//!
//! The delay the Cadence gate acts on is [`RateLimitHints::effective_retry_after`]:
//!
//! 1. `Retry-After` (delta-seconds, a non-negative decimal, or an HTTP-date);
//! 2. otherwise `retry-after-ms`;
//! 3. otherwise the reset of an *exhausted* dimension, tagged
//!    [`RetryDelaySource::ResetHeader`] so the decorator can bound it (a full-replenishment reset
//!    over-estimates the provider's real minimum).
//!
//! Remaining and reset values are carried for operators and for a future proactive-pacing phase;
//! they never close the gate on their own (D-04).
//!
//! ## Nothing raw leaves the parser (research Pitfall 16)
//!
//! Header values are provider- or gateway-controlled. Every value is parsed to an integer or a
//! [`Duration`], or dropped. No raw value is stored on [`RateLimitHints`] (it has no string field),
//! embedded in an error, or logged, so `Debug`-rendering the hints can never echo one.
//!
//! ## Hostile input (research Pitfall 3)
//!
//! Parsing is panic-free and bounded: values longer than 128 bytes are refused, integers use
//! checked `u64` parsing, decimals go through [`Duration::try_from_secs_f64`] (never the panicking
//! `from_secs_f64`), Go-style durations are limited to 64 bytes and 8 groups and use saturating
//! `u128` nanosecond arithmetic, and every duration that leaves the module is clamped to
//! [`CADENCE_DELAY_CEILING`] (24 h). The module holds no shared state, so concurrent 429s on
//! different tasks parse independently.
//!
//! # Examples
//!
//! ```
//! use std::time::{Duration, SystemTime};
//! use paladin_llm::rate_limit_headers::{hints_from_headers, RateLimitHeaderFamily};
//! use paladin_ports::output::rate_limit_hints::RetryDelaySource;
//!
//! let headers = [("retry-after", "7"), ("x-ratelimit-remaining-requests", "0")];
//! let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, SystemTime::now(), |name| {
//!     headers.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
//! })
//! .expect("a rate-limit header was present");
//! assert_eq!(
//!     hints.effective_retry_after(),
//!     Some((Duration::from_secs(7), RetryDelaySource::RetryAfter))
//! );
//! ```

use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use paladin_ports::output::cadence_port::CADENCE_DELAY_CEILING;
use paladin_ports::output::rate_limit_hints::{
    RateLimitDimension, RateLimitDimensionKind, RateLimitHints, RetryDelaySource,
};

/// The standard `Retry-After` response header (RFC 7231 section 7.1.3): either a non-negative
/// number of seconds or an HTTP-date.
///
/// Anthropic (`platform.claude.com/docs/en/api/rate-limits`, "Response headers" table, retrieved
/// 2026-10-07): "The number of seconds to wait until you can retry the request. Earlier retries
/// will fail." It is **not** sent with the spend-cap 429, which a retry cannot fix.
///
/// OpenAI: "minimum number of seconds to wait before retrying a temporary rate-limit error, when
/// present"; it can also arrive on a 503. Pending operator verification against the official
/// OpenAI rate-limits guide (plan 43-12 checkpoint).
pub const RETRY_AFTER: &str = "retry-after";

/// An optional millisecond-precision retry delay.
///
/// This is **not** an OpenAI-documented header. It is emitted by Azure OpenAI and read by
/// `openai-python` ahead of `retry-after`, so it is parsed as an optional extra for the
/// OpenAI-compatible family (research assumption A3) and used only when `Retry-After` is absent
/// or unusable.
pub const RETRY_AFTER_MS: &str = "retry-after-ms";

/// OpenAI: the maximum number of requests permitted before the limit is exhausted (example `60`).
/// Pending operator verification against the official OpenAI rate-limits guide (plan 43-12
/// checkpoint).
pub const OPENAI_LIMIT_REQUESTS: &str = "x-ratelimit-limit-requests";

/// OpenAI: the maximum number of tokens permitted before the limit is exhausted (example
/// `150000`). Pending operator verification against the official OpenAI rate-limits guide (plan
/// 43-12 checkpoint).
pub const OPENAI_LIMIT_TOKENS: &str = "x-ratelimit-limit-tokens";

/// OpenAI: the number of requests remaining in the window (example `59`). `0` marks the
/// dimension exhausted. Pending operator verification against the official OpenAI rate-limits
/// guide (plan 43-12 checkpoint).
pub const OPENAI_REMAINING_REQUESTS: &str = "x-ratelimit-remaining-requests";

/// OpenAI: the number of tokens remaining in the window (example `149984`). Pending operator
/// verification against the official OpenAI rate-limits guide (plan 43-12 checkpoint).
pub const OPENAI_REMAINING_TOKENS: &str = "x-ratelimit-remaining-tokens";

/// OpenAI: the time until the request limit resets, as a Go-style duration string (`1s`, `6m0s`,
/// `6ms`) -- see [`parse_go_duration`]. Pending operator verification against the official OpenAI
/// rate-limits guide (plan 43-12 checkpoint).
pub const OPENAI_RESET_REQUESTS: &str = "x-ratelimit-reset-requests";

/// OpenAI: the time until the token limit resets, as a Go-style duration string (example `6m0s`).
/// Pending operator verification against the official OpenAI rate-limits guide (plan 43-12
/// checkpoint).
pub const OPENAI_RESET_TOKENS: &str = "x-ratelimit-reset-tokens";

/// The shared prefix of Anthropic's rate-limit headers:
/// `anthropic-ratelimit-{requests,tokens,input-tokens,output-tokens}-{limit,remaining,reset}`.
///
/// Source: `https://platform.claude.com/docs/en/api/rate-limits`, "Response headers" table,
/// retrieved 2026-10-07. Each `-reset` is "the time when the ... rate limit will be fully
/// replenished, provided in RFC 3339 format". Because the API "uses the token bucket algorithm"
/// (capacity is replenished continuously), a `-reset` is the time of *full* replenishment -- an
/// over-estimate of when one more request is admitted -- so `retry-after` is the precise value and
/// wins. The `tokens` dimension reports the most restrictive token limit in effect, with
/// `-remaining` rounded to the nearest thousand. The spend-cap 429
/// (`error.details.error_code == "enforced_spend_limit_reached"`) carries no `retry-after`.
pub const ANTHROPIC_RATELIMIT_PREFIX: &str = "anthropic-ratelimit-";

/// The response `Date` header, the reference clock for HTTP-date delays and RFC 3339 resets.
const DATE: &str = "date";

/// The longest header value this module will look at. No legitimate delay, count or timestamp
/// approaches this; anything longer is hostile or corrupt and is dropped unparsed.
pub(crate) const MAX_VALUE_BYTES: usize = 128;

/// The longest Go-style duration string accepted (bounded work on hostile input).
const GO_DURATION_MAX_BYTES: usize = 64;

/// The most `<number><unit>` groups a Go-style duration may have.
const GO_DURATION_MAX_GROUPS: usize = 8;

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Which provider's rate-limit header vocabulary [`hints_from_headers`] reads.
///
/// # Examples
///
/// ```
/// use paladin_llm::rate_limit_headers::RateLimitHeaderFamily;
///
/// assert_ne!(RateLimitHeaderFamily::Generic, RateLimitHeaderFamily::OpenAi);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RateLimitHeaderFamily {
    /// Only the generic delay headers: [`RETRY_AFTER`] and [`RETRY_AFTER_MS`].
    Generic,
    /// The generic headers plus OpenAI's six `x-ratelimit-*` headers.
    OpenAi,
    /// The generic headers plus Anthropic's `anthropic-ratelimit-*` headers.
    Anthropic,
}

/// Clamp a parsed delay to the hard ceiling the Cadence port accepts.
fn clamp(delay: Duration) -> Duration {
    delay.min(CADENCE_DELAY_CEILING)
}

/// The clock a relative delay is measured from: the response `Date` header when it parses as an
/// HTTP-date, otherwise the caller's `now`.
fn reference_time(now: SystemTime, date_header: Option<&str>) -> SystemTime {
    date_header
        .map(str::trim)
        .filter(|v| !v.is_empty() && v.len() <= MAX_VALUE_BYTES)
        .and_then(|v| httpdate::parse_http_date(v).ok())
        .unwrap_or(now)
}

/// `true` for `digits`, `digits.digits`, `digits.` and `.digits` -- an unsigned plain decimal.
/// Signs, exponents, `NaN`, `inf` and non-ASCII digits are all rejected.
fn is_plain_decimal(value: &str) -> bool {
    let mut seen_dot = false;
    let mut digits = 0usize;
    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' => digits += 1,
            b'.' if !seen_dot => seen_dot = true,
            _ => return false,
        }
    }
    digits > 0
}

/// Parse an already-trimmed plain decimal as seconds. `None` on overflow.
fn plain_decimal_seconds(value: &str) -> Option<Duration> {
    if value.bytes().all(|b| b.is_ascii_digit()) {
        value.parse::<u64>().ok().map(Duration::from_secs)
    } else {
        value
            .parse::<f64>()
            .ok()
            .and_then(|secs| Duration::try_from_secs_f64(secs).ok())
    }
}

/// Parse an already-trimmed plain decimal as milliseconds. `None` on overflow.
fn plain_decimal_millis(value: &str) -> Option<Duration> {
    if value.bytes().all(|b| b.is_ascii_digit()) {
        value.parse::<u64>().ok().map(Duration::from_millis)
    } else {
        value
            .parse::<f64>()
            .ok()
            .and_then(|millis| Duration::try_from_secs_f64(millis / 1000.0).ok())
    }
}

/// Parse a `Retry-After` header value (RFC 7231 section 7.1.3) into a delay.
///
/// Accepted forms, after trimming surrounding whitespace:
///
/// - delta-seconds, an unsigned integer (`7`); one that overflows `u64` yields `None`;
/// - a non-negative plain decimal some gateways send (`1.5`), converted with
///   [`Duration::try_from_secs_f64`] so `NaN`, `inf` and `1e400` give `None`, never a panic;
/// - an HTTP-date in any of the three RFC 7231 forms (IMF-fixdate, RFC 850, asctime), measured
///   against `date_header` (the response `Date`) when that parses and otherwise against `now`; a
///   date at or before the reference is `Duration::ZERO`.
///
/// Anything else -- empty, signed, non-ASCII digits, an unparseable date -- is `None`. Every
/// `Some` is clamped to [`CADENCE_DELAY_CEILING`].
///
/// # Examples
///
/// ```
/// use std::time::{Duration, SystemTime};
/// use paladin_llm::rate_limit_headers::parse_retry_after;
///
/// let now = SystemTime::now();
/// assert_eq!(parse_retry_after(" 7 ", now, None), Some(Duration::from_secs(7)));
/// assert_eq!(parse_retry_after("-1", now, None), None);
/// assert_eq!(parse_retry_after("", now, None), None);
/// ```
pub fn parse_retry_after(
    value: &str,
    now: SystemTime,
    date_header: Option<&str>,
) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_VALUE_BYTES {
        return None;
    }
    if is_plain_decimal(value) {
        return plain_decimal_seconds(value).map(clamp);
    }
    let target = httpdate::parse_http_date(value).ok()?;
    let reference = reference_time(now, date_header);
    Some(clamp(
        target.duration_since(reference).unwrap_or(Duration::ZERO),
    ))
}

/// Parse a `retry-after-ms` value: an unsigned integer or plain decimal count of milliseconds.
fn parse_retry_after_ms(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_VALUE_BYTES || !is_plain_decimal(value) {
        return None;
    }
    plain_decimal_millis(value).map(clamp)
}

/// The nanoseconds in one of a Go duration's units, and the text after the unit. Longest unit
/// first, so `ms` is never read as `m` followed by a stray `s`.
fn take_go_unit(text: &str) -> Option<(u128, &str)> {
    const UNITS: [(&str, u128); 8] = [
        ("ms", 1_000_000),
        ("us", 1_000),
        // U+00B5 MICRO SIGN and U+03BC GREEK SMALL LETTER MU, the two spellings Go accepts.
        ("\u{00B5}s", 1_000),
        ("\u{03BC}s", 1_000),
        ("ns", 1),
        ("h", 3_600 * 1_000_000_000),
        ("m", 60 * 1_000_000_000),
        ("s", 1_000_000_000),
    ];
    UNITS
        .iter()
        .find_map(|(unit, nanos)| text.strip_prefix(unit).map(|rest| (*nanos, rest)))
}

/// Parse one `<non-negative decimal><unit>` group: its nanoseconds (saturating) and the rest.
fn take_go_group(text: &str) -> Option<(u128, &str)> {
    let int_len = text.bytes().take_while(u8::is_ascii_digit).count();
    let after_int = &text[int_len..];
    let (frac, after_number) = match after_int.strip_prefix('.') {
        Some(rest) => {
            let frac_len = rest.bytes().take_while(u8::is_ascii_digit).count();
            (&rest[..frac_len], &rest[frac_len..])
        }
        None => ("", after_int),
    };
    if int_len == 0 && frac.is_empty() {
        return None;
    }
    let (unit_nanos, rest) = take_go_unit(after_number)?;

    // An integer part too large for u128 saturates; the final clamp makes it the ceiling.
    let int_part: u128 = if int_len == 0 {
        0
    } else {
        text[..int_len].parse().unwrap_or(u128::MAX)
    };
    // Nine fractional digits are more than any unit can resolve; the rest are truncated.
    let frac = &frac[..frac.len().min(9)];
    let frac_part: u128 = if frac.is_empty() {
        0
    } else {
        frac.parse().unwrap_or(0)
    };
    let scale = 10u128.pow(frac.len() as u32);
    let nanos = int_part
        .saturating_mul(unit_nanos)
        .saturating_add(frac_part * unit_nanos / scale);
    Some((nanos, rest))
}

/// Parse a Go-style duration string, the form OpenAI uses for `x-ratelimit-reset-*`
/// (`6m0s` is 360 s, `1s`, `6ms`, `1h2m3s`, `1.5s`).
///
/// The grammar is one or more `<non-negative decimal><unit>` groups with units `h`, `m`, `s`,
/// `ms`, `us` (also spelled with a micro sign), `ns`. A bare number, an empty string, a sign, or any trailing
/// text is `None`. Work is bounded: at most 64 bytes and 8 groups, with saturating `u128`
/// nanosecond arithmetic, and the result is clamped to [`CADENCE_DELAY_CEILING`].
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_llm::rate_limit_headers::parse_go_duration;
///
/// assert_eq!(parse_go_duration("6m0s"), Some(Duration::from_secs(360)));
/// assert_eq!(parse_go_duration("1.5s"), Some(Duration::from_millis(1500)));
/// assert_eq!(parse_go_duration("-1s"), None);
/// assert_eq!(parse_go_duration("6m0s trailing"), None);
/// ```
pub fn parse_go_duration(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() || value.len() > GO_DURATION_MAX_BYTES {
        return None;
    }
    let mut rest = value;
    let mut total: u128 = 0;
    let mut groups = 0usize;
    while !rest.is_empty() {
        if groups == GO_DURATION_MAX_GROUPS {
            return None;
        }
        let (nanos, tail) = take_go_group(rest)?;
        total = total.saturating_add(nanos);
        rest = tail;
        groups += 1;
    }
    Some(duration_from_nanos_clamped(total))
}

/// A `Duration` from saturating nanoseconds, clamped to the ceiling.
fn duration_from_nanos_clamped(nanos: u128) -> Duration {
    let capped = nanos.min(CADENCE_DELAY_CEILING.as_nanos());
    // `capped` <= 86_400e9, so both casts are lossless.
    Duration::new(
        (capped / NANOS_PER_SEC) as u64,
        (capped % NANOS_PER_SEC) as u32,
    )
}

/// An RFC 3339 reset timestamp as the time from `reference` until it. A timestamp at or before the
/// reference is `Duration::ZERO`.
fn parse_rfc3339_reset(value: &str, reference: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_VALUE_BYTES {
        return None;
    }
    let target = DateTime::parse_from_rfc3339(value)
        .ok()?
        .with_timezone(&Utc);
    let reference = DateTime::<Utc>::from(reference);
    Some(clamp(
        target
            .signed_duration_since(reference)
            .to_std()
            .unwrap_or(Duration::ZERO),
    ))
}

/// An unsigned integer count (`60`, `0`). A negative or placeholder value (`-1`) is `None`.
fn parse_count(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_VALUE_BYTES
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

/// Build a dimension from whatever parsed; `None` when nothing did.
fn dimension(
    limit: Option<u64>,
    remaining: Option<u64>,
    reset_after: Option<Duration>,
) -> Option<RateLimitDimension> {
    (limit.is_some() || remaining.is_some() || reset_after.is_some())
        .then(|| RateLimitDimension::new(limit, remaining, reset_after))
}

/// Turn the headers of a provider response into typed [`RateLimitHints`].
///
/// `lookup` answers a lowercase header name with its value as `&str` -- typically
/// `|name| response.headers().get(name).and_then(|v| v.to_str().ok())`, which is
/// case-insensitive and yields `None` for a non-UTF-8 value. `now` is the local clock used when
/// the response carries no usable `Date` header.
///
/// Returns `None` when nothing usable was found, else the populated hints. The function is a
/// pure function of `(family, now, lookup)`: calling it twice on the same inputs yields equal
/// hints, and it holds no state, so concurrent callers are independent. See the module docs for
/// precedence and the hostile-input bounds.
///
/// # Examples
///
/// ```
/// use std::time::{Duration, SystemTime};
/// use paladin_llm::rate_limit_headers::{hints_from_headers, RateLimitHeaderFamily};
/// use paladin_ports::output::rate_limit_hints::{RateLimitDimensionKind, RetryDelaySource};
///
/// let headers = [
///     ("x-ratelimit-remaining-requests", "0"),
///     ("x-ratelimit-reset-requests", "6m0s"),
/// ];
/// let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, SystemTime::now(), |name| {
///     headers.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
/// })
/// .expect("hints");
/// assert_eq!(
///     hints.dimension(RateLimitDimensionKind::Requests).reset_after(),
///     Some(Duration::from_secs(360))
/// );
/// assert_eq!(
///     hints.effective_retry_after(),
///     Some((Duration::from_secs(360), RetryDelaySource::ResetHeader))
/// );
/// ```
pub fn hints_from_headers<'a>(
    family: RateLimitHeaderFamily,
    now: SystemTime,
    lookup: impl Fn(&str) -> Option<&'a str>,
) -> Option<RateLimitHints> {
    let date_header = lookup(DATE);
    let mut hints = RateLimitHints::default();

    // Retry-After first, then retry-after-ms; an unusable Retry-After falls through.
    let explicit = lookup(RETRY_AFTER)
        .and_then(|v| parse_retry_after(v, now, date_header))
        .map(|delay| (delay, RetryDelaySource::RetryAfter))
        .or_else(|| {
            lookup(RETRY_AFTER_MS)
                .and_then(parse_retry_after_ms)
                .map(|delay| (delay, RetryDelaySource::RetryAfterMs))
        });
    if let Some((delay, source)) = explicit {
        hints = hints.with_retry_after(delay, source);
    }

    match family {
        RateLimitHeaderFamily::Generic => {}
        RateLimitHeaderFamily::OpenAi => {
            for (kind, limit, remaining, reset) in [
                (
                    RateLimitDimensionKind::Requests,
                    OPENAI_LIMIT_REQUESTS,
                    OPENAI_REMAINING_REQUESTS,
                    OPENAI_RESET_REQUESTS,
                ),
                (
                    RateLimitDimensionKind::Tokens,
                    OPENAI_LIMIT_TOKENS,
                    OPENAI_REMAINING_TOKENS,
                    OPENAI_RESET_TOKENS,
                ),
            ] {
                if let Some(dim) = dimension(
                    lookup(limit).and_then(parse_count),
                    lookup(remaining).and_then(parse_count),
                    lookup(reset).and_then(parse_go_duration),
                ) {
                    hints = hints.with_dimension(kind, dim);
                }
            }
        }
        RateLimitHeaderFamily::Anthropic => {
            let reference = reference_time(now, date_header);
            for (kind, name) in [
                (RateLimitDimensionKind::Requests, "requests"),
                (RateLimitDimensionKind::Tokens, "tokens"),
                (RateLimitDimensionKind::InputTokens, "input-tokens"),
                (RateLimitDimensionKind::OutputTokens, "output-tokens"),
            ] {
                let header =
                    |suffix: &str| lookup(&format!("{ANTHROPIC_RATELIMIT_PREFIX}{name}-{suffix}"));
                if let Some(dim) = dimension(
                    header("limit").and_then(parse_count),
                    header("remaining").and_then(parse_count),
                    header("reset").and_then(|v| parse_rfc3339_reset(v, reference)),
                ) {
                    hints = hints.with_dimension(kind, dim);
                }
            }
        }
    }

    (!hints.is_empty()).then_some(hints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::rate_limit_hints::{RateLimitDimensionKind, RetryDelaySource};

    /// 2023-11-14T22:13:20Z -- a fixed "local clock" so every date test is deterministic.
    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    const fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// A case-insensitive lookup over a fixed list of (name, value) pairs, mirroring how an
    /// HTTP header map answers.
    fn lookup_over<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<&'a str> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| *v)
        }
    }

    fn retry_after(value: &str) -> Option<Duration> {
        parse_retry_after(value, now(), None)
    }

    fn http_date(offset_secs: i64) -> String {
        let t = if offset_secs >= 0 {
            now() + secs(offset_secs as u64)
        } else {
            now() - secs(offset_secs.unsigned_abs())
        };
        httpdate::fmt_http_date(t)
    }

    #[test]
    fn retry_after_boundary_values() {
        let cases: [(&str, Option<Duration>); 9] = [
            ("0", Some(secs(0))),
            ("-1", None),
            ("86400", Some(secs(86_400))),
            // Above 24 h: clamped to the ceiling, not rejected.
            ("86401", Some(CADENCE_DELAY_CEILING)),
            ("9999999999", Some(CADENCE_DELAY_CEILING)),
            // Overflows u64: no value, never a panic.
            ("99999999999999999999", None),
            ("1.5", Some(Duration::from_millis(1500))),
            ("7", Some(secs(7))),
            ("+7", None),
        ];
        for (input, want) in cases {
            assert_eq!(retry_after(input), want, "Retry-After: {input:?}");
        }
    }

    #[test]
    fn retry_after_http_date_boundaries() {
        // One second in the past, and exactly now: both are "retry immediately".
        assert_eq!(retry_after(&http_date(-1)), Some(secs(0)));
        assert_eq!(retry_after(&http_date(0)), Some(secs(0)));
        // A future date is measured against the local clock when no Date header is present.
        assert_eq!(retry_after(&http_date(30)), Some(secs(30)));
        // Equal to the response Date header: zero, even though the local clock disagrees.
        let date = http_date(-100);
        assert_eq!(parse_retry_after(&date, now(), Some(&date)), Some(secs(0)));
        // Relative to the response Date header (not the local clock) when it parses.
        let target = http_date(-95);
        assert_eq!(
            parse_retry_after(&target, now(), Some(&date)),
            Some(secs(5))
        );
        // An unparseable Date header falls back to the local clock.
        assert_eq!(
            parse_retry_after(&http_date(30), now(), Some("not a date")),
            Some(secs(30))
        );
        // A date years away is clamped at the ceiling.
        assert_eq!(
            retry_after("Fri, 31 Dec 9999 23:59:59 GMT"),
            Some(CADENCE_DELAY_CEILING)
        );
    }

    #[test]
    fn retry_after_all_three_http_date_forms_parse() {
        // 2023-11-14T22:13:20Z is `now()`; each form names `now() + 0 s`.
        assert_eq!(retry_after("Tue, 14 Nov 2023 22:13:20 GMT"), Some(secs(0)));
        assert_eq!(
            retry_after("Tuesday, 14-Nov-23 22:13:20 GMT"),
            Some(secs(0))
        );
        assert_eq!(retry_after("Tue Nov 14 22:13:20 2023"), Some(secs(0)));
        // And one second ahead, to prove they are parsed rather than defaulted to zero.
        assert_eq!(retry_after("Tue, 14 Nov 2023 22:13:21 GMT"), Some(secs(1)));
        assert_eq!(
            retry_after("Tuesday, 14-Nov-23 22:13:21 GMT"),
            Some(secs(1))
        );
        assert_eq!(retry_after("Tue Nov 14 22:13:21 2023"), Some(secs(1)));
    }

    #[test]
    fn retry_after_empty_and_whitespace() {
        for input in ["", " ", "\t", "  \t "] {
            assert_eq!(retry_after(input), None, "Retry-After: {input:?}");
        }
        assert_eq!(retry_after(" 7 "), Some(secs(7)));
        assert_eq!(retry_after("\t7\t"), Some(secs(7)));
        // An absent header yields no hints at all.
        assert_eq!(
            hints_from_headers(RateLimitHeaderFamily::Generic, now(), |_| None),
            None
        );
        // A present-but-empty one is the same as absent.
        let pairs = [(RETRY_AFTER, ""), (RETRY_AFTER_MS, "   ")];
        assert_eq!(
            hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs)),
            None
        );
    }

    #[test]
    fn retry_after_encoding_edge_cases() {
        // U+0667 ARABIC-INDIC DIGIT SEVEN is not an ASCII digit.
        assert_eq!(retry_after("\u{0667}"), None);
        assert_eq!(retry_after("1\u{0667}"), None);
        // Fullwidth digits likewise.
        assert_eq!(retry_after("\u{FF17}"), None);
        // Arbitrary text.
        assert_eq!(retry_after("soon"), None);
        assert_eq!(retry_after("7 seconds"), None);
        // Header names are matched case-insensitively by the lookup.
        let pairs = [("Retry-After", "7")];
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after(),
            Some((secs(7), RetryDelaySource::RetryAfter))
        );
        // A value longer than any legitimate header is refused before parsing.
        let long = "7".repeat(MAX_VALUE_BYTES + 1);
        assert_eq!(retry_after(&long), None);
    }

    #[cfg(feature = "openai")]
    #[test]
    fn a_non_utf8_header_value_yields_none_through_a_real_header_map() {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("retry-after"),
            HeaderValue::from_bytes(&[0xff, 0xfe, b'7']).expect("opaque bytes are a valid value"),
        );
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), |name| {
            headers.get(name).and_then(|v| v.to_str().ok())
        });
        assert_eq!(hints, None);

        // The same map answers a differently-cased name: the lookup is over the header map.
        let mut headers = HeaderMap::new();
        headers.insert("Retry-After", HeaderValue::from_static("9"));
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), |name| {
            headers.get(name).and_then(|v| v.to_str().ok())
        })
        .expect("hints");
        assert_eq!(hints.effective_retry_after().map(|(d, _)| d), Some(secs(9)));
    }

    #[test]
    fn retry_after_precision_never_panics_on_hostile_numbers() {
        for input in [
            "NaN", "nan", "inf", "-inf", "Infinity", "1e400", "1e3", "-0.5", ".", "..5", "1.2.3",
            "1,5", "0x10",
        ] {
            assert_eq!(retry_after(input), None, "Retry-After: {input:?}");
        }
        // A decimal too large for a Duration: no value rather than a panic.
        let huge = format!("{}.0", "9".repeat(60));
        assert_eq!(retry_after(&huge), None);
        // Decimal forms with one side empty are still plain decimals.
        assert_eq!(retry_after(".5"), Some(Duration::from_millis(500)));
        assert_eq!(retry_after("2."), Some(secs(2)));
        // Delta-seconds are exact.
        assert_eq!(retry_after("3600"), Some(secs(3600)));
    }

    #[test]
    fn go_duration_accepts_openai_reset_strings() {
        let cases = [
            ("6m0s", secs(360)),
            ("1s", secs(1)),
            ("6ms", Duration::from_millis(6)),
            ("1h2m3s", secs(3723)),
            ("1.5s", Duration::from_millis(1500)),
            ("250us", Duration::from_micros(250)),
            ("250\u{00B5}s", Duration::from_micros(250)),
            ("250\u{03BC}s", Duration::from_micros(250)),
            ("17ns", Duration::from_nanos(17)),
            (".5s", Duration::from_millis(500)),
            ("0s", secs(0)),
            (" 20ms ", Duration::from_millis(20)),
            ("1m30.5s", Duration::from_millis(90_500)),
        ];
        for (input, want) in cases {
            assert_eq!(parse_go_duration(input), Some(want), "reset: {input:?}");
        }
    }

    #[test]
    fn go_duration_rejects_empty_negative_and_trailing_garbage() {
        for input in [
            "",
            "  ",
            "-1s",
            "+1s",
            "1",
            "0",
            "s",
            "1x",
            "6m0s garbage",
            "6m0sx",
            "6m 0s",
            "1e400",
            "NaN",
            "inf",
            "1..5s",
            "1.5.5s",
            "5s6",
            "\u{0667}s",
        ] {
            assert_eq!(parse_go_duration(input), None, "reset: {input:?}");
        }
    }

    #[test]
    fn go_duration_is_bounded_on_hostile_input() {
        // 8 groups are accepted, 9 are not.
        assert_eq!(parse_go_duration("1s1s1s1s1s1s1s1s"), Some(secs(8)));
        assert_eq!(parse_go_duration("1s1s1s1s1s1s1s1s1s"), None);
        // More than 64 bytes is refused outright.
        let long = format!("{}s", "1".repeat(64));
        assert_eq!(parse_go_duration(&long), None);
        // A number with a huge integer part saturates at the ceiling instead of overflowing.
        let big = format!("{}h", "9".repeat(40));
        assert_eq!(parse_go_duration(&big), Some(CADENCE_DELAY_CEILING));
        assert_eq!(
            parse_go_duration("99999999999999h"),
            Some(CADENCE_DELAY_CEILING)
        );
        // 24 h exactly is representable; one second more clamps.
        assert_eq!(parse_go_duration("24h"), Some(secs(86_400)));
        assert_eq!(parse_go_duration("24h0m1s"), Some(CADENCE_DELAY_CEILING));
    }

    fn openai_pairs() -> [(&'static str, &'static str); 6] {
        [
            (OPENAI_LIMIT_REQUESTS, "60"),
            (OPENAI_REMAINING_REQUESTS, "0"),
            (OPENAI_RESET_REQUESTS, "6m0s"),
            (OPENAI_LIMIT_TOKENS, "150000"),
            (OPENAI_REMAINING_TOKENS, "149984"),
            (OPENAI_RESET_TOKENS, "1s"),
        ]
    }

    #[test]
    fn openai_family_parses_all_six_x_ratelimit_headers_into_requests_and_tokens_dimensions() {
        let pairs = openai_pairs();
        let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, now(), lookup_over(&pairs))
            .expect("hints");

        let requests = hints.dimension(RateLimitDimensionKind::Requests);
        assert_eq!(requests.limit(), Some(60));
        assert_eq!(requests.remaining(), Some(0));
        assert_eq!(requests.reset_after(), Some(secs(360)));

        let tokens = hints.dimension(RateLimitDimensionKind::Tokens);
        assert_eq!(tokens.limit(), Some(150_000));
        assert_eq!(tokens.remaining(), Some(149_984));
        assert_eq!(tokens.reset_after(), Some(secs(1)));

        // The Anthropic-only dimensions stay unreported.
        assert_eq!(
            hints.dimension(RateLimitDimensionKind::InputTokens),
            &Default::default()
        );
        // No explicit delay was sent: the exhausted request dimension's reset stands in.
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(360), RetryDelaySource::ResetHeader))
        );
    }

    #[test]
    fn openai_explicit_retry_after_wins_over_the_reset_headers() {
        let mut pairs = openai_pairs().to_vec();
        pairs.push((RETRY_AFTER, "7"));
        let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(7), RetryDelaySource::RetryAfter))
        );
        assert_eq!(
            hints
                .dimension(RateLimitDimensionKind::Requests)
                .reset_after(),
            Some(secs(360))
        );
    }

    #[test]
    fn openai_placeholder_and_negative_values_become_none_per_field() {
        let pairs = [
            (OPENAI_LIMIT_REQUESTS, "-1"),
            (OPENAI_REMAINING_REQUESTS, "-1"),
            (OPENAI_RESET_REQUESTS, "-1s"),
            (OPENAI_REMAINING_TOKENS, "7"),
            (OPENAI_RESET_TOKENS, "garbage"),
        ];
        let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, now(), lookup_over(&pairs))
            .expect("the tokens remaining value is usable");
        assert_eq!(
            hints.dimension(RateLimitDimensionKind::Requests),
            &Default::default()
        );
        let tokens = hints.dimension(RateLimitDimensionKind::Tokens);
        assert_eq!(tokens.limit(), None);
        assert_eq!(tokens.remaining(), Some(7));
        assert_eq!(tokens.reset_after(), None);
    }

    fn anthropic_pairs() -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for (dim, limit, remaining, reset) in [
            ("requests", "50", "0", "2023-11-14T22:13:30Z"),
            ("tokens", "40000", "39000", "2023-11-14T22:13:25Z"),
            ("input-tokens", "30000", "29000", "2023-11-14T22:13:21Z"),
            ("output-tokens", "8000", "0", "2023-11-14T22:14:20Z"),
        ] {
            for (suffix, value) in [("limit", limit), ("remaining", remaining), ("reset", reset)] {
                pairs.push((
                    format!("{ANTHROPIC_RATELIMIT_PREFIX}{dim}-{suffix}"),
                    value.to_owned(),
                ));
            }
        }
        pairs
    }

    fn as_refs(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
        pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    #[test]
    fn anthropic_family_parses_four_dimensions_with_rfc3339_resets_relative_to_the_date_header() {
        let mut owned = anthropic_pairs();
        // The response Date is 100 s before the local clock: every reset is measured from it.
        let date = http_date(-100);
        owned.push(("date".to_owned(), date));
        owned.push(("retry-after".to_owned(), "7".to_owned()));
        let pairs = as_refs(&owned);

        let hints =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs))
                .expect("hints");

        // Date = 22:11:40, so +110 s, +105 s, +101 s, +160 s.
        let want = [
            (RateLimitDimensionKind::Requests, 50, 0, 110),
            (RateLimitDimensionKind::Tokens, 40_000, 39_000, 105),
            (RateLimitDimensionKind::InputTokens, 30_000, 29_000, 101),
            (RateLimitDimensionKind::OutputTokens, 8_000, 0, 160),
        ];
        for (kind, limit, remaining, reset) in want {
            let dim = hints.dimension(kind);
            assert_eq!(dim.limit(), Some(limit), "{kind:?} limit");
            assert_eq!(dim.remaining(), Some(remaining), "{kind:?} remaining");
            assert_eq!(dim.reset_after(), Some(secs(reset)), "{kind:?} reset");
        }
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(7), RetryDelaySource::RetryAfter))
        );
    }

    #[test]
    fn anthropic_resets_fall_back_to_the_local_clock_when_the_date_header_is_unusable() {
        let mut owned = anthropic_pairs();
        owned.push(("date".to_owned(), "yesterday-ish".to_owned()));
        let pairs = as_refs(&owned);
        let hints =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs))
                .expect("hints");
        // now() = 22:13:20, so the requests reset (22:13:30) is 10 s away.
        assert_eq!(
            hints
                .dimension(RateLimitDimensionKind::Requests)
                .reset_after(),
            Some(secs(10))
        );
        // No retry-after: the largest reset among exhausted dimensions (output tokens, 60 s).
        assert_eq!(
            hints.effective_retry_after(),
            Some((secs(60), RetryDelaySource::ResetHeader))
        );
    }

    #[test]
    fn anthropic_past_reset_is_zero_and_garbage_reset_is_none() {
        let pairs = [
            ("anthropic-ratelimit-requests-reset", "2020-01-01T00:00:00Z"),
            ("anthropic-ratelimit-tokens-reset", "not-a-timestamp"),
            ("anthropic-ratelimit-tokens-remaining", "5"),
        ];
        let hints =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs))
                .expect("hints");
        assert_eq!(
            hints
                .dimension(RateLimitDimensionKind::Requests)
                .reset_after(),
            Some(secs(0))
        );
        let tokens = hints.dimension(RateLimitDimensionKind::Tokens);
        assert_eq!(tokens.reset_after(), None);
        assert_eq!(tokens.remaining(), Some(5));
        // An offset timestamp is converted, not misread.
        let pairs = [(
            "anthropic-ratelimit-requests-reset",
            "2023-11-14T23:13:50+01:00",
        )];
        let hints =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs))
                .expect("hints");
        assert_eq!(
            hints
                .dimension(RateLimitDimensionKind::Requests)
                .reset_after(),
            Some(secs(30))
        );
    }

    #[test]
    fn generic_family_ignores_provider_specific_headers() {
        let mut owned: Vec<(String, String)> = anthropic_pairs();
        owned.extend(
            openai_pairs()
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        let pairs = as_refs(&owned);
        assert_eq!(
            hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs)),
            None
        );

        // Only the generic delay is read.
        let mut with_delay = owned.clone();
        with_delay.push(("retry-after".to_owned(), "3".to_owned()));
        let pairs = as_refs(&with_delay);
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after(),
            Some((secs(3), RetryDelaySource::RetryAfter))
        );
        assert_eq!(
            hints.dimension(RateLimitDimensionKind::Requests),
            &Default::default()
        );
    }

    #[test]
    fn openai_family_does_not_read_anthropic_headers_and_vice_versa() {
        let owned = anthropic_pairs();
        let pairs = as_refs(&owned);
        assert_eq!(
            hints_from_headers(RateLimitHeaderFamily::OpenAi, now(), lookup_over(&pairs)),
            None
        );
        let pairs = openai_pairs();
        assert_eq!(
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs)),
            None
        );
    }

    #[test]
    fn retry_after_ms_is_read_when_retry_after_is_absent_and_tagged_retry_after_ms() {
        let pairs = [(RETRY_AFTER_MS, "2500")];
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after(),
            Some((Duration::from_millis(2500), RetryDelaySource::RetryAfterMs))
        );

        // Retry-After wins when both are present and usable.
        let pairs = [(RETRY_AFTER_MS, "2500"), (RETRY_AFTER, "7")];
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after(),
            Some((secs(7), RetryDelaySource::RetryAfter))
        );

        // An unusable Retry-After falls through to retry-after-ms.
        let pairs = [(RETRY_AFTER_MS, "1500"), (RETRY_AFTER, "-1")];
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after(),
            Some((Duration::from_millis(1500), RetryDelaySource::RetryAfterMs))
        );

        // Hostile retry-after-ms values: no value, no panic; a huge one is clamped.
        for bad in ["", "-5", "NaN", "inf", "1e400", "99999999999999999999999"] {
            let pairs = [(RETRY_AFTER_MS, bad)];
            assert_eq!(
                hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs)),
                None,
                "retry-after-ms: {bad:?}"
            );
        }
        let pairs = [(RETRY_AFTER_MS, "18446744073709551615")];
        let hints = hints_from_headers(RateLimitHeaderFamily::Generic, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after().map(|(d, _)| d),
            Some(CADENCE_DELAY_CEILING)
        );
    }

    #[test]
    fn a_response_with_no_rate_limit_header_yields_no_hints() {
        let pairs = [("content-type", "application/json"), ("server", "envoy")];
        for family in [
            RateLimitHeaderFamily::Generic,
            RateLimitHeaderFamily::OpenAi,
            RateLimitHeaderFamily::Anthropic,
        ] {
            assert_eq!(
                hints_from_headers(family, now(), lookup_over(&pairs)),
                None,
                "{family:?}"
            );
        }
    }

    #[test]
    fn hints_from_headers_is_idempotent() {
        let mut owned = anthropic_pairs();
        owned.push(("retry-after".to_owned(), http_date(12)));
        owned.push(("date".to_owned(), http_date(-3)));
        let pairs = as_refs(&owned);
        let first =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs));
        let second =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs));
        assert!(first.is_some());
        assert_eq!(first, second);
    }

    #[test]
    fn parsed_hints_carry_numbers_only_never_a_raw_header_value() {
        let pairs = openai_pairs();
        let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, now(), lookup_over(&pairs))
            .expect("hints");
        let rendered = format!("{hints:?}");
        assert!(!rendered.contains("6m0s"), "{rendered}");
        let owned = anthropic_pairs();
        let pairs = as_refs(&owned);
        let hints =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs))
                .expect("hints");
        let rendered = format!("{hints:?}");
        assert!(!rendered.contains("2023-11-14T"), "{rendered}");
    }

    #[test]
    fn every_parsed_delay_is_at_most_the_cadence_ceiling() {
        let pairs = [
            (RETRY_AFTER, "99999999"),
            (OPENAI_RESET_REQUESTS, "999999h"),
            (OPENAI_REMAINING_REQUESTS, "0"),
            ("anthropic-ratelimit-tokens-reset", "9999-12-31T23:59:59Z"),
        ];
        let hints = hints_from_headers(RateLimitHeaderFamily::OpenAi, now(), lookup_over(&pairs))
            .expect("hints");
        assert_eq!(
            hints.explicit_retry_after().map(|(d, _)| d),
            Some(CADENCE_DELAY_CEILING)
        );
        assert_eq!(
            hints
                .dimension(RateLimitDimensionKind::Requests)
                .reset_after(),
            Some(CADENCE_DELAY_CEILING)
        );
        let hints =
            hints_from_headers(RateLimitHeaderFamily::Anthropic, now(), lookup_over(&pairs))
                .expect("hints");
        assert_eq!(
            hints
                .dimension(RateLimitDimensionKind::Tokens)
                .reset_after(),
            Some(CADENCE_DELAY_CEILING)
        );
    }
}
