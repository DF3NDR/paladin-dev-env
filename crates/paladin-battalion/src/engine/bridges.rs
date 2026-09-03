//! Legacy bridges (ENG-FR-19, X-03): `WarGraph::from_formation`,
//! `WarGraph::from_phalanx` and `WarGraph::from_campaign` reproduce
//! `FormationExecutionService`, `PhalanxExecutionService` and
//! `CampaignExecutionService`'s existing string-in/string-out data flow as
//! typed `WarGraph`s, byte for byte, without modifying any of those legacy
//! services or their public signatures (ENG-FR-20). This module is
//! deliberately additive: nothing under `formation_service.rs`,
//! `phalanx_service.rs`, `campaign_service.rs` or `commander.rs` is read
//! mutably or referenced as anything other than the golden test's own
//! comparison target.
//!
//! # The default three-field schema
//!
//! `from_formation` and `from_phalanx` build their graphs over EXACTLY three
//! Battlefield fields (ENG-FR-19's default schema): `input` (`LastWrite`),
//! `output` (`LastWrite`) and `history` (`Append`, defaulting to an empty
//! array). Both legacy patterns have a uniform, symmetric data flow (a single
//! linear chain; a single concurrent fan-out with no fan-in) that fits this
//! shared field set without ever producing two distinct writers touching the
//! same `LastWrite` field in one superstep.
//!
//! `from_campaign` starts from that SAME three-field baseline (the `input`
//! field an entry node reads, matching legacy's "entry point uses initial
//! input") but a general DAG cannot reproduce its own fan-out/fan-in
//! structure through a single shared `output` field: two Campaign siblings
//! (e.g. a diamond's two branches) run in the SAME superstep, and if both
//! wrote into one shared `LastWrite` field the merge would raise a hard
//! `DispatchConflict` -- a divergence from legacy's own `results.last()`
//! aggregation, which is a pure Rust `Vec` operation.  `from_campaign`
//! therefore extends the baseline schema with one dedicated `LastWrite`
//! field per Paladin (never touched by any other node), and its `output`
//! field is left declared-but-unused. See [`dedicated_output_field`] for the
//! per-node field this produces, and the module-level fan-in doc below for
//! how those dedicated fields let `InputMapping`'s plain string templating
//! reproduce the legacy fan-in concatenation with no engine-level
//! aggregation logic at all.
//!
//! # Reproducing the legacy Campaign fan-in with plain string templates
//!
//! `campaign_service.rs`'s `aggregate_inputs_for_node` (line 373) joins
//! multiple parents' outputs with [`CAMPAIGN_FAN_IN_SEPARATOR`], or passes a
//! single parent's output straight through with no separator. Because every
//! Campaign-bridged node's own output lives in its own dedicated
//! `LastWrite` field (never shared), a fan-in node's `InputMapping` template
//! can name each parent's dedicated field directly, joined by the SAME
//! separator, entirely as literal template text -- `InputMapping::render`
//! needs no special-cased "aggregate multiple fields" behavior for this to
//! work; it is exactly the same one-placeholder-per-field substitution every
//! other node uses, just with more than one placeholder in the same
//! template.

use std::collections::HashMap;

use petgraph::visit::EdgeRef;
use uuid::Uuid;

use paladin_core::platform::container::battalion::campaign::Campaign;
use paladin_core::platform::container::battlefield::{
    BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::NodeId;

use crate::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
use crate::engine::input_mapping::InputMapping;

/// The legacy Campaign fan-in separator, read verbatim from
/// `campaign_service.rs:373`'s own `inputs.join(...)` call so the bridge and
/// the legacy service can never drift independently of one another.
pub const CAMPAIGN_FAN_IN_SEPARATOR: &str = "\n\n---\n\n";

fn input_field() -> FieldName {
    FieldName::new("input").expect("literal \"input\" is non-empty")
}

fn output_field() -> FieldName {
    FieldName::new("output").expect("literal \"output\" is non-empty")
}

fn history_field() -> FieldName {
    FieldName::new("history").expect("literal \"history\" is non-empty")
}

/// The default three-field schema shared by every legacy bridge (ENG-FR-19):
/// `input` (`LastWrite`, no default), `output` (`LastWrite`, no default) and
/// `history` (`Append`, defaulting to an empty array so a reader always
/// gets `Some(vec![])` rather than `None` before anything is appended).
fn default_fields() -> Vec<FieldSpec> {
    vec![
        FieldSpec::new(input_field(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(output_field(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(
            history_field(),
            DispatchRule::Append,
            Some(serde_json::json!([])),
            false,
        ),
    ]
}

/// A zero-padded, sort-stable `NodeId`: `{prefix}0000`, `{prefix}0001`, ...
/// Padding keeps `NodeId`'s lexicographic `Ord` (which `Battlefield::merge`'s
/// `Append` dispatch sorts writers by) aligned with the caller's own `Vec`
/// insertion order for up to 10,000 nodes -- comfortably beyond any
/// `Formation`/`Phalanx` this bridge is ever asked to reproduce.
fn indexed_node_id(prefix: &str, index: usize) -> NodeId {
    NodeId::new(format!("{prefix}{index:04}"))
}

/// Produce a lowercase, hyphen-separated slug from a Paladin name (D-05):
/// every run of non-ASCII-alphanumeric characters collapses to one `-`,
/// with no leading or trailing `-`. Falls back to `"node"` for a name with
/// no alphanumeric characters at all, so a `NodeId` built from a slug is
/// never empty.
fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_sep = true; // suppresses a leading '-'
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('-');
            last_was_sep = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "node".to_string()
    } else {
        slug
    }
}

/// The first 8 hex characters of `id`'s simple (no-hyphen) form -- the
/// "short uuid" D-05 specifies for a colliding slug's disambiguating suffix.
fn short_uuid(id: Uuid) -> String {
    id.simple().to_string()[..8].to_string()
}

/// Compute the deterministic, human-readable `NodeId` for every Paladin in
/// `campaign` (D-05): a Paladin's name slug when that slug is unique within
/// the graph, else the slug suffixed with an 8-hex-character short form of
/// its own uuid. A pure function of `campaign`'s own content (every input is
/// read directly off `campaign`, nothing else) -- calling this twice on the
/// SAME `Campaign` value always returns the identical mapping, which is what
/// makes a bridged graph's `fingerprint()` and a resumed thread's stored
/// `NodeId`s stable across repeated construction.
pub fn campaign_node_ids(campaign: &Campaign) -> HashMap<Uuid, NodeId> {
    // Sorted by uuid purely so this function's own behavior is reproducible
    // byte-for-byte if ever traced or logged; the produced NodeId mapping
    // does not depend on this iteration order (every slug's uniqueness is
    // decided by a full count first, independent of visitation order).
    let mut entries: Vec<(Uuid, &Paladin)> = campaign
        .paladins()
        .iter()
        .map(|(uuid, paladin)| (*uuid, paladin))
        .collect();
    entries.sort_by_key(|(uuid, _)| *uuid);

    let mut slug_counts: HashMap<String, usize> = HashMap::new();
    let mut slugs: HashMap<Uuid, String> = HashMap::new();
    for (uuid, paladin) in &entries {
        let slug = slugify(&paladin.node.name);
        *slug_counts.entry(slug.clone()).or_insert(0) += 1;
        slugs.insert(*uuid, slug);
    }

    entries
        .into_iter()
        .map(|(uuid, _)| {
            let slug = slugs.remove(&uuid).unwrap_or_else(|| "node".to_string());
            let is_unique = slug_counts.get(&slug).copied().unwrap_or(0) <= 1;
            let id = if is_unique {
                slug
            } else {
                format!("{slug}-{}", short_uuid(uuid))
            };
            (uuid, NodeId::new(id))
        })
        .collect()
}

/// The dedicated `LastWrite` Battlefield field `from_campaign` writes
/// `node_id`'s own Paladin output into: `out__{node_id}`. Exposed so a
/// caller (the golden equivalence test, in particular) can read a specific
/// bridged Campaign node's output back out of a `Battlefield` without
/// needing to re-derive this naming scheme itself.
pub fn dedicated_output_field(node_id: &NodeId) -> FieldName {
    FieldName::new(format!("out__{}", node_id.as_str()))
        .expect("a NodeId's string form is always non-empty, so the prefixed field name is too")
}

impl WarGraph {
    /// Build a `WarGraph` reproducing `FormationExecutionService`'s
    /// sequential data flow (ENG-FR-19): the first Paladin reads the shared
    /// `input` field; every subsequent Paladin reads `output`, which is the
    /// PREVIOUS Paladin's own overwrite of that same `LastWrite` field --
    /// safe because Formation nodes execute one per superstep, never
    /// concurrently, so two distinct writers can never touch `output` in the
    /// same superstep. `paladins.is_empty()` produces a graph with no nodes
    /// and no entry point, which `WarGraph::validate` accepts and whose
    /// `WarEngine::start` run completes immediately with an empty Vanguard.
    pub fn from_formation(paladins: Vec<Paladin>) -> WarGraph {
        let schema = BattlefieldSchema::new(default_fields());
        let mut graph = WarGraph::new(schema, EngineLimits::default());

        let ids: Vec<NodeId> = (0..paladins.len())
            .map(|i| indexed_node_id("f", i))
            .collect();
        for (i, paladin) in paladins.into_iter().enumerate() {
            let input_template = if i == 0 {
                InputMapping::new(format!("{{{}}}", input_field()))
            } else {
                InputMapping::new(format!("{{{}}}", output_field()))
            };
            graph.add_node(
                ids[i].clone(),
                NodeSpec::Paladin {
                    paladin: Box::new(paladin),
                    input_template,
                    output_field: output_field(),
                },
            );
        }
        for pair in ids.windows(2) {
            graph.add_edge(EdgeSpec {
                from: pair[0].clone(),
                to: pair[1].clone(),
                condition: None,
            });
        }
        if let Some(first) = ids.first() {
            graph.add_entry(first.clone());
        }
        graph
    }

    /// Build a `WarGraph` reproducing `PhalanxExecutionService`'s concurrent
    /// data flow (ENG-FR-19): every Paladin is a graph entry point (all run
    /// in the same, first superstep), every one reads the shared `input`
    /// field, and every one writes into the shared `history` `Append` field
    /// -- concurrent distinct writers never conflict under `Append`, and are
    /// merged in `(NodeId, emission index)` order, which the zero-padded
    /// `NodeId`s this bridge assigns keep aligned with `paladins`' own `Vec`
    /// order. `paladins.is_empty()` produces a graph with no nodes and no
    /// entry point, exactly like [`WarGraph::from_formation`]'s empty case.
    pub fn from_phalanx(paladins: Vec<Paladin>) -> WarGraph {
        let schema = BattlefieldSchema::new(default_fields());
        let mut graph = WarGraph::new(schema, EngineLimits::default());

        let template = InputMapping::new(format!("{{{}}}", input_field()));
        for (i, paladin) in paladins.into_iter().enumerate() {
            let id = indexed_node_id("p", i);
            graph.add_node(
                id.clone(),
                NodeSpec::Paladin {
                    paladin: Box::new(paladin),
                    input_template: template.clone(),
                    output_field: history_field(),
                },
            );
            graph.add_entry(id);
        }
        graph
    }

    /// Build a `WarGraph` reproducing `CampaignExecutionService`'s
    /// graph-shaped data flow (ENG-FR-19, D-05): `campaign`'s own edges and
    /// entry points, each Paladin identified by its
    /// [`campaign_node_ids`]-computed `NodeId`, each writing to its own
    /// [`dedicated_output_field`] so concurrent siblings never conflict. An
    /// entry node's `InputMapping` reads the shared `input` field (matching
    /// legacy's "entry point uses initial input"); every other node's
    /// `InputMapping` is built from its incoming edges in the SAME order
    /// `campaign_service.rs::aggregate_inputs_for_node` iterates them,
    /// joined by [`CAMPAIGN_FAN_IN_SEPARATOR`] for two or more parents, with
    /// no separator for exactly one, and an empty literal template for zero
    /// (matching that function's `is_empty`/`len() == 1`/multi-parent
    /// branches exactly).
    pub fn from_campaign(campaign: &Campaign) -> WarGraph {
        let node_ids = campaign_node_ids(campaign);

        let mut fields = default_fields();
        let mut dedicated: Vec<&NodeId> = node_ids.values().collect();
        dedicated.sort();
        for id in &dedicated {
            fields.push(FieldSpec::new(
                dedicated_output_field(id),
                DispatchRule::LastWrite,
                None,
                false,
            ));
        }
        let schema = BattlefieldSchema::new(fields);
        let mut graph = WarGraph::new(schema, EngineLimits::default());

        let entry_points = campaign.entry_points();

        for (uuid, paladin) in campaign.paladins() {
            let node_id = node_ids
                .get(uuid)
                .expect("campaign_node_ids maps every paladin in this same campaign")
                .clone();
            let input_template = if entry_points.contains(uuid) {
                InputMapping::new(format!("{{{}}}", input_field()))
            } else {
                fan_in_template(campaign, *uuid, &node_ids)
            };
            graph.add_node(
                node_id.clone(),
                NodeSpec::Paladin {
                    paladin: Box::new(paladin.clone()),
                    input_template,
                    output_field: dedicated_output_field(&node_id),
                },
            );
        }

        for edge in campaign.graph().edge_weights() {
            let (Some(from), Some(to)) = (node_ids.get(&edge.source), node_ids.get(&edge.target))
            else {
                continue;
            };
            graph.add_edge(EdgeSpec {
                from: from.clone(),
                to: to.clone(),
                condition: Some(edge.condition.clone()),
            });
        }

        for uuid in &entry_points {
            if let Some(node_id) = node_ids.get(uuid) {
                graph.add_entry(node_id.clone());
            }
        }

        graph
    }
}

/// Build a non-entry Campaign node's `InputMapping`, reproducing
/// `campaign_service.rs::aggregate_inputs_for_node`'s exact
/// zero/one/many-parent branches as literal template text over each
/// parent's [`dedicated_output_field`] (see the module-level "Reproducing
/// the legacy Campaign fan-in" doc for why plain string templating is
/// sufficient here).
fn fan_in_template(
    campaign: &Campaign,
    target_uuid: Uuid,
    node_ids: &HashMap<Uuid, NodeId>,
) -> InputMapping {
    let Some(&node_index) = campaign.node_indices().get(&target_uuid) else {
        return InputMapping::new(String::new());
    };

    let mut placeholders = Vec::new();
    for edge in campaign
        .graph()
        .edges_directed(node_index, petgraph::Direction::Incoming)
    {
        let source_uuid = campaign.graph()[edge.source()];
        if let Some(source_node_id) = node_ids.get(&source_uuid) {
            placeholders.push(format!("{{{}}}", dedicated_output_field(source_node_id)));
        }
    }

    if placeholders.is_empty() {
        InputMapping::new(String::new())
    } else {
        InputMapping::new(placeholders.join(CAMPAIGN_FAN_IN_SEPARATOR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battalion::BattalionConfig;
    use paladin_core::platform::container::battalion::campaign::{CampaignEdge, EdgeCondition};
    use paladin_core::platform::container::paladin::PaladinData;
    use paladin_core::platform::container::waypoint::ThreadId;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
    use std::sync::Arc;

    use crate::engine::test_support::RecordingPaladinPort;
    use crate::engine::{RunOutcome, WarEngine};

    fn make_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            name: name.to_string(),
            ..Default::default()
        };
        Node::new(data, Some(name.to_string()))
    }

    // --- default three-field schema -------------------------------------

    #[test]
    fn default_schema_is_exactly_three_named_fields_with_stated_dispatch_rules() {
        let graph = WarGraph::from_formation(vec![make_paladin("solo")]);
        let fields = &graph.schema().fields;
        assert_eq!(fields.len(), 3);

        let by_name: HashMap<&str, &DispatchRule> = fields
            .iter()
            .map(|f| (f.name.as_str(), &f.dispatch))
            .collect();
        assert!(matches!(
            by_name.get("input"),
            Some(DispatchRule::LastWrite)
        ));
        assert!(matches!(
            by_name.get("output"),
            Some(DispatchRule::LastWrite)
        ));
        assert!(matches!(by_name.get("history"), Some(DispatchRule::Append)));
    }

    #[test]
    fn from_phalanx_schema_is_also_exactly_the_three_default_fields() {
        let graph = WarGraph::from_phalanx(vec![make_paladin("a"), make_paladin("b")]);
        assert_eq!(graph.schema().fields.len(), 3);
    }

    // --- from_formation ----------------------------------------------------

    #[tokio::test]
    async fn from_formation_chains_output_into_next_input() {
        let graph = WarGraph::from_formation(vec![
            make_paladin("p1"),
            make_paladin("p2"),
            make_paladin("p3"),
        ]);
        let port = Arc::new(RecordingPaladinPort::new());
        port.set_output("p1", "out1");
        port.set_output("p2", "out2");
        port.set_output("p3", "out3");
        let engine = WarEngine::new(port.clone(), Arc::new(InMemoryWaypointStore::new()));

        let mut initial = paladin_core::platform::container::battlefield::StateDelta::new();
        initial
            .set(FieldName::new("input").unwrap(), "seed")
            .unwrap();
        let outcome = engine
            .start(&graph, ThreadId::new("formation-chain").unwrap(), initial)
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&FieldName::new("output").unwrap())
                        .unwrap(),
                    Some("out3".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(
            port.call_log(),
            vec![
                ("p1".to_string(), "seed".to_string()),
                ("p2".to_string(), "out1".to_string()),
                ("p3".to_string(), "out2".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn from_formation_empty_list_validates_and_completes_immediately() {
        let graph = WarGraph::from_formation(vec![]);
        graph
            .validate(
                &paladin_core::platform::container::battlefield::CustomDispatchResolver::new(),
                &crate::edge_evaluator::EdgeEvaluatorRegistry::new(),
            )
            .expect("empty formation graph must validate");

        let engine = WarEngine::new(
            Arc::new(RecordingPaladinPort::new()),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let outcome = engine
            .start(
                &graph,
                ThreadId::new("formation-empty").unwrap(),
                paladin_core::platform::container::battlefield::StateDelta::new(),
            )
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    // --- from_phalanx --------------------------------------------------

    #[tokio::test]
    async fn from_phalanx_all_write_history_in_vec_order() {
        let graph = WarGraph::from_phalanx(vec![
            make_paladin("a"),
            make_paladin("b"),
            make_paladin("c"),
        ]);
        let port = Arc::new(RecordingPaladinPort::new());
        port.set_output("a", "alpha");
        port.set_output("b", "beta");
        port.set_output("c", "gamma");
        let engine = WarEngine::new(port.clone(), Arc::new(InMemoryWaypointStore::new()));

        let mut initial = paladin_core::platform::container::battlefield::StateDelta::new();
        initial.set(FieldName::new("input").unwrap(), "go").unwrap();
        let outcome = engine
            .start(&graph, ThreadId::new("phalanx-history").unwrap(), initial)
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let history: Vec<String> = final_state
                    .get(&FieldName::new("history").unwrap())
                    .unwrap()
                    .unwrap();
                assert_eq!(history, vec!["alpha", "beta", "gamma"]);
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn from_phalanx_empty_list_validates_and_completes_immediately() {
        let graph = WarGraph::from_phalanx(vec![]);
        graph
            .validate(
                &paladin_core::platform::container::battlefield::CustomDispatchResolver::new(),
                &crate::edge_evaluator::EdgeEvaluatorRegistry::new(),
            )
            .expect("empty phalanx graph must validate");

        let engine = WarEngine::new(
            Arc::new(RecordingPaladinPort::new()),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let outcome = engine
            .start(
                &graph,
                ThreadId::new("phalanx-empty").unwrap(),
                paladin_core::platform::container::battlefield::StateDelta::new(),
            )
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    // --- from_campaign: NodeId mapping (D-05) ---------------------------

    fn campaign_with(names: &[&str]) -> (Campaign, Vec<Uuid>) {
        let mut campaign = Campaign::new(BattalionConfig::new("test"));
        let ids = names
            .iter()
            .map(|name| campaign.add_paladin(make_paladin(name)))
            .collect();
        (campaign, ids)
    }

    #[test]
    fn unique_slugs_produce_bare_slug_node_ids() {
        let (campaign, ids) = campaign_with(&["alpha", "beta"]);
        let mapping = campaign_node_ids(&campaign);
        assert_eq!(mapping.get(&ids[0]), Some(&NodeId::new("alpha")));
        assert_eq!(mapping.get(&ids[1]), Some(&NodeId::new("beta")));
    }

    #[test]
    fn colliding_slugs_get_distinct_short_uuid_suffixes() {
        let mut campaign = Campaign::new(BattalionConfig::new("test"));
        let id_a = campaign.add_paladin(make_paladin("Worker"));
        let id_b = campaign.add_paladin(make_paladin("worker")); // same slug

        let mapping = campaign_node_ids(&campaign);
        let node_a = mapping.get(&id_a).unwrap();
        let node_b = mapping.get(&id_b).unwrap();

        assert_ne!(node_a, node_b);
        assert!(node_a.as_str().starts_with("worker-"));
        assert!(node_b.as_str().starts_with("worker-"));
        assert_ne!(node_a.as_str(), "worker");
        assert_ne!(node_b.as_str(), "worker");
    }

    #[test]
    fn same_campaign_built_twice_yields_identical_node_ids_and_fingerprint() {
        let (campaign, _ids) = campaign_with(&["researcher", "writer", "researcher"]);
        let graph_a = WarGraph::from_campaign(&campaign);
        let graph_b = WarGraph::from_campaign(&campaign);

        let mapping_a = campaign_node_ids(&campaign);
        let mapping_b = campaign_node_ids(&campaign);
        assert_eq!(mapping_a, mapping_b);
        assert_eq!(graph_a.fingerprint(), graph_b.fingerprint());
    }

    // --- from_campaign: fan-in construction (Task 1's own unit proof) ---

    #[test]
    fn two_parent_fan_in_uses_the_legacy_separator_read_from_its_source() {
        // Read the separator from the actual legacy call site's own source
        // file rather than a retyped literal, per the acceptance criterion:
        // assert the source file's own `inputs.join("...")` call literally
        // contains CAMPAIGN_FAN_IN_SEPARATOR's escaped form, rather than
        // parsing the source's string-literal syntax by hand.
        let legacy_source = include_str!("../campaign_service.rs");
        let escaped_separator = CAMPAIGN_FAN_IN_SEPARATOR.replace('\n', "\\n");
        let expected_call = format!("inputs.join(\"{escaped_separator}\")");
        assert!(
            legacy_source.contains(&expected_call),
            "campaign_service.rs must still contain {expected_call:?} verbatim"
        );

        let mut campaign = Campaign::new(BattalionConfig::new("fan-in"));
        let a = campaign.add_paladin(make_paladin("a"));
        let b = campaign.add_paladin(make_paladin("b"));
        let d = campaign.add_paladin(make_paladin("d"));
        campaign
            .add_edge(CampaignEdge::new(a, d, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(b, d, EdgeCondition::Always))
            .unwrap();
        campaign.set_entry_point(a).unwrap();
        campaign.set_entry_point(b).unwrap();

        let node_ids = campaign_node_ids(&campaign);
        let template = fan_in_template(&campaign, d, &node_ids);
        let a_field = dedicated_output_field(&node_ids[&a]);
        let b_field = dedicated_output_field(&node_ids[&b]);
        let expected = format!(
            "{{{a_field}}}{CAMPAIGN_FAN_IN_SEPARATOR}{{{b_field}}}",
            a_field = a_field,
            b_field = b_field,
        );
        // Both parent orderings are legitimate (petgraph's own incoming-edge
        // iteration order decides which); assert the template is one of the
        // two, joined with the real separator either way. Compared via
        // `InputMapping`'s own `PartialEq`, not a `{:?}`-debug-escaped
        // string, since `Debug` re-escapes the separator's real newlines as
        // literal `\n` two-character sequences.
        let reversed = format!(
            "{{{b_field}}}{CAMPAIGN_FAN_IN_SEPARATOR}{{{a_field}}}",
            a_field = a_field,
            b_field = b_field,
        );
        assert!(
            template == InputMapping::new(expected.clone())
                || template == InputMapping::new(reversed.clone()),
            "fan-in template {template:?} did not match either parent ordering ({expected:?} / {reversed:?})"
        );
    }

    #[test]
    fn one_parent_fan_in_inserts_no_separator() {
        let mut campaign = Campaign::new(BattalionConfig::new("single-parent"));
        let a = campaign.add_paladin(make_paladin("a"));
        let b = campaign.add_paladin(make_paladin("b"));
        campaign
            .add_edge(CampaignEdge::new(a, b, EdgeCondition::Always))
            .unwrap();
        campaign.set_entry_point(a).unwrap();

        let node_ids = campaign_node_ids(&campaign);
        let template = fan_in_template(&campaign, b, &node_ids);
        let a_field = dedicated_output_field(&node_ids[&a]);
        let expected = InputMapping::new(format!("{{{a_field}}}"));
        assert_eq!(template, expected);
    }

    // --- from_campaign: empty-list / structural sanity -------------------

    #[test]
    fn from_campaign_over_empty_campaign_produces_a_graph_with_no_nodes() {
        let campaign = Campaign::new(BattalionConfig::new("empty"));
        let graph = WarGraph::from_campaign(&campaign);
        assert!(graph.node_order().is_empty());
        assert!(graph.entry().is_empty());
        graph
            .validate(
                &paladin_core::platform::container::battlefield::CustomDispatchResolver::new(),
                &crate::edge_evaluator::EdgeEvaluatorRegistry::new(),
            )
            .expect("an empty campaign-bridged graph must still validate");
    }

    #[tokio::test]
    async fn from_campaign_reproduces_diamond_fan_out_and_fan_in() {
        let mut campaign = Campaign::new(BattalionConfig::new("diamond"));
        let a = campaign.add_paladin(make_paladin("paladin_a"));
        let b = campaign.add_paladin(make_paladin("paladin_b"));
        let c = campaign.add_paladin(make_paladin("paladin_c"));
        let d = campaign.add_paladin(make_paladin("paladin_d"));
        campaign
            .add_edge(CampaignEdge::new(a, b, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(a, c, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(b, d, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(c, d, EdgeCondition::Always))
            .unwrap();
        campaign.set_entry_point(a).unwrap();

        let graph = WarGraph::from_campaign(&campaign);
        let _ = &d; // d's own dedicated field holds D's OWN output, not its
        // input -- the fan-in proof below reads D's received input from the
        // recording port's call log instead.

        let port = Arc::new(RecordingPaladinPort::new());
        port.set_output("paladin_a", "A-out");
        port.set_output("paladin_b", "B-out");
        port.set_output("paladin_c", "C-out");
        port.set_output("paladin_d", "D-out");
        let engine = WarEngine::new(port.clone(), Arc::new(InMemoryWaypointStore::new()));

        let mut initial = paladin_core::platform::container::battlefield::StateDelta::new();
        initial
            .set(FieldName::new("input").unwrap(), "start")
            .unwrap();
        let outcome = engine
            .start(&graph, ThreadId::new("diamond").unwrap(), initial)
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let d_input = port
            .call_log()
            .into_iter()
            .find(|(name, _)| name == "paladin_d")
            .map(|(_, input)| input)
            .expect("paladin_d must have been executed exactly once");
        assert!(
            d_input == "B-out\n\n---\n\nC-out" || d_input == "C-out\n\n---\n\nB-out",
            "unexpected D input: {d_input:?}"
        );
    }

    #[test]
    fn legacy_services_untouched_marker() {
        // A structural reminder, not a behavioral test: this module must
        // never import formation_service/phalanx_service/campaign_service
        // as anything other than read-only references from documentation
        // comments and the golden test. The actual invariant is enforced by
        // this plan's <verify> step (`git diff --stat` over those four
        // files), not by a unit test -- Rust has no "this crate's module
        // graph excludes touching that file's internals" assertion to make
        // here.
    }
}
