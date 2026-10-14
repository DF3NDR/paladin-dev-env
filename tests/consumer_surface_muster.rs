//! Phase 49 consumer-surface muster (ADOPT-04, handoff D-4): every path, signature, struct shape
//! and trait a downstream library consumer takes from Paladin still resolves.
//!
//! The consumer (the web3sec audit workspace) depends on the `paladin` facade and on the
//! `paladin_ports` and `paladin_memory` crates by path. Nothing in Paladin executes that surface,
//! so a change that breaks it would otherwise be found only when the consumer bumps. This target
//! is the check that finds it first.
//!
//! The rule: **compile time only.** Every pin lives in a function that nothing calls, and the one
//! `#[test]` has a body made of comments, so `cargo test` reports a passing target exactly when
//! every `use`, fn-pointer coercion, struct literal and trait implementation below type-checked.
//! No pin is ever executed: nothing here spawns a process, reads the environment, opens a socket
//! or builds an HTTP client, and every credential-shaped argument is an empty string.
//!
//! Strictness policy: a struct the consumer constructs is pinned by a full literal without `..`,
//! so an additive field turns this target red on purpose. Where the consumer insulates itself
//! with `..Default::default()`, that form is pinned as well. A trait the consumer implements is
//! pinned by a stub that provides only the required methods and is coerced to `Arc<dyn Trait>`.
//!
//! Baseline: the current Paladin tree. This file is not expected to compile against the `v0.10.1`
//! tag, which predates several of the shapes pinned here.
//!
//! Red-muster rule: an intended break updates the pin, its MIGRATION.md section 9.2 row and the
//! section 9.8 library-consumer table in one commit. An unintended break is a library defect; the
//! library is fixed, and is never shimmed, re-exported or deprecated to turn this target green.
//!
//! Deliberately not pinned: the consumer-owned private auth-failure helper in the MCP protocol
//! module (it is private in Paladin and owned by the consumer), and the items removed since
//! `v0.10.1`, which MIGRATION.md lists. Surface the consumer does not use is not frozen here.
//!
//! ```bash
//! cargo test --test consumer_surface_muster
//! ```
// The fn-pointer pins are spelled out in full on purpose, so `type_complexity` is allowed; the
// pins live in never-called functions, so `dead_code` is allowed.
#![allow(dead_code, clippy::type_complexity)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

// ---- facade `paladin::` (features llm-anthropic + llm-deepseek) ----
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionError};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin::infrastructure::adapters::arsenal::mcp_protocol::MCPClient;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::{
    ArsenalPort, BattalionResult, LlmPort, PaladinError, PaladinResult, Phalanx, StopReason,
};
use paladin::{
    AnthropicAdapter, AnthropicConfig, Commissary, CommissaryError, CommissaryPlan, Consignment,
    ConsignmentItem, DeepSeekAdapter, DeepSeekConfig, MockLlmAdapter, MultiStepMockLlmPort,
};

// ---- direct path dependencies ----
use paladin_memory::token_counter::HeuristicTokenCounter;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
    TokenUsage,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::token_counter_port::TokenCounterPort;

/// The facade prelude and the ports crate name ONE `LlmError`, `LlmRequest` and `LlmResponse`.
fn prelude_and_ports_agree(
    e: paladin::prelude::LlmError,
    q: paladin::prelude::LlmRequest,
    r: paladin::prelude::LlmResponse,
) -> (LlmError, LlmRequest, LlmResponse) {
    (e, q, r)
}

/// The facade prelude and the core container modules name ONE `Armament` and `BattalionConfig`.
fn prelude_names_agree(
    a: paladin::prelude::Armament,
    c: paladin::prelude::BattalionConfig,
) -> (Armament, BattalionConfig) {
    (a, c)
}

/// B-3: `TokenUsage` has six fields and is built here with no `..`.
fn usage_literal() -> TokenUsage {
    TokenUsage {
        prompt_tokens: 1,
        completion_tokens: 2,
        total_tokens: 3,
        cache_read_tokens: None,
        cache_write_tokens: None,
        reasoning_tokens: None,
    }
}

/// B-1: the `Commissary` constructor signatures, pinned as fn pointers and by a real call.
fn commissary_signatures() {
    let _new: fn(
        String,
        ProviderCapabilities,
        Arc<dyn TokenCounterPort>,
        CommissaryPlan,
    ) -> Result<Commissary, CommissaryError> = Commissary::new;
    let _from_port: fn(
        &dyn LlmPort,
        Arc<dyn TokenCounterPort>,
        CommissaryPlan,
    ) -> Result<Commissary, CommissaryError> = Commissary::from_port;
    let _tokens: fn(&Commissary) -> u32 = Commissary::allotted_tokens;
    // A `&str` literal provider: narrowing the `impl Into<String>` bound fails to compile here.
    let counter: Arc<dyn TokenCounterPort> = Arc::new(HeuristicTokenCounter);
    let plan = CommissaryPlan {
        reserved_completion_tokens: 1,
        model_hint: "m".to_string(),
        ..Default::default()
    };
    let _ = Commissary::new(
        "literal-str",
        ProviderCapabilities::default(),
        counter,
        plan,
    );
}

/// D-2: the `TokenUsage` constructors, builders and the `Add`, `AddAssign` and `Sum` impls.
fn usage_signatures() {
    let _new: fn(u32, u32) -> TokenUsage = TokenUsage::new;
    let _cache_read: fn(TokenUsage, u32) -> TokenUsage = TokenUsage::with_cache_read;
    let _cache_write: fn(TokenUsage, u32) -> TokenUsage = TokenUsage::with_cache_write;
    let _reasoning: fn(TokenUsage, u32) -> TokenUsage = TokenUsage::with_reasoning;
    let _result_new: fn(String, TokenUsage, u64, u32, StopReason) -> PaladinResult =
        PaladinResult::new;
    let _sum: TokenUsage = vec![TokenUsage::default()].into_iter().sum();
    let mut acc = TokenUsage::default();
    acc += TokenUsage::new(1, 2);
    let _total = acc + usage_literal();
    let _insulated = TokenUsage {
        prompt_tokens: 1,
        ..Default::default()
    };
}

/// B-2, DR-5, DR-6: `PaladinResult` and `LlmResponse` built by full literals with no `..`.
fn result_literals() -> (PaladinResult, LlmResponse) {
    let result = PaladinResult {
        output: String::new(),
        usage: usage_literal(),
        cost: None,
        execution_time_ms: 0,
        loop_count: 1,
        stop_reason: StopReason::Completed,
        plan: None,
        handoff_history: Vec::new(),
        served_by: None,
    };
    let response = LlmResponse {
        id: Uuid::new_v4(),
        request_id: Uuid::new_v4(),
        model: String::new(),
        content: String::new(),
        finish_reason: FinishReason::Stop,
        usage: usage_literal(),
        cost: None,
        created_at: Utc::now(),
        metadata: HashMap::new(),
        function_call: None,
    };
    (result, response)
}

/// The consumer-preferred insulated form: named fields plus `..Default::default()`.
fn insulated_result_literal() -> PaladinResult {
    PaladinResult {
        output: String::new(),
        usage: usage_literal(),
        execution_time_ms: 0,
        loop_count: 1,
        stop_reason: StopReason::Completed,
        ..Default::default()
    }
}

/// DR-3: `Paladin` is `Entity<PaladinData>`; the payload has thirteen fields, all named here.
fn paladin_literal() -> Paladin {
    let data = PaladinData {
        system_prompt: String::new(),
        name: String::new(),
        user_name: String::new(),
        model: String::new(),
        temperature: 0.0,
        max_loops: MaxLoops::Fixed(1),
        stop_words: Vec::new(),
        status: PaladinStatus::Idle,
        vision_enabled: false,
        autonomous_planning: false,
        autonomous_prompts: false,
        agent_description: String::new(),
        dynamic_temperature: false,
    };
    Entity::new(data, Some("n".to_string()))
}

/// DR-4: a prompt item's payload is read through `.entity.data`.
fn prompt_signatures() {
    let item = PromptItem::new(PromptType::User(UserPrompt {
        query: String::new(),
        context: None,
    }));
    let _req = item.map(|p| {
        let _read: &PromptType = &p.entity.data.prompt_type;
        LlmRequest::new("m", p)
    });
}

// Minimal implementers: ONLY the required methods, so a new required method (E0046), a changed
// signature (E0053) or a loss of object safety fails to compile.

struct MusterLlm;

#[async_trait]
impl LlmPort for MusterLlm {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(result_literals().1)
    }
    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        Err(LlmError::NetworkError(String::new()))
    }
    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Err(LlmError::Timeout(String::new()))
    }
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(Vec::new())
    }
    fn get_provider_name(&self) -> &'static str {
        "muster"
    }
    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }
}

struct MusterPaladinPort;

#[async_trait]
impl PaladinPort for MusterPaladinPort {
    async fn execute(
        &self,
        paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        let _model: &str = &paladin.data.model;
        Ok(result_literals().0)
    }
    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        Err(PaladinError::ExecutionError(String::new()))
    }
    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

struct MusterArsenal;

#[async_trait]
impl ArsenalPort for MusterArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        Vec::new()
    }
    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        Ok(ArmamentResult::success(
            call.call_id,
            serde_json::Value::Null,
            0,
        ))
    }
    fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
        Ok(())
    }
}

/// Services, resilience, the MCP client and the provider adapters. NEVER CALLED: running it
/// would spawn a process, read the environment and build HTTP clients.
async fn service_and_adapter_signatures(args: &[String]) {
    let llm: Arc<dyn LlmPort> = Arc::new(MusterLlm);
    let breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let arsenal: Option<Arc<dyn ArsenalPort>> = Some(Arc::new(MusterArsenal));
    let service = PaladinExecutionService::new(llm, breaker.clone(), None, arsenal);
    let _r: Result<PaladinResult, PaladinError> = service.execute(&paladin_literal(), "x").await;
    let _cb: Result<u32, PaladinError> = breaker.call_async(async { Ok(1u32) }).await;

    let port: Arc<dyn PaladinPort> = Arc::new(MusterPaladinPort);
    let phalanx_service = PhalanxExecutionService::new(port);
    let phalanx: Result<Phalanx, BattalionError> =
        Phalanx::new(vec![paladin_literal()], BattalionConfig::new("n"));
    if let Ok(phalanx) = phalanx {
        let _b: Result<BattalionResult, BattalionError> =
            phalanx_service.execute(&phalanx, "x").await;
    }

    let _c: Result<MCPClient, ArsenalError> = MCPClient::connect_stdio("cmd", args).await;

    let _adapters = (
        AnthropicAdapter::new(AnthropicConfig::new(
            String::new(),
            String::new(),
            String::new(),
            1,
        )),
        DeepSeekConfig::from_env().map(DeepSeekAdapter::new),
        MockLlmAdapter::new(),
        MultiStepMockLlmPort::new(Vec::new()),
    );
    let _items = (
        Consignment::new(),
        ConsignmentItem {
            label: String::new(),
            body: String::new(),
            priority: 0,
        },
    );
}

#[test]
fn the_consumer_surface_still_resolves() {
    // Reaching this point means every `use`, fn-pointer, literal and trait-impl pin above
    // type-checked. Nothing is executed here, on purpose.
}
