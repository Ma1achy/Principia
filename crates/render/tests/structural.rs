//! The structural views and overlays' fragment side (TASK-M1-13): the stain's quad and tile lanes, the occupants'
//! styling, and the CPU mirror they are held to (`render::structural`):
//! - RQ-238: the stain's `CtxQuad` is colour_composition §3's quad lane, member for member the harness's
//!   (`render::bind::quad_lane`), and `preset_module` fills each `RenderQuad` member from `quad_read` by `QUAD_LANE`'s
//!   mapping (`structural_ctx_*`); `shade_sample`, which places no quad, fills the quad and tile lanes with the absence
//!   NaN, a u32 member with its bits, so the views draw the hatch and the overlays pass the colour on
//!   (`structural_absent_*`);
//! - REQ-TOOL-124: the pending hatch and the fallback tint as debug_tooling_plan §F defines them, in the WGSL and the
//!   mirror alike; each differs from `debug_invalid`'s hatch in pattern and in colours; and REQ-COL-055's no-collision
//!   measurement, rerun with their colours, finds each farther from every palette entry than R-16's flat magenta
//!   (`structural_styling_*`);
//! - REQ-RENDER-024: `edge_line`'s pixel size reads `1 / cell size` across a cell edge, where §12.1's `fwidth(d)` reads
//!   0 (`structural_edge_*`);
//! - colour_composition §6: `s_impurity`'s ramp, the prelude's `ramp_magma`, reads the published magma table
//!   (`structural_impurity_*`);
//! - the CPU mirror the goldens hold the occupants to reads and computes as worked by hand (`structural_mirror_*`).
//!
//! Each test registers its negative control (R-176).

use ledger::gen::prelude;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier};
use render::bind;
use render::present::{self, Rgb};
use render::structural as mirror;
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;
use validation::synthetic::simstate_words;

#[path = "support/palettes.rs"]
mod palettes;

const CONTEXT: &str = include_str!("../shaders/wgsl/stain/context.wgsl");
const EDGE_LINE: &str = include_str!("../shaders/wgsl/frag/post/edge_line.wgsl");
const TINT: &str = include_str!("../shaders/wgsl/frag/post/fallback_tint.wgsl");
const PENDING: &str = include_str!("../shaders/wgsl/frag/post/pending_hatch.wgsl");
const S_STATE: &str = include_str!("../shaders/wgsl/frag/debug/s_state.wgsl");

// ── The stain's quad and tile lanes (RQ-238) ────────────────────────────────────────────────────────────────────────

/// The members of the struct `name` in `text`, each `(name, type)`, in order.
fn struct_members(text: &str, name: &str) -> Vec<(String, String)> {
    let start = text
        .find(&format!("struct {name} {{"))
        .unwrap_or_else(|| panic!("no struct {name}"));
    let body = &text[start..];
    let body = &body[body.find('{').expect("a body") + 1..body.find('}').expect("a body's end")];
    body.lines()
        .filter_map(|l| l.trim().strip_suffix(','))
        .filter_map(|l| l.split_once(':'))
        .map(|(n, t)| (n.trim().to_owned(), t.trim().to_owned()))
        .collect()
}

/// Checks that `context`'s `CtxQuad` is the harness's quad lane, member for member, name and type, in order.
fn check_ctx_quad(context: &str) {
    let lane: Vec<(String, String)> = bind::quad_lane("q")
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|m| (m.name, m.ty))
        .collect();
    assert_eq!(
        struct_members(context, "CtxQuad"),
        lane,
        "the stain's CtxQuad is not the quad lane"
    );
    assert_eq!(
        struct_members(context, "CtxTile"),
        vec![("uv".to_owned(), "vec2<f32>".to_owned())],
        "the stain's CtxTile"
    );
}

#[test]
fn structural_ctx_quad_is_the_quad_lane() {
    check_ctx_quad(CONTEXT);
    // The harness's lanes and the stain's are one list: the harness's quad lane is `quad_lane` over `rc.quad`.
    let lanes = bind::lanes().unwrap_or_else(|e| panic!("{e}"));
    let quad = lanes
        .iter()
        .find(|l| l.name == "quad")
        .expect("the quad lane");
    assert_eq!(
        quad.members,
        bind::quad_lane("rc.quad").unwrap_or_else(|e| panic!("{e}")),
        "the harness's quad lane"
    );
}

negative_control!(
    structural_ctx_quad_is_the_quad_lane,
    "a CtxQuad with its depth as f32 is not the lane",
    expected = "the stain's CtxQuad is not the quad lane",
    check_ctx_quad(&CONTEXT.replace("    depth: u32,", "    depth: f32,"))
);

/// Checks that `module` fills each of the stain's quad lane's `RenderQuad` members from `quad_read` by `QUAD_LANE`'s
/// mapping, and its tile lane from the raster.
fn check_fills(module: &str) {
    assert!(
        module.contains("let q = quad_read(r.quad);"),
        "the quad read"
    );
    for (lane, field) in bind::QUAD_LANE {
        assert!(
            module.contains(&format!("quad.{lane} = q.{field};")),
            "ctx.quad.{lane} is not filled from RenderQuad's {field}"
        );
    }
    assert!(module.contains("quad.uv = r.quad_uv;"), "ctx.quad.uv");
    assert!(
        module.contains("let tile = CtxTile(r.tile_uv);"),
        "ctx.tile.uv"
    );
}

#[test]
fn structural_ctx_preset_module_fills_the_lanes() {
    check_fills(&bind::preset_module(""));
}

negative_control!(
    structural_ctx_preset_module_fills_the_lanes,
    "a module filling the state from the depth is refused",
    expected = "ctx.quad.state is not filled from RenderQuad's quad_state",
    check_fills(
        &bind::preset_module("")
            .replace("quad.state = q.quad_state;", "quad.state = q.quad_depth;")
    )
);

/// A colour that is white when every member of the quad and tile lanes holds the absence NaN, or its bits, and black
/// otherwise: the lanes' members from the quad lane, each tested by its type.
fn all_absent() -> String {
    let mut tests: Vec<String> = bind::quad_lane("q")
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|m| match m.ty.as_str() {
            "u32" => format!("ctx.quad.{} == CTX_ABSENT_BITS", m.name),
            "f32" => format!("is_absent_nan(ctx.quad.{})", m.name),
            _ => format!(
                "is_absent_nan(ctx.quad.{0}.x) && is_absent_nan(ctx.quad.{0}.y)",
                m.name
            ),
        })
        .collect();
    tests.push("is_absent_nan(ctx.tile.uv.x) && is_absent_nan(ctx.tile.uv.y)".to_owned());
    format!(
        "fn colour(ctx: Ctx) -> vec3<f32> {{ return select(vec3<f32>(0.0), vec3<f32>(1.0), {}); }}\n",
        tests.join(" && ")
    )
}

/// The zero source.
const ZERO: &str = "fn source(ctx: Ctx) -> Field { return Field(0.0, 0.0, 0.0, 0.0); }";

/// `colour` fed the zero source, into the pass-through combiner, then each built-in post of `posts`, then OUT.
fn stain(colour: &str, posts: &[&str]) -> Stain {
    let n = |kind, occupant, inputs: &[Option<usize>]| Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    let mut nodes = vec![
        n(Kind::Source, Occupant::Custom(ZERO.to_owned()), &[]),
        n(
            Kind::Colour,
            Occupant::Custom(colour.to_owned()),
            &[Some(0)],
        ),
        n(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
    ];
    for p in posts {
        let at = nodes.len();
        nodes.push(n(
            Kind::Post,
            Occupant::BuiltIn((*p).into()),
            &[Some(at - 1)],
        ));
    }
    let at = nodes.len();
    nodes.push(n(Kind::Out, Occupant::None, &[Some(at - 1)]));
    Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"))
}

/// The test entry: shades sample 0 through `shade_sample`, which places no quad, at the pixel; the colour's bits.
const ENTRY: &str = r"
@fragment
fn t_absent(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let rgb = shade_sample(0u, pos.xy, 0.25, vec3<f32>(1.0, 1.0, 1.0), ReadParams(0.01, 1e-6, 16u, 1000u));
    return vec4<u32>(bitcast<vec3<u32>>(rgb), 0u);
}
";

/// The colour `stain` draws through `shade_sample` at the pixel `(0.5, 0.5)`, over a fresh sample.
fn draw(h: &GpuHarness, stain: &Stain) -> Rgb {
    let source = assemble::assemble(stain, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"))
        .source
        + ENTRY;
    let kinds: [&[BindingKind]; 2] = [
        &[BindingKind::Uniform],
        &[BindingKind::Storage, BindingKind::Storage],
    ];
    let kernel = h
        .fragment(&source, "t_absent", &kinds, 1, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let uniforms = prelude::uniform_words(0);
    let state = simstate_words(&Default::default());
    let word = [0u32, 0, 0, 0];
    let pixels = kernel
        .draw(&[&[&uniforms], &[&state, &word]])
        .unwrap_or_else(|e| panic!("{e}"));
    [0, 1, 2].map(|k| f64::from(f32::from_bits(pixels[0][k])))
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= 1e-5)
}

/// Checks that through `shade_sample` every quad and tile lane member reads absent, the overlays pass that colour on,
/// and the state view draws the hatch.
fn check_absent(h: &GpuHarness, posts: &[&str]) {
    let white = draw(h, &stain(&all_absent(), posts));
    assert!(
        near(white, [1.0; 3]),
        "a lane member is not absent: {white:?}"
    );
    let hatch = draw(h, &stain(S_STATE, &[]));
    assert!(
        near(hatch, present::debug_invalid([0.5, 0.5])),
        "the state view of an absent quad is not the hatch: {hatch:?}"
    );
}

#[test]
fn structural_absent_lanes_read_the_absence_nan() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    check_absent(&h, &["fallback_tint", "pending_hatch"]);
}

negative_control!(
    structural_absent_lanes_read_the_absence_nan,
    "a post drawing over every pixel does not pass the absent lane's colour on",
    expected = "a lane member is not absent",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let black = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }";
        let white = draw(&h, &stain(black, &["fallback_tint"]));
        assert!(
            near(white, [1.0; 3]),
            "a lane member is not absent: {white:?}"
        );
    }
);

// ── The styling (REQ-TOOL-124) ──────────────────────────────────────────────────────────────────────────────────────

/// Checks that the WGSL `tint` and `pending` hold the mirror's styling constants.
fn check_constants(tint: &str, pending: &str) {
    let [r, g, b] = mirror::FALLBACK_TINT_SRGB8;
    for (text, want, what) in [
        (
            tint,
            format!("const FALLBACK_TINT_SRGB8: vec3<f32> = vec3<f32>({r}.0, {g}.0, {b}.0);"),
            "the tint's colour",
        ),
        (
            tint,
            format!(
                "const FALLBACK_OPACITY: f32 = {:?};",
                mirror::FALLBACK_OPACITY
            ),
            "the tint's opacity",
        ),
        (
            pending,
            {
                let [r, g, b] = mirror::PENDING_SRGB8;
                format!("const PENDING_SRGB8: vec3<f32> = vec3<f32>({r}.0, {g}.0, {b}.0);")
            },
            "the hatch's colour",
        ),
        (
            pending,
            format!("const PENDING_PERIOD: i32 = {};", mirror::PENDING_PERIOD),
            "the hatch's period",
        ),
        (
            pending,
            format!("const PENDING_LINE: i32 = {};", mirror::PENDING_LINE),
            "the hatch's line width",
        ),
        (
            pending,
            format!("ctx.quad.state != {}u", mirror::PENDING),
            "the pending state",
        ),
    ] {
        assert!(text.contains(&want), "{what} is not the mirror's: {want}");
    }
}

#[test]
fn structural_styling_wgsl_is_the_mirrors() {
    check_constants(TINT, PENDING);
}

negative_control!(
    structural_styling_wgsl_is_the_mirrors,
    "a tint at opacity 0.5 is not the mirror's",
    expected = "the tint's opacity is not the mirror's",
    check_constants(&TINT.replace("= 0.4;", "= 0.5;"), PENDING)
);

/// The OKLab distance between two linear colours.
fn distance(a: Rgb, b: Rgb) -> f64 {
    let (p, q) = (present::linear_to_oklab(a), present::linear_to_oklab(b));
    (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f64>().sqrt()
}

/// The OKLab distance from `c` to the nearest palette entry, and its name.
fn nearest(c: Rgb, palettes: &[(String, Rgb)]) -> (f64, String) {
    palettes
        .iter()
        .map(|(name, p)| (distance(c, *p), name.clone()))
        .fold((f64::INFINITY, String::new()), |a, b| {
            if b.0 < a.0 {
                b
            } else {
                a
            }
        })
}

/// REQ-COL-055's measurement rerun with `colours` (REQ-TOOL-124): each farther in OKLab from every palette entry than
/// R-16's flat magenta, `#FF00FF`, is from its nearest, and none either of `debug_invalid`'s colours; the distances
/// are printed, the PR's evidence.
fn check_no_collision(colours: &[(&str, [u8; 3])]) {
    let palettes = palettes::entries();
    let (magenta, magenta_near) = nearest(palettes::hex(0xFF00FF), &palettes);
    eprintln!("flat magenta #FF00FF: {magenta:.3} from {magenta_near}");
    let hatch = prelude::hatch().colours.map(present::srgb8);
    for (what, c) in colours {
        let rgb = present::srgb8(*c);
        let (d, near) = nearest(rgb, &palettes);
        let from_hatch = hatch.map(|h| distance(rgb, h));
        eprintln!(
            "{what} #{:02X}{:02X}{:02X}: {d:.3} from {near}; {:.3} and {:.3} from the hatch's violet and cyan",
            c[0], c[1], c[2], from_hatch[0], from_hatch[1]
        );
        assert!(
            d > magenta,
            "{what} is {d:.3} from {near}, no farther than the flat magenta's {magenta:.3}"
        );
        assert!(
            from_hatch.iter().all(|&h| h > magenta),
            "{what} is one of debug_invalid's colours"
        );
    }
}

#[test]
fn structural_styling_collides_with_no_palette_entry() {
    check_no_collision(&[
        ("the pending hatch", mirror::PENDING_SRGB8),
        ("the fallback tint", mirror::FALLBACK_TINT_SRGB8),
    ]);
}

negative_control!(
    structural_styling_collides_with_no_palette_entry,
    "a tint of body-1 escape's magenta collides",
    expected = "no farther than the flat magenta's",
    check_no_collision(&[("the fallback tint", [0xE0, 0x34, 0xC6])])
);

/// Checks that the pending hatch's pattern is not `debug_invalid`'s: its lines run along `x − y`, so a step of
/// `(1, 1)` keeps a pixel on or off a line, where the invalid hatch's stripes run along `x + y`, and a step of `(1, −1)`
/// keeps its stripe; and the pending pattern leaves most pixels to the colour beneath, the invalid hatch none.
fn check_pattern(on: fn([f64; 2]) -> bool) {
    let (mut along, mut across, mut lines) = (true, true, 0);
    for y in 1..33 {
        for x in 0..32 {
            let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            along &= on(p) == on([p[0] + 1.0, p[1] + 1.0]);
            across &= present::hatch_stripe(p) == present::hatch_stripe([p[0] + 1.0, p[1] - 1.0]);
            lines += u32::from(on(p));
        }
    }
    assert!(along, "the pending lines do not run along x − y");
    assert!(across, "the invalid hatch's stripes do not run along x + y");
    assert_eq!(
        lines,
        32 * 32 * mirror::PENDING_LINE as u32 / mirror::PENDING_PERIOD as u32,
        "the pending lines' share of the pixels"
    );
}

#[test]
fn structural_styling_pattern_is_not_the_invalid_hatch() {
    check_pattern(mirror::pending_on);
}

negative_control!(
    structural_styling_pattern_is_not_the_invalid_hatch,
    "a pending pattern along the invalid hatch's x + y fails",
    expected = "the pending lines do not run along x − y",
    check_pattern(|p| {
        let s = p[0].floor() as i64 + p[1].floor() as i64;
        s.rem_euclid(8) < 2
    })
);

// ── edge_line (REQ-RENDER-024) ──────────────────────────────────────────────────────────────────────────────────────

/// `edge_line.wgsl`'s pixel size and §12.1's `fwidth(d)`, each drawn on a 4 × 1 target where an 8 px cell's edge
/// falls inside a 2 × 2 block (cells from x = 1): the coverage at pixels 0 to 3, one channel each.
const EDGE_ENTRY: &str = r"
fn spec_edge_line(uv: vec2<f32>, width_px: f32) -> f32 {
    let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let w = fwidth(d);
    return 1.0 - smoothstep(0.0, w * width_px, d);
}

@fragment
fn t_edge(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    // Cells 8 px wide from x = 1, so the edge at x = 1 splits the block of pixels 0 and 1; v mid-cell.
    let uv = vec2<f32>(fract((pos.x - 1.0) / 8.0), 0.5 + pos.y / 64.0);
    return vec4<u32>(bitcast<u32>(edge_line(uv, 2.0)), bitcast<u32>(spec_edge_line(uv, 2.0)), 0u, 0u);
}
";

/// The coverages at pixels 0 to 3 of [`EDGE_ENTRY`]'s line, built over `edge`'s functions: `(this, §12.1's)`.
fn edge_coverages(h: &GpuHarness, edge: &str) -> Vec<(f32, f32)> {
    let functions = &edge[..edge.find("fn post(").expect("the post")];
    let source = format!("{functions}{EDGE_ENTRY}");
    let kernel = h
        .fragment(&source, "t_edge", &[], 4, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    kernel
        .draw(&[])
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| (f32::from_bits(p[0]), f32::from_bits(p[1])))
        .collect()
}

/// Checks that across the cell edge between pixels 0 and 1, which share a 2 × 2 block, each pixel half a pixel from
/// the edge is covered as `smoothstep` places it, 1 − smoothstep(0, 2, ½), where §12.1's `fwidth(d)` reads 0.
fn check_straddle(got: &[(f32, f32)]) {
    let want = 1.0 - mirror::smoothstep(0.0, 2.0, 0.5);
    for (x, (cov, _)) in got.iter().enumerate().take(2) {
        assert!(
            (f64::from(*cov) - want).abs() < 1e-5,
            "pixel {x}, ½ px from the edge, is covered {cov}, not {want}"
        );
    }
    eprintln!(
        "§12.1's fwidth(d) at the straddled edge: pixels 0 and 1 covered {} and {}",
        got[0].1, got[1].1
    );
}

#[test]
fn structural_edge_line_reads_the_pixel_across_a_cell_edge() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    check_straddle(&edge_coverages(&h, EDGE_LINE));
}

negative_control!(
    structural_edge_line_reads_the_pixel_across_a_cell_edge,
    "§12.1's fwidth(d) leaves the straddled edge's pixels uncovered",
    expected = "½ px from the edge, is covered",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let got = edge_coverages(&h, EDGE_LINE);
        let spec: Vec<(f32, f32)> = got.iter().map(|&(_, s)| (s, s)).collect();
        check_straddle(&spec);
    }
);

// ── The s_impurity ramp ─────────────────────────────────────────────────────────────────────────────────────────────

/// Checks that `stops`, the prelude's magma as the CPU mirror reads it, is the published table the palette measurement
/// reads (`tests/data/lut/magma.txt`), stop for stop.
fn check_magma(stops: &[Rgb]) {
    let published = palettes::table_stops(include_str!("data/lut/magma.txt"));
    assert_eq!(published.len(), 256, "the published table's stops");
    assert_eq!(
        stops,
        &published[..],
        "the prelude's magma is not the published table"
    );
}

#[test]
fn structural_impurity_ramp_is_the_published_magma() {
    check_magma(&present::magma_stops());
    let [first, last] = [0.0, 1.0].map(present::ramp_magma);
    assert_eq!(
        first,
        present::srgb_to_linear3([0.001462, 0.000466, 0.013866]),
        "ramp_magma(0)"
    );
    assert_eq!(
        last,
        present::srgb_to_linear3([0.987053, 0.991438, 0.749504]),
        "ramp_magma(1)"
    );
}

negative_control!(
    structural_impurity_ramp_is_the_published_magma,
    "viridis read as magma is not the published magma",
    expected = "the prelude's magma is not the published table",
    check_magma(&present::viridis_stops())
);

// ── The CPU mirror, against values worked by hand ───────────────────────────────────────────────────────────────────

/// A quad whose members are all distinct: depth 3, pending, an ancestor gap of 2.
fn meta() -> mirror::QuadMeta {
    mirror::QuadMeta {
        depth: 3,
        state: 1,
        coherence: 0.25,
        impurity: 0.5,
        spread: 0.75,
        suspect_frac: 0.125,
        priority: -1.5,
        ancestor_gap: 2,
        cache_age: 9,
        dominant_outcome: 4,
    }
}

/// `meta()`'s words in §3.7a's order.
const META_WORDS: [u32; 10] = [
    3,
    1,
    0x3e80_0000, // 0.25
    0x3f00_0000, // 0.5
    0x3f40_0000, // 0.75
    0x3e00_0000, // 0.125
    0xbfc0_0000, // −1.5
    2,
    9,
    4,
];

/// Checks the mirror's reads and arithmetic against `words`, which must be `meta()`'s.
fn check_mirror(words: &[u32]) {
    let q = meta();
    assert_eq!(
        mirror::QuadMeta::from_words(words),
        q,
        "the words read back"
    );
    let short = mirror::QuadMeta::from_words(&words[..2]);
    assert_eq!(
        (short.depth, short.state),
        (3, 1),
        "a short record's first members"
    );
    assert_eq!(
        short.ancestor_gap,
        mirror::ABSENT_BITS,
        "a member past a short record"
    );
    assert!(
        short.coherence.is_nan(),
        "a float member past a short record"
    );

    let want = [
        ("s_depth", (3.0, false, Some(0.0), None)),
        ("s_coherence", (0.25, false, Some(0.0), Some(1.0))),
        ("s_impurity", (0.5, false, Some(0.0), Some(1.0))),
        ("s_spread", (0.75, false, Some(0.0), Some(1.0))),
        ("s_suspect", (0.125, false, Some(0.0), Some(1.0))),
        ("s_priority", (-1.5, false, None, None)),
        ("s_cache_age", (9.0, false, Some(0.0), None)),
        ("s_ancestor_gap", (2.0, false, Some(0.0), None)),
    ];
    for (id, w) in want {
        assert_eq!(mirror::scalar(id, &q), Some(w), "{id}'s scalar");
    }
    assert_eq!(
        mirror::scalar("s_state", &q),
        None,
        "s_state is categorical"
    );
    assert_eq!(mirror::scalar("s_nothing", &q), None, "no view");
    let absent = mirror::QuadMeta::from_words(&[]);
    assert!(
        mirror::scalar("s_depth", &absent).is_some_and(|s| s.1),
        "an absent u32 member"
    );
    assert!(
        mirror::scalar("s_impurity", &absent).is_some_and(|s| s.1),
        "an absent f32 member"
    );
    let auto: Vec<&str> = mirror::VIEWS
        .into_iter()
        .filter(|id| mirror::range_auto(id))
        .collect();
    assert_eq!(
        auto,
        ["s_depth", "s_priority", "s_cache_age", "s_ancestor_gap"],
        "RANGE_AUTO"
    );

    let xy = [5.0, 2.0];
    let view = |id, ens, auto, range| mirror::view(id, &q, xy, ens, auto, range);
    assert_eq!(
        view("s_state", true, false, [0.0, 1.0]),
        Some(present::dbg_cat(1, 5)),
        "s_state"
    );
    let s_state_absent = mirror::view("s_state", &absent, xy, true, false, [0.0, 1.0]);
    assert_eq!(
        s_state_absent,
        Some(present::debug_invalid(xy)),
        "s_state absent"
    );
    assert_eq!(view("s_nothing", true, false, [0.0, 1.0]), None, "no view");
    // Fixed: depth 3 on [0, measured high 5] is 0.6; measured: on [3, 5] it is 0.
    assert_eq!(
        view("s_depth", true, false, [3.0, 5.0]),
        Some(present::ramp_viridis(0.6)),
        "s_depth fixed"
    );
    assert_eq!(
        view("s_depth", true, true, [3.0, 5.0]),
        Some(present::ramp_viridis(0.0)),
        "s_depth measured"
    );
    assert_eq!(
        view("s_impurity", true, false, [0.0, 1.0]),
        Some(present::ramp_magma(0.5)),
        "s_impurity"
    );
    assert_eq!(
        view("s_spread", true, false, [0.0, 1.0]),
        Some(present::ramp_viridis(0.75)),
        "s_spread"
    );
    assert_eq!(
        view("s_spread", false, false, [0.0, 1.0]),
        Some(present::debug_invalid(xy)),
        "no ensemble"
    );
    let absent_depth = mirror::view("s_depth", &absent, xy, true, false, [0.0, 1.0]);
    assert_eq!(
        absent_depth,
        Some(present::debug_invalid(xy)),
        "s_depth absent"
    );

    assert_eq!(
        mirror::smoothstep(0.0, 2.0, 1.0),
        0.5,
        "smoothstep's middle"
    );
    assert_eq!(
        mirror::smoothstep(0.0, 4.0, 1.0),
        0.15625,
        "smoothstep at a quarter"
    );
    assert_eq!(
        mirror::smoothstep(1.0, 3.0, 2.5),
        0.84375,
        "smoothstep at three quarters, offset"
    );
    assert_eq!(mirror::smoothstep(0.0, 2.0, -1.0), 0.0, "smoothstep below");
    assert_eq!(mirror::smoothstep(0.0, 2.0, 3.0), 1.0, "smoothstep above");
    assert_eq!(
        mirror::edge_line([0.5, 0.015625], 64.0, 2.0),
        0.5,
        "1 px from the bottom edge"
    );
    assert_eq!(
        mirror::edge_line([0.984375, 0.5], 64.0, 2.0),
        0.5,
        "1 px from the right edge"
    );
    assert_eq!(
        mirror::edge_line([0.5, 0.5], 64.0, 2.0),
        0.0,
        "the cell's middle"
    );
    assert_eq!(
        mirror::edge_line([0.0078125, 0.5], 64.0, 2.0),
        0.84375,
        "half a pixel from the left edge"
    );
    assert_eq!(
        mirror::mix([1.0, 0.0, 2.0], [3.0, 2.0, 0.0], 0.25),
        [1.5, 0.5, 1.5],
        "mix"
    );

    let rgb = [0.25, 0.5, 0.125];
    assert_eq!(mirror::fallback_tint(rgb, 0), rgb, "no gap");
    assert_eq!(
        mirror::fallback_tint(rgb, mirror::ABSENT_BITS),
        rgb,
        "an absent gap"
    );
    let pink = present::srgb8([0xff, 0x69, 0xff]);
    let tinted = [0, 1, 2].map(|c| rgb[c] * 0.6 + pink[c] * 0.4);
    let got = mirror::fallback_tint(rgb, 2);
    assert!(
        (0..3).all(|c| (got[c] - tinted[c]).abs() < 1e-12),
        "a gap of 2: {got:?}, not {tinted:?}"
    );
    let on: Vec<bool> = [
        [0.0, 0.0],
        [1.5, 0.0],
        [2.0, 0.0],
        [0.0, 1.0],
        [0.0, 7.0],
        [7.9, 0.2],
        [8.0, 0.0],
        [-6.0, 0.0],
    ]
    .into_iter()
    .map(mirror::pending_on)
    .collect();
    assert_eq!(
        on,
        [true, true, false, false, true, false, true, false],
        "the pending hatch's lines"
    );
    let blue = present::srgb8([0, 0, 0xff]);
    assert_eq!(
        mirror::pending_hatch(rgb, 1, [0.0, 0.0]),
        blue,
        "a pending line pixel"
    );
    assert_eq!(
        mirror::pending_hatch(rgb, 1, [3.0, 0.0]),
        rgb,
        "a pending pixel between lines"
    );
    assert_eq!(
        mirror::pending_hatch(rgb, 0, [0.0, 0.0]),
        rgb,
        "a loaded quad"
    );
}

#[test]
fn structural_mirror_known_answers() {
    check_mirror(&META_WORDS);
}

negative_control!(
    structural_mirror_known_answers,
    "words with the members swapped do not read back as the quad",
    expected = "the words read back",
    {
        let mut words = META_WORDS;
        words.swap(7, 8);
        check_mirror(&words);
    }
);
