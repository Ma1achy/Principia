//! The occupant tree's codegen (colour_composition §1, §5): a colour or brightness occupant's expression tree, as the
//! engine lowers it ([`Node`]), to deterministic, readable WGSL, the occupant's text for the one assembler
//! ([`crate::assemble`]; REQ-RENDER-005, REQ-RENDER-010), which places it in the template's node functions layer,
//! `[prelude][node functions][shade()]`, at its stain node's position in the canonical form (lowering contract Part 5;
//! REQ-RENDER-075). There is no second compile path: the text is an occupant's WGSL like any other.
//!
//! **One function per tree node.** Node `id` is `fn node_<id>(ctx: Ctx) -> <its type>`, its name from its id alone
//! ([`function_name`]), so it is the same function across recompiles and an edit elsewhere in the tree leaves it byte
//! for byte. Each function is headed by a comment carrying the node's name and its parameters' values, then its
//! uniforms' declarations; its body calls one function of the shared WGSL library (the prelude and the maps after it,
//! render_gui_spec §10.1), its arguments the context, its children's functions and its uniforms. The functions are in
//! the tree's post-order, each node's children in input order before it, and the slot's function, `colour` or
//! `brightness`, last, returning the root's: the order is the tree's, never the order it was built in.
//!
//! **Parameters are uniforms** (lowering contract Part 3, Part 5): parameter `p` of node `id` is the uniform
//! `node_<id>_<p>` ([`uniform_name`]), declared `// @uniform` with the node's value at generation as its default and,
//! for a map parameter, its adopted range ([`schema`]). A slider edit writes that uniform and regenerates nothing, so
//! the occupant's text, and with it the fragment key, is unchanged: it rebinds and never recompiles.

pub mod schema;

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use crate::assemble::{Kind, Uniform, UniformType};
use schema::MapParam;

/// A node's output type (colour_composition §1): a colour or a scalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    Vec3,
    F32,
}

impl ValueType {
    /// The type as WGSL writes it.
    pub const fn wgsl(self) -> &'static str {
        match self {
            ValueType::Vec3 => "vec3<f32>",
            ValueType::F32 => "f32",
        }
    }
}

/// The slot an occupant tree fills (colour_composition §1): its signature constrains only the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Colour,
    Brightness,
}

impl Slot {
    /// The slot's required output: `vec3` for `colour`, `f32` for `brightness`.
    pub const fn signature(self) -> ValueType {
        match self {
            Slot::Colour => ValueType::Vec3,
            Slot::Brightness => ValueType::F32,
        }
    }

    /// The stain node kind the slot is.
    pub const fn kind(self) -> Kind {
        match self {
            Slot::Colour => Kind::Colour,
            Slot::Brightness => Kind::Brightness,
        }
    }
}

/// One argument of a node's library call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arg {
    /// The context, `ctx`.
    Ctx,
    /// Input `k`'s value: its child's function, called on the context.
    Input(usize),
    /// Parameter `k`'s uniform.
    Param(usize),
}

/// One parameter of a node: its name, its uniform's type, its value, one number per component, and, for a map
/// parameter, which one, whose adopted range it is clamped into ([`MapParam::clamp`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: UniformType,
    pub value: Vec<f64>,
    pub range: Option<MapParam>,
}

impl Param {
    /// The map parameter `param` named `name`, with `value`.
    pub fn ranged(name: &str, param: MapParam, value: f64) -> Param {
        Param {
            name: name.to_owned(),
            ty: UniformType::F32,
            value: vec![value],
            range: Some(param),
        }
    }

    /// A parameter without an adopted range, of type `ty`, with `value`.
    pub fn free(name: &str, ty: UniformType, value: Vec<f64>) -> Param {
        Param {
            name: name.to_owned(),
            ty,
            value,
            range: None,
        }
    }

    /// The parameter as node `id`'s uniform, or why not: a map parameter's value clamped into its range, NaN refused;
    /// any other's a value of its type.
    fn uniform(&self, id: u32) -> Result<Uniform, CodegenError> {
        let name = uniform_name(id, &self.name);
        let bad = || {
            CodegenError(format!(
                "node {id}'s parameter `{}` = {:?} is no value of its uniform",
                self.name, self.value
            ))
        };
        let u = match (self.range, &self.value[..]) {
            (Some(p), &[v]) if self.ty == UniformType::F32 => {
                p.uniform(&name, v).ok_or_else(bad)?
            }
            (Some(_), _) => return Err(bad()),
            (None, _) => Uniform {
                name,
                ty: self.ty,
                default: self.value.clone(),
                range: None,
            },
        };
        if u.admits(&u.default) {
            Ok(u)
        } else {
            Err(bad())
        }
    }
}

/// What a node does: its name, which its function's comment carries; the shared-library function it calls, and that
/// call's arguments; and its parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub name: String,
    pub function: String,
    pub args: Vec<Arg>,
    pub params: Vec<Param>,
}

/// One node of an occupant tree as the codegen takes it: its id, unique in the tree; its output type; its call; and
/// its inputs, each a subtree, in input order. The engine's tree, type-checked, lowers to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: u32,
    pub output: ValueType,
    pub call: Call,
    pub inputs: Vec<Node>,
}

/// One node's function in the generated text: the node's id, the function's name, and its text, the header comment
/// and the uniforms' declarations with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeFunction {
    pub id: u32,
    pub name: String,
    pub text: String,
}

/// A generated occupant: its text, whole, and each node's function in it, in emission order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub source: String,
    pub functions: Vec<NodeFunction>,
}

/// Why a tree did not generate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenError(pub String);

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CodegenError {}

/// Node `id`'s function's name: `node_<id>`.
pub fn function_name(id: u32) -> String {
    format!("node_{id}")
}

/// The uniform of node `id`'s parameter `param`: `node_<id>_<param>`.
pub fn uniform_name(id: u32, param: &str) -> String {
    format!("node_{id}_{param}")
}

/// The tree rooted at `root`, filling `slot`, as the occupant's WGSL; refused where a node's id is used twice, a
/// function or parameter name is not a WGSL identifier or a parameter name is used twice in a node, a node's name
/// holds a control character (it would leave its comment), an argument names no input or parameter, or a parameter's
/// value is no value of its uniform.
pub fn generate(slot: Slot, root: &Node) -> Result<Generated, CodegenError> {
    let mut functions = Vec::new();
    emit(root, &mut BTreeSet::new(), &mut functions)?;
    let mut source = format!(
        "// A {} occupant generated from its expression tree (colour_composition §1, §5): one function per node, each\n\
         // node's inputs before it, then the slot's function, which returns the root's.\n",
        slot.kind().name()
    );
    for f in &functions {
        source.push('\n');
        source.push_str(&f.text);
    }
    let _ = write!(
        source,
        "\n// The {} slot: the root, {}.\nfn {}(ctx: Ctx) -> {} {{\n    return {}(ctx);\n}}\n",
        slot.kind().name(),
        function_name(root.id),
        slot.kind().slot().unwrap_or_default(),
        slot.signature().wgsl(),
        function_name(root.id)
    );
    Ok(Generated { source, functions })
}

/// `node`'s subtree's functions, in post-order, onto `out`; `seen` holds the ids emitted so far.
fn emit(
    node: &Node,
    seen: &mut BTreeSet<u32>,
    out: &mut Vec<NodeFunction>,
) -> Result<(), CodegenError> {
    for input in &node.inputs {
        emit(input, seen, out)?;
    }
    let id = node.id;
    let refuse = |why: String| Err(CodegenError(format!("node {id}: {why}")));
    if !seen.insert(id) {
        return refuse("the id is used twice in the tree".into());
    }
    let call = &node.call;
    if call.name.chars().any(char::is_control) {
        return refuse(format!(
            "the name {:?} holds a control character",
            call.name
        ));
    }
    if !is_identifier(&call.function) {
        return refuse(format!("`{}` is no function name", call.function));
    }
    let mut names = BTreeSet::new();
    let mut uniforms = Vec::with_capacity(call.params.len());
    for p in &call.params {
        if !is_identifier(&p.name) || !names.insert(p.name.as_str()) {
            return refuse(format!(
                "the parameter name `{}` is no identifier, or is used twice",
                p.name
            ));
        }
        uniforms.push(p.uniform(id)?);
    }
    let mut args = Vec::with_capacity(call.args.len());
    for &a in &call.args {
        args.push(match a {
            Arg::Ctx => "ctx".to_owned(),
            Arg::Input(k) => match node.inputs.get(k) {
                Some(child) => format!("{}(ctx)", function_name(child.id)),
                None => return refuse(format!("no input {k}")),
            },
            Arg::Param(k) => match uniforms.get(k) {
                Some(u) => format!("uniforms.{}", u.name),
                None => return refuse(format!("no parameter {k}")),
            },
        });
    }
    let name = function_name(id);
    let mut text = format!("// {name}: {}", call.name);
    for (k, (p, u)) in call.params.iter().zip(&uniforms).enumerate() {
        let sep = if k == 0 { " — " } else { ", " };
        let _ = write!(text, "{sep}{} = {}", p.name, value(&u.default));
    }
    text.push('\n');
    for u in &uniforms {
        let _ = write!(
            text,
            "// @uniform {}: {} = {}",
            u.name,
            u.ty.wgsl(),
            value(&u.default)
        );
        if let Some((lo, hi)) = u.range {
            let _ = write!(text, " [{lo}, {hi}]");
        }
        text.push('\n');
    }
    let _ = write!(
        text,
        "fn {name}(ctx: Ctx) -> {} {{\n    return {}({});\n}}\n",
        node.output.wgsl(),
        call.function,
        args.join(", ")
    );
    out.push(NodeFunction { id, name, text });
    Ok(())
}

/// A value as its declaration writes it: a number, or `(a, b, …)` for a vector.
fn value(v: &[f64]) -> String {
    match v {
        [x] => x.to_string(),
        _ => {
            let parts: Vec<String> = v.iter().map(f64::to_string).collect();
            format!("({})", parts.join(", "))
        }
    }
}

/// Whether `s` is an ASCII WGSL identifier: a letter or `_`, then letters, digits or `_`; not `_` alone, and not
/// beginning `__`.
fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && s != "_"
        && !s.starts_with("__")
}
