//! Treasurer admission value types (ALLOW-01, ALLOW-02, ALLOW-04, Phase 41 D-14).
//!
//! Pure, serde-derived value types with no I/O. One refusal/warning shape is shared by every
//! surface that reports an allowance outcome -- the HTTP error body, the trace event and the
//! operator webhook payload (D-14) -- so a figure cannot drift between them.
//!
//! Every money figure is a [`Cost`] (`i64` nano-units of one currency); it is rendered only
//! through [`format_cost`], the display edge, and is never fed back into a comparison. A
//! refusal carries the store instant it was evaluated at ([`AllowanceRefusal::evaluated_at`]),
//! so `Retry-After` is derived from the one authoritative store clock and never from a web
//! process's own clock (ALLOW-01).
//!
//! Scope and limit kinds are plain identifiers -- `Treasurer` is the only officer word the
//! allowance feature introduces (D-00f).

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::container::cost::Cost;
use crate::platform::container::treasury_ledger::format_cost;

/// Which identity an allowance ceiling is held against.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::AllowanceScopeKind;
///
/// assert_eq!(AllowanceScopeKind::ApiKey.as_str(), "api_key");
/// assert_eq!(serde_json::to_string(&AllowanceScopeKind::Tenant)?, "\"tenant\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowanceScopeKind {
    /// A ceiling shared by every API key of one tenant.
    Tenant,
    /// A ceiling held by one API key.
    ApiKey,
}

impl AllowanceScopeKind {
    /// The snake_case wire string for this kind (`"tenant"`, `"api_key"`).
    pub fn as_str(self) -> &'static str {
        match self {
            AllowanceScopeKind::Tenant => "tenant",
            AllowanceScopeKind::ApiKey => "api_key",
        }
    }
}

/// Which kind of limit a ceiling is.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::AllowanceLimitKind;
///
/// assert_eq!(AllowanceLimitKind::Window.as_str(), "window");
/// assert_eq!(serde_json::to_string(&AllowanceLimitKind::Lifetime)?, "\"lifetime\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowanceLimitKind {
    /// A rolling-period (tumbling, epoch-aligned) allowance.
    Window,
    /// A cap over everything ever recorded for the scope.
    Lifetime,
}

impl AllowanceLimitKind {
    /// The snake_case wire string for this kind (`"window"`, `"lifetime"`).
    pub fn as_str(self) -> &'static str {
        match self {
            AllowanceLimitKind::Window => "window",
            AllowanceLimitKind::Lifetime => "lifetime",
        }
    }
}

/// The refusal of an admission: a balance has reached a configured ceiling (D-14).
///
/// Carries the refused ceiling's own figures only -- never the caller's tenant id or key name,
/// and never another scope's figures (D-13). A balance exactly at the ceiling is exhausted.
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use paladin_core::platform::container::allowance::{
///     AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind,
/// };
/// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
///
/// let usd = CurrencyCode::new("USD")?;
/// let start = Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).single().ok_or("bad instant")?;
/// let end = Utc.with_ymd_and_hms(2026, 10, 4, 0, 0, 0).single().ok_or("bad instant")?;
/// let evaluated_at = Utc.with_ymd_and_hms(2026, 10, 3, 23, 0, 0).single().ok_or("bad instant")?;
/// let refusal = AllowanceRefusal {
///     scope_kind: AllowanceScopeKind::ApiKey,
///     limit_kind: AllowanceLimitKind::Window,
///     balance: Cost::new(2_500_000_000, usd.clone()),
///     ceiling: Cost::new(2_500_000_000, usd),
///     window: Some((start, end)),
///     evaluated_at,
/// };
/// assert_eq!(refusal.retry_after_secs(), Some(3600));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowanceRefusal {
    /// Which identity the refused ceiling is held against.
    pub scope_kind: AllowanceScopeKind,
    /// Which kind of limit the refused ceiling is.
    pub limit_kind: AllowanceLimitKind,
    /// The balance read for the refused ceiling's scope and window.
    pub balance: Cost,
    /// The ceiling the balance has reached.
    pub ceiling: Cost,
    /// The half-open `[start, end)` window the balance covers; `None` for a lifetime ceiling.
    pub window: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// The store instant (whole seconds) the window was computed from -- the base of
    /// `Retry-After`, so no web-process clock is ever read.
    pub evaluated_at: DateTime<Utc>,
}

impl AllowanceRefusal {
    /// Whole seconds from [`AllowanceRefusal::evaluated_at`] to the window's end, at least `1`;
    /// `None` for a lifetime refusal (a client must not spin on a cap that never reopens).
    pub fn retry_after_secs(&self) -> Option<u64> {
        let (_, end) = self.window?;
        let secs = (end - self.evaluated_at).num_seconds().max(1);
        Some(u64::try_from(secs).unwrap_or(1))
    }
}

impl std::fmt::Display for AllowanceRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} allowance exhausted: balance {} has reached the ceiling {}",
            self.scope_kind.as_str(),
            self.limit_kind.as_str(),
            format_cost(&self.balance),
            format_cost(&self.ceiling),
        )?;
        if let Some((start, end)) = self.window {
            write!(
                f,
                " (window {} to {})",
                start.to_rfc3339_opts(SecondsFormat::Secs, true),
                end.to_rfc3339_opts(SecondsFormat::Secs, true),
            )?;
        }
        Ok(())
    }
}

/// A warning that a balance crossed its configured `warn_at` percentage of a ceiling.
///
/// Both window bounds are `None` for a lifetime ceiling. Populated by the notice plans
/// (41-06, 41-07); defined here so the shape is stable across the phase (D-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowanceWarning {
    /// Which identity the ceiling is held against.
    pub scope_kind: AllowanceScopeKind,
    /// Which kind of limit the ceiling is.
    pub limit_kind: AllowanceLimitKind,
    /// The balance read when the warning fired.
    pub balance: Cost,
    /// The ceiling the balance is approaching.
    pub ceiling: Cost,
    /// The window's start (inclusive); `None` for a lifetime ceiling.
    pub window_start: Option<DateTime<Utc>>,
    /// The window's end (exclusive); `None` for a lifetime ceiling.
    pub window_end: Option<DateTime<Utc>>,
    /// The configured warn threshold, in whole percent of the ceiling.
    pub warn_at: u8,
}

/// A once-per-window notice this admission won the right to emit (41-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowanceNotice {
    /// Stable identifier of the notice row.
    pub notice_id: String,
    /// The warning the notice carries.
    pub warning: AllowanceWarning,
}

/// The outcome of a successful admission: the notices (if any) it won.
///
/// An admission that crossed no warn threshold is [`Admission::none`].
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::Admission;
///
/// let admission = Admission::none();
/// assert!(admission.is_empty());
/// assert_eq!(admission.warnings().count(), 0);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Admission {
    notices: Vec<AllowanceNotice>,
}

impl Admission {
    /// An admission that won no notice.
    pub fn none() -> Self {
        Self::default()
    }

    /// Add a won notice.
    pub fn with_notice(mut self, notice: AllowanceNotice) -> Self {
        self.notices.push(notice);
        self
    }

    /// The notices this admission won.
    pub fn notices(&self) -> &[AllowanceNotice] {
        &self.notices
    }

    /// The warnings carried by the won notices.
    pub fn warnings(&self) -> impl Iterator<Item = &AllowanceWarning> {
        self.notices.iter().map(|n| &n.warning)
    }

    /// Whether this admission won no notice.
    pub fn is_empty(&self) -> bool {
        self.notices.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::container::cost::CurrencyCode;
    use chrono::TimeZone;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is a valid currency code")
    }

    fn at(day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, day, hour, min, sec)
            .single()
            .expect("valid instant")
    }

    fn window_refusal(evaluated_at: DateTime<Utc>) -> AllowanceRefusal {
        AllowanceRefusal {
            scope_kind: AllowanceScopeKind::ApiKey,
            limit_kind: AllowanceLimitKind::Window,
            balance: Cost::new(2_500_000_000, usd()),
            ceiling: Cost::new(2_500_000_000, usd()),
            window: Some((at(3, 0, 0, 0), at(4, 0, 0, 0))),
            evaluated_at,
        }
    }

    #[test]
    fn scope_and_limit_kinds_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&AllowanceScopeKind::ApiKey).unwrap(),
            "\"api_key\""
        );
        assert_eq!(
            serde_json::to_string(&AllowanceScopeKind::Tenant).unwrap(),
            "\"tenant\""
        );
        assert_eq!(
            serde_json::to_string(&AllowanceLimitKind::Window).unwrap(),
            "\"window\""
        );
        assert_eq!(
            serde_json::to_string(&AllowanceLimitKind::Lifetime).unwrap(),
            "\"lifetime\""
        );
        for kind in [AllowanceScopeKind::Tenant, AllowanceScopeKind::ApiKey] {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.as_str())
            );
        }
        for kind in [AllowanceLimitKind::Window, AllowanceLimitKind::Lifetime] {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.as_str())
            );
        }
    }

    #[test]
    fn refusal_retry_after_is_seconds_to_window_end() {
        let refusal = window_refusal(at(3, 23, 0, 0));
        assert_eq!(refusal.retry_after_secs(), Some(3600));
        let refusal = window_refusal(at(3, 0, 0, 0));
        assert_eq!(refusal.retry_after_secs(), Some(86_400));
    }

    #[test]
    fn refusal_retry_after_is_at_least_one_second() {
        // At (or, defensively, past) the window end the answer is still 1, never 0.
        assert_eq!(window_refusal(at(4, 0, 0, 0)).retry_after_secs(), Some(1));
        assert_eq!(window_refusal(at(4, 0, 5, 0)).retry_after_secs(), Some(1));
        assert_eq!(
            window_refusal(at(3, 23, 59, 59)).retry_after_secs(),
            Some(1)
        );
    }

    #[test]
    fn lifetime_refusal_has_no_retry_after() {
        let refusal = AllowanceRefusal {
            scope_kind: AllowanceScopeKind::Tenant,
            limit_kind: AllowanceLimitKind::Lifetime,
            balance: Cost::new(10, usd()),
            ceiling: Cost::new(10, usd()),
            window: None,
            evaluated_at: at(3, 12, 0, 0),
        };
        assert_eq!(refusal.retry_after_secs(), None);
    }

    #[test]
    fn refusal_display_names_kind_and_figures_but_no_identity() {
        let text = window_refusal(at(3, 12, 0, 0)).to_string();
        assert!(
            text.contains("api_key window allowance exhausted"),
            "{text}"
        );
        assert!(text.contains("2.5000 USD"), "{text}");
        assert!(text.contains("2026-10-03T00:00:00Z"), "{text}");
        assert!(text.contains("2026-10-04T00:00:00Z"), "{text}");
        // No tenant id and no key name is ever in scope of the value.
        assert!(!text.contains("acme"));
        assert!(!text.contains("svc-a"));

        let lifetime = AllowanceRefusal {
            window: None,
            limit_kind: AllowanceLimitKind::Lifetime,
            scope_kind: AllowanceScopeKind::Tenant,
            ..window_refusal(at(3, 12, 0, 0))
        };
        let text = lifetime.to_string();
        assert!(
            text.contains("tenant lifetime allowance exhausted"),
            "{text}"
        );
        assert!(!text.contains("window 20"), "{text}");
    }

    #[test]
    fn refusal_round_trips_through_serde() {
        let refusal = window_refusal(at(3, 12, 0, 0));
        let json = serde_json::to_string(&refusal).unwrap();
        let back: AllowanceRefusal = serde_json::from_str(&json).unwrap();
        assert_eq!(refusal, back);
    }

    #[test]
    fn admission_none_is_empty() {
        let admission = Admission::none();
        assert!(admission.is_empty());
        assert!(admission.notices().is_empty());
        assert_eq!(admission.warnings().count(), 0);

        let warning = AllowanceWarning {
            scope_kind: AllowanceScopeKind::ApiKey,
            limit_kind: AllowanceLimitKind::Window,
            balance: Cost::new(80, usd()),
            ceiling: Cost::new(100, usd()),
            window_start: Some(at(3, 0, 0, 0)),
            window_end: Some(at(4, 0, 0, 0)),
            warn_at: 80,
        };
        let admission = Admission::none().with_notice(AllowanceNotice {
            notice_id: "n-1".to_string(),
            warning: warning.clone(),
        });
        assert!(!admission.is_empty());
        assert_eq!(admission.warnings().next(), Some(&warning));
    }
}
