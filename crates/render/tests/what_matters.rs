//! What can change a stain's picture, proved over naga's IR (`render::assemble::matters`), and what its assembly drops.
//!
//! - Every shipped source (each registry entry in its slot, each field's source, each field's default ramp and its
//!   style post) declares no uniform and no input that can never reach its result (`shipped_controls_all_can_matter`).
//! - The proof's answers on fixtures written to be hard: a uniform that reaches only through a branch, only through a
//!   write through a pointer, only as `0 * x`; dead ones beside them; a choice that hides a control
//!   (`proof_on_fixtures`, `standing_names_the_setting`).
//! - On the GPU: every uniform, input, node result and field judged dead, set to NaN, infinity and random bits,
//!   changes no bit of the picture, and the assembly that drops them draws the same bits, over every shipped source
//!   and the fixtures, on two payloads: the presentation view of every row, the debug view of every fixture and of
//!   any row where it drops something else (`dead_things_change_no_bit_*`, in parts so a runner spreads them).
//!
//! Each property has a control arm, the proof broken one way: branches forgotten, pointer writes forgotten, `0 * x`
//! folded, or a planted control that does nothing.

#[cfg(feature = "controls")]
use render::assemble::matters::Forget;
use render::assemble::matters::{self, Standing};
use render::assemble::{self, Kind, Node, Occupant, Uniform};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::registry::{self, ZERO};
use validation::negative_control;

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

fn sim_key(name: &str) -> Option<f64> {
    matches!(name, "eps_E" | "eps_L").then_some(1.0 / 1_048_576.0)
}

/// Every shipped source, by name, with the kind of node it occupies and its text.
fn shipped() -> Vec<(String, Kind, String)> {
    let mut out = Vec::new();
    for e in registry::registry().unwrap_or_else(|e| panic!("{e}")) {
        out.push((format!("registry {}", e.id), e.slot, e.source));
    }
    for f in assemble::source_fields().unwrap_or_else(|e| panic!("{e}")) {
        let text = assemble::occupant_source(Kind::Source, &Occupant::Field(f.clone()))
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_default();
        out.push((format!("field {f}"), Kind::Source, text));
    }
    let l = ledger::payload::ledger();
    let resolve = Resolve { sim_key: &sim_key };
    for e in ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}")) {
        for slot in [Slot::Colour, Slot::Brightness] {
            let Ok(ramp) = defaults::field_ramp(&e, slot, &resolve) else {
                continue;
            };
            let (kind, occupant) = ramp.occupant();
            let text = assemble::occupant_source(kind, &occupant)
                .unwrap_or_else(|e| panic!("{e}"))
                .unwrap_or_default();
            out.push((format!("ramp {} {}", e.name, kind.name()), kind, text));
            if slot == Slot::Brightness {
                out.push((
                    format!("ramp {} style post", e.name),
                    Kind::Post,
                    ramp.style_post(),
                ));
            }
        }
    }
    out
}

/// Each source in `sources` with a declared uniform or input that can never reach its result, and which.
fn never_matter(sources: &[(String, Kind, String)]) -> Vec<String> {
    sources
        .iter()
        .filter_map(|(name, kind, text)| {
            let dead =
                matters::never_matters(*kind, text).unwrap_or_else(|e| panic!("{name}: {e}"));
            (!dead.is_empty()).then(|| format!("{name}: {}", dead.join(", ")))
        })
        .collect()
}

fn check_shipped(sources: &[(String, Kind, String)]) {
    let dead = never_matter(sources);
    println!(
        "shipped sources checked: {}; with a control that can never matter: {}",
        sources.len(),
        dead.len()
    );
    assert!(
        dead.is_empty(),
        "a shipped control can never matter: {}",
        dead.join("; ")
    );
}

#[test]
fn shipped_controls_all_can_matter() {
    check_shipped(&shipped());
}

negative_control!(
    shipped_controls_all_can_matter,
    "a shipped source with a planted uniform it never reads fails",
    expected = "a shipped control can never matter",
    {
        let mut s = shipped();
        let (name, kind, text) = s
            .iter()
            .find(|(n, ..)| n == "registry colour/outcome_state")
            .cloned()
            .unwrap_or_else(|| panic!("no outcome_state"));
        s.push((
            format!("{name}, planted"),
            kind,
            format!("// @uniform planted: f32 = 0.5 [0, 1]\n{text}"),
        ));
        check_shipped(&s)
    }
);

// ── Fixtures ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A colour that reads `k` only through a branch: which value it returns.
pub const BRANCH: &str = "// @uniform k: f32 = 0.25 [0, 1]
// @uniform unused: f32 = 0.5 [0, 1]
fn colour(ctx: Ctx) -> vec3<f32> {
    let spare = uniforms.unused * 2.0;
    if (uniforms.k > 0.5) {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    return vec3<f32>(0.0, 0.0, 1.0);
}
";

/// A colour that reads `k` only through a write through a pointer argument.
// A second `put` of `unused` into a variable nothing reads would count as live: the proof keeps one summary of `put`
// for every call, so what one call's pointer needs another's is given too. That loses no soundness, only a drop.
pub const POINTER: &str = "// @uniform k: f32 = 0.25 [0, 1]
// @uniform unused: f32 = 0.5 [0, 1]
fn put(p: ptr<function, f32>, v: f32) {
    *p = v;
}
fn colour(ctx: Ctx) -> vec3<f32> {
    var x = 0.0;
    var y = uniforms.unused;
    put(&x, uniforms.k);
    return vec3<f32>(x, 0.5, 0.5);
}
";

/// A colour that reads `k` only as `0 * k`, which is NaN when `k` is.
pub const ZERO_TIMES: &str = "// @uniform k: f32 = 0.25 [0, 1]
fn colour(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(0.5 + 0.0 * uniforms.k, 0.5, 0.5);
}
";

/// A post with two field inputs, reading the first and a field, ignoring the second and another field's read.
pub const INPUTS: &str = "// @input a
// @input b
// @uniform gain: f32 = 0.5 [0, 1]
// @uniform unused: vec3<f32> = (0.1, 0.2, 0.3)
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {
    let ignored = ctx.inputs[1].x + ctx.sample.ftle + uniforms.unused.y;
    return rgb * uniforms.gain + vec3<f32>(ctx.inputs[0].x * 0.01, f32(ctx.sample.state) * 0.01, 0.0);
}
";

/// A colour whose dead reads of `x` and `n` sit where a constant zero would not compile: a divisor, a remainder's and
/// a logarithm's argument.
pub const DIVISOR: &str = "// @uniform gain: f32 = 0.5 [0, 1]
// @uniform x: f32 = 2.0 [1, 4]
// @uniform n: i32 = 3 [1, 9]
fn colour(ctx: Ctx) -> vec3<f32> {
    let a = 1.0 / uniforms.x;
    let b = 7 % uniforms.n;
    let c = log(uniforms.x);
    return vec3<f32>(uniforms.gain);
}
";

/// A colour whose choice decides which of two colours it draws.
pub const CHOICE: &str = r#"// @uniform mode: u32 = 0
// @control mode choice("first", "second") "Mode"
// @uniform first: vec3<f32> = (1.0, 0.0, 0.0)
// @control first colour "First"
// @uniform second: vec3<f32> = (0.0, 0.0, 1.0)
// @control second colour "Second"
fn colour(ctx: Ctx) -> vec3<f32> {
    if (uniforms.mode == 0u) {
        return uniforms.first;
    }
    return uniforms.second;
}
"#;

fn live(kind: Kind, text: &str) -> Vec<bool> {
    matters::shader(kind, text)
        .unwrap_or_else(|e| panic!("{e}"))
        .never()
        .uniforms
}

fn check_fixtures() {
    // Pass criteria, written before the first run: `k` is live in BRANCH, POINTER and ZERO_TIMES; `unused` is dead in
    // BRANCH and POINTER; in INPUTS input `a`, `gain` and the field `state` are live, input `b`, `unused` and `ftle`
    // dead; CHOICE's three uniforms are all live.
    assert_eq!(
        live(Kind::Colour, BRANCH),
        [true, false],
        "the proof on BRANCH"
    );
    assert_eq!(
        live(Kind::Colour, POINTER),
        [true, false],
        "the proof on POINTER"
    );
    assert_eq!(
        live(Kind::Colour, ZERO_TIMES),
        [true],
        "the proof on ZERO_TIMES"
    );
    let s = matters::shader(Kind::Post, INPUTS).unwrap_or_else(|e| panic!("{e}"));
    let r = s.never();
    assert_eq!(r.uniforms, [true, false], "the proof on INPUTS");
    assert_eq!(r.inputs, [true, false], "the proof on INPUTS");
    assert_eq!(r.values, [true], "the proof on INPUTS");
    assert!(
        r.fields.contains("state") && !r.fields.contains("ftle"),
        "the proof on INPUTS: {:?}",
        r.fields
    );
    assert_eq!(
        live(Kind::Colour, CHOICE),
        [true, true, true],
        "the proof on CHOICE"
    );
}

#[test]
fn proof_on_fixtures() {
    check_fixtures();
}

negative_control!(
    proof_on_fixtures,
    "a proof that forgets branches judges BRANCH's `k` dead",
    expected = "the proof on BRANCH",
    matters::probe::forgetting(Forget::Branches, check_fixtures)
);

// A picture cannot show this one on every backend: lavapipe draws `0.5 + 0.0 * k` as 0.5 for `k` NaN or infinite,
// folding the product itself. The proof must not: an IEEE backend draws NaN.
fn check_zero_times() {
    assert_eq!(
        live(Kind::Colour, ZERO_TIMES),
        [true],
        "the proof judged `0 * k`'s `k` dead"
    );
}

#[test]
fn zero_times_reaches() {
    // Pass criterion: `k` in `0.0 * k` can reach the picture.
    check_zero_times();
}

negative_control!(
    zero_times_reaches,
    "a proof that folds `0 * x` judges ZERO_TIMES's `k` dead",
    expected = "judged `0 * k`'s `k` dead",
    matters::probe::forgetting(Forget::ZeroTimes, check_zero_times)
);

/// DIVISOR assembled: its dead `x` and `n` are dropped and it compiles.
fn check_divisor() {
    let stain = Stain::new(showing(Kind::Colour, Occupant::Custom(DIVISOR.into())))
        .unwrap_or_else(|e| panic!("{e}"));
    let f = assemble::assemble(&stain, Tier::FULL)
        .unwrap_or_else(|e| panic!("the dropping assembly of DIVISOR does not compile: {e}"));
    let gone: Vec<&str> = f
        .dropped
        .uniforms
        .iter()
        .map(|(_, u)| u.name.as_str())
        .collect();
    assert_eq!(gone, ["x", "n"], "DIVISOR's dead uniforms");
}

#[test]
fn a_dead_divisor_assembles() {
    // Pass criterion: DIVISOR's `x` and `n` are dropped, and the assembly that drops them compiles.
    check_divisor();
}

negative_control!(
    a_dead_divisor_assembles,
    "a dropped uniform's reads given a constant zero, which naga folds and refuses",
    expected = "does not compile",
    matters::probe::constant_stand_in(check_divisor)
);

fn standing(text: &str, mode: f64) -> Vec<Standing> {
    let s = matters::shader(Kind::Colour, text).unwrap_or_else(|e| panic!("{e}"));
    s.standing(&|u: &Uniform| {
        if u.name == "mode" {
            vec![mode]
        } else {
            u.default.clone()
        }
    })
}

#[test]
fn standing_names_the_setting() {
    // Pass criterion: with `mode` at 0 (first), `second` cannot matter now and setting `mode` to 1 makes it; with
    // `mode` at 1, `first` likewise needs 0; `mode` itself always matters; BRANCH's `unused` never does.
    let at = |mode: f64, uniform: &str, value: f64| {
        let s = standing(CHOICE, mode);
        let names = ["mode", "first", "second"];
        let k = names.iter().position(|n| *n == uniform).unwrap_or(0);
        assert_eq!(s[0], Standing::Matters, "mode at {mode}: {s:?}");
        let other = 3 - k;
        assert_eq!(s[other], Standing::Matters, "mode at {mode}: {s:?}");
        match &s[k] {
            Standing::NotNow { needs: Some(n) } => {
                assert_eq!((n.uniform.as_str(), n.value), ("mode", value), "{s:?}")
            }
            other => panic!("mode at {mode}: `{uniform}` is {other:?}, not dimmed with a setting"),
        }
    };
    at(0.0, "second", 1.0);
    at(1.0, "first", 0.0);
    let s = matters::shader(Kind::Colour, BRANCH).unwrap_or_else(|e| panic!("{e}"));
    let b = s.standing(&|u: &Uniform| u.default.clone());
    assert_eq!(b, [Standing::Matters, Standing::Never]);
}

negative_control!(
    standing_names_the_setting,
    "the standing at mode 1 read as mode 0's: `second` matters there",
    expected = "not dimmed with a setting",
    {
        let s = standing(CHOICE, 1.0);
        match &s[2] {
            Standing::NotNow { needs: Some(_) } => {}
            other => panic!("`second` is {other:?}, not dimmed with a setting"),
        }
    }
);
/// A graph showing `occupant` in a node of `kind` over the zero source, as `declared_code.rs` shows each row, posts
/// after the pass-through combiner of a plain colour.
pub fn showing(kind: Kind, occupant: Occupant) -> Vec<Node> {
    let show = Occupant::Custom(
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }".into(),
    );
    let fed = |kind: Kind, occupant: Occupant, colour: Option<usize>| {
        let d = assemble::declaration(kind, &occupant).unwrap_or_else(|e| panic!("{e}"));
        let mut inputs: Vec<Option<usize>> = vec![Some(0); d.inputs.len()];
        if kind == Kind::Post {
            inputs.insert(0, colour);
        }
        node(kind, occupant, &inputs)
    };
    let pass = Occupant::BuiltIn("pass_through".into());
    let zero = node(Kind::Source, Occupant::BuiltIn(ZERO.to_owned()), &[]);
    match kind {
        Kind::Source => vec![
            node(Kind::Source, occupant, &[]),
            fed(Kind::Colour, show, None),
            node(Kind::Combiner, pass, &[Some(1), None]),
            node(Kind::Out, Occupant::None, &[Some(2)]),
        ],
        Kind::Colour => vec![
            zero,
            fed(Kind::Colour, occupant, None),
            node(Kind::Combiner, pass, &[Some(1), None]),
            node(Kind::Out, Occupant::None, &[Some(2)]),
        ],
        Kind::Brightness => vec![
            zero,
            fed(Kind::Brightness, occupant, None),
            node(Kind::Combiner, pass, &[None, Some(1)]),
            node(Kind::Out, Occupant::None, &[Some(2)]),
        ],
        Kind::Combiner => vec![
            zero,
            fed(Kind::Colour, show, None),
            node(Kind::Combiner, occupant, &[Some(1), None]),
            node(Kind::Out, Occupant::None, &[Some(2)]),
        ],
        Kind::Post => vec![
            zero,
            fed(Kind::Colour, show, None),
            node(Kind::Combiner, pass, &[Some(1), None]),
            fed(Kind::Post, occupant, Some(2)),
            node(Kind::Out, Occupant::None, &[Some(3)]),
        ],
        Kind::Out => unreachable!("OUT has no occupant"),
    }
}

// ── On the GPU: nothing judged dead changes a bit ─────────────────────────────────────────────────────────────────

use render::assemble::{Fragment, Stain, Tier, View};
use render::bind;
use render::headless::{self, Draw, Target};
use render::pipeline_cache::{block_layout, encode};
use validation::golden_scene::{self, Scene};
use validation::gpu::GpuHarness;

/// The payloads every row is drawn over: the outcome scene's states, failures and not-a-system samples, and the
/// detail view's, whose words, FTLEs and flags vary sample by sample.
fn payloads() -> Vec<Scene> {
    ["outcome", "detail_view"]
        .into_iter()
        .map(|n| golden_scene::scene(n).unwrap_or_else(|e| panic!("{e}")))
        .collect()
}

/// A round of poison. Every input, node result and field judged dead is poisoned pixel by pixel, NaN, +infinity,
/// -infinity or random bits by the pixel's place and the round's salt, so every tile holds each; every dead uniform
/// is poisoned in its block, one value a round: NaN, then ±infinity by component, then random bits.
#[derive(Clone, Copy, Debug)]
struct Poison {
    salt: u32,
}

const ROUNDS: [Poison; 3] = [Poison { salt: 0 }, Poison { salt: 1 }, Poison { salt: 2 }];

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

impl Poison {
    /// The bits of component `k` of a dead uniform this round: an `f32`'s, or an integer's.
    fn bits(self, k: u32, float: bool) -> u32 {
        match (self.salt, float) {
            (0, true) => 0x7fc0_0000,
            (1, true) => [0x7f80_0000, 0xff80_0000][k as usize % 2],
            (0, false) => 0xffff_ffff,
            (1, false) => 0x8000_0000,
            _ => hash(0x9e37_79b9 ^ k.wrapping_mul(0x2545_f491)),
        }
    }

    /// A WGSL expression of type `ty` poisoned this round at the pixel `at`, never a constant expression: WGSL refuses
    /// a non-finite one.
    fn wgsl(self, ty: &str, at: &str) -> String {
        let salt = self.salt;
        let scalar = |k: u32, s: &str| match s {
            "f32" => format!("poison_f32({at}, {k}u, {salt}u)"),
            "u32" => format!("poison_u32({at}, {k}u, {salt}u)"),
            "i32" => format!("bitcast<i32>(poison_u32({at}, {k}u, {salt}u))"),
            "bool" => format!("((poison_u32({at}, {k}u, {salt}u) & 1u) == 1u)"),
            other => panic!("no poison for `{other}`"),
        };
        if let Some(inner) = ty.strip_prefix("array<").and_then(|t| t.strip_suffix('>')) {
            let (elem, n) = inner
                .rsplit_once(',')
                .unwrap_or_else(|| panic!("no length in `{ty}`"));
            let n: usize = n.trim().parse().unwrap_or_else(|e| panic!("`{ty}`: {e}"));
            let parts: Vec<String> = (0..n).map(|_| self.wgsl(elem.trim(), at)).collect();
            return format!("{ty}({})", parts.join(", "));
        }
        if let Some(inner) = ty.strip_prefix("vec").and_then(|t| t.get(2..t.len() - 1)) {
            let n: u32 = ty[3..4].parse().unwrap_or(4);
            let parts: Vec<String> = (0..n).map(|k| scalar(k, inner)).collect();
            return format!("{ty}({})", parts.join(", "));
        }
        scalar(0, ty)
    }
}

/// The helpers a poisoned source calls: NaN, ±infinity or random bits by the pixel's place, component and salt.
const POISON_FNS: &str = "
fn poison_hash(n: u32) -> u32 {
    var x = n ^ (n >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    return x ^ (x >> 16u);
}
fn poison_u32(xy: vec2<f32>, k: u32, salt: u32) -> u32 {
    let n = u32(xy.x) * 7u + u32(xy.y) * 13u + k * 3u + salt;
    switch n % 4u {
        case 0u { return 0x7fc00000u; }
        case 1u { return 0x7f800000u; }
        case 2u { return 0xff800000u; }
        default { return poison_hash(n); }
    }
}
fn poison_f32(xy: vec2<f32>, k: u32, salt: u32) -> f32 { return bitcast<f32>(poison_u32(xy, k, salt)); }
";

/// The type of member `name` of the struct `ty` as `source` declares it.
fn member_type(source: &str, ty: &str, name: &str) -> String {
    let start = source
        .find(&format!("struct {ty} {{"))
        .unwrap_or_else(|| panic!("no struct {ty}"));
    let body = &source[start
        ..source[start..]
            .find('}')
            .map_or(source.len(), |e| start + e)];
    body.lines()
        .filter_map(|l| l.trim().strip_prefix(&format!("{name}:")))
        .map(|t| t.trim().trim_end_matches(',').to_owned())
        .next()
        .unwrap_or_else(|| panic!("no member {name} of {ty}"))
}

/// `kept`'s source with everything `dropped` names as dead poisoned `p`: each dead node's result, each dead wire's
/// value and each dead field as its read side hands it to the stain. Its dead uniforms are poisoned in their blocks.
fn poisoned(kept: &Fragment, dropped: &Fragment, p: Poison) -> String {
    fn swap(s: &mut String, from: &str, to: &str) {
        assert!(s.contains(from), "the kept source has no `{from}`");
        *s = s.replacen(from, to, 1);
    }
    let mut s = kept.source.clone();
    let d = &dropped.dropped;
    let at = "ctx.frag_xy";
    for &i in &d.nodes {
        for (from, to) in [
            (
                format!("let f{i} = n{i}_source(ctx);"),
                format!("let f{i} = {};", p.wgsl("vec4<f32>", at)),
            ),
            (
                format!("let rgb = n{i}_colour(c{i});"),
                format!("let rgb = {};", p.wgsl("vec3<f32>", at)),
            ),
            (
                format!("let b = n{i}_brightness(c{i});"),
                format!("let b = {};", p.wgsl("f32", at)),
            ),
            (
                format!("out = n{i}_post(c{i}, out);"),
                format!("out = {};", p.wgsl("vec3<f32>", at)),
            ),
        ] {
            if s.contains(&from) {
                swap(&mut s, &from, &to);
            }
        }
    }
    for &(i, k) in &d.inputs {
        let wire = format!("    c{i}.inputs[{k}] = f");
        let start = s.find(&wire).unwrap_or_else(|| panic!("no wire `{wire}`"));
        let end = start + s[start..].find(';').unwrap_or(0);
        s.replace_range(
            start..end,
            &format!("    c{i}.inputs[{k}] = {}", p.wgsl("vec4<f32>", at)),
        );
    }
    // In `shade_at`, before the context's pixel is set: the argument's.
    let at = "frag_xy";
    let mut fields = String::new();
    for f in kept.fields.iter().filter(|f| !dropped.fields.contains(f)) {
        let (path, ty) = match f.strip_prefix("ic.") {
            Some(m) => (format!("ctx.ic.{m}"), member_type(&s, "ICDescriptor", m)),
            None if f.starts_with("word.") => (format!("ctx.sample.{f}"), "u32".to_owned()),
            None => (format!("ctx.sample.{f}"), member_type(&s, "SimState", f)),
        };
        fields.push_str(&format!("    {path} = {};\n", p.wgsl(&ty, at)));
    }
    let after = "    ctx.ic = ic_read(i);\n";
    swap(&mut s, after, &format!("{after}{fields}"));
    s.push_str(POISON_FNS);
    s
}

/// Draws `fragment` over `scene`'s payload into an `Rgba32Float` target, each node block holding its declared defaults
/// but where `bits(node, uniform)` gives a uniform's raw component bits, which may be any bits at all.
fn draw(
    h: &GpuHarness,
    fragment: &Fragment,
    source: &str,
    scene: &Scene,
    bits: &dyn Fn(usize, &Uniform) -> Option<Vec<u32>>,
) -> Vec<u32> {
    use wgpu::util::DeviceExt;
    let device = h.device();
    let module = bind::preset_module(source);
    let bytes = scene.set.bytes_at(Tier::FULL.has_ftle);
    let bound = bind::upload(device, &bytes.payload(), &scene.context);
    let buffer = |label: &str, contents: &[u8], usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage,
        })
    };
    let words: Vec<u8> =
        ledger::gen::prelude::uniform_words_with(scene.context.grid.e, [0.1, 0.2, 0.3])
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
    let first = ledger::gen::prelude::uniforms_binding();
    let mut buffers = vec![(
        first.binding,
        buffer("prelude", &words, wgpu::BufferUsages::UNIFORM),
    )];
    for block in &fragment.uniforms {
        let (offsets, size) = block_layout(&block.uniforms);
        let mut contents = vec![0u8; size as usize];
        for (u, &at) in block.uniforms.iter().zip(&offsets) {
            let v: Vec<u8> = match bits(block.node, u) {
                Some(b) => b.iter().flat_map(|w| w.to_le_bytes()).collect(),
                None => encode(u.ty, &u.default),
            };
            contents[at as usize..at as usize + v.len()].copy_from_slice(&v);
        }
        buffers.push((
            block.binding,
            buffer("node", &contents, wgpu::BufferUsages::UNIFORM),
        ));
    }
    let entry = |binding: u32, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let entries: Vec<_> = buffers
        .iter()
        .map(|(b, _)| entry(*b, wgpu::BufferBindingType::Uniform))
        .collect();
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    });
    let group_entries: Vec<_> = buffers
        .iter()
        .map(|(b, buf)| wgpu::BindGroupEntry {
            binding: *b,
            resource: buf.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &group_entries,
    });
    let counts = buffer("counts", &[0u8; 12], wgpu::BufferUsages::STORAGE);
    let counts_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[entry(
            assemble::DEBUG_COUNTS_BINDING,
            wgpu::BufferBindingType::Storage { read_only: false },
        )],
    });
    let counts_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &counts_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: assemble::DEBUG_COUNTS_BINDING,
            resource: counts.as_entire_binding(),
        }],
    });
    let [_, l1, l2] = bound.layout_refs();
    let [_, g1, g2] = bound.group_refs();
    let mut layouts = vec![&layout, l1, l2];
    let mut groups = vec![&group, g1, g2];
    if fragment.debug_arm {
        layouts.push(&counts_layout);
        groups.push(&counts_group);
    }
    let (width, height) = scene.context.grid.target();
    let image = headless::render(
        device,
        h.queue(),
        &Draw {
            module: &module,
            entry: bind::PRESET_ENTRY,
            layouts: &layouts,
            groups: &groups,
        },
        Target {
            width,
            height,
            format: wgpu::TextureFormat::Rgba32Float,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    image
        .bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

/// The rows: every shipped source shown as `declared_code.rs` shows it, and the fixtures.
fn rows() -> Vec<(String, Vec<Node>)> {
    let mut out: Vec<(String, Vec<Node>)> = Vec::new();
    for (name, kind, text) in shipped() {
        let occupant = match name.strip_prefix("registry ") {
            Some(_) => Occupant::Custom(text),
            None => match name.strip_prefix("field ") {
                Some(f) => Occupant::Field(f.to_owned()),
                None => Occupant::Custom(text),
            },
        };
        out.push((name, showing(kind, occupant)));
    }
    out.extend(fixture_rows());
    out
}

fn fixture_rows() -> Vec<(String, Vec<Node>)> {
    vec![
        (
            "fixture branch".into(),
            showing(Kind::Colour, Occupant::Custom(BRANCH.into())),
        ),
        (
            "fixture pointer".into(),
            showing(Kind::Colour, Occupant::Custom(POINTER.into())),
        ),
        (
            "fixture zero times".into(),
            showing(Kind::Colour, Occupant::Custom(ZERO_TIMES.into())),
        ),
        (
            "fixture inputs".into(),
            showing(Kind::Post, Occupant::Custom(INPUTS.into())),
        ),
        (
            "fixture divisor".into(),
            showing(Kind::Colour, Occupant::Custom(DIVISOR.into())),
        ),
        (
            "fixture choice".into(),
            showing(Kind::Colour, Occupant::Custom(CHOICE.into())),
        ),
    ]
}

/// What a run of [`check_rows`] found: rows drawn, rows with something dropped, and each thing poisoned.
#[derive(Default, Debug)]
struct Tally {
    rows: usize,
    dropped_rows: usize,
    uniforms: usize,
    inputs: usize,
    nodes: usize,
    fields: usize,
    draws: usize,
    /// The rows, by view, the proof could not follow: in them nothing is judged dead.
    unfollowed: Vec<String>,
    /// The rows whose debug view drops what their presentation view does, not drawn again.
    debug_as_presented: usize,
}

impl Tally {
    fn add(&mut self, o: &Tally) {
        self.rows += o.rows;
        self.dropped_rows += o.dropped_rows;
        self.uniforms += o.uniforms;
        self.inputs += o.inputs;
        self.nodes += o.nodes;
        self.fields += o.fields;
        self.draws += o.draws;
        self.unfollowed.extend(o.unfollowed.iter().cloned());
        self.debug_as_presented += o.debug_as_presented;
    }
}

/// Row `name` in each view, over `scene`: drawn kept at its defaults, kept with everything judged dead poisoned, a
/// round for each way a dead uniform is poisoned (one where there is none), and dropped; each the same bits as the
/// first.
fn check_row(
    h: &GpuHarness,
    name: &str,
    g: &[Node],
    scene: &Scene,
    t: &mut Tally,
    every_view: bool,
) {
    let stain = Stain::new(g.to_vec()).unwrap_or_else(|e| panic!("{name}: {e}"));
    t.rows += 1;
    let mut presented = None;
    for view in [View::Presentation, View::Debug] {
        let kept = assemble::assemble_kept(&stain, Tier::FULL, view)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let dropped = assemble::assemble_view(&stain, Tier::FULL, view)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let d = &dropped.dropped;
        let gone: Vec<&String> = kept
            .fields
            .iter()
            .filter(|f| !dropped.fields.contains(f))
            .collect();
        let fewer = gone.len();
        // The debug arm reads the sample itself, so what it shares with the stain is live in it: where the debug view
        // drops what the presentation view does, the poison reaches the arm through nothing the first draw did not
        // test. Drawing it again would double the sweep's GPU time for no new path.
        let this = (
            d.clone(),
            gone.into_iter().cloned().collect::<Vec<String>>(),
        );
        if view == View::Debug && !every_view && presented.as_ref() == Some(&this) {
            t.debug_as_presented += 1;
            continue;
        }
        presented = Some(this);
        if d.nodes.is_empty() && d.inputs.is_empty() && d.uniforms.is_empty() && fewer == 0 {
            assert_eq!(
                kept.source, dropped.source,
                "{name} {view:?}: nothing dropped, yet the code changed"
            );
            continue;
        }
        if view == View::Presentation {
            t.dropped_rows += 1;
            t.uniforms += d.uniforms.len();
            t.inputs += d.inputs.len();
            t.nodes += d.nodes.len();
            t.fields += fewer;
        }
        let want = draw(h, &kept, &kept.source, scene, &|_, _| None);
        let rounds = if d.uniforms.is_empty() {
            &ROUNDS[..1]
        } else {
            &ROUNDS[..]
        };
        for &p in rounds {
            let source = poisoned(&kept, &dropped, p);
            let bits = |node: usize, u: &Uniform| {
                d.uniforms
                    .iter()
                    .any(|(n, v)| *n == node && v == u)
                    .then(|| {
                        let float = !matches!(
                            u.ty,
                            assemble::UniformType::I32 | assemble::UniformType::U32
                        );
                        (0..u.ty.components() as u32)
                            .map(|k| p.bits(k, float))
                            .collect()
                    })
            };
            let got = draw(h, &kept, &source, scene, &bits);
            t.draws += 1;
            let differ = got.iter().zip(&want).filter(|(a, b)| a != b).count();
            assert!(
                differ == 0,
                "{name} {view:?} on `{}`: poisoning what was judged dead ({p:?}) changed a bit of {differ} \
                 channel(s); dropped {d:?}",
                scene.name
            );
        }
        let got = draw(h, &dropped, &dropped.source, scene, &|_, _| None);
        t.draws += 2;
        let differ = got.iter().zip(&want).filter(|(a, b)| a != b).count();
        assert!(
            differ == 0,
            "{name} {view:?} on `{}`: the assembly that drops changed a bit of {differ} channel(s)",
            scene.name
        );
    }
}

/// Every row checked ([`check_row`]), over the payloads in turn, the rows shared among threads on one device.
fn check_rows(h: &GpuHarness, rows: &[(String, Vec<Node>)]) -> Tally {
    let scenes = payloads();
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(1);
    let rows: Vec<(usize, &(String, Vec<Node>))> = rows.iter().enumerate().collect();
    let chunk = rows.len().div_ceil(threads).max(1);
    let mut t = Tally::default();
    std::thread::scope(|s| {
        let handles: Vec<_> = rows
            .chunks(chunk)
            .map(|part| {
                let scenes = &scenes;
                s.spawn(move || {
                    let mut t = Tally::default();
                    for (k, (name, g)) in part {
                        let fixture = name.starts_with("fixture ");
                        check_row(h, name, g, &scenes[k % scenes.len()], &mut t, fixture);
                    }
                    t
                })
            })
            .collect();
        for handle in handles {
            t.add(
                &handle
                    .join()
                    .unwrap_or_else(|e| std::panic::resume_unwind(e)),
            );
        }
    });
    t
}

/// A fixture row checked over every payload.
#[cfg(feature = "controls")]
fn check_fixture(h: &GpuHarness, name: &str) {
    let (_, g) = fixture_rows()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no fixture `{name}`"));
    for scene in payloads() {
        check_row(h, name, &g, &scene, &mut Tally::default(), true);
    }
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// The parts the rows are checked in, so a test runner spreads them.
const PARTS: usize = 16;

/// Part `part` of the rows: each row whose place is `part` modulo [`PARTS`].
fn part(part: usize) -> Vec<(String, Vec<Node>)> {
    rows()
        .into_iter()
        .enumerate()
        .filter(|(k, _)| k % PARTS == part)
        .map(|(_, r)| r)
        .collect()
}

fn check_part(k: usize) {
    // Pass criterion: no channel of any pixel differs, by a single bit, between a row drawn kept at its defaults and
    // drawn with what was judged dead poisoned, or drawn as the assembly that drops it.
    let h = gpu();
    let started = std::time::Instant::now();
    let t = check_rows(&h, &part(k));
    println!("part {k}: {t:?}, in {:.1?}", started.elapsed());
}

/// The fixture row `name` checked with the proof broken by `forget`.
#[cfg(feature = "controls")]
fn broken(forget: Forget, name: &str) {
    let h = gpu();
    matters::probe::forgetting(forget, || check_fixture(&h, name));
}

macro_rules! parts {
    ($($test:ident: $k:expr, $forget:ident, $fixture:literal;)*) => {$(
        #[test]
        fn $test() {
            check_part($k);
        }

        negative_control!(
            $test,
            "a proof broken one way drops what reaches the picture, and the picture changes",
            expected = "changed a bit",
            broken(Forget::$forget, $fixture)
        );
    )*};
}

parts! {
    dead_things_change_no_bit_0: 0, Branches, "fixture branch";
    dead_things_change_no_bit_1: 1, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_2: 2, Branches, "fixture branch";
    dead_things_change_no_bit_3: 3, Branches, "fixture branch";
    dead_things_change_no_bit_4: 4, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_5: 5, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_6: 6, Branches, "fixture branch";
    dead_things_change_no_bit_7: 7, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_8: 8, Branches, "fixture branch";
    dead_things_change_no_bit_9: 9, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_10: 10, Branches, "fixture branch";
    dead_things_change_no_bit_11: 11, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_12: 12, Branches, "fixture branch";
    dead_things_change_no_bit_13: 13, PointerWrites, "fixture pointer";
    dead_things_change_no_bit_14: 14, Branches, "fixture branch";
    dead_things_change_no_bit_15: 15, PointerWrites, "fixture pointer";
}

/// What the proof drops over every row, a presentation view, counted without drawing, the rows shared among threads;
/// and the rows, by view, it could not follow.
fn tally() -> Tally {
    let rows = rows();
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let chunk = rows.len().div_ceil(threads).max(1);
    let mut t = Tally::default();
    std::thread::scope(|s| {
        let handles: Vec<_> = rows
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut t = Tally::default();
                    for (name, g) in part {
                        tally_row(name, g, &mut t);
                    }
                    t
                })
            })
            .collect();
        for h in handles {
            t.add(&h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)));
        }
    });
    t
}

fn tally_row(name: &str, g: &[Node], t: &mut Tally) {
    let stain = Stain::new(g.to_vec()).unwrap_or_else(|e| panic!("{name}: {e}"));
    let kept = assemble::assemble_kept(&stain, Tier::FULL, View::Presentation)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let f = assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{name}: {e}"));
    let d = &f.dropped;
    t.rows += 1;
    t.dropped_rows += usize::from(kept.source != f.source);
    t.uniforms += d.uniforms.len();
    t.inputs += d.inputs.len();
    t.nodes += d.nodes.len();
    t.fields += kept.fields.iter().filter(|x| !f.fields.contains(x)).count();
    for view in [View::Presentation, View::Debug] {
        if !matters::followed(&stain, Tier::FULL, view).unwrap_or_else(|e| panic!("{name}: {e}")) {
            t.unfollowed.push(format!("{name} {view:?}"));
        }
    }
}

fn check_each_kind(t: &Tally) {
    assert!(
        t.unfollowed.is_empty(),
        "the proof could not follow {:?}",
        t.unfollowed
    );
    assert!(
        t.uniforms > 0 && t.inputs > 0 && t.nodes > 0 && t.fields > 0,
        "not every kind is dropped somewhere: {t:?}"
    );
}

#[test]
fn something_of_each_kind_is_dropped() {
    // Pass criterion: the proof follows every row in both views, so none passes the picture tests for having nothing
    // judged dead; and over the rows at least one uniform, input, node and field is dropped, so those tests poison
    // something of each kind.
    let t = tally();
    println!("{t:?}");
    check_each_kind(&t);
}

negative_control!(
    something_of_each_kind_is_dropped,
    "the tally with its dropped uniforms taken away",
    expected = "not every kind is dropped somewhere",
    check_each_kind(&Tally {
        uniforms: 0,
        ..tally()
    })
);

// ── Question 3: what can change a stain's picture ─────────────────────────────────────────────────────────────────

fn changes(colour: &str) -> matters::Changes {
    let g = showing(Kind::Colour, Occupant::Custom(colour.into()));
    let stain = Stain::new(g).unwrap_or_else(|e| panic!("{e}"));
    matters::stain_changes(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"))
}

fn check_changes(cases: &[(&str, &str, bool, bool, bool)]) {
    for &(name, colour, data, pixel, constant) in cases {
        let c = changes(colour);
        assert_eq!(
            (!c.fields.is_empty(), c.pixel, c.constant()),
            (data, pixel, constant),
            "{name}: what can change the picture: {c:?}"
        );
    }
}

/// Each colour: whether it reads a field, the pixel's place, and nothing at all.
const CHANGES: [(&str, &str, bool, bool, bool); 4] = [
    ("flat", "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }", false, false, true),
    (
        "a field",
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.sample.d_min); }",
        true,
        false,
        false,
    ),
    (
        "a pixel",
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.frag_xy.x); }",
        false,
        true,
        false,
    ),
    (
        "a field read into nothing",
        "fn colour(ctx: Ctx) -> vec3<f32> { let x = ctx.sample.d_min + ctx.frag_xy.y; return vec3<f32>(0.5); }",
        false,
        false,
        true,
    ),
];

#[test]
fn what_changes_the_picture() {
    // Pass criterion: as `CHANGES` says, for each of its four colours.
    check_changes(&CHANGES);
}

negative_control!(
    what_changes_the_picture,
    "the field read into nothing, expected to change the picture",
    expected = "what can change the picture",
    check_changes(&[(
        "a field read into nothing",
        CHANGES[3].1,
        true,
        false,
        false
    )])
);
