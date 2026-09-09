//! Application router composition for the REST API.
//!
//! [`create_app_router`](crate::app::create_app_router) wires together public routes
//! (registration and login)
//! and authenticated routes (everything else) using the auth middleware from
//! [`crate::auth_middleware`]. Admin-only routes (user deletion and listing) are
//! additionally protected by the admin guard, while self-scoped routes (fetch
//! and update by id) enforce ownership inside their handlers. The content-delivery
//! routes from [`crate::delivery_controller`] are merged in as public routes.

use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
};
use paladin_core::platform::manager::user_service::UserServiceTrait;
use paladin_ports::output::auth_port::AuthPort;

use crate::adapters::api_content_deliverer::ApiContentDeliverer;
use crate::agent_controller::{AgentApiState, agent_router};
use crate::auth_middleware::{require_admin, require_auth};
use crate::delivery_controller::create_delivery_routes;
#[cfg(feature = "dev-ui")]
use crate::dev_ui_controller::{DevUiState, dev_ui_inspector_page};
use crate::user_controller::{
    delete_user, get_user, list_users, login_user, register_user, update_user_profile,
};

/// Build the complete application router (user management + content delivery).
///
/// Public routes (`POST /users/register`, `POST /users/login`, and the
/// `/api/delivery/*` endpoints) require no authentication. All other user routes
/// require a valid bearer token verified by `auth_port`. The admin-only routes
/// (`GET /users`, `DELETE /users/:id`) are further restricted to users holding the
/// `Admin` role.
pub fn create_app_router(
    user_service: Arc<dyn UserServiceTrait>,
    auth_port: Arc<dyn AuthPort>,
    deliverer: Arc<ApiContentDeliverer>,
) -> Router {
    let public_routes = Router::new()
        .route("/users/register", post(register_user))
        .route("/users/login", post(login_user))
        .with_state(user_service.clone());

    let protected_routes = Router::new()
        .route("/users/{id}", get(get_user))
        .route("/users/{id}", put(update_user_profile))
        .with_state(user_service.clone())
        .layer(from_fn_with_state(auth_port.clone(), require_auth));

    let admin_routes = Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}", delete(delete_user))
        .with_state(user_service)
        .layer(axum::middleware::from_fn(require_admin))
        .layer(from_fn_with_state(auth_port, require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .merge(create_delivery_routes(deliverer))
}

/// Build the feature-gated `dev-ui` run inspector router (`GET
/// /v1/dev-ui/threads/{id}`, 28-15, D-25/D-26).
///
/// Admin-gated under the SAME [`require_auth`]/[`require_admin`] middleware layers
/// [`create_app_router`]'s `admin_routes` block above already uses (reused verbatim,
/// not reimplemented) -- an unauthenticated request is rejected `401` and an
/// authenticated non-admin request `403`, both before
/// [`crate::dev_ui_controller::dev_ui_inspector_page`] ever runs. Compiled only when the
/// `dev-ui` cargo feature is enabled (default off): the route is a plain
/// [`axum::Router`], never an `OpenApiRouter`, and carries no `#[utoipa::path]`
/// attribute, so it is structurally absent from `crates/paladin-web/openapi.json`
/// regardless of whether this feature is compiled (D-25).
///
/// Not merged into [`create_app_router`] by default -- production wiring of a
/// [`paladin_ports::input::run_inspector_port::RunInspectorPort`] backend into a running
/// server binary is a separate concern from this crate's own router composition (mirrors
/// how `paladin-server.rs` merges `thread_router`/`run_router` alongside `agent_router`
/// rather than inside `create_app_router`).
#[cfg(feature = "dev-ui")]
pub fn create_dev_ui_router(auth_port: Arc<dyn AuthPort>, state: DevUiState) -> Router {
    Router::new()
        .route("/v1/dev-ui/threads/{id}", get(dev_ui_inspector_page))
        .with_state(state)
        .layer(axum::middleware::from_fn(require_admin))
        .layer(from_fn_with_state(auth_port, require_auth))
}

/// Build the application router and additionally mount the agent-execution API.
///
/// This is [`create_app_router`] plus the `/agents/*` routes from
/// [`agent_router`], merged in. It is the
/// composition entry point for the HTTP service-host topology (Milestone 12): the
/// server binary (Epic 2) builds the [`AgentApiState`] — including the concrete
/// executor and optional provisioner — and passes it here.
///
/// The agent routes are unauthenticated in Epic 1; Epic 5 layers auth on without
/// changing this composition.
pub fn create_app_router_with_agents(
    user_service: Arc<dyn UserServiceTrait>,
    auth_port: Arc<dyn AuthPort>,
    deliverer: Arc<ApiContentDeliverer>,
    agent_state: AgentApiState,
) -> Router {
    create_app_router(user_service, auth_port, deliverer).merge(agent_router(agent_state))
}
