//! The Print graph as data (principia_print_graph.md §4.1; R-486, R-491): the format `principia.print/1`, its
//! settings and `$` references, presets used as nodes, and its one canonical serialisation.
//!
//! A graph is `{ format, name, settings, nodes, out }`. `nodes` maps an id to `{ type, params, in }`; `in` maps each
//! input port to `node` or `node.port`. `settings` are the graph's public knobs: a parameter written `"$name"` reads
//! setting `name`, and `"$name+k"` or `"$name-k"` reads it plus or less the integer `k`, as the Python prototype does
//! ([`resolve`]). A node of type `preset:<name>` runs that preset's graph as a sub-graph ([`expand`]). Editor
//! positions are `ViewUI`'s, never the graph's, so a node carrying one is refused. The graph serialises in JCS (R-309,
//! R-318; RFC 8785), its node ids sorted and carrying no meaning ([`Graph::canonical`]).
//!
//! This module, [`super::ports`] and [`super::registry`] are compiled into render's build script as well, which runs
//! the registry check (§4.3, §11 test 8): they name each other through `super::` only.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value as Json};

/// The format every Print graph declares.
pub const FORMAT: &str = "principia.print/1";

/// The type prefix of a node that runs a preset as a sub-graph.
pub const PRESET_PREFIX: &str = "preset:";

/// The node type of the Render node, the stain's finished colours (§4.3: "Print source").
pub const RENDER: &str = "render";

/// The node type of the graph's output.
pub const OUTPUT: &str = "output";

/// What separates a sub-graph's node ids from its preset node's in an expanded graph: `pol/grain`.
pub const PATH_SEPARATOR: char = '/';

/// The keys an editor stores a node's position under: refused, since positions are `ViewUI`'s (§4.1).
const POSITION_KEYS: [&str; 4] = ["x", "y", "pos", "position"];

/// A parameter's or a setting's value: a number, or a string (a colour, or a `$` reference).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(v) => write!(f, "{v}"),
            Value::Text(t) => write!(f, "{t:?}"),
        }
    }
}

/// One node: its type, its parameters by name, and its wired inputs, each port to `node` or `node.port`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub ty: String,
    pub params: BTreeMap<String, Value>,
    pub inputs: BTreeMap<String, String>,
}

/// A Print graph (§4.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    pub name: String,
    pub settings: BTreeMap<String, Value>,
    pub nodes: BTreeMap<String, Node>,
    pub out: String,
}

/// Why a graph did not parse, resolve or expand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphError(pub String);

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GraphError {}

fn err<T>(why: String) -> Result<T, GraphError> {
    Err(GraphError(why))
}

impl Graph {
    /// The graph `text` holds, or why not: not JSON, another format, a member missing or of the wrong kind, a member
    /// the format does not have (an editor position among them), a node id holding `.` or `/`, or a wire or `out`
    /// naming no node.
    pub fn parse(text: &str) -> Result<Graph, GraphError> {
        let json: Json =
            serde_json::from_str(text).map_err(|e| GraphError(format!("not JSON: {e}")))?;
        Graph::from_json(&json)
    }

    /// The graph the JSON value `json` is; see [`Graph::parse`].
    pub fn from_json(json: &Json) -> Result<Graph, GraphError> {
        let Some(top) = json.as_object() else {
            return err("a Print graph is a JSON object".into());
        };
        for key in top.keys() {
            if !["format", "name", "settings", "nodes", "out"].contains(&key.as_str()) {
                return err(format!(
                    "the graph carries `{key}`, which is no member of {FORMAT} (editor positions are ViewUI's, never \
                     the graph's)"
                ));
            }
        }
        match top.get("format").and_then(Json::as_str) {
            Some(FORMAT) => {}
            other => return err(format!("the format is {other:?}, not {FORMAT:?}")),
        }
        let name = string(top, "name", "the graph")?;
        let settings = match top.get("settings") {
            None => BTreeMap::new(),
            Some(s) => values(s, "the graph's settings")?,
        };
        let Some(members) = top.get("nodes").and_then(Json::as_object) else {
            return err("the graph has no `nodes` object".into());
        };
        let mut nodes = BTreeMap::new();
        for (id, n) in members {
            if id.is_empty() || id.contains(['.', PATH_SEPARATOR]) {
                return err(format!(
                    "the node id {id:?} is empty or holds `.` or `{PATH_SEPARATOR}`"
                ));
            }
            nodes.insert(id.clone(), node(id, n)?);
        }
        let out = string(top, "out", "the graph")?;
        let graph = Graph {
            name,
            settings,
            nodes,
            out,
        };
        if !graph.nodes.contains_key(&graph.out) {
            return err(format!("`out` names `{}`, which is no node", graph.out));
        }
        for (id, n) in &graph.nodes {
            for (port, wire) in &n.inputs {
                let (to, _) = split_wire(wire);
                if !graph.nodes.contains_key(to) {
                    return err(format!(
                        "node `{id}`: input `{port}` reads `{wire}`, which is no node"
                    ));
                }
            }
        }
        Ok(graph)
    }

    /// The graph in the one canonical serialisation (§4.1; R-309, JCS by R-318): JCS, every object's members sorted
    /// by their names' UTF-16 code units, node ids with them. A node's `params` and `in` are left out when empty, and
    /// `settings` is always written. Refused only for a number with no JSON form (NaN or an infinity), which no
    /// parsed graph holds.
    pub fn canonical(&self) -> Result<String, GraphError> {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (id, n) in &self.nodes {
            let mut members = vec![("type".to_owned(), quote(&n.ty))];
            if !n.params.is_empty() {
                members.push(("params".to_owned(), value_map(&n.params)?));
            }
            if !n.inputs.is_empty() {
                let wires = n
                    .inputs
                    .iter()
                    .map(|(port, wire)| (port.clone(), quote(wire)))
                    .collect();
                members.push(("in".to_owned(), object(wires)));
            }
            nodes.push((id.clone(), object(members)));
        }
        Ok(object(vec![
            ("format".to_owned(), quote(FORMAT)),
            ("name".to_owned(), quote(&self.name)),
            ("settings".to_owned(), value_map(&self.settings)?),
            ("nodes".to_owned(), object(nodes)),
            ("out".to_owned(), quote(&self.out)),
        ]))
    }
}

/// The string member `key` of `obj`, or why not; `what` names the object.
fn string(obj: &Map<String, Json>, key: &str, what: &str) -> Result<String, GraphError> {
    match obj.get(key).and_then(Json::as_str) {
        Some(s) => Ok(s.to_owned()),
        None => err(format!("{what} has no string `{key}`")),
    }
}

/// A JSON object of numbers and strings, by name; `what` names it.
fn values(json: &Json, what: &str) -> Result<BTreeMap<String, Value>, GraphError> {
    let Some(members) = json.as_object() else {
        return err(format!("{what} is not an object"));
    };
    let mut out = BTreeMap::new();
    for (k, v) in members {
        let value = match v {
            Json::Number(n) => match n.as_f64() {
                Some(x) => Value::Number(x),
                None => return err(format!("{what}: `{k}` = {n} is no double")),
            },
            Json::String(s) => Value::Text(s.clone()),
            _ => {
                return err(format!(
                    "{what}: `{k}` is {v}, neither a number nor a string"
                ))
            }
        };
        out.insert(k.clone(), value);
    }
    Ok(out)
}

/// Node `id` as `json` holds it.
fn node(id: &str, json: &Json) -> Result<Node, GraphError> {
    let what = format!("node `{id}`");
    let Some(members) = json.as_object() else {
        return err(format!("{what} is not an object"));
    };
    for key in members.keys() {
        if POSITION_KEYS.contains(&key.as_str()) {
            return err(format!(
                "{what} carries an editor position, `{key}`: editor positions are ViewUI's, never the graph's \
                 (principia_print_graph.md §4.1)"
            ));
        }
        if !["type", "params", "in"].contains(&key.as_str()) {
            return err(format!(
                "{what} carries `{key}`, which is no member of a node"
            ));
        }
    }
    let ty = string(members, "type", &what)?;
    let params = match members.get("params") {
        None => BTreeMap::new(),
        Some(p) => values(p, &format!("{what}'s params"))?,
    };
    let mut inputs = BTreeMap::new();
    if let Some(wires) = members.get("in") {
        let Some(wires) = wires.as_object() else {
            return err(format!("{what}'s `in` is not an object"));
        };
        for (port, wire) in wires {
            let Some(wire) = wire.as_str() else {
                return err(format!(
                    "{what}: input `{port}` is {wire}, not `node` or `node.port`"
                ));
            };
            inputs.insert(port.clone(), wire.to_owned());
        }
    }
    Ok(Node { ty, params, inputs })
}

/// A wire's node and, after a `.`, its port.
fn split_wire(wire: &str) -> (&str, Option<&str>) {
    match wire.split_once('.') {
        Some((node, port)) => (node, Some(port)),
        None => (wire, None),
    }
}

// ── `$` references ──────────────────────────────────────────────────────────────────────────────────────────────

/// `value` with its `$` reference read from `settings`: `"$name"` is setting `name`; `"$name+k"` and `"$name-k"`,
/// `k` a decimal integer, are that setting, a number, plus or less `k` (the Python prototype's `$` rule: the name ends
/// at the first `+`, else the first `-`). Any other value is itself. Refused, naming the setting, where the graph has
/// no such setting, and where an offset is no integer or its setting no number.
pub fn resolve(value: &Value, settings: &BTreeMap<String, Value>) -> Result<Value, String> {
    let Value::Text(text) = value else {
        return Ok(value.clone());
    };
    let Some(reference) = text.strip_prefix('$') else {
        return Ok(value.clone());
    };
    let split = reference
        .split_once('+')
        .map(|(n, k)| (n, k, 1.0))
        .or_else(|| reference.split_once('-').map(|(n, k)| (n, k, -1.0)));
    let name = split.map_or(reference, |(n, _, _)| n);
    let Some(setting) = settings.get(name) else {
        return Err(format!(
            "{text:?} reads the setting `{name}`, which the graph does not have"
        ));
    };
    let Some((_, k, sign)) = split else {
        return Ok(setting.clone());
    };
    let digits = !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit());
    let offset = match k.parse::<u32>() {
        Ok(k) if digits => f64::from(k),
        _ => return Err(format!("{text:?}: the offset `{k}` is no decimal integer")),
    };
    match setting {
        Value::Number(base) => Ok(Value::Number(base + sign * offset)),
        Value::Text(_) => Err(format!(
            "{text:?}: the setting `{name}` = {setting} is no number, so it takes no offset"
        )),
    }
}

// ── Sub-graphs ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A wire in an expanded graph: a node and, where it names one, its output port.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wire {
    pub node: String,
    pub port: Option<String>,
}

/// A node of an expanded graph: a registered type, never a preset; its parameters with every `$` reference read;
/// its wired inputs; and, for a node inside a sub-graph, the wire its preset node's input comes from, which an
/// unconnected input defaulting to the render reads in the render's place (the prototype's `src`).
#[derive(Clone, Debug, PartialEq)]
pub struct ExpandedNode {
    pub ty: String,
    pub params: BTreeMap<String, Value>,
    pub inputs: BTreeMap<String, Wire>,
    pub entry: Option<Wire>,
}

/// A graph with every preset node replaced by its preset's graph: a sub-graph's node `n`, under preset node `p`, is
/// `p/n`; its Render node is the preset node's input where one is wired, and its output node is dropped, a wire to
/// the preset node reading what fed it.
#[derive(Clone, Debug, PartialEq)]
pub struct Expanded {
    pub name: String,
    pub nodes: BTreeMap<String, ExpandedNode>,
    pub out: String,
}

/// `graph` with every preset node expanded, each preset found by name through `presets`; refused, naming it, for a
/// `$` reference no setting answers, a preset that is not there, a preset that includes itself (naming the cycle), a
/// preset node's parameter its preset has no setting for, or a wire that names no node.
pub fn expand(
    graph: &Graph,
    presets: &dyn Fn(&str) -> Option<Graph>,
) -> Result<Expanded, GraphError> {
    let mut expander = Expander {
        presets,
        stack: Vec::new(),
        nodes: BTreeMap::new(),
    };
    expander.graph(graph, &graph.settings, "", None, true)?;
    Ok(Expanded {
        name: graph.name.clone(),
        nodes: expander.nodes,
        out: graph.out.clone(),
    })
}

struct Expander<'a> {
    presets: &'a dyn Fn(&str) -> Option<Graph>,
    /// The presets being expanded, outermost first.
    stack: Vec<String>,
    nodes: BTreeMap<String, ExpandedNode>,
}

/// One graph's expansion: its preset nodes' exits, and the preset nodes whose expansion is under way.
struct Scope<'g> {
    graph: &'g Graph,
    settings: &'g BTreeMap<String, Value>,
    prefix: &'g str,
    entry: Option<&'g Wire>,
    top: bool,
    exits: BTreeMap<String, Wire>,
    pending: Vec<String>,
}

impl Expander<'_> {
    /// Expands `graph` under `prefix`; returns what its output node is fed by (a sub-graph's exit).
    fn graph(
        &mut self,
        graph: &Graph,
        settings: &BTreeMap<String, Value>,
        prefix: &str,
        entry: Option<&Wire>,
        top: bool,
    ) -> Result<Wire, GraphError> {
        let mut scope = Scope {
            graph,
            settings,
            prefix,
            entry,
            top,
            exits: BTreeMap::new(),
            pending: Vec::new(),
        };
        for (id, n) in &graph.nodes {
            if n.ty.starts_with(PRESET_PREFIX) {
                self.preset(&mut scope, id)?;
            } else if (!top && *id == graph.out) || (n.ty == RENDER && entry.is_some()) {
                // A sub-graph's output node is its exit, and its Render node its preset node's input.
            } else {
                let mut params = BTreeMap::new();
                for (name, v) in &n.params {
                    let v = resolve(v, settings).map_err(|why| {
                        GraphError(format!("node `{prefix}{id}`: parameter `{name}` {why}"))
                    })?;
                    params.insert(name.clone(), v);
                }
                let mut inputs = BTreeMap::new();
                for (port, wire) in &n.inputs {
                    inputs.insert(port.clone(), self.reference(&mut scope, wire)?);
                }
                self.nodes.insert(
                    format!("{prefix}{id}"),
                    ExpandedNode {
                        ty: n.ty.clone(),
                        params,
                        inputs,
                        entry: entry.cloned(),
                    },
                );
            }
        }
        let out_id = &graph.out;
        if top {
            return Ok(Wire {
                node: out_id.clone(),
                port: None,
            });
        }
        let Some(out) = graph.nodes.get(out_id) else {
            return err(format!(
                "preset `{}`: `out` names `{out_id}`, which is no node",
                graph.name
            ));
        };
        if out.ty != OUTPUT {
            return err(format!(
                "preset `{}`: `out` names `{out_id}`, a `{}`, not an `{OUTPUT}` node",
                graph.name, out.ty
            ));
        }
        match out.inputs.get("in") {
            Some(wire) => self.reference(&mut scope, wire),
            None => err(format!(
                "preset `{}`: its output node `{out_id}` is not wired",
                graph.name
            )),
        }
    }

    /// The expanded wire `wire`, read in `scope`, names.
    fn reference(&mut self, scope: &mut Scope, wire: &str) -> Result<Wire, GraphError> {
        let (id, port) = split_wire(wire);
        let Some(target) = scope.graph.nodes.get(id) else {
            return err(format!(
                "`{}{wire}` names no node of `{}`",
                scope.prefix, scope.graph.name
            ));
        };
        if target.ty.starts_with(PRESET_PREFIX) || (target.ty == RENDER && scope.entry.is_some()) {
            if port.is_some_and(|p| p != "out") {
                return err(format!(
                    "`{}{wire}`: a preset node or a sub-graph's Render node has one output, `out`",
                    scope.prefix
                ));
            }
            return match scope.entry {
                Some(entry) if target.ty == RENDER => Ok(entry.clone()),
                _ => self.preset(scope, id),
            };
        }
        if !scope.top && id == scope.graph.out {
            return err(format!(
                "`{}{wire}` reads the preset's output node, which a sub-graph drops",
                scope.prefix
            ));
        }
        Ok(Wire {
            node: format!("{}{id}", scope.prefix),
            port: port.map(str::to_owned),
        })
    }

    /// Preset node `id` of `scope` expanded, once; its exit.
    fn preset(&mut self, scope: &mut Scope, id: &str) -> Result<Wire, GraphError> {
        if let Some(exit) = scope.exits.get(id) {
            return Ok(exit.clone());
        }
        let here = format!("{}{id}", scope.prefix);
        if scope.pending.iter().any(|p| p == id) {
            return err(format!("node `{here}`: its preset reads its own output"));
        }
        let n = &scope.graph.nodes[id];
        let name = &n.ty[PRESET_PREFIX.len()..];
        if self.stack.iter().any(|p| p == name) {
            let mut cycle = self.stack.clone();
            cycle.push(name.to_owned());
            let from = cycle.iter().position(|p| p == name).unwrap_or(0);
            return err(format!(
                "node `{here}`: the preset `{name}` includes itself: {}",
                cycle[from..].join(" → ")
            ));
        }
        let Some(sub) = (self.presets)(name) else {
            return err(format!("node `{here}`: there is no preset `{name}`"));
        };
        scope.pending.push(id.to_owned());
        let entry = match n.inputs.iter().next() {
            None => scope.entry.cloned(),
            Some((port, wire)) if port == "in" && n.inputs.len() == 1 => {
                Some(self.reference(scope, wire)?)
            }
            Some(_) => return err(format!("node `{here}`: a preset node has one input, `in`")),
        };
        let mut settings = sub.settings.clone();
        for (k, v) in &n.params {
            if !settings.contains_key(k) {
                return err(format!(
                    "node `{here}`: the preset `{name}` has no setting `{k}`"
                ));
            }
            let v = resolve(v, scope.settings)
                .map_err(|why| GraphError(format!("node `{here}`: parameter `{k}` {why}")))?;
            settings.insert(k.clone(), v);
        }
        self.stack.push(name.to_owned());
        let exit = self.graph(
            &sub,
            &settings,
            &format!("{here}{PATH_SEPARATOR}"),
            entry.as_ref(),
            false,
        );
        self.stack.pop();
        let exit = exit?;
        scope.pending.retain(|p| p != id);
        scope.exits.insert(id.to_owned(), exit.clone());
        Ok(exit)
    }
}

// ── JCS ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// An object of `members`, each a name and its value's text, sorted by the names' UTF-16 code units (RFC 8785
/// §3.2.3).
fn object(mut members: Vec<(String, String)>) -> String {
    members.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
    let body: Vec<String> = members
        .into_iter()
        .map(|(k, v)| format!("{}:{v}", quote(&k)))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// A map of values as a JCS object.
fn value_map(values: &BTreeMap<String, Value>) -> Result<String, GraphError> {
    let mut members = Vec::with_capacity(values.len());
    for (k, v) in values {
        let text = match v {
            Value::Number(x) => number(*x)?,
            Value::Text(t) => quote(t),
        };
        members.push((k.clone(), text));
    }
    Ok(object(members))
}

/// A string as JCS writes it (RFC 8785 §3.2.2.2): `"` and `\` escaped; the controls U+0000 to U+001F as `\b`, `\t`,
/// `\n`, `\f`, `\r` or `\u00xx`; every other character as itself.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// An f64 as JCS writes it (RFC 8785 §3.2.2.3, ECMAScript's Number-to-String), as the engine's canonical writer does
/// (`engine::contract::canonical::format_f64`, which render cannot depend on): `0` for both zeros; otherwise the
/// shortest digits d₁…d_k that read back to the value, with n the point's position (0.d₁…d_k × 10ⁿ): the digits and
/// n − k zeros when k ≤ n ≤ 21; the point after the n-th digit when 0 < n ≤ 21; `0.`, −n zeros and the digits when
/// −6 < n ≤ 0; otherwise d₁, `.` and the rest when k > 1, `e`, the sign of n − 1 and its magnitude. NaN and the
/// infinities have no JSON form.
pub fn number(value: f64) -> Result<String, GraphError> {
    if !value.is_finite() {
        return err(format!(
            "{value} has no JSON form, so no canonical serialisation"
        ));
    }
    if value == 0.0 {
        return Ok("0".to_owned());
    }
    let mut buffer = ryu::Buffer::new();
    let text = buffer.format_finite(value.abs());
    let (mantissa, exponent) = match text.split_once('e') {
        Some((m, e)) => (
            m,
            e.parse::<isize>().map_err(|e| GraphError(e.to_string()))?,
        ),
        None => (text, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all = format!("{int}{frac}");
    let significant = all.trim_start_matches('0');
    let digits = significant.trim_end_matches('0');
    // The lengths are a few tens of characters, so the casts are exact.
    let k = digits.len() as isize;
    let n = int.len() as isize + exponent - (all.len() - significant.len()) as isize;
    let mut out = String::new();
    if value < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.push_str(&"0".repeat((n - k).unsigned_abs()));
    } else if 0 < n && n <= 21 {
        let (before, after) = digits.split_at(n.unsigned_abs());
        out.push_str(before);
        out.push('.');
        out.push_str(after);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat(n.unsigned_abs()));
        out.push_str(digits);
    } else {
        let (first, rest) = digits.split_at(1);
        out.push_str(first);
        if !rest.is_empty() {
            out.push('.');
            out.push_str(rest);
        }
        let e = n - 1;
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.unsigned_abs().to_string());
    }
    Ok(out)
}
