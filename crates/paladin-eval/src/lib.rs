//! # `paladin-eval` — a deterministic evaluation harness for Paladin agent graphs
//!
//! `paladin-eval` provides a serde scenario file format (`.eval.yaml`, see
//! [`scenario::Scenario`]), a scripted [`paladin_ports::output::llm_port::LlmPort`]
//! implementation ([`scripted_llm::ScenarioLlm`]) that makes a run deterministic without
//! touching a real provider, an assertion library ([`assertion`]) evaluated over the
//! captured trace record and the final `Battlefield`, and [`runner::ScenarioRunner`] --
//! the `cargo test`-integrable driver plans 28-12/28-17's `[[test]] name = "evals"
//! harness = false` target and `paladin-cli eval run` both build on.
//!
//! ## Hexagonal position (D-27)
//!
//! `paladin-eval` is a **composition-tier tool crate**, not an extracted leaf crate: it
//! depends downward on `paladin-core`, `paladin-ports`, `paladin-battalion`,
//! `paladin-llm` (`mock` feature) and `paladin-storage` (`sqlite` feature), and never on
//! the facade `paladin-ai` — the same shape `doc-examples` uses, recorded in
//! `.planning/decisions/0048-paladin-eval-composition-crate.md` so ADR-0031's
//! leaf-crate-independence invariant (scoped to the *extracted* leaf crates) is not
//! misread as violated by these edges. Nothing in the workspace depends on
//! `paladin-eval` except as a `[dev-dependencies]` entry, WITH ONE NAMED EXCEPTION
//! (D-33, plan 28-12): the facade's own OPTIONAL `cli` feature adds `paladin-eval` as
//! an optional `[dependencies]` entry too, so `paladin-cli eval run` can build a
//! `ScenarioRunner` -- the facade's DEFAULT build (no `cli` feature) still gains no
//! edge to this crate. Outside that one opt-in exception, a downstream team building
//! their own agent graph on Paladin takes this crate as a dev-dependency to write
//! deterministic evaluation scenarios for it.
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

pub mod assertion;
pub mod runner;
pub mod scenario;
pub mod scripted_llm;

pub use assertion::{
    AssertionContext, AssertionFailure, AssertionOutcome, CustomAssertion, evaluate,
};
pub use runner::{
    AssertionResult, CaseOutcome, CaseReport, GraphConstructor, LiveModeError,
    PALADIN_EVAL_LIVE_ENV, RegistriesFactory, RunOptions, RunnerError, ScenarioRunner,
    ScriptedPorts, Verdict, check_live_mode,
};
pub use scenario::{
    Assertion, Case, EVAL_SCHEMA_VERSION, LiveOptions, LlmErrorKind, LlmScript, MatchRule,
    ParleyScript, RunStatusValue, Scenario, ScenarioError, ScenarioTarget, ScriptEntry, StoreKind,
    Times,
};
pub use scripted_llm::{CapturedRequest, ScenarioLlm, ScenarioLlmError};
