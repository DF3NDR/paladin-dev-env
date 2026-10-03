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
//! No field in this tree is secret-shaped: prices and an ISO currency code carry no credential.
//!
//! Phase 41 adds the `allowance` subtree (ALLOW-01, D-02): operator-configured per-API-key
//! rolling-window allowances, `treasurer.allowance.api_keys.<name>: { period, amount }`, with
//! `period` written as `<integer><m|h|d>` (1m to 366d) and `amount` a decimal string in whole
//! currency units. Omitted, the subtree is inert. Later phases (41, 43) add the rest of the
//! allowance grammar and pacing keys under this same `treasurer:` section.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::application::services::treasurer::{AllowancePolicy, ScopeAllowance};
use crate::config::env_utils::{EnvOverridable, read_env};
use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};
use paladin_core::platform::container::principal::TenantId;

/// The default warn threshold, in whole percent of a ceiling, until 41-03 makes it configurable.
const DEFAULT_WARN_AT_PERCENT: u8 = 80;

/// The longest allowance period, in seconds (366 days). A longer period is rejected, never
/// clamped (D-00d).
pub const MAX_ALLOWANCE_PERIOD_SECS: u64 = 31_622_400;

/// The shortest allowance period, in seconds (one minute).
const MIN_ALLOWANCE_PERIOD_SECS: u64 = 60;

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
/// Both fields are required by serde, and an unknown key is a load error.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::AllowanceEntryConfig;
///
/// let entry = AllowanceEntryConfig {
///     period: "1d".to_string(),
///     amount: "2.50".to_string(),
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
}

/// The `treasurer.allowance` subtree (ALLOW-01, D-02): per-API-key rolling-window allowances,
/// keyed by the API key's configured name. Omitted, it is inert. **Config-file only**.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::AllowanceConfig;
///
/// assert!(AllowanceConfig::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AllowanceConfig {
    /// API key name -> window allowance.
    pub api_keys: BTreeMap<String, AllowanceEntryConfig>,
}

impl AllowanceConfig {
    /// Whether no allowance is configured.
    pub fn is_empty(&self) -> bool {
        self.api_keys.is_empty()
    }

    /// Resolve every entry, in key order, into an [`AllowancePolicy`] denominated in
    /// `currency`: decimal strings and periods become integers once, here.
    ///
    /// # Errors
    ///
    /// A `String` naming the full config path of the first invalid entry: an invalid key name,
    /// a malformed or zero amount, or a malformed or out-of-range period.
    pub fn resolve(&self, currency: &CurrencyCode) -> Result<AllowancePolicy, String> {
        let mut policy = AllowancePolicy::new(currency.clone(), DEFAULT_WARN_AT_PERCENT);
        for (name, entry) in &self.api_keys {
            TenantId::new(name.as_str()).map_err(|e| {
                format!("treasurer.allowance.api_keys.{name} is not a valid API key name: {e}")
            })?;
            let amount = match parse_decimal_nanos(&entry.amount) {
                Ok(nanos) if nanos > 0 => nanos,
                _ => {
                    return Err(format!(
                        "treasurer.allowance.api_keys.{name}.amount must be a positive decimal \
                         string in whole currency units (digits with an optional fractional \
                         part of at most 9 places) (got {:?})",
                        entry.amount
                    ));
                }
            };
            let period = parse_period_secs(&entry.period).map_err(|_| {
                format!(
                    "treasurer.allowance.api_keys.{name}.period must be <integer><m|h|d> \
                     between 1m and 366d (got {:?})",
                    entry.period
                )
            })?;
            policy = policy.with_api_key(name.as_str(), ScopeAllowance::new(period, amount));
        }
        Ok(policy)
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
#[serde(default)]
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
}

impl Default for TreasurerConfig {
    fn default() -> Self {
        Self {
            currency: "USD".to_string(),
            pricing: BTreeMap::new(),
            allowance: AllowanceConfig::default(),
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
    /// Checks the price table (`self.price_table()`) and the allowance policy
    /// (`self.allowance_policy()`) -- validation and the values
    /// `crate::infrastructure::web::agent_host` and
    /// `crate::infrastructure::web::facade_provisioner` build from this same configuration can
    /// never disagree.
    ///
    /// # Errors
    ///
    /// See [`TreasurerConfig::price_table`] and [`TreasurerConfig::allowance_policy`].
    pub fn validate(&self) -> Result<(), String> {
        self.price_table()?;
        self.allowance_policy().map(|_| ())
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
        // `pricing` is config-file only (see module docs) -- no env var reads into this map.
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
        };
        let err = too_fine.price_table().expect_err("should be rejected");
        assert!(err.contains("9 decimal places"), "{err}");

        let overflow = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), row("9223372036.854775808", "1.00"))]),
            allowance: AllowanceConfig::default(),
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
        };
        let table = config.price_table().expect("should build");
        let price_row = table.row("gpt-4").expect("row should exist");
        assert_eq!(price_row.cache_read(), Some(250_000_000));
        assert_eq!(price_row.reasoning(), None);

        let plain = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), row("2.00", "5.00"))]),
            allowance: AllowanceConfig::default(),
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

    #[test]
    fn allowance_unknown_keys_fail_to_load() {
        let yaml = "treasurer:\n  allowance:\n    api_keys:\n      svc-a:\n        period: \"1d\"\n        amount: \"1\"\n        burst: \"3\"\n";
        let result: Result<Wrapper, _> = Config::builder()
            .add_source(File::from_str(yaml, FileFormat::Yaml))
            .build()
            .expect("config should build")
            .try_deserialize();
        assert!(result.is_err(), "an unknown entry key should fail to load");
    }
}
