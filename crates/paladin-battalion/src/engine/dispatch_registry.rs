//! Engine-level custom dispatch rule registry (ENG-FR-09).
//!
//! `paladin-core` only defines the shape [`CustomDispatchResolver`]
//! `Battlefield::merge` consumes as a read-only lookup; constructing and
//! populating it is application-layer responsibility, owned here in
//! `paladin-battalion`. `paladin-core` names neither this module nor
//! [`DispatchRegistry`] anywhere (X-01) -- a schema declares
//! `DispatchRule::Custom(name)` and the engine is the only thing that ever
//! resolves what `name` means.

use std::sync::Arc;

use paladin_core::platform::container::battlefield::{CustomDispatchFn, CustomDispatchResolver};

use crate::engine::EngineError;

/// Names reserved by the built-in [`paladin_core::platform::container::battlefield::DispatchRule`]
/// variants. Registering a custom rule under one of these is rejected
/// (ENG-FR-09) so a schema author cannot believe they have overridden e.g.
/// `LastWrite` when they have not -- an unregistered `Custom(name)` always
/// fails validation rather than silently falling back to a built-in.
const RESERVED_NAMES: &[&str] = &["LastWrite", "Append", "MergeObject", "Sum"];

/// Engine-owned registry of named custom dispatch resolvers
/// (`DispatchRule::Custom(name)`, ENG-FR-09).
///
/// Registration lives in the engine (application layer), never in
/// `paladin-core` (X-01): `paladin-core` only ever receives the resulting
/// [`CustomDispatchResolver`] as a read-only lookup, via
/// [`DispatchRegistry::resolver`].
#[derive(Default)]
pub struct DispatchRegistry {
    inner: CustomDispatchResolver,
}

impl DispatchRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `name` under a `(current, delta) -> merged` closure
    /// (ENG-FR-09's exact signature: `Fn(&serde_json::Value,
    /// &serde_json::Value) -> Result<serde_json::Value, BattlefieldError>`).
    ///
    /// Rejects a `name` that collides with a built-in `DispatchRule`
    /// variant name with `EngineError::ReservedDispatchName` -- registration
    /// is where this collision is caught, not silently ignored at
    /// validation or merge time.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        rule: Arc<CustomDispatchFn>,
    ) -> Result<(), EngineError> {
        let name = name.into();
        if RESERVED_NAMES.contains(&name.as_str()) {
            return Err(EngineError::ReservedDispatchName { name });
        }
        self.inner.insert(name, rule);
        Ok(())
    }

    /// The [`CustomDispatchResolver`] this registry has built, as
    /// `Battlefield::merge` and `WarGraph::validate` consume it.
    pub fn resolver(&self) -> &CustomDispatchResolver {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_resolve_a_custom_rule() {
        let mut registry = DispatchRegistry::new();
        registry
            .register(
                "max",
                Arc::new(|current: &serde_json::Value, delta: &serde_json::Value| {
                    let c = current.as_i64().unwrap_or(i64::MIN);
                    let d = delta.as_i64().unwrap_or(i64::MIN);
                    Ok(serde_json::json!(c.max(d)))
                }),
            )
            .unwrap();
        assert!(registry.resolver().contains_key("max"));
    }

    #[test]
    fn register_rejects_names_colliding_with_built_in_rules() {
        let mut registry = DispatchRegistry::new();
        for reserved in RESERVED_NAMES {
            let err = registry
                .register(
                    *reserved,
                    Arc::new(|_c: &serde_json::Value, d: &serde_json::Value| Ok(d.clone())),
                )
                .unwrap_err();
            assert!(matches!(
                err,
                EngineError::ReservedDispatchName { name } if name == *reserved
            ));
        }
        assert!(registry.resolver().is_empty());
    }
}
