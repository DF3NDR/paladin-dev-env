//! Configuration for agents served by the HTTP service host (Milestone 12, Epic 2).
//!
//! The top-level `agents:` key in `config.yml` is a list of [`AgentDefinition`]s. The
//! `paladin-server` binary turns each definition into a resident agent in the
//! `paladin_web::AgentRegistry` (see the facade `infrastructure::web` builder).
//!
//! Secrets (API keys) are **never** read from these definitions — they come from the
//! `llm:` provider configuration and the corresponding environment variables.

use paladin_core::platform::container::user::UserRole;
use serde::{Deserialize, Serialize};

/// Server-wide execution timeout configuration for the HTTP service host.
///
/// Maps onto `paladin_web::TimeoutPolicy`. Absent fields fall back to the defaults
/// (300s default, 600s max).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTimeoutsConfig {
    /// Default execution timeout (seconds) when neither request nor agent specifies one.
    #[serde(default = "default_timeout_seconds")]
    pub default_seconds: u64,
    /// Maximum execution timeout (seconds); per-request/agent values are clamped to it.
    #[serde(default = "default_max_timeout_seconds")]
    pub max_seconds: u64,
}

fn default_timeout_seconds() -> u64 {
    300
}

fn default_max_timeout_seconds() -> u64 {
    600
}

impl Default for AgentTimeoutsConfig {
    fn default() -> Self {
        Self {
            default_seconds: default_timeout_seconds(),
            max_seconds: default_max_timeout_seconds(),
        }
    }
}

/// Per-client rate-limit settings for the HTTP service host (off by default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether the rate limiter is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Sustained requests per second allowed per client IP.
    #[serde(default = "default_rate_per_second")]
    pub per_second: u64,
    /// Burst capacity per client IP.
    #[serde(default = "default_rate_burst")]
    pub burst: u32,
}

fn default_rate_per_second() -> u64 {
    10
}

fn default_rate_burst() -> u32 {
    20
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            per_second: default_rate_per_second(),
            burst: default_rate_burst(),
        }
    }
}

/// One static API key mapped to a principal (`name` + `role`).
///
/// The `key` should come from an environment variable / secret in practice, not be
/// committed in plaintext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// The secret key value presented in the `X-API-Key` header.
    pub key: String,
    /// A stable identifier for the caller (used as the principal id; appears in logs).
    pub name: String,
    /// The role granted to requests authenticated with this key.
    pub role: UserRole,
    /// The tenant this key's principal belongs to (required; an empty value fails boot,
    /// D-05). A plain identifier: non-empty, no whitespace, printable ASCII, at most 128
    /// bytes.
    #[serde(default)]
    pub tenant: String,
}

/// Opaque server-issued bearer-token settings for the agent API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BearerTokenAuthConfig {
    /// Whether to accept `Authorization: Bearer` tokens via the wired `AuthPort`.
    #[serde(default)]
    pub enabled: bool,
    /// The tenant every verified bearer principal carries (D-03). Required when `enabled`
    /// is `true`; a missing or empty value fails boot.
    #[serde(default)]
    pub tenant: Option<String>,
}

/// Authentication configuration for the agent API (maps onto `paladin_web::AgentAuthConfig`).
///
/// `enabled` defaults to **true** (secure by default); the server fails closed when auth is
/// enabled but no credential source (API keys or bearer token) is configured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Whether authentication is enforced on the agent routes.
    #[serde(default = "default_auth_enabled")]
    pub enabled: bool,
    /// Static API keys accepted via the `X-API-Key` header.
    #[serde(default)]
    pub api_keys: Vec<ApiKeyConfig>,
    /// Opaque server-issued bearer-token settings.
    #[serde(default)]
    pub bearer_token: BearerTokenAuthConfig,
}

fn default_auth_enabled() -> bool {
    true
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: default_auth_enabled(),
            api_keys: Vec::new(),
            bearer_token: BearerTokenAuthConfig::default(),
        }
    }
}

/// Interactive API-docs settings (OpenAPI spec + Swagger UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsConfig {
    /// Whether to serve `GET /openapi.json` and the Swagger UI at `/docs`.
    #[serde(default = "default_docs_enabled")]
    pub enabled: bool,
}

fn default_docs_enabled() -> bool {
    true
}

impl Default for DocsConfig {
    fn default() -> Self {
        Self {
            enabled: default_docs_enabled(),
        }
    }
}

/// Cross-cutting HTTP layer configuration (CORS, body limit, global timeout, rate limit, auth).
///
/// Maps onto `paladin_web::HttpLayersConfig` (+ `AgentAuthConfig`); absent fields use safe
/// defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebHttpConfig {
    /// Allowed CORS origins; empty ⇒ permissive (suitable for local dev).
    #[serde(default)]
    pub cors_allow_origins: Vec<String>,
    /// Maximum request body size in bytes.
    #[serde(default = "default_body_limit_bytes")]
    pub body_limit_bytes: usize,
    /// Global request timeout (seconds) for non-streaming routes; `0` disables it.
    #[serde(default)]
    pub global_timeout_seconds: u64,
    /// Rate-limit settings.
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    /// Authentication settings (enabled by default).
    #[serde(default)]
    pub auth: AuthConfig,
    /// Interactive API-docs settings (enabled by default).
    #[serde(default)]
    pub docs: DocsConfig,
}

fn default_body_limit_bytes() -> usize {
    1024 * 1024
}

impl Default for WebHttpConfig {
    fn default() -> Self {
        Self {
            cors_allow_origins: Vec::new(),
            body_limit_bytes: default_body_limit_bytes(),
            global_timeout_seconds: 0,
            rate_limit: RateLimitConfig::default(),
            auth: AuthConfig::default(),
            docs: DocsConfig::default(),
        }
    }
}

/// Declarative definition of one agent to load into the HTTP service host.
///
/// `id`, `model`, and `system_prompt` are required; everything else is optional and
/// falls back to a provider/builder default. Optional fields use `#[serde(default)]`
/// so new fields can be added without breaking existing configs.
///
/// # Example (YAML)
///
/// ```yaml
/// agents:
///   - id: "researcher"
///     provider: "openai"      # optional; defaults to llm.default_provider
///     model: "gpt-4"
///     system_prompt: "You research topics thoroughly."
///     temperature: 0.7
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique registry id — the `{id}` path segment in `/agents/{id}/…`.
    pub id: String,

    /// LLM model identifier (e.g. `"gpt-4"`).
    pub model: String,

    /// System prompt defining the agent's behavior.
    pub system_prompt: String,

    /// Provider name (e.g. `"openai"`, `"anthropic"`, `"deepseek"`).
    ///
    /// When absent, the server falls back to `llm.default_provider`.
    #[serde(default)]
    pub provider: Option<String>,

    /// Response randomness (`0.0`–`1.0`). When absent, the builder default applies.
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Maximum reasoning loops. When absent, the builder default applies.
    #[serde(default)]
    pub max_loops: Option<u32>,

    /// Tokens that signal the agent to stop processing.
    #[serde(default)]
    pub stop_words: Vec<String>,

    /// Per-agent execution timeout (seconds). When absent, the server default applies.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    /// Roles permitted to invoke this agent; empty/absent ⇒ any authenticated caller.
    #[serde(default)]
    pub allowed_roles: Vec<UserRole>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_full_definition() {
        let json = serde_json::json!({
            "id": "researcher",
            "model": "gpt-4",
            "system_prompt": "You research topics.",
            "provider": "openai",
            "temperature": 0.7,
            "max_loops": 5,
            "stop_words": ["STOP"]
        });
        let def: AgentDefinition = serde_json::from_value(json).expect("parses");
        assert_eq!(def.id, "researcher");
        assert_eq!(def.model, "gpt-4");
        assert_eq!(def.system_prompt, "You research topics.");
        assert_eq!(def.provider.as_deref(), Some("openai"));
        assert_eq!(def.temperature, Some(0.7));
        assert_eq!(def.max_loops, Some(5));
        assert_eq!(def.stop_words, vec!["STOP".to_string()]);
    }

    #[test]
    fn deserializes_minimal_definition_with_defaults() {
        // Only the three required fields are present.
        let json = serde_json::json!({
            "id": "summarizer",
            "model": "gpt-4",
            "system_prompt": "You summarize."
        });
        let def: AgentDefinition = serde_json::from_value(json).expect("parses");
        assert_eq!(def.id, "summarizer");
        assert!(def.provider.is_none());
        assert!(def.temperature.is_none());
        assert!(def.max_loops.is_none());
        assert!(def.stop_words.is_empty());
    }

    #[test]
    fn missing_required_field_fails() {
        // No `system_prompt` → must not deserialize.
        let json = serde_json::json!({ "id": "x", "model": "gpt-4" });
        let result: Result<AgentDefinition, _> = serde_json::from_value(json);
        assert!(result.is_err(), "missing required field must fail");
    }

    #[test]
    fn definition_parses_allowed_roles() {
        let json = serde_json::json!({
            "id": "x", "model": "gpt-4", "system_prompt": "p",
            "allowed_roles": ["admin", "user"]
        });
        let def: AgentDefinition = serde_json::from_value(json).expect("parses");
        assert_eq!(def.allowed_roles, vec![UserRole::Admin, UserRole::User]);
    }

    #[test]
    fn auth_config_defaults_to_enabled_with_no_credentials() {
        // An empty `auth:` section ⇒ enabled, no keys, bearer token off (secure default).
        let auth: AuthConfig = serde_json::from_value(serde_json::json!({})).expect("parses");
        assert!(auth.enabled);
        assert!(auth.api_keys.is_empty());
        assert!(!auth.bearer_token.enabled);
    }

    #[test]
    fn auth_config_parses_api_keys_with_roles() {
        let json = serde_json::json!({
            "enabled": true,
            "api_keys": [
                { "key": "sk-1", "name": "ci", "role": "admin", "tenant": "platform-ops" },
                { "key": "sk-2", "name": "fe", "role": "user", "tenant": "web-app" }
            ],
            "bearer_token": { "enabled": true, "tenant": "bearer-tenant" }
        });
        let auth: AuthConfig = serde_json::from_value(json).expect("parses");
        assert_eq!(auth.api_keys.len(), 2);
        assert_eq!(auth.api_keys[0].role, UserRole::Admin);
        assert_eq!(auth.api_keys[1].role, UserRole::User);
        assert!(auth.bearer_token.enabled);
    }

    #[test]
    fn auth_config_rejects_unknown_role() {
        let json = serde_json::json!({
            "api_keys": [ { "key": "sk", "name": "x", "role": "superuser", "tenant": "t" } ]
        });
        let result: Result<AuthConfig, _> = serde_json::from_value(json);
        assert!(result.is_err(), "unknown role must fail to parse");
    }

    // --- Phase 40 (TENANT-01, D-05): api_keys[].tenant / bearer_token.tenant ---

    #[test]
    fn auth_config_parses_api_key_tenant_and_bearer_tenant() {
        let json = serde_json::json!({
            "enabled": true,
            "api_keys": [
                { "key": "sk-1", "name": "ci", "role": "admin", "tenant": "acme" }
            ],
            "bearer_token": { "enabled": true, "tenant": "acme-bearer" }
        });
        let auth: AuthConfig = serde_json::from_value(json).expect("parses");
        assert_eq!(auth.api_keys[0].tenant, "acme");
        assert_eq!(auth.bearer_token.tenant.as_deref(), Some("acme-bearer"));
    }

    #[test]
    fn api_key_without_tenant_parses_to_an_empty_tenant() {
        // serde default keeps the file parseable so `build_auth_config` can name the key
        // in its fail-closed error (D-05) -- validation happens at boot, not at parse time.
        let json = serde_json::json!({
            "api_keys": [ { "key": "sk", "name": "x", "role": "user" } ]
        });
        let auth: AuthConfig = serde_json::from_value(json).expect("parses");
        assert_eq!(auth.api_keys[0].tenant, "");
    }

    // --- Phase 40 Plan 03 (D-03/D-05/D-06): AuthConfig::validate() fails closed ---

    fn api_key(name: &str, key: &str, tenant: &str) -> ApiKeyConfig {
        ApiKeyConfig {
            key: key.to_string(),
            name: name.to_string(),
            role: UserRole::User,
            tenant: tenant.to_string(),
        }
    }

    fn auth_with_keys(api_keys: Vec<ApiKeyConfig>) -> AuthConfig {
        AuthConfig {
            enabled: true,
            api_keys,
            bearer_token: BearerTokenAuthConfig::default(),
        }
    }

    fn validation_error(cfg: &AuthConfig) -> String {
        cfg.validate()
            .expect_err("this config must fail validation")
    }

    #[test]
    fn default_auth_config_validates() {
        assert_eq!(AuthConfig::default().validate(), Ok(()));
    }

    #[test]
    fn validate_accepts_two_keys_mapped_to_the_same_tenant() {
        // D-06 adjacency: different names, same tenant -- two principals, one read scope.
        let cfg = auth_with_keys(vec![
            api_key("ci", "sk-ci", "acme"),
            api_key("web", "sk-web", "acme"),
        ]);
        assert_eq!(cfg.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_an_api_key_without_a_tenant() {
        let cfg = auth_with_keys(vec![api_key("ci", "sk-ci", "")]);
        let err = validation_error(&cfg);
        assert_eq!(
            err,
            "http.auth.api_keys[ci]: 'tenant' is required — every API key must map to a tenant \
             (Phase 40, TENANT-01)"
        );
        assert!(!err.contains("sk-ci"), "never print the key value: {err}");
    }

    #[test]
    fn validate_rejects_a_whitespace_or_non_printable_tenant() {
        // Nothing is trimmed or defaulted: a leading/trailing space is as fatal as an inner one.
        for tenant in [" ", " acme", "acme ", "ac me", "acme\t", "acmé"] {
            let cfg = auth_with_keys(vec![api_key("ci", "sk-ci", tenant)]);
            let err = validation_error(&cfg);
            assert!(
                err.starts_with("http.auth.api_keys[ci]: 'tenant' is not a valid tenant id: "),
                "tenant {tenant:?} must be rejected as invalid, got: {err}"
            );
            assert!(!err.contains("sk-ci"), "never print the key value: {err}");
        }
    }

    #[test]
    fn validate_rejects_a_duplicate_key_name() {
        let cfg = auth_with_keys(vec![
            api_key("ci", "sk-1", "acme"),
            api_key("ci", "sk-2", "globex"),
        ]);
        let err = validation_error(&cfg);
        assert!(
            err.starts_with("http.auth.api_keys[ci]: duplicate 'name'"),
            "expected the duplicate-name message, got: {err}"
        );
        assert!(!err.contains("sk-1") && !err.contains("sk-2"), "never print key values: {err}");
    }

    #[test]
    fn validate_rejects_a_duplicate_key_value_without_printing_it() {
        // T-40-12 / T-40-13: a shared secret would make `lookup_api_key` resolve an arbitrary
        // principal (and tenant); it is rejected at boot and the secret itself never appears.
        let cfg = auth_with_keys(vec![
            api_key("ci", "sk-dup", "acme"),
            api_key("web", "sk-dup", "acme"),
        ]);
        let err = validation_error(&cfg);
        assert!(
            err.starts_with("http.auth.api_keys[web]: duplicate 'key'"),
            "expected the duplicate-key message naming the second entry, got: {err}"
        );
        assert!(err.contains("'ci'"), "must name the first key holding the same secret: {err}");
        assert!(!err.contains("sk-dup"), "the key value must never be printed: {err}");
    }

    #[test]
    fn validate_rejects_an_empty_key_value() {
        // T-40-14: an empty string must never become an accepted credential.
        let cfg = auth_with_keys(vec![
            api_key("ok", "sk-ok", "acme"),
            api_key("ci", "", "acme"),
        ]);
        assert_eq!(
            validation_error(&cfg),
            "http.auth.api_keys[#1]: 'key' must not be empty"
        );
    }

    #[test]
    fn validate_rejects_an_invalid_key_name() {
        // `name` is the API key id on every run and ledger row: same identifier rules as a tenant.
        let cfg = auth_with_keys(vec![api_key("ci runner", "sk-ci", "acme")]);
        let err = validation_error(&cfg);
        assert!(
            err.starts_with("http.auth.api_keys[#0]: 'name' is not a valid API key id: "),
            "expected the invalid-name message, got: {err}"
        );
        assert!(!err.contains("sk-ci"), "never print the key value: {err}");
    }

    #[test]
    fn validate_requires_a_bearer_tenant_when_bearer_is_enabled() {
        for tenant in [None, Some(String::new())] {
            let cfg = AuthConfig {
                enabled: true,
                api_keys: Vec::new(),
                bearer_token: BearerTokenAuthConfig {
                    enabled: true,
                    tenant,
                },
            };
            let err = validation_error(&cfg);
            assert!(
                err.starts_with(
                    "http.auth.bearer_token: 'tenant' is required when bearer_token.enabled is true"
                ),
                "expected the bearer tenant-required message, got: {err}"
            );
        }

        let cfg = AuthConfig {
            enabled: true,
            api_keys: Vec::new(),
            bearer_token: BearerTokenAuthConfig {
                enabled: true,
                tenant: Some("bearer callers".to_string()),
            },
        };
        assert!(
            validation_error(&cfg)
                .starts_with("http.auth.bearer_token: 'tenant' is not a valid tenant id: ")
        );
    }

    #[test]
    fn validate_ignores_the_bearer_tenant_when_bearer_is_disabled() {
        for tenant in [None, Some(String::new()), Some("bad tenant".to_string())] {
            let cfg = AuthConfig {
                enabled: true,
                api_keys: vec![api_key("ci", "sk-ci", "acme")],
                bearer_token: BearerTokenAuthConfig {
                    enabled: false,
                    tenant,
                },
            };
            assert_eq!(cfg.validate(), Ok(()), "a disabled bearer section is not validated");
        }
    }

    #[test]
    fn validate_checks_api_keys_even_when_auth_is_disabled() {
        // A disabled section's keys take effect the moment auth is re-enabled, so a malformed
        // mapping is rejected regardless of `enabled`.
        let mut cfg = auth_with_keys(vec![api_key("ci", "sk-ci", "")]);
        cfg.enabled = false;
        assert!(cfg.validate().is_err());
    }
}
