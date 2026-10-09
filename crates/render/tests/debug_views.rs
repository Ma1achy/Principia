//! The debug views (render contract Part 5, "Live-state & array inspection"; Part 6; debug_tooling_plan §B, §C, §E;
//! TASK-M1-12), rendered through the golden harness's scenes (`validation::golden_scene`; RQ-229):
//! - REQ-TOOL-010: every debug view the registry lists has its `debug-views` case and fixture, and renders far enough
//!   from every rounding tie that its one reference holds on every backend; the |n|−1 view is flat zero on the
//!   synthetic payload (`debug_views_*`);
//! - REQ-TOOL-024: the derived views, orbit_count and retrograde from `θ̃`, the reduced crossing count, the finalised
//!   `ftle` and the current drift `H(r, p) − E_0`, each computed in the fragment, match a CPU reference on synthetic
//!   states: the integers and booleans exactly, `ftle` and the drift within the derived WGSL-accuracy bounds of
//!   `crates/kernel/tests/derived.rs` (`derived_views_match_cpu_*`);
//! - REQ-TOOL-025, REQ-TOOL-155, REQ-TOOL-156: distinct words hash to distinct colours by the whole-word fold; the
//!   symbol at slot `k` is the appended sequence's; the reduced length of a truncated word draws the hatch
//!   (`word_hash_and_symbol_at_k_*`);
//! - REQ-VAL-010, REQ-VAL-122, REQ-TOOL-154, REQ-TOOL-157: the registry lists both reductions of every vector field;
//!   `‖n‖ − 1` over a test render at f32 is within the proposed tolerance; the direction cosines of `n`, `r` and `p`
//!   and the ternary masses draw their renderings (`norm_n_views_*`).
//!
//! A view's value is read exactly through a probe: the view's own WGSL with its `colour` renamed and a `colour` that
//! writes its `value` into the red channel of the float target. Each test registers its negative control (R-176).

use std::path::Path;

use ledger::schema::FieldType;
use render::assemble::Kind;
use render::present::{self, Rgb};
use render::registry::{self, Category};
use validation::golden_scene::{
    self, appended, debug_cases, row_scene, scene, shape, tie_margin, Colouring, DebugCase, Scene,
    FGW_NO_SYMBOL,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The largest per-channel difference, linear RGB, between a render and its CPU twin, `numeric_views.rs`'s: the renders
/// sit within 6e-7 of their twins on Metal.
const TOL: f64 = 1e-5;

/// The least distance, in 8-bit steps, of any channel of a debug view's render from a rounding tie, as for the numeric
/// views' scenes: some sixty times the 1.5e-4 steps a render sits from its twin.
const MIN_TIE_MARGIN: f64 = 0.01;

/// The f32 unit roundoff, `2⁻²⁴`, as `crates/kernel/tests/derived.rs` takes it.
const U: f64 = f32::EPSILON as f64 / 2.0;

/// The proposed tolerance on `‖n‖ − 1` at f32 (REQ-VAL-122, a calibration: proposed, R-71; the human confirms it at
/// the M1 gate). `n = (a − b, 2p, 2q)/I` is exactly unit for any `ρ̃`, `λ̃` (Lagrange's identity, `p² + q² = ab`), so
/// only the rounding of `a`, `b`, `p`, `q`, `I`, the three divisions and the fragment's `length` departs from it: to
/// first order `‖n‖² − 1` within `3U` each for `(a − b)²` and `I²` from `a` and `b`, `4U` each from `p` and `q` (their
/// cancellation bounded by `√(ab) ≤ I/2`), `2·5U` for the divisions, then `length`'s `dot` and `sqrt` (`3U`, `9U`):
/// about `30U` on `‖n‖`, rounded up to `64U = 2⁻¹⁸ ≈ 3.8e-6`. The measured worst over the test render is printed with
/// the test's output and recorded in the PR.
const NORM_TOL: f64 = 64.0 * U;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn named(name: &str) -> Scene {
    scene(name).unwrap_or_else(|e| panic!("{e}"))
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<[f32; 4]> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn rgb(p: [f32; 4]) -> Rgb {
    [p[0], p[1], p[2]].map(f64::from)
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// The registry entry `id`'s WGSL.
fn source(id: &str) -> String {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
        .source
}

/// The probe of the view `id`: its WGSL, its `colour` renamed, and a `colour` writing its `value` in red.
fn probe_of(id: &str, params: &[(&str, f64)]) -> Colouring {
    let wgsl = format!(
        "{}\nfn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(value(ctx), 0.0, 0.0); }}\n",
        source(id).replace("fn colour(", "fn shown(")
    );
    Colouring::Probe(
        wgsl,
        params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), vec![*v]))
            .collect(),
    )
}

/// `s` with its colouring replaced.
fn recoloured(mut s: Scene, colouring: Colouring) -> Scene {
    s.colouring = colouring;
    s
}

/// The value sample `i`'s probe wrote: the red channel at its tile's first pixel.
fn probed(s: &Scene, image: &[[f32; 4]], i: u32) -> f32 {
    let (x, y) = s.pixels(i)[0];
    image[(y * s.size().0 + x) as usize][0]
}

/// Checks that every pixel of sample `i` of `image` is `want`'s colour at that pixel.
fn check_tile(s: &Scene, image: &[[f32; 4]], i: u32, want: &dyn Fn([f64; 2]) -> Rgb, what: &str) {
    let width = s.size().0;
    for (x, y) in s.pixels(i) {
        let got = rgb(image[(y * width + x) as usize]);
        let colour = want([f64::from(x) + 0.5, f64::from(y) + 0.5]);
        assert!(
            near(got, colour),
            "pixel ({x}, {y}) of sample {i} of `{}` is {got:?}, not {what} {colour:?}",
            s.name
        );
    }
}

/// A leaked case of the view `id` with `params`, for a scene coloured by the view at other params than its cases'.
fn case(id: &str, params: &[(&str, f64)]) -> &'static DebugCase {
    Box::leak(Box::new(DebugCase {
        name: format!("test {id}"),
        id: id.to_owned(),
        params: params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), vec![*v]))
            .collect(),
        nudge: 0,
    }))
}

// ── REQ-TOOL-010: every debug view has its case, its fixture and a reference clear of every tie ────────────────────

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/debug-views")
}

/// The suite's fixtures, each by its directory's name and the scene its `case.json` renders (`None` without one).
fn suite_fixtures() -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = std::fs::read_dir(fixtures())
        .expect("the suite's directory")
        .map(|d| {
            let name = d
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned();
            let harness = std::fs::read_to_string(fixtures().join(&name).join("case.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|j| j["render"]["harness"].as_str().map(str::to_owned));
            (name, harness)
        })
        .collect();
    out.sort();
    out
}

/// Checks that each of `ids`, the registry's debug entries, has a `debug-views` case.
fn check_coverage(ids: &[String]) {
    let cases = debug_cases().unwrap_or_else(|e| panic!("{e}"));
    for id in ids {
        assert!(
            cases.iter().any(|c| &c.id == id),
            "the debug view `{id}` has no debug-views case"
        );
    }
}

/// Checks that each `debug-views` case has a fixture of `fixtures` naming its scene, and that every other fixture
/// renders a golden scene of another task's (`golden_scene::NAMES`: TASK-M1-10's `state_view` and `detail_view`; the
/// task's Notes say other tasks add their cases to this suite), so that no fixture is left without a case or a scene.
fn check_fixtures(fixtures: &[(String, Option<String>)]) {
    let cases = debug_cases().unwrap_or_else(|e| panic!("{e}"));
    for c in cases {
        let harness = fixtures
            .iter()
            .find(|(name, _)| *name == c.name)
            .unwrap_or_else(|| panic!("`{}` has no fixture", c.name))
            .1
            .as_deref();
        assert_eq!(
            harness,
            Some(c.name.as_str()),
            "{}: the case renders another scene",
            c.name
        );
    }
    for (name, harness) in fixtures {
        let scene = harness.as_deref().unwrap_or_default();
        assert!(
            cases.iter().any(|c| c.name == *name) || golden_scene::NAMES.contains(&scene),
            "the fixture `{name}` is no debug-views case and renders no other golden scene"
        );
    }
}

/// The registry's entries tagged debug, by id.
fn debug_ids() -> Vec<String> {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| e.category == Category::Debug)
        .map(|e| e.id)
        .collect()
}

#[test]
fn debug_views_cover_the_registry() {
    let ids = debug_ids();
    for id in [
        "debug/live_shape",
        "debug/accumulators/ftle_running",
        "debug/derived/orbit_count",
        "debug/word/hash",
        "debug/reductions/masses_ternary",
        "debug/generated/n",
    ] {
        assert!(ids.iter().any(|i| i == id), "the registry has no `{id}`");
    }
    check_coverage(&ids);
    check_fixtures(&suite_fixtures());
}

negative_control!(
    debug_views_cover_the_registry,
    "a debug view registered with no case",
    expected = "has no debug-views case",
    {
        let mut ids = debug_ids();
        ids.push("debug/word/unregistered".to_owned());
        check_coverage(&ids);
    }
);

#[test]
fn debug_views_fixtures_are_cases_or_scenes() {
    check_fixtures(&suite_fixtures());
}

negative_control!(
    debug_views_fixtures_are_cases_or_scenes,
    "a fixture of no case that renders no golden scene",
    expected =
        "the fixture `word-unknown` is no debug-views case and renders no other golden scene",
    {
        let mut fixtures = suite_fixtures();
        fixtures.push(("word-unknown".to_owned(), Some("word-unknown".to_owned())));
        check_fixtures(&fixtures);
    }
);

/// Checks that each of `cases`' renders sits at least [`MIN_TIE_MARGIN`] from every rounding tie.
fn check_margins(h: &GpuHarness, cases: &[&str], least: f64) {
    let near: Vec<String> = cases
        .iter()
        .filter_map(|name| {
            let pixels: Vec<Rgb> = render(h, &named(name)).into_iter().map(rgb).collect();
            let margin = tie_margin(&pixels);
            (margin < least).then(|| format!("`{name}` at {margin}"))
        })
        .collect();
    assert!(
        near.is_empty(),
        "renders within {least} of an 8-bit step from a rounding tie: {}",
        near.join(", ")
    );
}

#[test]
fn debug_views_render_far_from_ties() {
    let h = gpu();
    let cases = debug_cases().unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = cases.iter().map(|c| c.name.as_str()).collect();
    check_margins(&h, &names, MIN_TIE_MARGIN);
}

negative_control!(
    debug_views_render_far_from_ties,
    "a margin no render can reach",
    expected = "of an 8-bit step from a rounding tie: `word-hash`",
    check_margins(&gpu(), &["word-hash"], 0.5)
);

/// Checks that every pixel of `image` is the viridis ramp at `t`.
fn check_flat(s: &Scene, image: &[[f32; 4]], t: f64) {
    for i in 0..8 {
        check_tile(
            s,
            image,
            i,
            &|_| present::ramp_viridis(t),
            "the flat colour",
        );
    }
}

#[test]
fn debug_views_norm_error_is_flat_zero() {
    let s = named("live_shape-norm_error");
    let image = render(&gpu(), &s);
    check_flat(&s, &image, present::dbg_literal(0.0));
}

negative_control!(
    debug_views_norm_error_is_flat_zero,
    "the live shape view's mode 0, the direction cosines, is not flat",
    expected = "not the flat colour",
    {
        let s = named("live_shape");
        let image = render(&gpu(), &s);
        check_flat(&s, &image, present::dbg_literal(0.0));
    }
);

/// The live shape view before it hatched an undefined `n` or `θ̃`: mode 0 the raw `½(n + 1)`, mode 1 unguarded.
const LIVE_SHAPE_UNGUARDED: &str = "// @uniform u_mode: u32 = 0 [0, 2]
fn colour(ctx: Ctx) -> vec3<f32> {
    let n = ctx.sample.n;
    if (uniforms.u_mode == 0u) {
        return 0.5 * (n + vec3<f32>(1.0));
    }
    if (uniforms.u_mode == 1u) {
        return ramp_twilight(fract(ctx.sample.theta / 6.2831855));
    }
    return dbg_sentinel(length(n) - 1.0, ctx.frag_xy);
}
";

/// Checks that the live shape view, `wgsl` at each mode, draws the hatch where its value is undefined, on the showcase
/// set with three samples broken: sample 1 coincident at the origin (`I = 0`, so `n = 0/0`), sample 2's `θ̃` the absence NaN and
/// sample 3's `+inf`. Modes 0 and 2 hatch sample 1, mode 1 samples 2 and 3; every other tile is no hatch.
fn check_live_shape_undefined(wgsl: &str) {
    let h = gpu();
    for (mode, hatched) in [(0, [1].as_slice()), (1, &[2, 3]), (2, &[1])] {
        let mut s = recoloured(
            named("live_shape"),
            Colouring::Probe(
                wgsl.to_owned(),
                vec![("u_mode".to_owned(), vec![f64::from(mode)])],
            ),
        );
        s.set.sample(1).r([[0.0; 2]; 3]);
        s.set.sample(2).theta(f32::from_bits(0x7fc0_0000));
        s.set.sample(3).theta(f32::INFINITY);
        assert!(
            s.read(1).n.iter().all(|c| c.is_nan()),
            "sample 1's n is 0/0"
        );
        let image = render(&h, &s);
        for i in 0..8 {
            let (x, y) = s.pixels(i)[0];
            let hatch = near(
                rgb(image[(y * s.size().0 + x) as usize]),
                present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]),
            );
            let what = if hatched.contains(&i) {
                "the hatch"
            } else {
                "no hatch"
            };
            assert_eq!(
                hatch,
                hatched.contains(&i),
                "mode {mode}: sample {i} draws {}, not {what}",
                if hatch { "the hatch" } else { "a colour" }
            );
        }
    }
}

#[test]
fn debug_views_live_shape_hatches_undefined() {
    check_live_shape_undefined(&source("debug/live_shape"));
}

negative_control!(
    debug_views_live_shape_hatches_undefined,
    "the view before its guards, which colours an undefined n and θ̃",
    expected = "mode 0: sample 1 draws a colour, not the hatch",
    check_live_shape_undefined(LIVE_SHAPE_UNGUARDED)
);

/// Checks the `last_symbol` view, `colouring`, on the showcase set: the empty word (sample 0) and the truncated word
/// (sample 6) draw the hatch, every other sample its last symbol by `dbg_cat(·, 4)` (payload §2: meaningful iff
/// `length ≥ 1 && length ≠ 127`).
fn check_last_symbol(colouring: Colouring) {
    let s = showcase_scene(colouring);
    let image = render(&gpu(), &s);
    for i in 0..8 {
        let read = s.read(i);
        let length = golden_scene::fgw_length_raw(read.word);
        if length == 0 || length == 127 {
            assert!(i == 0 || i == 6, "sample {i}'s word is empty or truncated");
            check_tile(&s, &image, i, &present::debug_invalid, "the hatch");
        } else {
            let want = present::dbg_cat(read.last_symbol, 4);
            check_tile(&s, &image, i, &|_| want, "its last symbol");
        }
    }
}

#[test]
fn debug_views_last_symbol_gates_on_the_word() {
    check_last_symbol(Colouring::View("last_symbol"));
}

negative_control!(
    debug_views_last_symbol_gates_on_the_word,
    "the view ungated, which draws the empty word's stored 0 as symbol `a`",
    expected = "sample 0 of",
    check_last_symbol(Colouring::Probe(
        "fn colour(ctx: Ctx) -> vec3<f32> { return dbg_cat(ctx.sample.last_symbol, 4u); }\n"
            .to_owned(),
        Vec::new()
    ))
);

// ── REQ-TOOL-024: the derived views match a CPU reference ────────────────────────────────────────────────────────

/// The showcase scene coloured by `colouring`.
fn showcase_scene(colouring: Colouring) -> Scene {
    recoloured(named("derived-orbit_count"), colouring)
}

/// `⌊|θ̃| / 2π⌋` and `θ̃ < 0` in f64 from the stored `θ̃` (payload §5), which the showcase keeps away from every
/// multiple of 2π, so `floor` is stable.
fn winding_reference(theta: f32) -> (f32, f32) {
    let t = f64::from(theta);
    (
        (t.abs() / std::f64::consts::TAU).floor() as f32,
        if t < 0.0 { 1.0 } else { 0.0 },
    )
}

/// Checks the winding views on `s`, whose `θ̃` each sample's own: each probe writes the reference exactly, and each
/// view draws it, the count at its literal place and the sense as a flag.
fn check_winding(s: &Scene, reference: fn(f32) -> (f32, f32)) {
    let h = gpu();
    let probes = [
        render(
            &h,
            &showcase_scene(probe_of("debug/derived/orbit_count", &[])),
        ),
        render(
            &h,
            &showcase_scene(probe_of("debug/derived/retrograde", &[])),
        ),
    ];
    let views = [
        render(&h, &showcase_scene(debug("debug/derived/orbit_count"))),
        render(&h, &showcase_scene(debug("debug/derived/retrograde"))),
    ];
    for i in 0..8 {
        let theta = s.read(i).theta;
        let (count, retro) = reference(theta);
        assert_eq!(
            probed(s, &probes[0], i).to_bits(),
            count.to_bits(),
            "sample {i}: orbit_count of θ̃ = {theta} is {}, not {count}",
            probed(s, &probes[0], i)
        );
        assert_eq!(
            probed(s, &probes[1], i).to_bits(),
            retro.to_bits(),
            "sample {i}: retrograde of θ̃ = {theta} is {}, not {retro}",
            probed(s, &probes[1], i)
        );
        check_tile(
            s,
            &views[0],
            i,
            &|f| present::dbg_sentinel(count, f),
            "the count",
        );
        check_tile(
            s,
            &views[1],
            i,
            &|_| present::dbg_flag(retro == 1.0),
            "the sense",
        );
    }
}

/// The view `id` at its header's defaults.
fn debug(id: &str) -> Colouring {
    Colouring::Debug(case(id, &[]))
}

#[test]
fn derived_views_match_cpu_winding() {
    let s = showcase_scene(debug("debug/derived/orbit_count"));
    for i in 0..8 {
        let t = f64::from(s.read(i).theta).abs() / std::f64::consts::TAU;
        assert!(
            t == 0.0 || (t - t.round()).abs() > 1e-3,
            "sample {i}'s θ̃ is near a nonzero multiple of 2π"
        );
    }
    check_winding(&s, winding_reference);
}

negative_control!(
    derived_views_match_cpu_winding,
    "a reference that rounds the turns instead of flooring them",
    expected = "orbit_count of θ̃",
    check_winding(
        &showcase_scene(debug("debug/derived/orbit_count")),
        |theta| {
            let t = f64::from(theta);
            (
                (t.abs() / std::f64::consts::TAU).round() as f32,
                if t < 0.0 { 1.0 } else { 0.0 },
            )
        }
    )
);

/// Checks the reduced crossing count on `s`: where the word is not truncated the probe writes `fgw_reduced_length`
/// exactly and the view draws it on the ramp over [0, 76]; where it is, the probe writes NaN and the view draws the
/// hatch (REQ-TOOL-156). `truncated` is the reference's validity.
fn check_reduced(s: &Scene, truncated: fn([u32; 4]) -> bool) {
    let h = gpu();
    let probe = render(
        &h,
        &showcase_scene(probe_of("debug/word/reduced_length", &[])),
    );
    let view = render(&h, &showcase_scene(debug("debug/word/reduced_length")));
    let raw = render(&h, &showcase_scene(Colouring::View("length")));
    let mut seen = [false; 2];
    for i in 0..8 {
        let word = s.read(i).word;
        let length = golden_scene::fgw_length_raw(word);
        let got = probed(s, &probe, i);
        if truncated(word) {
            seen[0] = true;
            assert!(
                got.is_nan(),
                "sample {i}: a truncated word's reduced length is {got}, not NaN"
            );
            check_tile(s, &view, i, &present::debug_invalid, "the hatch");
            check_tile(
                s,
                &raw,
                i,
                &|_| present::ramp_viridis(present::dbg_literal(127.0)),
                "127",
            );
        } else {
            seen[1] = true;
            assert_eq!(
                got.to_bits(),
                (length as f32).to_bits(),
                "sample {i}: the reduced length is {got}, not {length}"
            );
            let t = f64::from(length) / 76.0;
            check_tile(
                s,
                &view,
                i,
                &|_| present::ramp_viridis(t),
                "the ramp at the length",
            );
        }
    }
    assert_eq!(
        seen,
        [true, true],
        "the showcase holds a truncated word and others"
    );
}

#[test]
fn derived_views_match_cpu_reduced_crossing_count() {
    check_reduced(&showcase_scene(debug("debug/word/reduced_length")), |w| {
        golden_scene::fgw_length_raw(w) == 127
    });
}

negative_control!(
    derived_views_match_cpu_reduced_crossing_count,
    "a reference that takes the truncated word's 127 as its count",
    expected = "the reduced length is NaN, not 127",
    check_reduced(&showcase_scene(debug("debug/word/reduced_length")), |w| {
        golden_scene::fgw_length_raw(w) > 127
    })
);

/// The finalised FTLE in f64 from the stored f32 values, `S_final/(n · dt)` with `S_final = S + ln(δ/δ₀)` (payload
/// §5), and the f32 read's bound against it, `crates/kernel/tests/derived.rs`'s `ftle_bound`,
/// `U·(32 + 8·|L| + 10·|S_final|)/(n·dt)`; `None` where `ftle_valid` is false: a failed state, `n = 0` or no completed
/// renormalisation (payload §6; R-253, R-254).
fn ftle_reference(s: &Scene, i: u32) -> Option<(f64, f64)> {
    let st = s.set.simstate(i);
    let read = s.read(i);
    let failed = read.state >= 4;
    let n = read.t_end_step;
    let c = &s.context;
    if failed || n == 0 || n / c.n_renorm == 0 {
        return None;
    }
    let sq = |a: &[[f32; 2]; 3], b: &[[f32; 2]; 3]| -> f64 {
        a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .map(|(&x, &y)| (f64::from(y) - f64::from(x)).powi(2))
            .sum()
    };
    let delta = (sq(&st.r, &st.r_sh) + sq(&st.p, &st.p_sh)).sqrt();
    let l = (delta / f64::from(c.delta_0)).ln();
    let s_final = f64::from(st.S) + l;
    let n_dt = f64::from(n) * f64::from(c.dt_macro);
    Some((
        s_final / n_dt,
        U * (32.0 + 8.0 * l.abs() + 10.0 * s_final.abs()) / n_dt,
    ))
}

type Reference = fn(&Scene, u32) -> Option<(f64, f64)>;

/// Checks the derived view `id` on `s`: where `reference` gives a value, the probe is within its bound of it and the
/// view draws the read's value at its literal place; elsewhere the probe is NaN and the view draws the hatch.
fn check_continuous(s: &Scene, id: &str, reference: Reference, read: fn(&Scene, u32) -> f32) {
    let h = gpu();
    let probe = render(&h, &showcase_scene(probe_of(id, &[])));
    let view = render(&h, &showcase_scene(debug(id)));
    let mut seen = [false; 2];
    for i in 0..8 {
        let got = probed(s, &probe, i);
        match reference(s, i) {
            Some((want, bound)) => {
                seen[0] = true;
                assert!(
                    (f64::from(got) - want).abs() <= bound,
                    "sample {i}: `{id}` reads {got}, {} from the reference {want}, past {bound}",
                    (f64::from(got) - want).abs()
                );
                let v = read(s, i);
                check_tile(s, &view, i, &|f| present::dbg_sentinel(v, f), "the value");
            }
            None => {
                seen[1] = true;
                assert!(
                    got.is_nan(),
                    "sample {i}: `{id}` reads {got} where it is invalid"
                );
                check_tile(s, &view, i, &present::debug_invalid, "the hatch");
            }
        }
    }
    assert!(seen[0], "`{id}`: no sample is valid");
}

#[test]
fn derived_views_match_cpu_finalised_ftle() {
    let s = showcase_scene(debug("debug/derived/ftle"));
    check_continuous(&s, "debug/derived/ftle", ftle_reference, |s, i| {
        s.read(i).ftle
    });
    let invalid = (0..8).filter(|&i| ftle_reference(&s, i).is_none()).count();
    assert_eq!(
        invalid, 3,
        "the unstepped and the two failed samples are invalid"
    );
}

negative_control!(
    derived_views_match_cpu_finalised_ftle,
    "a reference of plain S/t, the partial interval not finalised",
    expected = "from the reference",
    check_continuous(
        &showcase_scene(debug("debug/derived/ftle")),
        "debug/derived/ftle",
        |s, i| {
            let (_, bound) = ftle_reference(s, i)?;
            let st = s.set.simstate(i);
            let n_dt = f64::from(s.read(i).t_end_step) * f64::from(s.context.dt_macro);
            Some((f64::from(st.S) / n_dt, bound))
        },
        |s, i| s.read(i).ftle
    )
);

/// The current drift `H(r, p) − E_0` in f64 from the stored f32 values and the sample's own masses, `G = 1`
/// (integrator dd §3.5; payload §5), and its bound, `crates/kernel/tests/derived.rs`'s `DRIFT_TOL`, 1e-6 of the terms'
/// scale `|K| + |V| + |E_0|`: the drift is a cancellation, so its error is the terms'.
fn drift_reference(s: &Scene, i: u32) -> Option<(f64, f64)> {
    let st = s.set.simstate(i);
    let m = s.masses(i).map(f64::from);
    let f = |x: f32| f64::from(x);
    let k: f64 = (0..3)
        .map(|j| (f(st.p[j][0]).powi(2) + f(st.p[j][1]).powi(2)) / (2.0 * m[j]))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| {
            let d = (f(st.r[a][0]) - f(st.r[b][0])).hypot(f(st.r[a][1]) - f(st.r[b][1]));
            -m[a] * m[b] / d
        })
        .sum();
    let e_0 = f(st.E_0);
    Some((k + v - e_0, 1e-6 * (k.abs() + v.abs() + e_0.abs())))
}

#[test]
fn derived_views_match_cpu_current_drift() {
    let s = showcase_scene(debug("debug/derived/energy_drift"));
    let masses: Vec<[f32; 3]> = (0..8).map(|i| s.masses(i)).collect();
    assert!(
        masses.windows(2).all(|w| w[0] != w[1]),
        "each sample has its own masses"
    );
    check_continuous(&s, "debug/derived/energy_drift", drift_reference, |s, i| {
        s.read(i).energy_drift
    });
}

negative_control!(
    derived_views_match_cpu_current_drift,
    "a reference with the masses of the twin's default, thirds, not the sample's",
    expected = "from the reference",
    check_continuous(
        &showcase_scene(debug("debug/derived/energy_drift")),
        "debug/derived/energy_drift",
        |s, i| {
            let (_, bound) = drift_reference(s, i)?;
            Some((
                f64::from(s.read_with(i, [1.0 / 3.0; 3]).energy_drift),
                bound,
            ))
        },
        |s, i| s.read(i).energy_drift
    )
);

// ── REQ-TOOL-025, REQ-TOOL-155: the whole-word hash and the symbol at slot k ─────────────────────────────────────────

/// The fixture words: every freely reduced word of length 0 to 3 (53 of them), then words of 10, 40 and 76 symbols
/// and the truncated word, each built by the kernel's append from its symbols.
fn fixture_words() -> Vec<([u32; 4], Vec<u32>)> {
    let mut seqs: Vec<Vec<u32>> = vec![Vec::new()];
    let mut layer: Vec<Vec<u32>> = vec![Vec::new()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for w in &layer {
            for s in 0..4u32 {
                if w.last().is_some_and(|&p| p ^ 1 == s) {
                    continue;
                }
                let mut v = w.clone();
                v.push(s);
                next.push(v);
            }
        }
        seqs.extend(next.iter().cloned());
        layer = next;
    }
    let cycle = |n: usize| (0..n).map(|k| [0, 2, 1, 3][k % 4]).collect::<Vec<u32>>();
    seqs.extend([cycle(10), cycle(40), cycle(76), vec![2; 77]]);
    seqs.into_iter().map(|q| (appended(&q).0, q)).collect()
}

/// The scene of the fixture words, one per sample, coloured by `colouring`.
fn words_scene(colouring: Colouring) -> Scene {
    let words = fixture_words();
    let mut s =
        row_scene("fixture words", words.len() as u32, colouring).unwrap_or_else(|e| panic!("{e}"));
    for (i, (w, _)) in (0u32..).zip(&words) {
        s.set.sample(i).word_raw(*w);
    }
    s
}

/// Checks that `fold` gives each fixture word its own 24-bit colour, and that the word-hash view draws each word as
/// `dbg_hash_word`, so distinct words take distinct colours on screen, but the truncated word, which draws the hatch
/// (payload §3: word-derived quantities are invalid once truncated).
fn check_hash(fold: fn([u32; 4]) -> u32) {
    let words = fixture_words();
    for (a, (wa, qa)) in words.iter().enumerate() {
        for (wb, qb) in &words[a + 1..] {
            assert!(wa != wb, "the fixture words {qa:?} and {qb:?} are one word");
            assert!(
                fold(*wa) & 0xff_ffff != fold(*wb) & 0xff_ffff,
                "the words {qa:?} and {qb:?} share a colour"
            );
        }
    }
    let s = words_scene(debug("debug/word/hash"));
    let image = render(&gpu(), &s);
    let mut truncated = 0;
    for (i, (w, _)) in (0u32..).zip(&words) {
        if golden_scene::fgw_length_raw(*w) == 127 {
            truncated += 1;
            check_tile(&s, &image, i, &present::debug_invalid, "the hatch");
        } else {
            check_tile(
                &s,
                &image,
                i,
                &|_| present::dbg_hash_word(*w),
                "the word's hash",
            );
        }
    }
    assert_eq!(truncated, 1, "the fixture's one truncated word");
}

#[test]
fn word_hash_and_symbol_at_k_distinct_words_distinct_colours() {
    assert_eq!(fixture_words().len(), 57);
    check_hash(present::word_hash);
}

negative_control!(
    word_hash_and_symbol_at_k_distinct_words_distinct_colours,
    "a fold over the low limb alone, which drops the length",
    expected = "share a colour",
    check_hash(|w| present::pcg(w[0]))
);

/// Checks that the symbol-at-k view, at each slot `k` of `ks`, shows each fixture word's `k`-th appended symbol: the
/// probe writes its code exactly and the view draws its class, or, past the word's length or in a truncated word, NaN
/// and the hatch. `symbol` is the reference.
fn check_symbols(ks: &[u32], symbol: fn(&[u32], u32) -> Option<u32>) {
    let words = fixture_words();
    let h = gpu();
    for &k in ks {
        let params = [("u_k", f64::from(k))];
        let probe = render(
            &h,
            &words_scene(probe_of("debug/word/symbol_at_k", &params)),
        );
        let s = words_scene(Colouring::Debug(case("debug/word/symbol_at_k", &params)));
        let view = render(&h, &s);
        for (i, (_, seq)) in (0u32..).zip(&words) {
            let got = probed(&s, &probe, i);
            match symbol(seq, k) {
                Some(want) => {
                    assert_eq!(
                        got.to_bits(),
                        (want as f32).to_bits(),
                        "{seq:?}: the symbol at {k} reads {got}, not {want}"
                    );
                    check_tile(
                        &s,
                        &view,
                        i,
                        &|_| present::dbg_cat(want, 4),
                        "the symbol's class",
                    );
                }
                None => {
                    assert!(got.is_nan(), "{seq:?}: slot {k} reads {got}, not NaN");
                    check_tile(&s, &view, i, &present::debug_invalid, "the hatch");
                }
            }
        }
    }
}

/// The `k`-th symbol of the appended sequence `seq`, freely reduced on append; none past its length or once it is
/// truncated, past 76 symbols.
fn appended_symbol(seq: &[u32], k: u32) -> Option<u32> {
    (seq.len() <= 76)
        .then(|| seq.get(k as usize).copied())
        .flatten()
}

#[test]
fn word_hash_and_symbol_at_k_matches_the_appended_sequence() {
    let (w, _) = appended(&[0, 2, 3]);
    assert_eq!(golden_scene::fgw_symbol(w, 3), FGW_NO_SYMBOL);
    check_symbols(&[0, 1, 2, 3, 9, 39, 75], appended_symbol);
}

negative_control!(
    word_hash_and_symbol_at_k_matches_the_appended_sequence,
    "a reference that reads the sequence from its end",
    expected = "the symbol at",
    check_symbols(&[0], |seq, k| {
        let len = seq.len();
        (len <= 76 && (k as usize) < len).then(|| seq[len - 1 - k as usize])
    })
);

// ── REQ-VAL-010, REQ-VAL-122, REQ-TOOL-154, REQ-TOOL-157: the vector fields' reductions ──────────────────────────────

/// The ledger's vector fields.
fn vector_fields() -> Vec<&'static str> {
    ledger::gen::validate(&ledger::layout())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| matches!(e.ty, FieldType::Vector { .. }))
        .map(|e| e.name)
        .collect()
}

/// Checks that `entries` list, for each of `fields`, both reductions, '‖·‖ as scalar' and 'as direction-cosines', each
/// a debug colour occupant.
fn check_reductions(entries: &[registry::Entry], fields: &[&str]) {
    for field in fields {
        for (name, id) in registry::vector_reductions(field) {
            let e = entries
                .iter()
                .find(|e| e.id == id)
                .unwrap_or_else(|| panic!("`{field}` has no '{name}' view, `{id}`"));
            assert_eq!(e.category, Category::Debug, "`{id}` is not tagged debug");
            assert_eq!(e.slot, Kind::Colour, "`{id}` is not a colour occupant");
        }
    }
}

#[test]
fn norm_n_views_registry_lists_both_reductions() {
    let fields = vector_fields();
    for f in ["n", "r", "p"] {
        assert!(
            fields.contains(&f),
            "`{f}` is not a vector field of the ledger"
        );
    }
    check_reductions(
        &registry::registry().unwrap_or_else(|e| panic!("{e}")),
        &fields,
    );
}

negative_control!(
    norm_n_views_registry_lists_both_reductions,
    "a registry with n's direction cosines removed",
    expected = "`n` has no 'as direction-cosines' view",
    {
        let entries: Vec<registry::Entry> = registry::registry()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|e| e.id != "debug/reductions/n_dircos")
            .collect();
        check_reductions(&entries, &vector_fields());
    }
);

/// A random configuration and positive masses summing to 1, from `next`, a uniform draw on [0, 1).
fn configuration(next: &mut dyn FnMut() -> f64) -> ([[f32; 2]; 3], [f32; 3]) {
    let mut r = [[0f32; 2]; 3];
    r.iter_mut()
        .flatten()
        .for_each(|x| *x = (4.0 * next() - 2.0) as f32);
    let raw = [0, 1, 2].map(|_| 0.05 + next());
    let sum: f64 = raw.iter().sum();
    (r, raw.map(|m| (m / sum) as f32))
}

/// The test render of `‖n‖ − 1`: 512 samples, each a random configuration with random masses, `n` read in the
/// fragment through the read side and its norm's error written by a probe.
fn norm_scene() -> Scene {
    let wgsl = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(length(ctx.sample.n) - 1.0, 0.0, 0.0); }\n";
    let mut s = row_scene(
        "‖n‖ − 1",
        512,
        Colouring::Probe(wgsl.to_owned(), Vec::new()),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut state = 0x5eed_u64;
    let mut next = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    for i in 0..512 {
        let (r, m) = configuration(&mut next);
        s.set.sample(i).r(r);
        let ic = s.set.ic(i);
        (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
    }
    s
}

/// The largest `|‖n‖ − 1|` the probe wrote over `s`, which is checked against `tol`, and the same from the Rust read
/// side's `n` in f32.
fn check_norm(s: &Scene, tol: f64) -> (f64, f64) {
    let image = render(&gpu(), s);
    let mut worst = (0f64, 0f64);
    for i in 0..s.context.grid.sample_count() {
        let got = f64::from(probed(s, &image, i)).abs();
        assert!(
            got <= tol,
            "sample {i}: ‖n‖ − 1 is {got}, past the tolerance {tol}"
        );
        let n = shape(s.set.simstate(i).r, s.masses(i));
        let cpu = (n.iter().map(|x| x * x).sum::<f32>().sqrt() - 1.0).abs();
        worst = (worst.0.max(got), worst.1.max(f64::from(cpu)));
    }
    worst
}

#[test]
fn norm_n_views_norm_within_tolerance() {
    let (gpu_worst, cpu_worst) = check_norm(&norm_scene(), NORM_TOL);
    println!(
        "REQ-VAL-122 evidence: over 512 random configurations, max |‖n‖ − 1| is {gpu_worst:e} rendered at f32 \
         ({:.2} U) and {cpu_worst:e} from the Rust read side at f32 ({:.2} U); proposed tolerance {NORM_TOL:e} (64 U)",
        gpu_worst / U,
        cpu_worst / U
    );
}

negative_control!(
    norm_n_views_norm_within_tolerance,
    "a tolerance of zero, which f32 rounding does not meet",
    expected = "past the tolerance",
    {
        check_norm(&norm_scene(), 0.0);
    }
);

/// Checks that the direction-cosines views of `n`, `r` and `p`, and the ternary masses view, draw their renderings:
/// `dbg_dircos3` of the read's `n`, `dbg_dircos6` of `r` and `p`, `dbg_ternary` of the masses, with equal masses white
/// and each vertex its primary. `ternary` is the masses' reference.
fn check_renderings(ternary: fn([f32; 3], [f64; 2]) -> Rgb) {
    let h = gpu();
    for (name, field) in [
        ("reductions-n_dircos", "n"),
        ("reductions-r_dircos", "r"),
        ("reductions-p_dircos", "p"),
    ] {
        let s = named(name);
        let image = render(&h, &s);
        for i in 0..8 {
            let read = s.read(i);
            let want: Box<dyn Fn([f64; 2]) -> Rgb> = match field {
                "n" => Box::new(move |f| present::dbg_dircos3(read.n, f)),
                "r" => Box::new(move |f| present::dbg_dircos6(read.r, f)),
                _ => Box::new(move |f| present::dbg_dircos6(read.p, f)),
            };
            check_tile(&s, &image, i, &*want, "the direction cosines");
        }
    }
    let s = named("reductions-masses_ternary");
    let image = render(&h, &s);
    let corners = [[1.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for i in 0..8 {
        let m = s.masses(i);
        check_tile(&s, &image, i, &|f| ternary(m, f), "the ternary colour");
        if let Some(c) = corners.get(i as usize) {
            check_tile(&s, &image, i, &|_| *c, "white or a primary");
        }
    }
}

#[test]
fn norm_n_views_direction_cosines_and_masses_draw_their_renderings() {
    check_renderings(present::dbg_ternary);
}

negative_control!(
    norm_n_views_direction_cosines_and_masses_draw_their_renderings,
    "masses scaled by their sum rather than their maximum",
    expected = "the ternary colour",
    check_renderings(|m, _| {
        let sum: f32 = m.iter().sum();
        m.map(|x| f64::from(x / sum))
    })
);

/// One input of the direction-cosines and ternary helpers: the WGSL call, its CPU twin's colour at a pixel, and the
/// colour it is expected to draw there, `None` for the hatch.
struct Undefined {
    call: String,
    twin: Box<dyn Fn([f64; 2]) -> Rgb>,
    want: Option<Rgb>,
}

/// `x` as a WGSL f32 of its exact bits.
fn bits(x: f32) -> String {
    format!("bitcast<f32>({:#010x}u)", x.to_bits())
}

/// The helpers' inputs at and beside their undefined cases: no positive mass, a zero vector, an absence NaN, each the
/// hatch; a smallest positive mass, a vector along one axis, one body alone, each its colour.
fn undefined_cases() -> Vec<Undefined> {
    let absent = f32::from_bits(present::ABSENT_NAN_BITS);
    let mut out = Vec::new();
    for (m, want) in [
        ([0.0, 0.0, 0.0], None),
        ([-1.0, -0.5, -0.25], None),
        ([absent, 0.5, 0.5], None),
        ([1e-3, 0.0, 0.0], Some([1.0, 0.0, 0.0])),
    ] {
        out.push(Undefined {
            call: format!(
                "dbg_ternary(vec3<f32>({}), ctx.frag_xy)",
                m.map(bits).join(", ")
            ),
            twin: Box::new(move |f| present::dbg_ternary(m, f)),
            want,
        });
    }
    for (v, want) in [
        ([0.0, 0.0, 0.0], None),
        ([absent, 0.0, 1.0], None),
        ([0.0, 0.0, 2.0], Some([0.5, 0.5, 1.0])),
    ] {
        out.push(Undefined {
            call: format!(
                "dbg_dircos3(vec3<f32>({}), ctx.frag_xy)",
                v.map(bits).join(", ")
            ),
            twin: Box::new(move |f| present::dbg_dircos3(v, f)),
            want,
        });
    }
    for (v, want) in [
        ([[0.0, 0.0]; 3], None),
        ([[0.0, 0.0], [0.0, absent], [1.0, 0.0]], None),
        ([[0.0, 0.0], [0.0, 3.0], [0.0, 0.0]], Some([0.0, 1.0, 0.0])),
    ] {
        let pairs: Vec<String> = v
            .iter()
            .map(|b| format!("vec2<f32>({}, {})", bits(b[0]), bits(b[1])))
            .collect();
        out.push(Undefined {
            call: format!(
                "dbg_dircos6(array<vec2<f32>, 3>({}), ctx.frag_xy)",
                pairs.join(", ")
            ),
            twin: Box::new(move |f| present::dbg_dircos6(v, f)),
            want,
        });
    }
    out
}

/// Checks that each of `cases` draws, on the GPU, its CPU twin's colour, and that the twin is the hatch where `want`
/// is `None` and `want` elsewhere.
fn check_undefined(cases: &[Undefined]) {
    let h = gpu();
    for c in cases {
        let wgsl = format!(
            "fn colour(ctx: Ctx) -> vec3<f32> {{ return {}; }}\n",
            c.call
        );
        let s = row_scene(
            "an undefined direction",
            1,
            Colouring::Probe(wgsl, Vec::new()),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let image = render(&h, &s);
        check_tile(&s, &image, 0, &*c.twin, "the CPU twin");
        let what = format!("`{}`'s colour", c.call);
        match c.want {
            Some(colour) => check_tile(&s, &image, 0, &|_| colour, &what),
            None => check_tile(&s, &image, 0, &present::debug_invalid, &what),
        }
    }
}

#[test]
fn norm_n_views_undefined_directions_draw_the_hatch() {
    check_undefined(&undefined_cases());
}

negative_control!(
    norm_n_views_undefined_directions_draw_the_hatch,
    "no positive mass expected to draw black rather than the hatch",
    expected = "`dbg_ternary(vec3<f32>(bitcast<f32>(0x00000000u)",
    {
        let mut cases = undefined_cases();
        cases[0].want = Some([0.0; 3]);
        check_undefined(&cases);
    }
);

/// Checks that `dbg_hash_u32` of each of `values` draws, on the GPU, `twin`'s colour of it.
fn check_hash_u32(values: &[u32], twin: fn(u32) -> Rgb) {
    let h = gpu();
    for &v in values {
        let wgsl = format!("fn colour(ctx: Ctx) -> vec3<f32> {{ return dbg_hash_u32({v}u); }}\n");
        let s = row_scene("a hashed word", 1, Colouring::Probe(wgsl, Vec::new()))
            .unwrap_or_else(|e| panic!("{e}"));
        let image = render(&h, &s);
        check_tile(
            &s,
            &image,
            0,
            &|_| twin(v),
            &format!("dbg_hash_u32({v})'s twin"),
        );
    }
}

#[test]
fn word_hash_and_symbol_at_k_hash_u32_shares_the_byte_step() {
    for v in [0u32, 1, 0xdead_beef, u32::MAX] {
        assert_eq!(
            present::dbg_hash_u32(v),
            present::dbg_bytes_rgb(present::pcg(v)),
            "dbg_hash_u32({v}) is not the PCG hash's byte step"
        );
    }
    check_hash_u32(&[0, 1, 0xdead_beef, u32::MAX], present::dbg_hash_u32);
}

negative_control!(
    word_hash_and_symbol_at_k_hash_u32_shares_the_byte_step,
    "a twin that hashes the next word",
    expected = "dbg_hash_u32(0)'s twin",
    check_hash_u32(&[0], |v| present::dbg_hash_u32(v.wrapping_add(1)))
);
