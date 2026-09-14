//! Graph export (OBS-03, OBS-FR-08, D-18): renders a graph -- from a
//! [`WarGraphDoc`](crate::engine::graph_doc::WarGraphDoc) or a compiled
//! [`WarGraph`](crate::engine::graph::WarGraph) -- as a Mermaid flowchart or
//! a Graphviz digraph, for a human to read.
//!
//! Both exporters are pure functions over one shared, rendering-agnostic
//! type: [`GraphShape`]. Build a shape with [`GraphShape::from_doc`] or
//! [`GraphShape::from_graph`], then render it with [`to_mermaid`] or
//! [`to_dot`]. [`ExecutionOverlay`] (D-21, 28-10) layers execution history
//! -- from Waypoint history so far, a persisted-trace source and the
//! observed-only fallback (D-22) land in a later 28-10 task -- onto a
//! `GraphShape`, rendered by [`to_mermaid_overlay`]. The CLI `graph
//! export`/`run export` commands (28-13) and the inspector page (28-15) all
//! render through these same functions.
//!
//! # Example
//!
//! ```
//! use paladin_battalion::engine::export::{GraphShape, to_dot, to_mermaid};
//! use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
//! use paladin_battalion::engine::InputMapping;
//! use paladin_core::base::entity::node::Node;
//! use paladin_core::platform::container::battlefield::{
//!     BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
//! };
//! use paladin_core::platform::container::paladin::PaladinData;
//! use paladin_core::platform::container::waypoint::NodeId;
//!
//! let field = FieldName::new("out").expect("valid field name");
//! let schema = BattlefieldSchema::new(vec![FieldSpec::new(
//!     field.clone(),
//!     DispatchRule::LastWrite,
//!     None,
//!     false,
//! )]);
//! let mut graph = WarGraph::new(schema, EngineLimits::default());
//! let paladin = Node::new(PaladinData::default(), Some("greeter".to_string()));
//! let id = NodeId::new("greeter");
//! graph.add_node(
//!     id.clone(),
//!     NodeSpec::paladin(paladin, InputMapping::new("hi"), field),
//! );
//! graph.add_entry(id);
//!
//! let shape = GraphShape::from_graph(&graph);
//! assert!(to_mermaid(&shape).starts_with("flowchart TD"));
//! assert!(to_dot(&shape).starts_with("digraph"));
//! ```

pub mod dot;
pub mod mermaid;
pub mod overlay;
pub mod shape;

pub use dot::to_dot;
pub use mermaid::{to_mermaid, to_mermaid_overlay};
pub use overlay::{ExecutionOverlay, OverlaySource, Visit};
pub use shape::{GraphShape, ShapeEdge, ShapeKind, ShapeNode};
