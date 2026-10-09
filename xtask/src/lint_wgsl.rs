//! `cargo xtask lint wgsl` — the WGSL traps of the fragment stage (render contract Part 5, "Unpack layer"; payload
//! §6; REQ-RENDER-001, REQ-RENDER-083), checked over naga's IR. It parses and validates every WGSL file under
//! `crates/render/frag/` ([`FRAG_DIR`]), generated or written by hand (R-351), and every WGSL file of the shared
//! library under `crates/render/shaders/` ([`LIB_DIR`]): the generated prelude ([`PRELUDE`]) alone, and every other
//! file there, which follows the prelude at assembly (render_gui_spec §10.1), as its continuation
//! ([`check_fragment_after`]). The library's files are fragment-stage WGSL too, so the float rules hold there as well
//! (applied per R-369, TASK-M1-03). A built-in occupant, a file under [`OCCUPANT_DIR`], is linted as the assembler
//! presents it: after the prelude, the other files of [`LIB_FILES`] and its `// @uniform` block declared as the
//! struct `uniforms` ([`occupant_context`]; applied per R-369, TASK-M7-04). The stain's context, a file under
//! [`STAIN_DIR`], is linted after the prelude, the other files of [`LIB_FILES`], the unpack layer and the read side,
//! whose `SimState` it holds; a debug view, a file under [`DEBUG_VIEWS`], [`REDUCTION_VIEWS`] or [`DEBUG_OCCUPANTS`],
//! generated or written by hand, after all of those and the stain's context, whose `Ctx` it reads, and its
//! `// @uniform` block, as the assembler presents it ([`view_context`], [`uniform_block`]; applied per R-369,
//! TASK-M1-08, TASK-M1-09, TASK-M1-12).
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
//! layer's own rules, and so is the generated read side, `read_side.wgsl` beside it, linted as that file's
//! continuation, since it follows it at assembly ([`check_read_side`]):
//! - an `extractBits` whose argument is not u32: the i32 overload sign-extends ([`Rule::ExtractBitsU32`]);
//! - any f64 type: WGSL has none ([`Rule::NoF64`]);
//! - an `enable f16` directive, or any f16 type: f16 pairs are read through core `unpack2x16float`, which needs no
//!   `shader-f16` ([`Rule::NoEnableF16`]);
//! - a `SimState*` struct whose `r`, `p`, `r_sh` or `p_sh` is not `array<vec2<f32>, 3>`, or that lacks `r` or `p`
//!   ([`Rule::Vec2Groups`]);
//! - a word buffer that is not its own binding: `word_buffer` must be a storage global `array<vec4<u32>>` with a
//!   binding no other global shares, read only at a per-sample index (a function argument), as the `SimState` buffer
//!   is; and no stored `SimState*` struct may hold a `vec4<u32>`, the read-side `SimState` (lowering Part 3a) holding
//!   the word it read ([`Rule::WordBinding`]);
//! - a buffer off R-343's bindings, the numbers of the ledger's one table ([`ledger::payload::bindings`]):
//!   `simstate_buffer: array<SimStateFTLE>` at `@group(1) @binding(0)` and `word_buffer: array<vec4<u32>>` at
//!   `@group(1) @binding(1)`, each attribute equal to the generated `<PREFIX>_GROUP` and `<PREFIX>_BINDING` constants,
//!   and no binding in group 0, the assembler's per-frame uniforms ([`Rule::Bindings`]);
//! - a use of either buffer in any function but its one reader, the read side's generated `sample_read`
//!   ([`Rule::SampleOnly`], R-343 as R-378 amends it);
//! - a load of a whole stored struct, `simstate_buffer[i]` loaded as one value rather than one member at a time
//!   (`simstate_buffer[i].packed_a`), or the two buffers read at different arguments of one function, not the same
//!   sample index ([`Rule::PerMember`], R-378). A word may be loaded whole, by a field that needs all four of its
//!   components, or one component at a time; which components a field needs is the read side's own
//!   (`ledger/tests/per_member_loads.rs` checks them on the compiled output).
//!
//! naga folds a call whose arguments are all constant before the IR is built, so the `extractBits` rule sees only
//! calls on a runtime value, which every generated accessor's is (a parameter). The `enable` directive is not kept in
//! the IR, so that rule reads the source's directives, comments stripped. A hand-written file is held to the float
//! rules only: R-317's ban on `enable f16` covers the generated WGSL, so a hand-written file may use f16, and the
//! validator is built with every capability, `SHADER_FLOAT16` among them. naga does not fold a `bitcast`, nor
//! any constant expression over one (arithmetic, a math call, a component), so the float rules evaluate an operand's
//! constant expression themselves, in f32 for f32 ([`constant_floats`]).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use naga::{
    AddressSpace, Arena, ArraySize, BinaryOperator, Block, Expression, Function, Handle, Literal,
    MathFunction, Module, RelationalFunction, Scalar, ScalarKind, Span, Statement, TypeInner,
    UnaryOperator, VectorSize,
};

/// The generated file the lint checks, relative to the workspace root.
pub const GENERATED: &str = "crates/render/frag/generated/payload_unpack.wgsl";

/// The generated read side (lowering Part 3a), relative to the workspace root. It follows [`GENERATED`] at assembly
/// and reads its accessors and stored layouts, so it is linted as that file's continuation ([`check_read_side`]).
pub const READ_SIDE_FILE: &str = "crates/render/frag/generated/read_side.wgsl";

/// The fragment stage's WGSL: every `.wgsl` file under this directory, relative to the workspace root (R-351).
pub const FRAG_DIR: &str = "crates/render/frag";

/// The fragment stage's shared library (render_gui_spec §10.1; gui_state_contract §3's `shaders/wgsl/lib/`): every
/// `.wgsl` file under this directory, relative to the workspace root, held to the float rules.
pub const LIB_DIR: &str = "crates/render/shaders";

/// The generated prelude, relative to the workspace root: linted alone, and every other file under [`LIB_DIR`] as its
/// continuation, since each follows it at assembly.
pub const PRELUDE: &str = "crates/render/shaders/wgsl/lib/prelude.wgsl";

/// The built-in occupants (gui_state_contract §3's `shaders/wgsl/frag/<slot>/`): every `.wgsl` file under this
/// directory, relative to the workspace root, is linted with [`occupant_context`] before it.
pub const OCCUPANT_DIR: &str = "crates/render/shaders/wgsl/frag";

/// The library files the assembler places after the prelude and before every occupant (render_gui_spec §10.1):
/// every `.wgsl` file in this directory but [`PRELUDE`], relative to the workspace root.
pub const LIB_FILES: &str = "crates/render/shaders/wgsl/lib";

/// The stain's context (render contract Part 1), relative to the workspace root: every `.wgsl` file in this
/// directory follows the read side at assembly and precedes the node functions.
pub const STAIN_DIR: &str = "crates/render/shaders/wgsl/stain";

/// The debug catalogue's generated views (`ledger::gen::catalogue::DIR`, RQ-219), relative to the workspace root:
/// each a colour occupant reading the stain's context.
pub const DEBUG_VIEWS: &str = "crates/render/frag/debug/generated";

/// The debug catalogue's generated reductions (`ledger::gen::catalogue::REDUCTIONS_DIR`): each vector field's direction
/// cosines and the ternary masses, colour occupants that read `ctx` as the generated views do (TASK-M1-12).
pub const REDUCTION_VIEWS: &str = "crates/render/frag/debug/reductions";

/// The hand-written debug views, under the occupant directory's `debug/` (TASK-M1-12): colour occupants that read
/// `ctx`, linted as the generated views are.
pub const DEBUG_OCCUPANTS: &str = "crates/render/shaders/wgsl/frag/debug";

/// Whether the file at `rel` is a debug view, read in the stain's context ([`view_context`]).
fn is_view(rel: &str) -> bool {
    [DEBUG_VIEWS, REDUCTION_VIEWS, DEBUG_OCCUPANTS]
        .iter()
        .any(|dir| rel.starts_with(&format!("{dir}/")))
}

/// The fix every float-rule finding names (R-343).
pub const BIT_PATTERN_FIX: &str =
    "fix: test the bit pattern instead (R-343), as `pa_d_min_is_unset` does: \
     `extractBits(w, 16u, 16u) == PA_D_MIN_UNSET`";

/// The word buffer's global, and the `SimState` buffer's.
pub const WORD_BUFFER: &str = "word_buffer";
pub const SIMSTATE_BUFFER: &str = "simstate_buffer";

/// The struct-name prefix of the stored `SimState` layouts.
const SIMSTATE: &str = "SimState";
/// The read-side `SimState` (lowering Part 3a): not stored, so the word may be one of its members.
const READ_SIDE: &str = "SimState";

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
    PerMember,
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
            Rule::PerMember => "per-member",
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

/// Every `.wgsl` file under `root`'s [`FRAG_DIR`], linted: [`GENERATED`], which must exist, by [`check`], the read
/// side ([`READ_SIDE_FILE`]) as its continuation by [`check_read_side`], and every other file, written by hand, by
/// [`check_fragment`]; then every `.wgsl` file under [`LIB_DIR`], if it exists: [`PRELUDE`], which must exist then,
/// by [`check_fragment`], and every other file as its continuation by [`check_fragment_after`]. A file that does not
/// parse or validate is an error naming it.
pub fn lint(root: &Path) -> Result<Vec<FileReport>, String> {
    let mut reports = lint_frag(root)?;
    reports.extend(lint_lib(root)?);
    Ok(reports)
}

/// `path` under `root`, read.
fn read(root: &Path, path: &str) -> Result<String, String> {
    let path = root.join(path);
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// `parts` joined, each ending its line.
fn joined(parts: &[&str]) -> String {
    let mut out = String::new();
    for part in parts {
        out.push_str(part);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// What precedes the stain's context at assembly (render contract Part 2; TASK-M1-04): the prelude, the other files of
/// [`LIB_FILES`] in sorted order, the unpack layer ([`GENERATED`]) and the read side ([`READ_SIDE_FILE`]).
fn read_side_context(root: &Path) -> Result<String, String> {
    let mut parts = vec![read(root, PRELUDE)?];
    let mut library = Vec::new();
    wgsl_files(&root.join(LIB_FILES), &mut library)?;
    library.sort();
    for path in library {
        if relative(root, &path) != PRELUDE {
            parts.push(
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
    }
    parts.push(read(root, GENERATED)?);
    parts.push(read(root, READ_SIDE_FILE)?);
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    Ok(joined(&parts))
}

/// What precedes a generated debug view at assembly: the prelude, the library, the unpack layer and the read side,
/// then the stain's context, the files of [`STAIN_DIR`] in sorted order (render contract Part 2; gui_state_contract §3).
pub fn view_context(root: &Path) -> Result<String, String> {
    let mut parts = vec![read_side_context(root)?];
    let mut stain = Vec::new();
    let dir = root.join(STAIN_DIR);
    if dir.is_dir() {
        wgsl_files(&dir, &mut stain)?;
    }
    stain.sort();
    for path in stain {
        parts.push(std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    Ok(joined(&parts))
}

/// `path` relative to `root`, `/`-separated.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// [`lint`]'s reports for [`LIB_DIR`]: none when it does not exist.
fn lint_lib(root: &Path) -> Result<Vec<FileReport>, String> {
    let dir = root.join(LIB_DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let prelude_path = root.join(PRELUDE);
    let prelude = std::fs::read_to_string(&prelude_path)
        .map_err(|e| format!("{}: {e}", prelude_path.display()))?;
    let mut files = Vec::new();
    wgsl_files(&dir, &mut files)?;
    files.sort();
    // The library's files after the prelude, in the order a sorted listing gives them, as the assembler places
    // `colour_space.wgsl` then `present.wgsl`.
    let mut library = Vec::new();
    for path in &files {
        let rel = relative(root, path);
        if rel != PRELUDE && path.parent() == Some(root.join(LIB_FILES).as_path()) {
            library.push(
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
    }
    let mut reports = Vec::new();
    for path in files {
        let rel = relative(root, &path);
        let source =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let findings = if rel == PRELUDE {
            check_fragment(&source)
        } else if rel.starts_with(&format!("{STAIN_DIR}/")) {
            read_side_context(root).and_then(|context| check_fragment_after(&context, &source))
        } else if is_view(&rel) {
            view_context(root).and_then(|context| {
                check_fragment_after(&format!("{context}{}", uniform_block(&source)?), &source)
            })
        } else if rel.starts_with(&format!("{OCCUPANT_DIR}/")) {
            occupant_context(&prelude, &library, &source)
                .and_then(|context| check_fragment_after(&context, &source))
        } else {
            check_fragment_after(&prelude, &source)
        }
        .map_err(|e| format!("{rel}: {e}"))?;
        reports.push(FileReport {
            file: rel,
            findings,
        });
    }
    Ok(reports)
}

/// What precedes a built-in occupant's `source` when it is linted, as the assembler presents a node (render contract
/// Part 2; gui_state_contract §3): the prelude, the `library` files after it, then the occupant's `// @uniform` block,
/// each line `// @uniform <name>: <type> = <default> …`, declared as a struct in the prelude's uniform group, one
/// binding past the prelude's, and bound as `uniforms`, which the occupant reads as `uniforms.<name>`. With no
/// `// @uniform` line there is no block. The assembler also renames the occupant's functions and `uniforms` per node;
/// that renaming changes no float expression, so the lint reads the text as written. A `// @uniform` line without a
/// name before `:` and a type between `:` and `=` is an error.
pub fn occupant_context(prelude: &str, library: &[String], source: &str) -> Result<String, String> {
    let mut out = String::from(prelude);
    for file in library {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(file);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&uniform_block(source)?);
    Ok(out)
}

/// `source`'s `// @uniform` block as the assembler declares it: a struct of its uniforms in the prelude's uniform
/// group, one binding past the prelude's, bound as `uniforms`; empty with no `// @uniform` line. A line without a name
/// before `:` and a type between `:` and `=` is an error.
pub fn uniform_block(source: &str) -> Result<String, String> {
    let mut members = Vec::new();
    for (k, line) in source.lines().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("// @uniform") else {
            continue;
        };
        let parsed = rest
            .split_once(':')
            .and_then(|(name, rest)| rest.split_once('=').map(|(ty, _)| (name.trim(), ty.trim())))
            .filter(|(name, ty)| !name.is_empty() && !ty.is_empty());
        let Some((name, ty)) = parsed else {
            return Err(format!(
                "line {}: `{}` is not `// @uniform <name>: <type> = <default>`",
                k + 1,
                line.trim()
            ));
        };
        members.push(format!("    {name}: {ty},\n"));
    }
    if members.is_empty() {
        return Ok(String::new());
    }
    let first = ledger::gen::prelude::uniforms_binding();
    let mut out = String::from("struct OccupantUniforms {\n");
    out.extend(members);
    out.push_str(&format!(
        "}}\n@group({}) @binding({}) var<uniform> uniforms: OccupantUniforms;\n",
        first.group,
        first.binding + 1
    ));
    Ok(out)
}

/// [`lint`]'s reports for [`FRAG_DIR`].
fn lint_frag(root: &Path) -> Result<Vec<FileReport>, String> {
    let generated = root.join(GENERATED);
    if !generated.is_file() {
        return Err(format!("{}: no such file", generated.display()));
    }
    let layer =
        std::fs::read_to_string(&generated).map_err(|e| format!("{}: {e}", generated.display()))?;
    let mut files = Vec::new();
    wgsl_files(&root.join(FRAG_DIR), &mut files)?;
    files.sort();
    let mut reports = Vec::new();
    for path in files {
        let rel = relative(root, &path);
        let source =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let findings = if rel == GENERATED {
            check(&source)
        } else if rel == READ_SIDE_FILE {
            check_read_side(&layer, &source)
        } else if is_view(&rel) {
            view_context(root).and_then(|context| {
                check_fragment_after(&format!("{context}{}", uniform_block(&source)?), &source)
            })
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

/// The findings in the generated read side's `source`, linted as the continuation of the generated layer `layer` by
/// every rule [`check`] applies, or why it could not be checked: the two together do not parse or validate. A finding
/// on a line of `source` is reported at that line of it. One on a line of `layer`, or on none, is reported here only if
/// `layer` alone does not give it, since the layer's own report has it.
pub fn check_read_side(layer: &str, source: &str) -> Result<Vec<Finding>, String> {
    let own = check(layer)?;
    let lines = layer.lines().count();
    let joined = if layer.ends_with('\n') {
        format!("{layer}{source}")
    } else {
        format!("{layer}\n{source}")
    };
    let base = u32::try_from(lines).map_err(|e| e.to_string())?;
    Ok(check(&joined)?
        .into_iter()
        .filter_map(|mut f| match f.line {
            Some(line) if line > base => {
                f.line = Some(line - base);
                Some(f)
            }
            _ if own.contains(&f) => None,
            _ => Some(f),
        })
        .collect())
}

/// The float rules' findings in `source`, a fragment-stage WGSL file that follows `prefix` at assembly, linted as
/// `prefix`'s continuation; or why it could not be checked: the two together do not parse or validate. A finding on
/// a line of `source` is reported at that line of it; one on a line of `prefix` is `prefix`'s own, reported on it, and
/// left out here. One on no line (naga gives a synthesised expression no span) is kept, so it is never lost.
pub fn check_fragment_after(prefix: &str, source: &str) -> Result<Vec<Finding>, String> {
    let base = u32::try_from(prefix.lines().count()).map_err(|e| e.to_string())?;
    let joined = if prefix.ends_with('\n') {
        format!("{prefix}{source}")
    } else {
        format!("{prefix}\n{source}")
    };
    Ok(check_fragment(&joined)?
        .into_iter()
        .filter_map(|mut f| match f.line {
            Some(line) if line > base => {
                f.line = Some(line - base);
                Some(f)
            }
            Some(_) => None,
            None => Some(f),
        })
        .collect())
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
    found.extend(per_member(&module));
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
/// the `SimState` buffer exists with another binding, every read of either is at a function argument, and no stored
/// `SimState*` struct holds a `vec4<u32>`: the read-side `SimState`, which is never stored, holds the word it read.
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
    // The stored layouts only: the read-side `SimState` holds the word read from its own buffer, `sample.word`
    // (lowering Part 3a), and is never stored.
    for (_, ty) in module.types.iter() {
        let (Some(name), TypeInner::Struct { members, .. }) = (ty.name.as_deref(), &ty.inner)
        else {
            continue;
        };
        if !name.starts_with(SIMSTATE) || name == READ_SIDE {
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
    for b in ledger::payload::unpack_bindings() {
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

/// Each function or entry point but a buffer's one reader that uses the buffer (R-343, R-378: both buffers are read
/// only by the read side's generated `sample_read(i)`).
fn sample_only(module: &Module) -> Vec<Finding> {
    let table = ledger::payload::unpack_bindings();
    let mut found = Vec::new();
    for function in functions(module) {
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
                            "`{fname}` uses `{}`, which only `{}` reads (R-343, R-378)",
                            b.buffer, b.reader
                        ),
                    ));
                }
            }
        }
    }
    found
}

/// Every function and entry point of `module`.
fn functions(module: &Module) -> impl Iterator<Item = &Function> {
    module
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(module.entry_points.iter().map(|ep| &ep.function))
}

/// The buffer global `e` in `function` is, by name, if it is `simstate_buffer` or `word_buffer`.
fn buffer<'m>(module: &'m Module, function: &Function, e: Handle<Expression>) -> Option<&'m str> {
    let Expression::GlobalVariable(g) = function.expressions[e] else {
        return None;
    };
    module.global_variables[g]
        .name
        .as_deref()
        .filter(|n| *n == SIMSTATE_BUFFER || *n == WORD_BUFFER)
}

/// R-378's per-member reads: no load of a whole element of `simstate_buffer`, a stored struct, where the read side
/// loads one member at a time; and in each function, both buffers indexed by the same argument, the sample index.
fn per_member(module: &Module) -> Vec<Finding> {
    let mut found = Vec::new();
    for function in functions(module) {
        let fname = function.name.as_deref().unwrap_or("(unnamed)");
        let mut indices: Vec<u32> = Vec::new();
        for (_, expr) in function.expressions.iter() {
            match *expr {
                Expression::Load { pointer } => {
                    let base = match function.expressions[pointer] {
                        Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => {
                            base
                        }
                        _ => continue,
                    };
                    if buffer(module, function, base) == Some(SIMSTATE_BUFFER) {
                        found.push(finding(
                            Rule::PerMember,
                            format!(
                                "`{fname}` loads a whole stored struct from `{SIMSTATE_BUFFER}`: the read side loads \
                                 only the stored members each field needs, `{SIMSTATE_BUFFER}[i].<member>` (R-378)"
                            ),
                        ));
                    }
                }
                Expression::Access { base, index } if buffer(module, function, base).is_some() => {
                    if let Expression::FunctionArgument(k) = function.expressions[index] {
                        if !indices.contains(&k) {
                            indices.push(k);
                        }
                    }
                }
                _ => {}
            }
        }
        if indices.len() > 1 {
            found.push(finding(
                Rule::PerMember,
                format!(
                    "`{fname}` reads the buffers at different arguments: both are read at the same sample index \
                     (R-343, R-378)"
                ),
            ));
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
    let summaries = summaries(module);
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
        let timeline = Timeline::new(module, &summaries, &function.expressions, &function.body);
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
                        comparison(module, function, &timeline, h, op, left, right)
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

/// The float rules a float comparison `left op right`, the expression `at`, breaks, each with what breaks it.
fn comparison(
    module: &Module,
    function: &Function,
    timeline: &Timeline,
    at: Handle<Expression>,
    op: BinaryOperator,
    left: Handle<Expression>,
    right: Handle<Expression>,
) -> Vec<(Rule, String)> {
    let mut found = Vec::new();
    if same(&function.expressions, timeline, at, left, right) {
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
/// constant, holds an integer or a bool, or holds a value `evaluate` cannot give soundly. naga folds a constant
/// expression of literals before the IR is built, and rejects one at parse where WGSL makes it an error (the `sqrt` of
/// a negative literal, an overflowing `exp`), which the lint reports. It leaves unfolded any constant expression over a
/// `bitcast`, and some built-ins (`ldexp`, `mix`, `smoothstep`, `modf`, `frexp`, the packing built-ins) even over
/// literals, so `evaluate` evaluates what naga leaves, as the shader does at run time. naga concretises an abstract
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

/// A constant expression's value: a float of `width` bytes (f16 2, f32 4, f64 8), a NaN whose bit pattern is known, a
/// u32, an i32, a 16- or 64-bit integer, a bool, or a vector, matrix (a list of its columns), array or built-in result
/// struct (`modf`'s, `frexp`'s) of values.
#[derive(Clone, Debug)]
enum Value {
    /// A float and its width. A NaN here is one whose bits are not known: one an operator, a conversion or a built-in
    /// computed, whose bits WGSL leaves indeterminate.
    Float(f64, u8),
    /// A NaN made by a `bitcast`, and so known to the bit: its bit pattern (zero-extended) and width. It keeps them
    /// through a `bitcast`, a `let`, a constant, a component, a swizzle, a `select` and a `transpose`, which move a
    /// value without computing it; an operator, a conversion or any other built-in sees it as a NaN ([`Value::plain`]).
    NanBits(u64, u8),
    Uint(u32),
    Sint(i32),
    /// A u16, i16, u64 or i64: its bit pattern (zero-extended), its width (2 or 8) and whether it is signed. A u32 or
    /// an i32 is always a `Uint` or a `Sint` ([`integer`]).
    Int(u64, u8, bool),
    Bool(bool),
    List(Vec<Value>),
}

impl Value {
    /// Appends the value's floats to `out`, in order; `None` if it holds an integer or a bool.
    fn floats(&self, out: &mut Vec<f64>) -> Option<()> {
        match self {
            Value::Float(v, _) => out.push(*v),
            Value::NanBits(..) => out.push(f64::NAN),
            Value::List(items) => {
                for item in items {
                    item.floats(out)?;
                }
            }
            _ => return None,
        }
        Some(())
    }

    /// The value with each NaN of known bits made a NaN of unknown bits: what an operator, a conversion or a built-in
    /// that computes sees, as WGSL leaves the bits of a NaN it computes indeterminate.
    fn plain(self) -> Value {
        match self {
            Value::NanBits(_, w) => Value::Float(f64::NAN, w),
            Value::List(items) => Value::List(items.into_iter().map(Value::plain).collect()),
            v => v,
        }
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

/// The unary `op` on the scalar `v`, as WGSL evaluates it at run time (a signed integer's negation wraps).
fn unary(op: UnaryOperator, v: &Value) -> Option<Value> {
    match (op, v) {
        (UnaryOperator::Negate, &Value::Float(x, w)) => Some(Value::Float(-x, w)),
        (UnaryOperator::LogicalNot, &Value::Bool(b)) => Some(Value::Bool(!b)),
        _ => {
            // naga allows `-` on a signed integer only.
            let (b, w, signed) = int(v)?;
            match op {
                UnaryOperator::Negate => Some(integer(b.wrapping_neg(), w, signed)),
                UnaryOperator::BitwiseNot => Some(integer(!b, w, signed)),
                UnaryOperator::LogicalNot => None,
            }
        }
    }
}

/// An integer scalar's bit pattern (zero-extended), width in bytes and signedness: a u32, an i32, or a 16- or 64-bit
/// integer. `None` for anything else.
fn int(v: &Value) -> Option<(u64, u8, bool)> {
    match *v {
        Value::Uint(u) => Some((u64::from(u), 4, false)),
        Value::Sint(i) => Some((u64::from(i.cast_unsigned()), 4, true)),
        Value::Int(b, w, signed) => Some((b, w, signed)),
        _ => None,
    }
}

/// The integer of width `w` bytes, signed or not, whose bit pattern is the low `w` bytes of `b`: a `Uint` or a `Sint`
/// for a 32-bit one, else an `Int`.
fn integer(b: u64, w: u8, signed: bool) -> Value {
    let b = b & mask(w);
    match (w, signed) {
        (4, false) => Value::Uint(b as u32),
        (4, true) => Value::Sint((b as u32).cast_signed()),
        _ => Value::Int(b, w, signed),
    }
}

/// All ones in the low `w` bytes.
fn mask(w: u8) -> u64 {
    u64::MAX >> (64 - 8 * u32::from(w))
}

/// The low `w` bytes of `b`, sign-extended.
fn extend(b: u64, w: u8) -> i64 {
    let shift = 64 - 8 * u32::from(w);
    (b << shift).cast_signed() >> shift
}

/// `a op b` on integer scalars, each as [`int`] gives it, as WGSL evaluates it at run time: wrapping at the operands'
/// width, a division by zero giving `a` and a remainder by zero 0 (as do the signed `MIN / -1` and `MIN % -1`), and a
/// shift (by an unsigned amount) by the amount modulo the width in bits; comparisons giving a bool. `None` if the
/// operand types differ.
fn int_binary(op: BinaryOperator, a: (u64, u8, bool), b: (u64, u8, bool)) -> Option<Value> {
    use BinaryOperator as B;
    let ((x, w, signed), (y, yw, y_signed)) = (a, b);
    if matches!(op, B::ShiftLeft | B::ShiftRight) {
        if y_signed {
            return None;
        }
        let k = (y % (8 * u64::from(w))) as u32;
        let r = match op {
            B::ShiftLeft => x << k,
            _ if signed => (extend(x, w) >> k).cast_unsigned(),
            _ => x >> k,
        };
        return Some(integer(r, w, signed));
    }
    if (yw, y_signed) != (w, signed) {
        return None;
    }
    let (sx, sy) = (extend(x, w), extend(y, w));
    let r = match op {
        B::Add => x.wrapping_add(y),
        B::Subtract => x.wrapping_sub(y),
        B::Multiply => x.wrapping_mul(y),
        B::Divide if signed => sx.checked_div(sy).map_or(x, i64::cast_unsigned),
        B::Divide => x.checked_div(y).unwrap_or(x),
        B::Modulo if signed => sx.checked_rem(sy).map_or(0, i64::cast_unsigned),
        B::Modulo => x.checked_rem(y).unwrap_or(0),
        B::And => x & y,
        B::ExclusiveOr => x ^ y,
        B::InclusiveOr => x | y,
        _ if signed => return compare(op, sx, sy),
        _ => return compare(op, x, y),
    };
    Some(integer(r, w, signed))
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
/// arithmetic as [`int_binary`] gives it; comparisons giving a bool.
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
        (&Value::Bool(x), &Value::Bool(y)) => match op {
            B::And => Some(Value::Bool(x & y)),
            B::InclusiveOr => Some(Value::Bool(x | y)),
            _ => compare(op, x, y),
        },
        _ => int_binary(op, int(a)?, int(b)?),
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

/// The scalar `v`'s bit pattern (zero-extended) and width in bytes, if they are known: an integer's; a float's other
/// than a NaN, exactly (an f16's and an f32's value is exact in the f64 that holds it); or a NaN's made by a `bitcast`
/// ([`Value::NanBits`]). `None` for a NaN of unknown bits, which WGSL leaves indeterminate.
fn bits(v: &Value) -> Option<(u64, u8)> {
    match *v {
        Value::Float(x, w) if !x.is_nan() => Some(match w {
            2 => (u64::from(f16_bits(x)), 2),
            4 => (u64::from((x as f32).to_bits()), 4),
            _ => (x.to_bits(), w),
        }),
        Value::NanBits(b, w) => Some((b, w)),
        _ => int(v).map(|(b, w, _)| (b, w)),
    }
}

/// The float of width `w` bytes whose bit pattern is `b`: a [`Value::NanBits`] if it is a NaN, so its bits stay known.
fn float_from_bits(b: u64, w: u8) -> Option<Value> {
    let x = match w {
        2 => f16_value(b as u16),
        4 => f64::from(f32::from_bits(b as u32)),
        8 => f64::from_bits(b),
        _ => return None,
    };
    Some(if x.is_nan() {
        Value::NanBits(b, w)
    } else {
        float(x, w)
    })
}

/// The scalar `v` converted (`convert: Some(width)`, WGSL's `f32(e)`, `u32(e)`, …) or bitcast (`convert: None`) to
/// `kind`, as WGSL converts at run time: a float to an integer truncated and saturated, an integer to a float rounded
/// to its width, an integer to an integer of another width sign- or zero-extended (by the source's signedness) and
/// truncated; a `bitcast` keeping the bit pattern, which naga allows only between types of one width, a NaN's
/// included where it is known. `None` for a NaN to an integer, and for a `bitcast` of a NaN of unknown bits to an
/// integer, which WGSL leaves indeterminate.
fn cast(v: &Value, kind: ScalarKind, convert: Option<u8>) -> Option<Value> {
    use ScalarKind as K;
    let Some(w) = convert else {
        // A float to its own type is itself, whether its bits are known or not.
        if let (K::Float, &Value::Float(x, width)) = (kind, v) {
            return Some(float(x, width));
        }
        let (b, width) = bits(v)?;
        return match kind {
            K::Float => float_from_bits(b, width),
            K::Uint | K::Sint => Some(integer(b, width, kind == K::Sint)),
            _ => None,
        };
    };
    let signed = kind == K::Sint;
    match (kind, v) {
        (K::Float, &Value::Float(x, _)) => Some(float(round(x, w), w)),
        (K::Float, &Value::Bool(b)) => Some(float(f64::from(u8::from(b)), w)),
        (K::Float, _) => {
            let (b, from, s) = int(v)?;
            Some(float(int_to_float(b, from, s, w), w))
        }
        (K::Uint | K::Sint, &Value::Float(x, _)) => float_to_int(x, w, signed),
        (K::Uint | K::Sint, &Value::Bool(b)) => Some(integer(u64::from(b), w, signed)),
        (K::Uint | K::Sint, _) => {
            let (b, from, s) = int(v)?;
            let wide = if s {
                extend(b, from).cast_unsigned()
            } else {
                b
            };
            Some(integer(wide, w, signed))
        }
        (K::Bool, &Value::Float(x, _)) => Some(Value::Bool(x != 0.0)),
        (K::Bool, &Value::Bool(b)) => Some(Value::Bool(b)),
        (K::Bool, _) => Some(Value::Bool(int(v)?.0 != 0)),
        _ => None,
    }
}

/// The integer of `from` bytes with bit pattern `b`, signed or not, converted to a float of width `w`: rounded once to
/// nearest, ties to even (an f16 by way of an f64, which holds every integer that does not overflow f16 exactly).
fn int_to_float(b: u64, from: u8, signed: bool, w: u8) -> f64 {
    let s = extend(b, from);
    match w {
        4 if signed => f64::from(s as f32),
        4 => f64::from(b as f32),
        _ if signed => round(s as f64, w),
        _ => round(b as f64, w),
    }
}

/// The float `x` converted to an integer of `w` bytes, signed or not: truncated, and saturated at the type's bounds.
/// `None` for a NaN, which WGSL leaves indeterminate.
fn float_to_int(x: f64, w: u8, signed: bool) -> Option<Value> {
    if x.is_nan() {
        return None;
    }
    let b = match (w, signed) {
        (2, false) => u64::from(x as u16),
        (2, true) => i64::from(x as i16).cast_unsigned(),
        (4, false) => u64::from(x as u32),
        (4, true) => i64::from(x as i32).cast_unsigned(),
        (8, false) => x as u64,
        (8, true) => (x as i64).cast_unsigned(),
        _ => return None,
    };
    Some(integer(b, w, signed))
}

/// The zero value of type `ty`: a scalar, vector, matrix or fixed-size array (WGSL's `T()`).
fn zero(module: &Module, ty: Handle<naga::Type>) -> Option<Value> {
    let scalar = |s: Scalar| match s.kind {
        ScalarKind::Float => Some(float(0.0, s.width)),
        ScalarKind::Uint | ScalarKind::Sint => {
            Some(integer(0, s.width, s.kind == ScalarKind::Sint))
        }
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

/// The integer scalar `e` (as [`int`] gives it), with `offset` and `count` clamped to its width in bits, `n`: `(o, c)`
/// with `o = min(offset, n)` and `c = min(count, n - o)`, as WGSL's `extractBits` and `insertBits` clamp them.
fn bit_field(e: &Value, offset: &Value, count: &Value) -> Option<((u64, u8, bool), u32, u32)> {
    let (&Value::Uint(o), &Value::Uint(c)) = (offset, count) else {
        return None;
    };
    let e = int(e)?;
    let n = 8 * u32::from(e.1);
    let o = o.min(n);
    Some((e, o, c.min(n - o)))
}

/// WGSL's `extractBits(e, offset, count)` on the integer scalar `e`: the `count` bits from `offset`, sign-extended for
/// a signed `e`, with `offset` and `count` clamped to its width ([`bit_field`]).
fn extract_field(e: &Value, offset: &Value, count: &Value) -> Option<Value> {
    let ((x, w, signed), o, c) = bit_field(e, offset, count)?;
    if c == 0 {
        return Some(integer(0, w, signed));
    }
    let r = if signed {
        ((extend(x, w) << (64 - o - c)) >> (64 - c)).cast_unsigned()
    } else {
        (x >> o) & (u64::MAX >> (64 - c))
    };
    Some(integer(r, w, signed))
}

/// WGSL's `insertBits(e, newbits, offset, count)` on the integer scalars `e` and `newbits`: `e` with its `count` bits
/// from `offset` replaced by `newbits`' low bits, `offset` and `count` clamped to its width ([`bit_field`]).
fn insert_bits(e: &Value, newbits: &Value, offset: &Value, count: &Value) -> Option<Value> {
    let ((x, w, signed), o, c) = bit_field(e, offset, count)?;
    let (n, ..) = int(newbits)?;
    let r = if c == 0 {
        x
    } else {
        let field = (u64::MAX >> (64 - c)) << o;
        (x & !field) + ((n << o) & field)
    };
    Some(integer(r, w, signed))
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

/// The bit pattern of each integer lane of `args` mapped by `f` (given the pattern, zero-extended, and the width in
/// bits), the lane's type kept and the result truncated to it.
fn bit_lanes(args: &[Value], f: fn(u64, u32) -> u64) -> Option<Value> {
    lanes(args, &|lane| {
        let (b, w, signed) = int(&lane[0])?;
        Some(integer(f(b, 8 * u32::from(w)), w, signed))
    })
}

/// The position of `b`'s most significant set bit, or all ones if none is set.
fn first_leading(b: u64) -> u64 {
    if b == 0 {
        u64::MAX
    } else {
        u64::from(63 - b.leading_zeros())
    }
}

/// The larger (`max`) or smaller of two scalars of one type (a float's, by `f64::max` and `f64::min`).
fn min_max(a: &Value, b: &Value, max: bool) -> Option<Value> {
    if let (&Value::Float(x, w), &Value::Float(y, _)) = (a, b) {
        return Some(float(if max { x.max(y) } else { x.min(y) }, w));
    }
    let ((x, w, signed), (y, yw, y_signed)) = (int(a)?, int(b)?);
    if (yw, y_signed) != (w, signed) {
        return None;
    }
    let r = if signed {
        let (x, y) = (extend(x, w), extend(y, w));
        (if max { x.max(y) } else { x.min(y) }).cast_unsigned()
    } else if max {
        x.max(y)
    } else {
        x.min(y)
    };
    Some(integer(r, w, signed))
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
            ref v => match int(v)? {
                (b, w, true) => Some(integer(
                    extend(b, w).wrapping_abs().cast_unsigned(),
                    w,
                    true,
                )),
                (b, w, false) => Some(integer(b, w, false)),
            },
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
            ref v => match int(v)? {
                (b, w, true) => Some(integer(extend(b, w).signum().cast_unsigned(), w, true)),
                _ => None,
            },
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
        M::CountTrailingZeros => bit_lanes(args, |b, n| u64::from(b.trailing_zeros().min(n))),
        M::CountLeadingZeros => bit_lanes(args, |b, n| u64::from(b.leading_zeros() - (64 - n))),
        M::CountOneBits => bit_lanes(args, |b, _| u64::from(b.count_ones())),
        M::ReverseBits => bit_lanes(args, |b, n| b.reverse_bits() >> (64 - n)),
        M::FirstTrailingBit => bit_lanes(args, |b, _| {
            if b == 0 {
                u64::MAX
            } else {
                u64::from(b.trailing_zeros())
            }
        }),
        M::FirstLeadingBit => lanes(args, &|l| {
            let (b, w, signed) = int(&l[0])?;
            // A signed integer's highest bit that differs from its sign bit: the highest set bit of it, or of its
            // complement.
            let b = if signed && extend(b, w) < 0 {
                !b & mask(w)
            } else {
                b
            };
            Some(integer(first_leading(b), w, signed))
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
        M::Pack4xI8 | M::Pack4xU8 => pack(arg(0)?, &|v| Some(vec![int(v)?.0.to_le_bytes()[0]])),
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

/// The value of `h`, if it is a constant expression: a literal (float, an integer of any width, or bool); a module
/// constant; a zero value; a unary or binary operator on constants, a matrix product included; a splat; a vector, matrix or array
/// built of constants, one of its components, or a swizzle; a `select` or an `all` or `any` of them; any built-in
/// function WGSL allows in a constant expression ([`math`]); a conversion or a `bitcast` of a constant. Each is
/// evaluated as WGSL does at run time, float arithmetic in the operands' precision (f32 for f32). A NaN made by a
/// `bitcast` keeps its bit pattern until something computes on it ([`Value::NanBits`]), so a `bitcast` of it back to
/// an integer is exact. `None` otherwise. A bool `&&` or `||` over a runtime value is no expression in naga's IR: naga lowers it, short-circuiting, to an
/// `if` that stores to a variable, which is not followed.
fn evaluate(module: &Module, arena: &Arena<Expression>, h: Handle<Expression>) -> Option<Value> {
    let value = |e| evaluate(module, arena, e);
    // An operand of an operator, a conversion or a computing built-in: its NaNs' bits are not known ([`Value::plain`]).
    let computed = |e| value(e).map(Value::plain);
    match arena[h] {
        Expression::Literal(literal) => match literal {
            Literal::F64(v) => Some(float(v, 8)),
            Literal::F32(v) => Some(float(f64::from(v), 4)),
            Literal::F16(v) => Some(float(v.to_f64(), 2)),
            Literal::U32(u) => Some(Value::Uint(u)),
            Literal::I32(i) => Some(Value::Sint(i)),
            Literal::U16(u) => Some(integer(u64::from(u), 2, false)),
            Literal::I16(i) => Some(integer(u64::from(i.cast_unsigned()), 2, true)),
            Literal::U64(u) => Some(integer(u, 8, false)),
            Literal::I64(i) => Some(integer(i.cast_unsigned(), 8, true)),
            Literal::Bool(b) => Some(Value::Bool(b)),
            _ => None,
        },
        Expression::Constant(c) => {
            evaluate(module, &module.global_expressions, module.constants[c].init)
        }
        Expression::ZeroValue(ty) => zero(module, ty),
        Expression::Unary { op, expr } => lanes(&[computed(expr)?], &|l| unary(op, &l[0])),
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
            let (i, w, signed) = int(&value(index)?)?;
            let i = if signed {
                usize::try_from(extend(i, w)).ok()?
            } else {
                usize::try_from(i).ok()?
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
        Expression::Binary { op, left, right } => binary(op, computed(left)?, computed(right)?),
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
            // `transpose` moves its argument's components without computing them, so a NaN's known bits stay known.
            let operand = |e| {
                if fun == MathFunction::Transpose {
                    value(e)
                } else {
                    computed(e)
                }
            };
            let args = [Some(arg), arg1, arg2, arg3]
                .into_iter()
                .flatten()
                .map(operand)
                .collect::<Option<Vec<_>>>()?;
            math(fun, &args)
        }
        Expression::As {
            expr,
            kind,
            convert,
        } => {
            // A `bitcast` keeps a NaN's known bits; a conversion computes, so it sees a NaN of unknown bits.
            let v = if convert.is_none() {
                value(expr)?
            } else {
                computed(expr)?
            };
            lanes(&[v], &|l| cast(&l[0], kind, convert))
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

/// Whether another invocation can write a global in `space` while this one runs: workgroup memory, a read-write
/// storage buffer, or a payload shared with other shaders.
fn shared(space: AddressSpace) -> bool {
    match space {
        AddressSpace::WorkGroup
        | AddressSpace::TaskPayload
        | AddressSpace::RayPayload
        | AddressSpace::IncomingRayPayload => true,
        AddressSpace::Storage { access } => access.contains(naga::StorageAccess::STORE),
        AddressSpace::Function
        | AddressSpace::Private
        | AddressSpace::Uniform
        | AddressSpace::Handle
        | AddressSpace::Immediate => false,
    }
}

/// The variables `statement`, in a function over `arena`, may write (R-352's "store"): a store's or an atomic's
/// pointer; each pointer a call is given that its callee may write through, and every global its callee may write,
/// before it returns (`summaries`, by function, from [`summaries`]); and, for a statement that waits on or hands
/// control to other invocations (a barrier, `workgroupUniformLoad`, a ray-pipeline call), every global another
/// invocation can write.
fn written(
    module: &Module,
    summaries: &[Summary],
    arena: &Arena<Expression>,
    statement: &Statement,
) -> Vec<Root> {
    let shared_globals = || {
        module
            .global_variables
            .iter()
            .filter(|(_, g)| shared(g.space))
            .map(|(h, _)| Root::Global(h))
            .collect()
    };
    match *statement {
        Statement::Store { pointer, .. }
        | Statement::Atomic { pointer, .. }
        | Statement::CooperativeStore {
            target: pointer, ..
        }
        | Statement::ImageStore { image: pointer, .. }
        | Statement::ImageAtomic { image: pointer, .. } => {
            root(arena, pointer).into_iter().collect()
        }
        Statement::Call {
            function,
            ref arguments,
            ..
        } => {
            let callee = &summaries[function.index()].writes;
            (0u32..)
                .zip(arguments)
                .filter(|&(k, _)| callee.contains(&Root::Argument(k)))
                .filter_map(|(_, &a)| root(arena, a))
                .chain(
                    callee
                        .iter()
                        .copied()
                        .filter(|r| matches!(r, Root::Global(_))),
                )
                .collect()
        }
        Statement::ControlBarrier(_)
        | Statement::MemoryBarrier(_)
        | Statement::WorkGroupUniformLoad { .. }
        | Statement::RayPipelineFunction(_) => shared_globals(),
        _ => Vec::new(),
    }
}

/// What a call to a function does, as its caller sees it: whether it may return (rather than discard on every path,
/// or never leave a loop), and what it may write on some path from its start that returns: each global, each pointer
/// argument (by its index) it may write through, and its own locals, which [`written`] leaves out at a call. A store
/// that no path from the start reaches, or from which every path discards, is not in `writes`: no read after the call
/// sees it.
struct Summary {
    returns: bool,
    writes: Vec<Root>,
}

/// Each function of `module`'s [`Summary`] (by index), from its own control-flow graph ([`Timeline`]), each call in
/// which is its callee's summary. naga's validator puts a callee before its callers in the arena, so one pass in
/// order sees each callee's summary complete.
fn summaries(module: &Module) -> Vec<Summary> {
    let mut summaries: Vec<Summary> = Vec::new();
    for (_, f) in module.functions.iter() {
        let timeline = Timeline::new(module, &summaries, &f.expressions, &f.body);
        let started = timeline.reachable(timeline.start);
        let writes = timeline
            .stores
            .iter()
            .filter(|&&(s, _)| {
                started.contains(&s) && timeline.reachable(s).contains(&timeline.exit)
            })
            .map(|&(_, r)| r)
            .collect();
        summaries.push(Summary {
            returns: started.contains(&timeline.exit),
            writes,
        });
    }
    summaries
}

/// A function's control-flow graph, one node (a step) per statement in source order and a last step, `exit`, that
/// a return passes to: the step at which each expression is evaluated, each store with its step and the variable it
/// writes ([`written`]), the steps control may pass to from each step, and the step control starts at.
struct Timeline {
    at: HashMap<Handle<Expression>, usize>,
    stores: Vec<(usize, Root)>,
    next: Vec<Vec<usize>>,
    start: usize,
    exit: usize,
}

impl Timeline {
    fn new(
        module: &Module,
        summaries: &[Summary],
        arena: &Arena<Expression>,
        body: &Block,
    ) -> Self {
        let mut timeline = Timeline {
            at: HashMap::new(),
            stores: Vec::new(),
            next: Vec::new(),
            start: 0,
            exit: 0,
        };
        let mut steps = HashMap::new();
        timeline.number(module, summaries, arena, body, &mut steps);
        timeline.exit = timeline.next.len();
        timeline.next.push(Vec::new());
        let exit = [timeline.exit];
        timeline.link(summaries, &steps, body, &exit, &[], &[]);
        timeline.start = Self::entry(&steps, body, &exit)[0];
        timeline
    }

    /// Gives each of `block`'s statements its step (in `steps`, by address), each compound before the statements it
    /// holds, and records the expressions it evaluates and the variables it writes.
    fn number(
        &mut self,
        module: &Module,
        summaries: &[Summary],
        arena: &Arena<Expression>,
        block: &Block,
        steps: &mut HashMap<*const Statement, usize>,
    ) {
        for statement in block.iter() {
            let step = self.next.len();
            self.next.push(Vec::new());
            steps.insert(statement, step);
            if let Statement::Emit(ref range) = *statement {
                for h in range.clone() {
                    self.at.insert(h, step);
                }
            }
            for r in written(module, summaries, arena, statement) {
                self.stores.push((step, r));
            }
            match *statement {
                Statement::Block(ref b) => self.number(module, summaries, arena, b, steps),
                Statement::If {
                    ref accept,
                    ref reject,
                    ..
                } => {
                    self.number(module, summaries, arena, accept, steps);
                    self.number(module, summaries, arena, reject, steps);
                }
                Statement::Switch { ref cases, .. } => {
                    for case in cases {
                        self.number(module, summaries, arena, &case.body, steps);
                    }
                }
                Statement::Loop {
                    ref body,
                    ref continuing,
                    ..
                } => {
                    self.number(module, summaries, arena, body, steps);
                    self.number(module, summaries, arena, continuing, steps);
                }
                _ => {}
            }
        }
    }

    /// The steps control enters `block` at: its first statement's, or `after` if it is empty.
    fn entry(
        steps: &HashMap<*const Statement, usize>,
        block: &Block,
        after: &[usize],
    ) -> Vec<usize> {
        block
            .iter()
            .next()
            .map_or_else(|| after.to_vec(), |s| vec![steps[&(s as *const _)]])
    }

    /// Records where control may pass from each of `block`'s statements, and from those they hold: control leaves
    /// `block`'s end for `after`, a `break` for `out`, a `continue` for `again` (naga's IR, `Statement::Loop`,
    /// `Statement::Break`, `Statement::Continue`). A `return` passes to `exit`; a `discard` ("Aborts the current
    /// shader execution", `Statement::Kill`) passes nowhere, nor does a call whose callee never returns
    /// ([`Summary`]); a switch case falls through to the next case's entry or leaves the switch; a loop's body passes
    /// to its continuing, and its continuing back to the loop's head and, with a `break if`, out.
    fn link(
        &mut self,
        summaries: &[Summary],
        steps: &HashMap<*const Statement, usize>,
        block: &Block,
        after: &[usize],
        out: &[usize],
        again: &[usize],
    ) {
        let statements: Vec<&Statement> = block.iter().collect();
        for (k, &statement) in statements.iter().enumerate() {
            let me = steps[&(statement as *const _)];
            let next = statements
                .get(k + 1)
                .map_or_else(|| after.to_vec(), |&s| vec![steps[&(s as *const _)]]);
            let to = match *statement {
                Statement::Block(ref b) => {
                    self.link(summaries, steps, b, &next, out, again);
                    Self::entry(steps, b, &next)
                }
                Statement::If {
                    ref accept,
                    ref reject,
                    ..
                } => {
                    self.link(summaries, steps, accept, &next, out, again);
                    self.link(summaries, steps, reject, &next, out, again);
                    let mut to = Self::entry(steps, accept, &next);
                    to.extend(Self::entry(steps, reject, &next));
                    to
                }
                Statement::Switch { ref cases, .. } => {
                    // From the last case back, each case's entry is where the case before it falls through to.
                    let mut to = Vec::new();
                    let mut following = next.clone();
                    for case in cases.iter().rev() {
                        let end = if case.fall_through {
                            following
                        } else {
                            next.clone()
                        };
                        self.link(summaries, steps, &case.body, &end, &next, again);
                        following = Self::entry(steps, &case.body, &end);
                        to.extend(following.iter().copied());
                    }
                    to
                }
                Statement::Loop {
                    ref body,
                    ref continuing,
                    break_if,
                } => {
                    let mut back = vec![me];
                    if break_if.is_some() {
                        back.extend(next.iter().copied());
                    }
                    // naga's validator keeps `break` and `continue` that target this loop out of its continuing.
                    self.link(summaries, steps, continuing, &back, &next, &back);
                    let continuing = Self::entry(steps, continuing, &back);
                    self.link(summaries, steps, body, &continuing, &next, &continuing);
                    Self::entry(steps, body, &continuing)
                }
                Statement::Return { .. } => vec![self.exit],
                Statement::Call { function, .. } if !summaries[function.index()].returns => {
                    Vec::new()
                }
                Statement::Kill => Vec::new(),
                Statement::Break => out.to_vec(),
                Statement::Continue => again.to_vec(),
                _ => next,
            };
            self.next[me] = to;
        }
    }

    /// Whether `r` may be written between the evaluations of `a` and `b` that the expression evaluated at `at` uses,
    /// on some path; true if any of the three steps is unknown.
    fn stored_between(
        &self,
        r: Root,
        at: Handle<Expression>,
        a: Handle<Expression>,
        b: Handle<Expression>,
    ) -> bool {
        let (Some(&ta), Some(&tb), Some(&tc)) =
            (self.at.get(&a), self.at.get(&b), self.at.get(&at))
        else {
            return true;
        };
        self.stores.iter().any(|&(s, w)| {
            w == r && (self.separates(ta, tb, s, tc) || self.separates(tb, ta, s, tc))
        })
    }

    /// Whether the store at step `s` may run after the read at step `x` and before the read at step `y`, both of
    /// which the step `c` then uses: some path runs from `x` through `s` to `y` and on to `c` without passing `x`
    /// again, which would read it afresh after the store.
    fn separates(&self, x: usize, y: usize, s: usize, c: usize) -> bool {
        self.reaches(x, s, x) && self.reaches(s, y, x) && self.reaches(y, c, x)
    }

    /// The steps control may reach from step `from`, itself included.
    fn reachable(&self, from: usize) -> HashSet<usize> {
        let mut seen = HashSet::from([from]);
        let mut stack = vec![from];
        while let Some(step) = stack.pop() {
            for &n in &self.next[step] {
                if seen.insert(n) {
                    stack.push(n);
                }
            }
        }
        seen
    }

    /// Whether control may pass from step `from` to step `to` (at once, if they are one step) without entering step
    /// `avoid`.
    fn reaches(&self, from: usize, to: usize, avoid: usize) -> bool {
        if from == to {
            return true;
        }
        // `avoid` counts as seen from the start, so the search never enters it.
        let mut seen = HashSet::from([avoid]);
        let mut stack = vec![from];
        while let Some(step) = stack.pop() {
            for &n in &self.next[step] {
                if seen.insert(n) {
                    stack.push(n);
                }
            }
        }
        to != avoid && seen.contains(&to)
    }
}

/// Whether `a` and `b` are the same value: the same expression, or structurally equal expressions over the same
/// leaves, each pair of loads reading the same place with no store to its variable between them (R-352), as the
/// expression `at` uses them.
fn same(
    arena: &Arena<Expression>,
    timeline: &Timeline,
    at: Handle<Expression>,
    a: Handle<Expression>,
    b: Handle<Expression>,
) -> bool {
    if a == b {
        return true;
    }
    let eq = |x, y| same(arena, timeline, at, x, y);
    let eq_opt = |x: Option<_>, y: Option<_>| match (x, y) {
        (Some(x), Some(y)) => eq(x, y),
        (None, None) => true,
        _ => false,
    };
    match (&arena[a], &arena[b]) {
        (Expression::Load { pointer: p }, Expression::Load { pointer: q }) => {
            eq(*p, *q) && root(arena, *p).is_none_or(|r| !timeline.stored_between(r, at, a, b))
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
