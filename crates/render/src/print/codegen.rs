//! The Print graph's own codegen (principia_print_graph.md §4.3; render_gui_spec §12): an expanded, validated graph
//! to one WGSL module holding each node's passes, generated from the registry. It is not the stain's: the stain's
//! codegen ([`crate::codegen`]) refuses any function in this module's namespace, [`PREFIX`], so a Print graph node
//! never becomes a stain occupant.
//!
//! **Order and names.** The nodes run in their wiring's order, each after the nodes it reads, ties broken by id;
//! node `k` in that order owns every name `print_n<k>_…`. Its pass `j` is the function `print_n<k>_pass<j>` and the
//! fragment entry point `print_n<k>_pass<j>_main`; each input `p` is the texture `print_n<k>_in_<p>`; a node of more
//! than one pass reads its previous pass as `print_n<k>_prev` (black for the first, as the lab's `prev`); and its
//! parameters are the uniform `print_n<k>`, one member each, `p_<name>`: the pass's own name for it, the lab's, so a
//! parameter named as a WGSL reserved word (`type`, `smooth`, `from`, `_`) is still a member WGSL admits.
//!
//! **A pass** is its registry file's WGSL, a function `fn run(uv: vec2<f32>) -> vec4<f32>`, the lab's `vec4 run()`
//! in WGSL, and any helpers it declares at module level (a `fn`, `const`, `struct`, …), which the module names
//! `print_n<k>_pass<j>_h_<name>`, so they are the pass's own. It is written with the lab's names, which the generated
//! module renames: `in_<port>`, `prev`, `p_<param>`, `res`
//! and `px` (the target's size in pixels, and one pixel in UV), and `tex(t, uv)`, a texture read. `uv` is the
//! coordinate convention's, bottom-left origin, Y-up (`shaders/wgsl/lib/coords.wgsl`), as the lab's is.
//!
//! **Bindings.** Group [`FRAME_GROUP`] holds the frame (`res`, `px`) at 0 and the sampler at 1; group
//! [`TEXTURE_GROUP`] the inputs' and `prev` textures, numbered in order; group [`UNIFORM_GROUP`] node `k`'s uniform at
//! binding `k`. Each texture binding says what it reads ([`Reads`]): the stain's finished colours (a Render node, or
//! an input defaulting to the render), another node's output, a declared default, or the previous pass. The executor
//! binds them and runs the passes in [`Generated::passes`]' order.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use super::graph::{Expanded, Value, RENDER};
use super::ports::{validate, Feed, InputDefault};
use super::registry::{check_params, Entry, ParamKind, Registry};
use crate::assemble::block_comment_len;

/// The namespace of every name the Print graph's module declares; the stain's codegen refuses it.
pub const PREFIX: &str = "print_";

/// The bind group of the frame and the sampler.
pub const FRAME_GROUP: u32 = 0;
/// The bind group of the nodes' textures.
pub const TEXTURE_GROUP: u32 = 1;
/// The bind group of the nodes' uniforms.
pub const UNIFORM_GROUP: u32 = 2;

/// The coordinate convention's one flip and UV, which every pass's `uv` is in.
const COORDS: &str = include_str!("../../shaders/wgsl/lib/coords.wgsl");

/// What a texture binding reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reads {
    /// The stain's finished colours: a wire from a Render node, or an input defaulting to the render.
    Source,
    /// Node `node`'s output `port`: its last pass's target.
    Node { node: String, port: String },
    /// An unconnected input's declared default other than the render.
    Default(InputDefault),
    /// Node `node`'s previous pass, black before its first.
    Previous { node: String },
}

/// One texture binding of [`TEXTURE_GROUP`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    pub binding: u32,
    pub name: String,
    pub reads: Reads,
}

/// Node `node`'s uniform, at binding `binding` of [`UNIFORM_GROUP`]: each parameter's name and value, in declaration
/// order (the member of parameter `x` is `p_x`); a number or a choice's index one f32, a colour three, its hex digits over 255, as the lab sets them.
#[derive(Clone, Debug, PartialEq)]
pub struct Uniforms {
    pub node: String,
    pub binding: u32,
    pub name: String,
    pub values: Vec<(String, Vec<f32>)>,
}

/// One pass, in the order the executor runs them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassEntry {
    pub node: String,
    pub ty: String,
    pub pass: usize,
    pub function: String,
    pub entry_point: String,
}

/// A generated module: its WGSL, the nodes in run order, and its passes, textures and uniforms.
#[derive(Clone, Debug, PartialEq)]
pub struct Generated {
    pub source: String,
    pub order: Vec<String>,
    pub passes: Vec<PassEntry>,
    pub textures: Vec<Texture>,
    pub uniforms: Vec<Uniforms>,
}

/// Why a graph did not generate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenError(pub String);

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CodegenError {}

/// `graph`'s module, generated from `registry`; refused where its wiring does not validate
/// ([`super::ports::validate`]), a parameter is not its type's or out of its range ([`check_params`]), its nodes
/// read each other in a cycle, or a pass's block comment is never closed.
pub fn generate(graph: &Expanded, registry: &Registry) -> Result<Generated, CodegenError> {
    let wiring = validate(graph, registry).map_err(|e| CodegenError(e.0))?;
    let order = run_order(&wiring.inputs)?;
    let mut out = Generated {
        source: format!(
            "// The Print graph `{}`, generated from its registry (principia_print_graph.md §4.3): its own module, which\n\
             // the stain's codegen never sees.\n\n{COORDS}\n",
            graph.name.escape_debug()
        ),
        order: order.clone(),
        passes: Vec::new(),
        textures: Vec::new(),
        uniforms: Vec::new(),
    };
    let _ = write!(
        out.source,
        "struct PrintFrame {{\n    res: vec2<f32>,\n    px: vec2<f32>,\n}}\n\
         @group({FRAME_GROUP}) @binding(0) var<uniform> print_frame: PrintFrame;\n\
         @group({FRAME_GROUP}) @binding(1) var print_sampler: sampler;\n\n\
         // A texture of the graph read at `uv`, Y-up: its rows are the framebuffer's, Y-down, so the read takes the\n\
         // convention's one flip, at the boundary the texture was written across.\n\
         fn print_tex(t: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {{\n    \
         return textureSampleLevel(t, print_sampler, vec2<f32>(uv.x, flip_y(uv.y, 1.0)), 0.0);\n}}\n"
    );
    for (k, id) in order.iter().enumerate() {
        let node = &graph.nodes[id];
        let Some(entry) = registry.get(&node.ty) else {
            return Err(CodegenError(format!(
                "node `{id}`'s type is not registered"
            )));
        };
        check_params(id, entry, &node.params).map_err(CodegenError)?;
        node_module(
            &mut out,
            k,
            id,
            entry,
            &node.params,
            &wiring.inputs[id],
            graph,
        )?;
    }
    Ok(out)
}

/// The node ids of `inputs` in run order: each after every node it reads, ties broken by id; refused on a cycle.
fn run_order(inputs: &BTreeMap<String, Vec<(String, Feed)>>) -> Result<Vec<String>, CodegenError> {
    let reads = |id: &String| -> BTreeSet<&String> {
        inputs[id]
            .iter()
            .filter_map(|(_, f)| match f {
                Feed::Wire { node, .. } => Some(node),
                Feed::Default(_) => None,
            })
            .collect()
    };
    let mut done: BTreeSet<&String> = BTreeSet::new();
    let mut order = Vec::with_capacity(inputs.len());
    while order.len() < inputs.len() {
        let next = inputs
            .keys()
            .find(|id| !done.contains(id) && reads(id).iter().all(|r| done.contains(r)));
        let Some(next) = next else {
            let left: Vec<&String> = inputs.keys().filter(|id| !done.contains(id)).collect();
            return Err(CodegenError(format!(
                "the nodes {left:?} read each other in a cycle"
            )));
        };
        done.insert(next);
        order.push(next.clone());
    }
    Ok(order)
}

/// Node `k`, `id`, of type `entry`, its parameters checked: its textures, its uniform and its passes, onto `out`;
/// refused where a pass's block comment is never closed.
fn node_module(
    out: &mut Generated,
    k: usize,
    id: &str,
    entry: &Entry,
    params: &BTreeMap<String, Value>,
    feeds: &[(String, Feed)],
    graph: &Expanded,
) -> Result<(), CodegenError> {
    let base = format!("{PREFIX}n{k}");
    let _ = write!(
        out.source,
        "\n// Node {} ({}): {} pass(es).\n",
        id.escape_debug(),
        entry.id,
        entry.passes.len()
    );
    let mut texture = |name: String, reads: Reads, source: &mut String| {
        let binding = u32::try_from(out.textures.len()).unwrap_or(u32::MAX);
        let _ = writeln!(
            source,
            "@group({TEXTURE_GROUP}) @binding({binding}) var {name}: texture_2d<f32>;"
        );
        out.textures.push(Texture {
            binding,
            name,
            reads,
        });
    };
    for (port, feed) in feeds {
        let reads = match feed {
            Feed::Wire { node, .. } if graph.nodes[node].ty == RENDER => Reads::Source,
            Feed::Wire { node, port } => Reads::Node {
                node: node.clone(),
                port: port.clone(),
            },
            Feed::Default(InputDefault::Render) => Reads::Source,
            Feed::Default(d) => Reads::Default(*d),
        };
        texture(format!("{base}_in_{port}"), reads, &mut out.source);
    }
    if entry.passes.len() > 1 {
        let reads = Reads::Previous {
            node: id.to_owned(),
        };
        texture(format!("{base}_prev"), reads, &mut out.source);
    }
    if !entry.params.is_empty() {
        let ty = format!("PrintN{k}");
        let mut values = Vec::with_capacity(entry.params.len());
        let _ = writeln!(out.source, "struct {ty} {{");
        for p in &entry.params {
            let value = params.get(&p.name).unwrap_or(&p.default);
            let wgsl = if matches!(p.kind, ParamKind::Colour) {
                "vec3<f32>"
            } else {
                "f32"
            };
            let _ = writeln!(out.source, "    p_{}: {wgsl},", p.name);
            values.push((p.name.clone(), uniform_value(value)));
        }
        let binding = u32::try_from(k).unwrap_or(u32::MAX);
        let _ = writeln!(
            out.source,
            "}}\n@group({UNIFORM_GROUP}) @binding({binding}) var<uniform> {base}: {ty};"
        );
        out.uniforms.push(Uniforms {
            node: id.to_owned(),
            binding,
            name: base.clone(),
            values,
        });
    }
    for (j, pass) in entry.passes.iter().enumerate() {
        let function = format!("{base}_pass{j}");
        let entry_point = format!("{function}_main");
        let Some(wgsl) = rename(&pass.wgsl, &base, &function) else {
            return Err(CodegenError(format!(
                "node `{id}`'s pass `{}` has an unclosed `/*` comment",
                pass.file
            )));
        };
        let _ = write!(
            out.source,
            "\n// {} pass {j}, from {}.\n{}\n@fragment\nfn {entry_point}(@builtin(position) frag: vec4<f32>) -> \
             @location(0) vec4<f32> {{\n    return {function}(frag_uv(frag.xy, print_frame.res));\n}}\n",
            entry.id,
            pass.file,
            wgsl.trim_end()
        );
        out.passes.push(PassEntry {
            node: id.to_owned(),
            ty: entry.id.clone(),
            pass: j,
            function,
            entry_point,
        });
    }
    Ok(())
}

/// An admitted parameter value as its uniform holds it: a number, one f32; a colour `#rrggbb`, three, each pair of hex
/// digits over 255.
fn uniform_value(value: &Value) -> Vec<f32> {
    match value {
        // The uniform is f32, as the lab's `uniform1f` is.
        Value::Number(v) => vec![*v as f32],
        Value::Text(t) => {
            let hex = t.strip_prefix('#').filter(|h| h.len() == 6);
            let channels: Option<Vec<f32>> = hex.and_then(|h| {
                (0..3)
                    .map(|i| {
                        h.get(2 * i..2 * i + 2)
                            .and_then(|c| u8::from_str_radix(c, 16).ok())
                            .map(|c| f32::from(c) / 255.0)
                    })
                    .collect()
            });
            channels.unwrap_or_default()
        }
    }
}

/// A pass's WGSL with the lab's names renamed into node `base`'s namespace (the module docs): every identifier but
/// those in comments. The lab's names are reserved in a pass: a local named `res` or `p_x` is renamed too. The pass's
/// own module-level declarations ([`declarations`]: a helper `fn`, a `const`, `override`, `var`, `struct` or `alias`)
/// are its pass's, `<function>_h_<name>`, so two nodes of one type, or two passes declaring one name, never declare it
/// twice in the module; their every use is renamed with them, but a member (after `.`, or declared in a struct's
/// body) and a literal's suffix (after a digit), which never name a declaration. A block comment ends at the `*/`
/// closing it, its nested comments closed first, as WGSL's do; `None` where one is never closed.
fn rename(wgsl: &str, base: &str, function: &str) -> Option<String> {
    let own = declarations(wgsl);
    let mut out = String::with_capacity(wgsl.len() + 64);
    let mut rest = wgsl;
    // Whether a `struct` keyword awaits its body, and whether the scan is in one: WGSL declares a struct at module
    // level only, its body holding no braces.
    let mut struct_next = false;
    let mut struct_body = false;
    // The previous token, read from the source, never from `out`, which holds copied comments; comments are skipped:
    // whether it was a `.`, whitespace between skipped too, and whether it was a digit with no whitespace between (a
    // comment there splits a literal from its suffix, which is no valid WGSL either way).
    let mut after_dot = false;
    let mut after_digit = false;
    while let Some(c) = rest.chars().next() {
        let end = if rest.starts_with("//") {
            rest.find('\n').unwrap_or(rest.len())
        } else if rest.starts_with("/*") {
            block_comment_len(rest.as_bytes())?
        } else if c.is_ascii_alphabetic() || c == '_' {
            let end = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(rest.len());
            let word = &rest[..end];
            let member = after_dot
                || after_digit
                || (struct_body && rest[end..].trim_start().starts_with(':'));
            after_dot = false;
            after_digit = false;
            match renamed(word, base, function) {
                Some(name) => out.push_str(&name),
                None if own.contains(word) && !member => {
                    let _ = write!(out, "{function}_h_{word}");
                }
                None => out.push_str(word),
            }
            struct_next |= word == "struct";
            rest = &rest[end..];
            continue;
        } else {
            match c {
                '{' => {
                    struct_body = struct_next;
                    struct_next = false;
                }
                '}' => struct_body = false,
                _ => {}
            }
            if !c.is_whitespace() {
                after_dot = c == '.';
            }
            after_digit = c.is_ascii_digit();
            c.len_utf8()
        };
        out.push_str(&rest[..end]);
        rest = &rest[end..];
    }
    Some(out)
}

/// The names a pass's WGSL declares at module level, outside every brace and comment (a block comment nested, as in
/// [`rename`]): the name after `fn`, `const`, `override`, `struct`, `alias`, or `var` and its `<…>`.
fn declarations(wgsl: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    let mut rest = wgsl;
    let mut depth = 0usize;
    let mut declaring = false;
    while let Some(c) = rest.chars().next() {
        let end = if rest.starts_with("//") {
            rest.find('\n').unwrap_or(rest.len())
        } else if rest.starts_with("/*") {
            // Unclosed, it runs to the end; [`rename`] refuses the pass.
            block_comment_len(rest.as_bytes()).unwrap_or(rest.len())
        } else if c.is_ascii_alphabetic() || c == '_' {
            let end = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(rest.len());
            let word = &rest[..end];
            if declaring {
                names.insert(word);
                declaring = false;
            } else if depth == 0 {
                declaring = matches!(
                    word,
                    "fn" | "const" | "override" | "struct" | "alias" | "var"
                );
                let after = rest[end..].trim_start();
                if word == "var" && after.starts_with('<') {
                    // `var<private>`: the name comes after the template, from its `>` on.
                    let skip = rest.len() - after.len() + after.find('>').unwrap_or(after.len());
                    rest = &rest[skip..];
                    continue;
                }
            }
            end
        } else {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            c.len_utf8()
        };
        rest = &rest[end..];
    }
    names
}

/// The module's name for the lab's name `word` in node `base`'s pass `function`, if it is one.
fn renamed(word: &str, base: &str, function: &str) -> Option<String> {
    match word {
        "run" => Some(function.to_owned()),
        "tex" => Some(format!("{PREFIX}tex")),
        "res" | "px" => Some(format!("{PREFIX}frame.{word}")),
        "prev" => Some(format!("{base}_prev")),
        _ => match (word.strip_prefix("in_"), word.strip_prefix("p_")) {
            (Some(port), _) => Some(format!("{base}_in_{port}")),
            (None, Some(_)) => Some(format!("{base}.{word}")),
            (None, None) => None,
        },
    }
}
