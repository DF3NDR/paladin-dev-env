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
use crate::platform::container::run::RunId;
use crate::platform::container::trace::TraceEvent;
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

/// The horizon clause of a herald line: when a window ceiling resets, or that a lifetime ceiling
/// never does (Phase 42 review IN-4).
///
/// A window ceiling with no recorded end (a malformed or deserialised refusal or warning) names
/// itself `window` rather than `lifetime cap`: stating a lifetime cap would be the wrong fact.
fn horizon_phrase(kind: AllowanceLimitKind, window_end: Option<DateTime<Utc>>) -> String {
    match (kind, window_end) {
        (AllowanceLimitKind::Window, Some(end)) => {
            format!(
                "window resets {}",
                end.to_rfc3339_opts(SecondsFormat::Secs, true)
            )
        }
        (AllowanceLimitKind::Window, None) => "window".to_string(),
        (AllowanceLimitKind::Lifetime, _) => "lifetime cap".to_string(),
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// The refusal's figures as the one caller-facing JSON object: `scope`, `kind`, `balance`,
    /// `ceiling`, `window_start` and `window_end`.
    ///
    /// This is the single builder of the refusal figures for every wire surface: the Phase 41
    /// `429 allowance_exhausted` `details` object and the `halt_reason` of a halted run both
    /// call it, so the figures cannot drift between them. Money is rendered through
    /// [`format_cost`] (the display edge); instants are RFC 3339 with whole seconds and a `Z`
    /// suffix; both window bounds are JSON `null` for a lifetime refusal.
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
    /// let evaluated_at = Utc.with_ymd_and_hms(2026, 10, 3, 23, 0, 0).single().ok_or("bad instant")?;
    /// let refusal = AllowanceRefusal {
    ///     scope_kind: AllowanceScopeKind::Tenant,
    ///     limit_kind: AllowanceLimitKind::Lifetime,
    ///     balance: Cost::new(1_000_000_000, usd.clone()),
    ///     ceiling: Cost::new(1_000_000_000, usd),
    ///     window: None,
    ///     evaluated_at,
    /// };
    /// let details = refusal.details_json();
    /// assert_eq!(details["scope"], "tenant");
    /// assert_eq!(details["kind"], "lifetime");
    /// assert_eq!(details["ceiling"], "1.0000 USD");
    /// assert!(details["window_start"].is_null());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn details_json(&self) -> serde_json::Value {
        let rfc3339 = |instant: DateTime<Utc>| instant.to_rfc3339_opts(SecondsFormat::Secs, true);
        let (window_start, window_end) = match self.window {
            Some((start, end)) => (Some(rfc3339(start)), Some(rfc3339(end))),
            None => (None, None),
        };
        serde_json::json!({
            "scope": self.scope_kind.as_str(),
            "kind": self.limit_kind.as_str(),
            "balance": format_cost(&self.balance),
            "ceiling": format_cost(&self.ceiling),
            "window_start": window_start,
            "window_end": window_end,
        })
    }
}

/// Why a run was halted at a superstep boundary or an agent-loop cutoff by the Treasurer
/// (ALLOW-03, Phase 42 D-04, D-05, ADR-0057).
///
/// A halt is a resume point, not a failure: the run keeps its last checkpoint and ends with
/// status `halted` and no error. The serde form is the persisted form -- an internally tagged
/// object whose discriminator is named `reason` (never `kind`, which already means the limit
/// kind inside the `429` details) and whose figures are integer nano-units plus a currency,
/// never display strings.
///
/// [`HaltReason::wire_json`] is the ONE caller-facing builder (`GET /runs`, the SSE `done`, the
/// webhook key and the agent response all call it); do not serialise this enum straight onto a
/// caller-facing surface.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::HaltReason;
///
/// let reason = HaltReason::LedgerUnavailable;
/// assert_eq!(reason.as_str(), "ledger_unavailable");
/// assert_eq!(serde_json::to_string(&reason)?, r#"{"reason":"ledger_unavailable"}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
#[non_exhaustive]
pub enum HaltReason {
    /// A ceiling was reached; the figures are that ceiling's own.
    AllowanceExhausted(AllowanceRefusal),
    /// A ceiling could not be evaluated (a failed balance or store-clock read); the run halts
    /// fail-closed (D-03) and carries no figures.
    LedgerUnavailable,
}

impl HaltReason {
    /// The snake_case discriminator string: `"allowance_exhausted"` or `"ledger_unavailable"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::allowance::HaltReason;
    ///
    /// assert_eq!(HaltReason::LedgerUnavailable.as_str(), "ledger_unavailable");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            HaltReason::AllowanceExhausted(_) => "allowance_exhausted",
            HaltReason::LedgerUnavailable => "ledger_unavailable",
        }
    }

    /// The one-line operator rendering shared by the markdown, JSON and table heralds (D-19):
    /// `⛔ halted: allowance exhausted — 25.0000 of 25.0000 USD (api_key, window resets
    /// 2026-10-06T00:00:00Z)` for a window ceiling, `... (tenant, lifetime cap)` for a lifetime
    /// one, and `⛔ halted: allowance could not be evaluated (ledger unavailable)` for a ledger
    /// outage.
    ///
    /// It mirrors [`AllowanceWarning::herald_line`]: the line names the scope kind, never the
    /// tenant id or the key name, and never a key value (D-00g). Both figures go through
    /// [`format_cost`], the display edge (D-00h).
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{TimeZone, Utc};
    /// use paladin_core::platform::container::allowance::{
    ///     AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind, HaltReason,
    /// };
    /// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    ///
    /// let usd = CurrencyCode::new("USD")?;
    /// let reason = HaltReason::AllowanceExhausted(AllowanceRefusal {
    ///     scope_kind: AllowanceScopeKind::ApiKey,
    ///     limit_kind: AllowanceLimitKind::Window,
    ///     balance: Cost::new(25_000_000_000, usd.clone()),
    ///     ceiling: Cost::new(25_000_000_000, usd),
    ///     window: Utc
    ///         .with_ymd_and_hms(2026, 10, 5, 0, 0, 0)
    ///         .single()
    ///         .zip(Utc.with_ymd_and_hms(2026, 10, 6, 0, 0, 0).single()),
    ///     evaluated_at: Utc.with_ymd_and_hms(2026, 10, 5, 12, 0, 0).single().unwrap_or_default(),
    /// });
    /// assert_eq!(
    ///     reason.herald_line(),
    ///     "\u{26D4} halted: allowance exhausted \u{2014} 25.0000 of 25.0000 USD \
    ///      (api_key, window resets 2026-10-06T00:00:00Z)"
    /// );
    /// assert_eq!(
    ///     HaltReason::LedgerUnavailable.herald_line(),
    ///     "\u{26D4} halted: allowance could not be evaluated (ledger unavailable)"
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn herald_line(&self) -> String {
        match self {
            HaltReason::AllowanceExhausted(refusal) => {
                let horizon =
                    horizon_phrase(refusal.limit_kind, refusal.window.map(|(_, end)| end));
                // The currency is stated once, after the ceiling ("25.0000 of 25.0000 USD"); a
                // balance in a different currency keeps its own code so nothing is misread.
                let balance = format_cost(&refusal.balance);
                let balance = if refusal.balance.currency() == refusal.ceiling.currency() {
                    let code = format!(" {}", refusal.balance.currency().as_str());
                    balance.strip_suffix(code.as_str()).unwrap_or(&balance)
                } else {
                    balance.as_str()
                };
                format!(
                    "\u{26D4} halted: allowance exhausted \u{2014} {balance} of {} ({}, {horizon})",
                    format_cost(&refusal.ceiling),
                    refusal.scope_kind.as_str(),
                )
            }
            HaltReason::LedgerUnavailable => {
                "\u{26D4} halted: allowance could not be evaluated (ledger unavailable)".to_string()
            }
        }
    }

    /// The caller-facing JSON object for this reason: the one wire builder (D-06, D-14, D-16).
    ///
    /// For [`HaltReason::AllowanceExhausted`] it is the refusal's
    /// [`AllowanceRefusal::details_json`] object with `"reason": "allowance_exhausted"` added;
    /// for [`HaltReason::LedgerUnavailable`] it is exactly `{"reason":"ledger_unavailable"}`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::allowance::HaltReason;
    ///
    /// let wire = HaltReason::LedgerUnavailable.wire_json();
    /// assert_eq!(wire, serde_json::json!({"reason": "ledger_unavailable"}));
    /// ```
    pub fn wire_json(&self) -> serde_json::Value {
        match self {
            HaltReason::AllowanceExhausted(refusal) => {
                let mut object = refusal.details_json();
                if let Some(map) = object.as_object_mut() {
                    map.insert(
                        "reason".to_owned(),
                        serde_json::Value::String(self.as_str().to_owned()),
                    );
                }
                object
            }
            HaltReason::LedgerUnavailable => {
                serde_json::json!({ "reason": self.as_str() })
            }
        }
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

impl AllowanceWarning {
    /// The warning carried by a [`TraceEvent::AllowanceWarning`], or `None` for any other
    /// event (the inverse of `TraceEvent::from(warning)`).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::allowance::AllowanceWarning;
    /// use paladin_core::platform::container::trace::TraceEvent;
    ///
    /// let event = TraceEvent::RunStarted { run_id: None, graph_fingerprint: "fp".into() };
    /// assert_eq!(AllowanceWarning::from_trace_event(&event), None);
    /// ```
    pub fn from_trace_event(event: &TraceEvent) -> Option<AllowanceWarning> {
        match event {
            TraceEvent::AllowanceWarning {
                scope_kind,
                limit_kind,
                balance,
                ceiling,
                window_start,
                window_end,
                warn_at,
            } => Some(AllowanceWarning {
                scope_kind: *scope_kind,
                limit_kind: *limit_kind,
                balance: balance.clone(),
                ceiling: ceiling.clone(),
                window_start: *window_start,
                window_end: *window_end,
                warn_at: *warn_at,
            }),
            _ => None,
        }
    }

    /// The whole percent of the ceiling the balance has reached: the floor of
    /// `balance * 100 / ceiling` in integer arithmetic (never rounded up), `0` when the ceiling
    /// is not positive or the balance is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::allowance::{
    ///     AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning,
    /// };
    /// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    ///
    /// let usd = CurrencyCode::new("USD")?;
    /// let warning = AllowanceWarning {
    ///     scope_kind: AllowanceScopeKind::Tenant,
    ///     limit_kind: AllowanceLimitKind::Lifetime,
    ///     balance: Cost::new(829_999_999, usd.clone()),
    ///     ceiling: Cost::new(1_000_000_000, usd),
    ///     window_start: None,
    ///     window_end: None,
    ///     warn_at: 80,
    /// };
    /// assert_eq!(warning.percent_of_ceiling(), 82);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn percent_of_ceiling(&self) -> u64 {
        let ceiling = i128::from(self.ceiling.nanos());
        let balance = i128::from(self.balance.nanos());
        if ceiling <= 0 || balance < 0 {
            return 0;
        }
        u64::try_from(balance * 100 / ceiling).unwrap_or(u64::MAX)
    }

    /// The one-line operator rendering shared by the markdown, JSON and table heralds (D-18):
    /// `⚠ allowance: 82% of 25.0000 USD (api_key, window resets 2026-10-03T00:00:00Z)` for a
    /// window ceiling and `... (tenant, lifetime cap)` for a lifetime one.
    ///
    /// The line names the scope kind, never the tenant id or the key name, and never a key value
    /// (D-00g). The ceiling is rendered through [`format_cost`], the display edge (D-00h).
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{TimeZone, Utc};
    /// use paladin_core::platform::container::allowance::{
    ///     AllowanceLimitKind, AllowanceScopeKind, AllowanceWarning,
    /// };
    /// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
    ///
    /// let usd = CurrencyCode::new("USD")?;
    /// let warning = AllowanceWarning {
    ///     scope_kind: AllowanceScopeKind::ApiKey,
    ///     limit_kind: AllowanceLimitKind::Window,
    ///     balance: Cost::new(20_500_000_000, usd.clone()),
    ///     ceiling: Cost::new(25_000_000_000, usd),
    ///     window_start: Utc.with_ymd_and_hms(2026, 10, 2, 0, 0, 0).single(),
    ///     window_end: Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).single(),
    ///     warn_at: 80,
    /// };
    /// assert_eq!(
    ///     warning.herald_line(),
    ///     "⚠ allowance: 82% of 25.0000 USD (api_key, window resets 2026-10-03T00:00:00Z)"
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn herald_line(&self) -> String {
        let scope = self.scope_kind.as_str();
        let horizon = horizon_phrase(self.limit_kind, self.window_end);
        format!(
            "\u{26A0} allowance: {}% of {} ({scope}, {horizon})",
            self.percent_of_ceiling(),
            format_cost(&self.ceiling),
        )
    }
}

/// Which rung of the allowance ladder a notice reports (Phase 42 D-18, ADR-0057 group g).
///
/// `Warning` is the Phase 41 warn-threshold crossing; `Halt` is the operator-facing notice that a
/// spend halt ended a run at the ceiling. The kind is PART of the notice's once-per-window
/// identity, so a warning and a halt for the same scope, limit, window and ceiling are two
/// distinct notices, each recorded once. Every row written before migration 014 is a `Warning`.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::NoticeKind;
///
/// assert_eq!(NoticeKind::default(), NoticeKind::Warning);
/// assert_eq!(NoticeKind::Warning.as_str(), "warning");
/// assert_eq!(NoticeKind::Halt.as_str(), "halt");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NoticeKind {
    /// A warn-threshold crossing (the Phase 41 notice, and the default).
    #[default]
    Warning,
    /// A spend halt reached the ceiling (Phase 42 D-18).
    Halt,
}

impl NoticeKind {
    /// The stable lowercase text stored in `treasury_notices.notice_kind` and carried on the
    /// wire: `"warning"` or `"halt"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::allowance::NoticeKind;
    ///
    /// assert_eq!(NoticeKind::Halt.as_str(), "halt");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            NoticeKind::Warning => "warning",
            NoticeKind::Halt => "halt",
        }
    }
}

/// A once-per-window notice this admission won the right to emit (41-06).
///
/// Carries everything `AllowanceAdmissionPort::confirm` needs to deliver the operator notice
/// without re-reading the notice store (41-08): the warning, the tenant and (for an API-key
/// scope) the key name, the admitting run and the store instant the notice was recorded at.
/// Tenant ids and key names are log-safe identifiers; no key value ever appears here
/// (D-00g).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowanceNotice {
    /// Stable identifier of the notice row.
    pub notice_id: String,
    /// The tenant the notice was recorded for.
    pub tenant_id: String,
    /// The API key name; `Some` exactly for an API-key-scope notice.
    pub api_key_id: Option<String>,
    /// The warning the notice carries.
    pub warning: AllowanceWarning,
    /// The run whose admission won the notice; `None` on the HTTP agent path (no run row).
    pub run_id: Option<RunId>,
    /// The store instant (whole seconds) the notice was recorded at.
    pub recorded_at: DateTime<Utc>,
    /// Whether this is a warn-threshold notice or a spend-halt notice (Phase 42 D-18).
    #[serde(default)]
    pub kind: NoticeKind,
}

impl From<&NoticeRecord> for AllowanceNotice {
    fn from(record: &NoticeRecord) -> Self {
        Self {
            notice_id: record.notice_id.clone(),
            tenant_id: record.tenant_id.clone(),
            api_key_id: record.api_key_id.clone(),
            warning: record.warning.clone(),
            run_id: record.run_id.clone(),
            recorded_at: record.recorded_at,
            kind: record.kind,
        }
    }
}

/// The storage key of a lifetime notice's window: the Unix epoch.
///
/// A lifetime ceiling has no window, but the notice's unique identity needs a non-null
/// `window_start` (a NULL in a unique key never conflicts, C5), so lifetime notices are stored
/// with this fixed instant and read back with both window bounds `None`.
pub const LIFETIME_WINDOW_START: DateTime<Utc> = DateTime::<Utc>::UNIX_EPOCH;

/// Whether a balance has crossed the warn threshold of a ceiling (D-15).
///
/// `true` when `balance_nanos` is at least `warn_at` percent of `ceiling_nanos`; `warn_at == 0`
/// never crosses (the feature is off). The comparison is
/// `balance * 100 >= ceiling * warn_at` in `i128` -- an `i64` times a percent can never overflow
/// `i128`, so no checked arithmetic is needed -- with no floating point, so it is exact at every
/// `i64` nano-unit ceiling. Admission refuses a balance at the ceiling, so `warn_at == 100` can never
/// be observed on an admitted path.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::allowance::crosses_warn_threshold;
///
/// assert!(crosses_warn_threshold(80, 100, 80));
/// assert!(!crosses_warn_threshold(79, 100, 80));
/// assert!(!crosses_warn_threshold(99, 100, 0));
/// ```
pub fn crosses_warn_threshold(balance_nanos: i64, ceiling_nanos: i64, warn_at: u8) -> bool {
    if warn_at == 0 {
        return false;
    }
    i128::from(balance_nanos) * 100 >= i128::from(ceiling_nanos) * i128::from(warn_at)
}

/// The result of recording a notice claim.
///
/// Losing a claim is never an error: another admission (possibly on another replica) already
/// recorded this notice's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoticeOutcome {
    /// This claim won: the row now exists and the caller owns the emission.
    Recorded,
    /// The identity was already recorded; nothing was written.
    AlreadyRecorded,
}

/// A durable once-per-window notice row (D-16).
///
/// The identity is
/// `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling, kind)`:
/// a raised ceiling re-arms the notice for the same window, a new window is a new notice, a
/// warning and a halt notice are distinct, and changing `warn_at` alone does not re-arm. `api_key_id` is `Some` exactly for API-key scope;
/// `run_id` is the admitting run (`None` on the HTTP agent path).
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use paladin_core::platform::container::allowance::{
///     AllowanceLimitKind, AllowanceNotice, AllowanceScopeKind, AllowanceWarning, NoticeKind,
///     NoticeRecord,
/// };
/// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
///
/// let usd = CurrencyCode::new("USD")?;
/// let record = NoticeRecord {
///     notice_id: "n-1".to_string(),
///     tenant_id: "acme".to_string(),
///     api_key_id: None,
///     warning: AllowanceWarning {
///         scope_kind: AllowanceScopeKind::Tenant,
///         limit_kind: AllowanceLimitKind::Lifetime,
///         balance: Cost::new(80, usd.clone()),
///         ceiling: Cost::new(100, usd),
///         window_start: None,
///         window_end: None,
///         warn_at: 80,
///     },
///     run_id: None,
///     recorded_at: Utc.timestamp_opt(0, 0).single().ok_or("bad instant")?,
///     kind: NoticeKind::Warning,
/// };
/// assert_eq!(AllowanceNotice::from(&record).notice_id, "n-1");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoticeRecord {
    /// Stable identifier of the notice row (a fresh UUIDv7 string per claim attempt).
    pub notice_id: String,
    /// The tenant the notice is held against.
    pub tenant_id: String,
    /// The API key name; `Some` exactly for API-key scope.
    pub api_key_id: Option<String>,
    /// The warning the notice carries (pre-admission balance, ceiling, window, threshold).
    pub warning: AllowanceWarning,
    /// The admitting run; `None` on the HTTP agent path.
    pub run_id: Option<RunId>,
    /// The store instant the claim was made at.
    pub recorded_at: DateTime<Utc>,
    /// Whether this is a warn-threshold notice or a spend-halt notice; part of the
    /// once-per-window identity (Phase 42 D-18, G5).
    #[serde(default)]
    pub kind: NoticeKind,
}

/// The per-run token budget the Treasurer derives from a principal's remaining allowance
/// (ALLOW-05, Phase 42 D-09).
///
/// `max_tokens` is the largest total token count the run may accumulate before the one
/// `TokenBudget` cutoff ends it (`floor(remaining * 1_000_000 / dearest price)`, integer
/// arithmetic only). `halt_figures` is what the cutoff reports when this budget is the binding
/// limit: the binding ceiling with its `balance` set to the ceiling itself -- a conservative
/// bound, because the agent loop cannot know the exact post-spend balance (ADR-0057 A5).
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use paladin_core::platform::container::allowance::{
///     AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind, DerivedTokenBudget,
/// };
/// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
///
/// let usd = CurrencyCode::new("USD")?;
/// let evaluated_at = Utc.with_ymd_and_hms(2026, 10, 3, 23, 0, 0).single().ok_or("bad instant")?;
/// let figures = AllowanceRefusal {
///     scope_kind: AllowanceScopeKind::Tenant,
///     limit_kind: AllowanceLimitKind::Lifetime,
///     balance: Cost::new(1_000_000_000, usd.clone()),
///     ceiling: Cost::new(1_000_000_000, usd),
///     window: None,
///     evaluated_at,
/// };
/// let budget = DerivedTokenBudget::new(100_000, figures.clone());
/// assert_eq!(budget.max_tokens, 100_000);
/// assert_eq!(budget.halt_figures, figures);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DerivedTokenBudget {
    /// The most total tokens the run may accumulate before the cutoff ends it.
    pub max_tokens: u32,
    /// The figures reported when this budget ends the run (balance equal to the ceiling).
    pub halt_figures: AllowanceRefusal,
}

impl DerivedTokenBudget {
    /// Build a budget of `max_tokens` that reports `halt_figures` when it ends a run.
    pub fn new(max_tokens: u32, halt_figures: AllowanceRefusal) -> Self {
        Self {
            max_tokens,
            halt_figures,
        }
    }
}

/// The outcome of a successful admission: the notices (if any) it won, and, when the model
/// is known and the principal has a ceiling, the per-run token budget derived from what remains
/// of the allowance.
///
/// An admission that crossed no warn threshold and derived no budget is [`Admission::none`].
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use paladin_core::platform::container::allowance::{
///     Admission, AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind, DerivedTokenBudget,
/// };
/// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
///
/// let admission = Admission::none();
/// assert!(admission.is_empty());
/// assert_eq!(admission.warnings().count(), 0);
/// assert!(admission.derived_budget().is_none());
///
/// let usd = CurrencyCode::new("USD")?;
/// let evaluated_at = Utc.with_ymd_and_hms(2026, 10, 3, 23, 0, 0).single().ok_or("bad instant")?;
/// let figures = AllowanceRefusal {
///     scope_kind: AllowanceScopeKind::ApiKey,
///     limit_kind: AllowanceLimitKind::Lifetime,
///     balance: Cost::new(500, usd.clone()),
///     ceiling: Cost::new(500, usd),
///     window: None,
///     evaluated_at,
/// };
/// let admission = Admission::none().with_derived_budget(DerivedTokenBudget::new(42, figures));
/// assert_eq!(admission.derived_budget().map(|b| b.max_tokens), Some(42));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Admission {
    notices: Vec<AllowanceNotice>,
    derived_budget: Option<DerivedTokenBudget>,
}

impl Admission {
    /// An admission that won no notice and derived no budget.
    pub fn none() -> Self {
        Self::default()
    }

    /// Add a won notice.
    pub fn with_notice(mut self, notice: AllowanceNotice) -> Self {
        self.notices.push(notice);
        self
    }

    /// Attach the per-run token budget derived for this admission (ALLOW-05, D-09).
    #[must_use]
    pub fn with_derived_budget(mut self, budget: DerivedTokenBudget) -> Self {
        self.derived_budget = Some(budget);
        self
    }

    /// The notices this admission won.
    pub fn notices(&self) -> &[AllowanceNotice] {
        &self.notices
    }

    /// The per-run token budget derived for this admission, when one was.
    pub fn derived_budget(&self) -> Option<&DerivedTokenBudget> {
        self.derived_budget.as_ref()
    }

    /// The warnings carried by the won notices.
    pub fn warnings(&self) -> impl Iterator<Item = &AllowanceWarning> {
        self.notices.iter().map(|n| &n.warning)
    }

    /// Whether this admission won no notice. (A derived budget does not make an admission
    /// non-empty: `is_empty` answers only "are there notices to confirm or abandon".)
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
    fn derived_budget_is_carried_by_an_admission_and_does_not_make_it_non_empty() {
        let figures = window_refusal(at(3, 12, 0, 0));
        let budget = DerivedTokenBudget::new(77, figures.clone());
        assert_eq!(budget.max_tokens, 77);
        assert_eq!(budget.halt_figures, figures);

        assert!(Admission::none().derived_budget().is_none());
        let admission = Admission::none().with_derived_budget(budget.clone());
        assert_eq!(admission.derived_budget(), Some(&budget));
        // is_empty answers only "no notices to confirm or abandon".
        assert!(admission.is_empty());
    }

    #[test]
    fn derived_budget_round_trips_through_serde() {
        let budget = DerivedTokenBudget::new(u32::MAX, window_refusal(at(3, 12, 0, 0)));
        let json = serde_json::to_string(&budget).unwrap();
        let back: DerivedTokenBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(back, budget);
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
            tenant_id: "acme".to_string(),
            api_key_id: Some("svc-a".to_string()),
            warning: warning.clone(),
            run_id: None,
            recorded_at: at(3, 12, 0, 0),
            kind: NoticeKind::Warning,
        });
        assert!(!admission.is_empty());
        assert_eq!(admission.warnings().next(), Some(&warning));
    }

    fn warning(
        balance: i64,
        ceiling: i64,
        scope: AllowanceScopeKind,
        limit: AllowanceLimitKind,
    ) -> AllowanceWarning {
        let windowed = limit == AllowanceLimitKind::Window;
        AllowanceWarning {
            scope_kind: scope,
            limit_kind: limit,
            balance: Cost::new(balance, usd()),
            ceiling: Cost::new(ceiling, usd()),
            window_start: windowed.then(|| at(2, 0, 0, 0)),
            window_end: windowed.then(|| at(3, 0, 0, 0)),
            warn_at: 80,
        }
    }

    #[test]
    fn herald_line_for_a_window_ceiling() {
        let w = warning(
            20_500_000_000,
            25_000_000_000,
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
        );
        assert_eq!(
            w.herald_line(),
            "\u{26A0} allowance: 82% of 25.0000 USD (api_key, window resets 2026-10-03T00:00:00Z)"
        );
    }

    /// IN-4: a window warning with no window end must not claim a lifetime cap.
    #[test]
    fn herald_line_for_a_window_ceiling_without_a_window_end() {
        let mut w = warning(
            20_500_000_000,
            25_000_000_000,
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
        );
        w.window_end = None;
        assert_eq!(
            w.herald_line(),
            "\u{26A0} allowance: 82% of 25.0000 USD (api_key, window)"
        );
    }

    #[test]
    fn herald_line_for_a_lifetime_ceiling() {
        let w = warning(
            20_500_000_000,
            25_000_000_000,
            AllowanceScopeKind::Tenant,
            AllowanceLimitKind::Lifetime,
        );
        assert_eq!(
            w.herald_line(),
            "\u{26A0} allowance: 82% of 25.0000 USD (tenant, lifetime cap)"
        );
    }

    #[test]
    fn percent_floors_and_never_rounds_up() {
        let w = warning(
            829_999_999,
            1_000_000_000,
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
        );
        assert_eq!(w.percent_of_ceiling(), 82);
        let w = warning(
            830_000_000,
            1_000_000_000,
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
        );
        assert_eq!(w.percent_of_ceiling(), 83);
    }

    #[test]
    fn percent_is_zero_for_a_non_positive_ceiling_or_negative_balance_and_exact_at_the_extremes() {
        let w = warning(
            5,
            0,
            AllowanceScopeKind::Tenant,
            AllowanceLimitKind::Lifetime,
        );
        assert_eq!(w.percent_of_ceiling(), 0);
        let w = warning(
            -5,
            100,
            AllowanceScopeKind::Tenant,
            AllowanceLimitKind::Lifetime,
        );
        assert_eq!(w.percent_of_ceiling(), 0);
        let w = warning(
            i64::MAX,
            i64::MAX,
            AllowanceScopeKind::Tenant,
            AllowanceLimitKind::Lifetime,
        );
        assert_eq!(w.percent_of_ceiling(), 100);
    }

    #[test]
    fn herald_line_names_no_tenant_or_key() {
        let w = warning(
            80,
            100,
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
        );
        let line = w.herald_line();
        assert!(!line.contains("acme") && !line.contains("svc-a"), "{line}");
    }

    #[test]
    fn from_trace_event_inverts_the_conversion_and_ignores_other_variants() {
        let w = warning(
            80,
            100,
            AllowanceScopeKind::ApiKey,
            AllowanceLimitKind::Window,
        );
        let event = TraceEvent::from(w.clone());
        assert_eq!(AllowanceWarning::from_trace_event(&event), Some(w));
        let other = TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        };
        assert_eq!(AllowanceWarning::from_trace_event(&other), None);
    }

    #[test]
    fn crossing_is_inclusive_at_the_threshold() {
        assert!(crosses_warn_threshold(80, 100, 80));
        assert!(crosses_warn_threshold(81, 100, 80));
        assert!(crosses_warn_threshold(2_000_000_000, 2_500_000_000, 80));
    }

    #[test]
    fn one_nano_below_the_threshold_does_not_cross() {
        assert!(!crosses_warn_threshold(79, 100, 80));
        assert!(!crosses_warn_threshold(1_999_999_999, 2_500_000_000, 80));
    }

    #[test]
    fn warn_at_zero_never_crosses() {
        assert!(!crosses_warn_threshold(0, 100, 0));
        assert!(!crosses_warn_threshold(99, 100, 0));
        assert!(!crosses_warn_threshold(i64::MAX, i64::MAX, 0));
    }

    #[test]
    fn crossing_math_is_exact_at_i64_max() {
        // 80% of i64::MAX is not an integer; the exact boundary is the smallest balance whose
        // x100 product reaches ceiling x 80, found by integer search around the real value.
        let ceiling = i64::MAX;
        let boundary = (i128::from(ceiling) * 80 + 99) / 100; // ceil(ceiling * 0.8)
        let boundary = i64::try_from(boundary).expect("80% of i64::MAX fits i64");
        assert!(crosses_warn_threshold(boundary, ceiling, 80));
        assert!(!crosses_warn_threshold(boundary - 1, ceiling, 80));
        // A balance equal to the ceiling crosses every non-zero threshold without overflow.
        assert!(crosses_warn_threshold(ceiling, ceiling, 100));
        assert!(crosses_warn_threshold(ceiling, ceiling, 1));
    }

    #[test]
    fn lifetime_window_start_is_the_unix_epoch() {
        assert_eq!(LIFETIME_WINDOW_START.timestamp(), 0);
    }

    #[test]
    fn notice_record_converts_to_a_notice_losslessly() {
        let record = NoticeRecord {
            notice_id: "n-9".to_string(),
            tenant_id: "acme".to_string(),
            api_key_id: Some("svc-a".to_string()),
            warning: AllowanceWarning {
                scope_kind: AllowanceScopeKind::ApiKey,
                limit_kind: AllowanceLimitKind::Window,
                balance: Cost::new(80, usd()),
                ceiling: Cost::new(100, usd()),
                window_start: Some(at(3, 0, 0, 0)),
                window_end: Some(at(4, 0, 0, 0)),
                warn_at: 80,
            },
            run_id: Some(RunId::new_v7()),
            recorded_at: at(3, 12, 0, 0),
            kind: NoticeKind::Halt,
        };
        let notice = AllowanceNotice::from(&record);
        assert_eq!(notice.notice_id, record.notice_id);
        assert_eq!(notice.tenant_id, record.tenant_id);
        assert_eq!(notice.api_key_id, record.api_key_id);
        assert_eq!(notice.warning, record.warning);
        assert_eq!(notice.run_id, record.run_id);
        assert_eq!(notice.recorded_at, record.recorded_at);
        assert_eq!(notice.kind, NoticeKind::Halt);
        let json = serde_json::to_string(&record).unwrap();
        let back: NoticeRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn notice_kind_defaults_to_warning_and_round_trips_snake_case() {
        assert_eq!(NoticeKind::default(), NoticeKind::Warning);
        assert_eq!(
            serde_json::to_string(&NoticeKind::Halt).unwrap(),
            "\"halt\""
        );
        for kind in [NoticeKind::Warning, NoticeKind::Halt] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
            assert_eq!(serde_json::from_str::<NoticeKind>(&json).unwrap(), kind);
        }
    }

    #[test]
    fn a_notice_record_without_a_kind_deserializes_as_a_warning() {
        let record = NoticeRecord {
            notice_id: "n-legacy".to_string(),
            tenant_id: "acme".to_string(),
            api_key_id: None,
            warning: AllowanceWarning {
                scope_kind: AllowanceScopeKind::Tenant,
                limit_kind: AllowanceLimitKind::Lifetime,
                balance: Cost::new(80, usd()),
                ceiling: Cost::new(100, usd()),
                window_start: None,
                window_end: None,
                warn_at: 80,
            },
            run_id: None,
            recorded_at: at(3, 12, 0, 0),
            kind: NoticeKind::Halt,
        };
        let mut value = serde_json::to_value(&record).unwrap();
        value.as_object_mut().unwrap().remove("kind");
        let back: NoticeRecord = serde_json::from_value(value).unwrap();
        assert_eq!(back.kind, NoticeKind::Warning);
    }

    #[test]
    fn halt_reason_allowance_exhausted_wire_json_equals_reason_plus_the_429_details() {
        let refusal = window_refusal(at(3, 23, 0, 0));
        let reason = HaltReason::AllowanceExhausted(refusal.clone());
        let wire = reason.wire_json();
        let object = wire.as_object().expect("wire form is an object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "balance",
                "ceiling",
                "kind",
                "reason",
                "scope",
                "window_end",
                "window_start"
            ]
        );
        assert_eq!(object["reason"], "allowance_exhausted");
        let details = refusal.details_json();
        let details = details.as_object().expect("details is an object");
        for (key, value) in details {
            assert_eq!(&object[key], value, "key {key} must equal the 429 details");
        }
        assert_eq!(details.len(), 6);
    }

    #[test]
    fn details_json_renders_a_lifetime_refusal_with_null_window_bounds() {
        let mut refusal = window_refusal(at(3, 23, 0, 0));
        refusal.limit_kind = AllowanceLimitKind::Lifetime;
        refusal.window = None;
        let details = refusal.details_json();
        assert_eq!(details["kind"], "lifetime");
        assert!(details["window_start"].is_null());
        assert!(details["window_end"].is_null());
        assert_eq!(details["balance"], "2.5000 USD");
    }

    #[test]
    fn halt_reason_ledger_unavailable_wire_json_is_reason_only() {
        assert_eq!(
            HaltReason::LedgerUnavailable.wire_json(),
            serde_json::json!({"reason": "ledger_unavailable"})
        );
    }

    #[test]
    fn halt_reason_herald_line_for_a_window_ceiling() {
        let reason = HaltReason::AllowanceExhausted(window_refusal(at(3, 23, 0, 0)));
        assert_eq!(
            reason.herald_line(),
            "\u{26D4} halted: allowance exhausted \u{2014} 2.5000 of 2.5000 USD \
             (api_key, window resets 2026-10-04T00:00:00Z)"
        );
    }

    /// IN-4: a window refusal with no window must not claim a lifetime cap.
    #[test]
    fn halt_reason_herald_line_for_a_window_ceiling_without_a_window() {
        let mut refusal = window_refusal(at(3, 23, 0, 0));
        refusal.window = None;
        let line = HaltReason::AllowanceExhausted(refusal).herald_line();
        assert!(line.ends_with("(api_key, window)"), "{line}");
        assert!(!line.contains("lifetime"), "{line}");
    }

    #[test]
    fn halt_reason_herald_line_for_a_lifetime_ceiling() {
        let mut refusal = window_refusal(at(3, 23, 0, 0));
        refusal.scope_kind = AllowanceScopeKind::Tenant;
        refusal.limit_kind = AllowanceLimitKind::Lifetime;
        refusal.window = None;
        let line = HaltReason::AllowanceExhausted(refusal).herald_line();
        assert!(line.ends_with("(tenant, lifetime cap)"), "{line}");
    }

    #[test]
    fn halt_reason_herald_line_for_a_ledger_outage() {
        assert_eq!(
            HaltReason::LedgerUnavailable.herald_line(),
            "\u{26D4} halted: allowance could not be evaluated (ledger unavailable)"
        );
    }

    #[test]
    fn halt_reason_herald_line_names_no_tenant_or_key() {
        let line = HaltReason::AllowanceExhausted(window_refusal(at(3, 23, 0, 0))).herald_line();
        assert!(!line.contains("acme") && !line.contains("svc-a"), "{line}");
    }

    #[test]
    fn halt_reason_as_str_names_the_two_tags() {
        let exhausted = HaltReason::AllowanceExhausted(window_refusal(at(3, 23, 0, 0)));
        assert_eq!(exhausted.as_str(), "allowance_exhausted");
        assert_eq!(HaltReason::LedgerUnavailable.as_str(), "ledger_unavailable");
    }

    #[test]
    fn halt_reason_round_trips_through_serde_with_a_reason_tag() {
        let exhausted = HaltReason::AllowanceExhausted(window_refusal(at(3, 23, 0, 0)));
        let json = serde_json::to_value(&exhausted).unwrap();
        assert_eq!(json["reason"], "allowance_exhausted");
        // Integer nanos plus currency: the persisted form never holds a display string.
        assert_eq!(json["balance"]["nanos"], 2_500_000_000_i64);
        assert_eq!(json["balance"]["currency"], "USD");
        let back: HaltReason = serde_json::from_value(json).unwrap();
        assert_eq!(back, exhausted);

        let unavailable = serde_json::to_value(HaltReason::LedgerUnavailable).unwrap();
        assert_eq!(
            unavailable,
            serde_json::json!({"reason": "ledger_unavailable"})
        );
        let back: HaltReason = serde_json::from_value(unavailable).unwrap();
        assert_eq!(back, HaltReason::LedgerUnavailable);
    }
}
