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
    /// The id of the combiner [`StainGraph::new`] makes.
    pub const COMBINER: NodeId = NodeId(0);
    /// The id of the `OUT` [`StainGraph::new`] makes.
    pub const OUT: NodeId = NodeId(1);

    /// The backbone alone: the combiner, the built-in pass-through (M1's), wired to `OUT`.
    pub fn new() -> StainGraph {
        let node = |id, kind, occupant| Node {
            id,
            kind,
            occupant,
            params: BTreeMap::new(),
        };
        let combiner = node(
            Self::COMBINER,
            NodeKind::Combiner,
            Occupant::Builtin("pass_through".into()),
        );
        StainGraph {
            nodes: BTreeMap::from([
                (Self::COMBINER, combiner),
                (Self::OUT, node(Self::OUT, NodeKind::Out, Occupant::None)),
            ]),
            wires: BTreeMap::from([((Self::OUT, 0), Self::COMBINER)]),
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
        if id == Self::COMBINER || id == Self::OUT {
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

    /// Whether the graph is one: one combiner and one `OUT`; each node's occupant one its slot takes, each param a
    /// value of its schema; each wire into an in-port its target has, its type and the backbone kept; acyclic; and
    /// it lowers.
    pub fn check(&self) -> Result<(), GraphError> {
        for (kind, id) in [
            (NodeKind::Combiner, Self::COMBINER),
            (NodeKind::Out, Self::OUT),
        ] {
            let ids: Vec<NodeId> = self
                .nodes
                .values()
                .filter(|n| n.kind == kind)
                .map(|n| n.id)
                .collect();
            if ids != [id] {
                return Err(GraphError(format!(
                    "the {kind:?} is a fixed singleton, node {}; the graph has {ids:?}",
                    id.0
                )));
            }
        }
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
        if let Some(id) = self.cycle() {
            return Err(GraphError(format!(
                "node {} feeds itself: the stain graph is acyclic",
                id.0
            )));
        }
        self.lower()?;
        Ok(())
    }

    fn ports(&self, n: &Node) -> Result<Vec<PortType>, GraphError> {
        let d = assemble::declaration(n.kind.into(), &(&n.occupant).into())?;
        Ok(assemble::in_ports(n.kind.into(), &d))
    }

    /// A node on a cycle, if any: a depth-first walk upstream from each node.
    fn cycle(&self) -> Option<NodeId> {
        // 0 unvisited, 1 on the walk, 2 done.
        let mut state: BTreeMap<NodeId, u8> = BTreeMap::new();
        fn visit(g: &StainGraph, id: NodeId, state: &mut BTreeMap<NodeId, u8>) -> Option<NodeId> {
            match state.get(&id) {
                Some(1) => return Some(id),
                Some(_) => return None,
                None => {}
            }
            state.insert(id, 1);
            for (_, &from) in g.wires.range((id, 0)..=(id, usize::MAX)) {
                if let Some(c) = visit(g, from, state) {
                    return Some(c);
                }
            }
            state.insert(id, 2);
            None
        }
        self.nodes
            .keys()
            .find_map(|&id| visit(self, id, &mut state))
    }

    /// The node feeding `to`'s in-port `port`, if any.
    fn feeder(&self, to: NodeId, port: usize) -> Option<NodeId> {
        self.wires.get(&(to, port)).copied()
    }

    /// Whether node `id` is the identity (render_gui_spec §13): its occupant None, or a field input absent or fed by
    /// an identity source.
    fn identity(&self, id: NodeId) -> bool {
        let n = &self.nodes[&id];
        if n.occupant == Occupant::None {
            return true;
        }
        let ports = self.ports(n).unwrap_or_default();
        ports.iter().enumerate().any(|(k, &p)| {
            p == PortType::Field && self.feeder(id, k).is_none_or(|f| self.identity(f))
        })
    }

    /// The canonical form (lowering contract Part 5; render contract Part 3): the live nodes in canonical order.
    pub fn canonical(&self) -> Canonical {
        let live = |port: Option<NodeId>| port.filter(|&f| !self.identity(f));
        let mut chain = Vec::new();
        let mut at = self.feeder(Self::OUT, 0);
        while let Some(id) = at.filter(|&id| self.nodes[&id].kind == NodeKind::Post) {
            if !self.identity(id) {
                chain.push(id);
            }
            at = self.feeder(id, 0);
        }
        chain.reverse();
        let colour = live(self.feeder(Self::COMBINER, 0));
        let brightness = live(self.feeder(Self::COMBINER, 1));
        let field_ports = |id: NodeId| {
            let ports = self.ports(&self.nodes[&id]).unwrap_or_default();
            (0..ports.len())
                .filter(|&k| ports[k] == PortType::Field)
                .collect::<Vec<usize>>()
        };
        let mut order: Vec<NodeId> = Vec::new();
        for id in colour.iter().chain(&brightness).chain(&chain) {
            for k in field_ports(*id) {
                if let Some(s) = self.feeder(*id, k).filter(|s| !order.contains(s)) {
                    order.push(s);
                }
            }
        }
        order.extend(colour);
        order.extend(brightness);
        order.push(Self::COMBINER);
        order.extend(&chain);
        order.push(Self::OUT);
        let position = |id: NodeId| order.iter().position(|&o| o == id);
        let nodes = order
            .iter()
            .map(|&id| {
                let n = &self.nodes[&id];
                let inputs = match n.kind {
                    NodeKind::Combiner => {
                        vec![colour.and_then(position), brightness.and_then(position)]
                    }
                    NodeKind::Post | NodeKind::Out => {
                        let previous = order[..position(id).unwrap_or(0)]
                            .iter()
                            .rev()
                            .find(|&&o| {
                                matches!(self.nodes[&o].kind, NodeKind::Combiner | NodeKind::Post)
                            })
                            .copied();
                        let mut inputs = vec![previous.and_then(position)];
                        inputs.extend(
                            field_ports(id)
                                .into_iter()
                                .map(|k| self.feeder(id, k).and_then(position)),
                        );
                        inputs
                    }
                    _ => field_ports(id)
                        .into_iter()
                        .map(|k| self.feeder(id, k).and_then(position))
                        .collect(),
                };
                let schema = assemble::declaration(n.kind.into(), &(&n.occupant).into())
                    .map(|d| d.uniforms)
                    .unwrap_or_default();
                let params = schema
                    .into_iter()
                    .map(|u| {
                        let value = n.params.get(&u.name).cloned().unwrap_or(u.default);
                        (u.name, value)
                    })
                    .collect();
                CanonicalNode {
                    kind: n.kind,
                    occupant: n.occupant.clone(),
                    inputs,
                    params,
                }
            })
            .collect();
        Canonical { nodes }
    }

    /// The graph as the assembler takes it: its canonical form ([`Canonical::stain`]).
    pub fn lower(&self) -> Result<Stain, GraphError> {
        self.canonical().stain()
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

/// The canonical form of a stain graph (lowering contract Part 5; render contract Part 3; REQ-RENDER-075): its live
/// nodes, those `OUT` depends on with the identity dropped, in canonical order — the sources in order of first use,
/// then the colour, the brightness, the combiner, the posts from the combiner to `OUT`, and `OUT` — each wired by
/// position. Node ids and the order of construction are not in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Canonical {
    pub nodes: Vec<CanonicalNode>,
}

impl Canonical {
    fn json(&self, params: bool) -> Value {
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
                let mut v = json!({
                    "kind": Kind::from(n.kind).name(),
                    "occupant": occupant,
                    "inputs": n.inputs,
                });
                if params {
                    v["params"] = json!(n.params);
                }
                v
            })
            .collect();
        json!({ "nodes": nodes })
    }

    /// The form's text, JCS (gui_state_contract §2, R-318): the fragment key's input. The params are not in it.
    pub fn text(&self) -> String {
        canonical::json_to_string(&self.json(false)).unwrap_or_default()
    }

    /// The form's text with each node's `params`, its uniforms' values: the render key's graph part.
    pub fn render_text(&self) -> String {
        canonical::json_to_string(&self.json(true)).unwrap_or_default()
    }

    /// The fragment key (lowering contract Part 5): the 64-bit FNV-1a hash of [`Canonical::text`]'s UTF-8 bytes.
    pub fn fragment_key(&self) -> u64 {
        ledger::version::fnv1a64(self.text().as_bytes())
    }

    /// The render key's graph part (render contract Part 3): the 64-bit FNV-1a hash of [`Canonical::render_text`].
    pub fn render_key(&self) -> u64 {
        ledger::version::fnv1a64(self.render_text().as_bytes())
    }

    /// The form as the assembler's [`Stain`], in the same order.
    pub fn stain(&self) -> Result<Stain, GraphError> {
        let nodes = self
            .nodes
            .iter()
            .map(|n| assemble::Node {
                kind: n.kind.into(),
                occupant: (&n.occupant).into(),
                inputs: n.inputs.clone(),
            })
            .collect();
        Ok(Stain::new(nodes)?)
    }
}
