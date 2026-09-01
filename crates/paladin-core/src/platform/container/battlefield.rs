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

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::platform::container::battlefield_error::BattlefieldError;
use crate::platform::container::waypoint::NodeId;

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

/// Per-field merge strategy applied when one or more nodes' [`StateDelta`]s
/// are folded into the shared [`Battlefield`] in one superstep (PRD 3.1).
///
/// Declared `#[non_exhaustive]`: this is the full set of rules for Phase 22,
/// but later docs are known to extend the dispatch vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DispatchRule {
    /// Last write wins. Two or more distinct writer `NodeId`s touching a
    /// `LastWrite` field within the same superstep is a hard
    /// `DispatchConflict` — last-writer-wins never happens silently. A
    /// single writer contributing more than one delta in the same call
    /// resolves by emission order (no conflict).
    LastWrite,
    /// Value must be a JSON array; deltas append. Concurrent appends from
    /// distinct writers merge deterministically, ordered by
    /// `(writer NodeId, emission index)` (ENG-FR-08) — never a conflict.
    Append,
    /// Value must be a JSON object; deltas shallow-merge keys. Two distinct
    /// writers touching the SAME key within one field in the same superstep
    /// is a hard `DispatchConflict`; disjoint keys merge cleanly.
    MergeObject,
    /// Numeric accumulation (`value += delta`). See [`Battlefield::merge`]'s
    /// rustdoc for the exact integer-overflow / mixed-representation
    /// contract.
    Sum,
    /// Named custom rule, resolved via a [`CustomDispatchResolver`] supplied
    /// to [`Battlefield::merge`]. The resolver registry itself is owned by
    /// the engine (application layer), never by `paladin-core` (ENG-FR-09).
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

/// The core-side custom dispatch lookup consumed by [`Battlefield::merge`].
///
/// Named distinctly from [`CustomDispatchRegistry`] per this plan's artifact
/// list, even though the two are the same underlying type today: the engine
/// (application layer) owns constructing and populating the registry
/// (ENG-FR-09, Plan 22-07) — `paladin-core` only ever receives it as a
/// read-only lookup, and this alias is that read-only-consumer-facing name.
pub type CustomDispatchResolver = CustomDispatchRegistry;

/// Record of which fields changed as a result of one [`Battlefield::merge`]
/// call, in `BattlefieldSchema` declaration order. The engine consumes this
/// to emit the `DeltaMerged` trace event (Plan 22-09).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeReport {
    /// Fields whose value changed (or was newly set) by this merge, in
    /// schema declaration order.
    pub changed_fields: Vec<FieldName>,
}

/// Typed shared state for one workflow run.
///
/// Internally a map of named fields, each governed by a declared
/// [`DispatchRule`] in `schema`. Serializes with its schema embedded so a
/// `Waypoint` snapshot is self-describing (X-04).
///
/// Field values are stored in a `BTreeMap` (sorted by [`FieldName`]), never a
/// `HashMap`: `std::collections::HashMap`'s iteration order is randomized per
/// instance, so a `HashMap`-backed map would serialize two logically
/// identical Battlefields (built via different field insertion orders) to
/// different byte sequences — silently breaking the ENG-FR-08 byte-identical
/// determinism guarantee `Battlefield::merge` exists to provide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battlefield {
    schema: BattlefieldSchema,
    values: BTreeMap<FieldName, serde_json::Value>,
}

impl Battlefield {
    /// Construct a fresh `Battlefield` from `schema`, populating any
    /// declared `default` values immediately.
    pub fn new(schema: BattlefieldSchema) -> Self {
        let mut values = BTreeMap::new();
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
    ///     Battlefield, BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldName,
    ///     FieldSpec, StateDelta,
    /// };
    /// use paladin_core::platform::container::waypoint::NodeId;
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
    /// battlefield
    ///     .merge(vec![(NodeId::new("writer"), delta)], 0, &CustomDispatchResolver::new())
    ///     .unwrap();
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

        let mut values = BTreeMap::new();
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

    /// Merge one superstep's deltas into this Battlefield, applying each
    /// touched field's declared [`DispatchRule`] against every writer that
    /// contributed to it in this call (ENG-FR-07/08/09).
    ///
    /// # Algorithm
    ///
    /// 1. **Validate, then mutate.** Every field touched by any delta is
    ///    checked against the schema before any value is written; an
    ///    `UnknownField` anywhere leaves the Battlefield byte-identical to
    ///    its pre-merge form. If a dispatch rule itself later fails (a
    ///    `TypeMismatch`, `DispatchConflict`, or unregistered `Custom` rule),
    ///    the Battlefield is rolled back to its pre-merge snapshot before the
    ///    error is returned — merge is all-or-nothing.
    /// 2. **Deterministic grouping.** Touched fields are processed in
    ///    `BattlefieldSchema.fields` declaration order (never `HashMap`
    ///    order). For each field, every writer's contribution is collected
    ///    and sorted by `(writer NodeId, emission index)` — `emission index`
    ///    is this call's position of that `(NodeId, StateDelta)` pair within
    ///    the input `deltas` vector, a tiebreak that only matters when the
    ///    same `NodeId` contributes more than one delta in a single call
    ///    (`NodeId`s are unique within a graph, so this is a within-node
    ///    tiebreak, never a cross-node one).
    ///
    /// # Conflict semantics
    ///
    /// - `LastWrite`: more than one *distinct* writer `NodeId` touching the
    ///   field is `DispatchConflict { field, superstep, writers }` (writers
    ///   sorted, deduplicated). A single writer contributing more than one
    ///   delta resolves via emission order — no conflict.
    /// - `MergeObject`: two distinct writers touching the same object key is
    ///   `DispatchConflict`; disjoint keys shallow-merge cleanly.
    /// - `Append`: never conflicts; concurrent writers merge in
    ///   `(NodeId, emission index)` order.
    /// - `Sum`: never conflicts by writer identity; see the numeric contract
    ///   below.
    /// - `Custom(name)`: an unregistered name is `CustomDispatchNotRegistered`
    ///   — never a silent fallback to a built-in rule.
    ///
    /// # `Sum`'s numeric contract
    ///
    /// When the current value and every contributing delta are all JSON
    /// integers (`serde_json::Value::as_i64` succeeds), the sum is computed
    /// with `i64::checked_add`; an exact sum outside `i64::MIN..=i64::MAX`
    /// returns `TypeMismatch` (never wraps or silently truncates). If any
    /// value involved is a JSON float, every value is promoted to `f64` and
    /// summed in floating point — a documented precision trade-off (large
    /// `i64` magnitudes may lose precision once promoted), never a silent
    /// integer truncation. A non-numeric value at any point is
    /// `TypeMismatch`.
    pub fn merge(
        &mut self,
        deltas: Vec<(NodeId, StateDelta)>,
        superstep: u64,
        custom_dispatch: &CustomDispatchResolver,
    ) -> Result<MergeReport, BattlefieldError> {
        // 1. Schema membership, validated for every touched field across all
        // deltas, before any mutation.
        for (_, delta) in &deltas {
            for field in delta.values.keys() {
                self.schema
                    .field_spec(field)
                    .ok_or_else(|| BattlefieldError::UnknownField {
                        field: field.clone(),
                    })?;
            }
        }

        let snapshot = self.values.clone();
        let mut changed_fields = Vec::new();
        let field_specs = self.schema.fields.clone();

        for field_spec in &field_specs {
            let field = &field_spec.name;
            let mut entries: Vec<(NodeId, usize, &serde_json::Value)> = Vec::new();
            for (idx, (writer, delta)) in deltas.iter().enumerate() {
                if let Some(value) = delta.values.get(field) {
                    entries.push((writer.clone(), idx, value));
                }
            }
            if entries.is_empty() {
                continue;
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

            if let Err(err) = apply_field_dispatch(
                &mut self.values,
                field,
                &field_spec.dispatch,
                &entries,
                superstep,
                custom_dispatch,
            ) {
                self.values = snapshot;
                return Err(err);
            }
            changed_fields.push(field.clone());
        }

        Ok(MergeReport { changed_fields })
    }
}

/// Apply one field's [`DispatchRule`] against every writer's sorted
/// `(NodeId, emission index, value)` contribution. Free function (not a
/// method) so [`Battlefield::merge`] can hold `&mut self.values` without also
/// borrowing `self.schema` for the duration of the match.
fn apply_field_dispatch(
    values: &mut BTreeMap<FieldName, serde_json::Value>,
    field: &FieldName,
    dispatch: &DispatchRule,
    entries: &[(NodeId, usize, &serde_json::Value)],
    superstep: u64,
    custom_dispatch: &CustomDispatchResolver,
) -> Result<(), BattlefieldError> {
    match dispatch {
        DispatchRule::LastWrite => {
            let mut writers: Vec<NodeId> = entries.iter().map(|(w, _, _)| w.clone()).collect();
            writers.sort();
            writers.dedup();
            if writers.len() > 1 {
                return Err(BattlefieldError::DispatchConflict {
                    field: field.clone(),
                    superstep,
                    writers,
                });
            }
            // `entries` is sorted by (NodeId, emission index); the last
            // entry is the winning (highest emission index) value.
            let (_, _, value) = entries
                .last()
                .expect("apply_field_dispatch is only called with a non-empty entries slice");
            values.insert(field.clone(), (*value).clone());
        }
        DispatchRule::Append => {
            let mut current = match values.remove(field) {
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
            for (_, _, value) in entries {
                current.push((*value).clone());
            }
            values.insert(field.clone(), serde_json::Value::Array(current));
        }
        DispatchRule::MergeObject => {
            let mut current = match values.remove(field) {
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
            let mut key_writers: HashMap<String, NodeId> = HashMap::new();
            for (writer, _, value) in entries {
                let incoming = value
                    .as_object()
                    .ok_or_else(|| BattlefieldError::TypeMismatch {
                        field: field.clone(),
                        expected: "object".to_string(),
                        got: json_type_name(value).to_string(),
                    })?;
                for (key, v) in incoming {
                    if let Some(prev_writer) = key_writers.get(key)
                        && prev_writer != writer
                    {
                        let mut writers = vec![prev_writer.clone(), writer.clone()];
                        writers.sort();
                        writers.dedup();
                        return Err(BattlefieldError::DispatchConflict {
                            field: field.clone(),
                            superstep,
                            writers,
                        });
                    }
                    key_writers.insert(key.clone(), writer.clone());
                    current.insert(key.clone(), v.clone());
                }
            }
            values.insert(field.clone(), serde_json::Value::Object(current));
        }
        DispatchRule::Sum => {
            let mut acc = values
                .remove(field)
                .unwrap_or(serde_json::Value::from(0_i64));
            for (_, _, value) in entries {
                acc = sum_json_numbers(field, &acc, value)?;
            }
            values.insert(field.clone(), acc);
        }
        DispatchRule::Custom(name) => {
            let resolver = custom_dispatch.get(name).ok_or_else(|| {
                BattlefieldError::CustomDispatchNotRegistered { name: name.clone() }
            })?;
            let mut current = values
                .get(field)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            for (_, _, value) in entries {
                current = resolver(&current, value)?;
            }
            values.insert(field.clone(), current);
        }
    }
    Ok(())
}

/// Sum two JSON numbers per [`Battlefield::merge`]'s documented `Sum`
/// contract: exact `i64` addition (checked, never wrapping) when both values
/// are integers, else promotion to `f64`.
fn sum_json_numbers(
    field: &FieldName,
    current: &serde_json::Value,
    delta: &serde_json::Value,
) -> Result<serde_json::Value, BattlefieldError> {
    if let (Some(cur), Some(delt)) = (current.as_i64(), delta.as_i64()) {
        return cur
            .checked_add(delt)
            .map(serde_json::Value::from)
            .ok_or_else(|| BattlefieldError::TypeMismatch {
                field: field.clone(),
                expected: "i64 sum within i64::MIN..=i64::MAX".to_string(),
                got: "overflow".to_string(),
            });
    }
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
    Ok(serde_json::Value::from(cur_f + delt_f))
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
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
    }

    fn writer(id: &str) -> NodeId {
        NodeId::new(id)
    }

    /// Merge a single writer's delta as a one-entry superstep — convenience
    /// for tests that only care about single-writer behavior.
    fn merge_one(
        bf: &mut Battlefield,
        writer_id: &str,
        delta: StateDelta,
        registry: &CustomDispatchResolver,
    ) -> Result<MergeReport, BattlefieldError> {
        bf.merge(vec![(writer(writer_id), delta)], 0, registry)
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
        merge_one(&mut bf, "n1", delta, &CustomDispatchResolver::new()).unwrap();

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
    fn merge_empty_deltas_leaves_battlefield_byte_identical() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            Some(serde_json::json!("seed")),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let before = serde_json::to_string(&bf).unwrap();
        let report = bf.merge(vec![], 0, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(report, MergeReport::default());
        let after = serde_json::to_string(&bf).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn merge_last_write_single_writer_replaces() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("x"), 1_u64).unwrap();
        merge_one(&mut bf, "n1", d1, &CustomDispatchResolver::new()).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("x"), 2_u64).unwrap();
        merge_one(&mut bf, "n1", d2, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(bf.get::<u64>(&field("x")).unwrap(), Some(2));
    }

    #[test]
    fn merge_last_write_two_writers_conflicts_and_names_both_writers() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("x"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("x"), 1_u64).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("x"), 2_u64).unwrap();

        let err = bf
            .merge(
                vec![(writer("b"), d2), (writer("a"), d1)],
                3,
                &CustomDispatchResolver::new(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::DispatchConflict {
                field: field("x"),
                superstep: 3,
                writers: vec![writer("a"), writer("b")],
            }
        );
        // Rejected merge must leave the Battlefield untouched.
        assert_eq!(bf.get::<u64>(&field("x")).unwrap(), None);
    }

    #[test]
    fn merge_append_pushes_items_single_writer() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("log"), "a").unwrap();
        merge_one(&mut bf, "n1", d1, &CustomDispatchResolver::new()).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("log"), "b").unwrap();
        merge_one(&mut bf, "n1", d2, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(
            bf.get::<Vec<String>>(&field("log")).unwrap(),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn merge_append_with_no_current_value_produces_one_element_array() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("log"), "only").unwrap();
        merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(
            bf.get::<Vec<String>>(&field("log")).unwrap(),
            Some(vec!["only".to_string()])
        );
    }

    #[test]
    fn merge_append_two_writers_orders_by_node_id_then_emission_index() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d_b = StateDelta::new();
        d_b.set(field("log"), "from-b").unwrap();
        let mut d_a = StateDelta::new();
        d_a.set(field("log"), "from-a").unwrap();

        // Input order is (b, a) — output must still be ordered by NodeId
        // lexicographically ("a" before "b"), never by input position.
        bf.merge(
            vec![(writer("b"), d_b), (writer("a"), d_a)],
            0,
            &CustomDispatchResolver::new(),
        )
        .unwrap();

        assert_eq!(
            bf.get::<Vec<String>>(&field("log")).unwrap(),
            Some(vec!["from-a".to_string(), "from-b".to_string()])
        );
    }

    #[test]
    fn merge_append_to_non_array_current_value_errors() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            Some(serde_json::json!("not-an-array")),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("log"), "x").unwrap();
        let err = merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::TypeMismatch {
                field: field("log"),
                expected: "array".to_string(),
                got: "string".to_string(),
            }
        );
    }

    #[test]
    fn merge_merge_object_disjoint_keys_merge_successfully() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("obj"),
            DispatchRule::MergeObject,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set_raw(field("obj"), serde_json::json!({"a": 1}));
        let mut d2 = StateDelta::new();
        d2.set_raw(field("obj"), serde_json::json!({"b": 2}));

        bf.merge(
            vec![(writer("a"), d1), (writer("b"), d2)],
            0,
            &CustomDispatchResolver::new(),
        )
        .unwrap();
        assert_eq!(
            bf.get_raw(&field("obj")).unwrap(),
            &serde_json::json!({"a": 1, "b": 2})
        );
    }

    #[test]
    fn merge_merge_object_same_key_conflicts() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("obj"),
            DispatchRule::MergeObject,
            None,
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set_raw(field("obj"), serde_json::json!({"a": 1}));
        let mut d2 = StateDelta::new();
        d2.set_raw(field("obj"), serde_json::json!({"a": 99}));

        let err = bf
            .merge(
                vec![(writer("b"), d2), (writer("a"), d1)],
                7,
                &CustomDispatchResolver::new(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::DispatchConflict {
                field: field("obj"),
                superstep: 7,
                writers: vec![writer("a"), writer("b")],
            }
        );
        assert_eq!(bf.get_raw(&field("obj")), None);
    }

    #[test]
    fn merge_sum_accumulates_across_writers() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("count"),
            DispatchRule::Sum,
            Some(serde_json::json!(0)),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d1 = StateDelta::new();
        d1.set(field("count"), 5_i64).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("count"), 3_i64).unwrap();
        bf.merge(
            vec![(writer("a"), d1), (writer("b"), d2)],
            0,
            &CustomDispatchResolver::new(),
        )
        .unwrap();
        assert_eq!(bf.get::<i64>(&field("count")).unwrap(), Some(8));
    }

    #[test]
    fn merge_sum_non_numeric_delta_errors() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("count"),
            DispatchRule::Sum,
            Some(serde_json::json!(0)),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("count"), "not a number").unwrap();
        let err = merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::TypeMismatch {
                field: field("count"),
                expected: "number".to_string(),
                got: "string".to_string(),
            }
        );
    }

    #[test]
    fn merge_sum_i64_overflow_returns_type_mismatch_not_a_wrapped_value() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("count"),
            DispatchRule::Sum,
            Some(serde_json::json!(i64::MAX)),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("count"), 1_i64).unwrap();
        let err = merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap_err();
        assert_eq!(
            err,
            BattlefieldError::TypeMismatch {
                field: field("count"),
                expected: "i64 sum within i64::MIN..=i64::MAX".to_string(),
                got: "overflow".to_string(),
            }
        );
        // The pre-merge value must be untouched — no silent wrap.
        assert_eq!(bf.get::<i64>(&field("count")).unwrap(), Some(i64::MAX));
    }

    #[test]
    fn merge_sum_mixed_i64_and_f64_promotes_to_f64() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("count"),
            DispatchRule::Sum,
            Some(serde_json::json!(5)),
            false,
        )]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        d.set(field("count"), 2.5_f64).unwrap();
        merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(bf.get::<f64>(&field("count")).unwrap(), Some(7.5));
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
        let err = merge_one(&mut bf, "n1", delta, &CustomDispatchResolver::new()).unwrap_err();
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
        let mut registry: CustomDispatchResolver = CustomDispatchResolver::new();
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
        merge_one(&mut bf, "n1", d1, &registry).unwrap();
        let mut d2 = StateDelta::new();
        d2.set(field("special"), 7_i64).unwrap();
        merge_one(&mut bf, "n1", d2, &registry).unwrap();
        assert_eq!(bf.get::<i64>(&field("special")).unwrap(), Some(7));
    }

    #[test]
    fn merge_unknown_field_errors() {
        let schema = BattlefieldSchema::new(vec![]);
        let mut bf = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(field("ghost"), 1_u64).unwrap();
        let err = merge_one(&mut bf, "n1", delta, &CustomDispatchResolver::new()).unwrap_err();
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
        merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap();
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
        merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap();
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
        merge_one(&mut bf, "n1", known_delta, &CustomDispatchResolver::new()).unwrap();
        let before = serde_json::to_string(&bf).unwrap();

        let mut bad_delta = StateDelta::new();
        bad_delta.set(field("ghost"), 1_u64).unwrap();
        let err = merge_one(&mut bf, "n1", bad_delta, &CustomDispatchResolver::new()).unwrap_err();
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
        merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap();

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

    // --- Task 2: deterministic multi-writer merge across all five dispatch rules ---

    #[test]
    fn merge_sorted_field_insertion_order_does_not_affect_serialized_output() {
        // Two schemas declaring the same two fields in opposite order; the
        // final Battlefield.values map (BTreeMap, sorted by FieldName) must
        // serialize identically regardless of declaration/insertion order.
        let schema_xy = BattlefieldSchema::new(vec![
            FieldSpec::new(field("x"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("y"), DispatchRule::LastWrite, None, false),
        ]);
        let schema_yx = BattlefieldSchema::new(vec![
            FieldSpec::new(field("y"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("x"), DispatchRule::LastWrite, None, false),
        ]);

        let mut bf_xy = Battlefield::new(schema_xy);
        let mut d1 = StateDelta::new();
        d1.set(field("x"), 1_u64).unwrap();
        d1.set(field("y"), 2_u64).unwrap();
        merge_one(&mut bf_xy, "n1", d1, &CustomDispatchResolver::new()).unwrap();

        let mut bf_yx = Battlefield::new(schema_yx);
        let mut d2 = StateDelta::new();
        d2.set(field("y"), 2_u64).unwrap();
        d2.set(field("x"), 1_u64).unwrap();
        merge_one(&mut bf_yx, "n1", d2, &CustomDispatchResolver::new()).unwrap();

        let json_xy = serde_json::to_string(&bf_xy).unwrap();
        let json_yx = serde_json::to_string(&bf_yx).unwrap();
        // Field declaration order differs (embedded in `schema.fields`), so
        // compare only the `values` object's own key order via the parsed
        // JSON value rather than the raw string (schema.fields legitimately
        // differs between the two Battlefields).
        let parsed_xy: serde_json::Value = serde_json::from_str(&json_xy).unwrap();
        let parsed_yx: serde_json::Value = serde_json::from_str(&json_yx).unwrap();
        assert_eq!(parsed_xy["values"], parsed_yx["values"]);
        // And the raw `values` sub-object's serialized bytes are identical
        // (BTreeMap always emits key-sorted JSON, regardless of the order
        // values were inserted).
        let values_xy = serde_json::to_string(&parsed_xy["values"]).unwrap();
        let values_yx = serde_json::to_string(&parsed_yx["values"]).unwrap();
        assert_eq!(values_xy, values_yx);
    }

    #[test]
    fn merge_determinism_20_shuffled_iterations_byte_identical_output() {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(field("log"), DispatchRule::Append, None, false),
            FieldSpec::new(
                field("total"),
                DispatchRule::Sum,
                Some(serde_json::json!(0)),
                false,
            ),
        ]);

        let build_deltas = || -> Vec<(NodeId, StateDelta)> {
            let mut deltas = Vec::new();
            for (id, log_val, sum_val) in [
                ("alpha", "a-event", 5_i64),
                ("bravo", "b-event", 3_i64),
                ("charlie", "c-event", 2_i64),
                ("delta", "d-event", 10_i64),
            ] {
                let mut d = StateDelta::new();
                d.set(field("log"), log_val).unwrap();
                d.set(field("total"), sum_val).unwrap();
                deltas.push((writer(id), d));
            }
            deltas
        };

        let mut reference: Option<String> = None;
        for seed in 0_u64..20 {
            let mut deltas = build_deltas();
            let mut rng = StdRng::seed_from_u64(seed);
            deltas.shuffle(&mut rng);

            let mut bf = Battlefield::new(schema.clone());
            bf.merge(deltas, 0, &CustomDispatchResolver::new()).unwrap();
            let serialized = serde_json::to_string(&bf).unwrap();

            match &reference {
                None => reference = Some(serialized),
                Some(expected) => assert_eq!(
                    &serialized, expected,
                    "merge output diverged on seed {seed}"
                ),
            }
        }

        // Sanity: the accumulated values are what we expect, not just
        // "identical to each other by coincidence".
        let mut deltas = build_deltas();
        let mut bf = Battlefield::new(schema);
        // Sort ascending by writer id for a predictable expected Append order.
        deltas.sort_by(|a, b| a.0.cmp(&b.0));
        let expected_log: Vec<String> = deltas
            .iter()
            .map(|(_, d)| {
                d.values
                    .get(&field("log"))
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        bf.merge(deltas, 0, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(
            bf.get::<Vec<String>>(&field("log")).unwrap(),
            Some(expected_log)
        );
        assert_eq!(bf.get::<i64>(&field("total")).unwrap(), Some(20));
    }

    #[test]
    fn merge_report_lists_changed_fields_in_schema_declaration_order() {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(field("first"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("second"), DispatchRule::LastWrite, None, false),
        ]);
        let mut bf = Battlefield::new(schema);
        let mut d = StateDelta::new();
        // Insert in reverse-of-declaration order to prove the report follows
        // schema order, not delta insertion order.
        d.set(field("second"), "s").unwrap();
        d.set(field("first"), "f").unwrap();
        let report = merge_one(&mut bf, "n1", d, &CustomDispatchResolver::new()).unwrap();
        assert_eq!(report.changed_fields, vec![field("first"), field("second")]);
    }
}
