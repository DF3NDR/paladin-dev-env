//! `DirectiveParser` — turning a `NodeSpec::Paladin` node's raw string
//! output into a routing [`Directive`] (CF-FR-06, D-11).
//!
//! [`DirectiveParser::PlainOutput`] is the default and reproduces today's
//! behavior verbatim (D-11, D-17 in `23-CONTEXT.md`): the raw output is
//! written to `output_field` and the node routes via its static outgoing
//! edges (`NextStep::Edges`). A v0.9-shaped graph that never opts in
//! behaves identically before and after this module exists.
//!
//! [`DirectiveParser::StructuredDirective`] instead parses a documented JSON
//! envelope out of the output and applies ONLY the envelope's `delta` — it
//! performs no implicit `output_field` write. The envelope shape:
//!
//! ```json
//! {"delta": {"<field>": <json>, "..."}, "next": "edges" | {"goto": ["node_id"]} | "end" | {"muster": [{"worker": "w", "payload": {}, "task_key": "1"}]}}
//! ```
//!
//! Extraction follows exactly one order, locked by D-11, checked in this
//! sequence:
//!
//! 1. The trimmed whole output, if it parses as a JSON object AND
//!    deserializes into a valid envelope.
//! 2. Otherwise, the content of the FIRST ` ```json ` fenced block found in
//!    the output, if it deserializes into a valid envelope.
//! 3. Otherwise, resolved through [`OnParseError`].
//!
//! An envelope's `delta` is handed to the same [`StateDelta`] the engine
//! already merges through `Battlefield::merge`'s existing schema dispatch —
//! a field name the schema does not declare fails the run as
//! `BattlefieldError::UnknownField` exactly as any other node's delta would
//! (T-23-14); this module performs no separate allowlist check of its own.

use serde::{Deserialize, Serialize};

use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
use paladin_core::platform::container::waypoint::NodeId;

/// How a `NodeSpec::Paladin` node's raw string output is turned into a
/// routing [`Directive`] (CF-FR-06, D-11).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DirectiveParser {
    /// Write the raw output to `output_field` and route via the node's
    /// static outgoing edges (`NextStep::Edges`) — byte-identical to a
    /// pre-CF-02 Paladin node. The default (D-11, D-17).
    #[default]
    PlainOutput,
    /// Parse D-11's documented JSON envelope out of the output and apply
    /// ONLY the envelope's `delta` — no implicit `output_field` write.
    /// `output_field` is the target [`OnParseError::FallbackPlain`] falls
    /// back to.
    StructuredDirective {
        /// How to resolve a failure to extract a valid envelope from the
        /// output.
        on_parse_error: OnParseError,
    },
}

/// How [`DirectiveParser::StructuredDirective`] resolves a failure to
/// extract a valid envelope from a node's output (D-11, PRD CF-FR-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OnParseError {
    /// Fail the run with a typed error naming the node and the parse
    /// failure (`EngineError::DirectiveParseFailed`). The default (PRD).
    #[default]
    FailRun,
    /// Degrade to [`DirectiveParser::PlainOutput`] semantics: write the raw
    /// output to `output_field` and route via the node's static outgoing
    /// edges.
    FallbackPlain,
}

/// [`DirectiveParser::parse`]'s error. Carries no node identity — the
/// caller (`engine::superstep`) attaches the failing node id when folding
/// this into `EngineError::DirectiveParseFailed` (X-06: the typed engine
/// error, not this type, is the one carrying structured node context).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{reason}")]
pub struct DirectiveParseError {
    /// Why extraction/deserialization of a structured envelope failed.
    pub reason: String,
}

/// D-11's documented JSON envelope shape, deserialized directly by
/// [`extract_envelope`].
#[derive(Debug, Deserialize)]
struct Envelope {
    /// The state delta this envelope contributes. Field-name validity
    /// against the Battlefield schema is deliberately NOT checked here —
    /// `Battlefield::merge` is the single allowlist (T-23-14).
    delta: serde_json::Map<String, serde_json::Value>,
    /// How the superstep engine should route control after this node.
    next: EnvelopeNextStep,
}

/// The envelope's `next` shape (D-11): a lowercase-tagged mirror of
/// [`NextStep`] restricted to the variants an envelope may name, matching
/// serde's default externally-tagged representation for a unit variant
/// (`"edges"`, `"end"`) versus a single-field tuple variant
/// (`{"goto": [...]}`, `{"muster": [...]}`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum EnvelopeNextStep {
    /// `"edges"`.
    Edges,
    /// `{"goto": ["node_id", ...]}`.
    Goto(Vec<NodeId>),
    /// `"end"`.
    End,
    /// `{"muster": [{"worker": ..., "payload": ..., "task_key": ...}, ...]}`.
    /// The dispatch mechanism for this variant lands in a later plan
    /// (CF-03); this module only parses the shape.
    Muster(Vec<MusterTask>),
}

impl From<EnvelopeNextStep> for NextStep {
    fn from(next: EnvelopeNextStep) -> Self {
        match next {
            EnvelopeNextStep::Edges => NextStep::Edges,
            EnvelopeNextStep::Goto(targets) => NextStep::Goto(targets),
            EnvelopeNextStep::End => NextStep::End,
            EnvelopeNextStep::Muster(tasks) => NextStep::Muster(tasks),
        }
    }
}

impl DirectiveParser {
    /// Turn `output` — a Paladin node's raw string output — into a routing
    /// [`Directive`], per this parser's kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::directive_parser::{DirectiveParser, OnParseError};
    /// use paladin_core::platform::container::battlefield::FieldName;
    /// use paladin_core::platform::container::directive::NextStep;
    ///
    /// let parser = DirectiveParser::StructuredDirective {
    ///     on_parse_error: OnParseError::FailRun,
    /// };
    /// let output = r#"{"delta": {"verdict": "approved"}, "next": "edges"}"#;
    /// let directive = parser
    ///     .parse(output, &FieldName::new("raw_output").unwrap())
    ///     .unwrap();
    ///
    /// assert_eq!(directive.next, NextStep::Edges);
    /// assert_eq!(
    ///     directive.delta.values.get(&FieldName::new("verdict").unwrap()),
    ///     Some(&serde_json::json!("approved"))
    /// );
    /// ```
    pub fn parse(
        &self,
        output: &str,
        output_field: &FieldName,
    ) -> Result<Directive, DirectiveParseError> {
        match self {
            DirectiveParser::PlainOutput => Ok(plain_output_directive(output, output_field)),
            DirectiveParser::StructuredDirective { on_parse_error } => {
                match extract_envelope(output) {
                    Some(envelope) => Ok(envelope_to_directive(envelope)),
                    None => match on_parse_error {
                        OnParseError::FailRun => Err(DirectiveParseError {
                            reason: format!(
                                "StructuredDirective could not extract a valid JSON envelope \
                                 from the node's output (tried the trimmed whole output, then \
                                 the first ```json fenced block): {output:?}"
                            ),
                        }),
                        OnParseError::FallbackPlain => {
                            Ok(plain_output_directive(output, output_field))
                        }
                    },
                }
            }
        }
    }
}

/// `PlainOutput` semantics, shared by [`DirectiveParser::PlainOutput`] and
/// [`OnParseError::FallbackPlain`]'s degraded path: write the raw output to
/// `output_field` and route via `NextStep::Edges` — byte-identical to a
/// pre-CF-02 Paladin node's write.
fn plain_output_directive(output: &str, output_field: &FieldName) -> Directive {
    let mut delta = StateDelta::new();
    delta.set_raw(
        output_field.clone(),
        serde_json::Value::String(output.to_string()),
    );
    delta.into()
}

/// Fold a successfully-extracted [`Envelope`] into a [`Directive`], applying
/// ONLY the envelope's `delta` — no implicit `output_field` write (D-11).
fn envelope_to_directive(envelope: Envelope) -> Directive {
    let mut delta = StateDelta::new();
    for (field, value) in envelope.delta {
        // A `FieldName` rejects only the empty string; no Battlefield
        // schema ever declares a field named "" either, so an empty key
        // here can never match a real field regardless of how it is
        // handled. Silently skipping it changes nothing observable.
        if let Ok(field_name) = FieldName::new(field) {
            delta.set_raw(field_name, value);
        }
    }
    Directive {
        delta,
        next: envelope.next.into(),
    }
}

/// D-11's locked extraction order: (i) the trimmed whole output, if it
/// parses as a JSON object AND deserializes into a valid [`Envelope`]; (ii)
/// otherwise the FIRST ` ```json ` fenced block found in the output, under
/// the same parses-and-valid test; (iii) otherwise `None`, resolved by the
/// caller through [`OnParseError`]. The order is locked, not discretionary
/// (D-11) — do not reorder these two clauses.
fn extract_envelope(output: &str) -> Option<Envelope> {
    let trimmed = output.trim();
    if let Ok(serde_json::Value::Object(_)) = serde_json::from_str::<serde_json::Value>(trimmed)
        && let Ok(envelope) = serde_json::from_str::<Envelope>(trimmed)
    {
        return Some(envelope);
    }

    let block = first_fenced_json_block(output)?;
    serde_json::from_str::<Envelope>(block.trim()).ok()
}

/// The content of the FIRST ` ```json ... ``` ` fenced block in `output`, if
/// any (D-11's "first fenced block" rule — pinned by
/// `output_with_two_fenced_json_blocks_uses_the_first`).
fn first_fenced_json_block(output: &str) -> Option<&str> {
    const FENCE_OPEN: &str = "```json";
    const FENCE_CLOSE: &str = "```";
    let start = output.find(FENCE_OPEN)? + FENCE_OPEN.len();
    let rest = &output[start..];
    let end = rest.find(FENCE_CLOSE)?;
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
    }

    #[test]
    fn plain_output_is_the_default_and_writes_the_output_field() {
        assert_eq!(DirectiveParser::default(), DirectiveParser::PlainOutput);

        let parser = DirectiveParser::PlainOutput;
        let directive = parser.parse("hello world", &field("out")).unwrap();
        assert_eq!(directive.next, NextStep::Edges);
        assert_eq!(
            directive.delta.values.get(&field("out")),
            Some(&serde_json::json!("hello world"))
        );
    }

    #[test]
    fn structured_directive_parses_a_fenced_json_block() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        let output = "Here is my answer:\n```json\n\
                       {\"delta\": {\"verdict\": \"approved\"}, \"next\": \"edges\"}\n\
                       ```\nThanks.";
        let directive = parser.parse(output, &field("out")).unwrap();
        assert_eq!(directive.next, NextStep::Edges);
        assert_eq!(
            directive.delta.values.get(&field("verdict")),
            Some(&serde_json::json!("approved"))
        );
        assert!(
            !directive.delta.values.contains_key(&field("out")),
            "StructuredDirective never performs an implicit output_field write"
        );
    }

    #[test]
    fn malformed_output_under_fail_run_fails_the_run() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        let err = parser.parse("not json at all", &field("out")).unwrap_err();
        assert!(!err.reason.is_empty());
    }

    #[test]
    fn malformed_output_under_fallback_plain_writes_the_raw_output() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FallbackPlain,
        };
        let directive = parser.parse("not json at all", &field("out")).unwrap();
        assert_eq!(directive.next, NextStep::Edges);
        assert_eq!(
            directive.delta.values.get(&field("out")),
            Some(&serde_json::json!("not json at all"))
        );
    }

    #[test]
    fn default_on_parse_error_is_fail_run() {
        assert_eq!(OnParseError::default(), OnParseError::FailRun);
    }

    // --- Task 2 (red): pinning D-11's extraction order and the
    // deny_unknown_fields rejection not yet added to `Envelope`.

    #[test]
    fn empty_output_resolves_through_on_parse_error() {
        let fail_run = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        assert!(fail_run.parse("", &field("out")).is_err());

        let fallback = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FallbackPlain,
        };
        let directive = fallback.parse("", &field("out")).unwrap();
        assert_eq!(
            directive.delta.values.get(&field("out")),
            Some(&serde_json::json!(""))
        );
    }

    #[test]
    fn output_with_two_fenced_json_blocks_uses_the_first() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        let output = "first:\n```json\n\
                       {\"delta\": {\"which\": \"first\"}, \"next\": \"edges\"}\n\
                       ```\nsecond:\n```json\n\
                       {\"delta\": {\"which\": \"second\"}, \"next\": \"edges\"}\n\
                       ```\n";
        let directive = parser.parse(output, &field("out")).unwrap();
        assert_eq!(
            directive.delta.values.get(&field("which")),
            Some(&serde_json::json!("first")),
            "the FIRST fenced json block must win, never the last"
        );
    }

    #[test]
    fn envelope_with_an_unknown_top_level_key_is_rejected() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        let output = r#"{"delta": {"verdict": "approved"}, "next": "edges", "bogus": true}"#;
        assert!(
            parser.parse(output, &field("out")).is_err(),
            "an envelope carrying a key other than delta/next must fail, not be ignored"
        );
    }

    #[test]
    fn structured_directive_goto_parses_into_next_goto() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        let output = r#"{"delta": {}, "next": {"goto": ["node_a", "node_b"]}}"#;
        let directive = parser.parse(output, &field("out")).unwrap();
        assert_eq!(
            directive.next,
            NextStep::Goto(vec![NodeId::new("node_a"), NodeId::new("node_b")])
        );
    }

    #[test]
    fn structured_directive_end_parses_into_next_end() {
        let parser = DirectiveParser::StructuredDirective {
            on_parse_error: OnParseError::FailRun,
        };
        let output = r#"{"delta": {}, "next": "end"}"#;
        let directive = parser.parse(output, &field("out")).unwrap();
        assert_eq!(directive.next, NextStep::End);
    }
}
