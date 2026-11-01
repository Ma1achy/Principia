//! The stain graph (gui_state_contract §5; render_gui_spec Part II §3, §4, §6, §13; R-64): a free, typed node graph on
//! a fixed backbone. Nodes, their typed ports ([`ports`](super::ports)), the wires and the per-node params, a `colour`
//! node's custom-stop ramp among them ([`Ramp`]), are the stain's data, and serialise with `RenderState` as one value;
//! the canonical form ([`Canonical`]) is what the fragment key and the render key hash (lowering contract Part 5;
//! render contract Part 3; REQ-RENDER-075).
//!
//! Every construction is checked, an edit and a deserialised graph alike ([`StainGraph::check`]): `combiner` and
//! `OUT` are singletons, made by [`StainGraph::new`] and never added or deleted; each wire's out-port type is its
//! in-port's and it keeps the backbone (`render::assemble::check_wire`); an in-port takes one wire, a new one
//! replacing the old (§6); the graph is acyclic; each param is a value of its occupant's `uniformSchema`, and only a
//! `colour` node has a ramp; and the graph lowers ([`StainGraph::lower`]), so the live post chain keeps its bound. A
//! refused edit leaves the graph as it was. The editor's edits, typed and each with its refusal's reason, are
//! [`edit`](super::edit)'s.
//!
//! The port rules, the occupants' declarations and the lowering are the render crate's, the fragment side's
//! (systems_architecture §7.1: `engine` depends on `render`, never the reverse).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use render::assemble::{self, AssembleError, Kind, Stain};

pub use render::assemble::{PortType, Uniform, UniformType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::contract::canonical;
use crate::contract::stain::ports::{Direction, Port, Ports, Subtype};
pub use crate::contract::stain::ramp::Ramp;

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

impl NodeKind {
    /// The type on the kind's out-port (render_gui_spec Part II §3.1); `OUT` has none.
    pub fn output(self) -> Option<PortType> {
        Kind::from(self).output()
    }
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
/// component, and, on a `colour` node, its custom-stop ramp where it has one (colour_composition §9.4), serialised as
/// `ramp` and left out where it has none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub occupant: Occupant,
    #[serde(default)]
    pub params: BTreeMap<String, Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramp: Option<Ramp>,
}

/// One wire: from a node's out-port into another's in-port, by its position: a u32, as the graph serialises with
/// `RenderState`, and no serialised type has a `usize` field (R-329).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub from: NodeId,
    pub to: NodeId,
    pub port: u32,
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
            if g.wires.insert((w.to, w.port as usize), w.from).is_some() {
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
            ramp: None,
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
            .map(|(&(to, port), &from)| Wire {
                from,
                to,
                // An in-port's position is below the assembler's few inputs (`assemble::MAX_INPUTS`), so it fits.
                port: u32::try_from(port).expect("an in-port's position fits a u32"),
            })
            .collect()
    }

    /// The nodes, in id order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Node `id`'s in-ports, in order, each its type (render_gui_spec Part II §3.1): what an editor draws on its left
    /// edge.
    pub fn in_ports(&self, id: NodeId) -> Result<Vec<PortType>, GraphError> {
        let n = self.nodes.get(&id).ok_or_else(|| no_node(id))?;
        self.ports(n)
    }

    /// The uniforms node `id`'s occupant declares (its `uniformSchema`, lowering contract Part 3), each with its
    /// default and range: what a node's params are values of.
    pub fn uniforms(&self, id: NodeId) -> Result<Vec<Uniform>, GraphError> {
        let n = self.nodes.get(&id).ok_or_else(|| no_node(id))?;
        Ok(assemble::declaration(n.kind.into(), &(&n.occupant).into())?.uniforms)
    }

    /// Adds a node of `kind` with `occupant`, its id one past the largest in the graph; a combiner or an `OUT` is
    /// refused.
    pub fn add(&mut self, kind: NodeKind, occupant: Occupant) -> Result<NodeId, GraphError> {
        if matches!(kind, NodeKind::Combiner | NodeKind::Out) {
            return Err(GraphError(format!(
                "the {kind:?} is a fixed singleton: it is never added"
            )));
        }
        let id = self.next_id();
        self.edit(|g| {
            g.nodes.insert(
                id,
                Node {
                    id,
                    kind,
                    occupant,
                    params: BTreeMap::new(),
                    ramp: None,
                },
            );
            Ok(())
        })?;
        Ok(id)
    }

    /// Adds a copy of node `id`, its kind, occupant, params and ramp, without its wires, its id one past the largest
    /// in the graph; a combiner or an `OUT` is refused.
    pub fn duplicate(&mut self, id: NodeId) -> Result<NodeId, GraphError> {
        let n = self.nodes.get(&id).ok_or_else(|| no_node(id))?;
        if matches!(n.kind, NodeKind::Combiner | NodeKind::Out) {
            return Err(GraphError(format!(
                "the {:?} is a fixed singleton: it is never duplicated",
                n.kind
            )));
        }
        let copy = self.next_id();
        let mut node = n.clone();
        node.id = copy;
        self.edit(|g| {
            g.nodes.insert(copy, node);
            Ok(())
        })?;
        Ok(copy)
    }

    /// The id one past the largest in the graph.
    fn next_id(&self) -> NodeId {
        NodeId(self.nodes.keys().last().map_or(0, |n| n.0 + 1))
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

    /// Sets node `id`'s custom-stop ramp, or removes it with `None`; a ramp on any node but a `colour` is refused.
    pub fn set_ramp(&mut self, id: NodeId, ramp: Option<Ramp>) -> Result<(), GraphError> {
        self.edit(|g| {
            g.nodes.get_mut(&id).ok_or_else(|| no_node(id))?.ramp = ramp;
            Ok(())
        })
    }

    /// The live post chain (render_gui_spec Part II §4, `… → combiner → (post)* → OUT`): the posts `OUT` reads
    /// through, each feeding the next by its colour in-port, the one nearest the combiner first.
    pub fn chain(&self) -> Vec<NodeId> {
        let mut chain = Vec::new();
        let mut at = self.wires.get(&(self.out(), 0)).copied();
        while let Some(id) = at.filter(|id| self.nodes[id].kind == NodeKind::Post) {
            chain.push(id);
            at = self.wires.get(&(id, 0)).copied();
        }
        chain.reverse();
        chain
    }

    /// Moves post `post` to `position` in the live post chain ([`StainGraph::chain`]), 0 nearest the combiner, and
    /// rewires the chain in its new order: each post fed by the one before it, the first by what fed the chain before,
    /// and `OUT` by the last. One edit, checked as a whole; refused where `post` is not in the chain or `position` is
    /// past its end.
    pub fn reorder(&mut self, post: NodeId, position: usize) -> Result<(), GraphError> {
        let mut chain = self.chain();
        let at = chain
            .iter()
            .position(|&p| p == post)
            .ok_or_else(|| GraphError(format!("node {} is not in the live post chain", post.0)))?;
        if position >= chain.len() {
            return Err(GraphError(format!(
                "the live post chain holds {} post(s); no position {position}",
                chain.len()
            )));
        }
        let head = self.wires.get(&(chain[0], 0)).copied();
        chain.remove(at);
        chain.insert(position, post);
        let out = self.out();
        self.edit(|g| {
            let mut from = head;
            for &p in &chain {
                match from {
                    Some(f) => g.wires.insert((p, 0), f),
                    None => g.wires.remove(&(p, 0)),
                };
                from = Some(p);
            }
            if let Some(last) = from {
                g.wires.insert((out, 0), last);
            }
            Ok(())
        })
    }

    /// The `OUT` node's id: a checked graph holds one `OUT`, and no edit removes it.
    fn out(&self) -> NodeId {
        self.nodes
            .values()
            .find(|n| n.kind == NodeKind::Out)
            .map(|n| n.id)
            .expect("a checked graph holds one OUT")
    }

    /// Whether node `from` feeds node `to`, through one wire or several: so a wire from `to` into `from` would close a
    /// cycle. A node does not feed itself in an acyclic graph.
    pub fn feeds(&self, from: NodeId, to: NodeId) -> bool {
        let mut seen = BTreeSet::new();
        let mut stack = vec![to];
        while let Some(id) = stack.pop() {
            for (_, &f) in self.wires.range((id, 0)..=(id, usize::MAX)) {
                if f == from {
                    return true;
                }
                if seen.insert(f) {
                    stack.push(f);
                }
            }
        }
        false
    }

    /// Node `id`'s ports (render_gui_spec Part II §3, §3.1): its in-ports in order, each with the subtype arriving on
    /// its wire where a `field` is wired from a source that states one, and its out-port, a source's with the subtype
    /// of the field it reads (§5).
    pub fn ports_of(&self, id: NodeId) -> Result<Ports, GraphError> {
        let n = self.nodes.get(&id).ok_or_else(|| no_node(id))?;
        let inputs = self
            .ports(n)?
            .into_iter()
            .enumerate()
            .map(|(k, ty)| Port {
                direction: Direction::In,
                ty,
                subtype: self
                    .wires
                    .get(&(id, k))
                    .and_then(|f| self.source_subtype(&self.nodes[f])),
            })
            .collect();
        let outputs = n
            .kind
            .output()
            .map(|ty| Port {
                direction: Direction::Out,
                ty,
                subtype: self.source_subtype(n),
            })
            .into_iter()
            .collect();
        Ok(Ports { inputs, outputs })
    }

    /// The subtype a source's out-port carries: a built-in source's field's (render_gui_spec Part II §5); `None` for
    /// any other node, and for a source whose occupant states none.
    fn source_subtype(&self, n: &Node) -> Option<Subtype> {
        match (&n.kind, &n.occupant) {
            (NodeKind::Source, Occupant::Field(f)) => {
                assemble::field_subtype(f).ok().map(Into::into)
            }
            _ => None,
        }
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
            if n.ramp.is_some() && n.kind != NodeKind::Colour {
                return Err(GraphError(format!(
                    "node {} is a {:?}: only a colour node has a ramp",
                    n.id.0, n.kind
                )));
            }
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
                    ramp: n.ramp.clone(),
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
/// absent), its uniforms' values, each set value or the schema's default, and its custom-stop ramp where it has one.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalNode {
    pub kind: NodeKind,
    pub occupant: Occupant,
    pub inputs: Vec<Option<usize>>,
    pub params: BTreeMap<String, Vec<f64>>,
    pub ramp: Option<Ramp>,
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

    /// The form's text with each node's `params`, its uniforms' values, and a `colour` node's `ramp` where it has one
    /// (colour_composition §9.4), in JCS (gui_state_contract §2, R-318): the render key's graph part.
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
                let mut node = json!({
                    "kind": Kind::from(n.kind).name(),
                    "occupant": occupant,
                    "inputs": n.inputs,
                    "params": n.params,
                });
                if let Some(ramp) = &n.ramp {
                    node["ramp"] = json!(ramp);
                }
                node
            })
            .collect();
        // Every number here is a position in the graph, far below 2^53, a param, a finite f64 (each value is admitted
        // by its schema), or a ramp's, finite (each ramp is checked), so JCS holds each exactly and the serialisation
        // cannot fail.
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
