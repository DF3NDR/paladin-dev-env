//! Graphviz DOT rendering of a [`GraphShape`] (D-19): a `digraph` with
//! quoted, `n{i}`-sanitized node ids (T-28-07-01 -- mirrors `mermaid.rs`'s
//! own sanitization so neither diagram's own identifiers are ever an
//! author-supplied string), `subgraph cluster_<id>` for a Workflow node's
//! nested shape, `shape=diamond` for Gate, `style=dashed` for worker
//! templates and deferred nodes, and edge `label`s for conditions. Output is
//! byte-deterministic: built entirely from `GraphShape`'s own
//! declaration-order `Vec`s, never from `HashMap` iteration.

use std::collections::HashMap;

use crate::engine::export::shape::{GraphShape, ShapeKind};

/// Render `shape` as a Graphviz `digraph` string (D-19). Calling this twice
/// on the same `shape` returns byte-identical strings.
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::export::{GraphShape, to_dot};
/// use paladin_battalion::engine::graph::{EngineLimits, WarGraph};
/// use paladin_core::platform::container::battlefield::BattlefieldSchema;
///
/// let graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
/// let dot = to_dot(&GraphShape::from_graph(&graph));
/// assert!(dot.starts_with("digraph"));
/// ```
pub fn to_dot(shape: &GraphShape) -> String {
    let mut out = String::from("digraph WarGraph {\n");
    let mut counter = 0usize;
    render_shape(shape, &mut out, &mut counter, 1);
    out.push_str("}\n");
    out
}

/// Recursively renders `shape`'s nodes (assigning each the next `n{i}` in
/// pre-order declaration order, continuing the SAME global counter into a
/// nested cluster -- mirrors `mermaid.rs::render_shape`'s exact numbering
/// discipline) followed by this level's own edges.
fn render_shape(shape: &GraphShape, out: &mut String, counter: &mut usize, depth: u32) {
    let indent = "  ".repeat(depth as usize);
    let mut local_names: HashMap<&str, String> = HashMap::new();

    for node in &shape.nodes {
        let sanitized = format!("n{counter}");
        *counter += 1;
        local_names.insert(node.id.as_str(), sanitized.clone());
        let dashed = node.deferred || node.worker_template;

        match (&node.kind, &node.subgraph) {
            (ShapeKind::Workflow, Some(sub)) => {
                out.push_str(&format!(
                    "{indent}subgraph cluster_{sanitized} {{\n{indent}  label=\"{}\";\n",
                    node_label(node.id.as_str(), node.kind)
                ));
                if dashed {
                    out.push_str(&format!("{indent}  style=dashed;\n"));
                }
                render_shape(sub, out, counter, depth + 1);
                out.push_str(&format!("{indent}}}\n"));
            }
            (ShapeKind::Gate, _) => {
                out.push_str(&format!(
                    "{indent}\"{sanitized}\" [label=\"{}\", shape=diamond{}];\n",
                    node_label(node.id.as_str(), node.kind),
                    if dashed { ", style=dashed" } else { "" }
                ));
            }
            _ => {
                out.push_str(&format!(
                    "{indent}\"{sanitized}\" [label=\"{}\"{}];\n",
                    node_label(node.id.as_str(), node.kind),
                    if dashed { ", style=dashed" } else { "" }
                ));
            }
        }
    }

    for edge in &shape.edges {
        let from = local_names
            .get(edge.from.as_str())
            .cloned()
            .unwrap_or_else(|| edge.from.to_string());
        let to = local_names
            .get(edge.to.as_str())
            .cloned()
            .unwrap_or_else(|| edge.to.to_string());
        match &edge.condition {
            None => out.push_str(&format!("{indent}\"{from}\" -> \"{to}\";\n")),
            Some(label) => out.push_str(&format!(
                "{indent}\"{from}\" -> \"{to}\" [label=\"{}\"];\n",
                escape(label)
            )),
        }
    }
}

/// `"<real id> «kind»"` (D-19, 28-UI-SPEC.md), escaped for DOT's
/// double-quoted string syntax.
fn node_label(id: &str, kind: ShapeKind) -> String {
    format!("{} «{}»", escape(id), badge(kind))
}

/// The guillemet badge word for each `ShapeKind` (28-UI-SPEC.md, verbatim,
/// mirrors `mermaid.rs::badge`).
fn badge(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Paladin => "paladin",
        ShapeKind::Function => "function",
        ShapeKind::Gate => "gate",
        ShapeKind::Workflow => "workflow",
        ShapeKind::WorkerTemplate => "worker",
    }
}

/// Escapes a label's content for DOT's double-quoted `"..."` string syntax
/// (T-28-07-01): a node id or edge-condition value the graph's AUTHOR
/// supplied is rendered here, so a literal `"` cannot terminate the string
/// early and inject additional DOT attributes or statements.
fn escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_neutralizes_quotes() {
        // Every literal `"` gains a preceding backslash, so no quote in the
        // escaped output can terminate the DOT string early.
        let escaped = escape("node\" [shape=box]");
        assert_eq!(escaped, "node\\\" [shape=box]");
        assert_eq!(
            escaped.matches('"').count(),
            escaped.matches("\\\"").count()
        );
    }

    #[test]
    fn badges_cover_every_kind() {
        for kind in [
            ShapeKind::Paladin,
            ShapeKind::Function,
            ShapeKind::Gate,
            ShapeKind::Workflow,
            ShapeKind::WorkerTemplate,
        ] {
            assert!(!badge(kind).is_empty());
        }
    }
}
