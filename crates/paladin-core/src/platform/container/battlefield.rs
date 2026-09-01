//! Battlefield — Typed Shared State
//!
//! This module defines [`Battlefield`], the typed shared state passed to and
//! returned (as deltas) from every node in a `WarGraph` run. Unlike the
//! bare-string data flow between Paladins today, a `Battlefield` is a
//! schema-declared map of named fields, each with a declared [`DispatchRule`]
//! (reducer) describing how concurrent writes to that field are merged.
//!
//! `paladin-core` stays dependency-pure: this module adds no new dependency,
//! it only uses `serde_json` (already a core dependency).

use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::platform::container::battlefield_error::BattlefieldError;

/// Schema version stamped on every `Battlefield`, `BattlefieldSchema` and
/// `StateDelta` value produced by this module (X-04).
pub const BATTLEFIELD_SCHEMA_VERSION: &str = "1.0.0";

/// A validated, non-empty field name within a `Battlefield` schema.
///
/// Field names are the keys nodes read and write; validating them at
/// construction (rather than accepting any `String`) keeps `UnknownField`
/// and `MissingRequiredField` errors meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FieldName(String);

/// Error returned by [`FieldName::new`] when the supplied name is invalid.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldNameError {
    /// The supplied field name was empty.
    #[error("field name must not be empty")]
    Empty,
}

impl FieldName {
    /// Construct a `FieldName`, rejecting an empty string.
    pub fn new(name: impl Into<String>) -> Result<Self, FieldNameError> {
        let name = name.into();
        if name.is_empty() {
            return Err(FieldNameError::Empty);
        }
        Ok(Self(name))
    }

    /// Borrow the field name as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Per-field merge strategy applied when a node's [`StateDelta`] is folded
/// into the shared [`Battlefield`] (PRD 3.1).
///
/// Declared `#[non_exhaustive]`: this is the full set of rules for Phase 22,
/// but later docs are known to extend the dispatch vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DispatchRule {
    /// Last write wins. Concurrent writes in the same superstep to a
    /// `LastWrite` field are a conflict, surfaced as `DispatchConflict` by
    /// the engine (multi-writer conflict detection lands in Plan 22-02).
    LastWrite,
    /// Value must be a JSON array; deltas append. Concurrent appends merge
    /// in deterministic order (ENG-FR-08, enforced by the engine).
    Append,
    /// Value must be a JSON object; deltas shallow-merge keys. Same-key
    /// concurrent writes are a conflict (detected by the engine).
    MergeObject,
    /// Numeric accumulation (`value += delta`). Value and delta must be
    /// JSON numbers.
    Sum,
    /// Named custom rule, resolved at the engine level via a
    /// [`CustomDispatchRegistry`] supplied to [`Battlefield::merge`].
    Custom(String),
}

/// Declares one field of a `Battlefield`'s schema: its name, merge strategy,
/// optional default value, and whether the engine must refuse to start a run
/// that cannot resolve a value for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// The field's name.
    pub name: FieldName,
    /// The merge strategy applied to deltas targeting this field.
    pub dispatch: DispatchRule,
    /// Default value used when the field is absent and a merge or a
    /// required-field check needs an initial value.
    pub default: Option<serde_json::Value>,
    /// When `true`, the engine errors at `start` if this field cannot be
    /// resolved from the initial delta or `default` before any node runs.
    pub required: bool,
}

impl FieldSpec {
    /// Construct a new `FieldSpec`.
    pub fn new(
        name: FieldName,
        dispatch: DispatchRule,
        default: Option<serde_json::Value>,
        required: bool,
    ) -> Self {
        Self {
            name,
            dispatch,
            default,
            required,
        }
    }
}

/// The declared shape of a `Battlefield`: its fields and the schema version
/// they were declared under (X-04). Embedded in every `Battlefield` so a
/// persisted `Waypoint` is fully self-describing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattlefieldSchema {
    /// The declared fields, in the order they were registered.
    pub fields: Vec<FieldSpec>,
    /// Schema version this declaration was authored under.
    pub schema_version: String,
}

impl BattlefieldSchema {
    /// Construct a new schema from its fields, stamping the current
    /// `BATTLEFIELD_SCHEMA_VERSION`.
    pub fn new(fields: Vec<FieldSpec>) -> Self {
        Self {
            fields,
            schema_version: BATTLEFIELD_SCHEMA_VERSION.to_string(),
        }
    }

    /// Look up a field's declaration by name.
    pub fn field_spec(&self, name: &FieldName) -> Option<&FieldSpec> {
        self.fields.iter().find(|f| &f.name == name)
    }
}

/// A node's partial update: field name -> new value / append item / merge
/// fragment, to be folded into a `Battlefield` via each field's
/// `DispatchRule`.
///
/// Carries its own `schema_version` (X-04) so a delta serialized and queued
/// (e.g. for later replay) remains self-describing independent of the
/// `Battlefield` it will eventually be merged into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StateDelta {
    /// The field values this delta contributes.
    pub values: HashMap<FieldName, serde_json::Value>,
    /// Schema version this delta was produced under.
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
}

fn default_schema_version() -> String {
    BATTLEFIELD_SCHEMA_VERSION.to_string()
}

impl StateDelta {
    /// Construct an empty delta stamped with the current schema version.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            schema_version: BATTLEFIELD_SCHEMA_VERSION.to_string(),
        }
    }

    /// Set a typed value for `field`, serializing it to JSON.
    ///
    /// A `StateDelta` carries no schema of its own, so `set` cannot reject an
    /// undeclared field here — that check happens when the delta is folded
    /// into a `Battlefield` via [`Battlefield::merge`], which returns
    /// `UnknownField` and leaves the Battlefield untouched (ENG-FR-10).
    ///
    /// Returns `BattlefieldError::TypeMismatch` if `value` cannot be
    /// serialized (practically unreachable for well-formed `Serialize`
    /// implementations); the error carries type names only, never the
    /// offending value (T-22-02).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
    ///
    /// let mut delta = StateDelta::new();
    /// delta.set(FieldName::new("count").unwrap(), 42_i64).unwrap();
    /// assert_eq!(
    ///     delta.values.get(&FieldName::new("count").unwrap()),
    ///     Some(&serde_json::json!(42))
    /// );
    /// ```
    pub fn set<T: Serialize>(
        &mut self,
        field: FieldName,
        value: T,
    ) -> Result<(), BattlefieldError> {
        let json = serde_json::to_value(value).map_err(|_| BattlefieldError::TypeMismatch {
            field: field.clone(),
            expected: std::any::type_name::<T>().to_string(),
            got: "unserializable value".to_string(),
        })?;
        self.values.insert(field, json);
        Ok(())
    }

    /// Set a raw JSON value for `field` directly.
    pub fn set_raw(&mut self, field: FieldName, value: serde_json::Value) {
        self.values.insert(field, value);
    }
}

/// A registered custom dispatch resolver: `(current, delta) -> merged`.
pub type CustomDispatchFn = dyn Fn(&serde_json::Value, &serde_json::Value) -> Result<serde_json::Value, BattlefieldError>
    + Send
    + Sync;

/// Registry of named custom dispatch resolvers, keyed by the name used in
/// `DispatchRule::Custom(name)`.
///
/// Registration lives in the engine (application layer), never in
/// `paladin-core` (ENG-FR-09) — this module only defines the shape of the
/// registry [`Battlefield::merge`] consumes.
pub type CustomDispatchRegistry = HashMap<String, Arc<CustomDispatchFn>>;

/// Typed shared state for one workflow run.
///
/// Internally a map of named fields, each governed by a declared
/// [`DispatchRule`] in `schema`. Serializes with its schema embedded so a
/// `Waypoint` snapshot is self-describing (X-04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battlefield {
    schema: BattlefieldSchema,
    values: HashMap<FieldName, serde_json::Value>,
}

impl Battlefield {
    /// Construct a fresh `Battlefield` from `schema`, populating any
    /// declared `default` values immediately.
    pub fn new(schema: BattlefieldSchema) -> Self {
        let mut values = HashMap::new();
        for field in &schema.fields {
            if let Some(default) = &field.default {
                values.insert(field.name.clone(), default.clone());
            }
        }
        Self { schema, values }
    }

    /// The schema this Battlefield was constructed from.
    pub fn schema(&self) -> &BattlefieldSchema {
        &self.schema
    }

    /// Read a field's raw JSON value, if present.
    pub fn get_raw(&self, field: &FieldName) -> Option<&serde_json::Value> {
        self.values.get(field)
    }

    /// Read and deserialize a field's value into `T`.
    ///
    /// Returns `Ok(None)` if the field is declared but has no value (not an
    /// error). Returns `BattlefieldError::UnknownField` if `field` is not
    /// declared in this Battlefield's schema at all (ENG-FR-10) — schema
    /// enforcement is a hard error on every accessor, not just `merge`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::battlefield::{
    ///     Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
    ///     CustomDispatchRegistry,
    /// };
    ///
    /// let field = FieldName::new("greeting").unwrap();
    /// let schema = BattlefieldSchema::new(vec![FieldSpec::new(
    ///     field.clone(),
    ///     DispatchRule::LastWrite,
    ///     None,
    ///     false,
    /// )]);
    /// let mut battlefield = Battlefield::new(schema);
    /// let mut delta = StateDelta::new();
    /// delta.set(field.clone(), "hello").unwrap();
    /// battlefield.merge(&delta, &CustomDispatchRegistry::new()).unwrap();
    ///
    /// assert_eq!(battlefield.get::<String>(&field).unwrap(), Some("hello".to_string()));
    /// ```
    pub fn get<T: DeserializeOwned>(
        &self,
        field: &FieldName,
    ) -> Result<Option<T>, BattlefieldError> {
        self.schema
            .field_spec(field)
            .ok_or_else(|| BattlefieldError::UnknownField {
                field: field.clone(),
            })?;
        match self.values.get(field) {
            None => Ok(None),
            Some(value) => serde_json::from_value(value.clone())
                .map(Some)
                .map_err(|_| BattlefieldError::TypeMismatch {
                    field: field.clone(),
                    expected: std::any::type_name::<T>().to_string(),
                    got: json_type_name(value).to_string(),
                }),
        }
    }

    /// Verify every `required` field in the schema resolves to a value
    /// (already present, or has a `default`), per ENG-FR-10.
    pub fn validate_required(&self) -> Result<(), BattlefieldError> {
        for field in &self.schema.fields {
            if field.required && !self.values.contains_key(&field.name) {
                return Err(BattlefieldError::MissingRequiredField {
                    field: field.name.clone(),
                });
            }
        }
        Ok(())
    }

    /// Resolve a run's initial state from `schema` and `initial_delta`
    /// (ENG-FR-10): for each declared field, in schema declaration order,
    /// `initial_delta`'s value is used if present, else the field's
    /// `default`. This runs before any node executes.
    ///
    /// Returns `UnknownField` if `initial_delta` touches a field the schema
    /// does not declare, `SchemaVersionUnsupported` if `schema` was not
    /// authored under [`BATTLEFIELD_SCHEMA_VERSION`], and
    /// `MissingRequiredField` for the first `required` field (in schema
    /// declaration order) that neither `initial_delta` nor a `default` can
    /// resolve.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::battlefield::{
    ///     Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
    /// };
    ///
    /// let field = FieldName::new("topic").unwrap();
    /// let schema = BattlefieldSchema::new(vec![FieldSpec::new(
    ///     field.clone(),
    ///     DispatchRule::LastWrite,
    ///     None,
    ///     true,
    /// )]);
    /// let mut initial = StateDelta::new();
    /// initial.set(field.clone(), "rust").unwrap();
    ///
    /// let battlefield = Battlefield::initialize(schema, &initial).unwrap();
    /// assert_eq!(battlefield.get::<String>(&field).unwrap(), Some("rust".to_string()));
    /// ```
    pub fn initialize(
        schema: BattlefieldSchema,
        initial_delta: &StateDelta,
    ) -> Result<Self, BattlefieldError> {
        if schema.schema_version != BATTLEFIELD_SCHEMA_VERSION {
            return Err(BattlefieldError::SchemaVersionUnsupported {
                found: schema.schema_version.clone(),
                supported: BATTLEFIELD_SCHEMA_VERSION.to_string(),
            });
        }

        for field in initial_delta.values.keys() {
            if schema.field_spec(field).is_none() {
                return Err(BattlefieldError::UnknownField {
                    field: field.clone(),
                });
            }
        }

        let mut values = HashMap::new();
        for field_spec in &schema.fields {
            if let Some(value) = initial_delta.values.get(&field_spec.name) {
                values.insert(field_spec.name.clone(), value.clone());
            } else if let Some(default) = &field_spec.default {
                values.insert(field_spec.name.clone(), default.clone());
            } else if field_spec.required {
                return Err(BattlefieldError::MissingRequiredField {
                    field: field_spec.name.clone(),
                });
            }
        }

        Ok(Self { schema, values })
    }

    /// Deserialize a `Battlefield` from its JSON representation, enforcing
    /// (X-04) that its embedded `schema.schema_version` is the one this
    /// build supports before accepting the rest of the payload.
    ///
    /// A payload whose `schema_version` cannot even be read (malformed JSON,
    /// or a structurally different shape entirely) is reported the same way,
    /// with `found` set to `"<unparseable>"` — this method's contract is
    /// version enforcement, not general JSON-diagnostic reporting, and
    /// `BattlefieldError` deliberately has no free-form parse-error variant
    /// that could carry arbitrary (possibly sensitive) payload content
    /// (T-22-04).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::battlefield::Battlefield;
    /// use paladin_core::platform::container::battlefield_error::BattlefieldError;
    ///
    /// let stale = r#"{"schema":{"fields":[],"schema_version":"0.0.1"},"values":{}}"#;
    /// let err = Battlefield::from_json(stale).unwrap_err();
    /// assert!(matches!(err, BattlefieldError::SchemaVersionUnsupported { .. }));
    /// ```
    pub fn from_json(json: &str) -> Result<Self, BattlefieldError> {
        let value: serde_json::Value =
            serde_json::from_str(json).unwrap_or(serde_json::Value::Null);
        let found = value
            .get("schema")
            .and_then(|s| s.get("schema_version"))
            .and_then(|v| v.as_str())
            .unwrap_or("<unparseable>")
            .to_string();
        if found != BATTLEFIELD_SCHEMA_VERSION {
            return Err(BattlefieldError::SchemaVersionUnsupported {
                found,
                supported: BATTLEFIELD_SCHEMA_VERSION.to_string(),
            });
        }
        serde_json::from_value(value).map_err(|_| BattlefieldError::SchemaVersionUnsupported {
            found,
            supported: BATTLEFIELD_SCHEMA_VERSION.to_string(),
        })
    }

    /// Apply a single node's [`StateDelta`], merging each touched field via
    /// its declared [`DispatchRule`].
    ///
    /// Schema membership is validated for every touched field before any
    /// mutation is applied: if any field is undeclared, this returns
    /// `UnknownField` and the Battlefield is left byte-identical to its
    /// pre-merge form (ENG-FR-10).
    ///
    /// This is a **single-writer** merge: it does not detect conflicts
    /// between multiple concurrent deltas targeting the same superstep.
    /// Multi-writer `DispatchConflict` detection across a superstep's
    /// deltas is this method's next expansion within this same plan.
    pub fn merge(
        &mut self,
        delta: &StateDelta,
        custom_dispatch: &CustomDispatchRegistry,
    ) -> Result<(), BattlefieldError> {
        let mut specs = Vec::with_capacity(delta.values.len());
        for field in delta.values.keys() {
            let spec = self
                .schema
                .field_spec(field)
                .ok_or_else(|| BattlefieldError::UnknownField {
                    field: field.clone(),
                })?
                .clone();
            specs.push((field.clone(), spec));
        }

        for (field, spec) in specs {
            let field = &field;
            let delta_value = &delta.values[field];

            match &spec.dispatch {
                DispatchRule::LastWrite => {
                    self.values.insert(field.clone(), delta_value.clone());
                }
                DispatchRule::Append => {
                    let mut current = match self.values.remove(field) {
                        Some(serde_json::Value::Array(arr)) => arr,
                        Some(other) => {
                            return Err(BattlefieldError::TypeMismatch {
                                field: field.clone(),
                                expected: "array".to_string(),
                                got: json_type_name(&other).to_string(),
                            });
                        }
                        None => Vec::new(),
                    };
                    current.push(delta_value.clone());
                    self.values
                        .insert(field.clone(), serde_json::Value::Array(current));
                }
                DispatchRule::MergeObject => {
                    let mut current = match self.values.remove(field) {
                        Some(serde_json::Value::Object(obj)) => obj,
                        Some(other) => {
                            return Err(BattlefieldError::TypeMismatch {
                                field: field.clone(),
                                expected: "object".to_string(),
                                got: json_type_name(&other).to_string(),
                            });
                        }
                        None => serde_json::Map::new(),
                    };
                    let incoming =
                        delta_value
                            .as_object()
                            .ok_or_else(|| BattlefieldError::TypeMismatch {
                                field: field.clone(),
                                expected: "object".to_string(),
                                got: json_type_name(delta_value).to_string(),
                            })?;
                    for (key, value) in incoming {
                        current.insert(key.clone(), value.clone());
                    }
                    self.values
                        .insert(field.clone(), serde_json::Value::Object(current));
                }
                DispatchRule::Sum => {
                    let current = self
                        .values
                        .remove(field)
                        .unwrap_or(serde_json::Value::from(0_i64));
                    let merged = sum_json_numbers(field, &current, delta_value)?;
                    self.values.insert(field.clone(), merged);
                }
                DispatchRule::Custom(name) => {
                    let resolver = custom_dispatch.get(name).ok_or_else(|| {
                        BattlefieldError::CustomDispatchNotRegistered { name: name.clone() }
                    })?;
                    let current = self
                        .values
                        .get(field)
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let merged = resolver(&current, delta_value)?;
                    self.values.insert(field.clone(), merged);
                }
            }
        }
        Ok(())
    }
}

fn sum_json_numbers(
    field: &FieldName,
    current: &serde_json::Value,
    delta: &serde_json::Value,
) -> Result<serde_json::Value, BattlefieldError> {
    let (Some(cur), Some(delt)) = (current.as_i64(), delta.as_i64()) else {
        let cur_f = current
            .as_f64()
            .ok_or_else(|| BattlefieldError::TypeMismatch {
                field: field.clone(),
                expected: "number".to_string(),
                got: json_type_name(current).to_string(),
            })?;
        let delt_f = delta
            .as_f64()
            .ok_or_else(|| BattlefieldError::TypeMismatch {
                field: field.clone(),
                expected: "number".to_string(),
                got: json_type_name(delta).to_string(),
            })?;
        return Ok(serde_json::Value::from(cur_f + delt_f));
    };
    Ok(serde_json::Value::from(cur + delt))
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
    }

    #[test]
    fn field_name_rejects_empty() {
        assert_eq!(FieldName::new(""), Err(FieldNameError::Empty));
    }

    #[test]
    fn battlefield_round_trips_through_serde_json() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("result"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(field("result"), "hello").unwrap();
        bf.merge(&delta, &CustomDispatchRegistry::new()).unwrap();

        let json = serde_json::to_string(&bf).unwrap();
        let restored: Battlefield = serde_json::from_str(&json).unwrap();
        assert_eq!(bf, restored);
        assert_eq!(
            restored.get::<String>(&field("result")).unwrap(),
            Some("hello".to_string())
        );
    }

    #[test]
    fn state_delta_round_trips_and_carries_schema_version() {
        let mut delta = StateDelta::new();
        delta.set(field("result"), 42_u64).unwrap();
        assert_eq!(delta.schema_version, BATTLEFIELD_SCHEMA_VERSION);

        let json = serde_json::to_string(&delta).unwrap();
        let restored: StateDelta = serde_json::from_str(&json).unwrap();
        assert_eq!(delta, restored);
    }

    #[test]
    fn battlefield_schema_round_trips_and_carries_schema_version() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("result"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        assert_eq!(schema.schema_version, BATTLEFIELD_SCHEMA_VERSION);
        let json = serde_json::to_string(&schema).unwrap();
        let restored: BattlefieldSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema, restored);
    }

    #[test]
    fn merge_last_write_replaces() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("x"), 1_u64).unwrap();
        bf.merge(&d1, &CustomDispatchRegistry::new()).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("x"), 2_u64).unwrap();
        bf.merge(&d2, &CustomDispatchRegistry::new()).unwrap();
        assert_eq!(bf.get::<u64>(&field("x")).unwrap(), Some(2));
    }

    #[test]
    fn merge_append_pushes_items() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("log"), "a").unwrap();
        bf.merge(&d1, &CustomDispatchRegistry::new()).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("log"), "b").unwrap();
        bf.merge(&d2, &CustomDispatchRegistry::new()).unwrap();
        assert_eq!(
            bf.get::<Vec<String>>(&field("log")).unwrap(),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn merge_merge_object_shallow_merges_keys() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("obj"),
            DispatchRule::MergeObject,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set_raw(field("obj"), serde_json::json!({"a": 1}));
        bf.merge(&d1, &CustomDispatchRegistry::new()).unwrap();
        let mut d2 = StateDelta::new();
        d2.set_raw(field("obj"), serde_json::json!({"b": 2}));
        bf.merge(&d2, &CustomDispatchRegistry::new()).unwrap();
        assert_eq!(
            bf.get_raw(&field("obj")).unwrap(),
            &serde_json::json!({"a": 1, "b": 2})
        );
    }

    #[test]
    fn merge_sum_accumulates() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("count"),
            DispatchRule::Sum,
            Some(serde_json::json!(0)),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("count"), 5_i64).unwrap();
        bf.merge(&d1, &CustomDispatchRegistry::new()).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("count"), 3_i64).unwrap();
        bf.merge(&d2, &CustomDispatchRegistry::new()).unwrap();
        assert_eq!(bf.get::<i64>(&field("count")).unwrap(), Some(8));
    }

    #[test]
    fn merge_custom_dispatch_not_registered_errors() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("special"),
            DispatchRule::Custom("merge_scores".to_string()),
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(field("special"), 1_u64).unwrap();
        let err = bf
            .merge(&delta, &CustomDispatchRegistry::new())
            .unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::CustomDispatchNotRegistered {
                name: "merge_scores".to_string()
            }
        );
    }

    #[test]
    fn merge_custom_dispatch_resolves_when_registered() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("special"),
            DispatchRule::Custom("max".to_string()),
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut registry: CustomDispatchRegistry = CustomDispatchRegistry::new();
        registry.insert(
            "max".to_string(),
            Arc::new(|current: &serde_json::Value, delta: &serde_json::Value| {
                let c = current.as_i64().unwrap_or(i64::MIN);
                let d = delta.as_i64().unwrap_or(i64::MIN);
                Ok(serde_json::Value::from(c.max(d)))
            }),
        );
        let mut d1 = StateDelta::new();
        d1.set(field("special"), 3_i64).unwrap();
        bf.merge(&d1, &registry).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("special"), 7_i64).unwrap();
        bf.merge(&d2, &registry).unwrap();
        assert_eq!(bf.get::<i64>(&field("special")).unwrap(), Some(7));
    }

    #[test]
    fn merge_unknown_field_errors() {
        let schema = BattlefieldSchema::new(vec![]);
        let mut bf = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(field("ghost"), 1_u64).unwrap();
        let err = bf
            .merge(&delta, &CustomDispatchRegistry::new())
            .unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::UnknownField {
                field: field("ghost")
            }
        );
    }

    #[test]
    fn validate_required_errors_when_missing() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("must_have"),
            DispatchRule::LastWrite,
            None,
            true,
        )]);
        let bf = Battlefield::new(schema);
        let err = bf.validate_required().unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::MissingRequiredField {
                field: field("must_have")
            }
        );
    }

    #[test]
    fn validate_required_satisfied_by_default() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("must_have"),
            DispatchRule::LastWrite,
            Some(serde_json::json!("default")),
            true,
        )]);
        let bf = Battlefield::new(schema);
        assert!(bf.validate_required().is_ok());
    }

    // --- Task 1: typed accessors and hard schema enforcement ---

    #[test]
    fn get_on_declared_present_well_typed_field_returns_value() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("x"), 7_u64).unwrap();
        bf.merge(&d, &CustomDispatchRegistry::new()).unwrap();
        assert_eq!(bf.get::<u64>(&field("x")).unwrap(), Some(7));
    }

    #[test]
    fn get_on_declared_absent_field_with_no_default_returns_none() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let bf = Battlefield::new(schema);
        assert_eq!(bf.get::<u64>(&field("x")).unwrap(), None);
    }

    #[test]
    fn get_on_undeserializable_value_returns_type_mismatch_with_type_names() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("x"), "not a number").unwrap();
        bf.merge(&d, &CustomDispatchRegistry::new()).unwrap();
        let err = bf.get::<u64>(&field("x")).unwrap_err();
        match &err {
            BattlefieldError::TypeMismatch {
                field: f,
                expected,
                got,
            } => {
                assert_eq!(f, &field("x"));
                assert!(expected.contains("u64"));
                assert_eq!(got, "string");
            }
            other => panic!("expected TypeMismatch, got {other:?}"),
        }
    }

    #[test]
    fn get_on_undeclared_field_returns_unknown_field() {
        let bf = Battlefield::new(BattlefieldSchema::new(vec![]));
        let err = bf.get::<u64>(&field("ghost")).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::UnknownField {
                field: field("ghost")
            }
        );
    }

    #[test]
    fn set_on_undeclared_field_rejected_at_merge_time_and_battlefield_unchanged() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("known"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut known_delta = StateDelta::new();
        known_delta.set(field("known"), "pre-existing").unwrap();
        bf.merge(&known_delta, &CustomDispatchRegistry::new())
            .unwrap();
        let before = serde_json::to_string(&bf).unwrap();

        let mut bad_delta = StateDelta::new();
        bad_delta.set(field("ghost"), 1_u64).unwrap();
        let err = bf
            .merge(&bad_delta, &CustomDispatchRegistry::new())
            .unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::UnknownField {
                field: field("ghost")
            }
        );

        let after = serde_json::to_string(&bf).unwrap();
        assert_eq!(
            before, after,
            "Battlefield must be byte-identical after a rejected merge"
        );
    }

    #[test]
    fn initialize_resolves_required_fields_from_initial_delta_and_defaults() {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(field("topic"), DispatchRule::LastWrite, None, true),
            FieldSpec::new(
                field("count"),
                DispatchRule::Sum,
                Some(serde_json::json!(0)),
                true,
            ),
        ]);
        let mut initial = StateDelta::new();
        initial.set(field("topic"), "rust").unwrap();

        let bf = Battlefield::initialize(schema, &initial).unwrap();
        assert_eq!(
            bf.get::<String>(&field("topic")).unwrap(),
            Some("rust".to_string())
        );
        assert_eq!(bf.get::<i64>(&field("count")).unwrap(), Some(0));
    }

    #[test]
    fn initialize_missing_required_field_with_no_default_errors() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("topic"),
            DispatchRule::LastWrite,
            None,
            true,
        )]);
        let initial = StateDelta::new();

        let err = Battlefield::initialize(schema, &initial).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::MissingRequiredField {
                field: field("topic")
            }
        );
    }

    #[test]
    fn initialize_rejects_unknown_field_in_initial_delta() {
        let schema = BattlefieldSchema::new(vec![]);
        let mut initial = StateDelta::new();
        initial.set(field("ghost"), 1_u64).unwrap();

        let err = Battlefield::initialize(schema, &initial).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::UnknownField {
                field: field("ghost")
            }
        );
    }

    #[test]
    fn from_json_rejects_unsupported_schema_version() {
        let stale = r#"{"schema":{"fields":[],"schema_version":"0.0.1"},"values":{}}"#;
        let err = Battlefield::from_json(stale).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::SchemaVersionUnsupported {
                found: "0.0.1".to_string(),
                supported: BATTLEFIELD_SCHEMA_VERSION.to_string(),
            }
        );
    }

    #[test]
    fn from_json_accepts_supported_schema_version() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let bf = Battlefield::new(schema);
        let json = serde_json::to_string(&bf).unwrap();
        let restored = Battlefield::from_json(&json).unwrap();
        assert_eq!(bf, restored);
    }

    #[test]
    fn no_error_display_contains_a_value_placed_in_state() {
        let secret = "sk-super-secret-credential-value";
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("token"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("token"), secret).unwrap();
        bf.merge(&d, &CustomDispatchRegistry::new()).unwrap();

        // Trigger every error variant reachable from this state and assert
        // none of their Display output contains the value stored above.
        let errors: Vec<BattlefieldError> = vec![
            bf.get::<u64>(&field("token")).unwrap_err(), // TypeMismatch
            bf.get::<u64>(&field("nonexistent")).unwrap_err(), // UnknownField
            BattlefieldError::MissingRequiredField {
                field: field("token"),
            },
            BattlefieldError::SchemaVersionUnsupported {
                found: "0.0.1".to_string(),
                supported: BATTLEFIELD_SCHEMA_VERSION.to_string(),
            },
            BattlefieldError::CustomDispatchNotRegistered {
                name: "missing_rule".to_string(),
            },
            BattlefieldError::DispatchConflict {
                field: field("token"),
                superstep: 0,
                writers: vec![],
            },
        ];
        for err in errors {
            assert!(
                !err.to_string().contains(secret),
                "error Display leaked a state value: {err}"
            );
        }
    }
}
