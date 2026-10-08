//! `TreasurerConfig` -- the operator-configured per-model price table (PRICE-01, D-07).
//!
//! Mirrors [`crate::config::agent_runtime`]'s config idiom field-for-field: `Default` +
//! `validate()` + [`EnvOverridable`]. Omitting the `treasurer:` section from a config file
//! changes nothing -- [`TreasurerConfig::default()`] is an empty pricing table in `"USD"`, so a
//! server with no `treasurer:` key boots identically to one without this module (D-00a, D-00h).
//!
//! Prices are written as **decimal strings, in nano-units per 1M tokens** (D-01) -- an operator
//! copies a provider's published price-sheet figure verbatim, e.g. `"2.50"` for $2.50 per 1M
//! prompt tokens. [`TreasurerConfig::validate`] never rounds, clamps or silently reinterprets a
//! price: a string that is not exactly one or more ASCII digits, optionally followed by `.` and
//! one to nine ASCII digits, is rejected outright, naming the offending model and axis.
//!
//! A [`PriceRowConfig`] requires `prompt` and `completion`; `cache_read`, `cache_write` and
//! `reasoning` are optional and, when omitted, bill at the `prompt`/`completion` price
//! respectively (D-06) -- exactly `PriceRow`'s own fallback rule.
//!
//! The `pricing` map is **config-file only** -- there is deliberately no environment-variable
//! form for a collection-shaped field, mirroring
//! [`crate::config::agent_runtime::ToolCallLimitConfig::per_tool`]. Only `treasurer.currency`
//! has an env override (`APP_TREASURER_CURRENCY`), applied verbatim with no case-folding --
//! [`TreasurerConfig::validate`] rejects a lowercase override exactly as it would a lowercase
//! file value.
//!
//! No pricing field is secret-shaped: prices and an ISO currency code carry no credential.
//!
//! Phase 41 adds the `allowance` subtree (ALLOW-01, D-02): operator-configured rolling-window
//! allowances, `treasurer.allowance.tenants.<id>` and `treasurer.allowance.api_keys.<name>`,
//! each an entry with a required `period` (`<integer><m|h|d>`, 1m to 366d) and `amount` (a
//! decimal string in whole currency units) and an optional `lifetime` cap and `warn_at` percent.
//! A global `warn_at` (default 80) and an optional operator `webhook { url, secret }` sit beside
//! the maps. Omitted, the subtree is inert. Every struct under `treasurer:` rejects an unknown
//! key, so a typo such as `allowence:` fails to load rather than enforcing nothing, and
//! [`AllowanceConfig::validate_against`] stops a boot whose entries name no configured key or
//! tenant or have no run store to enforce against (D-11). The webhook `secret` is the one
//! credential-shaped field in this tree: it is redacted from `Debug`, skipped by `Serialize`,
//! and supplied through `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` (a `${VAR}` placeholder in a
//! YAML file is not expanded by the loader).
//!
//! Phase 43 adds the `cadence` subtree (PACE-02, D-08, D-10): rate pacing, **on by default**.
//! After a provider answers `429`, later calls to that provider and model wait out a gate
//! (exponential back-off with full jitter, `base_backoff_ms` to `max_backoff_ms`). Omitting the
//! subtree therefore turns pacing on; `treasurer.cadence.enabled: false` installs nothing and
//! restores the previous behaviour. Like every struct under `treasurer:`, [`CadenceConfig`]
//! rejects an unknown key, so a typo fails to load rather than silently disabling pacing, and
//! [`CadenceConfig::validate`] names the full key of whatever it rejects.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::application::services::treasurer::{AllowancePolicy, ScopeAllowance};
use crate::config::env_utils::{EnvOverridable, read_env};
use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};
use paladin_core::platform::container::principal::TenantId;
use paladin_llm::cadence::CadenceSettings;
use paladin_ports::output::cadence_port::CadencePolicy;

/// The default global warn threshold, in whole percent of a ceiling (`treasurer.allowance.warn_at`).
pub const DEFAULT_ALLOWANCE_WARN_AT: u8 = 80;

/// The longest allowance period, in seconds (366 days). A longer period is rejected, never
/// clamped (D-00d).
pub const MAX_ALLOWANCE_PERIOD_SECS: u64 = 31_622_400;

/// The shortest allowance period, in seconds (one minute).
const MIN_ALLOWANCE_PERIOD_SECS: u64 = 60;

/// Where the pacing state lives (`treasurer.cadence.backend`).
///
/// Only the in-process backend exists so far (YAML `backend: in_process`); the shared Redis
/// backend arrives with plan 43-09.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CadenceBackend {
    /// Gate state held in this process (`InMemoryCadence`), the default (D-08).
    InProcess,
}

/// The `treasurer.cadence` subtree: rate pacing (PACE-02, D-08, D-10).
///
/// On by default. Every duration is a positive integer; [`CadenceConfig::validate`] rejects zero
/// and nonsense rather than clamping it.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::CadenceConfig;
///
/// let config = CadenceConfig::default();
/// assert!(config.enabled);
/// assert_eq!(config.base_backoff_ms, 500);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CadenceConfig {
    /// Whether pacing is installed at all. `false` installs nothing (default `true`).
    pub enabled: bool,
    /// Where the gate state lives (default `in_process`).
    pub backend: CadenceBackend,
    /// The gate length after the first delay-less 429, in milliseconds (default 500).
    pub base_backoff_ms: u64,
    /// The largest delay-less gate, in milliseconds (default 30000).
    pub max_backoff_ms: u64,
    /// The longest a single call waits on a gate before surfacing a rate limit, in seconds
    /// (default 300).
    pub max_wait_secs: u64,
    /// The factor applied to delays while a shared backend is degraded (default 2.0, at least 1).
    pub degraded_multiplier: f64,
    /// How long a fallback hop is paced before the chain moves on, in seconds (default 60).
    pub fallback_pace_budget_secs: u64,
    /// The cache stampede lock lifetime, in seconds (default 120): long enough to outlive a
    /// typical LLM node including its retries; no lock renewal is built.
    pub lock_ttl_secs: u64,
}

impl Default for CadenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: CadenceBackend::InProcess,
            base_backoff_ms: 500,
            max_backoff_ms: 30_000,
            max_wait_secs: 300,
            degraded_multiplier: 2.0,
            fallback_pace_budget_secs: 60,
            lock_ttl_secs: 120,
        }
    }
}

impl CadenceConfig {
    /// Reject a configuration that cannot pace anything, naming the full key.
    ///
    /// # Errors
    ///
    /// A `String` naming the first offending `treasurer.cadence.*` key: a zero duration,
    /// `base_backoff_ms` above `max_backoff_ms`, or a `degraded_multiplier` that is not finite
    /// or is below 1.0.
    pub fn validate(&self) -> Result<(), String> {
        for (key, value) in [
            ("base_backoff_ms", self.base_backoff_ms),
            ("max_backoff_ms", self.max_backoff_ms),
            ("max_wait_secs", self.max_wait_secs),
            ("fallback_pace_budget_secs", self.fallback_pace_budget_secs),
            ("lock_ttl_secs", self.lock_ttl_secs),
        ] {
            if value == 0 {
                return Err(format!(
                    "treasurer.cadence.{key} must be greater than zero (got 0)"
                ));
            }
        }
        if self.base_backoff_ms > self.max_backoff_ms {
            return Err(format!(
                "treasurer.cadence.base_backoff_ms ({}) must not exceed \
                 treasurer.cadence.max_backoff_ms ({})",
                self.base_backoff_ms, self.max_backoff_ms
            ));
        }
        if !self.degraded_multiplier.is_finite() || self.degraded_multiplier < 1.0 {
            return Err(format!(
                "treasurer.cadence.degraded_multiplier must be a finite number of at least 1.0 \
                 (got {})",
                self.degraded_multiplier
            ));
        }
        Ok(())
    }

    /// The back-off policy this configuration describes.
    ///
    /// # Errors
    ///
    /// A `String` naming the offending key, as [`CadenceConfig::validate`].
    pub fn policy(&self) -> Result<CadencePolicy, String> {
        self.validate()?;
        CadencePolicy::new(
            std::time::Duration::from_millis(self.base_backoff_ms),
            std::time::Duration::from_millis(self.max_backoff_ms),
        )
        .map_err(|e| format!("treasurer.cadence: {e}"))
    }

    /// The decorator settings this configuration describes.
    pub fn settings(&self) -> CadenceSettings {
        CadenceSettings::new(
            std::time::Duration::from_secs(self.max_wait_secs),
            std::time::Duration::from_millis(self.max_backoff_ms),
            std::time::Duration::from_secs(self.fallback_pace_budget_secs),
        )
    }
}

/// The largest nano-units-per-1M-tokens price representable in a [`PriceRowConfig`] axis, as a
/// decimal string, for use in error messages (`i64::MAX` nano-units = `9223372036.854775807`).
const MAX_PRICE_DISPLAY: &str = "9223372036.854775807";

/// Errors constructing an `i64` nano-unit figure from an operator-entered decimal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PriceParseError {
    /// Not the narrow grammar: ASCII digits, optionally `.` followed by 1-9 ASCII digits. No
    /// sign, whitespace, exponent, separator or `NaN`/`inf` spelling is accepted.
    Malformed,
    /// More than nine digits after the decimal point -- finer than the nano-unit-per-1M scale
    /// this module represents exactly.
    TooManyDecimalPlaces,
    /// The value does not fit in `i64` nano-units.
    Overflow,
}

/// Parse a decimal string into `i64` nano-units with exact integer arithmetic -- one grammar for
/// both a per-1M-token price (D-01) and an allowance amount in whole currency units, where
/// `"25.00"` is `25_000_000_000` nano-units.
///
/// Accepts exactly one or more ASCII digits, optionally followed by `.` and one to nine ASCII
/// digits -- no sign, whitespace, exponent, thousands separator or `NaN`/`inf` spelling. No
/// floating point is used anywhere in this function; the integer part is accumulated with
/// `checked_mul`/`checked_add`, and the fractional digits are right-padded to nine places and
/// added as a nano-unit remainder.
fn parse_decimal_nanos(raw: &str) -> Result<i64, PriceParseError> {
    let mut parts = raw.splitn(2, '.');
    let int_part = parts.next().unwrap_or_default();
    let frac_part = parts.next();

    if int_part.is_empty() || !int_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PriceParseError::Malformed);
    }

    let mut int_value: i64 = 0;
    for b in int_part.bytes() {
        let digit = i64::from(b - b'0');
        int_value = int_value
            .checked_mul(10)
            .ok_or(PriceParseError::Overflow)?
            .checked_add(digit)
            .ok_or(PriceParseError::Overflow)?;
    }

    let frac_digits = match frac_part {
        None => String::new(),
        Some(fp) => {
            if fp.is_empty() || !fp.bytes().all(|b| b.is_ascii_digit()) {
                return Err(PriceParseError::Malformed);
            }
            if fp.len() > 9 {
                return Err(PriceParseError::TooManyDecimalPlaces);
            }
            fp.to_string()
        }
    };

    // Right-pad to nine digits so `"5"` (tenths) and `"5000000"` (nine places already) both
    // resolve to the same nano-unit remainder. `padded` is always exactly nine ASCII digits by
    // construction, so this cannot fail to parse.
    let padded = format!("{frac_digits:0<9}");
    let frac_value: i64 = padded.parse().map_err(|_| PriceParseError::Malformed)?;

    let scaled_int = int_value
        .checked_mul(1_000_000_000)
        .ok_or(PriceParseError::Overflow)?;
    scaled_int
        .checked_add(frac_value)
        .ok_or(PriceParseError::Overflow)
}

/// Errors constructing an allowance period from an operator-entered string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeriodParseError {
    /// Not `<digits><m|h|d>`: empty, signed, padded with whitespace, decimal, an upper-case or
    /// unknown unit (including `s` and `w`).
    Malformed,
    /// The period does not fit in `u64` seconds, or is outside `1m..=366d`.
    OutOfRange,
}

/// Parse an allowance period: one or more ASCII digits followed by exactly one unit byte `m`
/// (60 s), `h` (3600 s) or `d` (86400 s), with checked arithmetic. Never trims, never clamps;
/// anything below 60 seconds or above [`MAX_ALLOWANCE_PERIOD_SECS`] is rejected.
fn parse_period_secs(raw: &str) -> Result<u64, PeriodParseError> {
    let unit_secs: u64 = match raw.as_bytes().last() {
        Some(b'm') => 60,
        Some(b'h') => 3_600,
        Some(b'd') => 86_400,
        _ => return Err(PeriodParseError::Malformed),
    };
    // The last byte is an ASCII unit, so this slice always ends on a char boundary.
    let digits = &raw[..raw.len() - 1];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PeriodParseError::Malformed);
    }
    let mut count: u64 = 0;
    for b in digits.bytes() {
        count = count
            .checked_mul(10)
            .and_then(|c| c.checked_add(u64::from(b - b'0')))
            .ok_or(PeriodParseError::OutOfRange)?;
    }
    let secs = count
        .checked_mul(unit_secs)
        .ok_or(PeriodParseError::OutOfRange)?;
    if !(MIN_ALLOWANCE_PERIOD_SECS..=MAX_ALLOWANCE_PERIOD_SECS).contains(&secs) {
        return Err(PeriodParseError::OutOfRange);
    }
    Ok(secs)
}

/// One scope's allowance entry as operator-entered strings (ALLOW-01, D-02).
///
/// `period` and `amount` are required by serde -- a lifetime-only entry is not expressible, so
/// every entry carries a window ceiling. `lifetime` (a cumulative cap since the ledger began)
/// and `warn_at` (a per-entry override of the global warn threshold) are optional. An unknown
/// key is a load error.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::AllowanceEntryConfig;
///
/// let entry = AllowanceEntryConfig {
///     period: "1d".to_string(),
///     amount: "2.50".to_string(),
///     lifetime: Some("100.00".to_string()),
///     warn_at: None,
/// };
/// assert_eq!(entry.period, "1d");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowanceEntryConfig {
    /// The window period as `<integer><m|h|d>`, between `1m` and `366d` (e.g. `"1d"`).
    pub period: String,
    /// The window ceiling as a decimal string in whole currency units (e.g. `"2.50"`).
    pub amount: String,
    /// An optional lifetime cap as a decimal string in whole currency units: the most the scope
    /// may ever spend, however many windows pass.
    #[serde(default)]
    pub lifetime: Option<String>,
    /// An optional per-entry warn threshold, an integer percent `0..=100`, overriding the global
    /// `treasurer.allowance.warn_at`.
    #[serde(default)]
    pub warn_at: Option<u8>,
}

/// The operator webhook target a threshold crossing is delivered to (D-17).
///
/// The `secret` is credential-shaped: it is never rendered by [`std::fmt::Debug`] (the manual
/// impl prints `[redacted]`), never serialised outward (`skip_serializing`), and never echoed in
/// a validation error. A `${VAR}` placeholder in a YAML file is **not** expanded by the config
/// loader, so supply the secret through `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::AllowanceWebhookConfig;
///
/// let hook = AllowanceWebhookConfig {
///     url: "https://ops.example.com/hook".to_string(),
///     secret: Some("s3cr3t-value".to_string()),
/// };
/// assert!(!format!("{hook:?}").contains("s3cr3t-value"));
/// ```
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowanceWebhookConfig {
    /// The `http(s)` URL the Treasurer's notices are posted to. Scheme and address checks belong
    /// to the SSRF guard at wiring time, not to this type.
    pub url: String,
    /// The HMAC signing secret, if any. Never rendered by `Debug` and never serialised.
    #[serde(default, skip_serializing)]
    pub secret: Option<String>,
}

impl std::fmt::Debug for AllowanceWebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AllowanceWebhookConfig")
            .field("url", &self.url)
            .field("secret", &self.secret.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

/// The `treasurer.allowance` subtree (ALLOW-01, D-02): per-tenant and per-API-key rolling-window
/// allowances, an optional global warn threshold and an optional operator webhook target.
/// Omitted, it is inert. The two maps are **config-file only**.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::AllowanceConfig;
///
/// assert!(AllowanceConfig::default().is_empty());
/// assert_eq!(AllowanceConfig::default().warn_at, 80);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AllowanceConfig {
    /// The global warn threshold, an integer percent `0..=100` of a ceiling (`0` disables
    /// warnings). Defaults to [`DEFAULT_ALLOWANCE_WARN_AT`].
    pub warn_at: u8,
    /// The operator webhook target for threshold crossings, if any.
    pub webhook: Option<AllowanceWebhookConfig>,
    /// Tenant id -> allowance shared by every key of that tenant.
    pub tenants: BTreeMap<String, AllowanceEntryConfig>,
    /// API key name -> allowance held by that key.
    pub api_keys: BTreeMap<String, AllowanceEntryConfig>,
}

impl Default for AllowanceConfig {
    fn default() -> Self {
        Self {
            warn_at: DEFAULT_ALLOWANCE_WARN_AT,
            webhook: None,
            tenants: BTreeMap::new(),
            api_keys: BTreeMap::new(),
        }
    }
}

/// The message for an integer percent outside `0..=100`, naming its full config path.
fn warn_at_error(path: &str, got: u8) -> String {
    format!("{path} must be an integer percent between 0 and 100 (got {got})")
}

/// Parse one positive decimal figure (`amount` or `lifetime`), naming `path` on failure.
fn parse_positive_nanos(path: &str, raw: &str) -> Result<i64, String> {
    match parse_decimal_nanos(raw) {
        Ok(nanos) if nanos > 0 => Ok(nanos),
        _ => Err(format!(
            "{path} must be a positive decimal string in whole currency units (digits with an \
             optional fractional part of at most 9 places, at most {MAX_PRICE_DISPLAY}) \
             (got {raw:?})"
        )),
    }
}

impl AllowanceConfig {
    /// Whether no allowance entry is configured. A `webhook` or `warn_at` alone enforces
    /// nothing, so it does not count.
    pub fn is_empty(&self) -> bool {
        self.tenants.is_empty() && self.api_keys.is_empty()
    }

    /// Resolve every entry -- tenants first, then API keys, each in key order -- into an
    /// [`AllowancePolicy`] denominated in `currency`: decimal strings and periods become
    /// integers once, here.
    ///
    /// # Errors
    ///
    /// A `String` naming the full config path and raw value of the first invalid item: a
    /// `warn_at` above 100, an invalid map key, a malformed or zero `amount` or `lifetime`, a
    /// malformed or out-of-range `period`, or an empty webhook `url`. A webhook secret is never
    /// echoed.
    pub fn resolve(&self, currency: &CurrencyCode) -> Result<AllowancePolicy, String> {
        if self.warn_at > 100 {
            return Err(warn_at_error("treasurer.allowance.warn_at", self.warn_at));
        }
        if let Some(webhook) = &self.webhook
            && webhook.url.trim().is_empty()
        {
            return Err("treasurer.allowance.webhook.url must be a non-empty http(s) URL".into());
        }

        let mut policy = AllowancePolicy::new(currency.clone(), self.warn_at);
        for (id, entry) in &self.tenants {
            TenantId::new(id.as_str()).map_err(|e| {
                format!("treasurer.allowance.tenants: {id:?} is not a valid tenant id: {e}")
            })?;
            let allowance = Self::resolve_entry("tenants", id, entry)?;
            policy = policy.with_tenant(id.as_str(), allowance);
        }
        for (name, entry) in &self.api_keys {
            TenantId::new(name.as_str()).map_err(|e| {
                format!("treasurer.allowance.api_keys: {name:?} is not a valid API key name: {e}")
            })?;
            let allowance = Self::resolve_entry("api_keys", name, entry)?;
            policy = policy.with_api_key(name.as_str(), allowance);
        }
        Ok(policy)
    }

    /// Resolve one entry of `map` (`tenants` or `api_keys`) keyed `id`.
    fn resolve_entry(
        map: &str,
        id: &str,
        entry: &AllowanceEntryConfig,
    ) -> Result<ScopeAllowance, String> {
        let base = format!("treasurer.allowance.{map}.{id}");
        let amount = parse_positive_nanos(&format!("{base}.amount"), &entry.amount)?;
        let period = parse_period_secs(&entry.period).map_err(|_| {
            format!(
                "{base}.period must be <integer><m|h|d> between 1m and 366d (got {:?})",
                entry.period
            )
        })?;
        let mut allowance = ScopeAllowance::new(period, amount);
        if let Some(raw) = &entry.lifetime {
            allowance =
                allowance.with_lifetime(parse_positive_nanos(&format!("{base}.lifetime"), raw)?);
        }
        if let Some(percent) = entry.warn_at {
            if percent > 100 {
                return Err(warn_at_error(&format!("{base}.warn_at"), percent));
            }
            allowance = allowance.with_warn_at(percent);
        }
        Ok(allowance)
    }

    /// Cross-check the allowance entries against the rest of the server's configuration (D-11).
    ///
    /// Pure: the caller supplies the tenants and API key names the authentication configuration
    /// knows and whether the run store is disabled.
    ///
    /// # Errors
    ///
    /// A `String` naming the offending path when the allowance has entries and the run store is
    /// disabled (allowances are enforced against the spend ledger, which needs a run store), when
    /// an `api_keys.<name>` entry names no known API key, or when a `tenants.<id>` entry names no
    /// known tenant. An empty allowance is always `Ok`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeSet;
    /// use paladin::config::treasurer::AllowanceConfig;
    ///
    /// let none = BTreeSet::new();
    /// assert!(AllowanceConfig::default().validate_against(&none, &none, true).is_ok());
    /// ```
    pub fn validate_against(
        &self,
        known_tenants: &BTreeSet<String>,
        known_api_keys: &BTreeSet<String>,
        run_store_disabled: bool,
    ) -> Result<(), String> {
        if self.is_empty() {
            return Ok(());
        }
        if run_store_disabled {
            return Err(
                "treasurer.allowance has entries but run_store.backend is disabled -- \
                        allowances are enforced against the spend ledger, which needs a run \
                        store (set run_store.backend)"
                    .to_string(),
            );
        }
        for name in self.api_keys.keys() {
            if !known_api_keys.contains(name) {
                return Err(format!(
                    "treasurer.allowance.api_keys.{name} names no key in http.auth.api_keys -- \
                     a removed key must also lose its allowance entry; its surviving schedules \
                     stay gated by their tenant's allowance"
                ));
            }
        }
        for id in self.tenants.keys() {
            if !known_tenants.contains(id) {
                return Err(format!(
                    "treasurer.allowance.tenants.{id} names a tenant no http.auth.api_keys \
                     entry or http.auth.bearer_token.tenant maps to"
                ));
            }
        }
        Ok(())
    }
}

/// One model's per-1M-token price row, as operator-entered decimal strings (D-01, D-06).
///
/// `prompt` and `completion` are required by serde -- a row missing either fails to load rather
/// than silently billing at a fallback price. `cache_read`, `cache_write` and `reasoning` are
/// optional; when omitted, [`TreasurerConfig::price_table`] bills those tokens at the `prompt`
/// (cache axes) or `completion` (reasoning) price, mirroring `PriceRow`'s own fallback rule. An
/// unrecognized axis name (e.g. a `cache_reads` typo) is a load error (`deny_unknown_fields`),
/// never a silently-ignored key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriceRowConfig {
    /// Price per 1M prompt tokens, as a decimal string (e.g. `"2.50"`).
    pub prompt: String,
    /// Price per 1M completion tokens, as a decimal string (e.g. `"10.00"`).
    pub completion: String,
    /// Price per 1M cache-read tokens. Omitted, cache-read tokens bill at `prompt`.
    #[serde(default)]
    pub cache_read: Option<String>,
    /// Price per 1M cache-write tokens. Omitted, cache-write tokens bill at `prompt`.
    #[serde(default)]
    pub cache_write: Option<String>,
    /// Price per 1M reasoning tokens. Omitted, reasoning tokens bill at `completion`.
    #[serde(default)]
    pub reasoning: Option<String>,
}

/// The operator-configured `treasurer:` config section (D-07) -- one ISO currency and a price
/// table keyed by bare model name.
///
/// See the module-level documentation for the inert-when-omitted guarantee, the decimal-string
/// price format, and the config-file-only `pricing` map.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::{PriceRowConfig, TreasurerConfig};
///
/// let mut config = TreasurerConfig::default();
/// assert!(config.price_table().unwrap().is_empty());
///
/// config.pricing.insert(
///     "gpt-4".to_string(),
///     PriceRowConfig {
///         prompt: "2.50".to_string(),
///         completion: "10.00".to_string(),
///         cache_read: None,
///         cache_write: None,
///         reasoning: None,
///     },
/// );
/// let table = config.price_table().expect("valid prices should build a table");
/// assert!(!table.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TreasurerConfig {
    /// The ISO 4217 three-letter currency every price in `pricing` is denominated in.
    /// Defaults to `"USD"`. Validated as exactly three ASCII uppercase letters -- never
    /// case-folded.
    pub currency: String,
    /// Per-model price rows, keyed by the bare model name (D-05) -- matched exactly and
    /// case-sensitively against the string an `LlmResponse` reports. **Config-file only**: no
    /// environment variable can populate this map (see the module-level documentation).
    pub pricing: BTreeMap<String, PriceRowConfig>,
    /// Operator-configured allowances (ALLOW-01, D-02). Omitted, the subtree is inert.
    pub allowance: AllowanceConfig,
    /// Rate pacing (PACE-02, D-08). On by default; `cadence.enabled: false` installs nothing.
    pub cadence: CadenceConfig,
}

impl Default for TreasurerConfig {
    fn default() -> Self {
        Self {
            currency: "USD".to_string(),
            pricing: BTreeMap::new(),
            allowance: AllowanceConfig::default(),
            cadence: CadenceConfig::default(),
        }
    }
}

impl TreasurerConfig {
    /// Build a validated [`PriceTable`] from this configuration.
    ///
    /// Iterates `pricing` in its `BTreeMap` key order, so the first invalid entry
    /// deterministically produces the error. Every error names the full config path
    /// (`treasurer.currency` or `treasurer.pricing.{model}.{axis}`) and the offending raw
    /// string.
    ///
    /// # Errors
    ///
    /// A `String` naming the first validation failure: a malformed currency, an empty model
    /// key, or a price axis that is negative, non-decimal, has more than nine decimal places,
    /// or exceeds the largest representable price.
    pub fn price_table(&self) -> Result<PriceTable, String> {
        let currency = CurrencyCode::new(&self.currency).map_err(|_| {
            format!(
                "treasurer.currency must be exactly three ASCII uppercase letters (got {:?})",
                self.currency
            )
        })?;

        let mut table = PriceTable::new(currency);

        for (model, row_cfg) in &self.pricing {
            if model.is_empty() {
                return Err("treasurer.pricing keys must be non-empty model names".to_string());
            }

            let prompt = Self::parse_axis(model, "prompt", &row_cfg.prompt)?;
            let completion = Self::parse_axis(model, "completion", &row_cfg.completion)?;

            let mut price_row = PriceRow::new(prompt, completion)
                .map_err(|e| format!("treasurer.pricing.{model}: {e}"))?;

            if let Some(raw) = &row_cfg.cache_read {
                let value = Self::parse_axis(model, "cache_read", raw)?;
                price_row = price_row
                    .with_cache_read(value)
                    .map_err(|e| format!("treasurer.pricing.{model}: {e}"))?;
            }
            if let Some(raw) = &row_cfg.cache_write {
                let value = Self::parse_axis(model, "cache_write", raw)?;
                price_row = price_row
                    .with_cache_write(value)
                    .map_err(|e| format!("treasurer.pricing.{model}: {e}"))?;
            }
            if let Some(raw) = &row_cfg.reasoning {
                let value = Self::parse_axis(model, "reasoning", raw)?;
                price_row = price_row
                    .with_reasoning(value)
                    .map_err(|e| format!("treasurer.pricing.{model}: {e}"))?;
            }

            table = table.with_row(model.clone(), price_row);
        }

        Ok(table)
    }

    /// Resolve the `allowance` subtree into an [`AllowancePolicy`] in this configuration's
    /// currency (ALLOW-01, D-02).
    ///
    /// # Errors
    ///
    /// A `String` naming the first failure: a malformed currency (the same message
    /// [`TreasurerConfig::price_table`] gives), or an invalid allowance entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::treasurer::TreasurerConfig;
    ///
    /// let policy = TreasurerConfig::default().allowance_policy()?;
    /// assert!(policy.is_empty());
    /// # Ok::<(), String>(())
    /// ```
    pub fn allowance_policy(&self) -> Result<AllowancePolicy, String> {
        let currency = CurrencyCode::new(&self.currency).map_err(|_| {
            format!(
                "treasurer.currency must be exactly three ASCII uppercase letters (got {:?})",
                self.currency
            )
        })?;
        self.allowance.resolve(&currency)
    }

    /// Validate this configuration without discarding what it builds.
    ///
    /// Checks the price table (`self.price_table()`), the allowance policy
    /// (`self.allowance_policy()`) and the cadence subtree (`self.cadence.validate()`) -- validation and the values
    /// `crate::infrastructure::web::agent_host` and
    /// `crate::infrastructure::web::facade_provisioner` build from this same configuration can
    /// never disagree.
    ///
    /// # Errors
    ///
    /// See [`TreasurerConfig::price_table`], [`TreasurerConfig::allowance_policy`] and
    /// [`CadenceConfig::validate`].
    pub fn validate(&self) -> Result<(), String> {
        self.price_table()?;
        self.allowance_policy().map(|_| ())?;
        self.cadence.validate()
    }

    fn parse_axis(model: &str, axis: &str, raw: &str) -> Result<i64, String> {
        parse_decimal_nanos(raw).map_err(|e| match e {
            PriceParseError::Malformed => format!(
                "treasurer.pricing.{model}.{axis} must be a non-negative decimal string \
                 (digits with an optional fractional part of at most 9 places) (got {raw:?})"
            ),
            PriceParseError::TooManyDecimalPlaces => format!(
                "treasurer.pricing.{model}.{axis} has more than 9 decimal places, finer than \
                 one nano-unit per 1M tokens (got {raw:?})"
            ),
            PriceParseError::Overflow => format!(
                "treasurer.pricing.{model}.{axis} exceeds the largest representable price, \
                 {MAX_PRICE_DISPLAY} per 1M tokens (got {raw:?})"
            ),
        })
    }
}

impl EnvOverridable for TreasurerConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_TREASURER_CURRENCY") {
            self.currency = v;
        }
        // `pricing` and the allowance maps are config-file only (see module docs) -- no env var
        // reads into a collection.
        if let Some(v) = read_env::<u8>("APP_TREASURER_ALLOWANCE_WARN_AT") {
            self.allowance.warn_at = v;
        }
        // A secret without a url delivers nothing, so it is applied only to an existing webhook.
        if let Some(webhook) = self.allowance.webhook.as_mut()
            && let Some(v) = read_env::<String>("APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET")
        {
            webhook.secret = Some(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, File, FileFormat};
    use serial_test::serial;
    use std::env;

    fn row(prompt: &str, completion: &str) -> PriceRowConfig {
        PriceRowConfig {
            prompt: prompt.to_string(),
            completion: completion.to_string(),
            cache_read: None,
            cache_write: None,
            reasoning: None,
        }
    }

    #[derive(Debug, Deserialize)]
    struct Wrapper {
        #[serde(default)]
        treasurer: TreasurerConfig,
    }

    fn deserialize_wrapper_from_file(path: &str) -> Wrapper {
        Config::builder()
            .add_source(File::new(path, FileFormat::Yaml))
            .build()
            .unwrap_or_else(|e| panic!("{path} should build: {e}"))
            .try_deserialize()
            .unwrap_or_else(|e| panic!("{path}'s treasurer section should deserialize: {e}"))
    }

    fn deserialize_wrapper(yaml: &str) -> Wrapper {
        Config::builder()
            .add_source(File::from_str(yaml, FileFormat::Yaml))
            .build()
            .expect("config should build from the inline yaml fixture")
            .try_deserialize()
            .expect("wrapper should deserialize")
    }

    #[test]
    fn default_treasurer_config_is_inert() {
        let config = TreasurerConfig::default();
        assert_eq!(config.currency, "USD");
        assert!(config.pricing.is_empty());
        assert!(config.validate().is_ok());
        let table = config.price_table().expect("default should build a table");
        assert!(table.is_empty());
    }

    #[test]
    #[serial]
    fn v0_10_config_resolves_treasurer_inert() {
        let settings = crate::config::settings::Settings::load_from_file("config.test.yml")
            .expect("config.test.yml should load");
        assert_eq!(settings.treasurer, TreasurerConfig::default());
        assert!(settings.validate().is_ok());
    }

    #[test]
    #[serial]
    fn example_config_treasurer_block_round_trips_to_default() {
        let wrapper = deserialize_wrapper_from_file("config.example.yml");
        assert_eq!(wrapper.treasurer, TreasurerConfig::default());
    }

    #[test]
    fn parses_decimal_prices_exactly() {
        assert_eq!(parse_decimal_nanos("2.50"), Ok(2_500_000_000));
        assert_eq!(parse_decimal_nanos("10.00"), Ok(10_000_000_000));
        assert_eq!(parse_decimal_nanos("0.15"), Ok(150_000_000));
        assert_eq!(parse_decimal_nanos("002.50"), Ok(2_500_000_000));
        assert_eq!(parse_decimal_nanos("0"), Ok(0));
        assert_eq!(parse_decimal_nanos("0.0"), Ok(0));
        assert_eq!(parse_decimal_nanos("0.000000000"), Ok(0));
        assert_eq!(parse_decimal_nanos("0.000000001"), Ok(1));
        assert_eq!(parse_decimal_nanos("9223372036.854775807"), Ok(i64::MAX));
    }

    #[test]
    fn validate_rejects_malformed_prices() {
        for bad in [
            "-0.01", "", " 1", "1 ", "1.", ".5", "+1", "1e3", "NaN", "inf", "1,5", "1.2.3",
        ] {
            let config = TreasurerConfig {
                currency: "USD".to_string(),
                pricing: BTreeMap::from([("gpt-4".to_string(), row(bad, "1.00"))]),
                allowance: AllowanceConfig::default(),
                cadence: CadenceConfig::default(),
            };
            let err = config
                .price_table()
                .expect_err(&format!("{bad:?} should be rejected"));
            assert!(
                err.contains("treasurer.pricing.gpt-4.prompt"),
                "error for {bad:?} should name the axis: {err}"
            );
            assert!(
                err.contains("non-negative decimal"),
                "error for {bad:?} should explain the grammar: {err}"
            );
        }
    }

    #[test]
    fn validate_rejects_too_fine_and_overflowing_prices() {
        let too_fine = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), row("0.0000000001", "1.00"))]),
            allowance: AllowanceConfig::default(),
            cadence: CadenceConfig::default(),
        };
        let err = too_fine.price_table().expect_err("should be rejected");
        assert!(err.contains("9 decimal places"), "{err}");

        let overflow = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), row("9223372036.854775808", "1.00"))]),
            allowance: AllowanceConfig::default(),
            cadence: CadenceConfig::default(),
        };
        let err = overflow.price_table().expect_err("should be rejected");
        assert!(err.contains("largest representable price"), "{err}");
    }

    #[test]
    fn validate_rejects_bad_currency() {
        for bad in ["usd", "US", "USDX", ""] {
            let config = TreasurerConfig {
                currency: bad.to_string(),
                pricing: BTreeMap::new(),
                allowance: AllowanceConfig::default(),
                cadence: CadenceConfig::default(),
            };
            let err = config
                .price_table()
                .expect_err(&format!("{bad:?} should be rejected"));
            assert!(
                err.starts_with("treasurer.currency must be exactly three ASCII uppercase letters"),
                "{err}"
            );
        }
    }

    #[test]
    fn validate_rejects_empty_model_key() {
        let config = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([(String::new(), row("1.00", "1.00"))]),
            allowance: AllowanceConfig::default(),
            cadence: CadenceConfig::default(),
        };
        let err = config
            .price_table()
            .expect_err("empty model key should be rejected");
        assert!(
            err.contains("treasurer.pricing keys must be non-empty model names"),
            "{err}"
        );
    }

    #[test]
    fn optional_axes_parse_and_default_to_parent() {
        let mut with_cache = row("2.00", "5.00");
        with_cache.cache_read = Some("0.25".to_string());
        let config = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), with_cache)]),
            allowance: AllowanceConfig::default(),
            cadence: CadenceConfig::default(),
        };
        let table = config.price_table().expect("should build");
        let price_row = table.row("gpt-4").expect("row should exist");
        assert_eq!(price_row.cache_read(), Some(250_000_000));
        assert_eq!(price_row.reasoning(), None);

        let plain = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), row("2.00", "5.00"))]),
            allowance: AllowanceConfig::default(),
            cadence: CadenceConfig::default(),
        };
        let table = plain.price_table().expect("should build");
        let price_row = table.row("gpt-4").expect("row should exist");
        assert_eq!(price_row.cache_read(), None);
        assert_eq!(price_row.reasoning(), None);
    }

    #[test]
    fn missing_required_axis_or_unknown_axis_fails_to_load() {
        let missing_completion = "treasurer:\n  pricing:\n    gpt-4:\n      prompt: \"1.00\"\n";
        let result: Result<Wrapper, _> = Config::builder()
            .add_source(File::from_str(missing_completion, FileFormat::Yaml))
            .build()
            .expect("config should build")
            .try_deserialize();
        assert!(result.is_err(), "missing completion should fail to load");

        let unknown_axis = "treasurer:\n  pricing:\n    gpt-4:\n      prompt: \"1.00\"\n      completion: \"2.00\"\n      cache_reads: \"0.10\"\n";
        let result: Result<Wrapper, _> = Config::builder()
            .add_source(File::from_str(unknown_axis, FileFormat::Yaml))
            .build()
            .expect("config should build")
            .try_deserialize();
        assert!(result.is_err(), "unknown axis should fail to load");
    }

    #[test]
    fn model_keys_survive_loading_verbatim() {
        let yaml = "treasurer:\n  pricing:\n    gpt-3.5-turbo:\n      prompt: \"0.50\"\n      completion: \"1.50\"\n    Qwen-Plus:\n      prompt: \"0.40\"\n      completion: \"1.20\"\n";
        let wrapper = deserialize_wrapper(yaml);
        assert!(wrapper.treasurer.pricing.contains_key("gpt-3.5-turbo"));
        assert!(wrapper.treasurer.pricing.contains_key("Qwen-Plus"));
    }

    #[test]
    #[serial]
    fn env_override_currency() {
        unsafe {
            env::set_var("APP_TREASURER_CURRENCY", "EUR");
        }
        let mut config = TreasurerConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.currency, "EUR");
        unsafe {
            env::remove_var("APP_TREASURER_CURRENCY");
        }

        unsafe {
            env::set_var("APP_TREASURER_CURRENCY", "eur");
        }
        let mut config = TreasurerConfig::default();
        config.apply_env_overrides();
        assert!(config.validate().is_err());
        unsafe {
            env::remove_var("APP_TREASURER_CURRENCY");
        }
    }

    fn allowance_config(entries: &[(&str, &str, &str)]) -> TreasurerConfig {
        let mut config = TreasurerConfig::default();
        for (name, period, amount) in entries {
            config.allowance.api_keys.insert(
                (*name).to_string(),
                AllowanceEntryConfig {
                    period: (*period).to_string(),
                    amount: (*amount).to_string(),
                    lifetime: None,
                    warn_at: None,
                },
            );
        }
        config
    }

    #[test]
    fn allowance_omitted_is_inert() {
        let wrapper = deserialize_wrapper("treasurer:\n  currency: \"USD\"\n");
        assert!(wrapper.treasurer.allowance.is_empty());
        let policy = wrapper.treasurer.allowance_policy().expect("inert policy");
        assert!(policy.is_empty());
        assert!(wrapper.treasurer.validate().is_ok());
    }

    #[test]
    fn allowance_api_key_entry_resolves_to_nanos_and_seconds() {
        let wrapper = deserialize_wrapper(
            "treasurer:\n  allowance:\n    api_keys:\n      svc-a:\n        period: \"1h\"\n        amount: \"2.50\"\n",
        );
        let policy = wrapper.treasurer.allowance_policy().expect("valid policy");
        let subject = paladin_core::platform::container::principal::RunAttribution::new(
            TenantId::new("acme").expect("tenant"),
            "svc-a",
        );
        let ceilings = policy.ceilings_for(&subject);
        assert_eq!(ceilings.len(), 1);
        assert_eq!(ceilings[0].ceiling_nanos, 2_500_000_000);
        assert_eq!(ceilings[0].period_secs, Some(3_600));
        assert_eq!(ceilings[0].warn_at, 80);
    }

    #[test]
    fn allowance_rejects_zero_amount_naming_the_path() {
        for zero in ["0", "0.0", "0.000000000"] {
            let err = allowance_config(&[("ci-runner", "1d", zero)])
                .allowance_policy()
                .expect_err("a zero amount is rejected");
            assert!(
                err.contains("treasurer.allowance.api_keys.ci-runner.amount must be a positive"),
                "{err}"
            );
            assert!(err.contains(&format!("(got {zero:?})")), "{err}");
        }
    }

    #[test]
    fn allowance_rejects_malformed_amount_and_period_naming_the_path() {
        let err = allowance_config(&[("ci-runner", "1d", "-1")])
            .allowance_policy()
            .expect_err("negative amount");
        assert!(
            err.contains("treasurer.allowance.api_keys.ci-runner.amount"),
            "{err}"
        );

        let err = allowance_config(&[("ci-runner", "1d", "1.0000000001")])
            .allowance_policy()
            .expect_err("too fine");
        assert!(err.contains("api_keys.ci-runner.amount"), "{err}");

        let err = allowance_config(&[("ci-runner", "90s", "5")])
            .allowance_policy()
            .expect_err("seconds are not a unit");
        assert!(
            err.contains(
                "treasurer.allowance.api_keys.ci-runner.period must be <integer><m|h|d> between 1m and 366d (got \"90s\")"
            ),
            "{err}"
        );

        let err = allowance_config(&[("bad name", "1d", "5")])
            .allowance_policy()
            .expect_err("whitespace key name");
        assert!(err.contains("not a valid API key name"), "{err}");

        let mut bad_currency = allowance_config(&[("k", "1d", "5")]);
        bad_currency.currency = "usd".to_string();
        let err = bad_currency.allowance_policy().expect_err("currency");
        assert!(
            err.starts_with("treasurer.currency must be exactly three"),
            "{err}"
        );
        assert!(bad_currency.validate().is_err());
    }

    #[test]
    fn allowance_validate_reports_allowance_errors() {
        let config = allowance_config(&[("k", "0m", "5")]);
        assert!(config.validate().is_err());
    }

    #[test]
    fn period_grammar_accepts_minutes_hours_days_within_bounds() {
        assert_eq!(parse_period_secs("1m"), Ok(60));
        assert_eq!(parse_period_secs("90m"), Ok(5_400));
        assert_eq!(parse_period_secs("1h"), Ok(3_600));
        assert_eq!(parse_period_secs("7d"), Ok(604_800));
        assert_eq!(parse_period_secs("366d"), Ok(MAX_ALLOWANCE_PERIOD_SECS));
        assert_eq!(parse_period_secs("0001m"), Ok(60));
    }

    #[test]
    fn period_grammar_rejects_everything_else() {
        for bad in [
            "",
            "h",
            "0m",
            "59s",
            "1H",
            " 1h",
            "1h ",
            "1.5h",
            "-1h",
            "367d",
            "1w",
            "+1h",
            "1",
            "99999999999999999999d",
            "1hh",
            "m1",
        ] {
            assert!(parse_period_secs(bad).is_err(), "{bad:?} must be rejected");
        }
        assert_eq!(MAX_ALLOWANCE_PERIOD_SECS, 366 * 86_400);
    }

    fn fail_to_load(yaml: &str) {
        let result: Result<Wrapper, _> = Config::builder()
            .add_source(File::from_str(yaml, FileFormat::Yaml))
            .build()
            .expect("config should build")
            .try_deserialize();
        assert!(result.is_err(), "should fail to load: {yaml}");
    }

    fn subject(
        tenant: &str,
        key: &str,
    ) -> paladin_core::platform::container::principal::RunAttribution {
        paladin_core::platform::container::principal::RunAttribution::new(
            TenantId::new(tenant).expect("tenant"),
            key,
        )
    }

    #[test]
    fn allowance_full_grammar_resolves_every_limit_kind() {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceScopeKind,
        };
        let wrapper = deserialize_wrapper(
            "treasurer:\n  allowance:\n    warn_at: 90\n    tenants:\n      acme:\n        period: \"24h\"\n        amount: \"25.00\"\n        lifetime: \"500.00\"\n    api_keys:\n      ci-runner:\n        period: \"1h\"\n        amount: \"2.50\"\n        warn_at: 95\n",
        );
        let policy = wrapper.treasurer.allowance_policy().expect("valid policy");
        let ceilings = policy.ceilings_for(&subject("acme", "ci-runner"));
        assert_eq!(ceilings.len(), 3, "{ceilings:?}");
        assert_eq!(ceilings[0].scope_kind, AllowanceScopeKind::ApiKey);
        assert_eq!(ceilings[0].limit_kind, AllowanceLimitKind::Window);
        assert_eq!(ceilings[0].period_secs, Some(3_600));
        assert_eq!(ceilings[0].ceiling_nanos, 2_500_000_000);
        assert_eq!(ceilings[0].warn_at, 95);
        assert_eq!(ceilings[1].scope_kind, AllowanceScopeKind::Tenant);
        assert_eq!(ceilings[1].period_secs, Some(86_400));
        assert_eq!(ceilings[1].ceiling_nanos, 25_000_000_000);
        assert_eq!(ceilings[1].warn_at, 90);
        assert_eq!(ceilings[2].limit_kind, AllowanceLimitKind::Lifetime);
        assert_eq!(ceilings[2].period_secs, None);
        assert_eq!(ceilings[2].ceiling_nanos, 500_000_000_000);
        assert_eq!(ceilings[2].warn_at, 90);
    }

    #[test]
    fn allowance_warn_at_defaults_to_80_and_accepts_0_and_100() {
        assert_eq!(
            AllowanceConfig::default().warn_at,
            DEFAULT_ALLOWANCE_WARN_AT
        );
        assert_eq!(DEFAULT_ALLOWANCE_WARN_AT, 80);
        let mut config = allowance_config(&[("k", "1d", "5")]);
        let ceilings = config
            .allowance_policy()
            .expect("default")
            .ceilings_for(&subject("acme", "k"));
        assert_eq!(ceilings[0].warn_at, 80);
        for edge in [0u8, 100] {
            config.allowance.warn_at = edge;
            let ceilings = config
                .allowance
                .resolve(&CurrencyCode::new("USD").expect("usd"))
                .expect("edge accepted")
                .ceilings_for(&subject("acme", "k"));
            assert_eq!(ceilings[0].warn_at, edge);
        }
        config.allowance.warn_at = 80;
        for edge in [0u8, 100] {
            config
                .allowance
                .api_keys
                .get_mut("k")
                .expect("entry")
                .warn_at = Some(edge);
            let ceilings = config
                .allowance_policy()
                .expect("entry edge accepted")
                .ceilings_for(&subject("acme", "k"));
            assert_eq!(ceilings[0].warn_at, edge);
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn allowance_warn_at_above_100_is_rejected_globally_and_per_entry() {
        let mut config = allowance_config(&[("k", "1d", "5")]);
        config.allowance.warn_at = 101;
        let err = config.validate().expect_err("global 101");
        assert!(
            err.contains(
                "treasurer.allowance.warn_at must be an integer percent between 0 and 100 (got 101)"
            ),
            "{err}"
        );
        config.allowance.warn_at = 80;
        config
            .allowance
            .api_keys
            .get_mut("k")
            .expect("entry")
            .warn_at = Some(101);
        let err = config.validate().expect_err("entry 101");
        assert!(
            err.contains("treasurer.allowance.api_keys.k.warn_at"),
            "{err}"
        );
        assert!(err.contains("(got 101)"), "{err}");
    }

    #[test]
    fn allowance_lifetime_rejects_non_positive_malformed_and_overflowing_values() {
        for bad in [
            "0",
            "-1",
            "1e3",
            "1,000",
            " 5",
            "5 ",
            "+5",
            "",
            "5.0000000001",
            "9223372037",
        ] {
            let mut config = TreasurerConfig::default();
            config.allowance.tenants.insert(
                "acme".to_string(),
                AllowanceEntryConfig {
                    period: "1d".to_string(),
                    amount: "5".to_string(),
                    lifetime: Some(bad.to_string()),
                    warn_at: None,
                },
            );
            let err = config
                .validate()
                .expect_err(&format!("{bad:?} should be rejected"));
            assert!(
                err.contains("treasurer.allowance.tenants.acme.lifetime"),
                "{bad:?}: {err}"
            );
            assert!(err.contains(&format!("(got {bad:?})")), "{bad:?}: {err}");
        }
    }

    #[test]
    fn allowance_decimal_grammar_is_exact_to_one_nano_unit() {
        let config = allowance_config(&[("one", "1d", "0.000000001"), ("big", "1d", "25.00")]);
        let policy = config.allowance_policy().expect("valid");
        assert_eq!(
            policy.ceilings_for(&subject("acme", "one"))[0].ceiling_nanos,
            1
        );
        assert_eq!(
            policy.ceilings_for(&subject("acme", "big"))[0].ceiling_nanos,
            25_000_000_000
        );
        let mut config = TreasurerConfig::default();
        config.allowance.tenants.insert(
            "acme".to_string(),
            AllowanceEntryConfig {
                period: "1d".to_string(),
                amount: "5".to_string(),
                lifetime: Some("0.000000001".to_string()),
                warn_at: None,
            },
        );
        let ceilings = config
            .allowance_policy()
            .expect("valid")
            .ceilings_for(&subject("acme", "x"));
        assert_eq!(ceilings[1].ceiling_nanos, 1);
    }

    #[test]
    fn allowance_unknown_keys_fail_to_load() {
        // The section-level typo from RESEARCH Pitfall 17, then one case per struct.
        fail_to_load("treasurer:\n  allowence:\n    api_keys: {}\n");
        fail_to_load(
            "treasurer:\n  allowance:\n    api_key:\n      svc-a:\n        period: \"1d\"\n        amount: \"1\"\n",
        );
        fail_to_load(
            "treasurer:\n  allowance:\n    api_keys:\n      svc-a:\n        perid: \"1d\"\n        amount: \"1\"\n",
        );
        fail_to_load(
            "treasurer:\n  allowance:\n    tenants:\n      acme:\n        period: \"1d\"\n        amount: \"1\"\n        lifetim: \"9\"\n",
        );
        fail_to_load(
            "treasurer:\n  allowance:\n    webhook:\n      url: \"https://x.example\"\n      secrett: \"s\"\n",
        );
    }

    #[test]
    fn allowance_invalid_map_keys_are_rejected() {
        for bad in ["", "has space"] {
            let mut config = allowance_config(&[("ok", "1d", "5")]);
            config
                .allowance
                .tenants
                .insert(bad.to_string(), config.allowance.api_keys["ok"].clone());
            let err = config
                .validate()
                .expect_err(&format!("tenant key {bad:?} should be rejected"));
            assert!(err.contains("treasurer.allowance.tenants"), "{err}");
            assert!(err.contains("not a valid tenant id"), "{err}");
        }
    }

    #[test]
    fn allowance_empty_webhook_url_is_rejected() {
        let mut config = allowance_config(&[("k", "1d", "5")]);
        config.allowance.webhook = Some(AllowanceWebhookConfig {
            url: String::new(),
            secret: Some("s3cr3t-value".to_string()),
        });
        let err = config.validate().expect_err("empty url");
        assert!(err.contains("treasurer.allowance.webhook.url"), "{err}");
        assert!(!err.contains("s3cr3t-value"), "{err}");
    }

    #[test]
    fn allowance_webhook_secret_is_redacted_from_debug_and_serialize() {
        let mut config = allowance_config(&[("k", "1d", "5")]);
        config.allowance.webhook = Some(AllowanceWebhookConfig {
            url: "https://ops.example.com/hook".to_string(),
            secret: Some("s3cr3t-value".to_string()),
        });
        let debug = format!("{config:?}");
        assert!(!debug.contains("s3cr3t-value"), "{debug}");
        assert!(debug.contains("[redacted]"), "{debug}");
        assert!(debug.contains("https://ops.example.com/hook"), "{debug}");
        let json = serde_json::to_string(&config).expect("serialize");
        assert!(!json.contains("s3cr3t-value"), "{json}");
    }

    #[test]
    fn yaml_env_placeholder_is_not_expanded() {
        // C7: the loader performs no ${VAR} expansion, so the placeholder arrives literally.
        let wrapper = deserialize_wrapper(
            "treasurer:\n  allowance:\n    webhook:\n      url: \"https://ops.example.com/hook\"\n      secret: \"${ALLOWANCE_WEBHOOK_SECRET}\"\n",
        );
        let secret = wrapper
            .treasurer
            .allowance
            .webhook
            .and_then(|w| w.secret)
            .expect("secret loads");
        assert_eq!(secret, "${ALLOWANCE_WEBHOOK_SECRET}");
    }

    #[test]
    #[serial]
    fn env_override_allowance_warn_at() {
        unsafe {
            env::set_var("APP_TREASURER_ALLOWANCE_WARN_AT", "70");
        }
        let mut config = TreasurerConfig::default();
        config.apply_env_overrides();
        unsafe {
            env::remove_var("APP_TREASURER_ALLOWANCE_WARN_AT");
        }
        assert_eq!(config.allowance.warn_at, 70);
    }

    #[test]
    #[serial]
    fn env_override_allowance_webhook_secret_only_with_a_webhook() {
        unsafe {
            env::set_var("APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET", "from-env");
        }
        let mut without = TreasurerConfig::default();
        without.apply_env_overrides();
        assert!(without.allowance.webhook.is_none());

        let mut with = TreasurerConfig::default();
        with.allowance.webhook = Some(AllowanceWebhookConfig {
            url: "https://ops.example.com/hook".to_string(),
            secret: None,
        });
        with.apply_env_overrides();
        unsafe {
            env::remove_var("APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET");
        }
        assert_eq!(
            with.allowance.webhook.and_then(|w| w.secret).as_deref(),
            Some("from-env")
        );
    }

    fn names(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn validate_against_rejects_allowances_without_a_run_store() {
        let config = allowance_config(&[("k", "1d", "5")]);
        let err = config
            .allowance
            .validate_against(&names(&["acme"]), &names(&["k"]), true)
            .expect_err("disabled store");
        assert!(err.contains("treasurer.allowance"), "{err}");
        assert!(err.contains("run_store.backend"), "{err}");
    }

    #[test]
    fn validate_against_rejects_unknown_api_keys_and_tenants() {
        let config = allowance_config(&[("ghost", "1d", "5")]);
        let err = config
            .allowance
            .validate_against(&names(&["acme"]), &names(&["k"]), false)
            .expect_err("unknown key");
        assert!(err.contains("treasurer.allowance.api_keys.ghost"), "{err}");
        assert!(err.contains("http.auth.api_keys"), "{err}");

        let mut config = TreasurerConfig::default();
        config.allowance.tenants.insert(
            "globex".to_string(),
            AllowanceEntryConfig {
                period: "1d".to_string(),
                amount: "5".to_string(),
                lifetime: None,
                warn_at: None,
            },
        );
        let err = config
            .allowance
            .validate_against(&names(&["acme"]), &names(&["k"]), false)
            .expect_err("unknown tenant");
        assert!(err.contains("treasurer.allowance.tenants.globex"), "{err}");
    }

    #[test]
    fn validate_against_accepts_known_targets_and_empty_allowances() {
        let config = allowance_config(&[("k", "1d", "5")]);
        assert!(
            config
                .allowance
                .validate_against(&names(&["acme"]), &names(&["k"]), false)
                .is_ok()
        );
        assert!(
            AllowanceConfig::default()
                .validate_against(&names(&[]), &names(&[]), true)
                .is_ok()
        );
    }

    // ── Phase 43 (PACE-02, D-08, D-10): the `cadence` subtree ────────────────────────────────

    fn treasurer_from_json(json: &str) -> Result<TreasurerConfig, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn cadence_omitted_section_deserializes_to_the_defaults_with_pacing_on() {
        let config = treasurer_from_json("{}").expect("empty section");
        assert_eq!(config.cadence, CadenceConfig::default());
        assert!(config.cadence.enabled, "pacing is on by default (D-08)");
        assert_eq!(config.cadence.backend, CadenceBackend::InProcess);
        assert_eq!(config.cadence.base_backoff_ms, 500);
        assert_eq!(config.cadence.max_backoff_ms, 30_000);
        assert_eq!(config.cadence.max_wait_secs, 300);
        assert_eq!(config.cadence.degraded_multiplier, 2.0);
        assert_eq!(config.cadence.fallback_pace_budget_secs, 60);
        assert_eq!(config.cadence.lock_ttl_secs, 120);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn cadence_enabled_false_round_trips() {
        let config = treasurer_from_json(r#"{"cadence":{"enabled":false}}"#).expect("parse");
        assert!(!config.cadence.enabled);
        let json = serde_json::to_string(&config).expect("serialize");
        let back = treasurer_from_json(&json).expect("round trip");
        assert_eq!(back, config);
        assert!(!back.cadence.enabled);
    }

    #[test]
    fn cadence_backend_is_written_in_snake_case() {
        let config = treasurer_from_json(r#"{"cadence":{"backend":"in_process"}}"#).expect("parse");
        assert_eq!(config.cadence.backend, CadenceBackend::InProcess);
        assert!(treasurer_from_json(r#"{"cadence":{"backend":"InProcess"}}"#).is_err());
    }

    #[test]
    fn cadence_unknown_key_fails_to_deserialize() {
        let err = treasurer_from_json(r#"{"cadence":{"base_backoff_msec":10}}"#)
            .expect_err("a typo must not silently disable pacing");
        assert!(err.to_string().contains("base_backoff_msec"), "{err}");
    }

    #[test]
    fn cadence_validate_names_the_full_key_of_every_rule() {
        for key in [
            "base_backoff_ms",
            "max_backoff_ms",
            "max_wait_secs",
            "fallback_pace_budget_secs",
            "lock_ttl_secs",
        ] {
            let json = format!(r#"{{"cadence":{{"{key}":0}}}}"#);
            let config = treasurer_from_json(&json).expect("parse");
            let err = config.validate().expect_err("zero must be rejected");
            assert!(
                err.contains(&format!("treasurer.cadence.{key}")),
                "{key}: {err}"
            );
        }

        let inverted =
            treasurer_from_json(r#"{"cadence":{"base_backoff_ms":900,"max_backoff_ms":100}}"#)
                .expect("parse");
        let err = inverted.validate().expect_err("base above max");
        assert!(err.contains("treasurer.cadence.base_backoff_ms"), "{err}");
        assert!(err.contains("treasurer.cadence.max_backoff_ms"), "{err}");

        for bad in ["0.5", "-1.0", "1e999"] {
            let json = format!(r#"{{"cadence":{{"degraded_multiplier":{bad}}}}}"#);
            // 1e999 overflows f64 and is rejected at parse time by serde_json; the finite-range
            // cases reach validate().
            if let Ok(config) = treasurer_from_json(&json) {
                let err = config
                    .validate()
                    .expect_err("multiplier below 1 or non-finite");
                assert!(
                    err.contains("treasurer.cadence.degraded_multiplier"),
                    "{bad}: {err}"
                );
            }
        }
        let nan = CadenceConfig {
            degraded_multiplier: f64::NAN,
            ..CadenceConfig::default()
        };
        assert!(
            nan.validate()
                .expect_err("NaN")
                .contains("treasurer.cadence.degraded_multiplier")
        );
    }

    #[test]
    fn cadence_policy_and_settings_follow_the_configured_values() {
        let config = CadenceConfig {
            base_backoff_ms: 200,
            max_backoff_ms: 400,
            max_wait_secs: 7,
            fallback_pace_budget_secs: 9,
            ..CadenceConfig::default()
        };
        let policy = config.policy().expect("policy");
        assert_eq!(policy.base_backoff(), std::time::Duration::from_millis(200));
        assert_eq!(policy.max_backoff(), std::time::Duration::from_millis(400));
        let settings = config.settings();
        assert_eq!(settings.max_wait(), std::time::Duration::from_secs(7));
        assert_eq!(
            settings.max_backoff(),
            std::time::Duration::from_millis(400)
        );
        assert_eq!(
            settings.fallback_pace_budget(),
            std::time::Duration::from_secs(9)
        );
    }

    #[test]
    fn cadence_invalid_section_fails_treasurer_validate() {
        let mut config = TreasurerConfig::default();
        config.cadence.lock_ttl_secs = 0;
        let err = config.validate().expect_err("cadence is validated");
        assert!(err.contains("treasurer.cadence.lock_ttl_secs"), "{err}");
    }
}
