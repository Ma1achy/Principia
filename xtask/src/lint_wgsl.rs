//! `cargo xtask lint wgsl` — the WGSL traps of the fragment stage (render contract Part 5, "Unpack layer"; payload
//! §6; REQ-RENDER-001, REQ-RENDER-083), checked over naga's IR. It parses and validates every WGSL file under
//! `crates/render/frag/` ([`FRAG_DIR`]), generated or written by hand (R-351).
//!
//! In every one of those files it fails, naming the file, the line and the rule, and naming a bit-pattern test (R-343)
//! as the fix, on the float checks fast-math (R-297) may optimise away, fold or break (R-351, R-352):
//! - any use of `isinf` or `isnan`: naga's `IsInf` and `IsNan`, or a call to a function of either name
//!   ([`Rule::IsInfNan`]);
//! - a float comparison one of whose operands is an inf or NaN constant, a constant expression that evaluates to one
//!   (`bitcast<f32>(0x7f800000u)`) included ([`Rule::InfNanConstant`]);
//! - a float scalar or vector compared with itself, by any of the six comparison operators: the same expression, or
//!   structurally equal reads of the same `let`, argument, variable or buffer element with no store to it between the
//!   two reads ([`Rule::SelfCompare`]);
//! - a comparison against a finite-max stand-in: ±65504 (`f16_finite_max`) or ±3.40282347e38 (f32's largest finite
//!   value), in any spelling, as a `bitcast<f32>` of its bit pattern or as another constant expression
//!   ([`Rule::FiniteMax`]).
//!
//! The generated file, `crates/render/frag/generated/payload_unpack.wgsl`, is also checked against the unpack
//! layer's own rules:
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
//! the IR, so that rule reads the source's directives, comments stripped. A hand-written file is held to the float
//! rules only: R-317's ban on `enable f16` covers the generated WGSL, so a hand-written file may use f16, and the
//! validator is built with every capability, `SHADER_FLOAT16` among them. naga does not fold a `bitcast`, nor
//! any constant expression over one (arithmetic, a math call, a component), so the float rules evaluate an operand's
//! constant expression themselves, in f32 for f32 ([`constant_floats`]).

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use naga::{
    AddressSpace, Arena, ArraySize, BinaryOperator, Block, Expression, Function, Handle, Literal,
    MathFunction, Module, RelationalFunction, Scalar, ScalarKind, Span, Statement, TypeInner,
    UnaryOperator, VectorSize,
};

/// The generated file the lint checks, relative to the workspace root.
pub const GENERATED: &str = "crates/render/frag/generated/payload_unpack.wgsl";

/// The fragment stage's WGSL: every `.wgsl` file under this directory, relative to the workspace root (R-351).
pub const FRAG_DIR: &str = "crates/render/frag";

/// The fix every float-rule finding names (R-343).
pub const BIT_PATTERN_FIX: &str =
    "fix: test the bit pattern instead (R-343), as `pa_d_min_is_unset` does: \
     `extractBits(w, 16u, 16u) == PA_D_MIN_UNSET`";

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
    IsInfNan,
    InfNanConstant,
    SelfCompare,
    FiniteMax,
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
            Rule::IsInfNan => "isinf-isnan",
            Rule::InfNanConstant => "inf-nan-constant",
            Rule::SelfCompare => "self-compare",
            Rule::FiniteMax => "finite-max",
        })
    }
}

/// One finding: the rule broken, what breaks it, and its 1-based source line where the rule has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: Rule,
    pub what: String,
    pub line: Option<u32>,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: [{}] {}", self.rule, self.what),
            None => write!(f, "[{}] {}", self.rule, self.what),
        }
    }
}

impl Finding {
    /// The finding in `file` (relative to the workspace root): `file:line: [rule] what`, or `file: [rule] what`.
    pub fn at(&self, file: &str) -> String {
        match self.line {
            Some(line) => format!("{file}:{line}: [{}] {}", self.rule, self.what),
            None => format!("{file}: [{}] {}", self.rule, self.what),
        }
    }
}

/// One linted file: its path relative to the workspace root, `/`-separated, and its findings.
#[derive(Clone, Debug)]
pub struct FileReport {
    pub file: String,
    pub findings: Vec<Finding>,
}

/// Lints the fragment-stage WGSL of the workspace whose `Cargo.toml` is `manifest`: every file [`lint`] reads.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let reports = lint(root)?;
    let mut total = 0;
    for r in &reports {
        if r.findings.is_empty() {
            println!("lint wgsl: {}: every rule holds", r.file);
        }
        for f in &r.findings {
            eprintln!("lint wgsl: {}", f.at(&r.file));
        }
        total += r.findings.len();
    }
    if total == 0 {
        return Ok(());
    }
    let failing: Vec<&str> = reports
        .iter()
        .filter(|r| !r.findings.is_empty())
        .map(|r| r.file.as_str())
        .collect();
    Err(format!(
        "lint wgsl: {total} finding(s) in {}",
        failing.join(", ")
    ))
}

/// Every `.wgsl` file under `root`'s [`FRAG_DIR`], linted: [`GENERATED`], which must exist, by [`check`], and every
/// other file, written by hand, by [`check_fragment`]. A file that does not parse or validate is an error naming it.
pub fn lint(root: &Path) -> Result<Vec<FileReport>, String> {
    let generated = root.join(GENERATED);
    if !generated.is_file() {
        return Err(format!("{}: no such file", generated.display()));
    }
    let mut files = Vec::new();
    wgsl_files(&root.join(FRAG_DIR), &mut files)?;
    files.sort();
    let mut reports = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let source =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let findings = if rel == GENERATED {
            check(&source)
        } else {
            check_fragment(&source)
        }
        .map_err(|e| format!("{rel}: {e}"))?;
        reports.push(FileReport {
            file: rel,
            findings,
        });
    }
    Ok(reports)
}

/// Each `.wgsl` file under `dir`, recursively, into `out`.
fn wgsl_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() {
            wgsl_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "wgsl") {
            out.push(path);
        }
    }
    Ok(())
}

/// `source` parsed and validated, with every capability (`SHADER_FLOAT16` among them, for a hand-written f16 file).
fn parse(source: &str) -> Result<(Module, naga::valid::ModuleInfo), String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    let info = validate(&module, source)?;
    Ok((module, info))
}

/// `module` validated, with every capability; an error is rendered against `source`.
fn validate(module: &Module, source: &str) -> Result<naga::valid::ModuleInfo, String> {
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(module)
    .map_err(|e| e.emit_to_string(source))
}

/// The float rules' findings in a fragment-stage WGSL file written by hand, or why it could not be checked: it does
/// not parse or validate as WGSL.
pub fn check_fragment(source: &str) -> Result<Vec<Finding>, String> {
    let (module, info) = parse(source)?;
    Ok(float_checks(&module, &info, source))
}

/// The float rules' findings in `module`, IR from any front end, whose spans index `source`; or why it could not be
/// checked: it does not validate. naga's `IsInf` and `IsNan` reach the IR only from front ends other than WGSL's.
pub fn check_fragment_module(module: &Module, source: &str) -> Result<Vec<Finding>, String> {
    let info = validate(module, source)?;
    Ok(float_checks(module, &info, source))
}

/// Every finding in the generated file's `source`, by the unpack layer's rules and the float rules, or why it could
/// not be checked: it does not parse or validate as WGSL.
pub fn check(source: &str) -> Result<Vec<Finding>, String> {
    let (module, info) = parse(source)?;
    let mut found = Vec::new();
    found.extend(extract_bits(&module, &info));
    found.extend(scalars(&module));
    found.extend(enable_f16(source));
    found.extend(vec2_groups(&module));
    found.extend(word_binding(&module));
    found.extend(bindings(&module));
    found.extend(sample_only(&module));
    found.extend(float_checks(&module, &info, source));
    Ok(found)
}

fn finding(rule: Rule, what: String) -> Finding {
    Finding {
        rule,
        what,
        line: None,
    }
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
pub fn strip_comments(source: &str) -> String {
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

// ---------------------------------------------------------------------------------------------------------------
// The float rules (R-351, R-352): isinf/isnan, inf or NaN constants, self-comparisons and finite-max stand-ins.

/// binary16's largest finite value (dd_generation_root §3.8's `f16_finite_max`), a stand-in for +inf (R-352).
const F16_FINITE_MAX: f64 = 65504.0;

/// A float rule's finding at `span`'s line in `source`.
fn float_finding(rule: Rule, what: String, span: Span, source: &str) -> Finding {
    Finding {
        rule,
        what: format!("{what}; {BIT_PATTERN_FIX}"),
        line: span.is_defined().then(|| span.location(source).line_number),
    }
}

/// Every float-rule finding in `module`, over every function and entry point.
fn float_checks(module: &Module, info: &naga::valid::ModuleInfo, source: &str) -> Vec<Finding> {
    let functions = module.functions.iter().map(|(h, f)| (f, &info[h])).chain(
        module
            .entry_points
            .iter()
            .enumerate()
            .map(|(i, ep)| (&ep.function, info.get_entry_point(i))),
    );
    let mut found = Vec::new();
    for (function, fi) in functions {
        let fname = function.name.as_deref().unwrap_or("(unnamed)");
        let mut statements = Vec::new();
        flatten(&function.body, &mut statements);
        for &(statement, span) in &statements {
            let Statement::Call {
                function: callee, ..
            } = *statement
            else {
                continue;
            };
            let name = module.functions[callee].name.as_deref().unwrap_or("");
            if is_inf_nan_name(name) {
                found.push(float_finding(
                    Rule::IsInfNan,
                    format!("`{fname}` calls `{name}`, which fast-math (R-297) may optimise away"),
                    span,
                    source,
                ));
            }
        }
        let timeline = Timeline::new(&function.expressions, &statements);
        for (h, expr) in function.expressions.iter() {
            let span = function.expressions.get_span(h);
            match *expr {
                Expression::Relational {
                    fun: fun @ (RelationalFunction::IsInf | RelationalFunction::IsNan),
                    ..
                } => found.push(float_finding(
                    Rule::IsInfNan,
                    format!("`{fname}` uses {fun:?}, which fast-math (R-297) may optimise away"),
                    span,
                    source,
                )),
                Expression::Binary { op, left, right } if is_comparison(op) => {
                    let float = |e: Handle<Expression>| {
                        matches!(
                            *fi[e].ty.inner_with(&module.types),
                            TypeInner::Scalar(Scalar {
                                kind: ScalarKind::Float,
                                ..
                            }) | TypeInner::Vector {
                                scalar: Scalar {
                                    kind: ScalarKind::Float,
                                    ..
                                },
                                ..
                            }
                        )
                    };
                    // Both operands of a comparison have one type (naga's validator), so the left one's is the
                    // comparison's.
                    if !float(left) {
                        continue;
                    }
                    found.extend(
                        comparison(module, function, &timeline, op, left, right)
                            .into_iter()
                            .map(|(rule, what)| {
                                float_finding(rule, format!("`{fname}`: {what}"), span, source)
                            }),
                    );
                }
                _ => {}
            }
        }
    }
    found
}

/// Whether `name` is `isinf` or `isnan`, in any case (`isInf`, `isNan`).
fn is_inf_nan_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("isinf") || name.eq_ignore_ascii_case("isnan")
}

/// Whether `op` is one of the six comparisons.
fn is_comparison(op: BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
    )
}

/// The float rules a float comparison `left op right` breaks, each with what breaks it.
fn comparison(
    module: &Module,
    function: &Function,
    timeline: &Timeline,
    op: BinaryOperator,
    left: Handle<Expression>,
    right: Handle<Expression>,
) -> Vec<(Rule, String)> {
    let mut found = Vec::new();
    if same(&function.expressions, timeline, left, right) {
        found.push((
            Rule::SelfCompare,
            format!(
                "a float compared with itself by `{}`, a NaN test fast-math (R-297) may fold",
                symbol(op)
            ),
        ));
    }
    for operand in [left, right] {
        let Some(values) = constant_floats(module, &function.expressions, operand) else {
            continue;
        };
        if let Some(v) = values.iter().find(|v| !v.is_finite()) {
            let constant = if v.is_nan() { "a NaN" } else { "an inf" };
            found.push((
                Rule::InfNanConstant,
                format!(
                    "a comparison by `{}` against {constant} constant, which fast-math (R-297) may fold",
                    symbol(op)
                ),
            ));
        }
        if let Some(v) = values.iter().find(|v| is_finite_max(**v)) {
            found.push((
                Rule::FiniteMax,
                format!(
                    "a comparison by `{}` against ±{:e}, a finite-max stand-in for inf, which fast-math (R-297) \
                     may break",
                    symbol(op),
                    v.abs()
                ),
            ));
        }
    }
    found
}

/// A comparison's WGSL operator.
fn symbol(op: BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::Less => "<",
        BinaryOperator::LessEqual => "<=",
        BinaryOperator::Greater => ">",
        _ => ">=",
    }
}

/// Whether `v` is ±65504 (f16's largest finite value) or ±3.40282347e38 (f32's).
fn is_finite_max(v: f64) -> bool {
    let a = v.abs();
    a == F16_FINITE_MAX || a == f64::from(f32::MAX)
}

/// The float values of `h`, if it is a constant expression of floats, its components in order; `None` if it is not
/// constant, holds an integer or a bool, or holds a value [`evaluate`] cannot give soundly. naga folds a constant
/// expression of literals before the IR is built, and rejects one at parse where WGSL makes it an error (the `sqrt` of
/// a negative literal, an overflowing `exp`), which the lint reports. It leaves unfolded any constant expression over a
/// `bitcast`, and some built-ins (`ldexp`, `mix`, `smoothstep`, `modf`, `frexp`, the packing built-ins) even over
/// literals, so [`evaluate`] evaluates what naga leaves, as the shader does at run time. naga concretises an abstract
/// literal or constant before the IR, so no abstract literal reaches here.
pub fn constant_floats(
    module: &Module,
    arena: &Arena<Expression>,
    h: Handle<Expression>,
) -> Option<Vec<f64>> {
    let mut floats = Vec::new();
    evaluate(module, arena, h)?.floats(&mut floats)?;
    Some(floats)
}

/// A constant expression's value: a float of `width` bytes, a u32, an i32, a bool, or a vector, matrix (a list of its
/// columns), array or built-in result struct (`modf`'s, `frexp`'s) of values.
#[derive(Clone, Debug)]
enum Value {
    Float(f64, u8),
    Uint(u32),
    Sint(i32),
    Bool(bool),
    List(Vec<Value>),
}

impl Value {
    /// Appends the value's floats to `out`, in order; `None` if it holds an integer or a bool.
    fn floats(&self, out: &mut Vec<f64>) -> Option<()> {
        match self {
            Value::Float(v, _) => out.push(*v),
            Value::List(items) => {
                for item in items {
                    item.floats(out)?;
                }
            }
            _ => return None,
        }
        Some(())
    }

    /// The `i`th component of a vector, matrix, array or struct.
    fn index(&self, i: usize) -> Option<Value> {
        match self {
            Value::List(items) => items.get(i).cloned(),
            _ => None,
        }
    }

    /// A matrix's columns, if the value is a matrix: a list of lists.
    fn columns(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) if matches!(items.first(), Some(Value::List(_))) => Some(items),
            _ => None,
        }
    }
}

/// `f` applied lane by lane: to `args` themselves if all are scalars, else to the components at each index of the
/// lists among them, a scalar meeting every lane (WGSL's vector-scalar forms), and recursively, so a matrix's columns
/// meet column by column. `None` if the lists' lengths differ or `f` is `None` for any lane.
fn lanes(args: &[Value], f: &dyn Fn(&[Value]) -> Option<Value>) -> Option<Value> {
    let mut lengths = args.iter().filter_map(|a| match a {
        Value::List(items) => Some(items.len()),
        _ => None,
    });
    let Some(n) = lengths.next() else {
        return f(args);
    };
    if lengths.any(|m| m != n) {
        return None;
    }
    (0..n)
        .map(|i| {
            let lane: Vec<Value> = args
                .iter()
                .map(|a| match a {
                    Value::List(items) => items[i].clone(),
                    scalar => scalar.clone(),
                })
                .collect();
            lanes(&lane, f)
        })
        .collect::<Option<_>>()
        .map(Value::List)
}

/// A lane's floats and their width; `None` if it holds anything else.
fn floats_of(lane: &[Value]) -> Option<(Vec<f64>, u8)> {
    let Some(&Value::Float(_, width)) = lane.first() else {
        return None;
    };
    let floats = lane
        .iter()
        .map(|v| match *v {
            Value::Float(x, _) => Some(x),
            _ => None,
        })
        .collect::<Option<_>>()?;
    Some((floats, width))
}

/// `f` applied lane by lane to float `args` ([`lanes`]), each result rounded to their width; `None` if a lane holds
/// anything but floats.
fn float_lanes(args: &[Value], f: &dyn Fn(&[f64]) -> f64) -> Option<Value> {
    lanes(args, &|lane| {
        let (x, width) = floats_of(lane)?;
        Some(Value::Float(round(f(&x), width), width))
    })
}

/// `v` rounded to a float of `width` bytes: f16 (2), f32 (4) or f64 (8).
fn round(v: f64, width: u8) -> f64 {
    match width {
        // f32 arithmetic done in f64 and rounded once to f32 is exact f32 arithmetic: f64 holds more than twice
        // f32's 24 significant bits. A transcendental function done in f64 and rounded to f32 is within WGSL's
        // accuracy for it, which is all the float rules need: whether the value is inf, NaN or a stand-in.
        4 => f64::from(v as f32),
        2 => round_f16(v),
        _ => v,
    }
}

/// `v` rounded to binary16, ties to even: 11 significant bits, in steps no finer than the subnormals' 2^-24, and ±inf
/// past 65504, f16's largest finite value (65520, the midpoint to 2^16, rounds to even: up).
fn round_f16(v: f64) -> f64 {
    let biased = i32::try_from(v.abs().to_bits() >> 52).unwrap_or(0);
    let step = 2f64.powi((biased - 1023).max(-14) - 10);
    let r = (v / step).round_ties_even() * step;
    if r.abs() > F16_FINITE_MAX {
        f64::INFINITY.copysign(v)
    } else {
        r
    }
}

/// The binary16 bit pattern of `v`, rounded to binary16 ([`round_f16`]): WGSL's `pack2x16float` of one component.
fn f16_bits(v: f64) -> u16 {
    let r = round_f16(v);
    if r.is_nan() {
        return 0x7e00;
    }
    let a = r.abs();
    let magnitude = if a == f64::INFINITY {
        0x7c00
    } else {
        // A finite f16 of exponent e (-14 for the subnormals and zero) and significand s/1024 is (e + 14) * 1024 + s.
        let e = if a == 0.0 { -14 } else { exponent(a).max(-14) };
        (f64::from(e + 14) * 1024.0 + a * 2f64.powi(10 - e)) as u16
    };
    u16::from(r.is_sign_negative()) * 0x8000 + magnitude
}

/// The value of the binary16 bit pattern `b`: WGSL's `unpack2x16float` of one component.
fn f16_value(b: u16) -> f64 {
    let e = (b >> 10) & 0x1f;
    let s = f64::from(b & 0x3ff);
    let magnitude = match e {
        31 if s == 0.0 => f64::INFINITY,
        31 => f64::NAN,
        0 => s * 2f64.powi(-24),
        _ => (s + 1024.0) * 2f64.powi(i32::from(e) - 25),
    };
    if b >= 0x8000 {
        -magnitude
    } else {
        magnitude
    }
}

/// `a`'s binary exponent, `floor(log2(a))`, for a finite `a > 0`.
fn exponent(a: f64) -> i32 {
    let biased = i32::try_from(a.to_bits() >> 52).unwrap_or(0);
    if biased == 0 {
        // An f64 subnormal: scaled into the normals first.
        return exponent(a * 2f64.powi(64)) - 64;
    }
    biased - 1023
}

/// `x * 2^e`, exactly where the result is an f64 normal: WGSL's `ldexp`, before rounding to the operand's width. `e` is
/// applied in two halves, each clamped to f64's normal exponents, so neither factor overflows nor underflows.
fn ldexp(x: f64, e: i32) -> f64 {
    let pow2 = |n: i32| 2f64.powi(n.clamp(-1022, 1023));
    let half = e / 2;
    x * pow2(half) * pow2(e - half)
}

/// WGSL's `frexp` of `x`: `(fract, exp)` with `x = fract * 2^exp` and `fract`'s magnitude in [0.5, 1), or `(x, 0)` for
/// a zero. `None` for inf or NaN, whose parts WGSL leaves indeterminate.
fn frexp(x: f64) -> Option<(f64, i32)> {
    if !x.is_finite() {
        return None;
    }
    if x == 0.0 {
        return Some((x, 0));
    }
    let exp = exponent(x.abs()) + 1;
    Some((ldexp(x, -exp), exp))
}

/// `op`'s result on the scalars `x` and `y`, if it is a comparison.
fn compare<T: PartialOrd>(op: BinaryOperator, x: T, y: T) -> Option<Value> {
    let r = match op {
        BinaryOperator::Equal => x == y,
        BinaryOperator::NotEqual => x != y,
        BinaryOperator::Less => x < y,
        BinaryOperator::LessEqual => x <= y,
        BinaryOperator::Greater => x > y,
        BinaryOperator::GreaterEqual => x >= y,
        _ => return None,
    };
    Some(Value::Bool(r))
}

/// The unary `op` on the scalar `v`, as WGSL evaluates it at run time (an i32's negation wraps).
fn unary(op: UnaryOperator, v: &Value) -> Option<Value> {
    match (op, v) {
        (UnaryOperator::Negate, &Value::Float(x, w)) => Some(Value::Float(-x, w)),
        (UnaryOperator::Negate, &Value::Sint(i)) => Some(Value::Sint(i.wrapping_neg())),
        (UnaryOperator::LogicalNot, &Value::Bool(b)) => Some(Value::Bool(!b)),
        (UnaryOperator::BitwiseNot, &Value::Uint(u)) => Some(Value::Uint(!u)),
        (UnaryOperator::BitwiseNot, &Value::Sint(i)) => Some(Value::Sint(!i)),
        _ => None,
    }
}

/// `a op b`: a matrix product if both are lists and one is a matrix, else lane by lane ([`lanes`]).
fn binary(op: BinaryOperator, a: Value, b: Value) -> Option<Value> {
    let lists = matches!((&a, &b), (Value::List(_), Value::List(_)));
    if op == BinaryOperator::Multiply && lists && (a.columns().is_some() || b.columns().is_some()) {
        return product(&a, &b);
    }
    lanes(&[a, b], &|lane| scalar_binary(op, &lane[0], &lane[1]))
}

/// `a op b` on scalars, as WGSL evaluates it at run time: float arithmetic rounded to the operands' width; integer
/// arithmetic wrapping, an integer division by zero giving `a` and a remainder by zero 0 (as do i32's `MIN / -1` and
/// `MIN % -1`), and shifts by the amount modulo 32; comparisons giving a bool.
fn scalar_binary(op: BinaryOperator, a: &Value, b: &Value) -> Option<Value> {
    use BinaryOperator as B;
    match (a, b) {
        (&Value::Float(x, w), &Value::Float(y, _)) => {
            let r = match op {
                B::Add => x + y,
                B::Subtract => x - y,
                B::Multiply => x * y,
                B::Divide => x / y,
                B::Modulo => x % y,
                _ => return compare(op, x, y),
            };
            Some(Value::Float(round(r, w), w))
        }
        (&Value::Uint(x), &Value::Uint(y)) => {
            let r = match op {
                B::Add => x.wrapping_add(y),
                B::Subtract => x.wrapping_sub(y),
                B::Multiply => x.wrapping_mul(y),
                B::Divide => x.checked_div(y).unwrap_or(x),
                B::Modulo => x.checked_rem(y).unwrap_or(0),
                B::And => x & y,
                B::ExclusiveOr => x ^ y,
                B::InclusiveOr => x | y,
                B::ShiftLeft => x.wrapping_shl(y),
                B::ShiftRight => x.wrapping_shr(y),
                _ => return compare(op, x, y),
            };
            Some(Value::Uint(r))
        }
        (&Value::Sint(x), &Value::Sint(y)) => {
            let r = match op {
                B::Add => x.wrapping_add(y),
                B::Subtract => x.wrapping_sub(y),
                B::Multiply => x.wrapping_mul(y),
                B::Divide => x.checked_div(y).unwrap_or(x),
                B::Modulo => x.checked_rem(y).unwrap_or(0),
                B::And => x & y,
                B::ExclusiveOr => x ^ y,
                B::InclusiveOr => x | y,
                _ => return compare(op, x, y),
            };
            Some(Value::Sint(r))
        }
        (&Value::Sint(x), &Value::Uint(y)) => match op {
            B::ShiftLeft => Some(Value::Sint(x.wrapping_shl(y))),
            B::ShiftRight => Some(Value::Sint(x.wrapping_shr(y))),
            _ => None,
        },
        (&Value::Bool(x), &Value::Bool(y)) => match op {
            B::And => Some(Value::Bool(x & y)),
            B::InclusiveOr => Some(Value::Bool(x | y)),
            _ => compare(op, x, y),
        },
        _ => None,
    }
}

/// The linear-algebra product `a * b`, one a matrix and the other a matrix or a vector, each multiplication and
/// addition rounded to the operands' width.
fn product(a: &Value, b: &Value) -> Option<Value> {
    if let Some(columns) = b.columns() {
        // Column `j` of `a * B` is `a * B[j]`; component `j` of `v * B` is `dot(v, B[j])`.
        return columns
            .iter()
            .map(|c| {
                if a.columns().is_some() {
                    product(a, c)
                } else {
                    dot(a, c)
                }
            })
            .collect::<Option<_>>()
            .map(Value::List);
    }
    // `a` is a matrix and `b` a vector: the sum of `a`'s columns, each times its component of `b`.
    let (Some(columns), Value::List(v)) = (a.columns(), b) else {
        return None;
    };
    columns
        .iter()
        .zip(v)
        .map(|(c, s)| binary(BinaryOperator::Multiply, c.clone(), s.clone()))
        .reduce(|sum, t| binary(BinaryOperator::Add, sum?, t?))?
}

/// The dot product of two vectors, each multiplication and addition rounded to the operands' width (integer ones
/// wrapping).
fn dot(a: &Value, b: &Value) -> Option<Value> {
    let (Value::List(x), Value::List(y)) = (a, b) else {
        return None;
    };
    x.iter()
        .zip(y)
        .map(|(p, q)| scalar_binary(BinaryOperator::Multiply, p, q))
        .reduce(|sum, t| scalar_binary(BinaryOperator::Add, &sum?, &t?))?
}

/// WGSL's `length`: a scalar's magnitude, or a vector's `sqrt(dot(v, v))`.
fn length(v: &Value) -> Option<Value> {
    match *v {
        Value::Float(x, w) => Some(Value::Float(x.abs(), w)),
        _ => float_lanes(&[dot(v, v)?], &|x| x[0].sqrt()),
    }
}

/// The float `x` of width `w`, as a value.
fn float(x: f64, w: u8) -> Value {
    Value::Float(x, w)
}

/// WGSL's `cross` of two 3-vectors.
fn cross(a: &Value, b: &Value) -> Option<Value> {
    let m =
        |i: usize, j: usize| scalar_binary(BinaryOperator::Multiply, &a.index(i)?, &b.index(j)?);
    let c = |i: usize, j: usize| scalar_binary(BinaryOperator::Subtract, &m(i, j)?, &m(j, i)?);
    Some(Value::List(vec![c(1, 2)?, c(2, 0)?, c(0, 1)?]))
}

/// WGSL's `reflect(e1, e2)`: `e1 - 2 * dot(e2, e1) * e2`.
fn reflect(e1: &Value, e2: &Value) -> Option<Value> {
    let Value::Float(d, w) = dot(e2, e1)? else {
        return None;
    };
    let scaled = binary(
        BinaryOperator::Multiply,
        e2.clone(),
        float(round(2.0 * d, w), w),
    )?;
    binary(BinaryOperator::Subtract, e1.clone(), scaled)
}

/// WGSL's `refract(e1, e2, eta)`: with `k = 1 - eta^2 * (1 - dot(e2, e1)^2)`, the zero vector if `k < 0`, else
/// `eta * e1 - (eta * dot(e2, e1) + sqrt(k)) * e2`.
fn refract(e1: &Value, e2: &Value, eta: &Value) -> Option<Value> {
    let (Value::Float(d, w), &Value::Float(eta, _)) = (dot(e2, e1)?, eta) else {
        return None;
    };
    let r = |x: f64| round(x, w);
    let k = r(1.0 - r(r(eta * eta) * r(1.0 - r(d * d))));
    if k < 0.0 {
        return lanes(std::slice::from_ref(e1), &|_| Some(float(0.0, w)));
    }
    let along = binary(BinaryOperator::Multiply, e1.clone(), float(eta, w))?;
    let across = binary(
        BinaryOperator::Multiply,
        e2.clone(),
        float(r(r(eta * d) + r(k.sqrt())), w),
    )?;
    binary(BinaryOperator::Subtract, along, across)
}

/// The determinant of the square matrix of `columns`, by cofactor expansion down the first row, each operation
/// rounded to the entries' width.
fn determinant(columns: &[Vec<Value>]) -> Option<Value> {
    if columns.len() == 1 {
        return columns[0].first().cloned();
    }
    let mut sum: Option<Value> = None;
    for (j, column) in columns.iter().enumerate() {
        let minor: Vec<Vec<Value>> = columns
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != j)
            .map(|(_, c)| c[1..].to_vec())
            .collect();
        let term = scalar_binary(
            BinaryOperator::Multiply,
            column.first()?,
            &determinant(&minor)?,
        )?;
        sum = Some(match sum {
            None => term,
            Some(s) if j % 2 == 0 => scalar_binary(BinaryOperator::Add, &s, &term)?,
            Some(s) => scalar_binary(BinaryOperator::Subtract, &s, &term)?,
        });
    }
    sum
}

/// A matrix's columns, each as a list of its entries.
fn matrix(m: &Value) -> Option<Vec<Vec<Value>>> {
    m.columns()?
        .iter()
        .map(|c| match c {
            Value::List(items) => Some(items.clone()),
            _ => None,
        })
        .collect()
}

/// `v`'s bits, if it is a u32, an i32 or a non-NaN f32 (a NaN's payload is not kept in an f64).
fn bits(v: &Value) -> Option<u32> {
    match *v {
        Value::Uint(u) => Some(u),
        Value::Sint(i) => Some(i.cast_unsigned()),
        Value::Float(x, 4) if !x.is_nan() => Some((x as f32).to_bits()),
        _ => None,
    }
}

/// The scalar `v` converted (`convert: Some(width)`, WGSL's `f32(e)`, `u32(e)`, …) or bitcast (`convert: None`) to
/// `kind`, as WGSL converts at run time: a float to an integer truncated and saturated, an integer to a float rounded
/// to its width. `None` for a NaN to an integer, which WGSL leaves indeterminate.
fn cast(v: &Value, kind: ScalarKind, convert: Option<u8>) -> Option<Value> {
    use ScalarKind as K;
    match (kind, convert, v) {
        (K::Float, Some(w), &Value::Float(x, _)) => Some(float(round(x, w), w)),
        (K::Float, Some(w), &Value::Uint(u)) => Some(float(round(f64::from(u), w), w)),
        (K::Float, Some(w), &Value::Sint(i)) => Some(float(round(f64::from(i), w), w)),
        (K::Float, Some(w), &Value::Bool(b)) => Some(float(f64::from(u8::from(b)), w)),
        (K::Float, None, &Value::Float(x, 4)) => Some(float(x, 4)),
        (K::Float, None, v) => Some(float(f64::from(f32::from_bits(bits(v)?)), 4)),
        (K::Uint, Some(_), &Value::Float(x, _)) => (!x.is_nan()).then_some(Value::Uint(x as u32)),
        (K::Sint, Some(_), &Value::Float(x, _)) => (!x.is_nan()).then_some(Value::Sint(x as i32)),
        (K::Uint, Some(_), &Value::Bool(b)) => Some(Value::Uint(u32::from(b))),
        (K::Sint, Some(_), &Value::Bool(b)) => Some(Value::Sint(i32::from(b))),
        (K::Uint, _, v) => Some(Value::Uint(bits(v)?)),
        (K::Sint, _, v) => Some(Value::Sint(bits(v)?.cast_signed())),
        (K::Bool, Some(_), &Value::Float(x, _)) => Some(Value::Bool(x != 0.0)),
        (K::Bool, Some(_), &Value::Uint(u)) => Some(Value::Bool(u != 0)),
        (K::Bool, Some(_), &Value::Sint(i)) => Some(Value::Bool(i != 0)),
        (K::Bool, Some(_), &Value::Bool(b)) => Some(Value::Bool(b)),
        _ => None,
    }
}

/// The zero value of type `ty`: a scalar, vector, matrix or fixed-size array (WGSL's `T()`).
fn zero(module: &Module, ty: Handle<naga::Type>) -> Option<Value> {
    let scalar = |s: Scalar| match s.kind {
        ScalarKind::Float => Some(float(0.0, s.width)),
        ScalarKind::Uint => Some(Value::Uint(0)),
        ScalarKind::Sint => Some(Value::Sint(0)),
        ScalarKind::Bool => Some(Value::Bool(false)),
        _ => None,
    };
    match module.types[ty].inner {
        TypeInner::Scalar(s) => scalar(s),
        TypeInner::Vector { size, scalar: s } => Some(Value::List(vec![scalar(s)?; size as usize])),
        TypeInner::Matrix {
            columns,
            rows,
            scalar: s,
        } => Some(Value::List(vec![
            Value::List(vec![
                scalar(s)?;
                rows as usize
            ]);
            columns as usize
        ])),
        TypeInner::Array {
            base,
            size: ArraySize::Constant(n),
            ..
        } => Some(Value::List(vec![
            zero(module, base)?;
            usize::try_from(n.get()).ok()?
        ])),
        _ => None,
    }
}

/// WGSL's `extractBits(e, offset, count)` on the scalar `e`: the `count` bits from `offset`, sign-extended for an i32,
/// with `offset` and `count` clamped to the word.
fn extract_field(e: &Value, offset: &Value, count: &Value) -> Option<Value> {
    let (&Value::Uint(o), &Value::Uint(c)) = (offset, count) else {
        return None;
    };
    let o = o.min(32);
    let c = c.min(32 - o);
    if c == 0 {
        return match *e {
            Value::Uint(_) => Some(Value::Uint(0)),
            Value::Sint(_) => Some(Value::Sint(0)),
            _ => None,
        };
    }
    match *e {
        Value::Uint(u) => Some(Value::Uint((u >> o) & (u32::MAX >> (32 - c)))),
        Value::Sint(i) => Some(Value::Sint((i << (32 - o - c)) >> (32 - c))),
        _ => None,
    }
}

/// WGSL's `insertBits(e, newbits, offset, count)` on the scalars `e` and `newbits`: `e` with its `count` bits from
/// `offset` replaced by `newbits`' low bits, `offset` and `count` clamped to the word.
fn insert_bits(e: &Value, newbits: &Value, offset: &Value, count: &Value) -> Option<Value> {
    let (&Value::Uint(o), &Value::Uint(c)) = (offset, count) else {
        return None;
    };
    let o = o.min(32);
    let c = c.min(32 - o);
    let (x, n) = (bits(e)?, bits(newbits)?);
    let r = if c == 0 {
        x
    } else {
        let mask = (u32::MAX >> (32 - c)) << o;
        (x & !mask) + ((n << o) & mask)
    };
    match *e {
        Value::Uint(_) => Some(Value::Uint(r)),
        _ => Some(Value::Sint(r.cast_signed())),
    }
}

/// The u32 whose little-endian bytes are `f` of each component of the vector `v`, in order: WGSL's packing built-ins.
fn pack(v: &Value, f: &dyn Fn(&Value) -> Option<Vec<u8>>) -> Option<Value> {
    let Value::List(items) = v else {
        return None;
    };
    let bytes: Vec<u8> = items.iter().map(f).collect::<Option<Vec<_>>>()?.concat();
    Some(Value::Uint(u32::from_le_bytes(bytes.try_into().ok()?)))
}

/// A float component's normalised quantisation, `floor(0.5 + scale * min(1, max(low, x)))`: WGSL's `pack*norm`.
fn quantise(v: &Value, low: f64, scale: f64) -> Option<f64> {
    let &Value::Float(x, _) = v else {
        return None;
    };
    Some((0.5 + scale * x.max(low).min(1.0)).floor())
}

/// `f` of each `chunk`-byte group of the u32 `v`'s little-endian bytes, in order: WGSL's unpacking built-ins.
fn unpack(v: &Value, chunk: usize, f: &dyn Fn(&[u8]) -> Value) -> Option<Value> {
    let &Value::Uint(u) = v else {
        return None;
    };
    Some(Value::List(u.to_le_bytes().chunks(chunk).map(f).collect()))
}

/// The i32 of each byte of `u`, sign-extended if `signed`.
fn byte_values(u: u32, signed: bool) -> [i32; 4] {
    u.to_le_bytes().map(|b| {
        if signed {
            i32::from(b.cast_signed())
        } else {
            i32::from(b)
        }
    })
}

/// WGSL's `dot4I8Packed` (`signed`) or `dot4U8Packed` of two u32s: the dot product of their bytes.
fn packed_dot(a: &Value, b: &Value, signed: bool) -> Option<Value> {
    let (&Value::Uint(x), &Value::Uint(y)) = (a, b) else {
        return None;
    };
    let sum: i32 = byte_values(x, signed)
        .iter()
        .zip(byte_values(y, signed))
        .map(|(p, q)| p * q)
        .sum();
    Some(if signed {
        Value::Sint(sum)
    } else {
        Value::Uint(sum.cast_unsigned())
    })
}

/// The bits of each u32 or i32 lane of `args` mapped by `f`, the lane's type kept.
fn bit_lanes(args: &[Value], f: fn(u32) -> u32) -> Option<Value> {
    lanes(args, &|lane| match lane[0] {
        Value::Uint(u) => Some(Value::Uint(f(u))),
        Value::Sint(i) => Some(Value::Sint(f(i.cast_unsigned()).cast_signed())),
        _ => None,
    })
}

/// The position of `b`'s most significant set bit, or all ones if none is set.
fn first_leading(b: u32) -> u32 {
    if b == 0 {
        u32::MAX
    } else {
        31 - b.leading_zeros()
    }
}

/// The larger (`max`) or smaller of two scalars of one type (a float's, by `f64::max` and `f64::min`).
fn min_max(a: &Value, b: &Value, max: bool) -> Option<Value> {
    match (a, b) {
        (&Value::Float(x, w), &Value::Float(y, _)) => {
            Some(float(if max { x.max(y) } else { x.min(y) }, w))
        }
        (&Value::Uint(x), &Value::Uint(y)) => {
            Some(Value::Uint(if max { x.max(y) } else { x.min(y) }))
        }
        (&Value::Sint(x), &Value::Sint(y)) => {
            Some(Value::Sint(if max { x.max(y) } else { x.min(y) }))
        }
        _ => None,
    }
}

/// The built-in `fun` of `args`, as WGSL evaluates it at run time, over every type WGSL allows it: each built-in
/// WGSL may call in a constant expression. `None` for naga's `outer` and `inverse`, which WGSL has not, and for a
/// `smoothstep` whose `low` and `high` are equal, which divides by zero.
fn math(fun: MathFunction, args: &[Value]) -> Option<Value> {
    use MathFunction as M;
    let float1 = |f: fn(f64) -> f64| float_lanes(args, &|x| f(x[0]));
    let arg = |i: usize| args.get(i);
    match fun {
        M::Abs => lanes(args, &|l| match l[0] {
            Value::Float(x, w) => Some(float(x.abs(), w)),
            Value::Uint(u) => Some(Value::Uint(u)),
            Value::Sint(i) => Some(Value::Sint(i.wrapping_abs())),
            _ => None,
        }),
        M::Min => lanes(args, &|l| min_max(&l[0], &l[1], false)),
        M::Max => lanes(args, &|l| min_max(&l[0], &l[1], true)),
        M::Clamp => lanes(args, &|l| {
            min_max(&min_max(&l[0], &l[1], true)?, &l[2], false)
        }),
        M::Saturate => float1(|x| x.clamp(0.0, 1.0)),
        M::Cos => float1(f64::cos),
        M::Cosh => float1(f64::cosh),
        M::Sin => float1(f64::sin),
        M::Sinh => float1(f64::sinh),
        M::Tan => float1(f64::tan),
        M::Tanh => float1(f64::tanh),
        M::Acos => float1(f64::acos),
        M::Asin => float1(f64::asin),
        M::Atan => float1(f64::atan),
        M::Atan2 => float_lanes(args, &|x| x[0].atan2(x[1])),
        M::Asinh => float1(f64::asinh),
        M::Acosh => float1(f64::acosh),
        M::Atanh => float1(f64::atanh),
        M::Radians => float1(f64::to_radians),
        M::Degrees => float1(f64::to_degrees),
        M::Ceil => float1(f64::ceil),
        M::Floor => float1(f64::floor),
        M::Round => float1(f64::round_ties_even),
        M::Fract => float1(|x| x - x.floor()),
        M::Trunc => float1(f64::trunc),
        M::Modf => Some(Value::List(vec![
            float1(|x| x - x.trunc())?,
            float1(f64::trunc)?,
        ])),
        M::Frexp => Some(Value::List(vec![
            lanes(args, &|l| match l[0] {
                Value::Float(x, w) => Some(float(frexp(x)?.0, w)),
                _ => None,
            })?,
            lanes(args, &|l| match l[0] {
                Value::Float(x, _) => Some(Value::Sint(frexp(x)?.1)),
                _ => None,
            })?,
        ])),
        M::Ldexp => lanes(args, &|l| match (&l[0], &l[1]) {
            (&Value::Float(x, w), &Value::Sint(e)) => Some(float(round(ldexp(x, e), w), w)),
            _ => None,
        }),
        M::Exp => float1(f64::exp),
        M::Exp2 => float1(f64::exp2),
        M::Log => float1(f64::ln),
        M::Log2 => float1(f64::log2),
        M::Pow => float_lanes(args, &|x| x[0].powf(x[1])),
        M::Dot => dot(arg(0)?, arg(1)?),
        M::Dot4I8Packed => packed_dot(arg(0)?, arg(1)?, true),
        M::Dot4U8Packed => packed_dot(arg(0)?, arg(1)?, false),
        M::Cross => cross(arg(0)?, arg(1)?),
        M::Distance => length(&binary(
            BinaryOperator::Subtract,
            arg(0)?.clone(),
            arg(1)?.clone(),
        )?),
        M::Length => length(arg(0)?),
        M::Normalize => binary(BinaryOperator::Divide, arg(0)?.clone(), length(arg(0)?)?),
        M::FaceForward => match dot(arg(1)?, arg(2)?)? {
            Value::Float(d, _) if d < 0.0 => Some(arg(0)?.clone()),
            _ => lanes(&[arg(0)?.clone()], &|l| unary(UnaryOperator::Negate, &l[0])),
        },
        M::Reflect => reflect(arg(0)?, arg(1)?),
        M::Refract => refract(arg(0)?, arg(1)?, arg(2)?),
        M::Sign => lanes(args, &|l| match l[0] {
            Value::Float(x, w) => Some(float(if x == 0.0 { x } else { x.signum() }, w)),
            Value::Sint(i) => Some(Value::Sint(i.signum())),
            _ => None,
        }),
        M::Fma => float_lanes(args, &|x| x[0].mul_add(x[1], x[2])),
        M::Mix => float_lanes(args, &|x| x[0] * (1.0 - x[2]) + x[1] * x[2]),
        M::Step => float_lanes(args, &|x| if x[0] <= x[1] { 1.0 } else { 0.0 }),
        M::SmoothStep => lanes(args, &|l| {
            let (x, w) = floats_of(l)?;
            if x[0] == x[1] {
                return None;
            }
            let t = ((x[2] - x[0]) / (x[1] - x[0])).clamp(0.0, 1.0);
            Some(float(round(t * t * (3.0 - 2.0 * t), w), w))
        }),
        M::Sqrt => float1(f64::sqrt),
        M::InverseSqrt => float1(|x| 1.0 / x.sqrt()),
        M::Transpose => {
            let columns = matrix(arg(0)?)?;
            let rows = (0..columns.first()?.len())
                .map(|i| Value::List(columns.iter().map(|c| c[i].clone()).collect()))
                .collect();
            Some(Value::List(rows))
        }
        M::Determinant => determinant(&matrix(arg(0)?)?),
        M::QuantizeToF16 => float1(round_f16),
        M::CountTrailingZeros => bit_lanes(args, u32::trailing_zeros),
        M::CountLeadingZeros => bit_lanes(args, u32::leading_zeros),
        M::CountOneBits => bit_lanes(args, u32::count_ones),
        M::ReverseBits => bit_lanes(args, u32::reverse_bits),
        M::FirstTrailingBit => {
            bit_lanes(args, |b| if b == 0 { u32::MAX } else { b.trailing_zeros() })
        }
        M::FirstLeadingBit => lanes(args, &|l| match l[0] {
            Value::Uint(u) => Some(Value::Uint(first_leading(u))),
            // An i32's highest bit that differs from its sign bit: the highest set bit of it, or of its complement.
            Value::Sint(i) => {
                let b = if i < 0 { !i } else { i };
                Some(Value::Sint(first_leading(b.cast_unsigned()).cast_signed()))
            }
            _ => None,
        }),
        M::ExtractBits => lanes(args, &|l| extract_field(&l[0], &l[1], &l[2])),
        M::InsertBits => lanes(args, &|l| insert_bits(&l[0], &l[1], &l[2], &l[3])),
        M::Pack4x8snorm => pack(arg(0)?, &|v| {
            Some(vec![(quantise(v, -1.0, 127.0)? as i8).cast_unsigned()])
        }),
        M::Pack4x8unorm => pack(arg(0)?, &|v| Some(vec![quantise(v, 0.0, 255.0)? as u8])),
        M::Pack2x16snorm => pack(arg(0)?, &|v| {
            Some((quantise(v, -1.0, 32767.0)? as i16).to_le_bytes().to_vec())
        }),
        M::Pack2x16unorm => pack(arg(0)?, &|v| {
            Some((quantise(v, 0.0, 65535.0)? as u16).to_le_bytes().to_vec())
        }),
        M::Pack2x16float => pack(arg(0)?, &|v| match *v {
            Value::Float(x, _) => Some(f16_bits(x).to_le_bytes().to_vec()),
            _ => None,
        }),
        M::Pack4xI8 | M::Pack4xU8 => pack(arg(0)?, &|v| Some(vec![bits(v)?.to_le_bytes()[0]])),
        M::Pack4xI8Clamp => pack(arg(0)?, &|v| match *v {
            Value::Sint(i) => Some(vec![(i.clamp(-128, 127) as i8).cast_unsigned()]),
            _ => None,
        }),
        M::Pack4xU8Clamp => pack(arg(0)?, &|v| match *v {
            Value::Uint(u) => Some(vec![u.min(255) as u8]),
            _ => None,
        }),
        M::Unpack4x8snorm => unpack(arg(0)?, 1, &|b| {
            float(
                round((f64::from(b[0].cast_signed()) / 127.0).max(-1.0), 4),
                4,
            )
        }),
        M::Unpack4x8unorm => unpack(arg(0)?, 1, &|b| float(round(f64::from(b[0]) / 255.0, 4), 4)),
        M::Unpack2x16snorm => unpack(arg(0)?, 2, &|b| {
            let s = i16::from_le_bytes([b[0], b[1]]);
            float(round((f64::from(s) / 32767.0).max(-1.0), 4), 4)
        }),
        M::Unpack2x16unorm => unpack(arg(0)?, 2, &|b| {
            float(
                round(f64::from(u16::from_le_bytes([b[0], b[1]])) / 65535.0, 4),
                4,
            )
        }),
        M::Unpack2x16float => unpack(arg(0)?, 2, &|b| {
            float(f16_value(u16::from_le_bytes([b[0], b[1]])), 4)
        }),
        M::Unpack4xI8 => unpack(arg(0)?, 1, &|b| Value::Sint(i32::from(b[0].cast_signed()))),
        M::Unpack4xU8 => unpack(arg(0)?, 1, &|b| Value::Uint(u32::from(b[0]))),
        M::Outer | M::Inverse => None,
    }
}

/// The value of `h`, if it is a constant expression: a literal (float, u32, i32 or bool); a module constant; a zero
/// value; a unary or binary operator on constants, a matrix product included; a splat; a vector, matrix or array
/// built of constants, one of its components, or a swizzle; a `select` or an `all` or `any` of them; any built-in
/// function WGSL allows in a constant expression ([`math`]); a conversion or a `bitcast` of a constant. Each is
/// evaluated as WGSL does at run time, float arithmetic in the operands' precision (f32 for f32). `None` otherwise.
/// A bool `&&` or `||` over a runtime value is no expression in naga's IR: naga lowers it, short-circuiting, to an
/// `if` that stores to a variable, which is not followed.
fn evaluate(module: &Module, arena: &Arena<Expression>, h: Handle<Expression>) -> Option<Value> {
    let value = |e| evaluate(module, arena, e);
    match arena[h] {
        Expression::Literal(literal) => match literal {
            Literal::F64(v) => Some(float(v, 8)),
            Literal::F32(v) => Some(float(f64::from(v), 4)),
            Literal::F16(v) => Some(float(v.to_f64(), 2)),
            Literal::U32(u) => Some(Value::Uint(u)),
            Literal::I32(i) => Some(Value::Sint(i)),
            Literal::Bool(b) => Some(Value::Bool(b)),
            _ => None,
        },
        Expression::Constant(c) => {
            evaluate(module, &module.global_expressions, module.constants[c].init)
        }
        Expression::ZeroValue(ty) => zero(module, ty),
        Expression::Unary { op, expr } => lanes(&[value(expr)?], &|l| unary(op, &l[0])),
        Expression::Splat { size, value: v } => Some(Value::List(vec![value(v)?; size as usize])),
        Expression::Compose { ty, ref components } => {
            let parts = components
                .iter()
                .map(|&c| value(c))
                .collect::<Option<Vec<_>>>()?;
            if !matches!(module.types[ty].inner, TypeInner::Vector { .. }) {
                return Some(Value::List(parts));
            }
            // A vector's components may be vectors (`vec4(v, x, y)`); the vector holds theirs.
            let flat = parts.into_iter().flat_map(|p| match p {
                Value::List(items) => items,
                scalar => vec![scalar],
            });
            Some(Value::List(flat.collect()))
        }
        Expression::AccessIndex { base, index } => value(base)?.index(usize::try_from(index).ok()?),
        Expression::Access { base, index } => {
            let i = match value(index)? {
                Value::Uint(i) => usize::try_from(i).ok()?,
                Value::Sint(i) => usize::try_from(i).ok()?,
                _ => return None,
            };
            value(base)?.index(i)
        }
        Expression::Swizzle {
            size,
            vector,
            pattern,
        } => {
            let v = value(vector)?;
            let picks = pattern[..size as usize]
                .iter()
                .map(|&c| v.index(c as usize))
                .collect::<Option<_>>()?;
            Some(Value::List(picks))
        }
        Expression::Binary { op, left, right } => binary(op, value(left)?, value(right)?),
        Expression::Select {
            condition,
            accept,
            reject,
        } => lanes(
            &[value(condition)?, value(accept)?, value(reject)?],
            &|l| match l[0] {
                Value::Bool(c) => Some(l[if c { 1 } else { 2 }].clone()),
                _ => None,
            },
        ),
        Expression::Relational {
            fun: fun @ (RelationalFunction::All | RelationalFunction::Any),
            argument,
        } => {
            let v = value(argument)?;
            let items = match &v {
                Value::List(items) => items.as_slice(),
                scalar => std::slice::from_ref(scalar),
            };
            let bools = items
                .iter()
                .map(|b| match *b {
                    Value::Bool(b) => Some(b),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Value::Bool(if fun == RelationalFunction::All {
                bools.iter().all(|&b| b)
            } else {
                bools.iter().any(|&b| b)
            }))
        }
        Expression::Math {
            fun,
            arg,
            arg1,
            arg2,
            arg3,
        } => {
            let args = [Some(arg), arg1, arg2, arg3]
                .into_iter()
                .flatten()
                .map(value)
                .collect::<Option<Vec<_>>>()?;
            math(fun, &args)
        }
        Expression::As {
            expr,
            kind,
            convert,
        } => lanes(&[value(expr)?], &|l| cast(&l[0], kind, convert)),
        _ => None,
    }
}

/// `block`'s statements in source order, each compound statement before the statements it holds, with their spans.
fn flatten<'a>(block: &'a Block, out: &mut Vec<(&'a Statement, Span)>) {
    for (statement, span) in block.span_iter() {
        out.push((statement, *span));
        match *statement {
            Statement::Block(ref b) => flatten(b, out),
            Statement::If {
                ref accept,
                ref reject,
                ..
            } => {
                flatten(accept, out);
                flatten(reject, out);
            }
            Statement::Switch { ref cases, .. } => {
                for case in cases {
                    flatten(&case.body, out);
                }
            }
            Statement::Loop {
                ref body,
                ref continuing,
                ..
            } => {
                flatten(body, out);
                flatten(continuing, out);
            }
            _ => {}
        }
    }
}

/// What a pointer expression points into: a local or global variable, or a function argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Root {
    Local(Handle<naga::LocalVariable>),
    Global(Handle<naga::GlobalVariable>),
    Argument(u32),
}

/// The variable a pointer `h` points into, following its accesses.
fn root(arena: &Arena<Expression>, h: Handle<Expression>) -> Option<Root> {
    match arena[h] {
        Expression::LocalVariable(v) => Some(Root::Local(v)),
        Expression::GlobalVariable(g) => Some(Root::Global(g)),
        Expression::FunctionArgument(i) => Some(Root::Argument(i)),
        Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => root(arena, base),
        _ => None,
    }
}

/// A function's statements in order: the step at which each expression is evaluated, and each store with its step
/// and the variable it writes (a call given a pointer may write through it, so counts as one).
struct Timeline {
    at: HashMap<Handle<Expression>, usize>,
    stores: Vec<(usize, Root)>,
}

impl Timeline {
    fn new(arena: &Arena<Expression>, statements: &[(&Statement, Span)]) -> Self {
        let mut at = HashMap::new();
        let mut stores = Vec::new();
        for (step, &(statement, _)) in statements.iter().enumerate() {
            match *statement {
                Statement::Emit(ref range) => {
                    for h in range.clone() {
                        at.insert(h, step);
                    }
                }
                Statement::Store { pointer, .. } | Statement::Atomic { pointer, .. } => {
                    stores.extend(root(arena, pointer).map(|r| (step, r)));
                }
                Statement::Call { ref arguments, .. } => {
                    for &a in arguments {
                        if !matches!(arena[a], Expression::FunctionArgument(_)) {
                            stores.extend(root(arena, a).map(|r| (step, r)));
                        }
                    }
                }
                _ => {}
            }
        }
        Timeline { at, stores }
    }

    /// Whether `r` is written between the steps that evaluate `a` and `b`; true if either step is unknown.
    fn stored_between(&self, r: Root, a: Handle<Expression>, b: Handle<Expression>) -> bool {
        let (Some(&ta), Some(&tb)) = (self.at.get(&a), self.at.get(&b)) else {
            return true;
        };
        let (lo, hi) = (ta.min(tb), ta.max(tb));
        // Each statement has a step of its own, so a store's step is never `lo` itself.
        self.stores
            .iter()
            .any(|&(t, s)| s == r && (lo..hi).contains(&t))
    }
}

/// Whether `a` and `b` are the same value: the same expression, or structurally equal expressions over the same
/// leaves, each pair of loads reading the same place with no store to its variable between them (R-352).
fn same(
    arena: &Arena<Expression>,
    timeline: &Timeline,
    a: Handle<Expression>,
    b: Handle<Expression>,
) -> bool {
    if a == b {
        return true;
    }
    let eq = |x, y| same(arena, timeline, x, y);
    let eq_opt = |x: Option<_>, y: Option<_>| match (x, y) {
        (Some(x), Some(y)) => eq(x, y),
        (None, None) => true,
        _ => false,
    };
    match (&arena[a], &arena[b]) {
        (Expression::Load { pointer: p }, Expression::Load { pointer: q }) => {
            eq(*p, *q) && root(arena, *p).is_none_or(|r| !timeline.stored_between(r, a, b))
        }
        (Expression::Access { base: x, index: i }, Expression::Access { base: y, index: j }) => {
            eq(*x, *y) && eq(*i, *j)
        }
        (
            Expression::AccessIndex { base: x, index: i },
            Expression::AccessIndex { base: y, index: j },
        ) => i == j && eq(*x, *y),
        (
            Expression::Swizzle {
                size: s1,
                vector: v1,
                pattern: p1,
            },
            Expression::Swizzle {
                size: s2,
                vector: v2,
                pattern: p2,
            },
        ) => s1 == s2 && p1 == p2 && eq(*v1, *v2),
        (
            Expression::Splat {
                size: s1,
                value: v1,
            },
            Expression::Splat {
                size: s2,
                value: v2,
            },
        ) => s1 == s2 && eq(*v1, *v2),
        (
            Expression::Compose {
                ty: t1,
                components: c1,
            },
            Expression::Compose {
                ty: t2,
                components: c2,
            },
        ) => t1 == t2 && c1.len() == c2.len() && c1.iter().zip(c2).all(|(x, y)| eq(*x, *y)),
        (Expression::Unary { op: o1, expr: e1 }, Expression::Unary { op: o2, expr: e2 }) => {
            o1 == o2 && eq(*e1, *e2)
        }
        (
            Expression::Binary {
                op: o1,
                left: l1,
                right: r1,
            },
            Expression::Binary {
                op: o2,
                left: l2,
                right: r2,
            },
        ) => o1 == o2 && eq(*l1, *l2) && eq(*r1, *r2),
        (
            Expression::Select {
                condition: c1,
                accept: a1,
                reject: r1,
            },
            Expression::Select {
                condition: c2,
                accept: a2,
                reject: r2,
            },
        ) => eq(*c1, *c2) && eq(*a1, *a2) && eq(*r1, *r2),
        (
            Expression::Math {
                fun: f1,
                arg: a1,
                arg1: b1,
                arg2: c1,
                arg3: d1,
            },
            Expression::Math {
                fun: f2,
                arg: a2,
                arg1: b2,
                arg2: c2,
                arg3: d2,
            },
        ) => f1 == f2 && eq(*a1, *a2) && eq_opt(*b1, *b2) && eq_opt(*c1, *c2) && eq_opt(*d1, *d2),
        (
            Expression::As {
                expr: e1,
                kind: k1,
                convert: c1,
            },
            Expression::As {
                expr: e2,
                kind: k2,
                convert: c2,
            },
        ) => k1 == k2 && c1 == c2 && eq(*e1, *e2),
        (
            x @ (Expression::Literal(_)
            | Expression::Constant(_)
            | Expression::ZeroValue(_)
            | Expression::FunctionArgument(_)
            | Expression::GlobalVariable(_)
            | Expression::LocalVariable(_)),
            y,
        ) => x == y,
        _ => false,
    }
}
