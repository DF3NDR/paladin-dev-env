//! The resolved allowance policy the Treasurer evaluates (ALLOW-01, D-02, D-03).
//!
//! [`AllowancePolicy`] is built once, at boot, from operator configuration
//! (`crate::config::treasurer::AllowanceConfig::resolve`): decimal strings and periods are
//! already integers here, so nothing downstream parses or compares floating point (D-00d).
//!
//! A principal yields up to four [`Ceiling`]s, always in the same order -- API-key window,
//! API-key lifetime, tenant window, tenant lifetime -- each only when its entry and limit
//! exist. "Unlimited" is the absence of a ceiling, never a sentinel amount (D-03).

use std::collections::BTreeMap;

use paladin_core::platform::container::allowance::{AllowanceLimitKind, AllowanceScopeKind};
use paladin_core::platform::container::cost::CurrencyCode;
use paladin_core::platform::container::principal::RunAttribution;

/// One scope's configured allowance: a window ceiling and an optional lifetime cap.
///
/// # Examples
///
/// ```
/// use paladin::application::services::treasurer::ScopeAllowance;
///
/// let allowance = ScopeAllowance::new(86_400, 2_500_000_000).with_lifetime(100_000_000_000);
/// assert_eq!(allowance.period_secs(), 86_400);
/// assert_eq!(allowance.amount_nanos(), 2_500_000_000);
/// assert_eq!(allowance.lifetime_nanos(), Some(100_000_000_000));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeAllowance {
    period_secs: u64,
    amount_nanos: i64,
    lifetime_nanos: Option<i64>,
    warn_at: Option<u8>,
}

impl ScopeAllowance {
    /// A window allowance of `amount_nanos` per `period_secs`.
    pub fn new(period_secs: u64, amount_nanos: i64) -> Self {
        Self {
            period_secs,
            amount_nanos,
            lifetime_nanos: None,
            warn_at: None,
        }
    }

    /// Add a lifetime cap of `nanos`.
    pub fn with_lifetime(mut self, nanos: i64) -> Self {
        self.lifetime_nanos = Some(nanos);
        self
    }

    /// Override the warn threshold (whole percent of a ceiling) for this entry.
    pub fn with_warn_at(mut self, percent: u8) -> Self {
        self.warn_at = Some(percent);
        self
    }

    /// The window period in seconds.
    pub fn period_secs(&self) -> u64 {
        self.period_secs
    }

    /// The window ceiling in nano-units.
    pub fn amount_nanos(&self) -> i64 {
        self.amount_nanos
    }

    /// The lifetime cap in nano-units, when one is configured.
    pub fn lifetime_nanos(&self) -> Option<i64> {
        self.lifetime_nanos
    }

    /// The per-entry warn threshold override, when one is configured.
    pub fn warn_at(&self) -> Option<u8> {
        self.warn_at
    }
}

/// One ceiling: a (scope kind, limit kind, amount, optional period) tuple evaluated over one
/// balance read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ceiling {
    /// Which identity the ceiling is held against.
    pub scope_kind: AllowanceScopeKind,
    /// Which kind of limit it is.
    pub limit_kind: AllowanceLimitKind,
    /// The tenant the balance is summed over.
    pub tenant_id: String,
    /// The API key the balance is restricted to; `None` for a tenant ceiling.
    pub api_key_id: Option<String>,
    /// The window period in seconds; `None` for a lifetime ceiling.
    pub period_secs: Option<u64>,
    /// The ceiling in nano-units; a balance at or above it is exhausted.
    pub ceiling_nanos: i64,
    /// The warn threshold in whole percent of the ceiling.
    pub warn_at: u8,
}

/// The resolved allowance policy: per-API-key and per-tenant allowances in one currency.
///
/// # Examples
///
/// ```
/// use paladin::application::services::treasurer::{AllowancePolicy, ScopeAllowance};
/// use paladin_core::platform::container::cost::CurrencyCode;
/// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
///
/// let policy = AllowancePolicy::new(CurrencyCode::new("USD")?, 80)
///     .with_api_key("svc-a", ScopeAllowance::new(86_400, 2_500_000_000));
/// let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
/// assert_eq!(policy.ceilings_for(&subject).len(), 1);
/// let other = RunAttribution::new(TenantId::new("acme")?, "svc-b");
/// assert!(policy.ceilings_for(&other).is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowancePolicy {
    currency: CurrencyCode,
    default_warn_at: u8,
    tenants: BTreeMap<String, ScopeAllowance>,
    api_keys: BTreeMap<String, ScopeAllowance>,
}

impl AllowancePolicy {
    /// An empty policy in `currency`, warning at `default_warn_at` percent unless an entry
    /// overrides it.
    pub fn new(currency: CurrencyCode, default_warn_at: u8) -> Self {
        Self {
            currency,
            default_warn_at,
            tenants: BTreeMap::new(),
            api_keys: BTreeMap::new(),
        }
    }

    /// Add (or replace) the allowance shared by every key of tenant `id`.
    pub fn with_tenant(mut self, id: impl Into<String>, allowance: ScopeAllowance) -> Self {
        self.tenants.insert(id.into(), allowance);
        self
    }

    /// Add (or replace) the allowance held by API key `name`.
    pub fn with_api_key(mut self, name: impl Into<String>, allowance: ScopeAllowance) -> Self {
        self.api_keys.insert(name.into(), allowance);
        self
    }

    /// The currency every ceiling is denominated in.
    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    /// Whether the policy holds no allowance at all.
    pub fn is_empty(&self) -> bool {
        self.tenants.is_empty() && self.api_keys.is_empty()
    }

    /// The ceilings that apply to `subject`, in the fixed order API-key window, API-key
    /// lifetime, tenant window, tenant lifetime. A principal with no entry yields none.
    pub fn ceilings_for(&self, subject: &RunAttribution) -> Vec<Ceiling> {
        let tenant_id = subject.tenant_id.as_str();
        let mut out = Vec::new();

        if let Some(entry) = self.api_keys.get(&subject.api_key_id) {
            self.push_entry(
                &mut out,
                AllowanceScopeKind::ApiKey,
                tenant_id,
                Some(&subject.api_key_id),
                entry,
            );
        }
        if let Some(entry) = self.tenants.get(tenant_id) {
            self.push_entry(&mut out, AllowanceScopeKind::Tenant, tenant_id, None, entry);
        }
        out
    }

    fn push_entry(
        &self,
        out: &mut Vec<Ceiling>,
        scope_kind: AllowanceScopeKind,
        tenant_id: &str,
        api_key_id: Option<&str>,
        entry: &ScopeAllowance,
    ) {
        let warn_at = entry.warn_at.unwrap_or(self.default_warn_at);
        out.push(Ceiling {
            scope_kind,
            limit_kind: AllowanceLimitKind::Window,
            tenant_id: tenant_id.to_string(),
            api_key_id: api_key_id.map(str::to_string),
            period_secs: Some(entry.period_secs),
            ceiling_nanos: entry.amount_nanos,
            warn_at,
        });
        if let Some(lifetime) = entry.lifetime_nanos {
            out.push(Ceiling {
                scope_kind,
                limit_kind: AllowanceLimitKind::Lifetime,
                tenant_id: tenant_id.to_string(),
                api_key_id: api_key_id.map(str::to_string),
                period_secs: None,
                ceiling_nanos: lifetime,
                warn_at,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::principal::TenantId;

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is valid")
    }

    fn subject(tenant: &str, key: &str) -> RunAttribution {
        RunAttribution::new(TenantId::new(tenant).expect("valid tenant"), key)
    }

    #[test]
    fn ceilings_come_in_the_fixed_order() {
        let policy = AllowancePolicy::new(usd(), 80)
            .with_api_key(
                "svc-a",
                ScopeAllowance::new(3_600, 10)
                    .with_lifetime(100)
                    .with_warn_at(50),
            )
            .with_tenant("acme", ScopeAllowance::new(86_400, 20).with_lifetime(200));
        let ceilings = policy.ceilings_for(&subject("acme", "svc-a"));
        let shape: Vec<_> = ceilings
            .iter()
            .map(|c| (c.scope_kind, c.limit_kind, c.ceiling_nanos, c.warn_at))
            .collect();
        assert_eq!(
            shape,
            vec![
                (
                    AllowanceScopeKind::ApiKey,
                    AllowanceLimitKind::Window,
                    10,
                    50
                ),
                (
                    AllowanceScopeKind::ApiKey,
                    AllowanceLimitKind::Lifetime,
                    100,
                    50
                ),
                (
                    AllowanceScopeKind::Tenant,
                    AllowanceLimitKind::Window,
                    20,
                    80
                ),
                (
                    AllowanceScopeKind::Tenant,
                    AllowanceLimitKind::Lifetime,
                    200,
                    80
                ),
            ]
        );
        assert_eq!(ceilings[0].api_key_id.as_deref(), Some("svc-a"));
        assert_eq!(ceilings[2].api_key_id, None);
        assert_eq!(ceilings[1].period_secs, None);
        assert_eq!(ceilings[0].period_secs, Some(3_600));
    }

    #[test]
    fn absent_entry_yields_no_ceiling() {
        let policy =
            AllowancePolicy::new(usd(), 80).with_api_key("svc-a", ScopeAllowance::new(60, 1));
        assert!(policy.ceilings_for(&subject("acme", "svc-b")).is_empty());
        assert!(!policy.is_empty());
        assert!(AllowancePolicy::new(usd(), 80).is_empty());
        // A key entry is matched by name only; another tenant's same-named key is the same
        // configured key (key names are unique across the auth config).
        assert_eq!(policy.ceilings_for(&subject("globex", "svc-a")).len(), 1);
    }

    #[test]
    fn tenant_entry_applies_to_every_key_of_the_tenant() {
        let policy =
            AllowancePolicy::new(usd(), 80).with_tenant("acme", ScopeAllowance::new(60, 5));
        assert_eq!(policy.ceilings_for(&subject("acme", "any")).len(), 1);
        assert!(policy.ceilings_for(&subject("globex", "any")).is_empty());
    }
}
