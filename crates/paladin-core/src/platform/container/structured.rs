//! Structured output machinery — pure value types and free functions (D-26,
//! RT-05, RT-FR-17…19).
//!
//! Everything in this module is pure: no I/O, no async, no new `paladin-core`
//! dependency (ADR-0015). The bounded repair-loop *driver* that calls these
//! functions in a loop lives one layer up, in
//! `paladin_ports::output::structured_executor_port::run_structured` — this
//! module supplies the machinery the driver is built from, not the driver
//! itself.
//!
//! # The `extract_json` lift (D-26, CF-FR-06)
//!
//! [`extract_json`] is the Phase 23 D-11 rule lifted verbatim out of
//! `paladin-battalion`'s `DirectiveParser::StructuredDirective` (its former
//! private `extract_envelope` helper): the trimmed whole output, if it
//! parses as a JSON **object**; otherwise the content of the FIRST
//! ` ```json ` fenced block found in the output. `DirectiveParser` now calls
//! this function directly instead of maintaining its own copy, so exactly
//! one implementation of "find the JSON in this model output" exists in the
//! workspace — the tool-call protocol middleware (plan 26-19) and the
//! [`run_structured`](../../../../paladin_ports/output/structured_executor_port/fn.run_structured.html)
//! repair driver are the other two consumers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::platform::container::execution_result::PaladinResult;

/// The typed value AND the raw underlying [`PaladinResult`] a structured
/// execution produced (D-26). Callers who need token/timing/stop-reason
/// metadata alongside the typed value don't lose it just because the value
/// was validated and parsed out of the raw output.
#[derive(Debug, Clone)]
pub struct Structured<T> {
    /// The typed value, extracted from the raw output and validated against
    /// the caller's schema.
    pub value: T,
    /// The underlying execution result the value was extracted from.
    pub raw: PaladinResult,
}

/// Options governing
/// [`run_structured`](../../../../paladin_ports/output/structured_executor_port/fn.run_structured.html)'s
/// bounded repair loop (D-26, RT-FR-18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StructuredOptions {
    /// How many additional attempts the driver makes, beyond the first,
    /// when the model's output fails to parse or fails [`shape_check`].
    /// `0` means exactly one call total, ever — no re-prompt.
    pub max_repair_attempts: u32,
}

impl Default for StructuredOptions {
    /// `max_repair_attempts: 1` — one repair attempt beyond the first call,
    /// by default (D-26).
    fn default() -> Self {
        Self {
            max_repair_attempts: 1,
        }
    }
}

impl StructuredOptions {
    /// Builds a [`StructuredOptions`] with a custom `max_repair_attempts`.
    ///
    /// `#[non_exhaustive]` blocks both struct-literal construction and
    /// `..Default::default()` functional update from outside this crate
    /// (X-10.3), so this constructor is the only way a downstream crate
    /// (`paladin-ports`, the facade) can build a non-default value.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::structured::StructuredOptions;
    ///
    /// let opts = StructuredOptions::new(3);
    /// assert_eq!(opts.max_repair_attempts, 3);
    /// ```
    pub fn new(max_repair_attempts: u32) -> Self {
        Self {
            max_repair_attempts,
        }
    }
}

/// A reference to a JSON Schema (D-26): either the schema carried inline, or
/// the name of a schema registered elsewhere (e.g. `WarEngine`'s schema
/// registry, plan 26-18) and resolved by the consumer, not by this module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum SchemaRef {
    /// A JSON Schema value carried inline.
    Inline(Value),
    /// The name of a schema registered elsewhere.
    Registered(String),
}

/// Why [`shape_check`] rejected a value (D-26, RT-FR-18), naming the
/// offending JSON path (`$` for root, `$.field`, `$.list[2]`, …).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ShapeError {
    /// The value at `path` did not satisfy the schema, for `reason`.
    #[error("at `{path}`: {reason}")]
    Mismatch {
        /// A JSON-pointer-like path to the offending value.
        path: String,
        /// Why the value at `path` was rejected.
        reason: String,
    },
}

impl ShapeError {
    fn at(path: &str, reason: impl Into<String>) -> Self {
        ShapeError::Mismatch {
            path: path.to_string(),
            reason: reason.into(),
        }
    }
}

/// Extract a JSON value from a model's raw text output (D-26 — the Phase 23
/// D-11 rule lifted verbatim from `directive_parser.rs`'s former private
/// `extract_envelope`): the trimmed whole output, if it parses as a JSON
/// **object**; otherwise the content of the FIRST ` ```json ` fenced block
/// found in the output, if it parses as valid JSON; otherwise `None`.
///
/// Never panics. Returns `None` for empty input, plain prose, a bare scalar
/// (`42`, `"hi"`), and a fenced block containing invalid JSON.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::structured::extract_json;
/// use serde_json::json;
///
/// assert_eq!(extract_json(r#"  {"a": 1}  "#), Some(json!({"a": 1})));
/// assert_eq!(extract_json("not json at all"), None);
/// ```
pub fn extract_json(output: &str) -> Option<Value> {
    let trimmed = output.trim();
    if let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }

    let block = first_fenced_json_block(output)?;
    serde_json::from_str::<Value>(block.trim()).ok()
}

/// The content of the FIRST ` ```json ... ``` ` fenced block in `output`, if
/// any (D-11's "first fenced block" rule, lifted verbatim from
/// `directive_parser.rs`'s former private helper of the same name).
fn first_fenced_json_block(output: &str) -> Option<&str> {
    const FENCE_OPEN: &str = "```json";
    const FENCE_CLOSE: &str = "```";
    let start = output.find(FENCE_OPEN)? + FENCE_OPEN.len();
    let rest = &output[start..];
    let end = rest.find(FENCE_CLOSE)?;
    Some(&rest[..end])
}

/// Enforces a **documented subset** of JSON Schema against `value` (D-26,
/// D-30): `type`, `required`, `properties` (recursively), `enum`, `items`,
/// `additionalProperties: false`, and `anyOf`-with-null nullability — and
/// NOTHING else.
///
/// # What this checks
///
/// - `type`: `"object"`, `"array"`, `"string"`, `"number"`, `"integer"`,
///   `"boolean"`, `"null"`.
/// - `required`: every named key of an object schema must be present.
/// - `properties`: each declared property is recursively checked against
///   its own sub-schema.
/// - `additionalProperties: false`: an object schema with this set rejects
///   any key not declared under `properties`.
/// - `enum`: the value must equal one of the listed values.
/// - `items`: an array schema's `items` sub-schema is checked against every
///   element.
/// - `anyOf` with a `{"type": "null"}` member: the shape used to express
///   `Option<T>` nullability — `null` is accepted when a null variant is
///   present, otherwise the value must match at least one other member.
///
/// # What this does NOT check
///
/// `minLength`, `maxLength`, `pattern`, `format`, `minimum`, `maximum`,
/// `minItems`, `maxItems`, `uniqueItems`, `const`, `$ref`, and every other
/// JSON Schema keyword not listed above. A schema declaring `"minLength":
/// 5` and a value of `""` for that field **passes** this check — a full
/// JSON Schema validator (the `jsonschema` crate) would be a new
/// heavyweight dependency in the default feature set for no acceptance
/// criterion (X-11.4); a richer check is a Deferred Idea behind a future
/// feature (D-30).
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::structured::shape_check;
/// use serde_json::json;
///
/// let schema = json!({"type": "object", "required": ["name"], "properties": {"name": {"type": "string"}}});
/// assert!(shape_check(&json!({"name": "Alice"}), &schema).is_ok());
/// assert!(shape_check(&json!({}), &schema).is_err());
/// ```
pub fn shape_check(value: &Value, schema: &Value) -> Result<(), ShapeError> {
    shape_check_at("$", value, schema)
}

fn shape_check_at(path: &str, value: &Value, schema: &Value) -> Result<(), ShapeError> {
    let Some(schema_obj) = schema.as_object() else {
        // A schema that is not a JSON object (e.g. a boolean schema) is
        // outside this documented subset -- treat as "anything passes"
        // rather than panicking or rejecting everything.
        return Ok(());
    };

    if let Some(any_of) = schema_obj.get("anyOf").and_then(Value::as_array) {
        if value.is_null()
            && any_of
                .iter()
                .any(|member| member.get("type").and_then(Value::as_str) == Some("null"))
        {
            return Ok(());
        }
        for member in any_of {
            if shape_check_at(path, value, member).is_ok() {
                return Ok(());
            }
        }
        return Err(ShapeError::at(path, "value matches no member of `anyOf`"));
    }

    if let Some(type_name) = schema_obj.get("type").and_then(Value::as_str) {
        check_type(path, value, type_name)?;
    }

    if let Some(enum_values) = schema_obj.get("enum").and_then(Value::as_array)
        && !enum_values.contains(value)
    {
        return Err(ShapeError::at(
            path,
            "value is not one of the schema's `enum` values",
        ));
    }

    if let Value::Object(map) = value {
        if let Some(required) = schema_obj.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !map.contains_key(key) {
                    return Err(ShapeError::at(
                        &format!("{path}.{key}"),
                        "required property is missing",
                    ));
                }
            }
        }

        let properties = schema_obj.get("properties").and_then(Value::as_object);
        if let Some(properties) = properties {
            for (key, sub_schema) in properties {
                if let Some(sub_value) = map.get(key) {
                    shape_check_at(&format!("{path}.{key}"), sub_value, sub_schema)?;
                }
            }
        }

        if schema_obj.get("additionalProperties") == Some(&Value::Bool(false)) {
            for key in map.keys() {
                let declared = properties.is_some_and(|p| p.contains_key(key));
                if !declared {
                    return Err(ShapeError::at(
                        &format!("{path}.{key}"),
                        "unexpected property, not declared under `properties` \
                         (additionalProperties: false)",
                    ));
                }
            }
        }
    }

    if let Value::Array(items) = value
        && let Some(items_schema) = schema_obj.get("items")
    {
        for (i, item) in items.iter().enumerate() {
            shape_check_at(&format!("{path}[{i}]"), item, items_schema)?;
        }
    }

    Ok(())
}

fn check_type(path: &str, value: &Value, type_name: &str) -> Result<(), ShapeError> {
    let matches = match type_name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => {
            value.is_i64() || value.is_u64() || value.as_f64().is_some_and(|f| f.fract() == 0.0)
        }
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        // An unrecognized `type` keyword value is outside this documented
        // subset -- accept rather than reject on an unknown keyword value.
        _ => true,
    };
    if matches {
        Ok(())
    } else {
        Err(ShapeError::at(
            path,
            format!(
                "expected type `{type_name}`, got {}",
                describe_json_type(value)
            ),
        ))
    }
}

fn describe_json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Renders the schema-conformance instruction appended to a prompt when
/// requesting structured output (D-26, RT-FR-18). Deterministic: the same
/// schema renders the same block every time. Wording is at Claude's
/// discretion (D-26 leaves it so) — written once, here, so every call site
/// asks a model for structured output in the same words.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::structured::render_instruction_block;
/// use serde_json::json;
///
/// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
/// let first = render_instruction_block(&schema);
/// let second = render_instruction_block(&schema);
/// assert_eq!(first, second);
/// assert!(first.contains("\"type\""));
/// ```
pub fn render_instruction_block(schema: &Value) -> String {
    let pretty = serde_json::to_string_pretty(schema).unwrap_or_else(|_| schema.to_string());
    format!(
        "\n\n---\nRespond with ONLY a single JSON value conforming EXACTLY to the following \
         JSON Schema. Do not include any prose, explanation, or Markdown code fences -- the \
         entire response must be valid JSON, parseable on its own.\n\nSchema:\n{pretty}\n---\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Task 1, Test 1 ---

    #[test]
    fn extract_json_takes_a_bare_json_object() {
        let output = "  \n  {\"a\": 1, \"b\": \"two\"}  \n  ";
        assert_eq!(
            extract_json(output),
            Some(serde_json::json!({"a": 1, "b": "two"}))
        );
    }

    // --- Task 1, Test 2 ---

    #[test]
    fn extract_json_takes_the_first_fenced_json_block() {
        let output = "Here is my answer:\n```json\n{\"a\": 1}\n```\nThanks.";
        assert_eq!(extract_json(output), Some(serde_json::json!({"a": 1})));

        let two_blocks = "first:\n```json\n{\"which\": \"first\"}\n```\n\
                           second:\n```json\n{\"which\": \"second\"}\n```\n";
        assert_eq!(
            extract_json(two_blocks),
            Some(serde_json::json!({"which": "first"})),
            "the FIRST fenced json block must win, never the last"
        );
    }

    // --- Task 1, Test 3 ---

    #[test]
    fn extract_json_returns_none_for_non_json() {
        assert_eq!(extract_json(""), None);
        assert_eq!(extract_json("just some prose, no json here"), None);
        assert_eq!(extract_json("42"), None);
        assert_eq!(extract_json("\"hi\""), None);
        assert_eq!(extract_json("```json\nnot valid json\n```"), None);
    }

    // --- Task 1, Test 5 ---

    #[test]
    fn shape_check_enforces_exactly_the_documented_subset() {
        // type mismatch
        let schema = serde_json::json!({"type": "string"});
        assert!(shape_check(&serde_json::json!(42), &schema).is_err());

        // missing required key -- error names the offending path
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {"name": {"type": "string"}}
        });
        let err = shape_check(&serde_json::json!({}), &schema).unwrap_err();
        assert!(err.to_string().contains("name"), "{err}");

        // nested properties mismatch
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "inner": {
                    "type": "object",
                    "properties": {"count": {"type": "integer"}}
                }
            }
        });
        let value = serde_json::json!({"inner": {"count": "not a number"}});
        assert!(shape_check(&value, &schema).is_err());

        // unexpected key under additionalProperties: false
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "additionalProperties": false
        });
        let value = serde_json::json!({"name": "ok", "extra": true});
        assert!(shape_check(&value, &schema).is_err());

        // value outside enum
        let schema = serde_json::json!({"enum": ["a", "b", "c"]});
        assert!(shape_check(&serde_json::json!("z"), &schema).is_err());

        // items element mismatch
        let schema = serde_json::json!({"type": "array", "items": {"type": "string"}});
        let value = serde_json::json!(["ok", 42]);
        assert!(shape_check(&value, &schema).is_err());

        // a value against an anyOf-with-null schema that matches neither branch
        let schema = serde_json::json!({"anyOf": [{"type": "string"}, {"type": "null"}]});
        assert!(shape_check(&serde_json::json!(42), &schema).is_err());

        // an ignored keyword: minLength is NOT enforced -- a too-short
        // string still passes this documented partial check.
        let schema = serde_json::json!({"type": "string", "minLength": 5});
        assert!(shape_check(&serde_json::json!("hi"), &schema).is_ok());
    }

    // --- Task 1, Test 6 ---

    #[test]
    fn shape_check_accepts_a_conforming_value() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "status"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string"},
                "status": {"enum": ["active", "inactive"]},
                "tags": {"type": "array", "items": {"type": "string"}},
                "nickname": {"anyOf": [{"type": "string"}, {"type": "null"}]}
            }
        });

        let with_null_nickname = serde_json::json!({
            "name": "Alice",
            "status": "active",
            "tags": ["a", "b"],
            "nickname": null
        });
        assert!(shape_check(&with_null_nickname, &schema).is_ok());

        let with_string_nickname = serde_json::json!({
            "name": "Bob",
            "status": "inactive",
            "tags": [],
            "nickname": "Bobby"
        });
        assert!(shape_check(&with_string_nickname, &schema).is_ok());
    }

    // --- Task 1, Test 7 ---

    #[test]
    fn render_instruction_block_is_deterministic() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"name": {"type": "string"}}
        });
        let first = render_instruction_block(&schema);
        let second = render_instruction_block(&schema);
        assert_eq!(first, second);
        assert!(first.contains("\"type\""));
        assert!(first.contains("\"name\""));
    }

    // --- Task 1, Test 8 ---

    #[test]
    fn structured_and_options_round_trip() {
        let opts = StructuredOptions::default();
        assert_eq!(opts.max_repair_attempts, 1);

        let structured = Structured {
            value: serde_json::json!({"a": 1}),
            raw: PaladinResult::default(),
        };
        assert_eq!(structured.value, serde_json::json!({"a": 1}));
        assert_eq!(structured.raw.output, "");

        let inline = SchemaRef::Inline(serde_json::json!({"type": "object"}));
        let json = serde_json::to_string(&inline).unwrap();
        let back: SchemaRef = serde_json::from_str(&json).unwrap();
        assert_eq!(inline, back);

        let registered = SchemaRef::Registered("my_schema".to_string());
        let json = serde_json::to_string(&registered).unwrap();
        let back: SchemaRef = serde_json::from_str(&json).unwrap();
        assert_eq!(registered, back);
    }
}
