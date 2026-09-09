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

use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind};

use crate::engine::export::overlay::{ExecutionOverlay, Visit};
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

/// Render `shape` annotated by `overlay` as a Mermaid `flowchart TD` string
/// (D-21): the same node/edge structure [`to_mermaid`] draws, layered with
/// outcome-`classDef` node coloring (by each node's LAST visit), a `×N`
/// visit-count badge on any node visited more than once, `<duration>ms ·
/// <tokens>tok` cost figures on every visited node, a bold link style for
/// fired edges and a dotted one for evaluated-but-not-fired edges.
///
/// `overlay.observed_only`'s locked-title handling (D-22) lands in plan
/// 28-10's Task 2, alongside the observed-only shape builder it pairs with.
///
/// Calling this twice on the same `(shape, overlay)` pair returns
/// byte-identical strings, mirroring [`to_mermaid`]'s own determinism
/// contract.
pub fn to_mermaid_overlay(shape: &GraphShape, overlay: &ExecutionOverlay) -> String {
    let mut out = String::from("flowchart TD\n");
    let mut counter = 0usize;
    let mut classes: Vec<(String, ShapeKind, bool)> = Vec::new();
    let mut outcome_classes: Vec<(String, &'static str)> = Vec::new();

    render_overlay_shape(
        shape,
        overlay,
        &mut out,
        &mut counter,
        &mut classes,
        &mut outcome_classes,
    );
    write_class_defs(&mut out);
    write_outcome_class_defs(&mut out);
    apply_classes(&classes, &mut out);
    for (id, class) in &outcome_classes {
        out.push_str(&format!("class {id} {class}\n"));
    }
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

/// [`render_shape`]'s overlay-aware sibling (D-21): same node/edge
/// structure and the SAME global `n{i}` counter discipline, but each node's
/// label is [`overlay_node_label`] (outcome glyph, visit-count badge, cost
/// figures) instead of the plain [`node_label`], each visited node's
/// sanitized id is recorded in `outcome_classes` (from its LAST visit) for
/// the caller to `class`-apply after the diagram body, and each edge's link
/// style is chosen by [`edge_overlay_style`] instead of always being a plain
/// arrow.
fn render_overlay_shape(
    shape: &GraphShape,
    overlay: &ExecutionOverlay,
    out: &mut String,
    counter: &mut usize,
    classes: &mut Vec<(String, ShapeKind, bool)>,
    outcome_classes: &mut Vec<(String, &'static str)>,
) {
    let mut local_names: HashMap<&str, String> = HashMap::new();

    for node in &shape.nodes {
        let sanitized = format!("n{counter}");
        *counter += 1;
        local_names.insert(node.id.as_str(), sanitized.clone());
        let dashed = node.deferred || node.worker_template;
        classes.push((sanitized.clone(), node.kind, dashed));

        let visits = overlay.visits.get(&node.id);
        if let Some(last) = visits.and_then(|v| v.last()) {
            outcome_classes.push((
                sanitized.clone(),
                outcome_class(&last.outcome, last.cache_hit),
            ));
        }
        let label = overlay_node_label(node.id.as_str(), node.kind, visits);

        match (&node.kind, &node.subgraph) {
            (ShapeKind::Workflow, Some(sub)) => {
                out.push_str(&format!("subgraph {sanitized} [\"{label}\"]\n"));
                render_overlay_shape(sub, overlay, out, counter, classes, outcome_classes);
                out.push_str("end\n");
            }
            (ShapeKind::Gate, _) => {
                out.push_str(&format!("{sanitized}{{\"{label}\"}}\n"));
            }
            _ => {
                out.push_str(&format!("{sanitized}[\"{label}\"]\n"));
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
        let style = edge_overlay_style(overlay, &edge.from, &edge.to);
        out.push_str(&render_overlay_edge(&from, &to, &edge.condition, style));
    }
}

/// Whether `from -> to` fired, was evaluated but did not fire, or was never
/// evaluated at all in `overlay` (D-21).
enum EdgeOverlayStyle {
    /// Not present in either of `overlay`'s edge sets -- rendered exactly
    /// like [`render_shape`]'s plain arrow.
    Plain,
    /// In `overlay.fired_edges` -- rendered as a bold link.
    Fired,
    /// In `overlay.evaluated_edges` but not `overlay.fired_edges` --
    /// rendered as a dotted link. Never produced for a Waypoints-sourced
    /// overlay, whose `evaluated_edges` is always empty (D-21).
    EvaluatedNotFired,
}

fn edge_overlay_style(overlay: &ExecutionOverlay, from: &NodeId, to: &NodeId) -> EdgeOverlayStyle {
    let pair = (from.clone(), to.clone());
    if overlay.fired_edges.contains(&pair) {
        EdgeOverlayStyle::Fired
    } else if overlay.evaluated_edges.contains(&pair) {
        EdgeOverlayStyle::EvaluatedNotFired
    } else {
        EdgeOverlayStyle::Plain
    }
}

/// Renders one edge line per [`EdgeOverlayStyle`] (D-21): `==>`/`== label
/// ==>` bold for a fired edge, `-.->`/`-. label .->` dotted for an
/// evaluated-but-not-fired edge, and the same plain `-->`/`-- label -->`
/// [`render_shape`] always uses otherwise.
fn render_overlay_edge(
    from: &str,
    to: &str,
    condition: &Option<String>,
    style: EdgeOverlayStyle,
) -> String {
    match (style, condition) {
        (EdgeOverlayStyle::Fired, None) => format!("{from} ==> {to}\n"),
        (EdgeOverlayStyle::Fired, Some(label)) => format!("{from} == {} ==> {to}\n", escape(label)),
        (EdgeOverlayStyle::EvaluatedNotFired, None) => format!("{from} -.-> {to}\n"),
        (EdgeOverlayStyle::EvaluatedNotFired, Some(label)) => {
            format!("{from} -. {} .-> {to}\n", escape(label))
        }
        (EdgeOverlayStyle::Plain, None) => format!("{from} --> {to}\n"),
        (EdgeOverlayStyle::Plain, Some(label)) => format!("{from} -- {} --> {to}\n", escape(label)),
    }
}

/// `overlay_node_label`'s unvisited-node case is [`node_label`] unchanged;
/// a visited node appends its LAST visit's outcome glyph, a `×N` badge when
/// visited more than once, and `<duration>ms · <tokens>tok` cost figures
/// (D-21, 28-UI-SPEC.md "populated | E1").
fn overlay_node_label(id: &str, kind: ShapeKind, visits: Option<&Vec<Visit>>) -> String {
    let base = node_label(id, kind);
    let Some(last) = visits.and_then(|v| v.last()) else {
        return base;
    };
    let glyph = outcome_glyph(&last.outcome, last.cache_hit);
    let count_badge = match visits.map(|v| v.len()) {
        Some(n) if n > 1 => format!(" ×{n}"),
        _ => String::new(),
    };
    format!(
        "{base} {glyph}{count_badge}<br/>{}ms · {}tok",
        last.duration_ms, last.tokens
    )
}

/// The outcome `classDef` name for a visit (D-21, 28-UI-SPEC.md "Outcome
/// color set"): a cache hit always colors `outcomeCacheHit` regardless of
/// the underlying `NodeOutcomeKind` (a cache hit's `outcome` is always a
/// `Succeeded`-shaped value in practice, but the cache-hit flag is the more
/// specific, more useful signal to color by). `Ended` colors the same as
/// `Succeeded` -- both merged their `StateDelta` normally (Claude's
/// Discretion; `Ended`'s only difference from `Succeeded` is that it also
/// completed the run, not a distinct outcome quality). The wildcard arm
/// covers `NodeOutcomeKind`'s `#[non_exhaustive]` future variants with the
/// same `Succeeded` default.
fn outcome_class(outcome: &NodeOutcomeKind, cache_hit: bool) -> &'static str {
    if cache_hit {
        return "outcomeCacheHit";
    }
    match outcome {
        NodeOutcomeKind::Succeeded | NodeOutcomeKind::Ended => "outcomeSuccess",
        NodeOutcomeKind::Failed => "outcomeFailed",
        NodeOutcomeKind::Parleyed => "outcomeParleyed",
        NodeOutcomeKind::Skipped { .. } => "outcomeSkipped",
        _ => "outcomeSuccess",
    }
}

/// The outcome glyph for a visit (28-UI-SPEC.md "Outcome color set",
/// verbatim), mirroring [`outcome_class`]'s same cache-hit-first, `Ended`-as-
/// success, non-exhaustive-default rules.
fn outcome_glyph(outcome: &NodeOutcomeKind, cache_hit: bool) -> &'static str {
    if cache_hit {
        return "⚡";
    }
    match outcome {
        NodeOutcomeKind::Succeeded | NodeOutcomeKind::Ended => "✓",
        NodeOutcomeKind::Failed => "✗",
        NodeOutcomeKind::Parleyed => "⏸",
        NodeOutcomeKind::Skipped { .. } => "⊘",
        _ => "✓",
    }
}

/// One `classDef` per outcome (28-UI-SPEC.md "Outcome color set", light
/// fill/stroke/text hex values, verbatim -- the same single-set precedent
/// [`write_class_defs`] follows, no dark variant baked into the exported
/// text itself).
fn write_outcome_class_defs(out: &mut String) {
    out.push_str("classDef outcomeSuccess fill:#dcfce7,stroke:#16a34a,color:#14532d;\n");
    out.push_str("classDef outcomeFailed fill:#fee2e2,stroke:#dc2626,color:#7f1d1d;\n");
    out.push_str("classDef outcomeParleyed fill:#fef3c7,stroke:#d97706,color:#78350f;\n");
    out.push_str("classDef outcomeSkipped fill:#f1f5f9,stroke:#94a3b8,color:#475569;\n");
    out.push_str("classDef outcomeCacheHit fill:#e0e7ff,stroke:#4f46e5,color:#312e81;\n");
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
