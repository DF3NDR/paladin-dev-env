//! Base layer: generic infrastructure that predates the Medieval Military vocabulary.
//!
//! - `entity` holds the base entity wrapper [`Entity<T>`](crate::base::entity::Entity) and
//!   `Message`, plus `Field` and `CollectionType` / `Item`.
//! - `component` holds `Action`, `Event` and the comment-only `element` module.
//! - `service` holds `AnalysisService`, `EntityVersionService`, `MessageService`,
//!   `FieldVersionService` and `CollectionVersionService`.
//!
//! # Retained pending review
//!
//! `Field`, `CollectionType` / `Item`, the `element` module, `FieldVersionService` and
//! `CollectionVersionService` have no consumer outside `base`. They are kept because they are the
//! scaffolding of the content / general data orchestration system that was started with this
//! layer and never finished, and a consumer count must not decide their fate. The review
//! (ADR-0060 (g)) is recorded in
//! `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-BASE-LAYER-REVIEW.md`
//! (Phase 55, FIXES-02): every item is kept, and the two versioning services,
//! `FieldVersionService` and `CollectionVersionService`, are fenced where they invented
//! values. Removing or finishing any of them is a next-minor decision.

#[allow(missing_docs)]
pub mod component;
#[allow(missing_docs)]
pub mod entity;
#[allow(missing_docs)]
pub mod service;
