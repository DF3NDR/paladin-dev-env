//! Mermaid flowchart rendering of a [`GraphShape`] (D-19): `flowchart TD`,
//! node ids sanitized to `n{i}` in declaration order (T-28-07-01 -- the
//! diagram's own identifiers are never the author-supplied real id), the
//! real id plus a guillemet kind badge in each label, Gate rendered as a
//! diamond, Workflow rendered as a nested `subgraph` cluster over the child
//! shape, dashed styling for worker templates and deferred nodes, and
//! conditional edges labeled by condition kind. Output is byte-deterministic:
//! built entirely from `GraphShape`'s own declaration-order `Vec`s, never
//! from `HashMap` iteration.

use std::collections::HashMap;

use crate::engine::export::shape::{GraphShape, ShapeKind};

/// Render `shape` as a Mermaid `flowchart TD` string (D-19). Calling this
/// twice on the same `shape` returns byte-identical strings.
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::export::{GraphShape, to_mermaid};
/// use paladin_battalion::engine::graph::{EngineLimits, WarGraph};
/// use paladin_core::platform::container::battlefield::BattlefieldSchema;
///
/// let graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
/// let mermaid = to_mermaid(&GraphShape::from_graph(&graph));
/// assert!(mermaid.starts_with("flowchart TD"));
/// ```
pub fn to_mermaid(shape: &GraphShape) -> String {
    let mut out = String::from("flowchart TD\n");
    let mut counter = 0usize;
    let mut classes: Vec<(String, ShapeKind, bool)> = Vec::new();

    render_shape(shape, &mut out, &mut counter, &mut classes);
    write_class_defs(&mut out);
    apply_classes(&classes, &mut out);
    out
}

/// Recursively renders `shape`'s nodes (assigning each the next `n{i}` in
/// pre-order declaration order, so a nested subgraph's own nodes continue
/// the SAME global counter rather than restarting at zero -- required for
/// Mermaid node ids to stay unique across the whole diagram) followed by
/// this level's own edges. `classes` accumulates every node's `(sanitized
/// id, kind, dashed)` triple so the caller can emit every `classDef`
/// application once, after the whole diagram body.
fn render_shape(
    shape: &GraphShape,
    out: &mut String,
    counter: &mut usize,
    classes: &mut Vec<(String, ShapeKind, bool)>,
) {
    let mut local_names: HashMap<&str, String> = HashMap::new();

    for node in &shape.nodes {
        let sanitized = format!("n{counter}");
        *counter += 1;
        local_names.insert(node.id.as_str(), sanitized.clone());
        let dashed = node.deferred || node.worker_template;
        classes.push((sanitized.clone(), node.kind, dashed));

        match (&node.kind, &node.subgraph) {
            (ShapeKind::Workflow, Some(sub)) => {
                out.push_str(&format!(
                    "subgraph {sanitized} [\"{}\"]\n",
                    node_label(node.id.as_str(), node.kind)
                ));
                render_shape(sub, out, counter, classes);
                out.push_str("end\n");
            }
            (ShapeKind::Gate, _) => {
                out.push_str(&format!(
                    "{sanitized}{{\"{}\"}}\n",
                    node_label(node.id.as_str(), node.kind)
                ));
            }
            _ => {
                out.push_str(&format!(
                    "{sanitized}[\"{}\"]\n",
                    node_label(node.id.as_str(), node.kind)
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
            None => out.push_str(&format!("{from} --> {to}\n")),
            Some(label) => {
                out.push_str(&format!("{from} -- {} --> {to}\n", escape(label)));
            }
        }
    }
}

/// `"<real id> «kind»"` (D-19, 28-UI-SPEC.md), escaped for Mermaid's
/// double-quoted label syntax.
fn node_label(id: &str, kind: ShapeKind) -> String {
    format!("{} «{}»", escape(id), badge(kind))
}

/// The guillemet badge word for each `ShapeKind` (28-UI-SPEC.md, verbatim).
fn badge(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Paladin => "paladin",
        ShapeKind::Function => "function",
        ShapeKind::Gate => "gate",
        ShapeKind::Workflow => "workflow",
        ShapeKind::WorkerTemplate => "worker",
    }
}

/// The `classDef` name backing each `ShapeKind`'s fill/stroke style.
fn class_name(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Paladin => "paladinKind",
        ShapeKind::Function => "functionKind",
        ShapeKind::Gate => "gateKind",
        ShapeKind::Workflow => "workflowKind",
        ShapeKind::WorkerTemplate => "workerKind",
    }
}

/// One `classDef` per `ShapeKind` plus the shared `dashed` class worker
/// templates and deferred nodes both apply (D-19).
fn write_class_defs(out: &mut String) {
    out.push_str("classDef paladinKind fill:#eef2ff,stroke:#4338ca,color:#1e1b4b;\n");
    out.push_str("classDef functionKind fill:#ecfeff,stroke:#0891b2,color:#164e63;\n");
    out.push_str("classDef gateKind fill:#fefce8,stroke:#ca8a04,color:#713f12;\n");
    out.push_str("classDef workflowKind fill:#fdf4ff,stroke:#a21caf,color:#4a044e;\n");
    out.push_str("classDef workerKind fill:#f0fdf4,stroke:#15803d,color:#052e16;\n");
    out.push_str("classDef dashed stroke-dasharray: 5 5;\n");
}

/// One `class` application per collected node, in the SAME order they were
/// declared (never sorted or grouped by kind, so the output stays
/// declaration-order deterministic).
fn apply_classes(classes: &[(String, ShapeKind, bool)], out: &mut String) {
    for (id, kind, dashed) in classes {
        out.push_str(&format!("class {id} {}\n", class_name(*kind)));
        if *dashed {
            out.push_str(&format!("class {id} dashed\n"));
        }
    }
}

/// Escapes a label's content for Mermaid's double-quoted `"..."` syntax
/// (T-28-07-01): a node id or edge-condition value the graph's AUTHOR
/// supplied is rendered here, so `"`, `<` and `>` cannot break out of the
/// label into diagram syntax or (once embedded in an HTML page, 28-15)
/// markup. Uses Mermaid's own `#code;` HTML-entity-by-name escape form.
fn escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "#quot;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_neutralizes_quotes_and_angle_brackets() {
        let escaped = escape("node\" <script>&");
        assert!(!escaped.contains('"'));
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
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
            assert!(!class_name(kind).is_empty());
        }
    }
}
