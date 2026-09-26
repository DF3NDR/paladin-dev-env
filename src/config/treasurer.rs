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
//! Later phases (41, 43) add `allowance` and pacing keys under this same `treasurer:` section.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};
use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};

/// The largest nano-units-per-1M-tokens price representable in a [`PriceRowConfig`] axis, as a
/// decimal string, for use in error messages (`i64::MAX` nano-units = `9223372036.854775807`).
const MAX_PRICE_DISPLAY: &str = "9223372036.854775807";

/// Errors constructing an `i64` nano-units-per-1M price from an operator-entered decimal string.
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

/// Parse a decimal-string price (nano-units per 1M tokens, D-01) with exact integer arithmetic.
///
/// Accepts exactly one or more ASCII digits, optionally followed by `.` and one to nine ASCII
/// digits -- no sign, whitespace, exponent, thousands separator or `NaN`/`inf` spelling. No
/// floating point is used anywhere in this function; the integer part is accumulated with
/// `checked_mul`/`checked_add`, and the fractional digits are right-padded to nine places and
/// added as a nano-unit remainder.
fn parse_price_nanos_per_million(raw: &str) -> Result<i64, PriceParseError> {
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
    let frac_value: i64 = padded
        .parse()
        .map_err(|_| PriceParseError::Malformed)?;

    let scaled_int = int_value
        .checked_mul(1_000_000_000)
        .ok_or(PriceParseError::Overflow)?;
    scaled_int
        .checked_add(frac_value)
        .ok_or(PriceParseError::Overflow)
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
}

impl Default for TreasurerConfig {
    fn default() -> Self {
        Self {
            currency: "USD".to_string(),
            pricing: BTreeMap::new(),
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

    /// Validate this configuration without discarding a built [`PriceTable`].
    ///
    /// Exactly `self.price_table().map(|_| ())` -- validation and the table
    /// `crate::infrastructure::web::agent_host` and
    /// `crate::infrastructure::web::facade_provisioner` build from this same configuration can
    /// never disagree.
    ///
    /// # Errors
    ///
    /// See [`TreasurerConfig::price_table`].
    pub fn validate(&self) -> Result<(), String> {
        self.price_table().map(|_| ())
    }

    fn parse_axis(model: &str, axis: &str, raw: &str) -> Result<i64, String> {
        parse_price_nanos_per_million(raw).map_err(|e| match e {
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
        assert_eq!(parse_price_nanos_per_million("2.50"), Ok(2_500_000_000));
        assert_eq!(parse_price_nanos_per_million("10.00"), Ok(10_000_000_000));
        assert_eq!(parse_price_nanos_per_million("0.15"), Ok(150_000_000));
        assert_eq!(parse_price_nanos_per_million("002.50"), Ok(2_500_000_000));
        assert_eq!(parse_price_nanos_per_million("0"), Ok(0));
        assert_eq!(parse_price_nanos_per_million("0.0"), Ok(0));
        assert_eq!(parse_price_nanos_per_million("0.000000000"), Ok(0));
        assert_eq!(parse_price_nanos_per_million("0.000000001"), Ok(1));
        assert_eq!(
            parse_price_nanos_per_million("9223372036.854775807"),
            Ok(i64::MAX)
        );
    }

    #[test]
    fn validate_rejects_malformed_prices() {
        for bad in [
            "-0.01", "", " 1", "1 ", "1.", ".5", "+1", "1e3", "NaN", "inf", "1,5", "1.2.3",
        ] {
            let config = TreasurerConfig {
                currency: "USD".to_string(),
                pricing: BTreeMap::from([("gpt-4".to_string(), row(bad, "1.00"))]),
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
        };
        let err = too_fine.price_table().expect_err("should be rejected");
        assert!(err.contains("9 decimal places"), "{err}");

        let overflow = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([(
                "gpt-4".to_string(),
                row("9223372036.854775808", "1.00"),
            )]),
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
        };
        let table = config.price_table().expect("should build");
        let price_row = table.row("gpt-4").expect("row should exist");
        assert_eq!(price_row.cache_read(), Some(250_000_000));
        assert_eq!(price_row.reasoning(), None);

        let plain = TreasurerConfig {
            currency: "USD".to_string(),
            pricing: BTreeMap::from([("gpt-4".to_string(), row("2.00", "5.00"))]),
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
}
