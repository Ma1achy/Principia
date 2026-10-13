//! QA tests for TASK-M1-13, written from the requirements it closes, rendered on the GPU through the render harness
//! and measured here, never against `render::structural` (the occupants' CPU mirror):
//!
//! - REQ-RENDER-024 (render_gui_spec §12.1): the boundary overlay's line has one width in pixels, its `width` param,
//!   at quad sizes 16, 32 and 64 px, on depth-3 and depth-20 sets, at widths 2 and 3 px; also where a cell edge falls
//!   on an odd pixel, so a 2 × 2 pixel block straddles it (an odd quad size, an odd tile size), at the quad and the tile
//!   level; antialiased (a partial-coverage pixel). The thresholded-uv control (`u < 0.01`) and §12.1's printed
//!   `fwidth(d)` form at an odd edge each fail the same check (the negative controls).
//! - REQ-TOOL-026 / REQ-TOOL-124 (debug_tooling_plan §F): each structural view colours quads alike exactly where the
//!   CPU struct's value is alike; the spread view without an ensemble draws `debug_invalid`'s hatch (render contract
//!   Part 5: `((x + y) >> 2) & 1`, violet `#9B00FF` / cyan `#48FFFF`) and with one draws flat quads; the pending hatch
//!   is `#0000FF` on `(x − y) mod 8 < 2` in pending quads only, the colour beneath elsewhere; the fallback tint mixes
//!   0.4 toward `#FF69FF` in linear RGB in quads whose `ancestor_gap > 0` only.
//!
//! Each test registers its negative control (R-176).

#[cfg(feature = "controls")]
use engine::stain::Occupant;
#[cfg(feature = "controls")]
use engine::structural::overlay;
use engine::structural::Level;
use engine::synthetic::{structural_record, QuadRecord, Synthetic};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::structural_scene::{edge, scene, Post, StructuralScene};

type Rgb = [f64; 3];

/// Per-channel tolerance of a GPU render against a value computed here: `golden_scene`'s and the structural tests'
/// render-against-CPU tolerance, linear RGB.
const TOL: f64 = 1e-5;

/// The measurement window, in pixels on each side of the edge: wider than the widest line measured (3 px), so it
/// holds the whole line and its antialiasing, and narrower than the distance to any other edge in the scenes.
const HALF_WINDOW: u32 = 6;

/// The tolerance on a measured width, in pixels: [`TOL`] per pixel summed over the window.
const WIDTH_TOL: f64 = 2.0 * HALF_WINDOW as f64 * TOL;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn render(h: &GpuHarness, s: &StructuralScene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

/// A 2 × 2-quad scene over the flat mid-grey, each quad `n × n` tiles of `tile_px`, every quad at `depth` with its
/// deep_zoom frame, the whole target shown, with `posts` drawn: the template scene `quad_boundaries` with its grid,
/// set and posts replaced.
fn grid_scene(depth: u32, n: u32, tile_px: u32, posts: Vec<Post>) -> StructuralScene {
    let grid = Grid::new([2, 2], n, 0, tile_px).unwrap_or_else(|e| panic!("{e}"));
    let mut s = scene("quad_boundaries").unwrap_or_else(|e| panic!("{e}"));
    s.set = Synthetic::structural(grid, &[depth]).unwrap_or_else(|e| panic!("{e}"));
    s.context.grid = grid;
    s.context.valid_sample_count = n * n;
    s.view = None;
    s.posts = posts;
    s.crop = None;
    s
}

/// White edge lines at `level`, `width` px wide at the quad level and `tile_width` at the tile level, opaque.
fn white(level: Level, width: f64, tile_width: f64) -> Post {
    let mut e = edge(level);
    e.width = width;
    e.tile_width = tile_width;
    e.opacity = 1.0;
    e.tile_opacity = 1.0;
    e.colour = [1.0; 3];
    Post::Edge(e)
}

/// The measured width, in pixels, of the vertical line at `edge_x` crossing row `row` of `image` (`width` px wide):
/// the sum over the window of each pixel's coverage, how far its red channel lies from the background's toward
/// white's. The background is read off the image at `(bg_x, row)`, a pixel no line reaches. Also whether some pixel
/// is partly covered (antialiased).
fn measure(image: &[Rgb], width: u32, row: u32, edge_x: u32, bg_x: u32) -> (f64, bool) {
    let at = |x: u32| image[(row * width + x) as usize][0];
    let bg = at(bg_x);
    let coverage: Vec<f64> = (edge_x - HALF_WINDOW..edge_x + HALF_WINDOW)
        .map(|x| (at(x) - bg) / (1.0 - bg))
        .collect();
    let partial = coverage.iter().any(|c| *c > 0.05 && *c < 0.95);
    (coverage.iter().sum(), partial)
}

/// One width case: a label, the scene, the edge's x, a row crossing only that vertical edge, a background pixel on
/// that row, and the expected width.
struct Case {
    label: String,
    scene: StructuralScene,
    edge_x: u32,
    row: u32,
    bg_x: u32,
    want: f64,
}

/// A quad-level case: the inner vertical quad edge of a 2 × 2 grid of `quad_px` quads (`n` tiles of `quad_px / n`),
/// measured on the middle row of the top quads, `posts` drawn.
fn quad_case(depth: u32, n: u32, tile_px: u32, post: Post, want: f64, label: &str) -> Case {
    let quad_px = n * tile_px;
    Case {
        label: format!("{label} (depth {depth}, quad {quad_px} px)"),
        scene: grid_scene(depth, n, tile_px, vec![post]),
        edge_x: quad_px,
        row: quad_px / 2,
        bg_x: quad_px / 2,
        want,
    }
}

/// Checks each case's measured width against its expected width within [`WIDTH_TOL`], and that each is antialiased.
fn check_widths(h: &GpuHarness, cases: &[Case]) {
    for c in cases {
        let (width, _) = c.scene.size();
        let image = render(h, &c.scene);
        let (measured, partial) = measure(&image, width, c.row, c.edge_x, c.bg_x);
        assert!(
            (measured - c.want).abs() <= WIDTH_TOL,
            "{}: the line measures {measured} px wide, not {} px",
            c.label,
            c.want
        );
        assert!(
            partial,
            "{}: no pixel of the line is partly covered",
            c.label
        );
    }
}

/// REQ-RENDER-024's cases: depths 3 and 20, quad sizes 16, 32 and 64 px, widths 2 and 3 px.
fn constant_width_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for depth in [3, 20] {
        for quad_px in [16, 32, 64] {
            for w in [2.0, 3.0] {
                cases.push(quad_case(
                    depth,
                    4,
                    quad_px / 4,
                    white(Level::Quad, w, 1.0),
                    w,
                    &format!("edge_line, width {w}"),
                ));
            }
        }
    }
    cases
}

#[test]
fn qa_boundary_width_is_constant_across_quad_size_and_depth() {
    check_widths(&gpu(), &constant_width_cases());
}

/// The thresholded-uv control, render_gui_spec §12.1's `u < 0.01`, as a custom post.
#[cfg(feature = "controls")]
const THRESHOLD: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {\n    \
    let uv = ctx.quad.uv;\n    \
    let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));\n    \
    return select(rgb, vec3<f32>(1.0), d < 0.01);\n}\n";

/// Renders `source`, a custom post, over the 2 × 2 grid, and measures its line as [`check_widths`] does, against
/// `want`.
#[cfg(feature = "controls")]
fn check_custom_width(h: &GpuHarness, depth: u32, n: u32, tile_px: u32, source: &str, want: f64) {
    let s = grid_scene(depth, n, tile_px, vec![]);
    let mut g = s.graph().unwrap_or_else(|e| panic!("{e}"));
    overlay(&mut g, Occupant::Custom(source.to_owned())).unwrap_or_else(|e| panic!("{e}"));
    let canonical = g.canonical().unwrap_or_else(|e| panic!("{e}"));
    let full = validation::golden_scene::render_stain(
        h.device(),
        h.queue(),
        canonical.stain(),
        &s.set,
        &s.context,
        &|node, name| canonical.nodes.get(node)?.params.get(name).cloned(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let image: Vec<Rgb> = full
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect();
    let (width, _) = s.context.grid.target();
    let quad_px = n * tile_px;
    let (measured, _) = measure(&image, width, quad_px / 2, quad_px, quad_px / 2);
    assert!(
        (measured - want).abs() <= WIDTH_TOL,
        "custom post at quad {quad_px} px: the line measures {measured} px wide, not {want} px"
    );
}

negative_control!(
    qa_boundary_width_is_constant_across_quad_size_and_depth,
    "the thresholded-uv control (u < 0.01) does not keep one width across quad sizes",
    expected = "the line measures",
    {
        let h = gpu();
        // At 64 px the threshold catches one pixel on each side of the edge (0.5 / 64 < 0.01 < 1.5 / 64): a 2 px
        // line. One width would draw 2 px at 16 px too, where it catches none (0.5 / 16 > 0.01).
        check_custom_width(&h, 3, 4, 16, THRESHOLD, 2.0);
        check_custom_width(&h, 3, 4, 4, THRESHOLD, 2.0);
    }
);

/// The width cases where a cell edge falls on an odd pixel, so the 2 × 2 pixel block of the derivatives straddles
/// it: an odd quad (one tile of 17 px; edge at x = 17) at the quad level, and odd tiles (5 px; the tile edge at
/// x = 5) at the tile level, at depths 3 and 20.
fn straddle_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for depth in [3, 20] {
        cases.push(quad_case(
            depth,
            1,
            17,
            white(Level::Quad, 2.0, 1.0),
            2.0,
            "edge_line at an odd quad edge",
        ));
        // Tile level, tiles of 5 px, 4 per quad: the tile edges at x = 5 (odd) and x = 10 (even), measured on row 2,
        // the middle row of the top tile row, with a 2 px tile line.
        for edge_x in [5, 10] {
            cases.push(Case {
                label: format!("edge_line, tile level, tile edge at x = {edge_x} (depth {depth})"),
                scene: grid_scene(depth, 4, 5, vec![white(Level::Tile, 2.0, 2.0)]),
                edge_x,
                row: 2,
                bg_x: 2,
                want: 2.0,
            });
        }
    }
    cases
}

/// Measures a tile case over a window of ±2 px only (tile edges are 5 px apart, a 2 px line reaching 1 px each side
/// and its last sample 1.5 px out), the background read at the tile's centre, 2.5 px from every tile edge.
fn check_tile_case(h: &GpuHarness, c: &Case) {
    let (width, _) = c.scene.size();
    let image = render(h, &c.scene);
    let at = |x: u32| image[(c.row * width + x) as usize][0];
    let bg = at(c.bg_x);
    let coverage: Vec<f64> = (c.edge_x - 2..c.edge_x + 2)
        .map(|x| (at(x) - bg) / (1.0 - bg))
        .collect();
    let measured: f64 = coverage.iter().sum();
    assert!(
        (measured - c.want).abs() <= 4.0 * TOL,
        "{}: the line measures {measured} px wide, not {} px",
        c.label,
        c.want
    );
    assert!(
        coverage.iter().any(|v| *v > 0.05 && *v < 0.95),
        "{}: no pixel of the line is partly covered",
        c.label
    );
}

fn check_straddles(h: &GpuHarness, cases: &[Case]) {
    for c in cases {
        if c.label.contains("tile level") {
            check_tile_case(h, c);
        } else {
            check_widths(h, std::slice::from_ref(c));
        }
    }
}

#[test]
fn qa_boundary_width_holds_where_a_pixel_block_straddles_the_edge() {
    check_straddles(&gpu(), &straddle_cases());
}

/// render_gui_spec §12.1's printed `edge_line`, `fwidth(d)` of the edge distance, as a custom post.
#[cfg(feature = "controls")]
const SPEC_FWIDTH_D: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {\n    \
    let uv = ctx.quad.uv;\n    \
    let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));\n    \
    let w = fwidth(d);\n    \
    let a = 1.0 - smoothstep(0.0, w * 2.0, d);\n    \
    return mix(rgb, vec3<f32>(1.0), a);\n}\n";

negative_control!(
    qa_boundary_width_holds_where_a_pixel_block_straddles_the_edge,
    "§12.1's printed fwidth(d) form loses the line at an odd quad edge",
    expected = "the line measures",
    check_custom_width(&gpu(), 3, 1, 17, SPEC_FWIDTH_D, 2.0)
);

// ---- REQ-TOOL-026 / REQ-TOOL-124 -------------------------------------------------------------------------------

/// IEC 61966-2-1's sRGB decode, of an 8-bit value.
fn decode8(v: u8) -> f64 {
    let c = f64::from(v) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn hex(rgb: [u8; 3]) -> Rgb {
    rgb.map(decode8)
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// A named structural scene, rendered, and its width.
fn shown(h: &GpuHarness, name: &str) -> (StructuralScene, Vec<Rgb>, u32) {
    let s = scene(name).unwrap_or_else(|e| panic!("{e}"));
    let image = render(h, &s);
    let width = s.size().0;
    (s, image, width)
}

/// The value a view shows, from the CPU record and depth of quad `q` (debug_tooling_plan §F's table).
fn value(view: &str, r: &QuadRecord, depth: u32) -> f64 {
    match view {
        "s_depth" => f64::from(depth),
        "s_state" => f64::from(r.state),
        "s_coherence" => f64::from(r.coherence),
        "s_impurity" => f64::from(r.impurity),
        "s_spread_ensemble" => f64::from(r.spread),
        "s_suspect" => f64::from(r.suspect),
        "s_priority" => f64::from(r.priority),
        "s_cache_age" => f64::from(r.cache_age),
        "s_ancestor_gap" => f64::from(r.ancestor_gap),
        _ => panic!("no value for {view}"),
    }
}

/// The colour at the centre of each quad of a rendered scene, and the quad's depth as its CPU word holds it.
fn quad_centres(s: &StructuralScene, image: &[Rgb], width: u32) -> Vec<(u32, Rgb)> {
    let grid = s.context.grid;
    let side = grid.n * grid.tile_px;
    let (_, height) = grid.target();
    let mut out = vec![(0, [0.0; 3]); grid.quad_count() as usize];
    for qy in 0..grid.quads[1] {
        for qx in 0..grid.quads[0] {
            let x = qx * side + side / 2;
            let y = height - (qy * side + side / 2) - 1;
            let q = grid.cell(x, y).quad;
            let depth = s.quad(q).depth;
            out[q as usize] = (depth, image[(y * width + x) as usize]);
        }
    }
    out
}

/// Checks that view `view`'s scene colours two quads alike iff the CPU records give them one value.
fn check_view_reads_struct(h: &GpuHarness, view: &str, records: &dyn Fn(u32) -> QuadRecord) {
    let (s, image, width) = shown(h, view);
    let centres = quad_centres(&s, &image, width);
    for (a, (da, ca)) in centres.iter().enumerate() {
        for (b, (db, cb)) in centres.iter().enumerate().skip(a + 1) {
            let (va, vb) = (
                value(view, &records(a as u32), *da),
                value(view, &records(b as u32), *db),
            );
            assert_eq!(
                va == vb,
                near(*ca, *cb),
                "`{view}`: quads {a} and {b} hold {va} and {vb} but are coloured {ca:?} and {cb:?}"
            );
        }
    }
}

const VIEWS: [&str; 9] = [
    "s_depth",
    "s_state",
    "s_coherence",
    "s_impurity",
    "s_spread_ensemble",
    "s_suspect",
    "s_priority",
    "s_cache_age",
    "s_ancestor_gap",
];

#[test]
fn qa_structural_views_colour_by_the_cpu_struct() {
    let h = gpu();
    for view in VIEWS {
        check_view_reads_struct(&h, view, &structural_record);
    }
}

negative_control!(
    qa_structural_views_colour_by_the_cpu_struct,
    "the views checked against records whose two quads' values were swapped for a third's fail",
    expected = "but are coloured",
    {
        let h = gpu();
        // Quad 1 given quad 0's record: the image shows quad 1's own value, which is not quad 0's.
        check_view_reads_struct(&h, "s_impurity", &|q| {
            structural_record(if q == 1 { 0 } else { q })
        });
    }
);

/// `debug_invalid`'s hatch colour at pixel `(x, y)` (render contract Part 5): violet `#9B00FF` where
/// `((x + y) >> 2) & 1` is 0, cyan `#48FFFF` where it is 1.
fn invalid_at(x: u32, y: u32) -> Rgb {
    if ((x + y) >> 2) & 1 == 0 {
        hex([0x9B, 0x00, 0xFF])
    } else {
        hex([0x48, 0xFF, 0xFF])
    }
}

/// Checks that the spread scene `name` draws `debug_invalid`'s hatch at every pixel (`hatched`) or at none.
fn check_spread(h: &GpuHarness, name: &str, hatched: bool) {
    let (s, image, width) = shown(h, name);
    let (_, height) = s.size();
    for y in 0..height {
        for x in 0..width {
            let got = image[(y * width + x) as usize];
            assert_eq!(
                near(got, invalid_at(x, y)),
                hatched,
                "`{name}` pixel ({x}, {y}) is {got:?}; hatched should be {hatched}"
            );
        }
    }
}

#[test]
fn qa_spread_present_iff_ensemble() {
    let h = gpu();
    check_spread(&h, "s_spread", true);
    check_spread(&h, "s_spread_ensemble", false);
}

negative_control!(
    qa_spread_present_iff_ensemble,
    "the spread with an ensemble checked as hatched fails",
    expected = "hatched should be true",
    check_spread(&gpu(), "s_spread_ensemble", true)
);

/// Checks the pending hatch and the fallback tint against debug_tooling_plan §F's definition (REQ-TOOL-124), over
/// the quad-state view rendered without them: in `pending_hatch`, a pending quad's pixel `(x, y)` is `#0000FF` iff
/// `(x − y) mod 8 < 2`, and every other pixel is the colour beneath; in `fallback_tint`, a quad with `ancestor_gap > 0`
/// is the colour beneath mixed 0.4 toward `#FF69FF`, linear RGB, and every other quad the colour beneath.
fn check_hatch_and_tint(h: &GpuHarness, hatch_scene: &str, tint_scene: &str) {
    let (base, beneath, width) = shown(h, "s_state");
    let (_, height) = base.size();
    let grid = base.context.grid;
    let blue = hex([0x00, 0x00, 0xFF]);
    let pink = hex([0xFF, 0x69, 0xFF]);
    let (_, hatched, _) = shown(h, hatch_scene);
    let (_, tinted, _) = shown(h, tint_scene);
    let mut on = 0;
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let q = grid.cell(x, y).quad;
            let r = base.quad(q);
            let under = beneath[i];
            let line = (i64::from(x) - i64::from(y)).rem_euclid(8) < 2;
            let want = if r.state == 1 && line {
                on += 1;
                blue
            } else {
                under
            };
            assert!(
                near(hatched[i], want),
                "`{hatch_scene}` pixel ({x}, {y}) of quad {q} (state {}) is {:?}, not {want:?}",
                r.state,
                hatched[i]
            );
            let want = if r.ancestor_gap > 0 {
                [0, 1, 2].map(|k| under[k] * 0.6 + pink[k] * 0.4)
            } else {
                under
            };
            assert!(
                near(tinted[i], want),
                "`{tint_scene}` pixel ({x}, {y}) of quad {q} (gap {}) is {:?}, not {want:?}",
                r.ancestor_gap,
                tinted[i]
            );
            for c in [hatched[i], tinted[i]] {
                assert!(
                    !near(c, hex([0x9B, 0x00, 0xFF])) && !near(c, hex([0x48, 0xFF, 0xFF])),
                    "pixel ({x}, {y}) reads as debug_invalid's hatch: {c:?}"
                );
            }
        }
    }
    assert!(on > 0, "no pending quad was hatched");
}

#[test]
fn qa_pending_hatch_and_fallback_tint_follow_the_definition() {
    check_hatch_and_tint(&gpu(), "pending_hatch", "fallback_tint");
}

negative_control!(
    qa_pending_hatch_and_fallback_tint_follow_the_definition,
    "the tint scene checked as the hatch fails",
    expected = "`fallback_tint` pixel",
    check_hatch_and_tint(&gpu(), "fallback_tint", "fallback_tint")
);
