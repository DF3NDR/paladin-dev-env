# paladin-core

Pure domain types for the Paladin framework with zero infrastructure dependencies.

## Purpose

`paladin-core` defines foundational entities and value objects on the `Entity<T>` wrapper.

## Key Modules

- `base`: Core primitives: the `Entity<T>` wrapper, messages, actions, events, the entity version service, and the collection and field types retained for the content system (ADR-0060).
- `platform`: Domain entities for Paladins, Battalions, Garrison, Arsenal, Citadel, Herald, and Sanctum.

## Usage

```rust
use paladin_core::base::entity::Entity;
use paladin_core::platform::container::paladin::PaladinStatus;

// Import domain primitives and aggregate types from the core layer.
let entity = Entity::new("paladin".to_string(), None);
let _status = PaladinStatus::Idle;
assert_eq!(entity.data, "paladin");
```

## Feature Flags

This crate has no feature flags.
