# Deferred Items — Phase 38 (design-seams-pricing-cost-producer)

Out-of-scope discoveries found while executing plans in this phase. Per the executor's
scope-boundary rule, these are logged here rather than fixed, because they were not caused
by the task that found them.

## Found during 38-04 (Task 1/2 verification)

- **Pre-existing rustdoc `broken_intra_doc_links` warnings in `cost.rs`'s module doc comment**
  (introduced by plan 38-02, not touched by 38-04). `cargo doc --workspace --no-deps` reports:
  ```
  warning: unresolved link to `Cost`
    --> crates/paladin-core/src/platform/container/cost.rs:8
  warning: unresolved link to `CurrencyCode`
    --> crates/paladin-core/src/platform/container/cost.rs:7
  warning: unresolved link to `PriceRow`
    --> crates/paladin-core/src/platform/container/cost.rs:9
  ```
  The module-level `//!` doc comment at the top of `cost.rs` links to `[`Cost`]`,
  `[`CurrencyCode`]` and `[`PriceRow`]` before rustdoc's intra-doc-link resolver treats them
  as in-scope (bare-name links from a module's own `//!` header sometimes need the fully
  qualified path or a `[Cost]: crate::...::Cost` reference link when they precede the item's
  own doc block in resolution order). This blocks `make doc-check`/`make clean-code`'s
  ADR-0033 zero-warning bar, but is unrelated to 38-04's `LlmResponse.cost` /
  `PricingLlmAdapter::generate` scope — no line in this warning's file was touched by 38-04.
  Out of scope for this plan; not fixed here. Owner: whichever plan next runs
  `make doc-check`/`clean-code` against this crate (or a dedicated hygiene pass).
