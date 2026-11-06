//! Google Gemini (text-only) provider adapter and related configuration.
//!
//! Gemini's `generateContent` protocol is not OpenAI-compatible (D-08) — this
//! module implements [`paladin_ports::output::llm_port::LlmPort`] directly
//! against Gemini's own wire shape rather than delegating to
//! [`crate::compat::CompatEngine`]. See `adapter`'s module-level
//! documentation for the protocol divergences this adapter accounts for.

pub mod adapter;

pub use adapter::{GeminiAdapter, GeminiConfig};
