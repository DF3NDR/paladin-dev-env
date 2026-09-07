//! `Guardrail`: named rules that screen outbound prompt sections and inbound
//! responses with a regex or a code predicate, acting by failing the run,
//! redacting in place, or finishing with a fixed message (D-09, RT-FR-07).
//!
//! # Security: why a config-supplied regex is not a ReDoS surface
//!
//! The `regex` crate compiles to a finite-automata engine with **no
//! backtracking** and guarantees **linear-time matching** in the length of
//! the haystack — an adversarial *haystack* cannot make a compiled pattern
//! run slowly, no matter how the pattern is written. The residual concern is
//! an adversarial *pattern*: a large or deeply-nested expression can compile
//! to a very large program, which the crate bounds internally at a generous
//! default. This module never relies on that implicit default — every
//! [`GuardrailMatcher::Regex`] pattern compiles through
//! [`regex::RegexBuilder::size_limit`] with the explicit,
//! [`GuardrailConfig::pattern_size_limit_bytes`]-documented bound
//! (RESEARCH.md Pitfall 5), and a pattern that would exceed it fails
//! [`Guardrail::new`]/[`Guardrail::from_rules`] with a typed
//! [`GuardrailBuildError`] naming the offending rule — never a panic, never a
//! silent truncation, and never reached at runtime. Substituting
//! `fancy-regex` (a backtracking engine) for `regex` would invalidate this
//! argument entirely and is a recorded prohibition (D-09, D-41).
//!
//! # Rules apply in declaration order; a redaction does not stop the sweep
//!
//! [`Guardrail`]'s `before_model`/`after_model` walk `self.rules` in the
//! order they were declared. `Fail` and `Finish` are terminal: the first
//! rule whose action is `Fail` or `Finish` and which matches stops the chain
//! immediately, and no later rule runs. `Redact` is not terminal: it
//! rewrites the matched text in place and the sweep continues to the next
//! rule, which therefore sees the already-redacted text (D-09's EDGE
//! ordering rule).
//!
//! # Prompt screens run over structured sections, never a flattened string
//!
//! `before_model` applies each `Prompt`/`Both` rule to every one of
//! [`super::PromptAssembly`]'s text-bearing parts individually — the system
//! prompt, the retrieved-context block, every Garrison history entry's
//! content, the user input, the accumulated output, and every pushed
//! [`super::PromptSection`]'s body — so a `Redact` lands in the section it
//! actually matched (T-26-30, D-02). `after_model` applies each
//! `Response`/`Both` rule to [`super::LlmResponseView::content`].

use std::sync::Arc;

use async_trait::async_trait;
use regex::{Regex, RegexBuilder};

use crate::application::services::paladin::error::PaladinError;
use crate::config::agent_runtime::{
    GuardrailConfig, GuardrailOnMatch as ConfigGuardrailOnMatch, GuardrailRuleConfig,
    GuardrailTarget as ConfigGuardrailTarget,
};
use paladin_ports::output::paladin_port::StopReason;

use super::{
    ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
    PromptAssembly,
};

// ── rule shape (D-09) ───────────────────────────────────────────────────

/// Which part of a model interaction a [`GuardrailRule`] screens (D-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardrailTarget {
    /// Screen the rendered-prompt parts in `before_model`.
    Prompt,
    /// Screen [`super::LlmResponseView::content`] in `after_model`.
    Response,
    /// Screen both.
    Both,
}

impl GuardrailTarget {
    fn screens_prompt(self) -> bool {
        matches!(self, GuardrailTarget::Prompt | GuardrailTarget::Both)
    }

    fn screens_response(self) -> bool {
        matches!(self, GuardrailTarget::Response | GuardrailTarget::Both)
    }
}

impl From<ConfigGuardrailTarget> for GuardrailTarget {
    fn from(value: ConfigGuardrailTarget) -> Self {
        match value {
            ConfigGuardrailTarget::Prompt => GuardrailTarget::Prompt,
            ConfigGuardrailTarget::Response => GuardrailTarget::Response,
            ConfigGuardrailTarget::Both => GuardrailTarget::Both,
        }
    }
}

/// How a [`GuardrailRule`] decides whether it matches (D-09, D-10).
///
/// `Regex` patterns are config-supplied and compile exactly once, at
/// [`Guardrail::new`]/[`Guardrail::from_rules`] construction time — never
/// per screen. `Predicate` closures are code-only: [`GuardrailRuleConfig`]
/// has no representation for them, so a `Predicate` matcher can only be
/// built through [`Guardrail::from_rules`].
#[derive(Clone)]
pub enum GuardrailMatcher {
    /// A regular-expression pattern, compiled at construction.
    Regex(String),
    /// A code-only predicate over the candidate text.
    Predicate(Arc<dyn Fn(&str) -> bool + Send + Sync>),
}

impl std::fmt::Debug for GuardrailMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuardrailMatcher::Regex(pattern) => f.debug_tuple("Regex").field(pattern).finish(),
            GuardrailMatcher::Predicate(_) => f.write_str("Predicate(<fn>)"),
        }
    }
}

/// What a [`GuardrailRule`] does when its matcher matches (D-09).
#[derive(Debug, Clone)]
pub enum GuardrailAction {
    /// Fail the run with [`PaladinError::GuardrailTripped`].
    Fail,
    /// Replace every match with the given replacement text.
    Redact(String),
    /// Finish the run immediately with this message as the output, under
    /// [`StopReason::Completed`] — deliberately not a new `StopReason`
    /// variant (D-09, a Deferred Idea free under `#[non_exhaustive]` but not
    /// an RT FR).
    Finish(String),
}

/// One named guardrail rule — D-09's exact locked shape.
#[derive(Debug, Clone)]
pub struct GuardrailRule {
    /// A human-readable name, used in [`PaladinError::GuardrailTripped`] and
    /// in logs.
    pub name: String,
    /// Which part of the interaction this rule screens.
    pub target: GuardrailTarget,
    /// How this rule decides whether it matches.
    pub matcher: GuardrailMatcher,
    /// What to do when it matches.
    pub on_match: GuardrailAction,
}

impl GuardrailRule {
    /// Build a `GuardrailRule` from its config-file form. Always produces a
    /// `Regex` matcher — the config-supplied form (D-10); `Predicate` has no
    /// config representation and is never constructed here.
    fn from_config(config: GuardrailRuleConfig) -> Self {
        let on_match = match config.on_match {
            ConfigGuardrailOnMatch::Fail => GuardrailAction::Fail,
            ConfigGuardrailOnMatch::Redact { replacement } => GuardrailAction::Redact(replacement),
            ConfigGuardrailOnMatch::Finish { message } => GuardrailAction::Finish(message),
        };
        Self {
            name: config.name,
            target: GuardrailTarget::from(config.target),
            matcher: GuardrailMatcher::Regex(config.pattern),
            on_match,
        }
    }
}

// ── construction: compile once, typed failure (D-09) ────────────────────

/// A typed failure from [`Guardrail::new`]/[`Guardrail::from_rules`] — an
/// unparseable or oversized pattern never reaches a run.
#[derive(Debug, thiserror::Error)]
pub enum GuardrailBuildError {
    /// A rule's `Regex` pattern failed to compile — either a syntax error or
    /// a compiled program exceeding the configured
    /// `pattern_size_limit_bytes` bound. The [`regex::Error`] source names
    /// the bound in its own `Display` when the cause is
    /// [`regex::Error::CompiledTooBig`].
    #[error("guardrail rule `{rule}`: invalid regex pattern: {source}")]
    InvalidPattern {
        /// The offending rule's name.
        rule: String,
        /// The underlying `regex` crate error.
        #[source]
        source: regex::Error,
    },
}

/// One rule plus its compiled `Regex` (if any), held so a `Regex` matcher
/// compiles exactly once, at construction (D-09). `compiled` is `None` for
/// a `Predicate` matcher, which has nothing to compile.
#[derive(Debug)]
struct CompiledRule {
    rule: GuardrailRule,
    compiled: Option<Regex>,
}

/// The outcome of applying one rule to one text field.
enum RuleOutcome {
    /// The run should fail with this error.
    Fail(PaladinError),
    /// The run should finish now with this message.
    Finish(String),
    /// The field was redacted in place; the sweep continues.
    Redacted,
}

/// Screens prompt sections and response content against a set of named
/// rules (D-09, RT-FR-07). See the module docs for the security argument,
/// the ordering rule, and the prompt-section screening rule.
#[derive(Debug)]
pub struct Guardrail {
    rules: Vec<CompiledRule>,
}

impl Guardrail {
    /// Build a `Guardrail` from a [`GuardrailConfig`] — every rule's `Regex`
    /// matcher compiles through
    /// `RegexBuilder::size_limit(config.pattern_size_limit_bytes)`. The
    /// default (empty) rule set makes an enabled `Guardrail` with no rules
    /// observably identical to no `Guardrail` at all (D-09, PRD RT-FR-07).
    ///
    /// # Errors
    ///
    /// Returns [`GuardrailBuildError::InvalidPattern`] naming the offending
    /// rule if any pattern fails to parse or exceeds the configured size
    /// bound.
    pub fn new(config: GuardrailConfig) -> Result<Self, GuardrailBuildError> {
        let size_limit = config.pattern_size_limit_bytes;
        let rules = config
            .rules
            .into_iter()
            .map(GuardrailRule::from_config)
            .map(|rule| Self::compile(rule, size_limit))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { rules })
    }

    /// Build a `Guardrail` from code-constructed rules — the path that
    /// supplies [`GuardrailMatcher::Predicate`] matchers, which have no
    /// [`GuardrailRuleConfig`] representation.
    ///
    /// # Errors
    ///
    /// Returns [`GuardrailBuildError::InvalidPattern`] naming the offending
    /// rule if any `Regex` matcher fails to parse or exceeds
    /// `pattern_size_limit_bytes`.
    pub fn from_rules(
        rules: Vec<GuardrailRule>,
        pattern_size_limit_bytes: usize,
    ) -> Result<Self, GuardrailBuildError> {
        let rules = rules
            .into_iter()
            .map(|rule| Self::compile(rule, pattern_size_limit_bytes))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { rules })
    }

    fn compile(
        rule: GuardrailRule,
        _size_limit: usize,
    ) -> Result<CompiledRule, GuardrailBuildError> {
        // TODO(GREEN): compile through `RegexBuilder::size_limit` with the
        // configured bound instead of this bare `Regex::new`, which relies
        // on the crate's generous internal default.
        let compiled = match &rule.matcher {
            GuardrailMatcher::Regex(pattern) => Some(Regex::new(pattern).map_err(|source| {
                GuardrailBuildError::InvalidPattern {
                    rule: rule.name.clone(),
                    source,
                }
            })?),
            GuardrailMatcher::Predicate(_) => None,
        };
        Ok(CompiledRule { rule, compiled })
    }

    /// The number of rules whose matcher compiled to a stored `Regex` —
    /// a structural, timing-free witness that `Regex` matchers are compiled
    /// once at construction and held on the struct, never recompiled per
    /// screen (`valid_rules_compile_at_construction`).
    #[cfg(test)]
    fn compiled_regex_count(&self) -> usize {
        self.rules.iter().filter(|r| r.compiled.is_some()).count()
    }

    fn matches(compiled: &CompiledRule, text: &str) -> bool {
        match &compiled.rule.matcher {
            GuardrailMatcher::Regex(_) => compiled
                .compiled
                .as_ref()
                .map(|regex| regex.is_match(text))
                .unwrap_or(false),
            GuardrailMatcher::Predicate(predicate) => predicate(text),
        }
    }

    /// Redact every match of `compiled`'s matcher in `text` with
    /// `replacement`. A `Predicate` matcher has no match span to redact — it
    /// can only report "this text matched" — so `Redact` under a
    /// `Predicate` matcher replaces the WHOLE screened field, a deliberate,
    /// documented difference from a `Regex` matcher's partial substitution.
    fn redact_in_place(compiled: &CompiledRule, text: &mut String, replacement: &str) {
        match &compiled.rule.matcher {
            GuardrailMatcher::Regex(_) => {
                if let Some(regex) = compiled.compiled.as_ref() {
                    *text = regex.replace_all(text, replacement).into_owned();
                }
            }
            GuardrailMatcher::Predicate(_) => {
                *text = replacement.to_string();
            }
        }
    }

    /// Apply one rule to one mutable text field. `side` is `"prompt"` or
    /// `"response"` — the side actually being screened, carried into
    /// [`PaladinError::GuardrailTripped::target`] rather than the rule's
    /// (possibly `Both`) configured target.
    ///
    /// Returns `None` if the rule did not match this field at all.
    fn apply_to_field(
        compiled: &CompiledRule,
        text: &mut String,
        side: &'static str,
    ) -> Option<RuleOutcome> {
        if !Self::matches(compiled, text) {
            return None;
        }
        match &compiled.rule.on_match {
            GuardrailAction::Fail => Some(RuleOutcome::Fail(PaladinError::GuardrailTripped {
                rule: compiled.rule.name.clone(),
                target: side.to_string(),
            })),
            // TODO(GREEN): also write `message` into `*text` -- the
            // service's `after_model` call site reads the FINAL output from
            // `resp.content`, not from `FinalResult::output`.
            GuardrailAction::Finish(message) => Some(RuleOutcome::Finish(message.clone())),
            GuardrailAction::Redact(replacement) => {
                Self::redact_in_place(compiled, text, replacement);
                Some(RuleOutcome::Redacted)
            }
        }
    }

    /// Apply one rule across every text-bearing part of a [`PromptAssembly`]
    /// (D-02, D-09): sweeps every part when the outcome is a redaction (so a
    /// later part can still be redacted after an earlier one matched), but
    /// returns immediately on the first part where a `Fail`/`Finish` rule
    /// matches.
    fn apply_to_prompt(
        compiled: &CompiledRule,
        assembly: &mut PromptAssembly,
    ) -> Option<RuleOutcome> {
        macro_rules! sweep_field {
            ($field:expr) => {
                if let Some(outcome) = Self::apply_to_field(compiled, $field, "prompt") {
                    match outcome {
                        RuleOutcome::Redacted => {}
                        terminal => return Some(terminal),
                    }
                }
            };
        }

        sweep_field!(&mut assembly.system);
        if let Some(context) = assembly.retrieved_context.as_mut() {
            sweep_field!(context);
        }
        for entry in assembly.history.iter_mut() {
            sweep_field!(&mut entry.content);
        }
        sweep_field!(&mut assembly.input);
        sweep_field!(&mut assembly.accumulated_output);
        for section in assembly.sections.iter_mut() {
            sweep_field!(&mut section.body);
        }

        None
    }
}

#[async_trait]
impl ExecutionMiddleware for Guardrail {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        for compiled in &self.rules {
            // TODO(GREEN): use `screens_prompt()` -- this exact-equality
            // check misses `GuardrailTarget::Both`.
            if compiled.rule.target != GuardrailTarget::Prompt {
                continue;
            }
            match Self::apply_to_prompt(compiled, &mut cx.assembly) {
                Some(RuleOutcome::Fail(err)) => return Ok(MiddlewareFlow::Fail(err)),
                Some(RuleOutcome::Finish(message)) => {
                    return Ok(MiddlewareFlow::Finish(FinalResult::new(
                        message,
                        StopReason::Completed,
                    )));
                }
                Some(RuleOutcome::Redacted) | None => {}
            }
        }
        Ok(MiddlewareFlow::Continue)
    }

    async fn after_model(
        &self,
        _cx: &mut ModelCallContext<'_>,
        resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        for compiled in &self.rules {
            if !compiled.rule.target.screens_response() {
                continue;
            }
            match Self::apply_to_field(compiled, &mut resp.content, "response") {
                Some(RuleOutcome::Fail(err)) => return Ok(MiddlewareFlow::Fail(err)),
                Some(RuleOutcome::Finish(message)) => {
                    return Ok(MiddlewareFlow::Finish(FinalResult::new(
                        message,
                        StopReason::Completed,
                    )));
                }
                Some(RuleOutcome::Redacted) | None => {}
            }
        }
        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "guardrail"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_llm::mock::MockLlmAdapter;
    use std::sync::Arc;
    use std::time::Duration;

    fn make_rule_config(name: &str, pattern: &str) -> GuardrailRuleConfig {
        GuardrailRuleConfig {
            name: name.to_string(),
            target: ConfigGuardrailTarget::Prompt,
            pattern: pattern.to_string(),
            on_match: ConfigGuardrailOnMatch::Redact {
                replacement: "[X]".to_string(),
            },
        }
    }

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "You are a helpful assistant".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, None)
    }

    fn make_service(llm: Arc<MockLlmAdapter>) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            None,
            None,
        )
    }

    fn make_context(paladin: &Paladin) -> ModelCallContext<'_> {
        let assembly = PromptAssembly::new("You are a helpful assistant", "hi", "", vec![], None);
        ModelCallContext::new(uuid::Uuid::new_v4(), paladin, assembly)
    }

    // ── Task 1: construction ─────────────────────────────────────────────

    /// Test 1: a `Guardrail` built from three regex rules constructs
    /// successfully, holds all three compiled patterns on the struct, and
    /// running its `before_model` many times never recompiles them.
    #[tokio::test]
    async fn valid_rules_compile_at_construction() {
        let config = GuardrailConfig {
            enabled: true,
            rules: vec![
                make_rule_config("r1", r"foo"),
                make_rule_config("r2", r"bar"),
                make_rule_config("r3", r"baz"),
            ],
            pattern_size_limit_bytes: 1 << 16,
        };
        let guardrail = Guardrail::new(config).expect("valid patterns must compile");
        assert_eq!(
            guardrail.compiled_regex_count(),
            3,
            "all three regex matchers compiled and held on the struct at construction"
        );

        let paladin = make_paladin(1);
        for _ in 0..1000 {
            let mut cx = make_context(&paladin);
            guardrail
                .before_model(&mut cx)
                .await
                .expect("screening must not fail on non-matching text");
            assert_eq!(
                guardrail.compiled_regex_count(),
                3,
                "the compiled set never changes across repeated screens"
            );
        }
    }

    /// Test 2: an unparseable pattern fails `Guardrail::new` with a typed
    /// error naming the rule, not at first use and not by panic.
    #[test]
    fn invalid_regex_is_a_typed_construction_error() {
        let config = GuardrailConfig {
            enabled: true,
            rules: vec![make_rule_config("bad-rule", "(unclosed")],
            pattern_size_limit_bytes: 1 << 16,
        };
        let err =
            Guardrail::new(config).expect_err("an unparseable pattern must fail construction");
        match err {
            GuardrailBuildError::InvalidPattern { rule, source } => {
                assert_eq!(rule, "bad-rule");
                assert!(matches!(source, regex::Error::Syntax(_)));
            }
        }
    }

    /// Test 3: a pattern whose compiled program exceeds
    /// `pattern_size_limit_bytes` fails construction with a typed error
    /// naming the rule and the bound — proving the bound is the configured
    /// number, not the crate's internal default.
    #[test]
    fn oversized_pattern_is_rejected_at_the_documented_bound() {
        let tiny_limit = 10;
        let config = GuardrailConfig {
            enabled: true,
            rules: vec![make_rule_config("too-big", "[a-z]{1,80}")],
            pattern_size_limit_bytes: tiny_limit,
        };
        let err = Guardrail::new(config)
            .expect_err("a pattern exceeding the configured bound must fail construction");
        match err {
            GuardrailBuildError::InvalidPattern { rule, source } => {
                assert_eq!(rule, "too-big");
                match source {
                    regex::Error::CompiledTooBig(limit) => {
                        assert_eq!(
                            limit, tiny_limit,
                            "the error must name the CONFIGURED bound, not the crate's internal default"
                        );
                    }
                    other => panic!("expected CompiledTooBig, got {other:?}"),
                }
            }
        }
    }

    /// Test 4: `PaladinError::GuardrailTripped { rule, target }` carries the
    /// rule name and the target as named fields; `Display` renders both;
    /// matching it requires the existing `_` arm (this crate sees
    /// `PaladinError` as `#[non_exhaustive]`).
    #[test]
    fn guardrail_tripped_is_structured() {
        let err = PaladinError::GuardrailTripped {
            rule: "no-secrets".to_string(),
            target: "prompt".to_string(),
        };
        match &err {
            PaladinError::GuardrailTripped { rule, target } => {
                assert_eq!(rule, "no-secrets");
                assert_eq!(target, "prompt");
            }
            _ => panic!("expected GuardrailTripped"),
        }
        let rendered = err.to_string();
        assert!(rendered.contains("no-secrets"));
        assert!(rendered.contains("prompt"));

        // Matching a non-exhaustive enum from outside its defining crate
        // requires a wildcard arm.
        match err {
            PaladinError::GuardrailTripped { .. } => {}
            _ => panic!("expected GuardrailTripped"),
        }
    }

    /// Test 5: a `GuardrailRule` with a `Predicate` matcher can be
    /// constructed in code (`Guardrail::from_rules`) and has no
    /// `GuardrailRuleConfig` representation — a `Guardrail` built from a
    /// `GuardrailConfig` only ever holds `Regex` matchers.
    #[test]
    fn predicate_matcher_is_code_only() {
        let config = GuardrailConfig {
            enabled: true,
            rules: vec![make_rule_config("r1", "foo"), make_rule_config("r2", "bar")],
            pattern_size_limit_bytes: 1 << 16,
        };
        let from_config = Guardrail::new(config).unwrap();
        assert_eq!(
            from_config.compiled_regex_count(),
            2,
            "every rule surviving a GuardrailConfig round-trip is a Regex matcher"
        );

        let rules = vec![
            GuardrailRule {
                name: "regex-rule".to_string(),
                target: GuardrailTarget::Prompt,
                matcher: GuardrailMatcher::Regex("x".to_string()),
                on_match: GuardrailAction::Fail,
            },
            GuardrailRule {
                name: "predicate-rule".to_string(),
                target: GuardrailTarget::Prompt,
                matcher: GuardrailMatcher::Predicate(Arc::new(|text| text.contains("keyword"))),
                on_match: GuardrailAction::Fail,
            },
        ];
        let from_code = Guardrail::from_rules(rules, 1 << 16).unwrap();
        assert_eq!(
            from_code.compiled_regex_count(),
            1,
            "only the Regex-matcher rule compiled a Regex; the Predicate rule has none"
        );
    }

    /// Test 6: `GuardrailConfig::default().rules` is empty and a `Guardrail`
    /// built from it screens nothing.
    #[tokio::test]
    async fn empty_rule_set_is_the_default() {
        assert!(GuardrailConfig::default().rules.is_empty());
        let guardrail = Guardrail::new(GuardrailConfig::default()).unwrap();

        let paladin = make_paladin(1);
        let mut cx = make_context(&paladin);
        let outcome = guardrail.before_model(&mut cx).await.unwrap();
        assert!(matches!(outcome, MiddlewareFlow::Continue));
    }

    // ── Task 2: prompt/response screens ──────────────────────────────────

    /// Test 1: a rule targeting `Prompt` with `Redact` and a pattern
    /// matching text in the `input` section leaves the `system` section
    /// untouched and replaces only the matched text in `input`.
    #[tokio::test]
    async fn prompt_screen_redacts_in_the_matching_section() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let rule = GuardrailRule {
            name: "ssn".to_string(),
            target: GuardrailTarget::Prompt,
            matcher: GuardrailMatcher::Regex(r"\d{3}-\d{2}-\d{4}".to_string()),
            on_match: GuardrailAction::Redact("[REDACTED]".to_string()),
        };
        let guardrail = Arc::new(Guardrail::from_rules(vec![rule], 1 << 16).unwrap());
        let service = make_service(llm.clone()).with_middleware(guardrail);
        let paladin = make_paladin(1);

        service
            .execute(&paladin, "my ssn is 123-45-6789")
            .await
            .unwrap();

        let prompt = llm.last_prompt().unwrap();
        assert!(
            prompt.contains("You are a helpful assistant"),
            "the untouched system section must survive byte-identical: {prompt}"
        );
        assert!(
            !prompt.contains("123-45-6789"),
            "the matched SSN must be redacted: {prompt}"
        );
        assert!(
            prompt.contains("[REDACTED]"),
            "the redaction must land where the match occurred: {prompt}"
        );
    }

    /// Test 2: a rule targeting `Response` with `Redact` rewrites
    /// `LlmResponseView::content` and the accumulated output carries the
    /// redacted text.
    #[tokio::test]
    async fn response_screen_redacts_response_content() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("call me at 555-123-4567"));
        let rule = GuardrailRule {
            name: "phone".to_string(),
            target: GuardrailTarget::Response,
            matcher: GuardrailMatcher::Regex(r"\d{3}-\d{3}-\d{4}".to_string()),
            on_match: GuardrailAction::Redact("[REDACTED]".to_string()),
        };
        let guardrail = Arc::new(Guardrail::from_rules(vec![rule], 1 << 16).unwrap());
        let service = make_service(llm).with_middleware(guardrail);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert!(!result.output.contains("555-123-4567"));
        assert!(result.output.contains("[REDACTED]"));
    }

    /// Test 3: a `Fail` rule matching the prompt causes the run to fail with
    /// `PaladinError::GuardrailTripped { rule, target }` naming that rule,
    /// and the `LlmPort` call count is 0 for that iteration.
    #[tokio::test]
    async fn fail_action_returns_guardrail_tripped() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let rule = GuardrailRule {
            name: "no-secrets".to_string(),
            target: GuardrailTarget::Prompt,
            matcher: GuardrailMatcher::Regex(r"top-secret".to_string()),
            on_match: GuardrailAction::Fail,
        };
        let guardrail = Arc::new(Guardrail::from_rules(vec![rule], 1 << 16).unwrap());
        let service = make_service(llm.clone()).with_middleware(guardrail);
        let paladin = make_paladin(1);

        let result = service
            .execute(&paladin, "reveal the top-secret plan")
            .await;

        match result {
            Err(PaladinError::GuardrailTripped { rule, target }) => {
                assert_eq!(rule, "no-secrets");
                assert_eq!(target, "prompt");
            }
            other => panic!("expected GuardrailTripped, got {other:?}"),
        }
        assert_eq!(llm.call_count(), 0, "the model must never be called");
    }

    /// Test 4: a `Finish("blocked")` rule matching the response finishes the
    /// run with output `blocked` and `StopReason::Completed`.
    #[tokio::test]
    async fn finish_action_finishes_with_the_message_and_completed() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("trigger word here"));
        let rule = GuardrailRule {
            name: "stopper".to_string(),
            target: GuardrailTarget::Response,
            matcher: GuardrailMatcher::Regex(r"trigger word".to_string()),
            on_match: GuardrailAction::Finish("blocked".to_string()),
        };
        let guardrail = Arc::new(Guardrail::from_rules(vec![rule], 1 << 16).unwrap());
        let service = make_service(llm).with_middleware(guardrail);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(result.output, "blocked");
        assert_eq!(result.stop_reason, StopReason::Completed);
    }

    /// Test 5: a rule with `target: Both` fires on both sides within one
    /// iteration.
    #[tokio::test]
    async fn both_target_screens_prompt_and_response() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("classified: SECRETVAL"));
        let rule = GuardrailRule {
            name: "both".to_string(),
            target: GuardrailTarget::Both,
            matcher: GuardrailMatcher::Regex(r"SECRETVAL".to_string()),
            on_match: GuardrailAction::Redact("[X]".to_string()),
        };
        let guardrail = Arc::new(Guardrail::from_rules(vec![rule], 1 << 16).unwrap());
        let service = make_service(llm.clone()).with_middleware(guardrail);
        let paladin = make_paladin(1);

        let result = service
            .execute(&paladin, "the value is SECRETVAL")
            .await
            .unwrap();

        let prompt = llm.last_prompt().unwrap();
        assert!(
            !prompt.contains("SECRETVAL"),
            "prompt side must redact: {prompt}"
        );
        assert!(
            !result.output.contains("SECRETVAL"),
            "response side must redact: {}",
            result.output
        );
    }

    /// Test 6: with a `Redact` rule followed by a `Fail` rule both
    /// matching, the redaction is applied and then the `Fail` fires; with a
    /// `Finish` rule followed by a `Fail` rule, the `Finish` wins and the
    /// `Fail` never runs.
    #[tokio::test]
    async fn rules_apply_in_declaration_order_and_first_terminal_action_wins() {
        // Redact "foo" -> "bar", then Fail on "bar". The Fail rule only
        // matches if it runs AFTER the redaction -- proving Redact does not
        // stop the sweep and the later rule sees the redacted text.
        let llm = Arc::new(MockLlmAdapter::new().with_response("foo"));
        let redact_then_fail = vec![
            GuardrailRule {
                name: "redact-foo".to_string(),
                target: GuardrailTarget::Response,
                matcher: GuardrailMatcher::Regex("foo".to_string()),
                on_match: GuardrailAction::Redact("bar".to_string()),
            },
            GuardrailRule {
                name: "fail-bar".to_string(),
                target: GuardrailTarget::Response,
                matcher: GuardrailMatcher::Regex("bar".to_string()),
                on_match: GuardrailAction::Fail,
            },
        ];
        let guardrail = Arc::new(Guardrail::from_rules(redact_then_fail, 1 << 16).unwrap());
        let service = make_service(llm).with_middleware(guardrail);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await;
        match result {
            Err(PaladinError::GuardrailTripped { rule, .. }) => assert_eq!(rule, "fail-bar"),
            other => panic!("expected the second rule to trip on the redacted text, got {other:?}"),
        }

        // Finish("stopped") then Fail on the same text: Finish wins, Fail
        // never runs.
        let llm2 = Arc::new(MockLlmAdapter::new().with_response("trigger"));
        let finish_then_fail = vec![
            GuardrailRule {
                name: "finish-first".to_string(),
                target: GuardrailTarget::Response,
                matcher: GuardrailMatcher::Regex("trigger".to_string()),
                on_match: GuardrailAction::Finish("stopped".to_string()),
            },
            GuardrailRule {
                name: "fail-second".to_string(),
                target: GuardrailTarget::Response,
                matcher: GuardrailMatcher::Regex("trigger".to_string()),
                on_match: GuardrailAction::Fail,
            },
        ];
        let guardrail2 = Arc::new(Guardrail::from_rules(finish_then_fail, 1 << 16).unwrap());
        let service2 = make_service(llm2).with_middleware(guardrail2);

        let result2 = service2.execute(&paladin, "hi").await.unwrap();
        assert_eq!(result2.output, "stopped");
        assert_eq!(result2.stop_reason, StopReason::Completed);
    }

    /// Test 7: with rules installed but nothing matching, the rendered
    /// prompt, the port call count and the `PaladinResult` are identical to
    /// a run with no `Guardrail`.
    #[tokio::test]
    async fn no_match_is_a_pure_pass_through() {
        let llm_plain = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let plain_result = make_service(llm_plain.clone())
            .execute(&make_paladin(1), "hello")
            .await
            .unwrap();

        let llm_guarded = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let rule = GuardrailRule {
            name: "no-match".to_string(),
            target: GuardrailTarget::Both,
            matcher: GuardrailMatcher::Regex("this-never-appears".to_string()),
            on_match: GuardrailAction::Redact("[X]".to_string()),
        };
        let guardrail = Arc::new(Guardrail::from_rules(vec![rule], 1 << 16).unwrap());
        let guarded_result = make_service(llm_guarded.clone())
            .with_middleware(guardrail)
            .execute(&make_paladin(1), "hello")
            .await
            .unwrap();

        assert_eq!(llm_plain.last_prompt(), llm_guarded.last_prompt());
        assert_eq!(llm_plain.call_count(), llm_guarded.call_count());
        assert_eq!(plain_result.output, guarded_result.output);
        assert_eq!(plain_result.stop_reason, guarded_result.stop_reason);
    }

    /// Test 8: a `Predicate` matcher produces the same three actions as an
    /// equivalent `Regex` matcher.
    #[tokio::test]
    async fn predicate_rule_screens_like_a_regex_rule() {
        let llm_regex = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let regex_rule = GuardrailRule {
            name: "regex-fail".to_string(),
            target: GuardrailTarget::Prompt,
            matcher: GuardrailMatcher::Regex("keyword".to_string()),
            on_match: GuardrailAction::Fail,
        };
        let regex_guardrail = Arc::new(Guardrail::from_rules(vec![regex_rule], 1 << 16).unwrap());
        let regex_result = make_service(llm_regex.clone())
            .with_middleware(regex_guardrail)
            .execute(&make_paladin(1), "has the keyword in it")
            .await;

        let llm_predicate = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let predicate_rule = GuardrailRule {
            name: "predicate-fail".to_string(),
            target: GuardrailTarget::Prompt,
            matcher: GuardrailMatcher::Predicate(Arc::new(|text| text.contains("keyword"))),
            on_match: GuardrailAction::Fail,
        };
        let predicate_guardrail =
            Arc::new(Guardrail::from_rules(vec![predicate_rule], 1 << 16).unwrap());
        let predicate_result = make_service(llm_predicate.clone())
            .with_middleware(predicate_guardrail)
            .execute(&make_paladin(1), "has the keyword in it")
            .await;

        assert!(matches!(
            regex_result,
            Err(PaladinError::GuardrailTripped { .. })
        ));
        assert!(matches!(
            predicate_result,
            Err(PaladinError::GuardrailTripped { .. })
        ));
        assert_eq!(llm_regex.call_count(), 0);
        assert_eq!(llm_predicate.call_count(), 0);
    }
}
