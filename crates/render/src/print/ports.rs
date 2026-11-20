//! The Print graph's port types and wire validation (principia_print_graph.md §4.2; R-486): every port is typed, a
//! wire between mismatched types is refused naming both ports and their types, and an unconnected input takes the
//! default its node type declares (the lab's `defaults`, such as `{ ground: "paper", field: "white" }`).

use std::collections::BTreeMap;
use std::fmt;

use super::graph::{Expanded, ExpandedNode, Wire, OUTPUT};
use super::registry::Registry;

/// A port's type (§4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PortType {
    /// Linear RGB and alpha, f16: `render` and every medium.
    Image,
    /// One channel, `[0, 1]` unless stated: brightness, fringe density, noise, a payload field.
    Scalar,
    /// A unit tangent field and its coherence: `flow`.
    Direction,
    /// Surface colour plus tooth height: `paper`, `ground`.
    Ground,
    /// One channel, `{0, 1}`: thresholds.
    Mask,
}

impl PortType {
    /// Every type, in §4.2's order.
    pub const ALL: [PortType; 5] = [
        PortType::Image,
        PortType::Scalar,
        PortType::Direction,
        PortType::Ground,
        PortType::Mask,
    ];

    /// The type's name, as a registry entry writes it.
    pub const fn name(self) -> &'static str {
        match self {
            PortType::Image => "image",
            PortType::Scalar => "scalar",
            PortType::Direction => "direction",
            PortType::Ground => "ground",
            PortType::Mask => "mask",
        }
    }

    /// The type named `name`.
    pub fn parse(name: &str) -> Option<PortType> {
        PortType::ALL.into_iter().find(|t| t.name() == name)
    }
}

impl fmt::Display for PortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What an unconnected input reads (the lab's `defaults`, and its rule for an input without one: the picture for
/// `in`, `a`, `colour` and `render`, black otherwise).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InputDefault {
    /// The picture coming in: the render, or, inside a sub-graph, its preset node's input.
    Render,
    /// The lab's default sheet of paper.
    Paper,
    /// White.
    White,
    /// Black.
    Black,
}

impl InputDefault {
    /// Every default.
    pub const ALL: [InputDefault; 4] = [
        InputDefault::Render,
        InputDefault::Paper,
        InputDefault::White,
        InputDefault::Black,
    ];

    /// The default's name, as a registry entry writes it.
    pub const fn name(self) -> &'static str {
        match self {
            InputDefault::Render => "render",
            InputDefault::Paper => "paper",
            InputDefault::White => "white",
            InputDefault::Black => "black",
        }
    }

    /// The default named `name`.
    pub fn parse(name: &str) -> Option<InputDefault> {
        InputDefault::ALL.into_iter().find(|d| d.name() == name)
    }
}

/// What one input of a node reads: a wire, its port named, or its declared default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feed {
    Wire { node: String, port: String },
    Default(InputDefault),
}

/// A validated graph's inputs: for each node, each of its type's inputs, in declaration order, with what it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wiring {
    pub inputs: BTreeMap<String, Vec<(String, Feed)>>,
}

/// Why a graph's wiring was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortError(pub String);

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PortError {}

/// `graph`'s wiring against `registry`, or why not: a node of a type that is not registered, an input its type does
/// not declare, a wire from a port its source does not have, a wire between mismatched types (naming both ports and
/// both types), or an `out` that is not an `output` node.
pub fn validate(graph: &Expanded, registry: &Registry) -> Result<Wiring, PortError> {
    let refuse = |why: String| Err(PortError(why));
    let mut inputs = BTreeMap::new();
    for (id, node) in &graph.nodes {
        let Some(entry) = registry.get(&node.ty) else {
            return refuse(unknown(id, &node.ty));
        };
        if let Some(port) = node
            .inputs
            .keys()
            .find(|p| !entry.inputs.iter().any(|i| &i.name == *p))
        {
            return refuse(format!("node `{id}` ({}) has no input `{port}`", node.ty));
        }
        let mut feeds = Vec::with_capacity(entry.inputs.len());
        for input in &entry.inputs {
            let wire = match (node.inputs.get(&input.name), &node.entry) {
                (Some(wire), _) => Some(wire),
                (None, Some(entry)) if input.default == InputDefault::Render => Some(entry),
                (None, _) => None,
            };
            let feed = match wire {
                Some(wire) => {
                    let to = (id.as_str(), node, input.name.as_str(), input.ty);
                    wired(graph, registry, wire, to)?
                }
                None => Feed::Default(input.default),
            };
            feeds.push((input.name.clone(), feed));
        }
        inputs.insert(id.clone(), feeds);
    }
    match graph.nodes.get(&graph.out) {
        Some(out) if out.ty == OUTPUT => Ok(Wiring { inputs }),
        Some(out) => refuse(format!(
            "the graph's `out`, `{}`, is a `{}`, not an `{OUTPUT}` node",
            graph.out, out.ty
        )),
        None => refuse(format!("the graph's `out`, `{}`, is no node", graph.out)),
    }
}

/// The refusal of node `id`, of the type `ty` no entry registers.
fn unknown(id: &str, ty: &str) -> String {
    format!("node `{id}` names the node type `{ty}`, which is not registered")
}

/// `wire`, into input `to` = (node id, node, port, type), checked: its source exists, has the port, and the port's
/// type is the input's.
fn wired(
    graph: &Expanded,
    registry: &Registry,
    wire: &Wire,
    to: (&str, &ExpandedNode, &str, PortType),
) -> Result<Feed, PortError> {
    let (id, node, port, want) = to;
    let Some(source) = graph.nodes.get(&wire.node) else {
        return Err(PortError(format!(
            "node `{id}`: input `{port}` reads `{}`, which is no node",
            wire.node
        )));
    };
    let Some(entry) = registry.get(&source.ty) else {
        return Err(PortError(unknown(&wire.node, &source.ty)));
    };
    let output = match &wire.port {
        Some(p) => entry.outputs.iter().find(|o| &o.name == p),
        None => entry.outputs.first(),
    };
    let Some(output) = output else {
        return Err(PortError(format!(
            "node `{id}` ({}): input `{port}` reads `{}.{}`, an output `{}` does not have",
            node.ty,
            wire.node,
            wire.port.as_deref().unwrap_or("out"),
            source.ty
        )));
    };
    if output.ty != want {
        return Err(PortError(format!(
            "the wire `{}.{}` ({}) → `{id}.{port}` ({}) joins mismatched types: a `{}` output cannot feed a `{}` \
             input",
            wire.node, output.name, output.ty, want, output.ty, want
        )));
    }
    Ok(Feed::Wire {
        node: wire.node.clone(),
        port: output.name.clone(),
    })
}
