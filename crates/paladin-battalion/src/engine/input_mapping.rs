//! `InputMapping`: the X-03 string bridge between typed `Battlefield` state
//! and a legacy string-in/string-out `Paladin`.
//!
//! A template string with `{field}` placeholders is resolved against a
//! [`Battlefield`] snapshot before being handed to
//! [`PaladinPort::execute`](paladin_ports::output::paladin_port::PaladinPort::execute)
//! as the Paladin's plain-text input. This is the ONLY place a typed
//! Battlefield field value is converted to a bare string for a Paladin that
//! knows nothing about typed state (ENG-FR-13, X-03) — the resolution rules
//! below are therefore a compatibility contract, not an implementation
//! detail:
//!
//! - A placeholder over a field whose value is a JSON **string** is inserted
//!   **raw**, with no surrounding quotes, so a legacy string-in/string-out
//!   Paladin sees exactly the text it would have seen before typed state
//!   existed.
//! - A placeholder over a field whose value is any other JSON type (object,
//!   array, number, bool, null) is inserted as that value's JSON-stringified
//!   form.
//! - A placeholder naming a field the Battlefield's schema does not declare
//!   is a typed [`InputMappingError::UndeclaredField`] — **never** an empty
//!   substitution, which would silently truncate a prompt and could not be
//!   distinguished from a legitimately empty field.
//! - A placeholder naming a declared field with no current value falls back
//!   to that field's schema `default`; a declared field with no value and no
//!   default is [`InputMappingError::NoValueOrDefault`].
//! - A template with no placeholder renders literally, unchanged.
//! - A placeholder in the `muster.` namespace (`{muster.payload}`,
//!   `{muster.task_key}`, CF-03 / D-15) resolves ONLY from the executing
//!   node's [`MusterContext`](paladin_core::platform::container::directive::MusterContext)
//!   — passed to [`InputMapping::render`] separately — never from the
//!   Battlefield. With no muster context present (an ordinary,
//!   non-Muster-worker execution), such a placeholder is a typed
//!   [`InputMappingError::UndeclaredField`], never a silent Battlefield
//!   read. Graph validation independently rejects any schema field named
//!   with the `muster.` prefix, so this namespace can never be shadowed.
//! - A placeholder in the `parley.` namespace (`{parley.value}`,
//!   `{parley.prompt}`, `{parley.kind}`, `{parley.responded_by}`, HITL-01,
//!   D-07) resolves ONLY from the executing node's own outstanding
//!   [`ParleyResponse`](paladin_core::platform::container::parley::ParleyResponse)
//!   — passed to [`InputMapping::render`] separately, `Some` only on the
//!   post-resume re-run of a parleying node — never from the Battlefield.
//!   `kind`/`prompt` are stamped onto the response by `WarEngine::resume_with`
//!   from the matching `ParleyRequest`, so all four keys resolve from this
//!   one type; `responded_by` renders as an empty string for a D-12
//!   `OnExpire::ResumeWithDefault` substitution (`responded_by: None`),
//!   never a panic. With no parley context present, or an unrecognized key
//!   under the prefix, such a placeholder is a typed
//!   [`InputMappingError::UndeclaredField`], never a silent Battlefield
//!   read. Graph validation independently rejects any schema field named
//!   with the `parley.` prefix (23 D-15's precedent, extended), so this
//!   namespace can never be shadowed either (T-24-09).

use thiserror::Error;

use paladin_core::platform::container::battlefield::{Battlefield, FieldName};
use paladin_core::platform::container::directive::MusterContext;
use paladin_core::platform::container::parley::ParleyResponse;

/// Error returned by [`InputMapping::render`].
///
/// Both variants carry the raw placeholder text (not a `FieldName`, since an
/// [`UndeclaredField`](InputMappingError::UndeclaredField) may not even be a
/// syntactically valid one) so a failing render names exactly which
/// placeholder in the template could not be resolved.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum InputMappingError {
    /// A `{field}` placeholder named a field the Battlefield's schema does
    /// not declare.
    #[error("input mapping references undeclared field: {field}")]
    UndeclaredField {
        /// The undeclared placeholder text.
        field: String,
    },
    /// A `{field}` placeholder named a declared field with no current value
    /// and no schema default to fall back to.
    #[error("input mapping references field {field} with no value and no default")]
    NoValueOrDefault {
        /// The declared field with no resolvable value.
        field: FieldName,
    },
}

/// Renders a Paladin's string input from a [`Battlefield`]: a template
/// string with `{field}` placeholders resolved from typed state.
///
/// See the module-level rustdoc for the exact resolution/error contract
/// (the X-03 bridge).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InputMapping {
    template: String,
}

impl InputMapping {
    /// Construct an `InputMapping` from a template string containing
    /// `{field}` placeholders.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }

    /// Render this template against `state`, substituting each `{field}`
    /// placeholder per the module-level resolution rules. `muster` is the
    /// executing node's Muster task context (CF-03, D-15) -- `Some` only
    /// for a worker-template dispatch, in which case a `{muster.payload}`/
    /// `{muster.task_key}` placeholder resolves from it; `None` for every
    /// ordinary execution, in which case such a placeholder is a typed
    /// error rather than a silent Battlefield read. `parley` is the
    /// executing node's own outstanding `ParleyResponse` (HITL-01, D-07) --
    /// `Some` only on the post-resume re-run of a parleying node, in which
    /// case a `{parley.value}`/`{parley.prompt}`/`{parley.kind}`/
    /// `{parley.responded_by}` placeholder resolves from it; `None` for
    /// every other execution (including a parleying node's OWN first
    /// visit, which raises the parley rather than answers it), in which
    /// case such a placeholder is a typed error rather than a silent
    /// Battlefield read.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::InputMapping;
    /// use paladin_core::platform::container::battlefield::{
    ///     Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    /// };
    ///
    /// let schema = BattlefieldSchema::new(vec![FieldSpec::new(
    ///     FieldName::new("topic").unwrap(),
    ///     DispatchRule::LastWrite,
    ///     Some(serde_json::json!("rust")),
    ///     false,
    /// )]);
    /// let battlefield = Battlefield::new(schema);
    ///
    /// let mapping = InputMapping::new("research {topic}");
    /// assert_eq!(
    ///     mapping.render(&battlefield, None, None).unwrap(),
    ///     "research rust"
    /// );
    /// ```
    pub fn render(
        &self,
        state: &Battlefield,
        muster: Option<&MusterContext>,
        parley: Option<&ParleyResponse>,
    ) -> Result<String, InputMappingError> {
        let mut rendered = String::with_capacity(self.template.len());
        let mut rest = self.template.as_str();

        while let Some(rel_start) = rest.find('{') {
            rendered.push_str(&rest[..rel_start]);
            let after_open = &rest[rel_start + 1..];
            let Some(rel_end) = after_open.find('}') else {
                // No closing brace: the rest of the template (including the
                // lone `{`) is literal text.
                rendered.push_str(&rest[rel_start..]);
                rest = "";
                break;
            };
            let placeholder = &after_open[..rel_end];
            rendered.push_str(&Self::resolve(placeholder, state, muster, parley)?);
            rest = &after_open[rel_end + 1..];
        }
        rendered.push_str(rest);

        Ok(rendered)
    }

    /// Resolve one `{field}` placeholder's text against `state`, or against
    /// `muster`/`parley` for a `muster.`/`parley.`-prefixed placeholder
    /// (CF-03/D-15, HITL-01/D-07).
    fn resolve(
        placeholder: &str,
        state: &Battlefield,
        muster: Option<&MusterContext>,
        parley: Option<&ParleyResponse>,
    ) -> Result<String, InputMappingError> {
        if let Some(name) = placeholder.strip_prefix("muster.") {
            return Self::resolve_muster(name, placeholder, muster);
        }
        if let Some(name) = placeholder.strip_prefix("parley.") {
            return Self::resolve_parley(name, placeholder, parley);
        }

        let field =
            FieldName::new(placeholder).map_err(|_| InputMappingError::UndeclaredField {
                field: placeholder.to_string(),
            })?;
        let spec = state.schema().field_spec(&field).ok_or_else(|| {
            InputMappingError::UndeclaredField {
                field: placeholder.to_string(),
            }
        })?;

        let value = match state.get_raw(&field) {
            Some(value) => value,
            None => spec
                .default
                .as_ref()
                .ok_or_else(|| InputMappingError::NoValueOrDefault {
                    field: field.clone(),
                })?,
        };

        Ok(match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
    }

    /// Resolve a `muster.`-namespaced placeholder (`name` is the text after
    /// the `muster.` prefix) from `muster`, never from the Battlefield
    /// (D-15). Absent context, or an unrecognized name, is
    /// [`InputMappingError::UndeclaredField`] naming the FULL placeholder
    /// (including the `muster.` prefix) -- never a silent Battlefield
    /// fallback.
    fn resolve_muster(
        name: &str,
        placeholder: &str,
        muster: Option<&MusterContext>,
    ) -> Result<String, InputMappingError> {
        let ctx = muster.ok_or_else(|| InputMappingError::UndeclaredField {
            field: placeholder.to_string(),
        })?;
        match name {
            "payload" => Ok(match &ctx.payload {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }),
            "task_key" => Ok(ctx.task_key.clone()),
            _ => Err(InputMappingError::UndeclaredField {
                field: placeholder.to_string(),
            }),
        }
    }

    /// Resolve a `parley.`-namespaced placeholder (`name` is the text after
    /// the `parley.` prefix) from `parley`, never from the Battlefield
    /// (HITL-01, D-07, T-24-09). Absent context, or an unrecognized name,
    /// is [`InputMappingError::UndeclaredField`] naming the FULL placeholder
    /// (including the `parley.` prefix) -- never a silent Battlefield
    /// fallback.
    ///
    /// Supported keys, all sourced from this ONE [`ParleyResponse`] (`kind`
    /// and `prompt` are stamped onto it by `WarEngine::resume_with` from
    /// the matching `ParleyRequest`, never supplied meaningfully by an
    /// external caller):
    /// - `value`: the response's JSON value, rendered per the module-level
    ///   string/other-JSON-type rule.
    /// - `prompt`: the originating request's prompt, verbatim.
    /// - `kind`: the `ParleyKind` discriminant's `Debug` name (`"Approval"`,
    ///   `"Choice"`, `"FreeText"`, `"StateEdit"`).
    /// - `responded_by`: the responder, or an empty string for a D-12
    ///   `OnExpire::ResumeWithDefault` substitution (`responded_by: None`)
    ///   -- a documented, defined rendering, never a panic.
    fn resolve_parley(
        name: &str,
        placeholder: &str,
        parley: Option<&ParleyResponse>,
    ) -> Result<String, InputMappingError> {
        let response = parley.ok_or_else(|| InputMappingError::UndeclaredField {
            field: placeholder.to_string(),
        })?;
        match name {
            "value" => Ok(match &response.value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }),
            "prompt" => Ok(response.prompt.clone()),
            "kind" => Ok(format!("{:?}", response.kind)),
            "responded_by" => Ok(response.responded_by.clone().unwrap_or_default()),
            _ => Err(InputMappingError::UndeclaredField {
                field: placeholder.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldSpec, StateDelta,
    };
    use paladin_core::platform::container::waypoint::NodeId;

    fn schema_with(fields: Vec<FieldSpec>) -> BattlefieldSchema {
        BattlefieldSchema::new(fields)
    }

    fn field(name: &str, dispatch: DispatchRule, default: Option<serde_json::Value>) -> FieldSpec {
        FieldSpec::new(FieldName::new(name).unwrap(), dispatch, default, false)
    }

    fn battlefield_with_value(
        schema: BattlefieldSchema,
        field_name: &str,
        value: serde_json::Value,
    ) -> Battlefield {
        let mut battlefield = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set_raw(FieldName::new(field_name).unwrap(), value);
        battlefield
            .merge(
                vec![(NodeId::new("writer"), delta)],
                0,
                &CustomDispatchResolver::new(),
            )
            .unwrap();
        battlefield
    }

    #[test]
    fn renders_json_string_field_raw_with_no_surrounding_quotes() {
        let schema = schema_with(vec![field("name", DispatchRule::LastWrite, None)]);
        let battlefield = battlefield_with_value(schema, "name", serde_json::json!("world"));

        let mapping = InputMapping::new("hello {name}!");
        assert_eq!(
            mapping.render(&battlefield, None, None).unwrap(),
            "hello world!"
        );
    }

    #[test]
    fn renders_json_object_field_as_stringified_json() {
        let schema = schema_with(vec![field("payload", DispatchRule::LastWrite, None)]);
        let battlefield =
            battlefield_with_value(schema, "payload", serde_json::json!({"a": 1, "b": "x"}));

        let mapping = InputMapping::new("data={payload}");
        let rendered = mapping.render(&battlefield, None, None).unwrap();
        assert_eq!(rendered, r#"data={"a":1,"b":"x"}"#);
    }

    #[test]
    fn renders_json_number_field_as_its_json_form() {
        let schema = schema_with(vec![field("count", DispatchRule::LastWrite, None)]);
        let battlefield = battlefield_with_value(schema, "count", serde_json::json!(42));

        let mapping = InputMapping::new("count={count}");
        assert_eq!(
            mapping.render(&battlefield, None, None).unwrap(),
            "count=42"
        );
    }

    #[test]
    fn renders_two_placeholders_both_in_position() {
        let schema = schema_with(vec![
            field("first", DispatchRule::LastWrite, None),
            field("second", DispatchRule::LastWrite, None),
        ]);
        let mut battlefield = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set_raw(FieldName::new("first").unwrap(), serde_json::json!("a"));
        delta.set_raw(FieldName::new("second").unwrap(), serde_json::json!("b"));
        battlefield
            .merge(
                vec![(NodeId::new("writer"), delta)],
                0,
                &CustomDispatchResolver::new(),
            )
            .unwrap();

        let mapping = InputMapping::new("{first}-{second}");
        assert_eq!(mapping.render(&battlefield, None, None).unwrap(), "a-b");
    }

    #[test]
    fn undeclared_field_is_a_typed_error_not_an_empty_substitution() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("value={missing}");
        let err = mapping.render(&battlefield, None, None).unwrap_err();
        assert_eq!(
            err,
            InputMappingError::UndeclaredField {
                field: "missing".to_string()
            }
        );
    }

    #[test]
    fn declared_field_with_no_value_renders_schema_default() {
        let schema = schema_with(vec![field(
            "topic",
            DispatchRule::LastWrite,
            Some(serde_json::json!("rust")),
        )]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("about {topic}");
        assert_eq!(
            mapping.render(&battlefield, None, None).unwrap(),
            "about rust"
        );
    }

    #[test]
    fn declared_field_with_no_value_and_no_default_is_a_typed_error() {
        let schema = schema_with(vec![field("topic", DispatchRule::LastWrite, None)]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("about {topic}");
        let err = mapping.render(&battlefield, None, None).unwrap_err();
        assert_eq!(
            err,
            InputMappingError::NoValueOrDefault {
                field: FieldName::new("topic").unwrap()
            }
        );
    }

    #[test]
    fn template_with_no_placeholder_renders_literally() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("no placeholders here");
        assert_eq!(
            mapping.render(&battlefield, None, None).unwrap(),
            "no placeholders here"
        );
    }

    #[test]
    fn unclosed_brace_renders_as_literal_text() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("dangling {brace");
        assert_eq!(
            mapping.render(&battlefield, None, None).unwrap(),
            "dangling {brace"
        );
    }

    // --- CF-03, D-15: the `muster.` namespace (Plan 23-05, Task 3).

    #[test]
    fn renders_muster_payload_placeholder_from_context() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);
        let ctx = MusterContext {
            payload: serde_json::json!("widget-42"),
            task_key: "k1".to_string(),
        };

        let mapping = InputMapping::new("process {muster.payload}");
        assert_eq!(
            mapping.render(&battlefield, Some(&ctx), None).unwrap(),
            "process widget-42"
        );
    }

    #[test]
    fn renders_muster_task_key_placeholder_from_context() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);
        let ctx = MusterContext {
            payload: serde_json::json!({"a": 1}),
            task_key: "task-7".to_string(),
        };

        let mapping = InputMapping::new("key={muster.task_key}");
        assert_eq!(
            mapping.render(&battlefield, Some(&ctx), None).unwrap(),
            "key=task-7"
        );
    }

    #[test]
    fn muster_placeholder_with_no_context_is_a_typed_error_not_a_battlefield_read() {
        // A Battlefield field literally named "muster.payload" (graph
        // validation rejects declaring this in a real WarGraph schema, but
        // this unit test exercises InputMapping::render in isolation) must
        // NOT be read when no muster context is present -- the placeholder
        // fails typed rather than silently falling through to state.get_raw.
        let schema = schema_with(vec![field(
            "muster.payload",
            DispatchRule::LastWrite,
            Some(serde_json::json!("battlefield-value")),
        )]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("{muster.payload}");
        let err = mapping.render(&battlefield, None, None).unwrap_err();
        assert_eq!(
            err,
            InputMappingError::UndeclaredField {
                field: "muster.payload".to_string()
            }
        );
    }

    #[test]
    fn unrecognized_muster_placeholder_name_is_a_typed_error() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);
        let ctx = MusterContext {
            payload: serde_json::json!("x"),
            task_key: "k".to_string(),
        };

        let mapping = InputMapping::new("{muster.nonexistent}");
        let err = mapping.render(&battlefield, Some(&ctx), None).unwrap_err();
        assert_eq!(
            err,
            InputMappingError::UndeclaredField {
                field: "muster.nonexistent".to_string()
            }
        );
    }

    // --- HITL-01, D-07: the `parley.` namespace (Plan 24-03, Task 1).

    use paladin_core::platform::container::parley::{ParleyId, ParleyKind};

    fn sample_parley_response(responded_by: Option<&str>, defaulted: bool) -> ParleyResponse {
        ParleyResponse {
            parley_id: ParleyId::new(),
            kind: ParleyKind::Approval,
            prompt: "Proceed with the deploy?".to_string(),
            value: serde_json::json!(true),
            responded_by: responded_by.map(str::to_string),
            responded_at: chrono::Utc::now(),
            defaulted,
        }
    }

    #[test]
    fn parley_namespace_resolves_from_node_context() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);
        let response = sample_parley_response(Some("ops@example.com"), false);

        let mapping = InputMapping::new(
            "value={parley.value} prompt={parley.prompt} kind={parley.kind} by={parley.responded_by}",
        );
        assert_eq!(
            mapping.render(&battlefield, None, Some(&response)).unwrap(),
            "value=true prompt=Proceed with the deploy? kind=Approval by=ops@example.com"
        );
    }

    #[test]
    fn parley_namespace_never_reads_battlefield() {
        // A Battlefield field literally named "parley.value" (graph
        // validation rejects declaring this in a real WarGraph schema, but
        // this unit test exercises InputMapping::render in isolation) must
        // NOT be read -- the placeholder still resolves from the NodeContext
        // parley response, never falling through to state.get_raw.
        let schema = schema_with(vec![field(
            "parley.value",
            DispatchRule::LastWrite,
            Some(serde_json::json!("battlefield-value")),
        )]);
        let battlefield = Battlefield::new(schema);
        let response = sample_parley_response(Some("alice"), false);

        let mapping = InputMapping::new("{parley.value}");
        assert_eq!(
            mapping.render(&battlefield, None, Some(&response)).unwrap(),
            "true"
        );
    }

    #[test]
    fn parley_namespace_without_context_is_typed_error() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);

        let mapping = InputMapping::new("{parley.value}");
        let err = mapping.render(&battlefield, None, None).unwrap_err();
        assert_eq!(
            err,
            InputMappingError::UndeclaredField {
                field: "parley.value".to_string()
            }
        );
    }

    #[test]
    fn parley_namespace_unknown_key_is_typed_error() {
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);
        let response = sample_parley_response(Some("alice"), false);

        let mapping = InputMapping::new("{parley.nonexistent}");
        let err = mapping
            .render(&battlefield, None, Some(&response))
            .unwrap_err();
        assert_eq!(
            err,
            InputMappingError::UndeclaredField {
                field: "parley.nonexistent".to_string()
            }
        );
    }

    #[test]
    fn responded_by_none_renders_empty_for_defaulted_response() {
        // D-12: a ResumeWithDefault expiry substitution carries
        // `responded_by: None` -- `{parley.responded_by}` must render a
        // defined, documented empty string rather than panicking.
        let schema = schema_with(vec![]);
        let battlefield = Battlefield::new(schema);
        let response = sample_parley_response(None, true);

        let mapping = InputMapping::new("by=[{parley.responded_by}]");
        assert_eq!(
            mapping.render(&battlefield, None, Some(&response)).unwrap(),
            "by=[]"
        );
    }
}
