//! Battlefield Error Types
//!
//! This module defines error types for the Battlefield typed-state system. All
//! errors carry structured fields (X-06) rather than free-form strings, so
//! callers can pattern-match on the failure without parsing an error message.
//!
//! `TypeMismatch`'s `expected`/`got` fields are type NAMES only, never the
//! offending value: an LLM output or secret stored in a Battlefield field must
//! never be able to reach a log line through this error (see phase 22 threat
//! register, T-22-02).

use thiserror::Error;

use crate::platform::container::battlefield::FieldName;
use crate::platform::container::waypoint::NodeId;

/// Errors that can occur while reading, writing, or merging Battlefield state.
///
/// All variants carry structured fields (X-06): field/type names only, never
/// the JSON value stored in a field, so a `BattlefieldError` can be logged
/// safely even when the field itself carries sensitive data.
#[derive(Debug, Error, Clone, PartialEq)]
#[non_exhaustive]
pub enum BattlefieldError {
    /// A delta or accessor referenced a field not declared in the schema.
    #[error("unknown field: {field}")]
    UnknownField {
        /// The undeclared field name.
        field: FieldName,
    },

    /// A typed accessor could not deserialize the stored value into the
    /// requested type.
    #[error("type mismatch on field {field}: expected {expected}, got {got}")]
    TypeMismatch {
        /// The field whose stored value did not match.
        field: FieldName,
        /// The requested/expected Rust type name.
        expected: String,
        /// The actual JSON value's type name.
        got: String,
    },

    /// Two or more nodes wrote to the same `LastWrite` (or otherwise
    /// conflict-checked) field within the same superstep.
    #[error("dispatch conflict on field {field} at superstep {superstep}: writers {writers:?}")]
    DispatchConflict {
        /// The field with conflicting writers.
        field: FieldName,
        /// The superstep index at which the conflict occurred.
        superstep: u64,
        /// The nodes that wrote to the field this superstep.
        writers: Vec<NodeId>,
    },

    /// A `required` field had no default and was not supplied by the initial
    /// delta before the run started.
    #[error("missing required field: {field}")]
    MissingRequiredField {
        /// The required field that was not resolvable.
        field: FieldName,
    },

    /// A stored `Battlefield`/`BattlefieldSchema` carries a schema version
    /// this build does not know how to read.
    #[error("unsupported schema version: found {found}, supported {supported}")]
    SchemaVersionUnsupported {
        /// The schema version found on the stored data.
        found: String,
        /// The schema version(s) this build supports.
        supported: String,
    },

    /// A field declared `DispatchRule::Custom(name)` but no resolver was
    /// registered under that name.
    #[error("custom dispatch rule not registered: {name}")]
    CustomDispatchNotRegistered {
        /// The unregistered custom dispatch rule name.
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_field_display_contains_field_name() {
        let err = BattlefieldError::UnknownField {
            field: FieldName::new("result").unwrap(),
        };
        assert!(err.to_string().contains("result"));
    }

    #[test]
    fn type_mismatch_carries_type_names_not_values() {
        let err = BattlefieldError::TypeMismatch {
            field: FieldName::new("result").unwrap(),
            expected: "u64".to_string(),
            got: "String".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("u64"));
        assert!(msg.contains("String"));
    }

    #[test]
    fn dispatch_conflict_lists_writers() {
        let err = BattlefieldError::DispatchConflict {
            field: FieldName::new("result").unwrap(),
            superstep: 3,
            writers: vec![NodeId::new("a"), NodeId::new("b")],
        };
        let msg = err.to_string();
        assert!(msg.contains("superstep 3"));
    }

    #[test]
    fn custom_dispatch_not_registered_names_the_rule() {
        let err = BattlefieldError::CustomDispatchNotRegistered {
            name: "merge_scores".to_string(),
        };
        assert!(err.to_string().contains("merge_scores"));
    }
}
