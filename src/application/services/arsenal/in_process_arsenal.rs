//! `InProcessArsenal` — an [`ArsenalPort`] over registered [`Armament`]
//! definitions plus in-process async closures (Doc 05 RT-04, D-22).
//!
//! Every shipped `ArsenalPort` in the tree routes to an MCP client
//! (`arsenal_execution_service.rs`'s `ArsenalExecutionService::invoke`,
//! `:184-230` at the time this module was written) — a real subprocess or
//! HTTP round-trip to a server that discovers and serves tools. This type is
//! that method's in-process counterpart: a registered async Rust closure
//! stands in for the MCP round-trip, with the same `list_armaments` /
//! `invoke` / `validate_call` contract. It exists because the tree has no
//! way to expose a built-in capability (starting with [`VaultTools`](
//! super::vault_tools::VaultTools)'s `vault_get`/`vault_put`) as an
//! `Armament` without either running a real MCP server for it or adding this
//! type.
//!
//! # Argument validation reuses the shared shape checker
//!
//! [`InProcessArsenal::validate_call`] validates a call's arguments against
//! the registered [`Armament::parameters`] JSON Schema using
//! [`shape_check`](paladin_core::platform::container::structured::shape_check)
//! (plan 26-12) rather than a second hand-rolled validator — one JSON-Schema
//! subset implementation in the workspace, not two.
//!
//! # No unwind guard around a handler (house rule)
//!
//! [`InProcessArsenal::invoke`] does **not** wrap the registered handler in
//! `std::panic::catch_unwind`. A handler that panics propagates the panic
//! exactly like any other Rust code, unwinding the calling task -- this is a
//! deliberate absence, not an oversight: this workspace's convention is that
//! library code returns `Result` rather than panicking (see
//! `.github/instructions/rust.instructions.md`), so a panicking handler is
//! a bug in the handler to fix, not a condition this port should convert
//! into a value. A handler that legitimately fails should return `Err`,
//! which [`invoke`](Self::invoke) turns into a failed [`ArmamentResult`]
//! (never a panic) — see `a_closure_error_becomes_an_armament_result_error_not_a_panic`
//! below.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use paladin_core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};
use paladin_core::platform::container::structured::shape_check;
use paladin_ports::output::arsenal_port::ArsenalPort;

/// A boxed, cloneable async handler: takes the call's arguments and returns
/// either the tool's JSON output or a plain-text error message.
///
/// The error type is a plain `String`, not [`ArsenalError`] — a handler is
/// typically implemented against a domain-specific error (e.g.
/// [`VaultError`](paladin_ports::output::vault_port::VaultError)) that has
/// nothing to do with the Arsenal/MCP transport-shaped variants
/// [`ArsenalError`] enumerates; requiring every handler author to map into
/// `ArsenalError` would force an arbitrary and often-wrong choice of
/// variant. [`InProcessArsenal::invoke`] converts a handler's `Err(message)`
/// into a failed [`ArmamentResult`] carrying that message text.
type ToolHandler = Arc<
    dyn Fn(HashMap<String, Value>) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
        + Send
        + Sync,
>;

/// An [`ArsenalPort`] over a registry of [`Armament`] definitions paired
/// with in-process async closures — the in-process counterpart of the
/// tree's MCP-routed `ArsenalExecutionService` (module documentation above).
///
/// # Examples
///
/// ```
/// use paladin::application::services::arsenal::in_process_arsenal::InProcessArsenal;
/// use paladin_core::platform::container::arsenal::{Armament, ArmamentCall};
/// use paladin_ports::output::arsenal_port::ArsenalPort;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// # async fn example() {
/// let arsenal = InProcessArsenal::new().with_tool(
///     Armament {
///         name: "double".to_string(),
///         description: "Doubles a number".to_string(),
///         parameters: json!({"type": "object", "required": ["n"], "properties": {"n": {"type": "number"}}}),
///         required_params: vec![],
///     },
///     |args: HashMap<String, serde_json::Value>| async move {
///         let n = args.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
///         Ok(json!({"result": n * 2.0}))
///     },
/// );
///
/// let mut args = HashMap::new();
/// args.insert("n".to_string(), json!(21));
/// let result = arsenal.invoke(ArmamentCall::new("double", args)).await.unwrap();
/// assert_eq!(result.output, Some(json!({"result": 42.0})));
/// # }
/// ```
#[derive(Clone, Default)]
pub struct InProcessArsenal {
    tools: HashMap<String, (Armament, ToolHandler)>,
}

impl InProcessArsenal {
    /// Creates an empty `InProcessArsenal` with no registered tools.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Registers `armament`, served by `handler`, and returns `self` for
    /// chaining (matches the workspace's `with_*` builder idiom).
    ///
    /// A second call with the same `armament.name` replaces the earlier
    /// registration.
    pub fn with_tool<F, Fut>(mut self, armament: Armament, handler: F) -> Self
    where
        F: Fn(HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, String>> + Send + 'static,
    {
        let boxed: ToolHandler = Arc::new(move |args| Box::pin(handler(args)));
        self.tools.insert(armament.name.clone(), (armament, boxed));
        self
    }
}

#[async_trait]
impl ArsenalPort for InProcessArsenal {
    async fn list_armaments(&self) -> Vec<Armament> {
        self.tools
            .values()
            .map(|(armament, _)| armament.clone())
            .collect()
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        self.validate_call(&call)?;

        let (_, handler) = self
            .tools
            .get(&call.tool_name)
            .ok_or_else(|| ArsenalError::ToolNotFound(call.tool_name.clone()))?;

        let start = tokio::time::Instant::now();
        // No `catch_unwind` here -- see the module documentation's "house
        // rule" section. A handler panic propagates unchanged.
        let outcome = handler(call.arguments.clone()).await;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        match outcome {
            Ok(value) => Ok(ArmamentResult::success(
                call.call_id,
                value,
                execution_time_ms,
            )),
            Err(message) => Ok(ArmamentResult::failure(
                call.call_id,
                message,
                execution_time_ms,
            )),
        }
    }

    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
        let (armament, _) = self
            .tools
            .get(&call.tool_name)
            .ok_or_else(|| ArsenalError::ToolNotFound(call.tool_name.clone()))?;

        let arguments_value = Value::Object(
            call.arguments
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        );

        shape_check(&arguments_value, &armament.parameters)
            .map_err(|e| ArsenalError::InvalidArguments(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn echo_armament() -> Armament {
        Armament {
            name: "echo".to_string(),
            description: "Echoes its input back".to_string(),
            parameters: json!({
                "type": "object",
                "required": ["message"],
                "properties": {"message": {"type": "string"}}
            }),
            required_params: vec![],
        }
    }

    #[tokio::test]
    async fn in_process_arsenal_lists_and_invokes_a_registered_closure() {
        let arsenal = InProcessArsenal::new().with_tool(echo_armament(), |args| async move {
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(json!({"echoed": message}))
        });

        let listed = arsenal.list_armaments().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "echo");

        let mut args = HashMap::new();
        args.insert("message".to_string(), json!("hello"));
        let result = arsenal
            .invoke(ArmamentCall::new("echo", args))
            .await
            .expect("invoke should succeed");

        assert!(result.success);
        assert_eq!(result.output, Some(json!({"echoed": "hello"})));
    }

    #[tokio::test]
    async fn in_process_arsenal_rejects_an_unknown_tool() {
        let arsenal = InProcessArsenal::new();

        let result = arsenal
            .invoke(ArmamentCall::new("nonexistent", HashMap::new()))
            .await;

        match result {
            Err(ArsenalError::ToolNotFound(name)) => assert_eq!(name, "nonexistent"),
            other => panic!("expected ToolNotFound, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn validate_call_checks_arguments_against_the_declared_schema() {
        let arsenal = InProcessArsenal::new().with_tool(echo_armament(), |_args| async move {
            Ok(json!("should never run"))
        });

        // Missing the required `message` property.
        let call = ArmamentCall::new("echo", HashMap::new());
        let validation = arsenal.validate_call(&call);
        assert!(matches!(validation, Err(ArsenalError::InvalidArguments(_))));

        // Invoking with the same invalid call must fail at validation,
        // before the closure (which would otherwise succeed) ever runs.
        let invoked = arsenal.invoke(call).await;
        assert!(matches!(invoked, Err(ArsenalError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn a_closure_error_becomes_an_armament_result_error_not_a_panic() {
        let arsenal = InProcessArsenal::new().with_tool(echo_armament(), |_args| async move {
            Err("the handler deliberately failed".to_string())
        });

        let mut args = HashMap::new();
        args.insert("message".to_string(), json!("hi"));
        let result = arsenal
            .invoke(ArmamentCall::new("echo", args))
            .await
            .expect("a handler Err must surface as a failed ArmamentResult, not a propagated Err");

        assert!(!result.success);
        assert_eq!(
            result.error.as_deref(),
            Some("the handler deliberately failed")
        );

        // Documented, not tested: a handler that panics is NOT caught by
        // `invoke` (no `catch_unwind` -- see the module documentation's
        // "house rule" section). Testing an actual panic here would abort
        // the test harness under the default panic strategy, which is
        // exactly the reason this behavior is documented rather than
        // exercised directly.
    }
}
