//! `VaultTools` — the `vault_get`/`vault_put` Armaments a model uses to read
//! and write its own granted Vault subtree (Doc 05 RT-04, D-20, D-22).
//!
//! [`VaultTools::new`] builds an [`InProcessArsenal`] carrying exactly two
//! Armaments, both closing over the SAME [`ConfinedVault`] handle: every
//! call -- however the model phrases the `namespace` argument -- is checked
//! against that handle's grant before it ever reaches the underlying store
//! (`ConfinedVault`'s own contract; see `crates/paladin-ports/src/output/
//! vault_confined.rs`).
//!
//! # Absolute addressing (D-20)
//!
//! `namespace` is an **absolute** array of path segments, not relative to
//! the grant -- the same design [`ConfinedVault`] itself uses, and for the
//! same reason: PRD 05 §3.4's attack test needs a scripted tool call to be
//! able to literally *name* a sibling namespace like `["user", "bob"]` from
//! inside a `["user", "alice"]` grant, and an agent needs to be able to
//! inspect what it was granted, not have it permanently hidden behind a
//! relative addressing scheme.
//!
//! # A model-supplied namespace goes through the type's own invariants first
//!
//! Both handlers construct a [`Namespace`] from the raw argument through
//! [`Namespace::new`] before ever touching [`ConfinedVault`]. A malformed
//! shape -- a `..` segment, an empty segment, an over-long segment, or an
//! empty segment list -- is rejected by that constructor's own invariants;
//! [`ConfinedVault`]'s confinement check is never even reached for those
//! inputs. This is intentional defence in depth (D-19, D-41): namespace
//! *validity* and namespace *authorization* are two different gates, and a
//! malformed namespace never gets far enough to ask the second question.
//!
//! # Every `VaultError` becomes a tool-result error, not a panic
//!
//! Both handlers return `Err(message)` on any [`VaultError`], which
//! [`InProcessArsenal::invoke`] turns into a failed [`ArmamentResult`] fed
//! back to the model on its next iteration -- never a propagated `Err` that
//! would abort the run. A [`VaultError::NamespaceDenied`] message names both
//! the requested and granted namespaces so the model has enough information
//! to correct itself. Plan 26-19's `ToolResultFormatter::format_error` will
//! own the redact-then-bound formatting for every tool error in the tree;
//! until it lands, this module's error text is kept minimal and names only
//! namespaces (never a raw store/backend message) to stay safely inside
//! that eventual contract.

use std::collections::HashMap;

use serde_json::{Value, json};

use paladin_core::platform::container::arsenal::Armament;
use paladin_ports::output::vault_confined::ConfinedVault;
use paladin_ports::output::vault_port::{Namespace, VaultError, VaultPort};

use super::in_process_arsenal::InProcessArsenal;

/// The literal JSON `vault_get` returns when the requested key does not
/// exist -- written once here (not re-typed at each call site) and quoted
/// verbatim in [`vault_get_armament`]'s description, so the model knows
/// exactly what absence looks like without inferring it from an error.
const VAULT_GET_NOT_FOUND_JSON: &str = r#"{"found":false}"#;

/// Builds the two built-in Vault Armaments confined to one run's grant
/// (D-22). Not a type with state of its own -- [`VaultTools::new`] returns
/// an [`InProcessArsenal`] directly, since that is the only public surface
/// this module needs.
pub struct VaultTools;

impl VaultTools {
    /// Builds an [`InProcessArsenal`] carrying `vault_get` and `vault_put`,
    /// both confined to `confined`'s granted namespace.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::application::services::arsenal::vault_tools::VaultTools;
    /// use paladin_core::platform::container::vault::Namespace;
    /// use paladin_ports::output::vault_confined::ConfinedVault;
    /// use paladin_ports::output::vault_port::VaultPort;
    /// use std::sync::Arc;
    ///
    /// # async fn example(store: Arc<dyn VaultPort>) {
    /// let granted = Namespace::parse("user/alice").unwrap();
    /// let confined = ConfinedVault::new(store, granted);
    /// let arsenal = VaultTools::new(confined);
    /// # }
    /// ```
    // `VaultTools` is a stateless factory (module doc above): `new` is a
    // deliberate factory function returning the `InProcessArsenal` it
    // builds, not a constructor of `Self` -- there is no `VaultTools`
    // value to construct, so `clippy::new_ret_no_self` does not apply here.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(confined: ConfinedVault) -> InProcessArsenal {
        let for_get = confined.clone();
        let for_put = confined;

        InProcessArsenal::new()
            .with_tool(vault_get_armament(), move |args| {
                let confined = for_get.clone();
                async move { vault_get_handler(&confined, args).await }
            })
            .with_tool(vault_put_armament(), move |args| {
                let confined = for_put.clone();
                async move { vault_put_handler(&confined, args).await }
            })
    }
}

fn vault_get_armament() -> Armament {
    Armament {
        name: "vault_get".to_string(),
        description: format!(
            "Reads a value from this agent's long-term Vault memory. Arguments: \
             `namespace` (an array of path segments, e.g. [\"user\",\"alice\"]) and \
             `key` (a string). The namespace is ABSOLUTE, not relative to this \
             agent's grant. If the key does not exist, returns \
             `{VAULT_GET_NOT_FOUND_JSON}` -- absence is not an error, the agent \
             should treat it as \"there is nothing stored there yet\". A namespace \
             outside this agent's granted subtree is denied."
        ),
        parameters: json!({
            "type": "object",
            "required": ["namespace", "key"],
            "properties": {
                "namespace": {"type": "array", "items": {"type": "string"}},
                "key": {"type": "string"}
            }
        }),
        required_params: vec![],
    }
}

fn vault_put_armament() -> Armament {
    Armament {
        name: "vault_put".to_string(),
        description: "Writes a value into this agent's long-term Vault memory. \
            Arguments: `namespace` (an array of path segments, e.g. \
            [\"user\",\"alice\"]), `key` (a string) and `value` (any JSON value). \
            The namespace is ABSOLUTE, not relative to this agent's grant. Returns \
            `{\"ok\":true}` on success. A namespace outside this agent's granted \
            subtree is denied."
            .to_string(),
        parameters: json!({
            "type": "object",
            "required": ["namespace", "key", "value"],
            "properties": {
                "namespace": {"type": "array", "items": {"type": "string"}},
                "key": {"type": "string"}
            }
        }),
        required_params: vec![],
    }
}

/// Parses the `namespace` argument into a [`Namespace`], going through
/// [`Namespace::new`]'s own invariants -- a `..`/empty/over-long segment, or
/// an empty segment list, is rejected here, before any [`ConfinedVault`]
/// method is ever called (module documentation above).
fn parse_namespace_arg(args: &HashMap<String, Value>) -> Result<Namespace, String> {
    let raw = args
        .get("namespace")
        .ok_or_else(|| "missing required argument `namespace`".to_string())?;
    let array = raw
        .as_array()
        .ok_or_else(|| "argument `namespace` must be an array of strings".to_string())?;
    let segments = array
        .iter()
        .map(|segment| {
            segment
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "argument `namespace` must be an array of strings".to_string())
        })
        .collect::<Result<Vec<String>, String>>()?;
    Namespace::new(segments).map_err(|e| e.to_string())
}

/// Parses the `key` argument as a string.
fn parse_key_arg(args: &HashMap<String, Value>) -> Result<String, String> {
    args.get("key")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "missing or invalid required argument `key` (expected a string)".to_string())
}

/// Renders a [`VaultError`] as tool-result text -- minimal, and naming only
/// namespaces, never a raw backend message (module documentation above).
fn render_vault_error(error: VaultError) -> String {
    match error {
        VaultError::NamespaceDenied { requested, granted } => format!(
            "namespace {requested} is outside this agent's granted namespace {granted}; \
             this agent may only access {granted} and its descendants"
        ),
        other => format!("vault operation failed: {other}"),
    }
}

async fn vault_get_handler(
    confined: &ConfinedVault,
    args: HashMap<String, Value>,
) -> Result<Value, String> {
    let namespace = parse_namespace_arg(&args)?;
    let key = parse_key_arg(&args)?;

    match confined.get(&namespace, &key).await {
        Ok(Some(record)) => Ok(json!({"found": true, "value": record.value()})),
        Ok(None) => serde_json::from_str(VAULT_GET_NOT_FOUND_JSON)
            .map_err(|e| format!("internal error building not-found result: {e}")),
        Err(e) => Err(render_vault_error(e)),
    }
}

async fn vault_put_handler(
    confined: &ConfinedVault,
    args: HashMap<String, Value>,
) -> Result<Value, String> {
    let namespace = parse_namespace_arg(&args)?;
    let key = parse_key_arg(&args)?;
    let value = args
        .get("value")
        .cloned()
        .ok_or_else(|| "missing required argument `value`".to_string())?;

    match confined.put(&namespace, &key, value).await {
        Ok(()) => Ok(json!({"ok": true})),
        Err(e) => Err(render_vault_error(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::arsenal::ArmamentCall;
    use paladin_ports::output::arsenal_port::ArsenalPort;
    use paladin_ports::output::vault_port::{Page, ScoredVaultRecord, VaultRecord};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A `VaultPort` double that counts every call it receives, so a test
    /// can assert "the store was never touched" as a real, direct
    /// assertion rather than an inference from a denied result.
    #[derive(Default)]
    struct CountingVault {
        calls: AtomicUsize,
        records: tokio::sync::Mutex<HashMap<(String, String), VaultRecord>>,
    }

    impl CountingVault {
        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl VaultPort for CountingVault {
        async fn put(&self, ns: &Namespace, key: &str, value: Value) -> Result<(), VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let record = VaultRecord::new(ns.clone(), key, value)?;
            self.records
                .lock()
                .await
                .insert((ns.to_string(), key.to_string()), record);
            Ok(())
        }

        async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self
                .records
                .lock()
                .await
                .get(&(ns.to_string(), key.to_string()))
                .cloned())
        }

        async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self
                .records
                .lock()
                .await
                .remove(&(ns.to_string(), key.to_string()))
                .is_some())
        }

        async fn list(
            &self,
            _ns: &Namespace,
            _prefix: Option<&str>,
            _page: Page,
        ) -> Result<Vec<VaultRecord>, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        async fn search(
            &self,
            _ns: &Namespace,
            _query: &str,
            _top_k: u32,
        ) -> Result<Vec<ScoredVaultRecord>, VaultError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(VaultError::Unsupported {
                operation: "search",
            })
        }
    }

    fn alice() -> Namespace {
        Namespace::parse("user/alice").unwrap()
    }

    fn args(namespace: Value, key: &str) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("namespace".to_string(), namespace);
        map.insert("key".to_string(), json!(key));
        map
    }

    #[tokio::test]
    async fn vault_get_returns_the_value_for_a_granted_namespace() {
        let store: Arc<CountingVault> = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
        confined
            .put(&alice(), "color", json!("blue"))
            .await
            .unwrap();

        let arsenal = VaultTools::new(confined);
        let call = ArmamentCall::new("vault_get", args(json!(["user", "alice"]), "color"));
        let result = arsenal.invoke(call).await.expect("invoke should succeed");

        assert!(result.success);
        assert_eq!(result.output, Some(json!({"found": true, "value": "blue"})));
    }

    #[tokio::test]
    async fn vault_get_returns_a_documented_not_found_result() {
        let store: Arc<CountingVault> = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(store as Arc<dyn VaultPort>, alice());

        let arsenal = VaultTools::new(confined);
        let call = ArmamentCall::new("vault_get", args(json!(["user", "alice"]), "missing"));
        let result = arsenal.invoke(call).await.expect("invoke should succeed");

        assert!(
            result.success,
            "a missing key is a successful result, not an error"
        );
        assert_eq!(result.output, Some(json!({"found": false})));
    }

    #[tokio::test]
    async fn vault_put_stores_within_the_grant() {
        let store: Arc<CountingVault> = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());

        let arsenal = VaultTools::new(confined.clone());
        let mut put_args = args(json!(["user", "alice"]), "color");
        put_args.insert("value".to_string(), json!("green"));
        let result = arsenal
            .invoke(ArmamentCall::new("vault_put", put_args))
            .await
            .expect("invoke should succeed");

        assert!(result.success);
        assert_eq!(result.output, Some(json!({"ok": true})));

        let stored = confined.get(&alice(), "color").await.unwrap().unwrap();
        assert_eq!(stored.value(), &json!("green"));
    }

    #[tokio::test]
    async fn tools_declare_json_schemas_for_their_arguments() {
        let store: Arc<CountingVault> = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(store as Arc<dyn VaultPort>, alice());
        let arsenal = VaultTools::new(confined);

        let listed = arsenal.list_armaments().await;
        assert_eq!(listed.len(), 2);
        for armament in &listed {
            let schema = armament
                .parameters
                .as_object()
                .expect("schema must be an object");
            assert!(
                schema.get("properties").is_some(),
                "{} must declare properties",
                armament.name
            );
            let required = schema
                .get("required")
                .and_then(Value::as_array)
                .expect("schema must declare required fields");
            assert!(
                required.iter().any(|v| v == "namespace"),
                "{} must require namespace",
                armament.name
            );
        }

        // Missing `namespace` fails validate_call.
        let mut no_namespace = HashMap::new();
        no_namespace.insert("key".to_string(), json!("x"));
        let validation = arsenal.validate_call(&ArmamentCall::new("vault_get", no_namespace));
        assert!(validation.is_err());
    }

    /// Test 5: five malformed namespace shapes, asserted individually, each
    /// producing a typed error while leaving the store untouched.
    #[tokio::test]
    async fn a_malformed_namespace_argument_is_a_typed_tool_error() {
        // 1. Non-array namespace -- caught at validate_call (schema level),
        //    before the handler (and therefore the store) is ever reached.
        {
            let store: Arc<CountingVault> = Arc::new(CountingVault::default());
            let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
            let arsenal = VaultTools::new(confined);
            let call = ArmamentCall::new("vault_get", args(json!("not-an-array"), "k"));
            let result = arsenal.invoke(call).await;
            assert!(
                result.is_err(),
                "a non-array namespace must fail validate_call"
            );
            assert_eq!(store.call_count(), 0);
        }

        // 2. Empty array -- passes shape_check (an array of zero strings is
        //    still an array), rejected by Namespace::new's own invariant.
        {
            let store: Arc<CountingVault> = Arc::new(CountingVault::default());
            let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
            let arsenal = VaultTools::new(confined);
            let call = ArmamentCall::new("vault_get", args(json!([]), "k"));
            let result = arsenal.invoke(call).await.expect("invoke itself succeeds");
            assert!(
                !result.success,
                "an empty namespace must be a failed ArmamentResult"
            );
            assert_eq!(store.call_count(), 0);
        }

        // 3. A `..` segment.
        {
            let store: Arc<CountingVault> = Arc::new(CountingVault::default());
            let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
            let arsenal = VaultTools::new(confined);
            let call = ArmamentCall::new("vault_get", args(json!(["user", "alice", ".."]), "k"));
            let result = arsenal.invoke(call).await.expect("invoke itself succeeds");
            assert!(
                !result.success,
                "a `..` segment must be a failed ArmamentResult"
            );
            assert_eq!(store.call_count(), 0);
        }

        // 4. An empty-string segment.
        {
            let store: Arc<CountingVault> = Arc::new(CountingVault::default());
            let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
            let arsenal = VaultTools::new(confined);
            let call = ArmamentCall::new("vault_get", args(json!(["user", ""]), "k"));
            let result = arsenal.invoke(call).await.expect("invoke itself succeeds");
            assert!(
                !result.success,
                "an empty segment must be a failed ArmamentResult"
            );
            assert_eq!(store.call_count(), 0);
        }

        // 5. A 65-character segment (over the 64-char bound).
        {
            let store: Arc<CountingVault> = Arc::new(CountingVault::default());
            let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
            let arsenal = VaultTools::new(confined);
            let over_long = "a".repeat(65);
            let call = ArmamentCall::new("vault_get", args(json!(["user", over_long]), "k"));
            let result = arsenal.invoke(call).await.expect("invoke itself succeeds");
            assert!(
                !result.success,
                "an over-long segment must be a failed ArmamentResult"
            );
            assert_eq!(store.call_count(), 0);
        }
    }

    #[tokio::test]
    async fn vault_tools_are_not_listed_without_a_grant() {
        // `VaultTools::new` requires a `ConfinedVault` -- constructed here
        // with a grant of `["user","alice"]` to prove the OTHER half of
        // D-21: a call OUTSIDE the grant is denied and never lists as
        // reachable regardless (the "without a grant at all" half of this
        // truth is proven at the `PaladinExecutionService` level, in
        // `paladin_execution_service.rs`'s
        // `vault_tools_are_not_listed_without_a_grant` test, where a run
        // with no `RunScope`/service-default grant gets no `VaultTools`
        // arsenal wired in at all).
        let store: Arc<CountingVault> = Arc::new(CountingVault::default());
        let confined = ConfinedVault::new(store.clone() as Arc<dyn VaultPort>, alice());
        let arsenal = VaultTools::new(confined);

        let mut put_args = args(json!(["user", "bob"]), "x");
        put_args.insert("value".to_string(), json!(1));
        let result = arsenal
            .invoke(ArmamentCall::new("vault_put", put_args))
            .await
            .expect("invoke itself succeeds");

        assert!(!result.success, "a sibling namespace must be denied");
        assert_eq!(
            store.call_count(),
            0,
            "the store must never be touched on denial"
        );
    }
}
