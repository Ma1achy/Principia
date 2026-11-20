//! The stain editor's edits (render_gui_spec Part II §4, §6, §7; gui_state_contract §5; R-52): add, delete,
//! duplicate, wire, unwire and reorder, picking a node's occupant and setting its params and ramp, each a typed
//! [`StainEdit`]. [`apply`] checks one against a graph and, accepted, returns the one [`SetField`] that writes the
//! edited graph, `RenderField::StainGraph`, undoable (R-52, R-69); refused, it returns the reason ([`Refusal`]) and
//! the graph is as it was.
//!
//! - `OUT` and the combiner are fixed singletons: adding, deleting or duplicating one is refused (§4, §7).
//! - A wire attaches only where the out-port's type is the in-port's (`field` → `field`, `vec3` → `vec3`, `f32` →
//!   `f32`); the `field` subtype is not compared (§6). A wire that would close a cycle is refused (§4), as is one that
//!   breaks the backbone `… → combiner → (post)* → OUT` (§4).
//! - An in-port takes one wire: a wire dropped on an occupied in-port replaces the one there (last write wins); an
//!   out-port feeds any number (§6).
//! - An edit that leaves the graph as it is (a wire dropped where it already is, an empty in-port unwired, a post moved
//!   to its own place) is refused as [`Refusal::NoChange`], so every accepted edit changes the graph.

use std::fmt;

use render::assemble;

use crate::contract::set_field::{Edit, RenderField, SetField};
use crate::contract::stain::graph::{GraphError, NodeId, NodeKind, Occupant, Ramp, StainGraph};
use crate::contract::stain::ports::PortType;

/// One edit of the stain graph.
#[derive(Clone, Debug, PartialEq)]
pub enum StainEdit {
    /// Add a node of `kind` holding `occupant` (§7, "Add node"), its id one past the largest.
    Add { kind: NodeKind, occupant: Occupant },
    /// Delete node `node` and its wires (§7, "Delete node"); its consumers' in-ports dangle and read as None (§13).
    Delete { node: NodeId },
    /// Add a copy of node `node`, its kind, occupant, params and ramp, without its wires, its id one past the largest.
    Duplicate { node: NodeId },
    /// Wire `from`'s out-port into `to`'s in-port `port` (§7, "Wire (create)").
    Wire { from: NodeId, to: NodeId, port: u32 },
    /// Remove the wire into `to`'s in-port `port` (§7, "Wire (delete)").
    Unwire { to: NodeId, port: u32 },
    /// Move post `post` to `position` in the live post chain, 0 nearest the combiner (§4's `(post)*`).
    Reorder { post: NodeId, position: u32 },
    /// Pick node `node`'s occupant (gui_state_contract §5, mode 1: "Selecting writes an id. One field edit.").
    Occupant { node: NodeId, occupant: Occupant },
    /// Set node `node`'s uniform `name` to `value`.
    Param {
        node: NodeId,
        name: String,
        value: Vec<f64>,
    },
    /// Set a `colour` node's custom-stop ramp, or remove it with `None` (colour_composition §9.4).
    Ramp { node: NodeId, ramp: Option<Ramp> },
}

/// What a refused add, delete or duplicate asked of a fixed singleton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Add,
    Delete,
    Duplicate,
}

/// Why an edit was refused; the graph is as it was.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// `OUT` and the combiner are fixed singletons: never added, deleted or duplicated (§4).
    FixedSingleton { kind: NodeKind, action: Action },
    /// The graph has no node `0`.
    NoNode(NodeId),
    /// Node `node` has `ports` in-port(s), so no in-port `port`.
    NoPort {
        node: NodeId,
        port: u32,
        ports: usize,
    },
    /// The out-port's type (none, for `OUT`) is not the in-port's (§6): the wire snaps back.
    TypeMismatch {
        from: Option<PortType>,
        to: PortType,
    },
    /// `to` already feeds `from`, so the wire would close a cycle (§4).
    Cycle { from: NodeId, to: NodeId },
    /// The wire breaks the backbone `… → combiner → (post)* → OUT` (§4).
    Backbone(String),
    /// Node `0` is not in the live post chain, so it is not reordered.
    NotInChain(NodeId),
    /// The live post chain holds `chain` post(s), so no `position`.
    NoPosition { position: u32, chain: usize },
    /// The edit leaves the graph as it is.
    NoChange,
    /// The graph refuses the result for another reason: an occupant its slot does not take, a param no uniform
    /// admits, a ramp off a `colour` node, or a post chain past its bound.
    Graph(GraphError),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::FixedSingleton { kind, action } => {
                let done = match action {
                    Action::Add => "added",
                    Action::Delete => "deleted",
                    Action::Duplicate => "duplicated",
                };
                let kind = match kind {
                    NodeKind::Out => "OUT",
                    other => assemble::Kind::from(*other).name(),
                };
                write!(f, "the {kind} is a fixed singleton: it is never {done}")
            }
            Refusal::NoNode(id) => write!(f, "no node {}", id.0),
            Refusal::NoPort { node, port, ports } => {
                write!(f, "node {} has {ports} in-port(s); no port {port}", node.0)
            }
            Refusal::TypeMismatch { from, to } => {
                let from = from.map_or("a node with no out-port".to_owned(), |t| {
                    format!("a {} out-port", type_name(t))
                });
                write!(
                    f,
                    "{from} cannot feed an in-port of type {}: the types differ",
                    type_name(*to)
                )
            }
            Refusal::Cycle { from, to } => write!(
                f,
                "node {} already feeds node {}: the wire would close a cycle",
                to.0, from.0
            ),
            Refusal::Backbone(why) => f.write_str(why),
            Refusal::NotInChain(id) => write!(f, "node {} is not in the live post chain", id.0),
            Refusal::NoPosition { position, chain } => write!(
                f,
                "the live post chain holds {chain} post(s); no position {position}"
            ),
            Refusal::NoChange => f.write_str("the edit leaves the graph as it is"),
            Refusal::Graph(e) => write!(f, "{e}"),
        }
    }
}

/// A wire-carried type as render_gui_spec Part II §3 writes it.
fn type_name(t: PortType) -> &'static str {
    match t {
        PortType::Field => "field",
        PortType::Vec3 => "vec3",
        PortType::F32 => "f32",
    }
}

impl std::error::Error for Refusal {}

impl From<GraphError> for Refusal {
    fn from(e: GraphError) -> Refusal {
        Refusal::Graph(e)
    }
}

/// Checks `edit` against `graph`: accepted, the one `SetField` writing the edited graph, undoable (R-52); refused,
/// why.
pub fn apply(graph: &StainGraph, edit: &StainEdit) -> Result<SetField, Refusal> {
    let mut next = graph.clone();
    match edit {
        StainEdit::Add { kind, occupant } => {
            singleton(*kind, Action::Add)?;
            next.add(*kind, occupant.clone())?;
        }
        StainEdit::Delete { node } => {
            singleton(kind_of(graph, *node)?, Action::Delete)?;
            next.remove(*node)?;
        }
        StainEdit::Duplicate { node } => {
            singleton(kind_of(graph, *node)?, Action::Duplicate)?;
            next.duplicate(*node)?;
        }
        StainEdit::Wire { from, to, port } => {
            wire(graph, *from, *to, *port)?;
            next.connect(*from, *to, *port as usize)?;
        }
        StainEdit::Unwire { to, port } => {
            kind_of(graph, *to)?;
            next.disconnect(*to, *port as usize)?;
        }
        StainEdit::Reorder { post, position } => {
            let chain = graph.chain();
            if !chain.contains(post) {
                return Err(Refusal::NotInChain(*post));
            }
            if *position as usize >= chain.len() {
                return Err(Refusal::NoPosition {
                    position: *position,
                    chain: chain.len(),
                });
            }
            next.reorder(*post, *position as usize)?;
        }
        StainEdit::Occupant { node, occupant } => {
            kind_of(graph, *node)?;
            next.set_occupant(*node, occupant.clone())?;
        }
        StainEdit::Param { node, name, value } => {
            kind_of(graph, *node)?;
            next.set_param(*node, name, value.clone())?;
        }
        StainEdit::Ramp { node, ramp } => {
            kind_of(graph, *node)?;
            next.set_ramp(*node, ramp.clone())?;
        }
    }
    if next == *graph {
        return Err(Refusal::NoChange);
    }
    Ok(SetField {
        edit: Edit::Render(RenderField::StainGraph(next)),
        no_history: false,
    })
}

/// Refuses `action` on a fixed singleton.
fn singleton(kind: NodeKind, action: Action) -> Result<(), Refusal> {
    match kind {
        NodeKind::Combiner | NodeKind::Out => Err(Refusal::FixedSingleton { kind, action }),
        _ => Ok(()),
    }
}

/// Node `id`'s kind, or [`Refusal::NoNode`].
fn kind_of(graph: &StainGraph, id: NodeId) -> Result<NodeKind, Refusal> {
    graph.node(id).map(|n| n.kind).ok_or(Refusal::NoNode(id))
}

/// Checks a wire `from` → `to`.`port` by the rules of §4 and §6, in turn: both nodes exist, the port does, the types
/// match, no cycle closes and the backbone holds.
fn wire(graph: &StainGraph, from: NodeId, to: NodeId, port: u32) -> Result<(), Refusal> {
    let source = kind_of(graph, from)?;
    let target = kind_of(graph, to)?;
    let ports = graph.in_ports(to)?;
    let Some(&ty) = ports.get(port as usize) else {
        return Err(Refusal::NoPort {
            node: to,
            port,
            ports: ports.len(),
        });
    };
    if source.output() != Some(ty) {
        return Err(Refusal::TypeMismatch {
            from: source.output(),
            to: ty,
        });
    }
    if from == to || graph.feeds(to, from) {
        return Err(Refusal::Cycle { from, to });
    }
    assemble::check_wire(source.into(), target.into(), ty)
        .map_err(|e| Refusal::Backbone(e.to_string()))
}
