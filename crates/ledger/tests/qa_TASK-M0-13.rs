//! QA tests for TASK-M0-13, written from the requirements, not from the implementation: the checked-in generated
//! WGSL (`crates/render/frag/generated/payload_unpack.wgsl`), parsed with naga, against the corpus's own numbers.
//! - REQ-PAY-091 (payload §1; dd_generation_root §3.6; R-86, R-343): `SimStateFTLE` and `SimStateBase` have payload
//!   §1's members in its order, at the offsets its sizes give (144 B and 96 B, aligned to 8), with `closure_step` and
//!   `_reserved` as one `closure_step_reserved: u32` at byte 140 (FTLE) and 92 (Base, the 48 B shadow dropped);
//!   `ICDescriptor` is §3.6's twelve f32 fields then 16 B of declared padding, 64 B in all.
//! - REQ-RENDER-001 (render contract Part 5 "Unpack layer"; R-343, R-271): `simstate_buffer: array<SimStateFTLE>` at
//!   `@group(1) @binding(0)`, `word_buffer: array<vec4<u32>>` at `@group(1) @binding(1)`, both read-only storage;
//!   nothing in group 0; the attributes equal `SIMSTATE_GROUP`, `SIMSTATE_BINDING`, `WORD_GROUP`, `WORD_BINDING` in
//!   the WGSL and in the generated Rust; only the read side's `sample_read` touches the buffers, both at its sample
//!   index, one stored member per load and never a whole stored struct (R-378, amending R-343's `sample_state` /
//!   `sample_word`), checked on the fragment's generated WGSL, `payload_unpack.wgsl` then `read_side.wgsl`; no setter
//!   (no store to a buffer, no `insertBits`, no `set_` function); no `isinf`/`isnan`, no self-comparison and no float
//!   comparison standing in for an infinity test; `PA_D_MIN_UNSET` is f16 +inf's bits, 0x7c00.
//!
//! Each test has a registered negative control (R-176). The GPU-run checks (accessor values, `closure_step`,
//! `pa_d_min_is_unset`, the tables, the schema version) are in `crates/kernel/tests/qa_TASK-M0-13.rs`.

use std::path::Path;

use naga::{
    AddressSpace, ArraySize, BinaryOperator, Block, Expression, Literal, MathFunction, Module,
    Scalar, ScalarKind, Statement, StorageAccess, TypeInner,
};
use validation::negative_control;

const WGSL: &str = "crates/render/frag/generated/payload_unpack.wgsl";
/// The generated read side, which follows the unpack layer in the fragment's WGSL and holds `sample_read` (R-378).
const READ_SIDE: &str = "crates/render/frag/generated/read_side.wgsl";
const RUST: &str = "crates/kernel/src/payload/generated.rs";

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn wgsl() -> String {
    read(WGSL)
}

/// The fragment's generated WGSL: the unpack layer, then the read side.
fn fragment() -> String {
    format!("{}\n{}", read(WGSL), read(READ_SIDE))
}

fn parse(src: &str) -> Module {
    naga::front::wgsl::parse_str(src).unwrap_or_else(|e| panic!("{}", e.emit_to_string(src)))
}

fn ty_name(m: &Module, h: naga::Handle<naga::Type>) -> String {
    let sc = |s: Scalar| match (s.kind, s.width) {
        (ScalarKind::Float, 4) => "f32".to_string(),
        (ScalarKind::Uint, 4) => "u32".to_string(),
        (ScalarKind::Sint, 4) => "i32".to_string(),
        other => format!("{other:?}"),
    };
    match &m.types[h].inner {
        TypeInner::Scalar(s) => sc(*s),
        TypeInner::Vector { size, scalar } => format!("vec{}<{}>", *size as u8, sc(*scalar)),
        TypeInner::Array { base, size, .. } => match size {
            ArraySize::Constant(n) => format!("array<{}, {n}>", ty_name(m, *base)),
            _ => format!("array<{}>", ty_name(m, *base)),
        },
        TypeInner::Struct { .. } => m.types[h].name.clone().unwrap_or_default(),
        other => format!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-091: the struct layouts, from payload §1 and dd_generation_root §3.6 as written there.

const V2X3: &str = "array<vec2<f32>, 3>";

/// Payload §1's `SimStateFTLE`, member by member: name, WGSL type, byte size.
const FTLE: [(&str, &str, u32); 16] = [
    ("r", V2X3, 24),
    ("p", V2X3, 24),
    ("r_sh", V2X3, 24),
    ("p_sh", V2X3, 24),
    ("S", "f32", 4),
    ("theta", "f32", 4),
    ("mean_y", "f32", 4),
    ("C_ty", "f32", 4),
    ("E_0", "f32", 4),
    ("Lz_0", "f32", 4),
    ("packed_a", "u32", 4),
    ("packed_b", "u32", 4),
    ("times", "u32", 4),
    ("total_substeps", "u32", 4),
    ("closure_min", "f32", 4),
    // closure_step: u16 and _reserved: u16 as one u32 (R-343).
    ("closure_step_reserved", "u32", 4),
];

/// `(name, offset, type)` for each member of `fields` laid end to end from 0.
fn packed(fields: &[(&'static str, &'static str, u32)]) -> Vec<(String, u32, String)> {
    let mut at = 0;
    fields
        .iter()
        .map(|&(n, t, s)| {
            let r = (n.to_string(), at, t.to_string());
            at += s;
            r
        })
        .collect()
}

/// `(name, offset, type)` of each member of WGSL struct `name`, its span and its alignment.
fn wgsl_struct(src: &str, name: &str) -> (Vec<(String, u32, String)>, u32, u32) {
    let m = parse(src);
    let mut lay = naga::proc::Layouter::default();
    lay.update(m.to_ctx()).expect("layouts");
    let (h, t) = m
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no WGSL struct `{name}`"));
    let TypeInner::Struct { members, span } = &t.inner else {
        panic!("`{name}` is not a struct")
    };
    let got = members
        .iter()
        .map(|mm| {
            (
                mm.name.clone().unwrap_or_default(),
                mm.offset,
                ty_name(&m, mm.ty),
            )
        })
        .collect();
    (got, *span, lay[h].alignment.round_up(1))
}

fn check_simstate(src: &str) {
    let base: Vec<_> = FTLE
        .iter()
        .copied()
        .filter(|(n, _, _)| *n != "r_sh" && *n != "p_sh")
        .collect();
    for (name, fields, size) in [
        ("SimStateFTLE", FTLE.to_vec(), 144),
        ("SimStateBase", base, 96),
    ] {
        let (got, span, align) = wgsl_struct(src, name);
        let want = packed(&fields);
        assert_eq!(got, want, "`{name}` is not payload §1's layout");
        assert_eq!(span, size, "`{name}` is {span} B, not payload §1's {size}");
        assert_eq!(align, 8, "`{name}` is aligned to {align}, not 8");
    }
    // R-343's offsets, stated as numbers: 140 (FTLE), 92 (Base).
    let (f, ..) = wgsl_struct(src, "SimStateFTLE");
    let (b, ..) = wgsl_struct(src, "SimStateBase");
    let at = |v: &[(String, u32, String)]| {
        v.iter()
            .find(|x| x.0 == "closure_step_reserved")
            .map(|x| (x.1, x.2.clone()))
    };
    assert_eq!(
        at(&f),
        Some((140, "u32".to_string())),
        "FTLE closure_step_reserved"
    );
    assert_eq!(
        at(&b),
        Some((92, "u32".to_string())),
        "Base closure_step_reserved"
    );
}

#[test]
fn qa_wgsl_simstate_layouts_are_payload_section_1() {
    check_simstate(&wgsl());
}

negative_control!(
    qa_wgsl_simstate_layouts_are_payload_section_1,
    "closure_min declared u32, not f32, is not payload §1's layout",
    expected = "`SimStateFTLE` is not payload §1's layout",
    check_simstate(&wgsl().replacen("closure_min: f32,", "closure_min: u32,", 1))
);

#[test]
fn qa_wgsl_simstate_base_drops_only_the_shadow() {
    check_simstate(&wgsl());
}

negative_control!(
    qa_wgsl_simstate_base_drops_only_the_shadow,
    "a SimStateBase that keeps the shadow is 144 B, not 96",
    expected = "`SimStateBase` is not payload §1's layout",
    check_simstate(&wgsl().replacen(
        "struct SimStateBase {\n    r: array<vec2<f32>, 3>,\n    p: array<vec2<f32>, 3>,",
        "struct SimStateBase {\n    r: array<vec2<f32>, 3>,\n    p: array<vec2<f32>, 3>,\n    r_sh: array<vec2<f32>, 3>,\n    p_sh: array<vec2<f32>, 3>,",
        1
    ))
);

/// dd_generation_root §3.6: twelve f32 fields in order, then declared padding to 64 B (R-86).
const IC: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

fn check_icdescriptor(src: &str) {
    let (got, span, _) = wgsl_struct(src, "ICDescriptor");
    assert!(
        got.len() > 12,
        "`ICDescriptor` declares no padding: {got:?}"
    );
    for (i, n) in IC.iter().enumerate() {
        assert_eq!(
            got[i],
            (n.to_string(), 4 * i as u32, "f32".to_string()),
            "`ICDescriptor` member {i} is not §3.6's `{n}`"
        );
    }
    for pad in &got[12..] {
        assert!(
            pad.0.starts_with('_'),
            "`ICDescriptor.{}` is not declared padding",
            pad.0
        );
        assert!(pad.1 >= 48, "padding inside the fields");
    }
    assert_eq!(span, 64, "`ICDescriptor` is {span} B, not R-86's 64");
}

#[test]
fn qa_wgsl_icdescriptor_is_generation_root_3_6() {
    check_icdescriptor(&wgsl());
}

negative_control!(
    qa_wgsl_icdescriptor_is_generation_root_3_6,
    "V_0 and K_0 exchanged is not §3.6's order",
    expected = "`ICDescriptor` member 8 is not §3.6's `K_0`",
    check_icdescriptor(&wgsl().replacen(
        "    K_0: f32,\n    V_0: f32,",
        "    V_0: f32,\n    K_0: f32,",
        1
    ))
);

#[test]
fn qa_wgsl_icdescriptor_is_64_bytes() {
    check_icdescriptor(&wgsl());
}

negative_control!(
    qa_wgsl_icdescriptor_is_64_bytes,
    "padding of 12 B leaves the descriptor at 60 B",
    expected = "`ICDescriptor` is 60 B, not R-86's 64",
    check_icdescriptor(&wgsl().replacen("_pad: array<u32, 4>,", "_pad: array<u32, 3>,", 1))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-RENDER-001: bindings (R-343).

fn wgsl_u32_const(m: &Module, name: &str) -> u32 {
    let (_, c) = m
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no WGSL const `{name}`"));
    match m.global_expressions[c.init] {
        Expression::Literal(Literal::U32(v)) => v,
        ref e => panic!("`{name}` is not a u32 literal: {e:?}"),
    }
}

fn rust_u32_const(src: &str, name: &str) -> u32 {
    let head = format!("pub const {name}: u32 = ");
    let line = src
        .lines()
        .find(|l| l.starts_with(&head))
        .unwrap_or_else(|| panic!("no Rust `pub const {name}: u32`"));
    line[head.len()..]
        .trim_end_matches(';')
        .trim()
        .parse()
        .expect("a decimal u32")
}

fn check_bindings(src: &str, rust: &str) {
    let m = parse(src);
    // R-343's numbers, from the ruling, not the ledger.
    for (buf, elem, prefix, group, binding) in [
        ("simstate_buffer", "array<SimStateFTLE>", "SIMSTATE", 1, 0),
        ("word_buffer", "array<vec4<u32>>", "WORD", 1, 1),
    ] {
        let (_, g) = m
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some(buf))
            .unwrap_or_else(|| panic!("no `{buf}`"));
        assert_eq!(ty_name(&m, g.ty), elem, "`{buf}`'s type");
        assert_eq!(
            g.space,
            AddressSpace::Storage {
                access: StorageAccess::LOAD
            },
            "`{buf}` is not read-only storage"
        );
        let b = g
            .binding
            .as_ref()
            .unwrap_or_else(|| panic!("`{buf}` unbound"));
        assert_eq!(
            (b.group, b.binding),
            (group, binding),
            "`{buf}` is not at R-343's binding"
        );
        for (suffix, v) in [("GROUP", b.group), ("BINDING", b.binding)] {
            let name = format!("{prefix}_{suffix}");
            assert_eq!(
                wgsl_u32_const(&m, &name),
                v,
                "WGSL `{name}` differs from `{buf}`'s attribute"
            );
            assert_eq!(
                rust_u32_const(rust, &name),
                v,
                "Rust `{name}` differs from `{buf}`'s attribute"
            );
        }
    }
    for (_, g) in m.global_variables.iter() {
        if let Some(b) = &g.binding {
            assert_ne!(b.group, 0, "`{:?}` is bound in group 0", g.name);
        }
    }
}

#[test]
fn qa_wgsl_bindings_are_r343s() {
    check_bindings(&wgsl(), &read(RUST));
}

negative_control!(
    qa_wgsl_bindings_are_r343s,
    "the state buffer in group 0 is not R-343's binding",
    expected = "`simstate_buffer` is not at R-343's binding",
    check_bindings(
        &wgsl().replace("@group(1) @binding(0) var", "@group(0) @binding(0) var"),
        &read(RUST)
    )
);

#[test]
fn qa_wgsl_binding_constants_match_the_rust() {
    check_bindings(&wgsl(), &read(RUST));
}

negative_control!(
    qa_wgsl_binding_constants_match_the_rust,
    "a Rust WORD_BINDING of 2 differs from the attribute",
    expected = "Rust `WORD_BINDING` differs",
    check_bindings(
        &wgsl(),
        &read(RUST).replace(
            "pub const WORD_BINDING: u32 = 1;",
            "pub const WORD_BINDING: u32 = 2;"
        )
    )
);

#[test]
fn qa_wgsl_buffers_are_read_only_storage() {
    check_bindings(&wgsl(), &read(RUST));
}

negative_control!(
    qa_wgsl_buffers_are_read_only_storage,
    "a writable word buffer is not read-only",
    expected = "`word_buffer` is not read-only storage",
    check_bindings(
        &wgsl().replace(
            "var<storage, read> word_buffer",
            "var<storage, read_write> word_buffer"
        ),
        &read(RUST)
    )
);

#[test]
fn qa_wgsl_nothing_in_group_0() {
    check_bindings(&wgsl(), &read(RUST));
}

negative_control!(
    qa_wgsl_nothing_in_group_0,
    "an extra uniform in group 0 must fail",
    expected = "is bound in group 0",
    check_bindings(
        &format!(
            "{}\n@group(0) @binding(0) var<uniform> frame: vec4<f32>;\n",
            wgsl()
        ),
        &read(RUST)
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-RENDER-001: only sample_read uses the buffers, one stored member per load (R-343 as R-378 amends it); no setter.

/// The global a pointer expression in `f` is rooted at, if any.
fn root_global(
    f: &naga::Function,
    mut e: naga::Handle<Expression>,
) -> Option<naga::Handle<naga::GlobalVariable>> {
    loop {
        match f.expressions[e] {
            Expression::GlobalVariable(g) => return Some(g),
            Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => e = base,
            _ => return None,
        }
    }
}

fn stores_to_global(f: &naga::Function, b: &Block) -> bool {
    b.iter().any(|s| match s {
        Statement::Store { pointer, .. } => root_global(f, *pointer).is_some(),
        Statement::Atomic { pointer, .. } => root_global(f, *pointer).is_some(),
        Statement::Block(b) => stores_to_global(f, b),
        Statement::If { accept, reject, .. } => {
            stores_to_global(f, accept) || stores_to_global(f, reject)
        }
        Statement::Loop {
            body, continuing, ..
        } => stores_to_global(f, body) || stores_to_global(f, continuing),
        Statement::Switch { cases, .. } => cases.iter().any(|c| stores_to_global(f, &c.body)),
        _ => false,
    })
}

/// The steps from the global a pointer expression in `f` is rooted at down to it: `None` for a runtime index (its
/// expression alongside), `Some(k)` for a member or component.
fn access_path(
    f: &naga::Function,
    mut e: naga::Handle<Expression>,
) -> Vec<(Option<u32>, Option<naga::Handle<Expression>>)> {
    let mut steps = Vec::new();
    loop {
        match f.expressions[e] {
            Expression::Access { base, index } => {
                steps.push((None, Some(index)));
                e = base;
            }
            Expression::AccessIndex { base, index } => {
                steps.push((Some(index), None));
                e = base;
            }
            _ => break,
        }
    }
    steps.reverse();
    steps
}

/// The one reader of both buffers (R-343, R-378).
const READER: &str = "sample_read";

fn check_read_only_access(src: &str) {
    let m = parse(src);
    let fns = m
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(m.entry_points.iter().map(|e| &e.function));
    for f in fns {
        let name = f.name.clone().unwrap_or_default();
        for (_, e) in f.expressions.iter() {
            if let Expression::GlobalVariable(g) = *e {
                let gname = m.global_variables[g].name.as_deref().unwrap_or("");
                if gname == "simstate_buffer" || gname == "word_buffer" {
                    assert_eq!(
                        name, READER,
                        "`{name}` uses `{gname}`, which only `{READER}` may read"
                    );
                }
            }
            if let Expression::Math {
                fun: MathFunction::InsertBits,
                ..
            } = e
            {
                panic!("`{name}` calls insertBits: the unpack layer emits no setter");
            }
        }
        assert!(
            !stores_to_global(f, &f.body),
            "`{name}` stores to a global: the layer only reads"
        );
        assert!(
            !name.starts_with("set_") && !name.contains("_set_"),
            "`{name}` is a setter: the layer only reads"
        );
    }
    // `sample_read(i, …)` indexes both buffers by its first argument, a u32, the same i for both, and loads one stored
    // member at a time, never a whole `SimStateFTLE` (R-378).
    let (_, f) = m
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some(READER))
        .unwrap_or_else(|| panic!("no `{READER}`"));
    assert!(!f.arguments.is_empty(), "`{READER}` takes no index");
    assert_eq!(
        ty_name(&m, f.arguments[0].ty),
        "u32",
        "`{READER}`'s index is not a u32"
    );
    let mut read = Vec::new();
    for (_, e) in f.expressions.iter() {
        let Expression::Load { pointer } = *e else {
            continue;
        };
        let Some(g) = root_global(f, pointer) else {
            continue;
        };
        let gname = m.global_variables[g].name.clone().unwrap_or_default();
        if gname != "simstate_buffer" && gname != "word_buffer" {
            continue;
        }
        let path = access_path(f, pointer);
        let by_arg = matches!(path.first(), Some((None, Some(index)))
            if matches!(f.expressions[*index], Expression::FunctionArgument(0)));
        assert!(
            by_arg,
            "`{READER}` does not index `{gname}` by its argument"
        );
        if gname == "simstate_buffer" {
            assert!(
                matches!(path.get(1), Some((Some(_), _))),
                "`{READER}` loads a whole stored struct from `simstate_buffer`, not one member (R-378)"
            );
        }
        read.push(gname);
    }
    for buffer in ["simstate_buffer", "word_buffer"] {
        assert!(
            read.iter().any(|g| g == buffer),
            "`{READER}` does not read `{buffer}`"
        );
    }
}

#[test]
fn qa_wgsl_buffers_read_only_through_sample_functions() {
    check_read_only_access(&fragment());
}

negative_control!(
    qa_wgsl_buffers_read_only_through_sample_functions,
    "an accessor indexing word_buffer itself must fail",
    expected = "`fgw_at` uses `word_buffer`, which only `sample_read` may read",
    check_read_only_access(&format!(
        "{}\nfn fgw_at(i: u32) -> u32 {{ return fgw_length_raw(word_buffer[i]); }}\n",
        fragment()
    ))
);

#[test]
fn qa_wgsl_no_setter() {
    check_read_only_access(&fragment());
}

negative_control!(
    qa_wgsl_no_setter,
    "an insertBits setter must fail",
    expected = "calls insertBits",
    check_read_only_access(&format!(
        "{}\nfn sd_put_last_symbol(pa: u32, s: u32) -> u32 {{ return insertBits(pa, s, 8u, 2u); }}\n",
        fragment()
    ))
);

#[test]
fn qa_wgsl_sample_functions_index_by_their_argument() {
    check_read_only_access(&fragment());
}

negative_control!(
    qa_wgsl_sample_functions_index_by_their_argument,
    "sample_read reading the word at a constant index, not its argument, must fail",
    expected = "`sample_read` does not index `word_buffer` by its argument",
    {
        let src = fragment();
        assert!(
            src.contains("word_buffer[i]"),
            "the control's pattern is gone"
        );
        check_read_only_access(&src.replace("word_buffer[i]", "word_buffer[0u + 0u * i]"))
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-RENDER-001 / R-271: unset by bits, never isinf / isnan / a float comparison standing in for them.

fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_float(m: &Module, info: &naga::valid::FunctionInfo, e: naga::Handle<Expression>) -> bool {
    matches!(
        info[e].ty.inner_with(&m.types),
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
}

/// A float constant of magnitude ≥ f16's largest finite (65504) or non-finite: the stand-in for an infinity test.
fn big_float(f: &naga::Function, e: naga::Handle<Expression>) -> bool {
    match f.expressions[e] {
        Expression::Literal(Literal::F32(v)) => !v.is_finite() || v.abs() >= 65504.0,
        Expression::Literal(Literal::AbstractFloat(v)) => !v.is_finite() || v.abs() >= 65504.0,
        _ => false,
    }
}

fn check_no_float_unset_tests(src: &str) {
    let code = strip_line_comments(src).to_lowercase();
    for word in ["isinf", "isnan"] {
        assert!(!code.contains(word), "the generated WGSL calls {word}");
    }
    let m = parse(src);
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&m)
    .expect("the generated WGSL validates");
    for (h, f) in m.functions.iter() {
        let fi = &info[h];
        let name = f.name.clone().unwrap_or_default();
        for (_, e) in f.expressions.iter() {
            match *e {
                Expression::Relational { fun, .. } => assert!(
                    !matches!(
                        fun,
                        naga::RelationalFunction::IsNan | naga::RelationalFunction::IsInf
                    ),
                    "`{name}` tests {fun:?}"
                ),
                Expression::Binary { op, left, right } => {
                    let cmp = matches!(
                        op,
                        BinaryOperator::Equal
                            | BinaryOperator::NotEqual
                            | BinaryOperator::Less
                            | BinaryOperator::LessEqual
                            | BinaryOperator::Greater
                            | BinaryOperator::GreaterEqual
                    );
                    if !cmp {
                        continue;
                    }
                    assert!(left != right, "`{name}` compares a value with itself");
                    if is_float(&m, fi, left) || is_float(&m, fi, right) {
                        assert!(
                            !big_float(f, left) && !big_float(f, right),
                            "`{name}` compares a float with an infinity stand-in"
                        );
                        // a float compared with itself through two loads of the same argument
                        let same_arg = matches!(
                            (&f.expressions[left], &f.expressions[right]),
                            (Expression::FunctionArgument(a), Expression::FunctionArgument(b)) if a == b
                        );
                        assert!(!same_arg, "`{name}` compares a value with itself");
                    }
                }
                _ => {}
            }
        }
    }
    // PA_D_MIN_UNSET is f16 +inf: sign 0, exponent all ones (5 bits), mantissa 0 (IEEE 754 binary16).
    let inf16 = 0x1f << 10;
    assert_eq!(
        wgsl_u32_const(&m, "PA_D_MIN_UNSET"),
        inf16,
        "PA_D_MIN_UNSET is not f16 +inf's bits"
    );
    assert!(
        m.functions
            .iter()
            .any(|(_, f)| f.name.as_deref() == Some("pa_d_min_is_unset")),
        "no `pa_d_min_is_unset`"
    );
}

#[test]
fn qa_wgsl_unset_is_tested_by_bits() {
    check_no_float_unset_tests(&wgsl());
}

negative_control!(
    qa_wgsl_unset_is_tested_by_bits,
    "a self-comparison NaN test must fail",
    expected = "compares a value with itself",
    check_no_float_unset_tests(&format!(
        "{}\nfn pa_d_min_is_nan(w: u32) -> bool {{ let d = pa_d_min(w); return d != d; }}\n",
        wgsl()
    ))
);

#[test]
fn qa_wgsl_no_float_stand_in_for_isinf() {
    check_no_float_unset_tests(&wgsl());
}

negative_control!(
    qa_wgsl_no_float_stand_in_for_isinf,
    "an unset test by d_min > 65504.0 must fail",
    expected = "compares a float with an infinity stand-in",
    check_no_float_unset_tests(&wgsl().replace(
        "fn pa_d_min_is_unset(w: u32) -> bool { return extractBits(w, 16u, 16u) == PA_D_MIN_UNSET; }",
        "fn pa_d_min_is_unset(w: u32) -> bool { return pa_d_min(w) > 65504.0; }"
    ))
);

#[test]
fn qa_wgsl_pa_d_min_unset_is_f16_inf() {
    check_no_float_unset_tests(&wgsl());
}

negative_control!(
    qa_wgsl_pa_d_min_unset_is_f16_inf,
    "PA_D_MIN_UNSET as f16 -inf's bits must fail",
    expected = "PA_D_MIN_UNSET is not f16 +inf's bits",
    check_no_float_unset_tests(&wgsl().replace(
        "PA_D_MIN_UNSET: u32 = 0x7c00u",
        "PA_D_MIN_UNSET: u32 = 0xfc00u"
    ))
);
