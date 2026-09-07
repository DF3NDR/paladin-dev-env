//! Ungated `TokenCounterPort` adapters (Doc 05 RT-FR-10, D-13).
//!
//! [`HeuristicTokenCounter`] is the phase-wide default: a synchronous,
//! infallible approximation available with no cargo feature and no external
//! tokenizer dependency, so every budget feature this phase adds
//! (`HistoryTrimmer`, `SummarizationMiddleware`) works out of the box. The
//! exact, BPE-based alternative is `garrison::TiktokenCounter`'s own `impl
//! TokenCounterPort`, gated behind the pre-existing `content-processing`
//! feature.

pub mod heuristic;
pub use heuristic::HeuristicTokenCounter;
