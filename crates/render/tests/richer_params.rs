//! A node's richer parameters (`render::assemble::params`, grammar version 2): a gradient, a list, a lookup table, a
//! field and an image; `pair`, `soft`, `unit` and `format` on a control; `@port` on an input.
//!
//! - Each type parses with its default, its value survives the bytes the uniform block holds, and the block's layout is
//!   the one naga computes for the same module (`types_parse_*`, `values_round_trip_*`, `block_layout_*`).
//! - Each malformed declaration is refused at its line, and a version 1 source cannot use the new words (`*_refused_*`).
//! - Today's field ramps, `lerp` and `diverging`, read as gradients to the same `f32` bits as `ramp.wgsl` computes them
//!   (`ramps_as_gradients_*`); the same on a device is `gradient_draw_*`, which runs where there is an adapter.
//! - What can matter is proved for the new types as for the old, and a gradient or a lut nothing reads changes no bit of
//!   the block (`proof_*`, `a_dead_gradient_*`).
//! - A field parameter is a different code, and so a different key; a `@port` edit is not (`field_*`, `port_*`).
//!
//! Each property has a control arm that breaks it.

#[cfg(feature = "controls")]
use render::assemble::declare::{probe as declare_probe, Check};
use render::assemble::matters;
#[cfg(feature = "controls")]
use render::assemble::matters::Forget;
use render::assemble::params;
use render::assemble::{
    self, AssembleError, Control, Declaration, Fragment, Kind, Node, Occupant, Stain, Tier,
    Uniform, UniformType,
};
use render::colour::field_ramp::Ramp;
use render::pipeline_cache::{block_layout, pack};
use validation::negative_control;

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// A stain showing a colour node of `text`, over the zero source.
fn showing(text: &str) -> Vec<Node> {
    vec![
        node(
            Kind::Source,
            Occupant::BuiltIn(render::registry::ZERO.to_owned()),
            &[],
        ),
        node(Kind::Colour, Occupant::Custom(text.to_owned()), &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ]
}

fn fragment(text: &str) -> Fragment {
    let stain = Stain::new(showing(text)).unwrap_or_else(|e| panic!("{e}"));
    assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"))
}

fn key(text: &str) -> u64 {
    Stain::new(showing(text))
        .unwrap_or_else(|e| panic!("{e}"))
        .canonical()
        .fragment_key()
}

fn declaration(text: &str) -> Declaration {
    Declaration::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

// ── The types ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A colour node declaring each new type, reading each.
const EVERY_TYPE: &str = "// @version 2
// @uniform ramp: gradient<8> = oklab 0:(0.25, 0.0, 0.0) 1:(0.875, 0.0, 0.0)
// @control ramp gradient \"Ramp\"
// @uniform sites: list<vec3<f32>, 4> = [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0)]
// @control sites list \"Sites\"
// @uniform curve: lut<8> = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0]
// @control curve lut \"Curve\"
// @uniform source: field = d_min
// @control source field \"Field\"
// @uniform picture: image = none
// @control picture image \"Picture\"
fn colour(ctx: Ctx) -> vec3<f32> {
    let t = lut8_at(uniforms.curve, uniforms.source);
    let a = gradient8_at(uniforms.ramp, t);
    return a + uniforms.sites.items[0u].xyz * f32(uniforms.sites.count) * f32(uniforms.picture);
}
";

fn check_types_parse(source: &str) {
    // Pass criterion: each type is the declared one, with the default the line writes: the gradient `[space, cyclic,
    // (x, y, z, position) per stop]`, the list its items flat, the lut its eight numbers, the field the index of
    // `d_min` among the source fields, the image 0.
    let d = declaration(source);
    let ty: Vec<UniformType> = d.uniforms.iter().map(|u| u.ty).collect();
    assert_eq!(
        ty,
        [
            UniformType::Gradient(8),
            UniformType::List { width: 3, max: 4 },
            UniformType::Lut(8),
            UniformType::Field,
            UniformType::Image
        ],
        "the types parsed"
    );
    assert_eq!(
        d.uniforms[0].default,
        [1.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.875, 0.0, 0.0, 1.0],
        "the gradient's default"
    );
    assert_eq!(
        d.uniforms[1].default,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        "the list's default"
    );
    assert_eq!(d.uniforms[2].default.len(), 8, "the lut's default");
    let fields = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    let at = fields.iter().position(|f| f == "d_min").expect("d_min");
    assert_eq!(d.uniforms[3].default, [at as f64], "the field's default");
    assert_eq!(d.uniforms[4].default, [0.0], "the image's default");
    let controls: Vec<&Control> = d.described.controls.iter().map(|c| &c.control).collect();
    assert_eq!(
        controls,
        [
            &Control::Gradient(Vec::new()),
            &Control::List,
            &Control::Curve,
            &Control::FieldPicker,
            &Control::ImagePicker
        ],
        "the controls"
    );
}

#[test]
fn types_parse_with_their_defaults() {
    check_types_parse(EVERY_TYPE);
}

negative_control!(
    types_parse_with_their_defaults,
    "a gradient written in linear RGB is not the OKLab default the check expects",
    expected = "the gradient's default",
    check_types_parse(&EVERY_TYPE.replace("= oklab 0:", "= linear 0:"))
);

/// Values of each type that `f32` holds exactly.
fn values() -> Vec<(UniformType, Vec<f64>)> {
    vec![
        (
            UniformType::Gradient(8),
            vec![
                1.0, 1.0, 0.25, 0.0, -0.5, 0.0, 0.5, 0.125, 0.25, 0.5, 0.75, 0.0, 0.5, 1.0,
            ],
        ),
        (
            UniformType::List { width: 3, max: 4 },
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        ),
        (
            UniformType::Lut(8),
            vec![0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0],
        ),
    ]
}

fn check_round_trip() {
    // Pass criterion: the value encoded is as long as WGSL lays the struct out, from its own arithmetic (a 16 byte
    // header and `max` 16-byte elements; a lut's `N / 4` of them), and decodes to the value.
    let sizes = [144, 80, 32];
    for ((ty, v), size) in values().into_iter().zip(sizes) {
        let bytes = params::encode(ty, &v).expect("encoded");
        assert_eq!(
            bytes.len(),
            size,
            "the bytes of {ty:?} are the size of its struct"
        );
        assert_eq!(params::decode(ty, &bytes).expect("decoded"), v, "{ty:?}");
    }
}

#[test]
fn values_round_trip_through_the_block() {
    check_round_trip();
}

negative_control!(
    values_round_trip_through_the_block,
    "with a gradient's array counted at stride 12, its bytes are not the struct's size",
    expected = "are the size of its struct",
    params::probe::narrow_stride_in(check_round_trip)
);

/// `fragment`'s uniform block of `node`, as naga lays it out: each member's offset, and the struct's span.
fn naga_layout(source: &str, node: usize) -> (Vec<u64>, u64) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{e:?}"));
    let name = format!("n{node}_uniforms");
    let (_, g) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some(name.as_str()))
        .unwrap_or_else(|| panic!("no {name}"));
    match &module.types[g.ty].inner {
        naga::TypeInner::Struct { members, span } => (
            members.iter().map(|m| u64::from(m.offset)).collect(),
            u64::from(*span),
        ),
        other => panic!("{name} is {other:?}"),
    }
}

fn check_layout() {
    // Pass criterion: the packer's offsets and size equal naga's for the module the assembler wrote.
    let f = fragment(EVERY_TYPE);
    let block = f.uniforms.first().expect("the node has a block");
    let (offsets, size) = block_layout(&block.uniforms);
    let (naga_offsets, naga_size) = naga_layout(&f.source, block.node);
    assert_eq!(offsets, naga_offsets, "the block's offsets");
    assert_eq!(size, naga_size, "the block's size");
}

#[test]
fn block_layout_is_the_one_naga_computes() {
    check_layout();
}

negative_control!(
    block_layout_is_the_one_naga_computes,
    "a packer that strides a gradient's array by 12 lays the block out differently from naga",
    expected = "the block's",
    params::probe::narrow_stride_in(check_layout)
);

// ── Refusals ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn refused_line(source: &str) -> Option<usize> {
    match Declaration::parse(source) {
        Ok(_) => None,
        Err(AssembleError::Declaration { line, .. }) => Some(line),
        Err(other) => panic!("refused, but not as a declaration: {other}"),
    }
}

fn check_refusals(cases: &[(String, usize)]) {
    for (source, line) in cases {
        assert_eq!(
            refused_line(source),
            Some(*line),
            "the source is not refused at line {line}:\n{source}"
        );
    }
}

fn v2(rest: &str) -> String {
    format!("// @version 2\n{rest}")
}

fn param_cases() -> Vec<(String, usize)> {
    let u = |decl: &str| v2(&format!("// @uniform x: {decl}\n"));
    let mut cases = vec![
        // Version 1 cannot use a version 2 type.
        (
            "// @uniform x: gradient<4> = linear 0:(0,0,0) 1:(1,1,1)\n".to_owned(),
            1,
        ),
        (u("gradient<1> = linear 0:(0,0,0) 1:(1,1,1)"), 2),
        (u("gradient<17> = linear 0:(0,0,0) 1:(1,1,1)"), 2),
        (u("gradient<4> = linear 0:(0,0,0)"), 2),
        (u("gradient<4> = linear 0.5:(0,0,0) 0.25:(1,1,1)"), 2),
        (u("gradient<4> = linear 0:(0,0,0) 1.5:(1,1,1)"), 2),
        (u("gradient<4> = oklab 0:(2,0,0) 1:(1,0,0)"), 2),
        (u("gradient<2> = linear 0:(0,0,0) 0.5:(1,1,1) 1:(0,0,0)"), 2),
        (u("gradient<4> = linear 0:(0,0) 1:(1,1,1)"), 2),
        (u("gradient<4> = wide 0:(0,0,0) 1:(1,1,1)"), 2),
        (u("list<vec5<f32>, 4> = []"), 2),
        (u("list<f32, 0> = []"), 2),
        (u("list<f32, 33> = []"), 2),
        (u("list<f32, 2> = [1.0, 2.0, 3.0]"), 2),
        (u("list<vec2<f32>, 4> = [(1.0, 2.0, 3.0)]"), 2),
        (u("list<vec2<f32>, 4> = [1.0, 2.0]"), 2),
        (u("lut<1> = [0.0]"), 2),
        (u("lut<257> = [0.0]"), 2),
        (u("lut<4> = [0.0, 1.0]"), 2),
        (u("field = no_such_field"), 2),
        (u("image = 5"), 2),
        (u("gradient<4> = linear 0:(0,0,0) 1:(1,1,1) [0, 1]"), 2),
    ];
    // A node's block is at most 16 KiB: seventeen 1 KiB tables are 17 KiB, and the seventeenth is the line.
    let big: String = (0..17)
        .map(|k| {
            format!(
                "// @uniform t{k}: lut<256> = [{}]\n",
                vec!["0.0"; 256].join(", ")
            )
        })
        .collect();
    cases.push((v2(&big), 18));
    cases
}

#[test]
fn types_with_a_bad_size_or_default_are_refused_at_their_line() {
    check_refusals(&param_cases());
}

negative_control!(
    types_with_a_bad_size_or_default_are_refused_at_their_line,
    "with the parameter check lifted, a bad gradient, list, lut, field or image is accepted",
    expected = "is not refused",
    declare_probe::without(Check::Param, || check_refusals(&param_cases()))
);

fn fit_cases() -> Vec<(String, usize)> {
    let head = "// @version 2\n// @uniform a: f32 = 0.25 [0, 1]\n// @uniform b: f32 = 0.75 [0, 1]\n\
                // @uniform t: u32 = 0 [0, 1]\n// @uniform g: gradient<4> = linear 0:(0,0,0) 1:(1,1,1)\n";
    let with = |tail: &str| format!("{head}{tail}");
    vec![
        (with("// @control a gradient \"A\"\n"), 6),
        (with("// @control g slider \"G\"\n"), 6),
        (with("// @control a list \"A\"\n"), 6),
        (with("// @control a lut \"A\"\n"), 6),
        (with("// @control a field \"A\"\n"), 6),
        (with("// @control a image \"A\"\n"), 6),
        (with("// @control a slider \"A\" soft [0.5, 2]\n"), 6),
        (with("// @control a slider \"A\" soft [-1, 0.5]\n"), 6),
        (with("// @control t toggle \"T\" unit \"deg\"\n"), 6),
        (with("// @control g gradient \"G\" soft [0, 1]\n"), 6),
        (with("// @control a slider \"A\" pair a\n"), 6),
        (with("// @control a slider \"A\" pair nowhere\n"), 6),
        (with("// @control a slider \"A\" pair b\n"), 6),
        (
            with("// @control a slider \"A\" pair b\n// @control b slider \"B\" pair a\n"),
            6,
        ),
        // The low end's default is above the high end's.
        (
            v2("// @uniform a: f32 = 0.75 [0, 1]\n// @uniform b: f32 = 0.25 [0, 1]\n\
                // @control a slider \"A\" pair b\n// @control b slider \"B\"\n"),
            4,
        ),
        // Two lows name one high.
        (
            v2("// @uniform a: f32 = 0.0 [0, 1]\n// @uniform c: f32 = 0.0 [0, 1]\n// @uniform b: f32 = 1.0 [0, 1]\n\
                // @control a slider \"A\" pair b\n// @control c slider \"C\" pair b\n// @control b slider \"B\"\n"),
            5,
        ),
    ]
}

#[test]
fn controls_that_do_not_fit_are_refused_at_their_line() {
    check_refusals(&fit_cases());
}

negative_control!(
    controls_that_do_not_fit_are_refused_at_their_line,
    "with the fit check lifted, a control on the wrong type, a soft range outside the range or a bad pair is accepted",
    expected = "is not refused",
    declare_probe::without(Check::Fit, || check_refusals(&fit_cases()))
);

fn malformed_cases() -> Vec<(String, usize)> {
    let head = "// @version 2\n// @input field\n// @uniform a: f32 = 0.25 [0, 1]\n";
    let with = |tail: &str| format!("{head}{tail}");
    vec![
        (with("// @control a slider \"A\" format fixed(10)\n"), 4),
        (with("// @control a slider \"A\" format fixed\n"), 4),
        (with("// @control a slider \"A\" unit \"\"\n"), 4),
        (with("// @control a slider \"A\" soft [1, 0]\n"), 4),
        (with("// @control a slider \"A\" soft [0.1]\n"), 4),
        (
            with("// @control a slider \"A\" unit \"m\" unit \"s\"\n"),
            4,
        ),
        (with("// @control a slider \"A\" order 1 order 2\n"), 4),
        (with("// @port nowhere \"N\"\n"), 4),
        (with("// @port field\n"), 4),
        (with("// @port field \"F\" accepts sparkle\n"), 4),
        (with("// @port field \"F\" accepts scalar | scalar\n"), 4),
        (with("// @port field \"F\" required optional\n"), 4),
        (with("// @port field \"F\"\n// @port field \"G\"\n"), 5),
    ]
}

#[test]
fn malformed_relations_and_ports_are_refused_at_their_line() {
    check_refusals(&malformed_cases());
}

negative_control!(
    malformed_relations_and_ports_are_refused_at_their_line,
    "with the malformed-line check lifted, a bad format, soft range or port is accepted",
    expected = "is not refused",
    declare_probe::without(Check::Malformed, || check_refusals(&malformed_cases()))
);

fn version_cases() -> Vec<(String, usize)> {
    let head =
        "// @input field\n// @uniform a: f32 = 0.25 [0, 1]\n// @uniform b: f32 = 0.75 [0, 1]\n";
    let with = |tail: &str| format!("{head}{tail}");
    vec![
        (
            with("// @control a slider \"A\" pair b\n// @control b slider \"B\"\n"),
            4,
        ),
        (with("// @control a slider \"A\" soft [0, 0.5]\n"), 4),
        (with("// @control a slider \"A\" unit \"m\"\n"), 4),
        (with("// @control a slider \"A\" format percent\n"), 4),
        (with("// @port field \"F\"\n"), 4),
        (
            "// @uniform g: vec3<f32> = (0.0, 0.0, 0.0)\n// @control g gradient \"G\"\n".to_owned(),
            2,
        ),
        // Version 3 is a later grammar than this build's.
        ("// @version 3\n".to_owned(), 1),
    ]
}

#[test]
fn version_two_words_need_version_two() {
    check_refusals(&version_cases());
}

negative_control!(
    version_two_words_need_version_two,
    "with the version check lifted, a version 1 source may use version 2's words",
    expected = "is not refused",
    declare_probe::without(Check::Version, || check_refusals(&version_cases()))
);

/// A source using every relation, which parses and reads back; its low end's unit is `unit`.
fn check_relations(unit: &str) {
    // Pass criterion: each clause is on its control line, and the port says what it declares.
    let d = declaration(&format!(
        "// @version 2\n// @input field [0, 1]\n// @uniform lo: f32 = 0.25 [0, 10]\n\
         // @uniform hi: f32 = 4.0 [0, 10]\n\
         // @control lo slider \"Low\" in \"Range\" pair hi soft [0, 5] unit \"{unit}\" format fixed(2) order 1\n\
         // @control hi slider \"High\" unit \"m\" format scientific\n\
         // @port field \"Depth\" accepts scalar | position optional unit \"m\"\n"
    ));
    let lo = &d.described.controls[0];
    assert_eq!(lo.pair.as_deref(), Some("hi"));
    assert_eq!(lo.soft, Some((0.0, 5.0)));
    assert_eq!(lo.unit.as_deref(), Some("m"), "the low end's unit");
    assert_eq!(lo.format, Some(assemble::declare::Format::Fixed(2)));
    assert_eq!(lo.group.as_deref(), Some("Range"));
    assert_eq!(lo.order, Some(1));
    assert_eq!(
        d.described.controls[1].format,
        Some(assemble::declare::Format::Scientific)
    );
    let p = &d.described.ports[0];
    assert_eq!(p.label, "Depth");
    assert_eq!(
        p.accepts,
        [
            assemble::declare::Accepts::Scalar,
            assemble::declare::Accepts::Position
        ]
    );
    assert!(p.optional);
    assert_eq!(p.unit.as_deref(), Some("m"));
}

#[test]
fn relations_and_ports_read_back() {
    check_relations("m");
}

negative_control!(
    relations_and_ports_read_back,
    "a low end whose unit is seconds is not metres",
    expected = "the low end's unit",
    check_relations("s")
);

// ── Today's ramps as gradients ────────────────────────────────────────────────────────────────────────────────────

/// The two forms a backend may compute WGSL's `mix` in; the ramps and the gradient must agree under each.
type Mix = fn(f32, f32, f32) -> f32;

fn mixes() -> [(&'static str, Mix); 2] {
    [
        ("a (1 - u) + b u", |a, b, u| a * (1.0 - u) + b * u),
        ("a + (b - a) u", |a, b, u| a + (b - a) * u),
    ]
}

fn lerp_f32(c0: [f64; 3], c1: [f64; 3], t: f32, mix: Mix) -> [f32; 3] {
    let u = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| mix(c0[i] as f32, c1[i] as f32, u))
}

fn diverging_f32(lo: [f64; 3], mid: [f64; 3], hi: [f64; 3], t: f32, mix: Mix) -> [f32; 3] {
    let u = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| {
        if u < 0.5 {
            mix(lo[i] as f32, mid[i] as f32, 2.0 * u)
        } else {
            mix(mid[i] as f32, hi[i] as f32, 2.0 * u - 1.0)
        }
    })
}

fn grid() -> Vec<f32> {
    let mut ts: Vec<f32> = (0..=4096).map(|k| k as f32 / 4096.0).collect();
    ts.extend([
        -3.0,
        -0.0,
        -1e-9,
        0.25,
        0.5,
        f32::from_bits(0.5f32.to_bits() - 1),
        f32::from_bits(0.5f32.to_bits() + 1),
        f32::from_bits(1.0f32.to_bits() - 1),
        1.0,
        1.0 + 1e-6,
        7.0,
    ]);
    ts
}

fn check_ramps(shift: f64) {
    // Pass criterion, written before the first run: over the grid, and with `mix` computed in either form, the
    // gradient's `f32` result equals `ramp.wgsl`'s, every component, to the bit.
    let (c0, c1) = ([0.1, 0.4, 0.9], [0.8, 0.2, 0.3]);
    let (lo, mid, hi) = ([0.0, 0.2, 0.8], [0.9, 0.9, 0.9], [0.8, 0.1, 0.0]);
    let lerp = Ramp::Lerp { c0, c1 }.as_gradient().expect("a gradient");
    let mut diverging = Ramp::Diverging { lo, mid, hi }
        .as_gradient()
        .expect("a gradient");
    diverging[2 + 4 + 3] += shift;
    for (form, mix) in mixes() {
        for t in grid() {
            let g = params::gradient_at_with(&lerp, t, &mix).expect("a colour");
            let w = lerp_f32(c0, c1, t, mix);
            assert_eq!(
                g.map(f32::to_bits),
                w.map(f32::to_bits),
                "lerp at t = {t}, mix as {form}: gradient {g:?}, ramp {w:?}"
            );
            let g = params::gradient_at_with(&diverging, t, &mix).expect("a colour");
            let w = diverging_f32(lo, mid, hi, t, mix);
            assert_eq!(
                g.map(f32::to_bits),
                w.map(f32::to_bits),
                "diverging at t = {t}, mix as {form}: gradient {g:?}, ramp {w:?}"
            );
        }
    }
}

#[test]
fn ramps_as_gradients_read_to_the_same_bits() {
    check_ramps(0.0);
}

negative_control!(
    ramps_as_gradients_read_to_the_same_bits,
    "a diverging gradient whose middle stop sits at 0.5 + 1/64 reads differently from the ramp",
    expected = "diverging at t",
    check_ramps(1.0 / 64.0)
);

/// A custom-stop OKLab gradient reads, in `f32`, as the `f64` ramp reads, to `f32` rounding of the OKLab conversion.
fn check_oklab(shift: f64) {
    // Pass criterion: within 1e-5 of render's f64 OKLab conversion of the same mix, over the grid.
    let stops = [
        ([0.2 + shift, 0.05, -0.1], 0.0),
        ([0.3, 0.1, 0.05], 0.5),
        ([0.9, 0.0, -0.1], 1.0),
    ];
    let mut g = vec![1.0, 0.0];
    for (c, p) in stops {
        g.extend([c[0], c[1], c[2], p]);
    }
    for t in grid() {
        let got = params::gradient_at(&g, t).expect("a colour");
        let u = f64::from(t.clamp(0.0, 1.0));
        let lab = |a: [f64; 3], b: [f64; 3], u: f64| [0, 1, 2].map(|i| a[i] * (1.0 - u) + b[i] * u);
        let twin = [
            ([0.2, 0.05, -0.1], 0.0),
            ([0.3, 0.1, 0.05], 0.5),
            ([0.9, 0.0, -0.1], 1.0),
        ];
        let want = if u < 0.5 {
            lab(twin[0].0, twin[1].0, u / 0.5)
        } else {
            lab(twin[1].0, twin[2].0, (u - 0.5) / 0.5)
        };
        let want = render::colour::space::oklab_to_linear(want);
        for i in 0..3 {
            assert!(
                (f64::from(got[i]) - want[i]).abs() < 1e-5,
                "t = {t}: {got:?} vs {want:?}"
            );
        }
    }
}

#[test]
fn an_oklab_gradient_reads_as_its_f64_twin() {
    check_oklab(0.0);
}

negative_control!(
    an_oklab_gradient_reads_as_its_f64_twin,
    "a first stop 0.01 lighter than the twin's reads differently",
    expected = "vs",
    check_oklab(0.01)
);

// ── The proof ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Reads `ramp`; leaves `spare` (a gradient), `unused_list` and `unused_lut` alone.
const DEAD_PARTS: &str = "// @version 2
// @uniform gain: f32 = 0.5 [0, 1]
// @uniform ramp: gradient<8> = oklab 0:(0.25, 0.0, 0.0) 1:(0.875, 0.0, 0.0)
// @uniform spare: gradient<4> = linear 0:(0.0, 0.0, 0.0) 1:(1.0, 1.0, 1.0)
// @uniform unused_list: list<vec3<f32>, 4> = [(1.0, 0.0, 0.0)]
// @uniform unused_lut: lut<8> = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0]
fn colour(ctx: Ctx) -> vec3<f32> {
    let unused = gradient4_at(uniforms.spare, 0.5) + uniforms.unused_list.items[0u].xyz * 0.0;
    return gradient8_at(uniforms.ramp, ctx.inputs[0].x) * uniforms.gain;
}
";

/// A gradient read only inside a branch, a list only through a pointer write.
const REACHED: &str = "// @version 2
// @uniform mode: u32 = 0
// @uniform ramp: gradient<8> = oklab 0:(0.25, 0.0, 0.0) 1:(0.875, 0.0, 0.0)
// @uniform sites: list<f32, 4> = [1.0, 2.0]
// @uniform curve: lut<8> = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0]
// @uniform gate: gradient<4> = linear 0:(0.0, 0.0, 0.0) 1:(1.0, 1.0, 1.0)
fn put(p: ptr<function, f32>, v: f32) {
    *p = v;
}
fn colour(ctx: Ctx) -> vec3<f32> {
    var x = 0.0;
    put(&x, uniforms.sites.items[1u].x);
    if (uniforms.gate.cyclic != 0u) {
        return vec3<f32>(0.5);
    }
    if (uniforms.mode == 1u) {
        return gradient8_at(uniforms.ramp, x);
    }
    return vec3<f32>(lut8_at(uniforms.curve, x));
}
";

fn live(text: &str) -> Vec<bool> {
    matters::shader(Kind::Colour, text)
        .unwrap_or_else(|e| panic!("{e}"))
        .never()
        .uniforms
}

fn check_proof() {
    // Pass criteria, written before the first run: in DEAD_PARTS the gain and the ramp are live and `spare`,
    // `unused_list` and `unused_lut` are dead (each is read only into a value nothing reads, or not at all); in REACHED
    // every uniform is live: the gate only as a branch's condition, the mode, the gradient and the lut through a
    // branch, the list through a pointer write.
    assert_eq!(
        live(DEAD_PARTS),
        [true, true, false, false, false],
        "the proof on DEAD_PARTS"
    );
    assert_eq!(
        live(REACHED),
        [true, true, true, true, true],
        "the proof on REACHED"
    );
}

#[test]
fn proof_follows_the_new_types_through_branches() {
    check_proof();
}

negative_control!(
    proof_follows_the_new_types_through_branches,
    "a proof that forgets branches judges REACHED's gate, read only as a condition, dead",
    expected = "the proof on REACHED",
    matters::probe::forgetting(Forget::Branches, check_proof)
);

#[test]
fn proof_follows_the_new_types_through_pointers() {
    check_proof();
}

negative_control!(
    proof_follows_the_new_types_through_pointers,
    "a proof that forgets pointer writes judges REACHED's list dead",
    expected = "the proof on REACHED",
    matters::probe::forgetting(Forget::PointerWrites, check_proof)
);

fn dead_names(text: &str) -> Vec<String> {
    fragment(text)
        .dropped
        .uniforms
        .iter()
        .map(|(_, u)| u.name.clone())
        .collect()
}

/// The block the assembly leaves, packed with `value` for each uniform it still holds.
fn packed(text: &str, value: &dyn Fn(&Uniform) -> Vec<f64>) -> (Vec<String>, Vec<u8>) {
    let f = fragment(text);
    let block = f.uniforms.first().expect("a block");
    (
        block.uniforms.iter().map(|u| u.name.clone()).collect(),
        pack(&block.uniforms, value),
    )
}

fn check_dead_parts(read_it: bool) {
    // Pass criterion: `spare`, `unused_list` and `unused_lut` are dropped from the block, so its bytes are the same
    // whatever those hold, and still differ with the live `ramp`. In the control `spare` is read, so it stays.
    let text = if read_it {
        DEAD_PARTS.replace(
            "* uniforms.gain;",
            "* uniforms.gain + gradient4_at(uniforms.spare, 0.5);",
        )
    } else {
        DEAD_PARTS.to_owned()
    };
    let other = vec![
        0.0, 1.0, 0.1, 0.1, 0.1, 0.0, 0.9, 0.9, 0.9, 1.0, // another gradient
    ];
    let live_value = |u: &Uniform| u.default.clone();
    let with_dead_changed = |u: &Uniform| {
        if matches!(u.name.as_str(), "spare" | "unused_list" | "unused_lut") {
            match u.ty {
                UniformType::Gradient(_) => other.clone(),
                UniformType::List { .. } => vec![5.0, 5.0, 5.0],
                _ => vec![1.0; 8],
            }
        } else {
            u.default.clone()
        }
    };
    let (names, a) = packed(&text, &live_value);
    let (_, b) = packed(&text, &with_dead_changed);
    assert_eq!(
        names,
        ["gain", "ramp"],
        "the uniforms the block holds; dead: {:?}",
        dead_names(&text)
    );
    assert_eq!(
        a, b,
        "a dead gradient, list or lut changed a byte of the block"
    );
    let moved = |u: &Uniform| {
        if u.name == "ramp" {
            other.clone()
        } else {
            u.default.clone()
        }
    };
    let (_, c) = packed(&text, &moved);
    assert_ne!(a, c, "a live gradient changed no byte of the block");
}

#[test]
fn a_dead_gradient_list_or_lut_changes_no_byte_of_the_block() {
    check_dead_parts(false);
}

negative_control!(
    a_dead_gradient_list_or_lut_changes_no_byte_of_the_block,
    "with `spare` read, it stays in the block, and a change to it changes a byte",
    expected = "the uniforms the block holds",
    check_dead_parts(true)
);

// ── A field read in the code; a port is no code ───────────────────────────────────────────────────────────────────

const FIELD_NODE: &str = "// @version 2
// @uniform src: field = d_min
// @uniform gain: f32 = 0.5 [0, 1]
// @control gain slider \"Gain\"
// @control src field \"Field\"
fn colour(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(uniforms.src * uniforms.gain);
}
";

fn chosen(text: &str, field: &str) -> String {
    let d = declaration(text);
    let fields = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    let at = fields.iter().position(|f| f == field).expect("a field");
    let src: Vec<&Uniform> = d.uniforms.iter().filter(|u| u.name == "src").collect();
    params::specialise(text, &src, &|_| vec![at as f64]).unwrap_or_else(|e| panic!("{e}"))
}

fn check_field(a: &str, b: &str) {
    // Pass criterion: the field a stain picks is read in the code, so it is in the fragment's field set, and picking
    // another is another key; moving the other uniform's default changes no key.
    let (ta, tb) = (chosen(FIELD_NODE, a), chosen(FIELD_NODE, b));
    let (fa, fb) = (fragment(&ta), fragment(&tb));
    assert!(
        fa.fields.iter().any(|f| f == a),
        "the picked field is read: {:?}",
        fa.fields
    );
    assert_ne!(key(&ta), key(&tb), "a different field is a different code");
    assert!(
        fb.fields.iter().any(|f| f == b),
        "the second field is read: {:?}",
        fb.fields
    );
    let relabelled = FIELD_NODE.replace("\"Gain\"", "\"Strength\"");
    assert_eq!(
        key(&chosen(&relabelled, a)),
        key(&ta),
        "a control's label is no code"
    );
}

#[test]
fn field_parameter_is_read_in_the_code() {
    check_field("d_min", "ftle");
}

negative_control!(
    field_parameter_is_read_in_the_code,
    "two picks of one field are one code",
    expected = "a different field is a different code",
    check_field("d_min", "d_min")
);

/// The same node's key, with its port and its control lines edited.
fn port_key(label: &str, domain: &str) -> u64 {
    key(&format!(
        "// @version 2\n// @input field {domain}\n// @uniform a: f32 = 0.25 [0, 1]\n\
         // @control a slider \"{label}\" unit \"m\"\n\
         // @port field \"{label}\" accepts scalar optional unit \"m\"\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(ctx.inputs[0].x * uniforms.a); }}\n"
    ))
}

fn check_port(domain_b: &str) {
    // Pass criterion: the labels, units and accepted subtypes on declaration lines change no key; a domain, which the
    // code is compiled against, is the contrast.
    assert_eq!(
        port_key("One", "[0, 1]"),
        port_key("Two", domain_b),
        "the same key"
    );
}

#[test]
fn a_port_edit_changes_no_key() {
    check_port("[0, 1]");
}

negative_control!(
    a_port_edit_changes_no_key,
    "an `@input` domain is code: changing it changes the key",
    expected = "the same key",
    check_port("[0, 2]")
);

fn check_no_new_words(extra: Option<&str>) {
    // Pass criterion: no shipped source declares a version 2 type or a `@port`, so the pinned code is untouched; but
    // `source/field`, whose one uniform is a `field`, and the three field ramps, whose uniforms are a `field`, a
    // gradient and plain numbers.
    let mut sources: Vec<(String, String)> = render::registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|e| (e.id, e.source))
        .collect();
    sources.extend(extra.map(|t| ("planted".to_owned(), t.to_owned())));
    for (id, source) in sources {
        let d = Declaration::parse(&source).unwrap_or_else(|x| panic!("{id}: {x}"));
        if id == "source/field" {
            assert!(
                d.described.ports.is_empty()
                    && matches!(
                        d.uniforms.iter().map(|u| u.ty).collect::<Vec<_>>()[..],
                        [UniformType::Field]
                    ),
                "{id} uses a version 2 word other than its one field"
            );
            continue;
        }
        if id.ends_with("/field_ramp") {
            assert!(
                d.described.ports.is_empty()
                    && d.uniforms.iter().all(|u| !matches!(
                        u.ty,
                        UniformType::List { .. } | UniformType::Lut(_) | UniformType::Image
                    )),
                "{id} uses a version 2 word other than a field and a gradient"
            );
            continue;
        }
        assert!(
            d.described.ports.is_empty()
                && d.uniforms.iter().all(|u| !matches!(
                    u.ty,
                    UniformType::Gradient(_)
                        | UniformType::List { .. }
                        | UniformType::Lut(_)
                        | UniformType::Field
                        | UniformType::Image
                )),
            "{id} uses a version 2 word"
        );
    }
}

#[test]
fn no_shipped_source_uses_a_new_word_yet() {
    check_no_new_words(None);
}

negative_control!(
    no_shipped_source_uses_a_new_word_yet,
    "a source with a gradient uniform uses a version 2 word",
    expected = "uses a version 2 word",
    check_no_new_words(Some(
        "// @version 2\n// @uniform g: gradient<2> = linear 0:(0,0,0) 1:(1,1,1)\n"
    ))
);

// ── A field parameter before a slider ─────────────────────────────────────────────────────────────────────────────

/// A `field` declared first, a slider after it: the slider is the second declared uniform and the first member.
fn field_first(reads_gain: bool) -> String {
    format!(
        "// @version 2\n// @uniform src: field = d_min\n// @uniform gain: f32 = 0.5 [0, 1]\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(uniforms.src{}); }}\n",
        if reads_gain { " * uniforms.gain" } else { "" }
    )
}

fn check_field_first(reads_gain: bool) {
    // Pass criterion, written before the first run: the proof of a node with a field parameter is no error, and each
    // uniform is judged from its own member: both live where both are read, the field live and the slider dead where
    // only the field is.
    let want = [true, reads_gain];
    assert_eq!(
        live(&field_first(reads_gain)),
        want,
        "the proof on a field then a slider"
    );
    assert_eq!(
        live(&field_first(true)),
        [true, true],
        "the proof on a field then a slider, both read"
    );
}

#[test]
fn proof_follows_a_slider_after_a_field() {
    check_field_first(true);
    check_field_first(false);
}

negative_control!(
    proof_follows_a_slider_after_a_field,
    "a slider judged from the field's member before it is judged wrong",
    expected = "the proof on a field then a slider",
    // The member numbering a proof that counted the field would use: the slider read as the second member, which the
    // block does not have.
    assert_eq!(
        live(&field_first(false)),
        [true, true],
        "the proof on a field then a slider"
    )
);

// ── On a device: today's ramps drawn as gradients ─────────────────────────────────────────────────────────────────

mod device {
    use render::assemble::params;
    use render::assemble::{Tier, UniformType};
    use render::colour::field_ramp::RAMP_WGSL;
    use validation::gpu::{BindingKind, GpuHarness};
    use validation::negative_control;

    const ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> cases: array<vec4<u32>>;
const CASE_WIDTH: u32 = 64u;
// The colours are read at run time, as a node's uniforms are: constants let the compiler fold the ramp's `mix` and
// not the gradient's, and the two would differ by an ulp that no node can show.
@group(2) @binding(1) var<storage, read> colours: array<vec4<f32>>;

fn rgb(c: vec3<f32>) -> vec4<u32> { return vec4<u32>(bitcast<vec3<u32>>(c), 0u); }

fn lerp_gradient() -> Gradient2 {
    return Gradient2(2u, 0u, 0u, 0u, array<vec4<f32>, 2>(vec4<f32>(colours[0].xyz, 0.0), vec4<f32>(colours[1].xyz, 1.0)));
}
fn diverging_gradient() -> Gradient3 {
    return Gradient3(3u, 0u, 0u, 0u, array<vec4<f32>, 3>(
        vec4<f32>(colours[0].xyz, 0.0), vec4<f32>(colours[2].xyz, 0.5), vec4<f32>(colours[1].xyz, 1.0)));
}

@fragment
fn t_gradient(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let a = cases[u32(pos.y) * CASE_WIDTH + u32(pos.x)];
    let t = bitcast<f32>(a.y);
    switch a.x {
        case 1u: { return rgb(ramp_lerp(t, colours[0].xyz, colours[1].xyz)); }
        case 2u: { return rgb(gradient2_at(lerp_gradient(), t)); }
        case 3u: { return rgb(ramp_diverging(t, colours[0].xyz, colours[2].xyz, colours[1].xyz)); }
        case 4u: { return rgb(gradient3_at(diverging_gradient(), t)); }
        default: { return vec4<u32>(0xffffffffu); }
    }
}
";

    fn module(ramp: &str) -> String {
        let library = params::library(&[UniformType::Gradient(2), UniformType::Gradient(3)]);
        format!(
            "{}\n{ramp}\n{library}\n{ENTRY}",
            ledger::gen::prelude::wgsl(Tier::FULL)
        )
    }

    /// Where there is no device: the module the device test runs is valid WGSL.
    fn check_validates(ramp: &str) {
        let m = naga::front::wgsl::parse_str(&module(ramp))
            .unwrap_or_else(|e| panic!("the device module does not parse: {e:?}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&m)
        .unwrap_or_else(|e| panic!("the device module is invalid: {e:?}"));
    }

    #[test]
    fn the_device_module_validates() {
        check_validates(RAMP_WGSL);
    }

    negative_control!(
        the_device_module_validates,
        "a ramp shader that returns the wrong type does not validate",
        expected = "the device module is invalid",
        check_validates(&RAMP_WGSL.replace("return mix(c0, c1, clamp(t, 0.0, 1.0));", "return t;"))
    );

    /// The cases' results on the device, four words each.
    fn run(h: &GpuHarness, ramp: &str, cases: &[(u32, f32)]) -> Vec<[u32; 4]> {
        let module = module(ramp);
        let words: Vec<u32> = cases
            .iter()
            .flat_map(|&(op, t)| [op, t.to_bits(), 0, 0])
            .collect();
        let colours: Vec<u32> = [[0.1f32, 0.2, 0.3], [0.9, 0.5, 0.0], [0.25, 0.75, 1.0]]
            .iter()
            .flat_map(|c| [c[0].to_bits(), c[1].to_bits(), c[2].to_bits(), 0])
            .collect();
        let rows = cases.len().div_ceil(64);
        let kernel = h
            .fragment(
                &module,
                "t_gradient",
                &[&[], &[], &[BindingKind::Storage, BindingKind::Storage]],
                64,
                rows as u32,
            )
            .unwrap_or_else(|e| panic!("{e}"));
        let pixels = kernel
            .draw(&[&[], &[], &[&words, &colours]])
            .unwrap_or_else(|e| panic!("{e}"));
        pixels[..cases.len()].to_vec()
    }

    fn check(ramp: &str) {
        // Pass criterion, written before the first run: for every t, the gradient's three channels are the ramp
        // shader's, to the bit, for `lerp` and for `diverging`.
        let h = GpuHarness::new().expect("a GPU device");
        let ts: Vec<f32> = (0..=1024)
            .map(|k| k as f32 / 1024.0 * 1.5 - 0.25)
            .chain([0.0, 0.5, 1.0, 0.25, 0.75])
            .collect();
        for (name, a, b) in [("lerp", 1, 2), ("diverging", 3, 4)] {
            let cases: Vec<(u32, f32)> = ts.iter().flat_map(|&t| [(a, t), (b, t)]).collect();
            let got = run(&h, ramp, &cases);
            for (pair, &t) in got.chunks(2).zip(&ts) {
                assert_eq!(
                    pair[0][..3],
                    pair[1][..3],
                    "{name} at t = {t}: the gradient differs from the ramp"
                );
            }
        }
    }

    #[test]
    fn gradient_draw_matches_the_ramp_shader() {
        check(RAMP_WGSL);
    }

    negative_control!(
        gradient_draw_matches_the_ramp_shader,
        "a ramp shader whose diverging halves meet at ⅓ differs from the gradient",
        expected = "the gradient differs from the ramp",
        check(
            &RAMP_WGSL
                .replace("mix(mid, hi, 2.0 * u - 1.0)", "mix(mid, hi, 1.5 * u - 0.5)")
                .replace("mix(lo, mid, 2.0 * u)", "mix(lo, mid, 3.0 * u)")
                .replace("u < 0.5", "u < 0.33333334")
        )
    );
}

// ── A field and a whole read of the uniforms ──────────────────────────────────────────────────────────────────────

fn check_whole_read_refused(with_field: bool) {
    // Pass criterion: a node that declares a field and reads its uniforms other than as `uniforms.<name>` is refused
    // where it is declared; the same read without a field is not.
    let field = if with_field {
        "// @uniform src: field = d_min\n"
    } else {
        ""
    };
    let text = format!(
        "// @version 2\n{field}// @uniform gain: f32 = 0.5 [0, 1]\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ let u = uniforms; return vec3<f32>(u.gain); }}\n"
    );
    let refused = assemble::declaration(Kind::Colour, &Occupant::Custom(text)).is_err();
    assert!(refused, "the whole read was not refused");
}

#[test]
fn a_field_with_a_whole_read_of_the_uniforms_is_refused_at_its_declaration() {
    check_whole_read_refused(true);
}

negative_control!(
    a_field_with_a_whole_read_of_the_uniforms_is_refused_at_its_declaration,
    "without a field the whole read is allowed, so nothing is refused",
    expected = "the whole read was not refused",
    check_whole_read_refused(false)
);
