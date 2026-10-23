//! The outcome state and detail views (TASK-M1-10, as R-444 to R-447 change them, TASK-M2-32): the outcome palette,
//! the built-in colour `outcome_state` (`shaders/wgsl/frag/colour/outcome_state.wgsl`; colour_composition §1.4; R-77,
//! R-444 to R-447), and the debug catalogue's `state` and `detail` views, rendered through the golden harness's scenes
//! (`validation::golden_scene`, `m1-outcome` and `debug-views`):
//! - REQ-COL-002: one sample per class renders exactly §1.4's sRGB values, the nine classes physical outcomes only; a
//!   collision and a triple collision at `t_end_step == 0` their own classes' colours (R-445); running, `sim_failed`
//!   of each `detail` and the reserved codes `#141418` (R-446); `decode_failed` of each `detail` and a forbidden-region
//!   sample the canvas background the harness records (R-511, R-467; REQ-COL-101, REQ-COL-108); the swatches are the occupant's params, both sets', and editing one changes only its class
//!   (`outcome_palette_*`).
//! - REQ-COL-004: the `detail` legend switches per state, and the view draws each state's segment from it
//!   (`detail_legend_per_state`).
//! - REQ-TOOL-021: the raw `state` view shows six distinct `dbg_cat` colours for the six states
//!   (`state_view_six_colours`).
//! - REQ-TOOL-022: escapes of different bodies render different colours, in the outcome palette and the `detail` view:
//!   `detail` is read, not dropped (`three_colours_regression`).
//! - REQ-GUI-188: each swatch's label is the one conversion module's class name of its `state` and `detail`, and of
//!   every sample its class takes; no swatch label and no key of the outcome legend's segments shows an internal id,
//!   "body 0" or "pair " and a digit (`display_names_swatch_labels_*`, the render half of `ledger`'s `display_names`,
//!   which the ledger, the build DAG's root, cannot reach).
//! - REQ-RENDER-017: a synthetic RUNNING sample renders its palette colour, bounded / nothing yet's `#141418` (R-446),
//!   not discarded and not blank (`running_is_coloured`).
//!
//! REQ-COL-062's proposal evidence for the triple swatches `#D6A1FF` and `#000097` (TASK-M1-10, confirmed by R-433)
//! leaves with them: R-445 gives the triple outcomes `#F29620` and `#611597`, and their evidence is REQ-COL-104's
//! (`tests/outcome_view.rs`, `outcome_palette_separation`).
//!
//! Each GPU check takes a render, so its control runs it on the render of an altered occupant and shows it fails
//! (pitfalls §9). Each test registers its negative control (R-176).

use ledger::display;
use ledger::gen::catalogue;
use render::assemble::Declaration;
use render::colour::outcome::{self, SWATCHES};
use render::present::{self, Rgb};
use validation::golden_scene::{scene, Scene, OUTCOME_EDIT, OUTCOME_FORBIDDEN, OUTCOME_SAMPLES};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The largest per-channel difference, linear RGB, between a computed pixel (the hatch, a `dbg_cat` colour) and its
/// CPU twin, as `numeric_views.rs` allows.
const TOL: f64 = 1e-5;

/// colour_composition §1.4's table, as written there (R-444, R-445; the names as shown, R-449): each class's label and
/// its sRGB, in the table's order.
const SECTION_1_4: [(&str, u32); 9] = [
    ("bounded / nothing yet", 0x141418),
    ("collision 1–2", 0xDE2D2D),
    ("collision 1–3", 0x2EBC4E),
    ("collision 2–3", 0x3462E0),
    ("triple collision", 0xF29620),
    ("escape body 1", 0xF0DE32),
    ("escape body 2", 0xE034C6),
    ("escape body 3", 0x30C8DC),
    ("triple ejection", 0x611597),
];

/// colour_composition §1.4's OKLab-uniform variant (R-447), as written there, bounded `#141418` with it, in the first
/// table's order.
const VARIANT_1_4: [(&str, u32); 9] = [
    ("bounded / nothing yet", 0x141418),
    ("collision 1–2", 0xE7857A),
    ("collision 1–3", 0x6EB976),
    ("collision 2–3", 0x7FA2F2),
    ("triple collision", 0xD99349),
    ("escape body 1", 0xB3A841),
    ("escape body 2", 0xD486C3),
    ("escape body 3", 0x09B9CD),
    ("triple ejection", 0xB790E2),
];

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
    /// The canvas background the harness records, exactly: a `decode_failed` sample (R-511) or one in a forbidden
    /// chart region (R-467).
    Background,
}

/// The outcome scene's samples' wants, in [`OUTCOME_SAMPLES`]' order, from §1.4's table: bounded, the collisions per
/// pair, the triple collision, the escapes per body, the triple ejection; the collision of pair 2 at t = 0 in collision
/// 1–2's red and the triple collision at t = 0 in its orange (R-445); running, `sim_failed` of each `detail` and the
/// reserved codes in `#141418` (R-446); `decode_failed` of each `detail` and the forbidden sample the background
/// (R-511, R-467).
fn outcome_wants() -> Vec<Want> {
    let c = |k: usize| Want::Srgb(SECTION_1_4[k].1);
    let mut out: Vec<Want> = (0..9).map(c).collect();
    out.extend([c(1), c(4)]);
    out.extend([c(0); 5]);
    out.extend([Want::Background; 4]);
    out.extend([c(0); 2]);
    assert_eq!(
        out.len(),
        OUTCOME_FORBIDDEN as usize,
        "the forbidden sample is last"
    );
    out.push(Want::Background);
    assert_eq!(out.len(), OUTCOME_SAMPLES.len(), "a want per sample");
    out
}

/// Checks that every sample of the outcome scene `s`'s `image` shows `wants`' colour, and alpha 1: a swatch exactly
/// (its f32 bits, the declared default's, and its 8-bit sRGB code), the background exactly, its f32 bits the recorded
/// value's.
fn check_outcome(s: &Scene, image: &[[f32; 4]], wants: &[Want]) {
    assert_eq!(wants.len(), OUTCOME_SAMPLES.len());
    for (i, want) in wants.iter().enumerate() {
        let i = i as u32;
        for ((x, y), got) in tile(s, image, i) {
            assert_eq!(got[3], 1.0, "sample {i} pixel ({x}, {y}): alpha {}", got[3]);
            let ok = match *want {
                Want::Srgb(h) => {
                    let exact = present::srgb8(hex(h)).map(|c| c as f32);
                    [got[0], got[1], got[2]] == exact
                        && [got[0], got[1], got[2]].map(encode8) == hex(h)
                }
                Want::Background => [got[0], got[1], got[2]] == present::canvas_background(),
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
    "an occupant drawing escape body 2 in escape body 3's swatch renders not §1.4's value",
    expected = "pixel (48, 0) is",
    {
        let h = gpu();
        let s = named("outcome");
        let image = render_altered(
            &h,
            &s,
            outcome::WGSL,
            "select(uniforms.escape_body_1, uniforms.oklab_escape_body_1, v)",
            "select(uniforms.escape_body_2, uniforms.oklab_escape_body_2, v)",
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

/// Checks that the occupant's declared params are the set selector, `swatch_set`, a `u32` in `[0, 1]` defaulting to
/// 0, the sRGB palette (REQ-COL-103), then `swatches` and `variant`, the two sets, each default the f32 nearest its sRGB
/// decode; that `swatches` are §1.4's table and `variant` its OKLab-uniform table, labels as shown (R-449), none still
/// a proposal; and that `outcome::SETS` holds them in that order.
fn check_declared(swatches: &[outcome::Swatch], variant: &[outcome::Swatch]) {
    let d = Declaration::parse(outcome::WGSL).unwrap_or_else(|e| panic!("{e:?}"));
    let (set, rest) = d
        .uniforms
        .split_first()
        .expect("the occupant declares params");
    assert_eq!(
        (set.name.as_str(), set.ty, set.default.as_slice(), set.range),
        (
            outcome::SET_PARAM,
            render::assemble::UniformType::U32,
            &[0.0][..],
            Some((0.0, 1.0))
        ),
        "the set selector"
    );
    let all: Vec<&outcome::Swatch> = swatches.iter().chain(variant).collect();
    let names: Vec<&str> = rest.iter().map(|u| u.name.as_str()).collect();
    let params: Vec<&str> = all.iter().map(|s| s.param).collect();
    assert_eq!(names, params, "the occupant's params are the swatches");
    for (u, s) in rest.iter().zip(&all) {
        let want = present::srgb8(s.srgb8).map(|c| c as f32);
        let got: Vec<f32> = u.default.iter().map(|&v| v as f32).collect();
        assert_eq!(
            got, want,
            "`{}`'s default is not {:02X?} decoded",
            s.param, s.srgb8
        );
    }
    for (table, set) in [(SECTION_1_4, swatches), (VARIANT_1_4, variant)] {
        let got: Vec<_> = set.iter().map(|s| (s.label, s.srgb8, s.proposed)).collect();
        let want: Vec<_> = table.iter().map(|&(l, h)| (l, hex(h), false)).collect();
        assert_eq!(got, want, "a set is not §1.4's table");
    }
    assert_eq!(
        outcome::SETS.map(|(name, set)| (name, set.to_vec())),
        [
            ("sRGB", swatches.to_vec()),
            ("OKLab-uniform", variant.to_vec())
        ],
        "the named sets, the sRGB palette first, the default"
    );
}

#[test]
fn outcome_palette_swatches_are_the_occupants_params() {
    check_declared(&SWATCHES, &outcome::OKLAB_UNIFORM);
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
        check_declared(&swatches, &outcome::OKLAB_UNIFORM);
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

/// Checks that the running sample of `image` is drawn, alpha 1, in bounded / nothing yet's swatch, `#141418` (R-446):
/// neither blank (black, or the clear colour) nor any other class's colour.
fn check_running(s: &Scene, image: &[[f32; 4]]) {
    let i = running_sample();
    assert_eq!(
        s.read(i).state,
        outcome::code("running"),
        "a RUNNING sample"
    );
    let want = present::srgb8(hex(0x141418)).map(|c| c as f32);
    for ((x, y), got) in tile(s, image, i) {
        assert!(
            got[3] == 1.0 && got[..3] == want && got[..3] != [0.0; 3],
            "the running sample's pixel ({x}, {y}) is {got:?}, not bounded / nothing yet's {want:?}"
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
            "    return select(uniforms.bounded, uniforms.oklab_bounded, v);",
            "    return vec3<f32>(0.0, 0.0, 0.0);",
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

/// The outcome scene's escapes, bodies 0, 1, 2 and all three (shown escape body 1, 2, 3 and triple ejection), and the
/// `detail` view scene's escapes and collisions, each by `detail` code.
const OUTCOME_ESCAPES: [u32; 4] = [5, 6, 7, 8];
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

/// The `detail` legend payload §2 gives each state code 0–7, as a person reads it (R-449): escape a body, shown 1-based,
/// collision a pair by its two bodies, smaller first, never its pair id (pair `k` the side opposite body `k`, R-22),
/// `3` all three, the failure states their categories; none for bounded, running and the reserved codes.
fn section_2_legend(state: u32) -> Option<Vec<&'static str>> {
    match state {
        0 => Some(vec![
            "escape body 1",
            "escape body 2",
            "escape body 3",
            "triple ejection",
        ]),
        2 => Some(vec![
            "collision 2–3",
            "collision 1–3",
            "collision 1–2",
            "triple collision",
        ]),
        4 => Some(vec![
            "sim_failed: NaN in state",
            "sim_failed: Inf/overflow in state",
            "sim_failed: non-finite derived quantity",
            "sim_failed: reserved",
        ]),
        5 => Some(vec![
            "decode_failed: non-finite decode output",
            "decode_failed: degenerate configuration",
            "decode_failed: invalid mass construction",
            "decode_failed: other/reserved",
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

/// The outcome legend's segment of each state code, by swatch param: keyed by state, as the detail legend is. Every
/// state but escape, collision and `decode_failed` is the bounded / nothing yet class alone (R-446), the reserved codes
/// 6 and 7 too; `decode_failed` has none, its samples drawn blank, in no key (R-511 Z1).
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
            "collision_pair_1",
            "collision_pair_2",
            "triple_collision",
        ],
        &["bounded"],
        &["bounded"],
        &[],
        &["bounded"],
        &["bounded"],
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

// ── REQ-GUI-188: the swatch labels are the display module's names ────────────────────────────────────────────────

/// Checks that each swatch of `sets` is labelled with the class name of its `state` and `detail`, that its class is
/// its own index, that every `(state, detail)` takes a class labelled with its class name, and that no label, nor any
/// key of a state's outcome legend segment, shows an internal id.
fn check_swatch_labels(sets: &[&[outcome::Swatch]]) {
    for set in sets {
        for (k, s) in set.iter().enumerate() {
            assert_eq!(
                Some(s.label),
                display::class_name_of(s.state, s.detail),
                "`{}`'s label",
                s.param
            );
            assert_eq!(
                outcome::class(s.state, s.detail),
                k,
                "`{}`'s class",
                s.param
            );
            assert!(
                !display::shows_an_internal_id(s.label),
                "`{}`'s label `{}` shows an internal id",
                s.param,
                s.label
            );
        }
        for state in 0..8 {
            for detail in 0..4 {
                assert_eq!(
                    Some(set[outcome::class(state, detail)].label),
                    display::class_name_of(state, detail),
                    "the class of state {state}, detail {detail}"
                );
            }
            for k in outcome::segment(state) {
                assert!(!display::shows_an_internal_id(set[k].label));
            }
        }
    }
}

#[test]
fn display_names_swatch_labels_are_the_modules() {
    check_swatch_labels(&[&SWATCHES, &outcome::OKLAB_UNIFORM]);
    assert_eq!(SWATCHES[0].label, "bounded / nothing yet");
    assert_eq!(SWATCHES[5].label, "escape body 1");
}

negative_control!(
    display_names_swatch_labels_are_the_modules,
    "a swatch labelled 0-based fails",
    expected = "`escape_body_0`'s label",
    {
        let mut set = SWATCHES;
        set[5].label = "escape body 0";
        check_swatch_labels(&[&set]);
    }
);
