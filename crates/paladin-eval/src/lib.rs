//! # `paladin-eval` — a deterministic evaluation harness for Paladin agent graphs
//!
//! `paladin-eval` provides a serde scenario file format (`.eval.yaml`, see
//! [`scenario::Scenario`]), a scripted [`paladin_ports::output::llm_port::LlmPort`]
//! implementation ([`scripted_llm::ScenarioLlm`]) that makes a run deterministic without
//! touching a real provider, and (in later plans of Phase 28) an assertion library
//! evaluated over the captured trace record and the final `Battlefield`, plus a
//! `cargo test`-integrable runner.
//!
//! ## Hexagonal position (D-27)
//!
//! `paladin-eval` is a **composition-tier tool crate**, not an extracted leaf crate: it
//! depends downward on `paladin-core`, `paladin-ports`, `paladin-battalion`,
//! `paladin-llm` (`mock` feature) and `paladin-storage` (`sqlite` feature), and never on
//! the facade `paladin-ai` — the same shape `doc-examples` uses, recorded in
//! `.planning/decisions/0047-paladin-eval-composition-crate.md` so ADR-0031's
//! leaf-crate-independence invariant (scoped to the *extracted* leaf crates) is not
//! misread as violated by these edges. Nothing in the workspace depends on
//! `paladin-eval` except as a `[dev-dependencies]` entry — that is the crate's entire
//! purpose: a downstream team building their own agent graph on Paladin takes this
//! crate as a dev-dependency to write deterministic evaluation scenarios for it.
//!
//! ## Safety (T-28-05-01)
//!
//! A scenario file is untrusted structured input. It carries only deserialized
//! assertion and script parameters — no shell command, no path that gets executed, no
//! dynamically-loaded code. See [`scenario`]'s module docs for the closed-format
//! argument in full.
//!
//! ## Example
//!
//! ```rust
//! use paladin_eval::scenario::{Scenario, ScenarioTarget};
//!
//! let yaml = r#"
//! schema_version: "1"
//! target:
//!   graph_doc: "fixtures/linear.json"
//! cases:
//!   - name: happy_path
//!     assertions:
//!       - run_status: completed
//! "#;
//!
//! let scenario: Scenario = serde_yaml::from_str(yaml).unwrap();
//! assert_eq!(
//!     scenario.target,
//!     ScenarioTarget::GraphDoc {
//!         graph_doc: "fixtures/linear.json".into(),
//!     }
//! );
//! ```

#![warn(missing_docs)]

pub mod scenario;
pub mod scripted_llm;

pub use scenario::{
    Assertion, Case, EVAL_SCHEMA_VERSION, LiveOptions, LlmErrorKind, LlmScript, MatchRule,
    ParleyScript, RunStatusValue, Scenario, ScenarioError, ScenarioTarget, ScriptEntry, StoreKind,
    Times,
};
pub use scripted_llm::{CapturedRequest, ScenarioLlm, ScenarioLlmError};
