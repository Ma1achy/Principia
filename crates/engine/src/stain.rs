//! The stain graph (gui_state_contract §5; render_gui_spec Part II §3, §4, §6, §13; R-64): a free, typed node graph on
//! a fixed backbone. Nodes, their typed ports, the wires and the per-node params are the stain's data, and serialise
//! as one value; the canonical form ([`Canonical`]) is what the fragment key and the render key hash (lowering
//! contract Part 5; render contract Part 3; REQ-RENDER-075).
//!
//! Every construction is checked, an edit and a deserialised graph alike ([`StainGraph::check`]): `combiner` and
//! `OUT` are singletons, made by [`StainGraph::new`] and never added or deleted; each wire's out-port type is its
//! in-port's and it keeps the backbone (`render::assemble::check_wire`); an in-port takes one wire, a new one
//! replacing the old (§6); the graph is acyclic; each param is a value of its occupant's `uniformSchema`; and the
//! graph lowers ([`StainGraph::lower`]), so the live post chain keeps its bound. A refused edit leaves the graph as it
//! was.
//!
//! The port rules, the occupants' declarations and the lowering are the render crate's, the fragment side's
//! (systems_architecture §7.1: `engine` depends on `render`, never the reverse).
//!
//! [`Canonical`]: crate::stain::Canonical
//! [`StainGraph::check`]: crate::stain::StainGraph::check
//! [`StainGraph::new`]: crate::stain::StainGraph::new
//! [`StainGraph::lower`]: crate::stain::StainGraph::lower

use std::collections::BTreeMap;
use std::fmt;

use render::assemble::{self, AssembleError, Kind, PortType, Stain};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::contract::canonical;

/// A node's id: stable across edits, and not part of the canonical form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// A node kind (render_gui_spec Part II §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Source,
    Colour,
    Brightness,
    Combiner,
    Post,
    Out,
}

impl From<NodeKind> for Kind {
    fn from(k: NodeKind) -> Kind {
        match k {
            NodeKind::Source => Kind::Source,
            NodeKind::Colour => Kind::Colour,
            NodeKind::Brightness => Kind::Brightness,
            NodeKind::Combiner => Kind::Combiner,
            NodeKind::Post => Kind::Post,
            NodeKind::Out => Kind::Out,
        }
    }
}

/// What occupies a node (render contract Part 2; gui_state_contract §5): None, a built-in source's field, a built-in
/// occupant's id, or a custom occupant's WGSL.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Occupant {
    None,
    Field(String),
    Builtin(String),
    Custom(String),
}

impl From<&Occupant> for assemble::Occupant {
    fn from(o: &Occupant) -> assemble::Occupant {
        match o {
            Occupant::None => assemble::Occupant::None,
            Occupant::Field(f) => assemble::Occupant::Field(f.clone()),
            Occupant::Builtin(id) => assemble::Occupant::BuiltIn(id.clone()),
            Occupant::Custom(text) => assemble::Occupant::Custom(text.clone()),
        }
    }
}

/// One node: its id, kind, occupant and params, each a uniform's value by the uniform's name, one number per
/// component.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub occupant: Occupant,
    #[serde(default)]
    pub params: BTreeMap<String, Vec<f64>>,
}

/// One wire: from a node's out-port into another's in-port, by its position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub from: NodeId,
    pub to: NodeId,
    pub port: usize,
}

/// Why a graph or an edit was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphError(pub String);

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GraphError {}

impl From<AssembleError> for GraphError {
    fn from(e: AssembleError) -> GraphError {
        GraphError(e.to_string())
    }
}

/// The stain graph. Serialised as its nodes and wires, in id order; deserialising checks it as an edit does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Parts", into = "Parts")]
pub struct StainGraph {
    nodes: BTreeMap<NodeId, Node>,
    /// Each in-port's wire, keyed by `(to, port)`, its value the node feeding it.
    wires: BTreeMap<(NodeId, usize), NodeId>,
}

/// The serialised form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    nodes: Vec<Node>,
    wires: Vec<Wire>,
}

impl TryFrom<Parts> for StainGraph {
    type Error = GraphError;

    fn try_from(p: Parts) -> Result<StainGraph, GraphError> {
        let mut g = StainGraph {
            nodes: BTreeMap::new(),
            wires: BTreeMap::new(),
        };
        for n in p.nodes {
            if g.nodes.insert(n.id, n.clone()).is_some() {
                return Err(GraphError(format!("node id {} is used twice", n.id.0)));
            }
        }
        for w in p.wires {
            if g.wires.insert((w.to, w.port), w.from).is_some() {
                return Err(GraphError(format!(
                    "node {}'s in-port {} has two wires; an in-port takes one",
                    w.to.0, w.port
                )));
            }
        }
        g.check()?;
        Ok(g)
    }
}

impl From<StainGraph> for Parts {
    fn from(g: StainGraph) -> Parts {
        Parts {
            wires: g.wires(),
            nodes: g.nodes.into_values().collect(),
        }
    }
}

impl Default for StainGraph {
    fn default() -> StainGraph {
        StainGraph::new()
    }
}

impl StainGraph {
    /// The backbone alone: the combiner (id 0), the built-in pass-through (M1's), wired to `OUT` (id 1).
    pub fn new() -> StainGraph {
        let node = |id, kind, occupant| Node {
            id,
            kind,
            occupant,
            params: BTreeMap::new(),
        };
        let (combiner, out) = (NodeId(0), NodeId(1));
        StainGraph {
            nodes: BTreeMap::from([
                (
                    combiner,
                    node(
                        combiner,
                        NodeKind::Combiner,
                        Occupant::Builtin("pass_through".into()),
                    ),
                ),
                (out, node(out, NodeKind::Out, Occupant::None)),
            ]),
            wires: BTreeMap::from([((out, 0), combiner)]),
        }
    }

    /// The node `id`.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The wires, in `(to, port)` order.
    pub fn wires(&self) -> Vec<Wire> {
        self.wires
            .iter()
            .map(|(&(to, port), &from)| Wire { from, to, port })
            .collect()
    }

    /// Adds a node of `kind` with `occupant`, its id one past the largest in the graph; a combiner or an `OUT` is
    /// refused.
    pub fn add(&mut self, kind: NodeKind, occupant: Occupant) -> Result<NodeId, GraphError> {
        if matches!(kind, NodeKind::Combiner | NodeKind::Out) {
            return Err(GraphError(format!(
                "the {kind:?} is a fixed singleton: it is never added"
            )));
        }
        let id = NodeId(self.nodes.keys().last().map_or(0, |n| n.0 + 1));
        self.edit(|g| {
            g.nodes.insert(
                id,
                Node {
                    id,
                    kind,
                    occupant,
                    params: BTreeMap::new(),
                },
            );
            Ok(())
        })?;
        Ok(id)
    }

    /// Removes node `id` and its wires; the combiner and `OUT` are refused.
    pub fn remove(&mut self, id: NodeId) -> Result<(), GraphError> {
        if self
            .nodes
            .get(&id)
            .is_some_and(|n| matches!(n.kind, NodeKind::Combiner | NodeKind::Out))
        {
            return Err(GraphError(
                "the combiner and OUT are fixed singletons: never deleted".into(),
            ));
        }
        self.edit(|g| {
            g.nodes.remove(&id).ok_or_else(|| no_node(id))?;
            g.wires.retain(|&(to, _), from| to != id && *from != id);
            Ok(())
        })
    }

    /// Wires `from`'s out-port into `to`'s in-port `port`, replacing the wire there (last write wins, §6).
    pub fn connect(&mut self, from: NodeId, to: NodeId, port: usize) -> Result<(), GraphError> {
        self.edit(|g| {
            g.wires.insert((to, port), from);
            Ok(())
        })
    }

    /// Removes the wire into `to`'s in-port `port`, if any; the input then reads as absent (§13).
    pub fn disconnect(&mut self, to: NodeId, port: usize) -> Result<(), GraphError> {
        self.edit(|g| {
            g.wires.remove(&(to, port));
            Ok(())
        })
    }

    /// Sets node `id`'s occupant; the wires into in-ports it no longer has and the params its schema no longer
    /// declares are dropped.
    pub fn set_occupant(&mut self, id: NodeId, occupant: Occupant) -> Result<(), GraphError> {
        self.edit(|g| {
            let n = g.nodes.get_mut(&id).ok_or_else(|| no_node(id))?;
            let d = assemble::declaration(n.kind.into(), &(&occupant).into())?;
            let ports = assemble::in_ports(n.kind.into(), &d).len();
            n.occupant = occupant;
            n.params
                .retain(|name, _| d.uniforms.iter().any(|u| &u.name == name));
            g.wires.retain(|&(to, port), _| to != id || port < ports);
            Ok(())
        })
    }

    /// Sets node `id`'s uniform `name` to `value`.
    pub fn set_param(&mut self, id: NodeId, name: &str, value: Vec<f64>) -> Result<(), GraphError> {
        self.edit(|g| {
            let n = g.nodes.get_mut(&id).ok_or_else(|| no_node(id))?;
            n.params.insert(name.to_owned(), value);
            Ok(())
        })
    }

    /// Applies `f` to a copy and keeps it only if it passes [`StainGraph::check`].
    fn edit(
        &mut self,
        f: impl FnOnce(&mut StainGraph) -> Result<(), GraphError>,
    ) -> Result<(), GraphError> {
        let mut next = self.clone();
        f(&mut next)?;
        next.check()?;
        *self = next;
        Ok(())
    }

    /// Whether the graph is one: each node's occupant one its slot takes, each param a value of its schema; each
    /// wire into an in-port its target has, its type and the backbone kept; acyclic; and it lowers, so one combiner and
    /// one `OUT` and the post chain within its bound ([`Stain::new`]).
    pub fn check(&self) -> Result<(), GraphError> {
        for n in self.nodes.values() {
            let d = assemble::declaration(n.kind.into(), &(&n.occupant).into())?;
            for (name, value) in &n.params {
                let ok = d
                    .uniforms
                    .iter()
                    .any(|u| &u.name == name && u.admits(value));
                if !ok {
                    return Err(GraphError(format!(
                        "node {}'s param `{name}` = {value:?} is no value of a uniform its occupant declares",
                        n.id.0
                    )));
                }
            }
        }
        for (&(to, port), &from) in &self.wires {
            let (Some(f), Some(t)) = (self.nodes.get(&from), self.nodes.get(&to)) else {
                return Err(GraphError(format!(
                    "a wire {} → {} names no node",
                    from.0, to.0
                )));
            };
            let ports = self.ports(t)?;
            let Some(&ty) = ports.get(port) else {
                return Err(GraphError(format!(
                    "node {} has {} in-port(s); no port {port}",
                    to.0,
                    ports.len()
                )));
            };
            assemble::check_wire(f.kind.into(), t.kind.into(), ty)?;
        }
        self.lower()?;
        Ok(())
    }

    fn ports(&self, n: &Node) -> Result<Vec<PortType>, GraphError> {
        let d = assemble::declaration(n.kind.into(), &(&n.occupant).into())?;
        Ok(assemble::in_ports(n.kind.into(), &d))
    }

    /// The node ids in an order every wire runs forward in, each node after the nodes feeding it, ties in id order;
    /// refused on a cycle.
    fn order(&self) -> Result<Vec<NodeId>, GraphError> {
        // `true` on the walk, `false` done.
        fn visit(
            g: &StainGraph,
            id: NodeId,
            state: &mut BTreeMap<NodeId, bool>,
            order: &mut Vec<NodeId>,
        ) -> Result<(), GraphError> {
            match state.get(&id) {
                Some(true) => {
                    return Err(GraphError(format!(
                        "node {} feeds itself: the stain graph is acyclic",
                        id.0
                    )))
                }
                Some(false) => return Ok(()),
                None => {}
            }
            state.insert(id, true);
            for (_, &from) in g.wires.range((id, 0)..=(id, usize::MAX)) {
                visit(g, from, state, order)?;
            }
            state.insert(id, false);
            order.push(id);
            Ok(())
        }
        let (mut state, mut order) = (BTreeMap::new(), Vec::new());
        for &id in self.nodes.keys() {
            visit(self, id, &mut state, &mut order)?;
        }
        Ok(order)
    }

    /// The graph as the assembler takes it, every node in [`StainGraph::order`], each in-port its feeder's position,
    /// with that order.
    fn lowered(&self) -> Result<(Vec<NodeId>, Stain), GraphError> {
        let order = self.order()?;
        let position = |id: NodeId| order.iter().position(|&o| o == id);
        let mut nodes = Vec::with_capacity(order.len());
        for &id in &order {
            let n = &self.nodes[&id];
            let inputs = (0..self.ports(n)?.len())
                .map(|k| self.wires.get(&(id, k)).and_then(|&f| position(f)))
                .collect();
            nodes.push(assemble::Node {
                kind: n.kind.into(),
                occupant: (&n.occupant).into(),
                inputs,
            });
        }
        Ok((order, Stain::new(nodes)?))
    }

    /// The graph as the assembler takes it ([`Stain`]); it assembles its canonical form.
    pub fn lower(&self) -> Result<Stain, GraphError> {
        Ok(self.lowered()?.1)
    }

    /// The canonical form (lowering contract Part 5; render contract Part 3): the assembler's ([`Stain::canonical`]),
    /// each node with its params.
    pub fn canonical(&self) -> Result<Canonical, GraphError> {
        let (order, stain) = self.lowered()?;
        let form = stain.canonical();
        let nodes = form
            .order()
            .iter()
            .zip(form.stain().nodes())
            .enumerate()
            .map(|(j, (&i, lowered))| {
                let n = &self.nodes[&order[i]];
                // The schema the form's stain parsed at construction, so it is the node's, never a fallback.
                let schema = &form.stain().declaration(j).uniforms;
                let params = schema
                    .iter()
                    .map(|u| {
                        let value = n.params.get(&u.name).unwrap_or(&u.default).clone();
                        (u.name.clone(), value)
                    })
                    .collect();
                CanonicalNode {
                    kind: n.kind,
                    occupant: n.occupant.clone(),
                    inputs: lowered.inputs.clone(),
                    params,
                }
            })
            .collect();
        Ok(Canonical { nodes, form })
    }
}

fn no_node(id: NodeId) -> GraphError {
    GraphError(format!("no node {}", id.0))
}

/// One node of the canonical form: its kind, occupant, the position of the node feeding each in-port (`None` where
/// absent) and its uniforms' values, each set value or the schema's default.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalNode {
    pub kind: NodeKind,
    pub occupant: Occupant,
    pub inputs: Vec<Option<usize>>,
    pub params: BTreeMap<String, Vec<f64>>,
}

/// The canonical form of a stain graph (lowering contract Part 5; render contract Part 3; REQ-RENDER-075): the
/// assembler's form of its lowering ([`assemble::Canonical`]), its live nodes in canonical order, each wired by
/// position, and each node's params. Node ids and the order of construction are not in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Canonical {
    pub nodes: Vec<CanonicalNode>,
    form: assemble::Canonical,
}

impl Canonical {
    /// The form's text, JCS: the fragment key's input ([`assemble::Canonical::text`]). The params are not in it.
    pub fn text(&self) -> String {
        self.form.text()
    }

    /// The form's text with each node's `params`, its uniforms' values, in JCS (gui_state_contract §2, R-318): the
    /// render key's graph part.
    pub fn render_text(&self) -> String {
        let nodes: Vec<Value> = self
            .nodes
            .iter()
            .map(|n| {
                let occupant = match &n.occupant {
                    Occupant::None => Value::Null,
                    Occupant::Field(f) => json!({ "field": f }),
                    Occupant::Builtin(id) => json!({ "builtin": id }),
                    Occupant::Custom(text) => json!({ "custom": text }),
                };
                json!({
                    "kind": Kind::from(n.kind).name(),
                    "occupant": occupant,
                    "inputs": n.inputs,
                    "params": n.params,
                })
            })
            .collect();
        // Every number here is a position in the graph, far below 2^53, or a param, a finite f64 (each value is
        // admitted by its schema), so JCS holds each exactly and the serialisation cannot fail.
        canonical::json_to_string(&json!({ "nodes": nodes }))
            .expect("positions and finite params are JCS numbers")
    }

    /// The fragment key (lowering contract Part 5): the 64-bit FNV-1a hash of [`Canonical::text`]'s UTF-8 bytes.
    pub fn fragment_key(&self) -> u64 {
        self.form.fragment_key()
    }

    /// The render key's graph part (render contract Part 3): the 64-bit FNV-1a hash of [`Canonical::render_text`].
    pub fn render_key(&self) -> u64 {
        ledger::version::fnv1a64(self.render_text().as_bytes())
    }

    /// The form as the assembler's [`Stain`], in canonical order.
    pub fn stain(&self) -> &Stain {
        self.form.stain()
    }
}
