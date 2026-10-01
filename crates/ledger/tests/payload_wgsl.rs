//! The WGSL emitter (dd_generation_root §1; render contract Part 5, "Unpack layer"; payload §3, §6):
//! - REQ-PAY-016: the generated Rust and WGSL continuation tables equal payload §3's frozen arrays (`continuation_index`
//!   with 3 in its four inverse cells, R-307); each digit map is an involution; no continuation is `inverse(prev)`; and
//!   each WGSL table function, run on the GPU, reads the cell at `input & 3` for a symbol ≥ 4 and the digit-2 cell for a
//!   digit ≥ 3, as the Rust release build does (R-321, R-324).
//! - REQ-PAY-091: the generated WGSL struct layouts for `SimState` and `ICDescriptor` match the ledger's field by field,
//!   and the checked-in WGSL is the emitter's output.

use std::path::Path;

use ledger::gen::{self, rust, wgsl};
use ledger::layout;
use ledger::schema::{Storage, Struct};
use naga::{ArraySize, Expression, Literal, Module, Scalar, TypeInner};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// Payload §3's frozen arrays, as the payload doc writes them.
const INVERSE: [u32; 4] = [1, 0, 3, 2];
const CONT_SYMBOL: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
const CONTINUATION_INDEX: [[u32; 4]; 4] = [[0, 3, 1, 2], [3, 0, 2, 1], [1, 2, 0, 3], [2, 1, 3, 0]];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn checked_in(path: &str) -> String {
    let path = root().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn generated_wgsl() -> String {
    checked_in(wgsl::PATH)
}

fn parse(source: &str) -> Module {
    naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)))
}

/// The value of an array or scalar constant expression, flattened in order.
fn flatten(module: &Module, h: naga::Handle<Expression>, out: &mut Vec<u32>) {
    match &module.global_expressions[h] {
        Expression::Literal(Literal::U32(v)) => out.push(*v),
        Expression::Compose { components, .. } => {
            for &c in components {
                flatten(module, c, out);
            }
        }
        Expression::Constant(c) => flatten(module, module.constants[*c].init, out),
        other => panic!("not a u32 constant expression: {other:?}"),
    }
}

/// The WGSL constant `name`, flattened.
fn wgsl_table(module: &Module, name: &str) -> Vec<u32> {
    let (_, c) = module
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("the WGSL declares no `{name}`"));
    let mut out = Vec::new();
    flatten(module, c.init, &mut out);
    out
}

/// The Rust `pub const name: … = …;`, its numbers flattened in order.
fn rust_table(source: &str, name: &str) -> Vec<u32> {
    let head = format!("pub const {name}:");
    let start = source
        .find(&head)
        .unwrap_or_else(|| panic!("the Rust declares no `{name}`"));
    let rest = &source[start..];
    let value = &rest[rest.find('=').expect("an initializer") + 1..];
    let value = &value[..value.find(';').expect("an end")];
    value
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().expect("a number"))
        .collect()
}

/// The four tables, flattened: `inverse`, `cont_symbol`, `predecessor_symbol`, `continuation_index`.
type Tables = [Vec<u32>; 4];

fn wgsl_tables(source: &str) -> Tables {
    let m = parse(source);
    [
        "INVERSE",
        "CONT_SYMBOL",
        "PREDECESSOR_SYMBOL",
        "CONTINUATION_INDEX",
    ]
    .map(|n| wgsl_table(&m, n))
}

fn rust_tables(source: &str) -> Tables {
    [
        "INVERSE",
        "CONT_SYMBOL",
        "PREDECESSOR_SYMBOL",
        "CONTINUATION_INDEX",
    ]
    .map(|n| rust_table(source, n))
}

/// `tables` (from `target`) are payload §3's frozen arrays; `predecessor_symbol` equals `cont_symbol`, each map being
/// an involution.
fn check_frozen(target: &str, tables: &Tables) {
    let cont: Vec<u32> = CONT_SYMBOL.concat();
    let index: Vec<u32> = CONTINUATION_INDEX.concat();
    assert_eq!(
        tables[0], INVERSE,
        "{target} inverse differs from payload §3"
    );
    assert_eq!(
        tables[1], cont,
        "{target} cont_symbol differs from payload §3"
    );
    assert_eq!(
        tables[2], cont,
        "{target} predecessor_symbol differs from payload §3"
    );
    assert_eq!(
        tables[3], index,
        "{target} continuation_index differs from payload §3"
    );
}

#[test]
fn continuation_table_generated_rust_and_wgsl_equal_the_frozen_arrays() {
    check_frozen("Rust", &rust_tables(&checked_in(rust::PATH)));
    check_frozen("WGSL", &wgsl_tables(&generated_wgsl()));
}

negative_control!(
    continuation_table_generated_rust_and_wgsl_equal_the_frozen_arrays,
    "WGSL with 0, not R-307's 3, in continuation_index's first inverse cell must fail the frozen check",
    expected = "WGSL continuation_index differs from payload §3",
    check_frozen(
        "WGSL",
        &wgsl_tables(
            &generated_wgsl().replace("array<u32, 4>(0u, 3u, 1u, 2u)", "array<u32, 4>(0u, 0u, 1u, 2u)")
        )
    )
);

/// Each digit map of `tables` is an involution, so `predecessor_symbol` is `cont_symbol` (payload §3).
fn check_involutions(tables: &Tables) {
    for e in 0..3 {
        let map = &tables[1][4 * e..4 * e + 4];
        for prev in 0..4 {
            let next = map[prev] as usize;
            assert_eq!(
                map[next] as usize, prev,
                "digit {e}'s map is not an involution at {prev}"
            );
        }
    }
}

#[test]
fn continuation_table_digit_maps_are_involutions() {
    check_involutions(&rust_tables(&checked_in(rust::PATH)));
    check_involutions(&wgsl_tables(&generated_wgsl()));
}

negative_control!(
    continuation_table_digit_maps_are_involutions,
    "a digit-1 map that is a 4-cycle is no involution and must fail",
    expected = "is not an involution",
    check_involutions(&wgsl_tables(&generated_wgsl().replace(
        "array<u32, 4>(2u, 3u, 0u, 1u)",
        "array<u32, 4>(2u, 3u, 1u, 0u)"
    )))
);

/// No digit continues `prev` with `inverse(prev)` in `tables` (payload §3).
fn check_never_inverse(tables: &Tables) {
    for e in 0..3 {
        for prev in 0..4 {
            assert_ne!(
                tables[1][4 * e + prev],
                tables[0][prev],
                "digit {e} continues {prev} with its inverse"
            );
        }
    }
}

#[test]
fn continuation_table_no_continuation_is_the_inverse() {
    check_never_inverse(&rust_tables(&checked_in(rust::PATH)));
    check_never_inverse(&wgsl_tables(&generated_wgsl()));
}

negative_control!(
    continuation_table_no_continuation_is_the_inverse,
    "an inverse table equal to digit 0's map makes every digit-0 continuation the inverse and must fail",
    expected = "digit 0 continues 0 with its inverse",
    check_never_inverse(&wgsl_tables(
        &generated_wgsl().replace("array<u32, 4>(1u, 0u, 3u, 2u)", "array<u32, 4>(0u, 1u, 2u, 3u)")
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// R-321, R-324: the WGSL table functions are total, run on the GPU.

/// The test kernels: each table function of input `a` (and `b`), into `out`. The generated layer's own bindings are
/// moved to group 1, which these kernels do not use, so the harness's group 0 is theirs alone.
const KERNELS: &str = r"
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read> b: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;

@compute @workgroup_size(64)
fn t_inverse(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&a) { out[id.x] = inverse(a[id.x]) + 0u * b[id.x]; }
}

@compute @workgroup_size(64)
fn t_continuation_symbol(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&a) { out[id.x] = continuation_symbol(a[id.x], b[id.x]); }
}

@compute @workgroup_size(64)
fn t_predecessor_symbol(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&a) { out[id.x] = predecessor_symbol(a[id.x], b[id.x]); }
}

@compute @workgroup_size(64)
fn t_continuation_index(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&a) { out[id.x] = continuation_index(a[id.x], b[id.x]); }
}
";

/// Each WGSL table function of `generated`, over every pair of inputs 0…15 and a few far past the codes, returns the
/// Rust release build's value: the cell at each symbol `& 3` and at each digit `min(d, 2)`.
fn check_total(generated: &str) {
    let module = format!("{}\n{KERNELS}", generated.replace("@group(0)", "@group(1)"));
    let values: Vec<u32> = (0..16)
        .chain([255, 1 << 16, u32::MAX - 1, u32::MAX])
        .collect();
    let (a, b): (Vec<u32>, Vec<u32>) = values
        .iter()
        .flat_map(|&x| values.iter().map(move |&y| (x, y)))
        .unzip();
    let gpu = GpuHarness::new().expect("a GPU device");
    let cont = |e: u32, s: u32| CONT_SYMBOL[e.min(2) as usize][(s & 3) as usize];
    type Expect = fn(u32, u32, &dyn Fn(u32, u32) -> u32) -> u32;
    let cases: [(&str, Expect); 4] = [
        ("inverse", |x, _, _| INVERSE[(x & 3) as usize]),
        ("continuation_symbol", |x, y, cont| cont(y, x)),
        ("predecessor_symbol", |x, y, cont| cont(y, x)),
        ("continuation_index", |x, y, _| {
            CONTINUATION_INDEX[(x & 3) as usize][(y & 3) as usize]
        }),
    ];
    for (name, expect) in cases {
        let got = gpu.run_wgsl(&module, &format!("t_{name}"), &[&a, &b]);
        for i in 0..a.len() {
            let want = expect(a[i], b[i], &cont);
            assert_eq!(
                got[i], want,
                "WGSL {name}({}, {}) is {}, not {want}",
                a[i], b[i], got[i]
            );
        }
    }
}

#[test]
fn continuation_table_wgsl_out_of_range() {
    check_total(&generated_wgsl());
}

negative_control!(
    continuation_table_wgsl_out_of_range,
    "an inverse that clamps its symbol to 3, not masks it, reads the wrong cell at 4 and must fail",
    expected = "WGSL inverse(4, 0) is 2, not 1",
    check_total(&generated_wgsl().replace("INVERSE[s & 3u]", "INVERSE[min(s, 3u)]"))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-091: the WGSL struct layouts match the ledger's field by field.

/// One expected WGSL member: its name, byte offset, size and type, from the ledger's struct.
#[derive(Debug, PartialEq)]
struct Field {
    name: String,
    offset: u32,
    size: u32,
    ty: String,
}

/// `s`'s members as the ledger lays them out ([`rust::offsets`]): each at its offset, a u16 pair (WGSL has no u16) as
/// one u32 named `<first>_<second>`, the second's leading `_` dropped (RQ-188, option 2a).
fn ledger_fields(s: &Struct) -> Vec<Field> {
    let (offsets, _) = rust::offsets(s);
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.members.len() {
        let m = &s.members[i];
        let pair = m.storage == Storage::U16
            && s.members
                .get(i + 1)
                .is_some_and(|n| n.storage == Storage::U16);
        if pair {
            let n = &s.members[i + 1];
            out.push(Field {
                name: format!("{}_{}", m.name, n.name.trim_start_matches('_')),
                offset: offsets[i],
                size: 4,
                ty: "u32".to_owned(),
            });
            i += 2;
            continue;
        }
        let ty = match m.storage {
            Storage::F32 => "f32".to_owned(),
            Storage::U32 => "u32".to_owned(),
            Storage::Vec2x3 => "array<vec2<f32>, 3>".to_owned(),
            Storage::U32x4 => "vec4<u32>".to_owned(),
            Storage::Pad(n) => format!("array<u32, {n}>"),
            Storage::U16 => panic!("`{}.{}`: a u16 with no u16 beside it", s.name, m.name),
        };
        out.push(Field {
            name: m.name.to_owned(),
            offset: offsets[i],
            size: m.storage.size(),
            ty,
        });
        i += 1;
    }
    out
}

/// `ty` in WGSL syntax, for the types the layouts use.
fn type_name(module: &Module, ty: naga::Handle<naga::Type>) -> String {
    let scalar = |s: Scalar| match s {
        Scalar::F32 => "f32".to_owned(),
        Scalar::U32 => "u32".to_owned(),
        other => format!("{other:?}"),
    };
    match module.types[ty].inner {
        TypeInner::Scalar(s) => scalar(s),
        TypeInner::Vector { size, scalar: s } => format!("vec{}<{}>", size as u8, scalar(s)),
        TypeInner::Array { base, size, .. } => match size {
            ArraySize::Constant(n) => format!("array<{}, {n}>", type_name(module, base)),
            _ => format!("array<{}>", type_name(module, base)),
        },
        TypeInner::Struct { .. } => module.types[ty].name.clone().unwrap_or_default(),
        ref other => format!("{other:?}"),
    }
}

/// Every payload struct but the word buffer's element has a WGSL struct in `source` whose members, offsets, sizes,
/// types, size and alignment are the ledger's; the word buffer is bound as `array<vec4<u32>>`, one element of the
/// ledger's word size, and the `SimState` buffer as the indexed variant.
fn check_layouts(source: &str) {
    let module = parse(source);
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .expect("the layouts compute");
    for s in ledger::payload::structs() {
        if s.buffer == Some("word") {
            let (_, size) = rust::offsets(&s);
            let (_, g) = module
                .global_variables
                .iter()
                .find(|(_, g)| g.name.as_deref() == Some("word_buffer"))
                .expect("the WGSL binds word_buffer");
            assert_eq!(
                type_name(&module, g.ty),
                "array<vec4<u32>>",
                "word_buffer's type"
            );
            let TypeInner::Array { base, .. } = module.types[g.ty].inner else {
                unreachable!()
            };
            assert_eq!(
                layouter[base].size, size,
                "word_buffer's element is not `{}`'s {size} B",
                s.name
            );
            continue;
        }
        let (ty, wgsl) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some(s.name))
            .unwrap_or_else(|| panic!("the WGSL declares no `{}`", s.name));
        let TypeInner::Struct { members, span } = &wgsl.inner else {
            panic!("`{}` is not a struct", s.name)
        };
        let got: Vec<Field> = members
            .iter()
            .map(|m| Field {
                name: m.name.clone().unwrap_or_default(),
                offset: m.offset,
                size: layouter[m.ty].size,
                ty: type_name(&module, m.ty),
            })
            .collect();
        let want = ledger_fields(&s);
        assert_eq!(
            got.len(),
            want.len(),
            "`{}` has {} members, not {}",
            s.name,
            got.len(),
            want.len()
        );
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g, w, "`{}.{}` differs from the ledger", s.name, w.name);
        }
        let (_, size) = rust::offsets(&s);
        assert_eq!(
            *span, size,
            "`{}` is {span} B, not the ledger's {size}",
            s.name
        );
        let align = layouter[ty].alignment.round_up(1);
        assert_eq!(
            align, s.align,
            "`{}` is aligned to {align}, not the ledger's {}",
            s.name, s.align
        );
    }
    let (_, state) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some("simstate_buffer"))
        .expect("the WGSL binds simstate_buffer");
    let bound = type_name(&module, state.ty);
    assert!(
        bound.starts_with("array<SimState") && !bound.contains(','),
        "simstate_buffer is {bound}, not a runtime array of a SimState layout"
    );
}

#[test]
fn wgsl_layouts_match_the_ledger_field_by_field() {
    check_layouts(&generated_wgsl());
}

negative_control!(
    wgsl_layouts_match_the_ledger_field_by_field,
    "SimStateFTLE with `S` and `theta` exchanged is not the ledger's layout and must fail",
    expected = "`SimStateFTLE.S` differs from the ledger",
    check_layouts(&generated_wgsl().replacen(
        "    S: f32,\n    theta: f32,",
        "    theta: f32,\n    S: f32,",
        1
    ))
);

/// `on_disk` is the WGSL the emitter writes from the payload ledger.
fn check_checked_in(on_disk: &str) {
    let files = gen::generate(&layout(), &[wgsl::emit]).expect("the payload ledger generates");
    let emitted = files
        .iter()
        .find(|f| f.path.ends_with(wgsl::PATH))
        .expect("the WGSL emitter writes payload_unpack.wgsl");
    assert!(
        emitted.contents == on_disk,
        "the checked-in payload_unpack.wgsl is not the emitter's output"
    );
}

#[test]
fn wgsl_layouts_checked_in_file_is_the_emitted_one() {
    check_checked_in(&generated_wgsl());
}

negative_control!(
    wgsl_layouts_checked_in_file_is_the_emitted_one,
    "a checked-in file with one accessor edited is not the emitter's output",
    expected = "is not the emitter's output",
    check_checked_in(&generated_wgsl().replace("extractBits(w, 0u, 3u)", "extractBits(w, 0u, 4u)"))
);

/// The WGSL `PAYLOAD_SCHEMA_VERSION` is the Rust one's two halves, `.x` low.
fn check_version(wgsl: &str, rust: &str) {
    let halves = wgsl_table(&parse(wgsl), "PAYLOAD_SCHEMA_VERSION");
    let line = rust
        .lines()
        .find(|l| l.starts_with("pub const PAYLOAD_SCHEMA_VERSION: u64 = "))
        .expect("the Rust declares the version");
    let hex = line.rsplit("0x").next().expect("hex").trim_end_matches(';');
    let v = u64::from_str_radix(hex, 16).expect("a hex version");
    assert_eq!(
        halves,
        [(v & 0xffff_ffff) as u32, (v >> 32) as u32],
        "the WGSL schema version is not the Rust one's halves"
    );
}

#[test]
fn wgsl_layouts_schema_version_is_the_rust_one() {
    check_version(&generated_wgsl(), &checked_in(rust::PATH));
}

negative_control!(
    wgsl_layouts_schema_version_is_the_rust_one,
    "the halves exchanged are not the Rust version",
    expected = "is not the Rust one's halves",
    {
        let wgsl = generated_wgsl();
        let start = wgsl.find("vec2<u32>(0x").expect("the version") + "vec2<u32>(".len();
        let end = start + wgsl[start..].find(')').expect("its end");
        let (lo, hi) = wgsl[start..end].split_once(", ").expect("two halves");
        let swapped = format!("{}{hi}, {lo}{}", &wgsl[..start], &wgsl[end..]);
        check_version(&swapped, &checked_in(rust::PATH));
    }
);
