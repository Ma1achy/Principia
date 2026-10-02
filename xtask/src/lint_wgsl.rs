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
//! validator is built with every capability, `SHADER_FLOAT16` among them. naga does not fold a `bitcast`, so the
//! float rules evaluate an operand's constant expression themselves ([`constant_floats`]).

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

/// The float values of `h`, up to sign, if it is a constant expression of floats: a literal, a module constant, a
/// negation, a splat, a vector built of constants, a conversion, or a `bitcast<f32>` of a constant bit pattern (which
/// naga does not fold). `None` if it is not constant. naga concretises an abstract literal or constant before the IR,
/// so no abstract literal reaches here.
pub fn constant_floats(
    module: &Module,
    arena: &Arena<Expression>,
    h: Handle<Expression>,
) -> Option<Vec<f64>> {
    match arena[h] {
        Expression::Literal(Literal::F64(v)) => Some(vec![v]),
        Expression::Literal(Literal::F32(v)) => Some(vec![f64::from(v)]),
        Expression::Literal(Literal::F16(v)) => Some(vec![v.to_f64()]),
        Expression::Constant(c) => {
            constant_floats(module, &module.global_expressions, module.constants[c].init)
        }
        // Every rule reading these values is symmetric in sign (an inf, a NaN, ±65504, ±3.40282347e38), so a
        // negation passes its operand's values through.
        Expression::Unary {
            op: UnaryOperator::Negate,
            expr,
        } => constant_floats(module, arena, expr),
        Expression::Splat { value, .. } => constant_floats(module, arena, value),
        Expression::Compose { ref components, .. } => {
            let mut all = Vec::new();
            for &c in components {
                all.extend(constant_floats(module, arena, c)?);
            }
            Some(all)
        }
        Expression::As {
            expr,
            kind: ScalarKind::Float,
            convert: None,
        } => Some(
            constant_bits(module, arena, expr)?
                .into_iter()
                .map(|b| f64::from(f32::from_bits(b)))
                .collect(),
        ),
        Expression::As {
            expr,
            kind: ScalarKind::Float,
            convert: Some(_),
        } => constant_floats(module, arena, expr),
        _ => None,
    }
}

/// The 32-bit patterns of `h`, if it is a constant expression of 32-bit integers.
fn constant_bits(
    module: &Module,
    arena: &Arena<Expression>,
    h: Handle<Expression>,
) -> Option<Vec<u32>> {
    match arena[h] {
        Expression::Literal(Literal::U32(b)) => Some(vec![b]),
        Expression::Literal(Literal::I32(i)) => Some(vec![u32::from_ne_bytes(i.to_ne_bytes())]),
        Expression::Constant(c) => {
            constant_bits(module, &module.global_expressions, module.constants[c].init)
        }
        Expression::Splat { value, .. } => constant_bits(module, arena, value),
        Expression::Compose { ref components, .. } => {
            let mut all = Vec::new();
            for &c in components {
                all.extend(constant_bits(module, arena, c)?);
            }
            Some(all)
        }
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
