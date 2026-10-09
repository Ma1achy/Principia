//! The outcome state and detail views (TASK-M1-10): the outcome palette, the built-in colour `outcome_state`
//! (`shaders/wgsl/frag/colour/outcome_state.wgsl`; colour_composition §1.4; R-77, R-96), and the debug catalogue's
//! `state` and `detail` views, rendered through the golden harness's scenes (`validation::golden_scene`, `m1-outcome`
//! and `debug-views`):
//! - REQ-COL-002: one sample per class renders exactly §1.4's sRGB values, a triple collision and a triple ejection
//!   REQ-COL-062's proposed swatches, running the neutral grey and sim_failed the invalid pattern; the swatches are the
//!   occupant's params, and editing one changes only its class (`outcome_palette_*`).
//! - REQ-COL-004: the `detail` legend switches per state, and the view draws each state's segment from it
//!   (`detail_legend_per_state`).
//! - REQ-TOOL-021: the raw `state` view shows six distinct `dbg_cat` colours for the six states
//!   (`state_view_six_colours`).
//! - REQ-TOOL-022: escapes of different bodies render different colours, in the outcome palette and the `detail` view:
//!   `detail` is read, not dropped (`three_colours_regression`).
//! - REQ-RENDER-017: a synthetic RUNNING sample renders its palette colour, the grey, not discarded and not blank
//!   (`running_is_coloured`).
//! - REQ-COL-062: the proposal's evidence: the two triple-outcome swatches' OKLab lightness and their separation from
//!   the nine classes, the running grey and the hatch's two colours, and that each is the 8-bit colour farthest from
//!   its nearest such colour (`outcome_triple_swatches_proposal`). Run with `--nocapture` for the table the PR
//!   attaches.
//!
//! Each GPU check takes a render, so its control runs it on the render of an altered occupant and shows it fails
//! (pitfalls §9). Each test registers its negative control (R-176).

use ledger::gen::catalogue;
use render::assemble::Declaration;
use render::colour::outcome::{self, SWATCHES};
use render::present::{self, Rgb};
use validation::golden_scene::{scene, Scene, OUTCOME_EDIT, OUTCOME_SAMPLES};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The largest per-channel difference, linear RGB, between a computed pixel (the grey, the hatch, a `dbg_cat` colour)
/// and its CPU twin, as `numeric_views.rs` allows.
const TOL: f64 = 1e-5;

/// colour_composition §1.4's table, as written there: each class's label and its sRGB.
const SECTION_1_4: [(&str, u32); 9] = [
    ("collision 0–1 (pair 2)", 0xDE2D2D),
    ("collision 0–2 (pair 1)", 0x2EBC4E),
    ("collision 1–2 (pair 0)", 0x3462E0),
    ("bounded", 0x141418),
    ("degenerate", 0xECECF0),
    ("body 0 escape", 0xF0DE32),
    ("body 1 escape", 0xE034C6),
    ("body 2 escape", 0x30C8DC),
    ("collision @ t=0", 0xF29620),
];

/// REQ-COL-062's proposed swatches: triple collision and triple ejection.
const TRIPLE_COLLISION: u32 = 0xD6A1FF;
const TRIPLE_EJECTION: u32 = 0x000097;

fn hex(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

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

/// `s` rendered with its colour occupant's WGSL `from` replaced by `to`: a control's altered occupant.
#[cfg(feature = "controls")]
fn render_altered(h: &GpuHarness, s: &Scene, source: &str, from: &str, to: &str) -> Vec<[f32; 4]> {
    assert!(source.contains(from), "the occupant has no `{from}`");
    s.render_colour(h.device(), h.queue(), &source.replace(from, to))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// The generated view of `field`, as the registry scans it.
#[cfg(feature = "controls")]
fn view_source(field: &str) -> String {
    let id = format!("debug/generated/{field}");
    render::registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .map(|e| e.source)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
}

/// Sample `i`'s pixels in `image`, with their positions.
fn tile(s: &Scene, image: &[[f32; 4]], i: u32) -> Vec<((u32, u32), [f32; 4])> {
    let (width, _) = s.size();
    s.pixels(i)
        .into_iter()
        .map(|(x, y)| ((x, y), image[(y * width + x) as usize]))
        .collect()
}

/// A linear channel, encoded to 8-bit sRGB, rounded.
fn encode8(c: f32) -> u8 {
    (present::linear_to_srgb(f64::from(c)).clamp(0.0, 1.0) * 255.0).round() as u8
}

/// What a sample of the outcome scene must show.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    /// A swatch: every pixel exactly the f32 nearest the sRGB decode of this 8-bit code, so it encodes back to it.
    Srgb(u32),
    /// The neutral "not yet" grey, `DBG_NOT_YET`.
    Grey,
    /// The hatched invalid pattern.
    Hatch,
}

/// The outcome scene's samples' wants, in [`OUTCOME_SAMPLES`]' order, from §1.4's table and the proposal: the
/// collisions per pair, bounded, degenerate, the escapes per body, the collision at t = 0, the triple collision and
/// ejection, running, sim_failed, and a triple collision at t = 0, the t = 0 collision's orange.
fn outcome_wants() -> Vec<Want> {
    let c = |k: usize| Want::Srgb(SECTION_1_4[k].1);
    vec![
        c(0),
        c(1),
        c(2),
        c(3),
        c(4),
        c(5),
        c(6),
        c(7),
        c(8),
        Want::Srgb(TRIPLE_COLLISION),
        Want::Srgb(TRIPLE_EJECTION),
        Want::Grey,
        Want::Hatch,
        c(8),
    ]
}

/// Checks that every sample of the outcome scene `s`'s `image` shows `wants`' colour, and alpha 1: a swatch exactly
/// (its f32 bits, the declared default's, and its 8-bit sRGB code), the grey and the hatch within [`TOL`].
fn check_outcome(s: &Scene, image: &[[f32; 4]], wants: &[Want]) {
    assert_eq!(wants.len(), OUTCOME_SAMPLES.len());
    for (i, want) in wants.iter().enumerate() {
        let i = i as u32;
        for ((x, y), got) in tile(s, image, i) {
            assert_eq!(got[3], 1.0, "sample {i} pixel ({x}, {y}): alpha {}", got[3]);
            let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let ok = match *want {
                Want::Srgb(h) => {
                    let exact = present::srgb8(hex(h)).map(|c| c as f32);
                    [got[0], got[1], got[2]] == exact
                        && [got[0], got[1], got[2]].map(encode8) == hex(h)
                }
                Want::Grey => near(got, present::not_yet()),
                Want::Hatch => near(got, present::debug_invalid(frag)),
            };
            assert!(
                ok,
                "sample {i} {:?} pixel ({x}, {y}) is {got:?}, not {want:?}",
                OUTCOME_SAMPLES[i as usize]
            );
        }
    }
}

fn near(got: [f32; 4], want: Rgb) -> bool {
    (0..3).all(|c| (f64::from(got[c]) - want[c]).abs() <= TOL)
}

#[test]
fn outcome_palette_classes_render_section_1_4() {
    let s = named("outcome");
    check_outcome(&s, &render(&gpu(), &s), &outcome_wants());
}

negative_control!(
    outcome_palette_classes_render_section_1_4,
    "an occupant with the body-1 and body-2 escapes' swatches swapped renders neither's §1.4 value",
    expected = "pixel (48, 0) is",
    {
        let h = gpu();
        let s = named("outcome");
        let image = render_altered(
            &h,
            &s,
            outcome::WGSL,
            "if (d == 1u) {\n            return uniforms.escape_body_1;",
            "if (d == 1u) {\n            return uniforms.escape_body_2;",
        );
        check_outcome(&s, &image, &outcome_wants());
    }
);

/// Checks that the outcome scene `base` and its edited twin `edited` differ only at the samples of the class the edit
/// names, `edited_samples`, which show the edit's colour exactly.
fn check_edit(s: &Scene, base: &[[f32; 4]], edited: &[[f32; 4]], edited_samples: &[u32]) {
    let (_, colour) = OUTCOME_EDIT;
    let want = present::srgb8(colour).map(|c| c as f32);
    for i in 0..OUTCOME_SAMPLES.len() as u32 {
        for (((x, y), a), (_, b)) in tile(s, base, i).into_iter().zip(tile(s, edited, i)) {
            if edited_samples.contains(&i) {
                assert!(
                    [b[0], b[1], b[2]] == want && a != b,
                    "sample {i} pixel ({x}, {y}) is {b:?}, not the edit {want:?}"
                );
            } else {
                assert_eq!(a, b, "sample {i} pixel ({x}, {y}) changed with the edit");
            }
        }
    }
}

#[test]
fn outcome_palette_edit_changes_only_its_class() {
    let h = gpu();
    let (base, edited) = (named("outcome"), named("outcome_edited"));
    let (param, _) = OUTCOME_EDIT;
    assert_eq!(param, "escape_body_1");
    assert_eq!(
        edited.params().unwrap_or_else(|e| panic!("{e}")),
        vec![(param.to_owned(), present::srgb8(OUTCOME_EDIT.1).to_vec())],
        "the edit is a node param"
    );
    let samples: Vec<u32> = OUTCOME_SAMPLES
        .iter()
        .enumerate()
        .filter(|(_, s)| **s == ("escape", 1, 120))
        .map(|(i, _)| i as u32)
        .collect();
    assert_eq!(samples, [6], "one sample of the body-1 escape's class");
    check_edit(&base, &render(&h, &base), &render(&h, &edited), &samples);
}

negative_control!(
    outcome_palette_edit_changes_only_its_class,
    "the edit expected at the body-0 escape's sample instead fails",
    expected = "sample 5 pixel (40, 0) is",
    {
        let h = gpu();
        let (base, edited) = (named("outcome"), named("outcome_edited"));
        check_edit(&base, &render(&h, &base), &render(&h, &edited), &[5]);
    }
);

/// Checks that `swatches` are the occupant's declared params in order, each default the f32 nearest its sRGB decode,
/// and that the first nine are §1.4's table, the last two the proposal.
fn check_declared(swatches: &[outcome::Swatch]) {
    let d = Declaration::parse(outcome::WGSL).unwrap_or_else(|e| panic!("{e:?}"));
    let names: Vec<&str> = d.uniforms.iter().map(|u| u.name.as_str()).collect();
    let params: Vec<&str> = swatches.iter().map(|s| s.param).collect();
    assert_eq!(names, params, "the occupant's params are the swatches");
    for (u, s) in d.uniforms.iter().zip(swatches) {
        let want = present::srgb8(s.srgb8).map(|c| c as f32);
        let got: Vec<f32> = u.default.iter().map(|&v| v as f32).collect();
        assert_eq!(
            got, want,
            "`{}`'s default is not {:02X?} decoded",
            s.param, s.srgb8
        );
    }
    for (s, (label, h)) in swatches.iter().zip(SECTION_1_4) {
        assert_eq!((s.label, s.srgb8, s.proposed), (label, hex(h), false));
    }
    let triples: Vec<_> = swatches[9..]
        .iter()
        .map(|s| (s.label, s.srgb8, s.proposed))
        .collect();
    assert_eq!(
        triples,
        [
            ("triple collision", hex(TRIPLE_COLLISION), true),
            ("triple ejection", hex(TRIPLE_EJECTION), true)
        ]
    );
}

#[test]
fn outcome_palette_swatches_are_the_occupants_params() {
    check_declared(&SWATCHES);
    assert_eq!(
        render::assemble::builtin(render::assemble::Kind::Colour, outcome::ID),
        Some(outcome::WGSL),
        "the occupant is the built-in colour `{}`",
        outcome::ID
    );
}

negative_control!(
    outcome_palette_swatches_are_the_occupants_params,
    "a swatch one 8-bit step off its declared default must fail",
    expected = "`escape_body_2`'s default is not",
    {
        let mut swatches = SWATCHES;
        swatches[7].srgb8[2] += 1;
        check_declared(&swatches);
    }
);

// ── REQ-RENDER-017: a RUNNING sample is coloured like any class ──────────────────────────────────────────────────────

/// The outcome scene's running sample.
fn running_sample() -> u32 {
    OUTCOME_SAMPLES
        .iter()
        .position(|s| s.0 == "running")
        .map(|i| i as u32)
        .unwrap_or_else(|| panic!("the outcome scene has no running sample"))
}

/// Checks that the running sample of `image` is drawn, alpha 1, in the "not yet" grey: neither blank (black, or the
/// clear colour) nor any other class's colour.
fn check_running(s: &Scene, image: &[[f32; 4]]) {
    let i = running_sample();
    assert_eq!(
        s.read(i).state,
        outcome::code("running"),
        "a RUNNING sample"
    );
    for ((x, y), got) in tile(s, image, i) {
        assert!(
            got[3] == 1.0 && near(got, present::not_yet()) && got[..3] != [0.0; 3],
            "the running sample's pixel ({x}, {y}) is {got:?}, not the grey {:?}",
            present::not_yet()
        );
    }
}

#[test]
fn running_is_coloured() {
    let s = named("outcome");
    check_running(&s, &render(&gpu(), &s));
}

negative_control!(
    running_is_coloured,
    "an occupant drawing a running sample blank must fail",
    expected = "the running sample's pixel (88, 0)",
    {
        let s = named("outcome");
        let image = render_altered(
            &gpu(),
            &s,
            outcome::WGSL,
            "return DBG_NOT_YET;",
            "return vec3<f32>(0.0, 0.0, 0.0);",
        );
        check_running(&s, &image);
    }
);

// ── REQ-TOOL-022: the three-colours regression ───────────────────────────────────────────────────────────────────

/// The colour of sample `i`'s first pixel.
fn first(s: &Scene, image: &[[f32; 4]], i: u32) -> [f32; 4] {
    tile(s, image, i)[0].1
}

/// Checks that the samples `samples` of `image` render pairwise different colours.
fn check_distinct(s: &Scene, image: &[[f32; 4]], samples: &[u32], what: &str) {
    for (k, &a) in samples.iter().enumerate() {
        for &b in &samples[k + 1..] {
            assert_ne!(
                first(s, image, a),
                first(s, image, b),
                "{what}: samples {a} and {b} render the same colour"
            );
        }
    }
}

/// The outcome scene's escapes, bodies 0, 1, 2 and all three, and the `detail` view scene's escapes and collisions,
/// each by `detail` code.
const OUTCOME_ESCAPES: [u32; 4] = [5, 6, 7, 10];
const DETAIL_ESCAPES: [u32; 4] = [0, 1, 2, 3];
const DETAIL_COLLISIONS: [u32; 4] = [4, 5, 6, 7];

#[test]
fn three_colours_regression() {
    let h = gpu();
    let s = named("outcome");
    for (k, &i) in OUTCOME_ESCAPES.iter().enumerate() {
        assert_eq!(OUTCOME_SAMPLES[i as usize].0, "escape");
        assert_eq!(s.read(i).detail, k as u32, "sample {i} escapes body {k}");
    }
    check_distinct(&s, &render(&h, &s), &OUTCOME_ESCAPES, "the outcome palette");
    let d = named("detail_view");
    let image = render(&h, &d);
    check_distinct(&d, &image, &DETAIL_ESCAPES, "the detail view's escapes");
    check_distinct(
        &d,
        &image,
        &DETAIL_COLLISIONS,
        "the detail view's collisions",
    );
}

negative_control!(
    three_colours_regression,
    "a detail view that drops `detail` renders every escape alike",
    expected = "the detail view's escapes: samples 0 and 1 render the same colour",
    {
        let d = named("detail_view");
        let source = view_source("detail");
        let image = render_altered(
            &gpu(),
            &d,
            &source,
            "let d = ctx.sample.detail;",
            "let d = 0u;",
        );
        check_distinct(&d, &image, &DETAIL_ESCAPES, "the detail view's escapes");
    }
);

// ── REQ-COL-004: the detail legend switches per state ────────────────────────────────────────────────────────────

/// The `detail` legend payload §2 gives each state code 0–7: escape a body id, collision a pair id (pair `k` the side
/// opposite body `k`, R-22), `3` all three, the failure states their categories; none for bounded, running and the
/// reserved codes.
fn section_2_legend(state: u32) -> Option<Vec<&'static str>> {
    match state {
        0 => Some(vec![
            "body 0",
            "body 1",
            "body 2",
            "all three: triple ejection",
        ]),
        2 => Some(vec![
            "pair 0: bodies 1–2",
            "pair 1: bodies 0–2",
            "pair 2: bodies 0–1",
            "all three: triple collision",
        ]),
        4 => Some(vec![
            "NaN in state",
            "Inf/overflow in state",
            "non-finite derived quantity",
            "reserved",
        ]),
        5 => Some(vec![
            "non-finite decode output",
            "degenerate configuration",
            "invalid mass construction",
            "other/reserved",
        ]),
        _ => None,
    }
}

/// The ledger's legend for state code `state`: its segment's labels.
fn ledger_legend(state: u32) -> Option<Vec<String>> {
    catalogue::detail_segment(state).map(|s| s.classes.into_iter().map(|c| c.label).collect())
}

/// Checks that `legend` gives each state code 0–7 payload §2's legend.
fn check_legend(legend: impl Fn(u32) -> Option<Vec<String>>) {
    for state in 0..8 {
        let want = section_2_legend(state).map(|l| l.iter().map(|x| (*x).to_owned()).collect());
        assert_eq!(legend(state), want, "state {state}'s detail legend");
    }
}

/// Checks that the `detail` view scene's `image` draws each sample in its legend class's colour, from the ledger's
/// segments: `dbg_cat(class, n)`, and blank where the state has no segment; the sixteen classes distinct.
fn check_view_is_legend(s: &Scene, image: &[[f32; 4]]) {
    let n = catalogue::detail_classes();
    assert_eq!(n, 17, "class 0 and four segments of four");
    for i in 0..s.context.grid.sample_count() {
        let r = s.read(i);
        let want = match catalogue::detail_segment(r.state) {
            Some(seg) => {
                let c = seg.classes[r.detail as usize].clone();
                assert_eq!(c.detail, r.detail);
                present::dbg_cat(c.class, n)
            }
            None => [0.0; 3],
        };
        for ((x, y), got) in tile(s, image, i) {
            assert!(
                near(got, want),
                "detail view sample {i} (state {}, detail {}) pixel ({x}, {y}) is {got:?}, not its legend's {want:?}",
                r.state,
                r.detail
            );
        }
    }
    check_distinct(
        s,
        image,
        &(0..16).collect::<Vec<_>>(),
        "the detail view's classes",
    );
}

/// The outcome legend's segment of each state code, by swatch param: keyed by state, as the detail legend is.
fn check_outcome_segments(segment: impl Fn(u32) -> Vec<usize>) {
    let want: [&[&str]; 8] = [
        &[
            "escape_body_0",
            "escape_body_1",
            "escape_body_2",
            "triple_ejection",
        ],
        &["bounded"],
        &[
            "collision_pair_0",
            "collision_at_start",
            "collision_pair_1",
            "collision_pair_2",
            "triple_collision",
        ],
        &[],
        &[],
        &["degenerate"],
        &[],
        &[],
    ];
    for (state, want) in want.iter().enumerate() {
        let got: Vec<&str> = segment(state as u32)
            .iter()
            .map(|&k| SWATCHES[k].param)
            .collect();
        assert_eq!(got, *want, "state {state}'s outcome legend segment");
    }
}

#[test]
fn detail_legend_per_state() {
    check_legend(ledger_legend);
    assert_ne!(
        ledger_legend(0),
        ledger_legend(2),
        "escape's legend and collision's differ"
    );
    let d = named("detail_view");
    check_view_is_legend(&d, &render(&gpu(), &d));
    check_outcome_segments(outcome::segment);
}

negative_control!(
    detail_legend_per_state,
    "a legend keyed by `detail` alone, escape's for every state (the three-colours bug's shape), must fail",
    expected = "state 1's detail legend",
    check_legend(|_| ledger_legend(0))
);

// ── REQ-TOOL-021: the raw `state` view's six colours ─────────────────────────────────────────────────────────────

/// Checks that the `state` view scene's `image` draws state `i` at sample `i` in `dbg_cat(i, 6)`, six distinct
/// colours.
fn check_state_view(s: &Scene, image: &[[f32; 4]]) {
    for i in 0..6 {
        assert_eq!(s.read(i).state, i, "sample {i}'s state");
        for ((x, y), got) in tile(s, image, i) {
            assert!(
                near(got, present::dbg_cat(i, 6)),
                "state view sample {i} pixel ({x}, {y}) is {got:?}, not dbg_cat({i}, 6)"
            );
        }
    }
    check_distinct(s, image, &[0, 1, 2, 3, 4, 5], "the state view");
}

#[test]
fn state_view_six_colours() {
    let s = named("state_view");
    check_state_view(&s, &render(&gpu(), &s));
}

negative_control!(
    state_view_six_colours,
    "a state view reading `state` halved draws two states alike",
    expected = "state view sample 1 pixel",
    {
        let s = named("state_view");
        let image = render_altered(
            &gpu(),
            &s,
            &view_source("state"),
            "dbg_cat(ctx.sample.state, 6u)",
            "dbg_cat(ctx.sample.state / 2u, 6u)",
        );
        check_state_view(&s, &image);
    }
);

// ── REQ-COL-062: the proposal's evidence ─────────────────────────────────────────────────────────────────────────────

/// The colours the triple swatches must stand apart from: §1.4's nine classes, the running grey (REQ-COL-053) and the
/// hatch's two colours (REQ-COL-055), by name, as OKLab.
fn neighbours() -> Vec<(String, Rgb)> {
    let mut out: Vec<(String, Rgb)> = SECTION_1_4
        .iter()
        .map(|&(name, h)| (format!("{name} #{h:06X}"), present::srgb8(hex(h))))
        .collect();
    let [g, ..] = present::NOT_YET_SRGB8;
    out.push((
        format!("running grey #{g:02X}{g:02X}{g:02X}"),
        present::not_yet(),
    ));
    for c in ledger::gen::prelude::hatch().colours {
        out.push((
            format!("hatch #{:02X}{:02X}{:02X}", c[0], c[1], c[2]),
            present::srgb8(c),
        ));
    }
    out.into_iter()
        .map(|(n, c)| (n, present::linear_to_oklab(c)))
        .collect()
}

fn distance(a: Rgb, b: Rgb) -> f64 {
    a.iter()
        .zip(&b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The OKLab of every 8-bit sRGB colour, by `(r << 16) | (g << 8) | b`: dd_colouring §3.1's map, `M₂ · cbrt(M₁ · rgb)`,
/// with `M₁ · rgb` summed from per-channel tables.
fn cube() -> Vec<Rgb> {
    let m = render::colour::space::OKLAB;
    let lin: Vec<f64> = (0..=255u8)
        .map(|v| present::srgb_to_linear(f64::from(v) / 255.0))
        .collect();
    let column = |c: usize| -> Vec<Rgb> {
        lin.iter()
            .map(|&x| [0, 1, 2].map(|r| m.m1[r][c] * x))
            .collect()
    };
    let (cr, cg, cb) = (column(0), column(1), column(2));
    let mut out = Vec::with_capacity(1 << 24);
    for r in &cr {
        for g in &cg {
            for b in &cb {
                let lms = [0, 1, 2].map(|k| (r[k] + g[k] + b[k]).cbrt());
                out.push(
                    [0, 1, 2]
                        .map(|k| m.m2[k][0] * lms[0] + m.m2[k][1] * lms[1] + m.m2[k][2] * lms[2]),
                );
            }
        }
    }
    out
}

/// Each 8-bit colour's squared OKLab distance from its nearest of `from`, lowered in place.
fn lower(nearest: &mut [f64], cube: &[Rgb], from: &[Rgb]) {
    for (n, c) in nearest.iter_mut().zip(cube) {
        for f in from {
            let d = (c[0] - f[0]).powi(2) + (c[1] - f[1]).powi(2) + (c[2] - f[2]).powi(2);
            if d < *n {
                *n = d;
            }
        }
    }
}

/// The 8-bit colour, as `0xRRGGBB`, whose `nearest` is the largest, and that distance.
fn farthest(nearest: &[f64]) -> (u32, f64) {
    let (k, d) =
        nearest.iter().enumerate().fold(
            (0, f64::NEG_INFINITY),
            |a, (k, &d)| if d > a.1 { (k, d) } else { a },
        );
    (k as u32, d.sqrt())
}

/// Checks that `first` is the 8-bit colour farthest from its nearest neighbour, and `second` the farthest once `first`
/// is a neighbour too.
fn check_farthest(cube: &[Rgb], first: u32, second: u32) {
    let from: Vec<Rgb> = neighbours().into_iter().map(|(_, c)| c).collect();
    let mut nearest = vec![f64::INFINITY; cube.len()];
    lower(&mut nearest, cube, &from);
    let (a, da) = farthest(&nearest);
    assert_eq!(
        a, first,
        "#{first:06X} is not the farthest colour, #{a:06X} is ({da:.4})"
    );
    lower(&mut nearest, cube, &[cube[a as usize]]);
    let (b, db) = farthest(&nearest);
    assert_eq!(
        b, second,
        "#{second:06X} is not the next farthest, #{b:06X} is ({db:.4})"
    );
}

#[test]
fn outcome_triple_swatches_proposal() {
    let cube = cube();
    let mut rows: Vec<(String, Rgb)> = neighbours();
    for (name, h) in [
        ("triple collision", TRIPLE_COLLISION),
        ("triple ejection", TRIPLE_EJECTION),
    ] {
        let lab = present::linear_to_oklab(present::srgb8(hex(h)));
        assert_eq!(
            lab, cube[h as usize],
            "the cube's OKLab is dd_colouring §3.1's"
        );
        println!(
            "REQ-COL-062 proposal: {name} #{h:06X}, OKLab L {:.3}",
            lab[0]
        );
        let mut d: Vec<(f64, &str)> = rows
            .iter()
            .map(|(n, c)| (distance(lab, *c), n.as_str()))
            .collect();
        d.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (dist, n) in &d {
            println!("  OKLab distance from {n}: {dist:.3}");
        }
        rows.push((format!("{name} #{h:06X}"), lab));
    }
    let l = |h: u32| present::linear_to_oklab(present::srgb8(hex(h)))[0];
    assert!(
        l(TRIPLE_COLLISION) > l(TRIPLE_EJECTION),
        "the collision's swatch is the lighter: the additive primaries mix toward white"
    );
    check_farthest(&cube, TRIPLE_EJECTION, TRIPLE_COLLISION);
}

negative_control!(
    outcome_triple_swatches_proposal,
    "mid grey, #808080, is not the farthest colour",
    expected = "#808080 is not the farthest colour",
    check_farthest(&cube(), 0x808080, TRIPLE_COLLISION)
);
