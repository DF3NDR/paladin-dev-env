//! Node-cache key composition (Doc 04 FT-FR-20, D-28; plan 25-13).
//!
//! `key = H(graph_fingerprint, node_id, input_component,
//! paladin_config_fingerprint)`, composed over a canonical
//! **length-prefixed** byte stream in exactly the discipline
//! [`WarGraph::fingerprint`](crate::engine::graph::WarGraph::fingerprint)
//! adopted at its `v1` -> `v2` bump (Phase 22.1 CR-01): every
//! variable-length field goes through `graph::push_field`, so no unescaped
//! delimiter ever separates two author-controlled strings and no byte
//! sequence can be reinterpreted as a different split (T-25-62).
//!
//! # What each component does
//!
//! - **graph fingerprint** (never omitted -- RESEARCH.md Pitfall 6, T-25-61):
//!   a graph edit changes the fingerprint and therefore every key, so a
//!   stored entry is invalidated naturally, and two graphs can never share
//!   a key by construction rather than by a runtime check. The fingerprint
//!   already covers each node's own `cache` policy (`v5`, D-11), so a
//!   change to a node's `CacheKeySpec` moves its key too.
//! - **node id**: one entry per node, and -- together with the fingerprint
//!   coming first in the rendered key -- what lets
//!   `NodeCachePort::invalidate(prefix)` target one graph
//!   ([`graph_prefix`]) or one node ([`node_prefix`]).
//! - **input component**: the rendered `InputMapping` string for a
//!   `NodeSpec::Paladin` node; for a `NodeSpec::Function` node under
//!   `CacheKeySpec::Default` a canonical hash of the FULL Battlefield
//!   snapshot (conservative and correct, since a `StateNode`'s read set is
//!   not statically knowable), narrowed to the listed fields under
//!   `CacheKeySpec::Fields`. For a Paladin node, `Fields` ADDS those fields
//!   on top of the rendered input -- the key is never narrower than the
//!   input the Paladin actually saw.
//! - **muster context**: the task payload and `task_key`, included whenever
//!   the dispatch is a mustered worker task, so a wide fan-out over one
//!   worker template keys per task rather than collapsing to one entry.
//! - **Paladin configuration fingerprint** (Paladin nodes only): model,
//!   system prompt, temperature, max loops and stop words -- a prompt or
//!   model change invalidates the entry naturally with no explicit
//!   eviction (FT-FR-20).
//!
//! # Rendered form
//!
//! `{NODE_CACHE_KEY_VERSION}:{graph fingerprint}:{node id length}:{node id}:{digest}`
//! -- the node id is written with its byte length ahead of it so a node
//! prefix is exact: `node_prefix` for node `a` can never match node `a:b`'s
//! key, whatever characters a `NodeId` contains. The digest is the blake3
//! hex of the whole canonical stream, so even the human-readable prefix
//! components are covered by the hash.

use paladin_core::platform::container::aegis::CacheKeySpec;
use paladin_core::platform::container::battlefield::{Battlefield, FieldName};
use paladin_core::platform::container::directive::MusterContext;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{GraphFingerprint, NodeId};
use paladin_ports::output::node_cache_port::NodeCacheKey;

use crate::engine::graph::push_field;

/// Version tag every composed key starts with. Bump it together with any
/// change to this module's byte layout, so an entry written under the old
/// composition is a miss (never a mis-addressed hit) after the change.
pub const NODE_CACHE_KEY_VERSION: &str = "k1";

/// The prefix shared by every key composed for `fingerprint`'s graph --
/// hand it to `NodeCachePort::invalidate` to drop one graph's entries.
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::cache_key::graph_prefix;
/// use paladin_core::platform::container::waypoint::GraphFingerprint;
///
/// let fingerprint = GraphFingerprint::from_canonical_bytes(b"example");
/// let prefix = graph_prefix(&fingerprint);
/// assert!(prefix.starts_with("k1:v6:"));
/// assert!(prefix.ends_with(':'));
/// ```
pub fn graph_prefix(fingerprint: &GraphFingerprint) -> String {
    format!("{NODE_CACHE_KEY_VERSION}:{}:", fingerprint.as_str())
}

/// The prefix shared by every key composed for `node_id` within
/// `fingerprint`'s graph -- hand it to `NodeCachePort::invalidate` to drop
/// one node's entries. Exact by construction: the node id's byte length
/// precedes it, so `node_prefix` for `a` never matches a key for `a:b`.
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::cache_key::{graph_prefix, node_prefix};
/// use paladin_core::platform::container::waypoint::{GraphFingerprint, NodeId};
///
/// let fingerprint = GraphFingerprint::from_canonical_bytes(b"example");
/// let prefix = node_prefix(&fingerprint, &NodeId::new("summarise"));
/// assert!(prefix.starts_with(&graph_prefix(&fingerprint)));
/// assert!(prefix.ends_with(":9:summarise:"));
/// ```
pub fn node_prefix(fingerprint: &GraphFingerprint, node_id: &NodeId) -> String {
    format!(
        "{}{}:{}:",
        graph_prefix(fingerprint),
        node_id.as_str().len(),
        node_id.as_str()
    )
}

/// The input half of a key: what the node actually read this dispatch.
pub(crate) enum InputComponent<'a> {
    /// A `NodeSpec::Paladin` node's rendered `InputMapping` string.
    Rendered(&'a str),
    /// A `NodeSpec::Function` node's superstep snapshot -- hashed in full
    /// under `CacheKeySpec::Default`.
    Snapshot(&'a Battlefield),
}

/// Everything a key is composed from (D-28).
pub(crate) struct CacheKeyInputs<'a> {
    /// The running graph's fingerprint -- never omitted.
    pub graph_fingerprint: &'a GraphFingerprint,
    /// The dispatched node.
    pub node_id: &'a NodeId,
    /// The node's input this dispatch.
    pub input: InputComponent<'a>,
    /// The superstep snapshot, read for `CacheKeySpec::Fields` (both node
    /// kinds) -- the same `Battlefield` an `InputComponent::Snapshot` names.
    pub snapshot: &'a Battlefield,
    /// The policy's key specification.
    pub key_spec: &'a CacheKeySpec,
    /// The muster task context, `Some` for a mustered worker dispatch.
    pub muster: Option<&'a MusterContext>,
    /// The Paladin a `NodeSpec::Paladin` node wraps; `None` for a
    /// `Function` node.
    pub paladin: Option<&'a Paladin>,
}

/// Compose the [`NodeCacheKey`] for `inputs`.
///
/// Deterministic: the same inputs compose the same key on every call
/// (schema fields walk in declaration order, JSON values render through
/// `serde_json`'s canonical map ordering, and every variable-length field
/// is length-prefixed). See the module rustdoc for what each component
/// contributes.
pub(crate) fn compose(inputs: &CacheKeyInputs<'_>) -> NodeCacheKey {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"node_cache:");
    push_field(&mut buf, NODE_CACHE_KEY_VERSION.as_bytes());
    push_field(&mut buf, inputs.graph_fingerprint.as_str().as_bytes());
    push_field(&mut buf, inputs.node_id.as_str().as_bytes());

    buf.extend_from_slice(b";input:");
    match &inputs.input {
        InputComponent::Rendered(rendered) => {
            buf.push(1); // "rendered input" tag
            push_field(&mut buf, rendered.as_bytes());
        }
        InputComponent::Snapshot(snapshot) => {
            buf.push(2); // "snapshot" tag
            // `CacheKeySpec::Fields` narrows a Function node's input to the
            // listed fields; `Default` (and, fail-safe, any future variant)
            // hashes the full snapshot.
            match inputs.key_spec {
                CacheKeySpec::Fields(_) => {}
                _ => push_full_snapshot(&mut buf, snapshot),
            }
        }
    }
    // `Fields` contributes the named fields' values for BOTH node kinds:
    // it narrows a Function node's snapshot (above), and adds to a Paladin
    // node's rendered input (never narrows below it).
    buf.extend_from_slice(b";fields:");
    match inputs.key_spec {
        CacheKeySpec::Fields(fields) => {
            buf.push(1); // "has field list" tag
            buf.extend_from_slice(&(fields.len() as u64).to_le_bytes());
            for field in fields {
                push_snapshot_field(&mut buf, inputs.snapshot, field);
            }
        }
        _ => buf.push(0), // "no field list" tag
    }

    buf.extend_from_slice(b";muster:");
    match inputs.muster {
        Some(muster) => {
            buf.push(1); // "mustered" tag
            push_field(&mut buf, muster.task_key.as_bytes());
            let payload = serde_json::to_string(&muster.payload).unwrap_or_default();
            push_field(&mut buf, payload.as_bytes());
        }
        None => buf.push(0), // "not mustered" tag
    }

    buf.extend_from_slice(b";paladin:");
    match inputs.paladin {
        Some(paladin) => {
            buf.push(1); // "paladin node" tag
            push_paladin_config(&mut buf, paladin);
        }
        None => buf.push(0), // "function node" tag
    }

    let digest = blake3::hash(&buf);
    NodeCacheKey::new(format!(
        "{}{}",
        node_prefix(inputs.graph_fingerprint, inputs.node_id),
        digest.to_hex()
    ))
}

/// Write every declared field of `snapshot`, in schema declaration order
/// (never `HashMap` order), each as its name plus a present/absent tag and
/// its canonical JSON value.
fn push_full_snapshot(buf: &mut Vec<u8>, snapshot: &Battlefield) {
    let fields = &snapshot.schema().fields;
    buf.extend_from_slice(&(fields.len() as u64).to_le_bytes());
    for spec in fields {
        push_snapshot_field(buf, snapshot, &spec.name);
    }
}

/// Write one field of `snapshot`: its name, a present/absent tag, and (when
/// present) its canonical JSON value.
fn push_snapshot_field(buf: &mut Vec<u8>, snapshot: &Battlefield, field: &FieldName) {
    push_field(buf, field.as_str().as_bytes());
    match snapshot.get_raw(field) {
        Some(value) => {
            buf.push(1); // "present" tag
            let json = serde_json::to_string(value).unwrap_or_default();
            push_field(buf, json.as_bytes());
        }
        None => buf.push(0), // "absent" tag
    }
}

/// Write the Paladin configuration fingerprint's inputs (FT-FR-20, D-28):
/// model, system prompt, temperature (its exact `f32` bits, never a
/// rounded rendering), max loops (serde-canonical, so `Fixed(3)` and
/// `Auto { .. }` never collide) and stop words (count-prefixed).
fn push_paladin_config(buf: &mut Vec<u8>, paladin: &Paladin) {
    let data = &paladin.node;
    push_field(buf, data.model.as_bytes());
    push_field(buf, data.system_prompt.as_bytes());
    buf.extend_from_slice(&data.temperature.to_le_bytes());
    let max_loops = serde_json::to_string(&data.max_loops).unwrap_or_default();
    push_field(buf, max_loops.as_bytes());
    buf.extend_from_slice(&(data.stop_words.len() as u64).to_le_bytes());
    for word in &data.stop_words {
        push_field(buf, word.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::{EngineLimits, NodeSpec, WarGraph};
    use crate::engine::input_mapping::InputMapping;
    use crate::engine::test_support::CountingFunctionNode;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldSpec, StateDelta,
    };
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData};

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
    }

    fn schema() -> BattlefieldSchema {
        BattlefieldSchema::new(vec![
            FieldSpec::new(field("a"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("b"), DispatchRule::LastWrite, None, false),
        ])
    }

    fn snapshot(a: &str, b: &str) -> Battlefield {
        let mut initial = StateDelta::new();
        initial.set(field("a"), a).unwrap();
        initial.set(field("b"), b).unwrap();
        Battlefield::initialize(schema(), &initial).unwrap()
    }

    fn make_paladin(data: PaladinData) -> Paladin {
        paladin_core::base::entity::node::Node::new(data, Some("p".to_string()))
    }

    fn graph_with_function(node: &str) -> WarGraph {
        let mut graph = WarGraph::new(schema(), EngineLimits::default());
        let id = NodeId::new(node);
        graph.add_node(
            id.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_, _| StateDelta::new())),
        );
        graph.add_entry(id);
        graph
    }

    fn function_key(
        fingerprint: &GraphFingerprint,
        node: &str,
        snap: &Battlefield,
        key_spec: &CacheKeySpec,
        muster: Option<&MusterContext>,
    ) -> NodeCacheKey {
        compose(&CacheKeyInputs {
            graph_fingerprint: fingerprint,
            node_id: &NodeId::new(node),
            input: InputComponent::Snapshot(snap),
            snapshot: snap,
            key_spec,
            muster,
            paladin: None,
        })
    }

    fn paladin_key(
        fingerprint: &GraphFingerprint,
        node: &str,
        rendered: &str,
        snap: &Battlefield,
        paladin: &Paladin,
    ) -> NodeCacheKey {
        compose(&CacheKeyInputs {
            graph_fingerprint: fingerprint,
            node_id: &NodeId::new(node),
            input: InputComponent::Rendered(rendered),
            snapshot: snap,
            key_spec: &CacheKeySpec::Default,
            muster: None,
            paladin: Some(paladin),
        })
    }

    /// Test 1: two graphs with the same node id and the same input but
    /// different fingerprints produce different keys (RESEARCH Pitfall 6).
    #[test]
    fn key_includes_the_graph_fingerprint() {
        let one = graph_with_function("n");
        let mut two = graph_with_function("n");
        two.add_node(
            NodeId::new("extra"),
            NodeSpec::Function(CountingFunctionNode::new(|_, _| StateDelta::new())),
        );
        two.add_entry(NodeId::new("extra"));
        assert_ne!(one.fingerprint(), two.fingerprint());

        let snap = snapshot("x", "y");
        let k1 = function_key(&one.fingerprint(), "n", &snap, &CacheKeySpec::Default, None);
        let k2 = function_key(&two.fingerprint(), "n", &snap, &CacheKeySpec::Default, None);
        assert_ne!(k1, k2);
        assert!(k1.starts_with(&graph_prefix(&one.fingerprint())));
        assert!(k2.starts_with(&graph_prefix(&two.fingerprint())));
        assert!(!k2.starts_with(&graph_prefix(&one.fingerprint())));
    }

    /// Test 2: the same node with a changed system prompt produces a
    /// different key; likewise model, temperature, max loops and stop
    /// words.
    #[test]
    fn key_changes_when_the_system_prompt_changes() {
        let graph = graph_with_function("p");
        let fp = graph.fingerprint();
        let snap = snapshot("x", "y");
        let base = PaladinData {
            system_prompt: "be brief".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(3),
            stop_words: vec!["STOP".to_string()],
            ..Default::default()
        };
        let reference = paladin_key(&fp, "p", "input", &snap, &make_paladin(base.clone()));

        let variants: Vec<(&str, PaladinData)> = vec![
            (
                "system prompt",
                PaladinData {
                    system_prompt: "be verbose".to_string(),
                    ..base.clone()
                },
            ),
            (
                "model",
                PaladinData {
                    model: "gpt-4o".to_string(),
                    ..base.clone()
                },
            ),
            (
                "temperature",
                PaladinData {
                    temperature: 0.2,
                    ..base.clone()
                },
            ),
            (
                "max loops",
                PaladinData {
                    max_loops: MaxLoops::Fixed(4),
                    ..base.clone()
                },
            ),
            (
                "stop words",
                PaladinData {
                    stop_words: vec!["STOP".to_string(), "END".to_string()],
                    ..base.clone()
                },
            ),
        ];
        for (what, data) in variants {
            let changed = paladin_key(&fp, "p", "input", &snap, &make_paladin(data));
            assert_ne!(
                reference, changed,
                "changing the {what} must change the key"
            );
        }
        // A field the fingerprint does NOT cover (display name) leaves the
        // key alone, so a cosmetic rename never evicts an entry.
        let renamed = paladin_key(
            &fp,
            "p",
            "input",
            &snap,
            &make_paladin(PaladinData {
                name: "renamed".to_string(),
                ..base
            }),
        );
        assert_eq!(reference, renamed);
    }

    /// Test 3: the same graph, node and input produce byte-identical keys
    /// on repeated composition (and a different rendered input does not).
    #[test]
    fn key_is_stable_across_runs_for_identical_inputs() {
        let graph = graph_with_function("p");
        let fp = graph.fingerprint();
        let snap = snapshot("x", "y");
        let paladin = make_paladin(PaladinData::default());
        let a = paladin_key(&fp, "p", "hello", &snap, &paladin);
        let b = paladin_key(&fp, "p", "hello", &snap, &paladin);
        assert_eq!(a, b);
        assert_eq!(a.as_str(), b.as_str());
        let c = paladin_key(&fp, "p", "hello!", &snap, &paladin);
        assert_ne!(a, c);
        // The Function-node composition is just as stable.
        let f1 = function_key(&fp, "p", &snap, &CacheKeySpec::Default, None);
        let f2 = function_key(&fp, "p", &snapshot("x", "y"), &CacheKeySpec::Default, None);
        assert_eq!(f1, f2);
    }

    /// Test 4: a Function node under `CacheKeySpec::Default` produces a
    /// key that changes when ANY Battlefield field changes.
    #[test]
    fn function_node_key_defaults_to_the_full_snapshot() {
        let graph = graph_with_function("f");
        let fp = graph.fingerprint();
        let base = function_key(&fp, "f", &snapshot("x", "y"), &CacheKeySpec::Default, None);
        let a_changed = function_key(&fp, "f", &snapshot("X", "y"), &CacheKeySpec::Default, None);
        let b_changed = function_key(&fp, "f", &snapshot("x", "Y"), &CacheKeySpec::Default, None);
        assert_ne!(base, a_changed);
        assert_ne!(base, b_changed);
        assert_ne!(a_changed, b_changed);
    }

    /// Test 5: under `CacheKeySpec::Fields(["a"])`, changing field `b`
    /// does not change the key and changing field `a` does.
    #[test]
    fn cache_key_spec_fields_narrows_the_key() {
        let graph = graph_with_function("f");
        let fp = graph.fingerprint();
        let spec = CacheKeySpec::Fields(vec![field("a")]);
        let base = function_key(&fp, "f", &snapshot("x", "y"), &spec, None);
        let b_changed = function_key(&fp, "f", &snapshot("x", "Y"), &spec, None);
        let a_changed = function_key(&fp, "f", &snapshot("X", "y"), &spec, None);
        assert_eq!(base, b_changed, "a field outside the list is not keyed on");
        assert_ne!(base, a_changed, "a listed field is keyed on");
    }

    /// Test 6: two mustered tasks with different payloads produce
    /// different keys, and a mustered dispatch never shares a key with an
    /// ordinary one.
    #[test]
    fn muster_payload_is_included_when_present() {
        let graph = graph_with_function("worker");
        let fp = graph.fingerprint();
        let snap = snapshot("x", "y");
        let task_a = MusterContext {
            payload: serde_json::json!({"item": "a"}),
            task_key: "a".to_string(),
        };
        let task_b = MusterContext {
            payload: serde_json::json!({"item": "b"}),
            task_key: "b".to_string(),
        };
        let plain = function_key(&fp, "worker", &snap, &CacheKeySpec::Default, None);
        let ka = function_key(&fp, "worker", &snap, &CacheKeySpec::Default, Some(&task_a));
        let kb = function_key(&fp, "worker", &snap, &CacheKeySpec::Default, Some(&task_b));
        assert_ne!(ka, kb);
        assert_ne!(plain, ka);
        assert_ne!(plain, kb);
        // All three still share the node prefix, so `invalidate(node_prefix)`
        // drops every task's entry at once.
        let prefix = node_prefix(&fp, &NodeId::new("worker"));
        assert!(plain.starts_with(&prefix) && ka.starts_with(&prefix) && kb.starts_with(&prefix));
    }

    /// T-25-62: the node prefix is exact -- a node whose id extends another's
    /// with a `:` can never be swept by the shorter node's prefix.
    #[test]
    fn node_prefix_is_exact_under_delimiter_bearing_ids() {
        let graph = graph_with_function("a");
        let fp = graph.fingerprint();
        let snap = snapshot("x", "y");
        let key_for_a_colon_b = function_key(&fp, "a:b", &snap, &CacheKeySpec::Default, None);
        let prefix_for_a = node_prefix(&fp, &NodeId::new("a"));
        assert!(!key_for_a_colon_b.starts_with(&prefix_for_a));
        assert!(key_for_a_colon_b.starts_with(&node_prefix(&fp, &NodeId::new("a:b"))));
    }

    /// The Paladin/Function distinction and the rendered/snapshot
    /// distinction are both tagged, so a Function node reading a snapshot
    /// whose single field happens to equal a Paladin's rendered input can
    /// never collide with it.
    #[test]
    fn paladin_and_function_compositions_never_collide() {
        let graph = graph_with_function("n");
        let fp = graph.fingerprint();
        let snap = snapshot("x", "y");
        let mapping = InputMapping::new("{a}");
        let rendered = mapping.render(&snap, None, None).unwrap();
        let p = paladin_key(
            &fp,
            "n",
            &rendered,
            &snap,
            &make_paladin(PaladinData::default()),
        );
        let f = function_key(
            &fp,
            "n",
            &snap,
            &CacheKeySpec::Fields(vec![field("a")]),
            None,
        );
        assert_ne!(p, f);
    }
}
