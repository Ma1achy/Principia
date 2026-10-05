//! QA's tests for TASK-M1-04, written from the requirements the task closes, not from the implementation:
//! - REQ-RENDER-009 (render contract Part 2; colour_composition §4, §4.1, §4.2; R-64): no wiring that reorders the
//!   backbone, feeds post back into colour or closes a cycle is accepted; and the assembled `shade()`, drawn on the GPU,
//!   computes sources → colour / brightness → combiner → (post)* → OUT, known answers checked (`qa_backbone_*`);
//! - REQ-RENDER-010: a built-in occupant and a custom occupant of the same text assemble to one source through the one
//!   entry point (`qa_one_path_*`);
//! - REQ-RENDER-012 (lowering Part 3a): a custom occupant reading `sample.ftle`, `sample.ensemble_spread` and
//!   `sample.word` by plain member access compiles and runs at every tier variant, reading the sentinels the contract
//!   names where a feature is absent (`qa_custom_reads_any_tier_*`);
//! - R-378: the field set is what the stain reads; a stain missing a field it reads is refused; an assembled stain
//!   reading `ftle` loads no stored member `ftle` does not need (`qa_assemble_field_set_*`);
//! - REQ-RENDER-016: colour returns `vec3<f32>`, brightness `f32`; any other signature is refused (`qa_slot_*`);
//! - REQ-RENDER-075 (lowering Part 5): permuted constructions of one graph hash equal, different graphs differ, and the
//!   key is the 64-bit FNV-1a of the form's text (`qa_canonical_hash_*`);
//! - REQ-GEN-027 (gui_state_contract §3): the `// @uniform` / `// @input` declaration format, its refusals, and a
//!   declared uniform read on the GPU as `uniforms.<name>` (`qa_declaration_*`).
//!
//! Every test has its negative control (R-176).

use ledger::gen::{prelude, read, rust};
use render::assemble::{
    self, AssembleError, Declaration, Kind, Node, Occupant, Stain, Tier, UniformType,
};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

fn n(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

fn pass_through() -> Occupant {
    Occupant::BuiltIn("pass_through".into())
}

fn src_const(v: f32) -> Occupant {
    custom(&format!(
        "fn source(ctx: Ctx) -> Field {{ return Field({v:?}, 0.0, 0.0, 0.0); }}"
    ))
}

const COLOUR_CONST: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.2, 0.4, 0.6); }";
const COLOUR_SHOW: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }";
const BRIGHT_HALF: &str = "fn brightness(ctx: Ctx) -> f32 { return 0.5; }";
const BRIGHT_SHOW: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";
const COMBINE_MUL: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }";
const POST_ADD: &str =
    "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb + vec3<f32>(0.1); }";
const POST_DOUBLE: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 2.0; }";

fn stain(nodes: Vec<Node>) -> Stain {
    Stain::new(nodes).unwrap_or_else(|e| panic!("a valid stain refused: {e}"))
}

/// The full backbone: s0 → colour, s1 → brightness, multiply combiner, then `posts` in order, then OUT.
fn backbone(colour: &str, brightness: &str, posts: &[&str]) -> Vec<Node> {
    let mut g = vec![
        n(Kind::Source, src_const(0.3), &[]),
        n(Kind::Source, src_const(0.7), &[]),
        n(Kind::Colour, custom(colour), &[Some(0)]),
        n(Kind::Brightness, custom(brightness), &[Some(1)]),
        n(Kind::Combiner, custom(COMBINE_MUL), &[Some(2), Some(3)]),
    ];
    let mut prev = 4;
    for p in posts {
        g.push(n(Kind::Post, custom(p), &[Some(prev)]));
        prev = g.len() - 1;
    }
    g.push(n(Kind::Out, Occupant::None, &[Some(prev)]));
    g
}

/// Our own entry: shades sample 0 with ensemble spread 0.25, unit masses, dt 0.01, δ₀ 1e-6, n_renorm 16, horizon 1000.
const ENTRY: &str = r"
@fragment
fn qa_entry(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let rgb = shade_sample(0u, pos.xy, 0.25, vec3<f32>(1.0, 1.0, 1.0), ReadParams(0.01, 1e-6, 16u, 1000u));
    return vec4<u32>(bitcast<vec3<u32>>(rgb), 0u);
}
";

/// The stored struct `tier` binds: (member name, byte offset), and its size in bytes.
fn stored_layout(tier: Tier) -> (Vec<(&'static str, u32)>, u32) {
    let name = if tier.has_ftle {
        "SimStateFTLE"
    } else {
        "SimStateBase"
    };
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .expect("the stored struct");
    let (offsets, size) = rust::offsets(&s);
    (
        s.members.iter().map(|m| m.name).zip(offsets).collect(),
        size,
    )
}

/// One stored sample: `S` = 2, `t_end_step` = 100, shadow offset δ = 1e-6 = δ₀ when the shadow is stored, else zero.
/// Payload §5: ftle = (S + ln(δ/δ₀)) / (n·dt) = 2 / (100 · 0.01) = 2.
fn stored_sample(tier: Tier) -> Vec<u32> {
    let (layout, size) = stored_layout(tier);
    let mut w = vec![0u32; size as usize / 4];
    let at = |m: &str| layout.iter().find(|x| x.0 == m).expect(m).1 as usize / 4;
    w[at("S")] = 2.0f32.to_bits();
    w[at("times")] = 100;
    if tier.has_ftle {
        w[at("r_sh")] = 1e-6f32.to_bits();
    }
    w
}

/// Draws `source` (an assembled stain plus [`ENTRY`]) at `tier` for `e` copies with `extra` uniform blocks in group 0.
fn draw(
    h: &GpuHarness,
    source: &str,
    tier: Tier,
    e: u32,
    extra: &[Vec<u32>],
    word: [u32; 4],
) -> [u32; 4] {
    let mut g0_kinds = vec![BindingKind::Uniform];
    g0_kinds.extend(extra.iter().map(|_| BindingKind::Uniform));
    let g1_kinds: Vec<BindingKind> = if tier.has_word {
        vec![BindingKind::Storage, BindingKind::Storage]
    } else {
        vec![BindingKind::Storage]
    };
    let kernel = h
        .fragment(source, "qa_entry", &[&g0_kinds, &g1_kinds], 1, 1)
        .unwrap_or_else(|err| panic!("{tier:?}: the assembled stain does not build: {err}"));
    let uniforms = prelude::uniform_words(e);
    let mut g0: Vec<&[u32]> = vec![&uniforms];
    g0.extend(extra.iter().map(Vec::as_slice));
    let state = stored_sample(tier);
    let g1: Vec<&[u32]> = if tier.has_word {
        vec![&state, &word]
    } else {
        vec![&state]
    };
    kernel
        .draw(&[&g0, &g1])
        .unwrap_or_else(|err| panic!("{tier:?}: the draw failed: {err}"))[0]
}

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

fn rgb(px: [u32; 4]) -> [f32; 3] {
    [
        f32::from_bits(px[0]),
        f32::from_bits(px[1]),
        f32::from_bits(px[2]),
    ]
}

/// f32 rounding of a handful of operations on values of order 1: a few ulps, bounded by 1e-6.
const F32_OPS: f32 = 1e-6;

fn assert_rgb(got: [f32; 3], want: [f32; 3], what: &str) {
    for k in 0..3 {
        assert!(
            (got[k] - want[k]).abs() <= F32_OPS,
            "{what}: shade() gives {got:?}, the backbone gives {want:?}"
        );
    }
}

fn shade_rgb(h: &GpuHarness, g: Vec<Node>) -> [f32; 3] {
    let f = assemble::assemble(&stain(g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    rgb(draw(h, &(f.source + ENTRY), Tier::FULL, 0, &[], [0; 4]))
}

// ── REQ-RENDER-009: the backbone ───────────────────────────────────────────────────────────────────────────────────

/// Graphs that break the backbone, each with what it tries.
fn backbone_breakers() -> Vec<(&'static str, Vec<Node>)> {
    let src = || n(Kind::Source, src_const(0.5), &[]);
    let col = |i: usize| n(Kind::Colour, custom(COLOUR_SHOW), &[Some(i)]);
    let bri = |i: usize| n(Kind::Brightness, custom(BRIGHT_SHOW), &[Some(i)]);
    let comb = |c: Option<usize>, b: Option<usize>| n(Kind::Combiner, pass_through(), &[c, b]);
    let post = |i: Option<usize>| n(Kind::Post, custom(POST_ADD), &[i]);
    let out = |i: usize| n(Kind::Out, Occupant::None, &[Some(i)]);
    vec![
        (
            "a post fed back into a colour's input",
            vec![src(), comb(None, None), post(Some(1)), col(2), out(2)],
        ),
        (
            "the combiner reading a post that reads the combiner",
            vec![
                comb(Some(1), None),
                n(Kind::Post, custom(POST_ADD), &[Some(0)]),
                out(1),
            ],
        ),
        (
            "a post into the combiner's colour, the post before it",
            vec![post(None), comb(Some(0), None), out(1)],
        ),
        (
            "a post fed into a brightness",
            vec![
                src(),
                col(0),
                comb(Some(1), None),
                post(Some(2)),
                bri(3),
                out(3),
            ],
        ),
        (
            "a colour straight to OUT, past the combiner",
            vec![src(), col(0), comb(Some(1), None), out(1)],
        ),
        (
            "a colour into a post, past the combiner",
            vec![src(), col(0), comb(None, None), post(Some(1)), out(3)],
        ),
        (
            "a brightness into the combiner's colour",
            vec![src(), bri(0), comb(Some(1), None), out(2)],
        ),
        (
            "a colour into the combiner's brightness",
            vec![src(), col(0), comb(None, Some(1)), out(2)],
        ),
        (
            "a source straight into the combiner",
            vec![src(), comb(Some(0), None), out(1)],
        ),
        (
            "a source straight into OUT",
            vec![src(), comb(None, None), out(0)],
        ),
        (
            "the combiner into a colour",
            vec![comb(None, None), col(0), out(0)],
        ),
        (
            "OUT read by a post",
            vec![comb(None, None), out(0), post(Some(1))],
        ),
        (
            "a post reading itself",
            vec![
                comb(None, None),
                n(Kind::Post, custom(POST_ADD), &[Some(1)]),
                out(1),
            ],
        ),
        (
            "two posts in a cycle",
            vec![
                comb(None, None),
                n(Kind::Post, custom(POST_ADD), &[Some(2)]),
                n(Kind::Post, custom(POST_ADD), &[Some(1)]),
                out(2),
            ],
        ),
        (
            "a second combiner",
            vec![comb(None, None), comb(None, None), out(1)],
        ),
        ("no combiner", vec![post(None), out(0)]),
        ("a second OUT", vec![comb(None, None), out(0), out(0)]),
        ("no OUT", vec![comb(None, None), post(Some(0))]),
        (
            "nine live posts",
            backbone(COLOUR_CONST, BRIGHT_HALF, &[POST_ADD; 9]),
        ),
    ]
}

fn check_all_refused(cases: Vec<(&'static str, Vec<Node>)>) {
    for (what, g) in cases {
        assert!(
            Stain::new(g).is_err(),
            "{what}: accepted, but the backbone forbids it"
        );
    }
}

#[test]
fn qa_backbone_no_wiring_reorders_it_or_feeds_post_back() {
    check_all_refused(backbone_breakers());
    // The legal shapes nearest them are accepted, so the refusals are of the breach, not of everything.
    for g in [
        backbone(COLOUR_CONST, BRIGHT_HALF, &[]),
        backbone(COLOUR_CONST, BRIGHT_HALF, &[POST_ADD; 8]),
    ] {
        stain(g);
    }
}

negative_control!(
    qa_backbone_no_wiring_reorders_it_or_feeds_post_back,
    "the legal backbone, offered as a breaker, is accepted",
    expected = "accepted, but the backbone forbids it",
    check_all_refused(vec![(
        "the legal backbone",
        backbone(COLOUR_CONST, BRIGHT_HALF, &[POST_ADD])
    )])
);

/// colour (0.2, 0.4, 0.6) × brightness 0.5 = (0.1, 0.2, 0.3); + 0.1 = (0.2, 0.3, 0.4); × 2 = (0.4, 0.6, 0.8).
fn check_backbone_order(posts: &[&str]) {
    let got = shade_rgb(&gpu(), backbone(COLOUR_CONST, BRIGHT_HALF, posts));
    assert_rgb(got, [0.4, 0.6, 0.8], "colour × brightness, + 0.1, × 2");
}

#[test]
fn qa_backbone_shade_runs_in_backbone_order() {
    check_backbone_order(&[POST_ADD, POST_DOUBLE]);
}

negative_control!(
    qa_backbone_shade_runs_in_backbone_order,
    "the posts wired in the other order give (0.3, 0.5, 0.7), not the chain's answer",
    expected = "the backbone gives",
    check_backbone_order(&[POST_DOUBLE, POST_ADD])
);

/// The identity table (colour_composition §4.1, Multiply column; render_gui_spec §13), on the GPU.
fn check_identities(mid_grey: f32) {
    let h = gpu();
    let mut colour_none = backbone(COLOUR_CONST, BRIGHT_HALF, &[]);
    colour_none[2].occupant = Occupant::None;
    assert_rgb(
        shade_rgb(&h, colour_none),
        [0.5; 3],
        "colour None: white · B",
    );
    let mut bright_none = backbone(COLOUR_CONST, BRIGHT_HALF, &[]);
    bright_none[3].occupant = Occupant::None;
    assert_rgb(
        shade_rgb(&h, bright_none),
        [0.2, 0.4, 0.6],
        "brightness None: C · 1",
    );
    let mut both = backbone(COLOUR_CONST, BRIGHT_HALF, &[]);
    both[2].occupant = Occupant::None;
    both[3].occupant = Occupant::None;
    // OKLab (0.6, 0, 0) in linear RGB is 0.6³ per channel: Ottosson's inverse rows each sum to 1.
    assert_rgb(
        shade_rgb(&h, both),
        [mid_grey; 3],
        "both None: mid-grey OKLab(0.6,0,0)",
    );
    let mut post_none = backbone(COLOUR_CONST, BRIGHT_HALF, &[POST_ADD, POST_DOUBLE]);
    post_none[5].occupant = Occupant::None;
    assert_rgb(
        shade_rgb(&h, post_none),
        [0.2, 0.4, 0.6],
        "a None post passes its colour on",
    );
    // A dangling field input is the identity (render_gui_spec §13): the colour unwired reads as colour None.
    let mut dangling = backbone(COLOUR_CONST, BRIGHT_HALF, &[]);
    dangling[2].inputs = vec![None];
    assert_rgb(
        shade_rgb(&h, dangling),
        [0.5; 3],
        "a dangling colour input: white · B",
    );
}

#[test]
fn qa_backbone_none_is_the_identity() {
    check_identities(0.216);
}

negative_control!(
    qa_backbone_none_is_the_identity,
    "a mid-grey of OKLab L 0.5 instead of 0.6 is not the doc's grey",
    expected = "both None",
    check_identities(0.125)
);

/// Field inputs arrive by port: the colour's two inputs show which source feeds which port.
fn check_ports(first: usize, second: usize) {
    const TWO: &str = "// @input a\n// @input b\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x, ctx.inputs[1].x, 0.0); }";
    let g = vec![
        n(Kind::Source, src_const(0.25), &[]),
        n(Kind::Source, src_const(0.75), &[]),
        n(Kind::Colour, custom(TWO), &[Some(first), Some(second)]),
        n(Kind::Brightness, Occupant::None, &[None]),
        n(Kind::Combiner, pass_through(), &[Some(2), Some(3)]),
        n(Kind::Out, Occupant::None, &[Some(4)]),
    ];
    assert_rgb(
        shade_rgb(&gpu(), g),
        [0.75, 0.25, 0.0],
        "port 0 from the 0.75 source, port 1 from the 0.25 source",
    );
}

#[test]
fn qa_backbone_field_inputs_arrive_by_port() {
    check_ports(1, 0);
}

negative_control!(
    qa_backbone_field_inputs_arrive_by_port,
    "the sources wired the other way round",
    expected = "the backbone gives",
    check_ports(0, 1)
);

// ── REQ-RENDER-010: one compile path ───────────────────────────────────────────────────────────────────────────────

fn one_path(text: &str) -> (String, String) {
    let with = |combiner: Occupant| {
        let g = vec![
            n(Kind::Source, src_const(0.5), &[]),
            n(Kind::Colour, custom(COLOUR_SHOW), &[Some(0)]),
            n(Kind::Combiner, combiner, &[Some(1), None]),
            n(Kind::Out, Occupant::None, &[Some(2)]),
        ];
        assemble::assemble(&stain(g), Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"))
            .source
    };
    (with(pass_through()), with(custom(text)))
}

/// `source` with its `//` comments removed: a node's label comment names its occupant, which is not compiled.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|l| l.split("//").next().unwrap_or("").trim_end())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_one_path(text: &str) {
    let (builtin, user) = one_path(text);
    let (builtin, user) = (code(&builtin), code(&user));
    assert!(
        builtin == user,
        "the built-in and a custom of its text assemble differently: a second path"
    );
}

#[test]
fn qa_one_path_a_builtin_is_a_custom_of_its_text() {
    let text =
        assemble::builtin(Kind::Combiner, "pass_through").expect("the built-in pass-through");
    check_one_path(text);
    // A built-in id that is no file is refused, never silently substituted.
    let g = vec![
        n(
            Kind::Combiner,
            Occupant::BuiltIn("no_such_combiner".into()),
            &[None, None],
        ),
        n(Kind::Out, Occupant::None, &[Some(0)]),
    ];
    assert!(Stain::new(g).is_err(), "an unknown built-in is accepted");
}

negative_control!(
    qa_one_path_a_builtin_is_a_custom_of_its_text,
    "a custom multiply is not the pass-through",
    expected = "a second path",
    check_one_path(COMBINE_MUL)
);

// ── REQ-RENDER-012: any field at any tier ───────────────────────────────────────────────────────────────────────────

/// Plain member access to the three fields of REQ-RENDER-012's verify.
const READS_THREE: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    let w = ctx.sample.word;
    return vec3<f32>(ctx.sample.ftle, ctx.sample.ensemble_spread, bitcast<f32>(w.x ^ w.w));
}
";

/// The canonical quiet NaN (lowering Part 3a).
const QNAN: u32 = 0x7FC0_0000;
/// The unbound word (lowering Part 3a, `FGW_UNBOUND`).
const UNBOUND: [u32; 4] = [0, 0, 0, 0xFE00_0000];
/// A bound word of our own.
const BOUND: [u32; 4] = [3, 0, 0, 0x0400_0000];

fn check_reads_any_tier(colour: &str) {
    let h = gpu();
    let g = vec![
        n(Kind::Source, src_const(0.0), &[]),
        n(Kind::Colour, custom(colour), &[Some(0)]),
        n(Kind::Combiner, pass_through(), &[Some(1), None]),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ];
    for tier in Tier::ALL {
        let f = assemble::assemble(&stain(g.clone()), tier)
            .unwrap_or_else(|e| panic!("{tier:?}: does not assemble: {e}"));
        for e in [0u32, 1] {
            let px = draw(&h, &(f.source.clone() + ENTRY), tier, e, &[], BOUND);
            if tier.has_ftle {
                let ftle = f32::from_bits(px[0]);
                assert!(
                    (ftle - 2.0).abs() <= 1e-5,
                    "{tier:?}, E = {e}: ftle reads {ftle}, payload §5 gives 2"
                );
            } else {
                assert_eq!(
                    px[0], QNAN,
                    "{tier:?}: a tier-absent ftle is not the canonical NaN"
                );
            }
            let spread = if e >= 1 { 0.25f32.to_bits() } else { QNAN };
            assert_eq!(
                px[1], spread,
                "{tier:?}, E = {e}: ensemble_spread reads {:#x}",
                px[1]
            );
            let word = if tier.has_word { BOUND } else { UNBOUND };
            assert_eq!(
                px[2],
                word[0] ^ word[3],
                "{tier:?}: the word reads {:#x}",
                px[2]
            );
        }
    }
}

#[test]
fn qa_custom_reads_any_tier_by_member_access() {
    check_reads_any_tier(READS_THREE);
}

negative_control!(
    qa_custom_reads_any_tier_by_member_access,
    "a colour that ignores the word reads the same at every tier",
    expected = "the word reads",
    check_reads_any_tier(
        &READS_THREE.replace("bitcast<f32>(w.x ^ w.w)", "bitcast<f32>(0x04000003u)")
    )
);

// ── R-378: the field set ───────────────────────────────────────────────────────────────────────────────────────────

fn reader(source: Occupant) -> Vec<Node> {
    vec![
        n(Kind::Source, source, &[]),
        n(Kind::Colour, custom(COLOUR_SHOW), &[Some(0)]),
        n(Kind::Combiner, pass_through(), &[Some(1), None]),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ]
}

fn check_field_set(source: Occupant, want: &[&str]) {
    for tier in Tier::ALL {
        let got = assemble::field_set(&stain(reader(source.clone())), tier)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            got, want,
            "{tier:?}: the field set is not what the stain reads"
        );
    }
}

#[test]
fn qa_assemble_field_set_is_what_the_stain_reads() {
    check_field_set(Occupant::Field("ftle".into()), &["ftle"]);
    check_field_set(
        custom(
            "fn source(ctx: Ctx) -> Field { return Field(f32(ctx.sample.word.w), 0.0, 0.0, 0.0); }",
        ),
        &["word.w"],
    );
    check_field_set(src_const(0.5), &[]);
}

negative_control!(
    qa_assemble_field_set_is_what_the_stain_reads,
    "an ftle reader's set is not empty",
    expected = "the field set is not what the stain reads",
    check_field_set(Occupant::Field("ftle".into()), &[])
);

fn check_unfilled(fields: &[&str]) {
    let s = stain(reader(Occupant::Field("ftle".into())));
    match assemble::assemble_reading(&s, Tier::FULL, fields) {
        Err(AssembleError::UnfilledField(f)) => assert_eq!(f, "ftle"),
        Err(e) => panic!("refused, but not for the unfilled field: {e}"),
        Ok(_) => panic!("assembled a stain whose read side leaves `ftle` unfilled"),
    }
}

#[test]
fn qa_assemble_field_set_missing_a_read_field_is_refused() {
    check_unfilled(&[]);
    check_unfilled(&["ensemble_spread", "word"]);
}

negative_control!(
    qa_assemble_field_set_missing_a_read_field_is_refused,
    "a set holding ftle is assembled",
    expected = "assembled a stain whose read side leaves",
    check_unfilled(&["ftle"])
);

/// The stored members naga's MSL loads from `simstate_buffer[…].<member>`, `(whole)` for a whole-struct load, and
/// whether it touches `word_buffer` at all.
fn msl_members(source: &str) -> (Vec<String>, bool) {
    use naga::valid::{Capabilities, ValidationFlags, Validator};
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{e}"));
    let info = Validator::new(
        ValidationFlags::all(),
        Capabilities::default() | Capabilities::SHADER_FLOAT16_IN_FLOAT32,
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{e:?}"));
    // MSL 3.2, the version the compute entry point targets (`engine::compute`).
    let options = naga::back::msl::Options {
        lang_version: (3, 2),
        ..Default::default()
    };
    let msl = naga::back::msl::write_string(&module, &info, &options, &Default::default())
        .unwrap_or_else(|e| panic!("MSL: {e}"))
        .0;
    let mut out = Vec::new();
    let key = "simstate_buffer[";
    let mut from = 0;
    while let Some(at) = msl[from..].find(key) {
        let mut i = from + at + key.len();
        let mut depth = 1;
        let bytes = msl.as_bytes();
        while depth > 0 {
            match bytes[i] {
                b'[' => depth += 1,
                b']' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        let rest = &msl[i..];
        let member = rest.strip_prefix('.').map_or("(whole)".to_owned(), |r| {
            r.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        });
        out.push(member);
        from = i;
    }
    out.sort();
    out.dedup();
    (out, msl.contains("word_buffer["))
}

/// Stored members `ftle` does not need (payload §5: ftle reads S, the shadow, the state, t_end_step and the
/// descriptor; never these).
const NOT_FOR_FTLE: [&str; 5] = ["theta", "mean_y", "C_ty", "E_0", "Lz_0"];

fn check_ftle_loads(fields: Option<&[&str]>) {
    let s = stain(reader(Occupant::Field("ftle".into())));
    let f = match fields {
        None => assemble::assemble(&s, Tier::FULL),
        Some(fields) => assemble::assemble_reading(&s, Tier::FULL, fields),
    }
    .unwrap_or_else(|e| panic!("{e}"));
    let (members, word) = msl_members(&(f.source + ENTRY));
    assert!(
        !members.is_empty(),
        "the ftle reader loads nothing: the check cannot see loads"
    );
    assert!(
        !members.contains(&"(whole)".to_owned()),
        "the whole stored struct is loaded at once"
    );
    for m in NOT_FOR_FTLE {
        assert!(
            !members.iter().any(|x| x.starts_with(m)),
            "an ftle-only stain loads `{m}`, which ftle never needs: {members:?}"
        );
    }
    assert!(
        !word,
        "an ftle reader loads the word, which ftle never needs"
    );
}

#[test]
fn qa_assemble_field_set_an_ftle_stain_loads_only_ftles_members() {
    assert!(
        every_field().contains(&"ftle".to_owned()),
        "ftle is a read-side field"
    );
    check_ftle_loads(None);
}

negative_control!(
    qa_assemble_field_set_an_ftle_stain_loads_only_ftles_members,
    "the same stain with every field filled loads every stored member",
    expected = "which ftle never needs",
    check_ftle_loads(Some(
        &every_field().iter().map(String::as_str).collect::<Vec<_>>()
    ))
);

/// Every read-side field, the full fill.
fn every_field() -> Vec<String> {
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).expect("the ledger validates");
    read::fields(&l.words, &entries)
}

// ── REQ-RENDER-016: slot signatures ────────────────────────────────────────────────────────────────────────────────

fn slot_stain(kind: Kind, text: &str) -> Result<Stain, AssembleError> {
    let mut g = vec![n(Kind::Source, src_const(0.5), &[])];
    match kind {
        Kind::Colour => {
            g.push(n(Kind::Colour, custom(text), &[Some(0)]));
            g.push(n(Kind::Combiner, pass_through(), &[Some(1), None]));
        }
        Kind::Brightness => {
            g.push(n(Kind::Brightness, custom(text), &[Some(0)]));
            g.push(n(Kind::Combiner, custom(COMBINE_MUL), &[None, Some(1)]));
        }
        _ => unreachable!(),
    }
    g.push(n(Kind::Out, Occupant::None, &[Some(2)]));
    let s = Stain::new(g)?;
    assemble::assemble(&s, Tier::FULL)?;
    Ok(s)
}

fn check_signatures(bad: &[(Kind, &str)]) {
    for &(kind, text) in bad {
        assert!(
            slot_stain(kind, text).is_err(),
            "a {kind:?} occupant `{text}` is accepted: the slot contract is colour → vec3 linear RGB, brightness → f32"
        );
    }
}

#[test]
fn qa_slot_colour_is_vec3_and_brightness_is_f32() {
    slot_stain(Kind::Colour, COLOUR_SHOW).unwrap_or_else(|e| panic!("{e}"));
    slot_stain(Kind::Brightness, BRIGHT_SHOW).unwrap_or_else(|e| panic!("{e}"));
    check_signatures(&[
        (
            Kind::Colour,
            "fn colour(ctx: Ctx) -> vec4<f32> { return vec4<f32>(1.0); }",
        ),
        (Kind::Colour, "fn colour(ctx: Ctx) -> f32 { return 1.0; }"),
        (
            Kind::Colour,
            "fn colour(ctx: Ctx) -> vec3<u32> { return vec3<u32>(1u); }",
        ),
        (
            Kind::Colour,
            "fn brightness(ctx: Ctx) -> f32 { return 1.0; }",
        ),
        (
            Kind::Brightness,
            "fn brightness(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
        ),
        (
            Kind::Brightness,
            "fn brightness(ctx: Ctx) -> u32 { return 1u; }",
        ),
        (
            Kind::Brightness,
            "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
        ),
    ]);
}

negative_control!(
    qa_slot_colour_is_vec3_and_brightness_is_f32,
    "a well-typed colour offered as a bad one",
    expected = "is accepted",
    check_signatures(&[(Kind::Colour, COLOUR_SHOW)])
);

// ── REQ-RENDER-075: the canonical form's hash ──────────────────────────────────────────────────────────────────────

/// FNV-1a, 64-bit (offset basis 0xcbf29ce484222325, prime 0x100000001b3), written here from its definition.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// One graph built three ways: the backbone order; the sources and nodes listed differently (the brightness's source
/// first, the brightness before the colour); and with a dead source, a dead colour and a None post in the chain.
fn one_graph_three_ways() -> [Vec<Node>; 3] {
    let a = backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]);
    let b = vec![
        n(Kind::Source, src_const(0.7), &[]),
        n(Kind::Brightness, custom(BRIGHT_SHOW), &[Some(0)]),
        n(Kind::Source, src_const(0.3), &[]),
        n(Kind::Colour, custom(COLOUR_SHOW), &[Some(2)]),
        n(Kind::Combiner, custom(COMBINE_MUL), &[Some(3), Some(1)]),
        n(Kind::Post, custom(POST_ADD), &[Some(4)]),
        n(Kind::Post, custom(POST_DOUBLE), &[Some(5)]),
        n(Kind::Out, Occupant::None, &[Some(6)]),
    ];
    let c = vec![
        n(Kind::Source, src_const(0.9), &[]),
        n(Kind::Source, src_const(0.3), &[]),
        n(Kind::Colour, custom(COLOUR_CONST), &[Some(0)]),
        n(Kind::Source, src_const(0.7), &[]),
        n(Kind::Colour, custom(COLOUR_SHOW), &[Some(1)]),
        n(Kind::Brightness, custom(BRIGHT_SHOW), &[Some(3)]),
        n(Kind::Combiner, custom(COMBINE_MUL), &[Some(4), Some(5)]),
        n(Kind::Post, custom(POST_ADD), &[Some(6)]),
        n(Kind::Post, Occupant::None, &[Some(7)]),
        n(Kind::Post, custom(POST_DOUBLE), &[Some(8)]),
        n(Kind::Out, Occupant::None, &[Some(9)]),
    ];
    [a, b, c]
}

/// Graphs that differ from [`one_graph_three_ways`]'s and from one another in what renders.
fn different_graphs() -> Vec<(&'static str, Vec<Node>)> {
    let mut swapped = backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]);
    swapped[2].inputs = vec![Some(1)];
    swapped[3].inputs = vec![Some(0)];
    let mut shared = backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]);
    shared[3].inputs = vec![Some(0)];
    let mut field = backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]);
    field[0].occupant = Occupant::Field("ftle".into());
    let mut field2 = field.clone();
    field2[0].occupant = Occupant::Field("theta".into());
    vec![
        (
            "the reference",
            backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]),
        ),
        (
            "the posts swapped",
            backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_DOUBLE, POST_ADD]),
        ),
        ("one post", backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD])),
        ("no post", backbone(COLOUR_SHOW, BRIGHT_SHOW, &[])),
        (
            "another colour",
            backbone(COLOUR_CONST, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]),
        ),
        ("the sources swapped between colour and brightness", swapped),
        ("one source shared", shared),
        ("a built-in field source", field),
        ("another built-in field source", field2),
        ("brightness None", {
            let mut g = backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]);
            g[3].occupant = Occupant::None;
            g
        }),
        (
            "the custom text differs by a space",
            backbone(
                &COLOUR_SHOW.replace("{ ", "{  "),
                BRIGHT_SHOW,
                &[POST_ADD, POST_DOUBLE],
            ),
        ),
    ]
}

fn key(g: Vec<Node>) -> (u64, String) {
    let c = stain(g).canonical();
    (c.fragment_key(), c.text())
}

fn check_equal(graphs: &[Vec<Node>]) {
    let (k0, t0) = key(graphs[0].clone());
    let s0 = assemble::assemble(&stain(graphs[0].clone()), Tier::FULL)
        .unwrap()
        .source;
    for (i, g) in graphs.iter().enumerate().skip(1) {
        let (k, t) = key(g.clone());
        assert!(
            k == k0 && t == t0,
            "construction {i} of one graph hashes differently:\n{t0}\n{t}"
        );
        let s = assemble::assemble(&stain(g.clone()), Tier::FULL)
            .unwrap()
            .source;
        assert!(
            s == s0,
            "construction {i} of one graph assembles differently"
        );
    }
}

#[test]
fn qa_canonical_hash_one_graph_built_differently_hashes_equal() {
    check_equal(&one_graph_three_ways());
}

negative_control!(
    qa_canonical_hash_one_graph_built_differently_hashes_equal,
    "a graph with its posts swapped is offered as a construction of the same graph",
    expected = "hashes differently",
    check_equal(&[
        backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_ADD, POST_DOUBLE]),
        backbone(COLOUR_SHOW, BRIGHT_SHOW, &[POST_DOUBLE, POST_ADD]),
    ])
);

fn check_distinct(graphs: Vec<(&'static str, Vec<Node>)>) {
    let keys: Vec<(&str, u64, String)> = graphs
        .into_iter()
        .map(|(w, g)| {
            let (k, t) = key(g);
            (w, k, t)
        })
        .collect();
    for (i, a) in keys.iter().enumerate() {
        assert_eq!(
            a.1,
            fnv1a64(a.2.as_bytes()),
            "{}: the key is not the FNV-1a 64 of the form's text",
            a.0
        );
        for b in &keys[i + 1..] {
            assert!(a.1 != b.1, "{} and {} hash equal", a.0, b.0);
        }
    }
}

#[test]
fn qa_canonical_hash_different_graphs_hash_differently() {
    check_distinct(different_graphs());
}

negative_control!(
    qa_canonical_hash_different_graphs_hash_differently,
    "two constructions of one graph are offered as different graphs",
    expected = "hash equal",
    check_distinct(vec![
        ("construction a", one_graph_three_ways()[0].clone()),
        ("construction c", one_graph_three_ways()[2].clone()),
    ])
);

/// Lowering Part 5's text, written out by hand for the smallest live graph: a field source, a colour, the combiner,
/// OUT. Order: sources, colour, (no brightness), combiner, (no posts), OUT; inputs by position; JCS keys sorted.
fn check_text(want: &str) {
    let g = vec![
        n(Kind::Source, Occupant::Field("ftle".into()), &[]),
        n(Kind::Colour, custom(COLOUR_SHOW), &[Some(0)]),
        n(Kind::Combiner, pass_through(), &[Some(1), None]),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ];
    let (k, t) = key(g);
    assert_eq!(t, want, "the canonical text is not lowering Part 5's");
    assert_eq!(k, fnv1a64(want.as_bytes()));
}

const SMALLEST_TEXT: &str = concat!(
    r#"{"nodes":["#,
    r#"{"inputs":[],"kind":"source","occupant":{"field":"ftle"}},"#,
    r#"{"inputs":[0],"kind":"colour","occupant":{"custom":"fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }"}},"#,
    r#"{"inputs":[1,null],"kind":"combiner","occupant":{"builtin":"pass_through"}},"#,
    r#"{"inputs":[2],"kind":"out","occupant":null}"#,
    r#"]}"#
);

#[test]
fn qa_canonical_hash_the_text_is_the_defined_form() {
    check_text(SMALLEST_TEXT);
}

negative_control!(
    qa_canonical_hash_the_text_is_the_defined_form,
    "the brightness input written as absent-omitted",
    expected = "the canonical text is not lowering Part 5's",
    check_text(&SMALLEST_TEXT.replace("[1,null]", "[1]"))
);

// ── REQ-GEN-027: the declaration format ────────────────────────────────────────────────────────────────────────────

fn check_parses() {
    let d = Declaration::parse(
        "  // @uniform gain: f32 = 0.5 [0.0, 2.0]\n\
         // @uniform tint: vec3<f32> = (1.0, 0.5, 0.25)\n\
         // @uniform count: u32 = 3\n\
         // @input a [0.0, 6.5]\n\
         // @input b\n\
         // an ordinary comment is not a declaration\n\
         fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }",
    )
    .unwrap_or_else(|e| panic!("the worked format is refused: {e}"));
    assert_eq!(d.uniforms.len(), 3);
    assert_eq!(d.uniforms[0].name, "gain");
    assert_eq!(d.uniforms[0].ty, UniformType::F32);
    assert_eq!(d.uniforms[0].default, vec![0.5]);
    assert_eq!(d.uniforms[0].range, Some((0.0, 2.0)));
    assert_eq!(d.uniforms[1].ty, UniformType::Vec3);
    assert_eq!(d.uniforms[1].default, vec![1.0, 0.5, 0.25]);
    assert_eq!(d.uniforms[1].range, None);
    assert_eq!(d.uniforms[2].ty, UniformType::U32);
    assert_eq!(d.inputs.len(), 2);
    assert_eq!(d.inputs[0].name, "a");
    assert_eq!(d.inputs[0].domain, Some((0.0, 6.5)));
    assert_eq!(d.inputs[1].name, "b");
    assert_eq!(
        d.inputs[1].domain, None,
        "an input with no bracket inherits the manifest's domain"
    );
}

fn check_refused_declarations(lines: &[&str]) {
    for line in lines {
        assert!(
            Declaration::parse(line).is_err(),
            "`{line}` is accepted, but gui_state_contract §3 refuses it"
        );
    }
}

#[test]
fn qa_declaration_the_format_parses_and_refuses() {
    check_parses();
    check_refused_declarations(&[
        "// @unifrom gain: f32 = 0.5",
        "// @output x",
        "// @uniform gain: f32 = 3.0 [0.0, 2.0]",
        "// @uniform gain: f32 = 0.5 [2.0, 0.0]",
        "// @uniform gain: f32 = 0.5 [1.0, 1.0]",
        "// @uniform tint: vec3<f32> = (1.0, 0.5, 0.25) [0.0, 1.0]",
        "// @uniform tint: vec3<f32> = (1.0, 0.5)",
        "// @uniform k: i32 = 1.5",
        "// @uniform k: u32 = -1",
        "// @uniform k: f64 = 1.0",
        "// @uniform k: f32 = 1.0f",
        "// @uniform 1k: f32 = 1.0",
        "// @uniform k: f32 = 1.0\n// @uniform k: f32 = 2.0",
        "// @input a\n// @input a",
        "// @uniform k: f32",
    ]);
}

negative_control!(
    qa_declaration_the_format_parses_and_refuses,
    "a well-formed declaration offered as a refused one",
    expected = "is accepted, but",
    check_refused_declarations(&["// @uniform gain: f32 = 0.5 [0.0, 2.0]"])
);

fn input_count_ok(kind: Kind, count: usize) -> bool {
    let decls: String = (0..count).map(|k| format!("// @input i{k}\n")).collect();
    let body = match kind {
        Kind::Colour => "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }",
        Kind::Brightness => "fn brightness(ctx: Ctx) -> f32 { return 0.0; }",
        Kind::Post => "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }",
        Kind::Source => "fn source(ctx: Ctx) -> Field { return Field(0.0); }",
        Kind::Combiner => "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }",
        Kind::Out => unreachable!(),
    };
    assemble::declaration(kind, &custom(&format!("{decls}{body}"))).is_ok()
}

fn check_input_bounds(colour_most: usize) {
    for (kind, most) in [
        (Kind::Colour, colour_most),
        (Kind::Brightness, 1),
        (Kind::Post, 4),
        (Kind::Source, 0),
        (Kind::Combiner, 0),
    ] {
        assert!(
            input_count_ok(kind, most),
            "{kind:?} with {most} inputs refused"
        );
        assert!(
            !input_count_ok(kind, most + 1),
            "{kind:?} with {} inputs accepted; gui_state_contract §3 bounds it at {most}",
            most + 1
        );
    }
    // A colour or brightness that declares no input has one, `field`, inherited.
    let d = assemble::declaration(
        Kind::Brightness,
        &custom("fn brightness(ctx: Ctx) -> f32 { return 0.0; }"),
    )
    .unwrap();
    assert_eq!(d.inputs.len(), 1);
    assert_eq!(d.inputs[0].name, "field");
    assert_eq!(d.inputs[0].domain, None);
}

#[test]
fn qa_declaration_input_counts_per_kind() {
    check_input_bounds(4);
}

negative_control!(
    qa_declaration_input_counts_per_kind,
    "a colour bound at three inputs",
    expected = "inputs accepted",
    check_input_bounds(3)
);

/// A declared uniform, read as `uniforms.<name>`, reaches the GPU from the block the assembler declares, in group 0 at
/// the binding after the prelude's.
fn check_uniform(value: f32, want: f32) {
    let colour = "// @uniform gain: f32 = 0.5 [0.0, 1.0]\n// @uniform tint: vec3<f32> = (1.0, 1.0, 1.0)\nfn colour(ctx: Ctx) -> vec3<f32> { return uniforms.tint * uniforms.gain; }";
    let g = vec![
        n(Kind::Source, src_const(0.0), &[]),
        n(Kind::Colour, custom(colour), &[Some(0)]),
        n(Kind::Combiner, pass_through(), &[Some(1), None]),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ];
    let f = assemble::assemble(&stain(g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(f.uniforms.len(), 1, "one node declares uniforms");
    let block = &f.uniforms[0];
    let p = prelude::uniforms_binding();
    assert_eq!((block.group, block.binding), (p.group, p.binding + 1));
    let names: Vec<&str> = block.uniforms.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        names,
        ["gain", "tint"],
        "the block holds the entries in declaration order"
    );
    // WGSL uniform layout: f32 at 0, vec3<f32> at 16 (align 16); 32 bytes.
    let words = vec![
        value.to_bits(),
        0,
        0,
        0,
        1.0f32.to_bits(),
        0.5f32.to_bits(),
        0.25f32.to_bits(),
        0,
    ];
    let px = rgb(draw(
        &gpu(),
        &(f.source + ENTRY),
        Tier::FULL,
        0,
        &[words],
        [0; 4],
    ));
    assert_rgb(
        px,
        [want, want * 0.5, want * 0.25],
        "the node's uniforms as bound",
    );
}

#[test]
fn qa_declaration_a_uniform_reaches_the_gpu() {
    check_uniform(0.8, 0.8);
}

negative_control!(
    qa_declaration_a_uniform_reaches_the_gpu,
    "the bound gain is not the default",
    expected = "the backbone gives",
    check_uniform(0.8, 0.5)
);
