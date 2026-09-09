//! Infrastructure Layer (facade)
//!
//! Infrastructure wiring for the Paladin facade crate. Most adapter **implementations** now live
//! in dedicated leaf crates; this module hosts the adapters that remain facade-resident and
//! re-exports the leaf-crate adapters so they are reachable under
//! `crate::infrastructure::adapters::…` for the composition root, examples, and tests.
//!
//! # Where the adapters live
//!
//! | Concern | Crate / location |
//! |---------|------------------|
//! | LLM providers (OpenAI, Anthropic, DeepSeek, mock) | [`paladin_llm`] (facade keeps only `adapters::llm::config_bridge`) |
//! | Garrison / Sanctum memory | [`paladin_memory`] (re-exported via `adapters::garrison`, `adapters::sanctum`) |
//! | Citadel state persistence | [`paladin_memory`]`::citadel` (re-exported via `adapters::citadel`) |
//! | Repositories (SQLite/MySQL) | [`paladin_storage`] (re-exported via [`repositories`]) |
//! | MinIO/S3 file storage (`s3-storage`) | [`paladin_storage`]`::minio` (re-exported via `adapters::file_storage`) |
//! | Redis queue (`redis-queue`) | [`paladin_storage`]`::redis` (re-exported via `adapters::queue`) |
//! | Content ingestion / documents | [`paladin_content`] (re-exported via `adapters::input`, `adapters::document`) |
//! | Notifications (`notifications`) | [`paladin_notifications`] (re-exported via `adapters::notifications`) |
//! | Web / API (`web-server`) | [`paladin_web`] (re-exported via [`web`]) |
//! | Herald output formatters | [`paladin_herald`] (re-exported via `adapters::herald`) |
//!
//! # Facade-resident adapters
//!
//! - **`adapters::arsenal`** — MCP (Model Context Protocol) client, STDIO/SSE transports, resource
//!   controls, and tool-result formatting ([`ArsenalPort`](paladin_ports::output::arsenal_port::ArsenalPort)).
//! - **`adapters::auth`** — in-memory token auth adapter.
//! - **`adapters::logs`** — system log adapter ([`LogPort`](paladin_ports::output::log_port::LogPort)).
//! - **`adapters::scheduling`** — Tokio-based cron scheduler.
//! - [`resilience`] — circuit breaker and related fault-tolerance utilities.
//! - [`security`] — encryption, audit, and TLS-verification helpers (cross-cutting).
//!
//! # Feature flags
//!
//! - `redis-queue` — Redis queue adapter (via `paladin-storage/redis-queue`)
//! - `s3-storage` — MinIO/S3 storage adapter (via `paladin-storage/s3`)
//! - `content-processing`, `notifications`, `web-server`, `storage-mysql` — enable the
//!   corresponding leaf-crate adapters and their re-exports.

// Internal modules (public for testing, not part of stable API)
#[allow(missing_docs)]
pub mod adapters;
#[allow(missing_docs)]
pub mod repositories;
pub mod resilience;
#[allow(missing_docs)]
pub mod security;
/// The facade's trace-sink composition module (OBS-02, D-10, D-11): the
/// default-on structured [`telemetry::LogTraceSink`] and
/// [`telemetry::build_run_sink`], the single place a run's sink fan-out is
/// assembled.
pub mod telemetry;
#[cfg(feature = "web-server")]
#[allow(missing_docs)]
pub mod web;
