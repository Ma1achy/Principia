//! `cargo xtask lint wgsl` — the WGSL traps of the generated unpack layer (render contract Part 5, "Unpack layer";
//! payload §6; REQ-RENDER-001), checked over naga's IR. It parses and validates
//! `crates/render/frag/generated/payload_unpack.wgsl` and fails, naming the rule, on:
//! - an `extractBits` whose argument is not u32: the i32 overload sign-extends ([`Rule::ExtractBitsU32`]);
//! - any f64 type: WGSL has none ([`Rule::NoF64`]);
//! - an `enable f16` directive, or any f16 type: f16 pairs are read through core `unpack2x16float`, which needs no
//!   `shader-f16` ([`Rule::NoEnableF16`]);
//! - a `SimState*` struct whose `r`, `p`, `r_sh` or `p_sh` is not `array<vec2<f32>, 3>`, or that lacks `r` or `p`
//!   ([`Rule::Vec2Groups`]);
//! - a word buffer that is not its own binding: `word_buffer` must be a storage global `array<vec4<u32>>` with a
//!   binding no other global shares, read only at a per-sample index (a function argument), as the `SimState` buffer
//!   is; and no `SimState*` struct may hold a `vec4<u32>` ([`Rule::WordBinding`]);
//! - a buffer off R-343's bindings, the numbers of the ledger's one table ([`ledger::payload::bindings`]):
//!   `simstate_buffer: array<SimStateFTLE>` at `@group(1) @binding(0)` and `word_buffer: array<vec4<u32>>` at
//!   `@group(1) @binding(1)`, each attribute equal to the generated `<PREFIX>_GROUP` and `<PREFIX>_BINDING` constants,
//!   and no binding in group 0, the assembler's per-frame uniforms ([`Rule::Bindings`]);
//! - a use of either buffer in any function but its one reader, `sample_state` or `sample_word` ([`Rule::SampleOnly`]).
//!
//! naga folds a call whose arguments are all constant before the IR is built, so the `extractBits` rule sees only
//! calls on a runtime value, which every generated accessor's is (a parameter). The `enable` directive is not kept in
//! the IR, so that rule reads the source's directives, comments stripped.

use std::fmt;
use std::path::Path;

use naga::{
    AddressSpace, ArraySize, Expression, MathFunction, Module, Scalar, ScalarKind, TypeInner,
    VectorSize,
};

/// The generated file the lint checks, relative to the workspace root.
pub const GENERATED: &str = "crates/render/frag/generated/payload_unpack.wgsl";

/// The word buffer's global, and the `SimState` buffer's.
pub const WORD_BUFFER: &str = "word_buffer";
pub const SIMSTATE_BUFFER: &str = "simstate_buffer";

/// The struct-name prefix of the stored `SimState` layouts.
const SIMSTATE: &str = "SimState";

/// The members that are vec2-grouped (R-86), and those every `SimState*` layout has.
const VEC2_GROUPED: [&str; 4] = ["r", "p", "r_sh", "p_sh"];
const ALWAYS: [&str; 2] = ["r", "p"];

/// The rule a finding breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    ExtractBitsU32,
    NoF64,
    NoEnableF16,
    Vec2Groups,
    WordBinding,
    Bindings,
    SampleOnly,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Rule::ExtractBitsU32 => "extractBits-u32",
            Rule::NoF64 => "no-f64",
            Rule::NoEnableF16 => "no-enable-f16",
            Rule::Vec2Groups => "vec2-groups",
            Rule::WordBinding => "word-binding",
            Rule::Bindings => "bindings",
            Rule::SampleOnly => "sample-only",
        })
    }
}

/// One finding: the rule broken and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: Rule,
    pub what: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.rule, self.what)
    }
}

/// Lints the generated WGSL of the workspace whose `Cargo.toml` is `manifest`.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let path = root.join(GENERATED);
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let found = check(&source).map_err(|e| format!("{GENERATED}: {e}"))?;
    if found.is_empty() {
        println!("lint wgsl: {GENERATED}: every rule holds");
        return Ok(());
    }
    for f in &found {
        eprintln!("lint wgsl: {GENERATED}: {f}");
    }
    Err(format!(
        "lint wgsl: {} finding(s) in {GENERATED}",
        found.len()
    ))
}

/// Every finding in `source`, or why it could not be checked: it does not parse or validate as WGSL.
pub fn check(source: &str) -> Result<Vec<Finding>, String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| e.emit_to_string(source))?;
    let mut found = Vec::new();
    found.extend(extract_bits(&module, &info));
    found.extend(scalars(&module));
    found.extend(enable_f16(source));
    found.extend(vec2_groups(&module));
    found.extend(word_binding(&module));
    found.extend(bindings(&module));
    found.extend(sample_only(&module));
    Ok(found)
}

fn finding(rule: Rule, what: String) -> Finding {
    Finding { rule, what }
}

/// Each `extractBits` whose argument is not u32 (a scalar or a vector of u32), naming its function.
fn extract_bits(module: &Module, info: &naga::valid::ModuleInfo) -> Vec<Finding> {
    let mut found = Vec::new();
    let functions = module.functions.iter().map(|(h, f)| (f, &info[h])).chain(
        module
            .entry_points
            .iter()
            .enumerate()
            .map(|(i, ep)| (&ep.function, info.get_entry_point(i))),
    );
    for (function, fi) in functions {
        for (_, expr) in function.expressions.iter() {
            let Expression::Math {
                fun: MathFunction::ExtractBits,
                arg,
                ..
            } = *expr
            else {
                continue;
            };
            let scalar = match fi[arg].ty.inner_with(&module.types) {
                TypeInner::Scalar(s) | TypeInner::Vector { scalar: s, .. } => Some(*s),
                _ => None,
            };
            if scalar != Some(Scalar::U32) {
                let name = function.name.as_deref().unwrap_or("(unnamed)");
                found.push(finding(
                    Rule::ExtractBitsU32,
                    format!("`{name}` calls extractBits on a non-u32 argument ({scalar:?}): the i32 overload sign-extends"),
                ));
            }
        }
    }
    found
}

/// Each f64 or f16 type in the module, under its rule.
fn scalars(module: &Module) -> Vec<Finding> {
    let mut found = Vec::new();
    for (_, ty) in module.types.iter() {
        let scalar = match ty.inner {
            TypeInner::Scalar(s)
            | TypeInner::Vector { scalar: s, .. }
            | TypeInner::Matrix { scalar: s, .. } => s,
            _ => continue,
        };
        if scalar.kind != ScalarKind::Float {
            continue;
        }
        let name = ty.name.as_deref().unwrap_or("");
        match scalar.width {
            8 => found.push(finding(
                Rule::NoF64,
                format!("an f64 type {name}{:?}: WGSL has no f64", ty.inner),
            )),
            2 => found.push(finding(
                Rule::NoEnableF16,
                format!(
                    "an f16 type {name}{:?}: f16 pairs are read through unpack2x16float",
                    ty.inner
                ),
            )),
            _ => {}
        }
    }
    found
}

/// `source` with its `//` and `/* */` comments blanked.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut depth = 0u32;
    while let Some(c) = chars.next() {
        if depth > 0 {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                depth -= 1;
            } else if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                depth += 1;
            }
            out.push(if c == '\n' { '\n' } else { ' ' });
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for d in chars.by_ref() {
                if d == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            depth = 1;
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
}

/// An `enable` directive naming `f16`, comments stripped.
fn enable_f16(source: &str) -> Vec<Finding> {
    let code = strip_comments(source);
    let mut found = Vec::new();
    for directive in code.split(';') {
        let mut words = directive.split(|c: char| c.is_whitespace() || c == ',');
        let words: Vec<&str> = words.by_ref().filter(|w| !w.is_empty()).collect();
        if words.first() == Some(&"enable") && words[1..].contains(&"f16") {
            found.push(finding(
                Rule::NoEnableF16,
                "an `enable f16` directive: f16 pairs are read through core unpack2x16float, no shader-f16".to_owned(),
            ));
        }
    }
    found
}

/// Whether `ty` is `array<vec2<f32>, 3>`.
fn is_vec2x3(module: &Module, ty: naga::Handle<naga::Type>) -> bool {
    let TypeInner::Array { base, size, .. } = module.types[ty].inner else {
        return false;
    };
    let three = matches!(size, ArraySize::Constant(n) if n.get() == 3);
    let vec2 = matches!(
        module.types[base].inner,
        TypeInner::Vector {
            size: VectorSize::Bi,
            scalar: Scalar::F32
        }
    );
    three && vec2
}

/// Each `SimState*` struct whose vec2-grouped members are not `array<vec2<f32>, 3>`, or that lacks `r` or `p`; none
/// at all is a finding.
fn vec2_groups(module: &Module) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut any = false;
    for (_, ty) in module.types.iter() {
        let (Some(name), TypeInner::Struct { members, .. }) = (ty.name.as_deref(), &ty.inner)
        else {
            continue;
        };
        if !name.starts_with(SIMSTATE) {
            continue;
        }
        any = true;
        for m in members {
            let Some(mname) = m.name.as_deref() else {
                continue;
            };
            if VEC2_GROUPED.contains(&mname) && !is_vec2x3(module, m.ty) {
                found.push(finding(
                    Rule::Vec2Groups,
                    format!("`{name}.{mname}` is not array<vec2<f32>, 3> (R-86)"),
                ));
            }
        }
        for want in ALWAYS {
            if !members.iter().any(|m| m.name.as_deref() == Some(want)) {
                found.push(finding(
                    Rule::Vec2Groups,
                    format!("`{name}` has no `{want}`"),
                ));
            }
        }
    }
    if !any {
        found.push(finding(
            Rule::Vec2Groups,
            "no SimState struct is declared".to_owned(),
        ));
    }
    found
}

/// Whether `ty` is a runtime-sized `array<vec4<u32>>`.
fn is_word_array(module: &Module, ty: naga::Handle<naga::Type>) -> bool {
    let TypeInner::Array { base, size, .. } = module.types[ty].inner else {
        return false;
    };
    size == ArraySize::Dynamic
        && matches!(
            module.types[base].inner,
            TypeInner::Vector {
                size: VectorSize::Quad,
                scalar: Scalar::U32
            }
        )
}

/// The word buffer's binding rule: `word_buffer` exists, is a storage `array<vec4<u32>>` with a binding of its own,
/// the `SimState` buffer exists with another binding, every read of either is at a function argument, and no
/// `SimState*` struct holds a `vec4<u32>`.
fn word_binding(module: &Module) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut bad = |what: String| found.push(finding(Rule::WordBinding, what));
    let global = |name: &str| {
        module
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some(name))
    };
    let word = global(WORD_BUFFER);
    let state = global(SIMSTATE_BUFFER);
    match word {
        None => bad(format!(
            "no `{WORD_BUFFER}` global: the word buffer is bound separately"
        )),
        Some((_, g)) => {
            if !matches!(g.space, AddressSpace::Storage { .. }) {
                bad(format!(
                    "`{WORD_BUFFER}` is not in the storage address space"
                ));
            }
            if !is_word_array(module, g.ty) {
                bad(format!(
                    "`{WORD_BUFFER}` is not array<vec4<u32>>, one word per sample"
                ));
            }
            match &g.binding {
                None => bad(format!("`{WORD_BUFFER}` has no binding")),
                Some(b) => {
                    let shared = module.global_variables.iter().any(|(_, o)| {
                        o.name.as_deref() != Some(WORD_BUFFER) && o.binding.as_ref() == Some(b)
                    });
                    if shared {
                        bad(format!(
                            "`{WORD_BUFFER}`'s binding is shared with another global"
                        ));
                    }
                }
            }
        }
    }
    if state.is_none_or(|(_, g)| g.binding.is_none()) {
        bad(format!(
            "no bound `{SIMSTATE_BUFFER}` global to index the word buffer like"
        ));
    }
    // Each read of either buffer is at a per-sample index: a function argument, not a constant.
    for (_, function) in module.functions.iter() {
        for (_, expr) in function.expressions.iter() {
            let (base, dynamic) = match *expr {
                Expression::Access { base, index } => (
                    base,
                    matches!(function.expressions[index], Expression::FunctionArgument(_)),
                ),
                Expression::AccessIndex { base, .. } => (base, false),
                _ => continue,
            };
            let Expression::GlobalVariable(g) = function.expressions[base] else {
                continue;
            };
            let name = module.global_variables[g].name.as_deref().unwrap_or("");
            if (name == WORD_BUFFER || name == SIMSTATE_BUFFER) && !dynamic {
                let fname = function.name.as_deref().unwrap_or("(unnamed)");
                bad(format!(
                    "`{fname}` reads `{name}` at an index other than its sample-index argument: the word buffer is indexed per copy, as samples are"
                ));
            }
        }
    }
    for (_, ty) in module.types.iter() {
        let (Some(name), TypeInner::Struct { members, .. }) = (ty.name.as_deref(), &ty.inner)
        else {
            continue;
        };
        if !name.starts_with(SIMSTATE) {
            continue;
        }
        for m in members {
            if matches!(
                module.types[m.ty].inner,
                TypeInner::Vector {
                    size: VectorSize::Quad,
                    scalar: Scalar::U32
                }
            ) {
                bad(format!(
                    "`{name}.{}` is a vec4<u32> inside SimState: the word lives in its own buffer",
                    m.name.as_deref().unwrap_or("")
                ));
            }
        }
    }
    found
}

/// The u32 value of the module constant `name`, if it is one.
fn constant(module: &Module, name: &str) -> Option<u32> {
    let (_, c) = module
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some(name))?;
    match module.global_expressions[c.init] {
        Expression::Literal(naga::Literal::U32(v)) => Some(v),
        _ => None,
    }
}

/// R-343's bindings, from the ledger's table: each buffer a storage global of its type at its group and binding, each
/// number equal to the generated constant, and no global bound in group 0.
fn bindings(module: &Module) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut bad = |what: String| found.push(finding(Rule::Bindings, what));
    for b in ledger::payload::bindings() {
        let want = if b.holds == "word" {
            "array<vec4<u32>>"
        } else {
            "array<SimStateFTLE>"
        };
        let Some((_, g)) = module
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some(b.buffer))
        else {
            bad(format!("no `{}` global (R-343)", b.buffer));
            continue;
        };
        let ty_ok = match module.types[g.ty].inner {
            TypeInner::Array {
                base,
                size: ArraySize::Dynamic,
                ..
            } => {
                if b.holds == "word" {
                    is_word_array(module, g.ty)
                } else {
                    module.types[base].name.as_deref() == Some("SimStateFTLE")
                }
            }
            _ => false,
        };
        if !ty_ok || !matches!(g.space, AddressSpace::Storage { .. }) {
            bad(format!("`{}` is not a storage `{want}` (R-343)", b.buffer));
        }
        match &g.binding {
            Some(r) if r.group == b.group && r.binding == b.binding => {}
            Some(r) => bad(format!(
                "`{}` is at @group({}) @binding({}), not @group({}) @binding({}) (R-343)",
                b.buffer, r.group, r.binding, b.group, b.binding
            )),
            None => bad(format!("`{}` has no binding (R-343)", b.buffer)),
        }
        for (suffix, number) in [("GROUP", b.group), ("BINDING", b.binding)] {
            let name = format!("{}_{suffix}", b.constant);
            match constant(module, &name) {
                Some(v) if v == number => {}
                Some(v) => bad(format!(
                    "`{name}` is {v}, not the table's {number} that `{}`'s attribute carries (R-343)",
                    b.buffer
                )),
                None => bad(format!("no u32 constant `{name}` (R-343)")),
            }
        }
    }
    for (_, g) in module.global_variables.iter() {
        if g.binding.as_ref().is_some_and(|r| r.group == 0) {
            bad(format!(
                "`{}` is bound in group 0, the assembler's per-frame uniforms (R-343)",
                g.name.as_deref().unwrap_or("(unnamed)")
            ));
        }
    }
    found
}

/// Each function or entry point but a buffer's one reader that uses the buffer (R-343: both buffers are read only
/// through `sample_state(i)` and `sample_word(i)`).
fn sample_only(module: &Module) -> Vec<Finding> {
    let table = ledger::payload::bindings();
    let functions = module
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(module.entry_points.iter().map(|ep| &ep.function));
    let mut found = Vec::new();
    for function in functions {
        let fname = function.name.as_deref().unwrap_or("(unnamed)");
        let mut used: Vec<&str> = Vec::new();
        for (_, expr) in function.expressions.iter() {
            let Expression::GlobalVariable(g) = *expr else {
                continue;
            };
            let gname = module.global_variables[g].name.as_deref().unwrap_or("");
            if let Some(b) = table.iter().find(|b| b.buffer == gname) {
                if b.reader != fname && !used.contains(&b.buffer) {
                    used.push(b.buffer);
                    found.push(finding(
                        Rule::SampleOnly,
                        format!(
                            "`{fname}` uses `{}`, which only `{}` reads (R-343)",
                            b.buffer, b.reader
                        ),
                    ));
                }
            }
        }
    }
    found
}
