//! Authentication & authorization for the agent API (Milestone 12, Epic 5).
//!
//! Two credential types are accepted when auth is enabled:
//!
//! - **API key** via the `X-API-Key` header, matched (constant-time) against a configured
//!   map of key → [`Principal`];
//! - **Opaque server-issued bearer token** via `Authorization: Bearer <token>`, verified by
//!   the injected `AuthPort` against the server's own in-process token store — not a
//!   signed or self-describing token.
//!
//! On success a [`Principal`] (role + identifier) is attached to the request for the
//! handlers' authorization checks (per-agent `allowed_roles`, admin gate). Failures render
//! as [`ApiError`] (`401`/`403`).
//!
//! ## Default posture
//!
//! [`AgentAuthConfig::default`] is **disabled** (open) so library/embedded use and the
//! existing tests are unaffected. The *server* (`paladin-server`) enables auth by default
//! and fails closed when no credentials are configured — that posture lives in the binary,
//! not here.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use paladin_core::platform::container::principal::{PrincipalRef, RunReadScope, TenantId};
use paladin_core::platform::container::user::UserRole;
use paladin_ports::output::auth_port::AuthPort;

use crate::agent_controller::AgentApiState;
use crate::error::ApiError;

/// An authenticated caller: an identifier, tenant and role used for authorization and
/// run-read scoping (Phase 40, TENANT-01, D-01).
///
/// `tenant_id` is required (no `Option`) -- every authenticated principal belongs to
/// exactly one tenant, derived server-side from [`AgentAuthConfig`] inside [`authenticate`]
/// and never from the request (D-02). `#[non_exhaustive]`: constructed only through
/// [`Principal::new`], so the next field this type gains stays non-breaking (X-10.3, the
/// `LlmRequest`/`GarrisonEntry` precedent).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Principal {
    /// Stable identifier (API-key name or opaque bearer-token subject).
    pub id: String,
    /// Role used for per-agent and admin authorization.
    pub role: UserRole,
    /// The tenant this principal belongs to -- derived server-side from `AgentAuthConfig`,
    /// never from the request (D-01, D-02).
    pub tenant_id: TenantId,
}

impl Principal {
    /// Construct a `Principal`.
    pub fn new(id: impl Into<String>, role: UserRole, tenant_id: TenantId) -> Self {
        Self {
            id: id.into(),
            role,
            tenant_id,
        }
    }

    /// The principal attached when auth is disabled: full access (`Admin`), carrying the
    /// documented open-access sentinel tenant (D-03) -- deployment-wide reads happen
    /// through the Admin arm of [`RunReadScope::for_principal`] (D-11), not a bypass.
    fn open_access() -> Self {
        Self::new("anonymous", UserRole::Admin, TenantId::open_access())
    }

    /// Derive this principal's run-read scope (D-12): `All` for `Admin`, otherwise scoped
    /// to its own tenant.
    pub fn read_scope(&self) -> RunReadScope {
        RunReadScope::for_principal(self.role, &self.tenant_id)
    }
}

impl From<&Principal> for PrincipalRef {
    fn from(principal: &Principal) -> Self {
        PrincipalRef::new(
            principal.id.clone(),
            principal.tenant_id.clone(),
            principal.role,
        )
    }
}

/// Authentication configuration for the agent routes.
#[derive(Clone)]
pub struct AgentAuthConfig {
    /// When `false`, requests pass through with full access (open mode).
    pub enabled: bool,
    /// API key → principal map.
    pub api_keys: HashMap<String, Principal>,
    /// Optional opaque bearer-token verifier (the `AuthPort` implementation is injected by
    /// the binary).
    pub token_verifier: Option<Arc<dyn AuthPort>>,
    /// The tenant every verified bearer principal carries (D-03), set from
    /// `http.auth.bearer_token.tenant`. While `None`, a verified bearer token does NOT
    /// authenticate (fail closed) -- every principal must carry a tenant (D-01).
    pub bearer_tenant: Option<TenantId>,
}

impl Default for AgentAuthConfig {
    /// Disabled (open) — see the module docs for why the *library* default is permissive.
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: HashMap::new(),
            token_verifier: None,
            bearer_tenant: None,
        }
    }
}

impl AgentAuthConfig {
    /// Whether any credential source is configured (used for the server's fail-closed check).
    pub fn has_credentials(&self) -> bool {
        !self.api_keys.is_empty() || self.token_verifier.is_some()
    }
}

/// Trait for extracting the shared [`AgentAuthConfig`] from router state, so
/// [`require_authentication`] can be reused, unmodified in behavior, by every
/// stateful router in this crate rather than each duplicating its ~15 lines.
///
/// Added alongside `thread_controller` (Phase 24 Plan 11, D-24): the thread
/// routes' `ThreadApiState` layers the SAME `require_authentication` function
/// `AgentApiState`'s routes already use, via this trait rather than a second,
/// copy-pasted middleware.
pub trait HasAgentAuth {
    /// Borrow this state's [`AgentAuthConfig`].
    fn agent_auth(&self) -> &AgentAuthConfig;
}

impl HasAgentAuth for AgentApiState {
    fn agent_auth(&self) -> &AgentAuthConfig {
        &self.auth
    }
}

/// Constant-time byte equality (length difference short-circuits).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Extract a non-empty bearer token from `Authorization: Bearer …`.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

/// Resolve a configured API key to its principal (constant-time compare).
fn lookup_api_key(keys: &HashMap<String, Principal>, presented: &str) -> Option<Principal> {
    keys.iter()
        .find(|(k, _)| ct_eq(k.as_bytes(), presented.as_bytes()))
        .map(|(_, p)| p.clone())
}

/// Authenticate a request from its headers (opaque bearer token checked first, then API key).
///
/// # Errors
///
/// Returns `401` ([`ApiError`]) when no presented credential is valid. The message is the
/// same for all failure modes and never echoes the credential.
pub async fn authenticate(
    headers: &HeaderMap,
    config: &AgentAuthConfig,
) -> Result<Principal, ApiError> {
    // 1. Opaque bearer token (only when a verifier is configured AND a bearer tenant is
    //    configured -- a verified bearer token with no configured tenant does not
    //    authenticate; every principal must carry a tenant (D-01, D-03, fail closed)).
    if let (Some(token), Some(verifier), Some(tenant)) = (
        bearer_token(headers),
        config.token_verifier.as_ref(),
        config.bearer_tenant.as_ref(),
    ) && let Ok(claims) = verifier.verify_token(token).await
    {
        return Ok(Principal::new(
            claims.user_id.to_string(),
            claims.role,
            tenant.clone(),
        ));
    }

    // 2. API key.
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok())
        && let Some(principal) = lookup_api_key(&config.api_keys, key)
    {
        return Ok(principal);
    }

    Err(ApiError::unauthorized("missing or invalid credentials"))
}

/// Axum middleware that authenticates the request and attaches a [`Principal`].
///
/// Generic over any router state exposing an [`AgentAuthConfig`] via
/// [`HasAgentAuth`] -- both `AgentApiState` (the agent routes) and
/// `thread_controller::ThreadApiState` (the thread routes, D-24) layer this
/// SAME function via `route_layer(axum::middleware::from_fn_with_state(...))`
/// rather than each carrying its own copy.
///
/// When auth is disabled it attaches an open-access principal and passes through; when
/// enabled it returns `401` on failure.
pub async fn require_authentication<S>(
    State(state): State<S>,
    mut request: Request,
    next: Next,
) -> Response
where
    S: HasAgentAuth + Clone + Send + Sync + 'static,
{
    let auth = state.agent_auth();
    if !auth.enabled {
        request.extensions_mut().insert(Principal::open_access());
        return next.run(request).await;
    }
    match authenticate(request.headers(), auth).await {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(err) => err.into_response(),
    }
}

/// Authorize invocation of an agent: `allowed_roles` empty ⇒ any authenticated caller,
/// otherwise the principal's role must be listed.
///
/// # Errors
///
/// Returns `403` ([`ApiError`]) when the role is not permitted.
pub fn authorize_invoke(principal: &Principal, allowed_roles: &[UserRole]) -> Result<(), ApiError> {
    if allowed_roles.is_empty() || allowed_roles.contains(&principal.role) {
        Ok(())
    } else {
        Err(ApiError::forbidden("role not permitted for this agent"))
    }
}

/// Require an admin principal (for runtime register/deregister).
///
/// # Errors
///
/// Returns `403` ([`ApiError`]) for non-admin principals.
pub fn require_admin(principal: &Principal) -> Result<(), ApiError> {
    if principal.role == UserRole::Admin {
        Ok(())
    } else {
        Err(ApiError::forbidden("admin role required"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::http::HeaderMap;
    use chrono::Utc;
    use paladin_core::platform::container::treasury_ledger::LedgerScope;
    use paladin_ports::output::auth_port::{AuthClaims, AuthError, AuthToken};
    use uuid::Uuid;

    /// Mock AuthPort: any token equal to `valid` verifies as a `User`.
    struct MockTokenVerifier {
        valid: String,
    }

    #[async_trait]
    impl AuthPort for MockTokenVerifier {
        async fn issue_token(&self, _u: Uuid, _r: UserRole) -> Result<AuthToken, AuthError> {
            Err(AuthError::Internal("unused".into()))
        }
        async fn verify_token(&self, token: &str) -> Result<AuthClaims, AuthError> {
            if token == self.valid {
                Ok(AuthClaims {
                    user_id: Uuid::nil(),
                    role: UserRole::User,
                    expires_at: Utc::now(),
                })
            } else {
                Err(AuthError::InvalidToken)
            }
        }
        async fn revoke_token(&self, _token: &str) -> Result<(), AuthError> {
            Ok(())
        }
    }

    fn config_with_key(key: &str, role: UserRole) -> AgentAuthConfig {
        config_with_key_and_tenant(key, role, "acme")
    }

    fn config_with_key_and_tenant(key: &str, role: UserRole, tenant: &str) -> AgentAuthConfig {
        let mut api_keys = HashMap::new();
        api_keys.insert(
            key.to_string(),
            Principal::new("svc", role, TenantId::new(tenant).unwrap()),
        );
        AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        }
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[tokio::test]
    async fn valid_api_key_authenticates() {
        let cfg = config_with_key("sk-abc", UserRole::User);
        let p = authenticate(&headers(&[("x-api-key", "sk-abc")]), &cfg)
            .await
            .expect("authenticates");
        assert_eq!(p.role, UserRole::User);
    }

    #[tokio::test]
    async fn missing_or_invalid_credential_is_401() {
        let cfg = config_with_key("sk-abc", UserRole::User);
        assert_eq!(
            authenticate(&HeaderMap::new(), &cfg)
                .await
                .unwrap_err()
                .status(),
            axum::http::StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            authenticate(&headers(&[("x-api-key", "wrong")]), &cfg)
                .await
                .unwrap_err()
                .status(),
            axum::http::StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn error_never_echoes_the_presented_credential() {
        // Secret hygiene: a rejected credential must not appear in the error body.
        let cfg = config_with_key("sk-abc", UserRole::User);
        let err = authenticate(&headers(&[("x-api-key", "super-secret-leak")]), &cfg)
            .await
            .unwrap_err();
        let body = err.to_body().to_string();
        assert!(
            !body.contains("super-secret-leak"),
            "error body must not echo the credential: {body}"
        );
    }

    #[tokio::test]
    async fn valid_bearer_token_authenticates() {
        let cfg = AgentAuthConfig {
            enabled: true,
            api_keys: HashMap::new(),
            token_verifier: Some(Arc::new(MockTokenVerifier {
                valid: "good-token".to_string(),
            })),
            bearer_tenant: Some(TenantId::new("bearer-tenant").unwrap()),
        };
        let p = authenticate(&headers(&[("authorization", "Bearer good-token")]), &cfg)
            .await
            .expect("authenticates");
        assert_eq!(p.role, UserRole::User);
        assert_eq!(p.tenant_id, TenantId::new("bearer-tenant").unwrap());

        assert!(
            authenticate(&headers(&[("authorization", "Bearer bad")]), &cfg)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn verified_bearer_token_without_configured_tenant_fails_closed() {
        let cfg = AgentAuthConfig {
            enabled: true,
            api_keys: HashMap::new(),
            token_verifier: Some(Arc::new(MockTokenVerifier {
                valid: "good-token".to_string(),
            })),
            bearer_tenant: None,
        };
        let err = authenticate(&headers(&[("authorization", "Bearer good-token")]), &cfg)
            .await
            .unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_token_is_resolved_before_api_key_and_carries_bearer_tenant() {
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "good-token".to_string(),
            Principal::new(
                "api-key-svc",
                UserRole::User,
                TenantId::new("api-tenant").unwrap(),
            ),
        );
        let cfg = AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: Some(Arc::new(MockTokenVerifier {
                valid: "good-token".to_string(),
            })),
            bearer_tenant: Some(TenantId::new("bearer-tenant").unwrap()),
        };
        // A request carrying both a verified bearer token and a value that ALSO happens to
        // match a configured API key name resolves to the bearer principal (lookup order
        // unchanged, D-03/D-00b).
        let p = authenticate(
            &headers(&[
                ("authorization", "Bearer good-token"),
                ("x-api-key", "good-token"),
            ]),
            &cfg,
        )
        .await
        .expect("authenticates");
        assert_eq!(p.tenant_id, TenantId::new("bearer-tenant").unwrap());
    }

    #[test]
    fn api_key_principal_carries_its_configured_tenant() {
        let cfg = config_with_key_and_tenant("sk-abc", UserRole::User, "acme");
        let principal = cfg.api_keys.get("sk-abc").unwrap();
        assert_eq!(principal.tenant_id, TenantId::new("acme").unwrap());
    }

    #[tokio::test]
    async fn tenant_is_derived_from_config_never_from_request_headers() {
        let cfg = config_with_key_and_tenant("sk-abc", UserRole::User, "acme");
        let p = authenticate(
            &headers(&[("x-api-key", "sk-abc"), ("x-tenant-id", "globex")]),
            &cfg,
        )
        .await
        .expect("authenticates");
        assert_eq!(p.tenant_id, TenantId::new("acme").unwrap());
    }

    #[tokio::test]
    async fn every_principal_source_yields_a_tenant() {
        // API key.
        let cfg = config_with_key_and_tenant("sk-abc", UserRole::User, "acme");
        let p = authenticate(&headers(&[("x-api-key", "sk-abc")]), &cfg)
            .await
            .unwrap();
        assert_eq!(p.tenant_id, TenantId::new("acme").unwrap());

        // Bearer.
        let bearer_cfg = AgentAuthConfig {
            enabled: true,
            api_keys: HashMap::new(),
            token_verifier: Some(Arc::new(MockTokenVerifier {
                valid: "good-token".to_string(),
            })),
            bearer_tenant: Some(TenantId::new("bearer-tenant").unwrap()),
        };
        let p = authenticate(
            &headers(&[("authorization", "Bearer good-token")]),
            &bearer_cfg,
        )
        .await
        .unwrap();
        assert_eq!(p.tenant_id, TenantId::new("bearer-tenant").unwrap());

        // Open access (auth disabled).
        let open = Principal::open_access();
        assert_eq!(open.tenant_id, TenantId::open_access());
        assert_eq!(open.role, UserRole::Admin);
    }

    #[test]
    fn read_scope_is_all_for_admin_and_tenant_for_user() {
        let admin = Principal::new("a", UserRole::Admin, TenantId::new("acme").unwrap());
        assert_eq!(admin.read_scope(), RunReadScope::All);
        let user = Principal::new("u", UserRole::User, TenantId::new("acme").unwrap());
        assert_eq!(
            user.read_scope(),
            RunReadScope::Tenant(TenantId::new("acme").unwrap())
        );
    }

    /// D-15/D-16: a principal's ledger scope is exactly `(tenant, id)` --
    /// the id is the key's configured name, never the secret.
    #[test]
    fn ledger_scope_is_the_principals_tenant_and_id() {
        let principal = Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap());
        assert_eq!(principal.ledger_scope(), LedgerScope::new("acme", "svc-a"));

        let open = Principal::open_access();
        assert_eq!(
            open.ledger_scope(),
            LedgerScope::new(TenantId::OPEN_ACCESS, "anonymous")
        );
        assert!(!open.ledger_scope().is_unattributed());
    }

    #[test]
    fn principal_ref_from_principal_copies_id_tenant_and_role() {
        let principal = Principal::new("svc-a", UserRole::Admin, TenantId::new("acme").unwrap());
        let principal_ref = PrincipalRef::from(&principal);
        assert_eq!(principal_ref.api_key_id, "svc-a");
        assert_eq!(principal_ref.tenant_id, TenantId::new("acme").unwrap());
        assert_eq!(principal_ref.role, UserRole::Admin);
    }

    #[test]
    fn authorize_invoke_respects_allowed_roles() {
        let user = Principal::new("u", UserRole::User, TenantId::new("t-1").unwrap());
        assert!(authorize_invoke(&user, &[]).is_ok()); // empty ⇒ any
        assert!(authorize_invoke(&user, &[UserRole::User]).is_ok());
        assert_eq!(
            authorize_invoke(&user, &[UserRole::Admin])
                .unwrap_err()
                .status(),
            axum::http::StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn require_admin_gates_non_admins() {
        let admin = Principal::new("a", UserRole::Admin, TenantId::new("t-1").unwrap());
        let user = Principal::new("u", UserRole::User, TenantId::new("t-1").unwrap());
        assert!(require_admin(&admin).is_ok());
        assert_eq!(
            require_admin(&user).unwrap_err().status(),
            axum::http::StatusCode::FORBIDDEN
        );
    }

    // --- Router-level wiring (auth applied to agent routes, health exempt) ---

    use crate::agent_controller::{AgentApiState, agent_router};
    use crate::agent_registry::AgentRegistry;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn authed_app(config: AgentAuthConfig) -> axum::Router {
        let state = AgentApiState::new(Arc::new(AgentRegistry::new())).with_auth(config);
        agent_router(state)
    }

    #[tokio::test]
    async fn agent_route_requires_credential_when_enabled() {
        let app = authed_app(config_with_key("sk-abc", UserRole::User));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn agent_route_accepts_valid_api_key() {
        let app = authed_app(config_with_key("sk-abc", UserRole::User));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/agents")
                    .header("x-api-key", "sk-abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_is_exempt_from_auth() {
        let app = authed_app(config_with_key("sk-abc", UserRole::User));
        for path in ["/health", "/ready"] {
            let resp = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{path} should be open");
        }
    }
}
