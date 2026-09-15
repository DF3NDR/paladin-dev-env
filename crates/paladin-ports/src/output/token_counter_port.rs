//! # Token Counter Port — a synchronous, infallible counting seam (Doc 05 RT-FR-10, D-13)
//!
//! [`TokenCounterPort`] is the ONE counting path every budget feature added by the
//! v0.10.0 agent-runtime-enhancements phase consumes -- `HistoryTrimmer`
//! (`src/application/services/paladin/middleware/history.rs`),
//! `SummarizationMiddleware` and any future budget-shaped middleware. No budget
//! feature this phase adds computes a token count inline; every count goes
//! through an `Arc<dyn TokenCounterPort>`.
//!
//! ## Synchronous and infallible, deliberately
//!
//! Counting a string's length is a pure, local, CPU-bound computation -- it never
//! needs `.await` and it never needs a `Result`. An adapter that does not
//! recognise `model` falls back to its own approximation **inside** the
//! adapter, so a caller never has to handle "I don't know this model" as an
//! error case. This mirrors [`crate::output::garrison_port::GarrisonPort`]'s
//! small, purpose-built trait shape rather than the general-purpose
//! `async_trait` ports elsewhere in this crate.
//!
//! ## Adapters
//!
//! - `paladin_memory::token_counter::HeuristicTokenCounter` -- the ungated
//!   default, `text.chars().count() / 4` rounded up, approximate (±30%);
//!   inherits [`TokenCounterPort::is_exact`]'s `false` default.
//! - `paladin_memory::garrison::TiktokenCounter` (under the `content-processing`
//!   feature) -- exact BPE tokenisation for OpenAI-family models, falling back
//!   to its already-loaded encoding for any `model` string it is not asked to
//!   re-resolve; overrides [`TokenCounterPort::is_exact`] to `true`.

/// A synchronous, infallible token-counting adapter.
///
/// # Contract
///
/// - `count` never returns an error and never panics for any `text`/`model`
///   pair, including an empty string or a model name the adapter does not
///   recognise -- an unrecognised model falls back to the adapter's own
///   approximation internally rather than surfacing as a caller-visible error.
/// - `count` is a pure function of its inputs and the adapter's own fixed
///   configuration: the same `(text, model)` pair returns the same count on
///   every call, across any number of repetitions and across a fresh adapter
///   instance constructed the same way.
/// - `name` identifies the adapter (e.g. `"heuristic"`, `"tiktoken"`) for use
///   in logs -- callers that need to explain "why did my history get trimmed
///   this much" log the resolved count's source via this name.
/// - `is_exact` is a property of the adapter **instance as constructed**, not
///   of an individual `(text, model)` call: `true` means `count` returns the
///   model's own tokenizer tally; `false` means an approximation. It defaults
///   to `false`, so an adapter that overrides neither `count` nor `is_exact`
///   is treated as approximate rather than trusted.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::token_counter_port::TokenCounterPort;
///
/// struct AlwaysOne;
///
/// impl TokenCounterPort for AlwaysOne {
///     fn count(&self, _text: &str, _model: &str) -> u32 {
///         1
///     }
///
///     fn name(&self) -> &str {
///         "always-one"
///     }
///
///     // No `is_exact` override is written here -- the trait default
///     // applies, and the assertion below proves it.
/// }
///
/// let counter = AlwaysOne;
/// assert_eq!(counter.count("hello, world", "any-model"), 1);
/// assert_eq!(counter.count("", "an-unrecognised-model"), 1);
/// assert_eq!(counter.name(), "always-one");
/// assert!(!counter.is_exact(), "the trait default is false");
/// ```
pub trait TokenCounterPort: Send + Sync {
    /// Counts the (approximate or exact) number of tokens `text` would
    /// occupy for `model`. Never fails: an adapter that does not recognise
    /// `model` falls back to its own approximation internally.
    fn count(&self, text: &str, model: &str) -> u32;

    /// A stable, adapter-identifying name (e.g. `"heuristic"`, `"tiktoken"`),
    /// usable in a debug log naming which counter produced a given count.
    fn name(&self) -> &str;

    /// Whether [`TokenCounterPort::count`] returns an exact tally for the
    /// adapter instance as constructed, rather than an approximation.
    ///
    /// This is a property of the instance, not of any particular
    /// `(text, model)` call: it does not vary by argument, and it does not
    /// change across repeated calls on the same instance. `true` means
    /// `count` returns the model's own tokenizer tally; `false` means an
    /// approximation. Defaults to `false` -- an adapter that says nothing is
    /// treated as approximate.
    fn is_exact(&self) -> bool {
        false
    }
}
