//! QA's tests for TASK-M2-34, written from REQ-COL-108's and REQ-COL-109's statements and verify details and the
//! rulings they cite (R-467, R-470, R-504, R-505, R-511), not from the implementation. Every expected colour is typed in
//! here: bounded / nothing yet's `#141418` (colour_composition §1.4), R-434's stipple (`#4E4E4E`, its `#909090` dot
//! where bit 1 of `x` and of `y` are both 0), R-16's override magenta `#FF00FF`; the hatch is REQ-COL-055's
//! `debug_invalid` through its CPU mirror, and the NaN / Inf style REQ-COL-110's proposal through its CPU mirror,
//! the calibration under review (R-71).
//!
//! The scene is QA's own: one row of 24 samples, one 8 px tile each, every sample's payload set here (state, detail,
//! `d_min`'s f16 bits, the word's `length`, `theta`'s f32 bits) and REQ-COL-102's lane set per sample. The renders take
//! a canvas background of this file's own, [`BG`], exact in f32 and no palette colour, and a second, [`BG2`], so a pixel
//! that draws the uniform is told from one that draws a constant.
//!
//! - REQ-COL-108 (`qa_m2_34_presentation_*`): through the outcome view, the field ramps of the word `length`, `d_min`
//!   and `theta` (an f32 field with no declared sentinel), each ramp's greyscale (its lightness under its closing post)
//!   and hue (on twilight) forms, and a test stain (length's ramp as the colour, `d_min`'s lightness as the brightness,
//!   Replace-L, `d_min`'s post): a sample whose lane is forbidden draws exactly the background uniform whatever its
//!   payload, and so does a `decode_failed` sample of each `detail` (R-511); `sim_failed` of each `detail`, a reserved
//!   `state` code, `running` in the outcome view, a NaN, a `±∞` that is no declared sentinel and length's 127 draw exactly
//!   `#141418`, a field ramp's `sim_failed` whatever its payload (R-399, R-511 Z3); with the node's invalid colour
//!   overridden, that colour, the background unchanged; `d_min`'s unset `+∞` of a sample that is a system keeps the
//!   stipple; no pixel equals the hatch at its position; a `debug/` field view of the same payload still hatches; every
//!   pixel but the background's is the same with either background. Controls: an occupant that hatches the lane, that
//!   draws the lane `#141418`, that decides the background from `decode_failed`'s detail, that draws `decode_failed`
//!   `#141418`, that draws `sim_failed` blank, that draws a constant for the background; a ramp that hatches its invalid
//!   lane, that draws its lane `#141418`, or that reads a `sim_failed` sample's payload; each fails.
//! - REQ-COL-109 (`qa_m2_34_debug_view_*`): the same scene in View › Debug view through the outcome view and the field
//!   ramps: a system pixel equals the presentation render bit for bit; a forbidden-lane pixel the hatch at its position;
//!   a non-finite sample (`sim_failed` of detail 0, 1 or 2, `decode_failed` of detail 0, a NaN or a `±∞` the ramp reads
//!   that is no declared sentinel) the NaN / Inf style; any other failure the hatch; the three counts the pixels drawn in
//!   each. The NaN / Inf style differs from the hatch at every such pixel, is not one flat colour, and each of its
//!   colours is farther than τ = 0.101 from `#FF00FF` in OKLab (R-505). Controls: a debug view that reads the payload
//!   before the lane, one that draws a NaN `sim_failed` with `debug_invalid`, and counts taken per sample rather than per
//!   pixel (R-408's statistic counts samples' area, not the figure's pixels), each fail.
//!
//! Each test registers its negative controls (R-176).

// The file name `qa_TASK-M2-34` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use render::assemble::{self, Kind, Node, Occupant, Stain, Tier, View};
use render::bind::Context;
use render::colour::field_ramp::{Compaction, FieldRamp, Lut, Map, Ramp, ScalarField};
use render::colour::outcome;
use render::raster::Grid;
use render::registry::{self, ZERO_SOURCE};
use validation::golden_scene::{render_fragment, render_stain_with, Inputs};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

type Rgb = [f64; 3];

// ── The corpus's values, typed in ───────────────────────────────────────────────────────────────────────────────

/// Payload §2's `state` codes.
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;
const SIM_FAILED: u32 = 4;
const DECODE_FAILED: u32 = 5;

/// Bounded / nothing yet's black (colour_composition §1.4), R-467's invalid colour for a presentation view.
const BLACK: u32 = 0x141418;
/// R-434's stipple: the grey and its dot.
const GREY: u32 = 0x4E4E4E;
const DOT: u32 = 0x909090;
/// R-16's plain magenta, a field ramp's override default.
const MAGENTA: u32 = 0xFF00FF;

/// The canvas backgrounds the renders take, linear RGB, exact in f32 and no palette colour: two, so a pixel drawing
/// the uniform follows it.
const BG: [f32; 3] = [0.125, 0.25, 0.5];
const BG2: [f32; 3] = [0.5, 0.375, 0.0625];

/// f16 bits of `d_min` (payload §1; R-271): the unset `+∞`, `−∞`, a quiet NaN, and 0.5.
const F16_INF: u32 = 0x7C00;
const F16_NEG_INF: u32 = 0xFC00;
const F16_NAN: u32 = 0x7E00;
const F16_HALF: u32 = 0x3800;

/// The word's truncation sentinel, `length` 127 (R-136).
const LENGTH_SENTINEL: u32 = 127;

/// The rendered-pixel tolerance for a colour that is not returned unchanged (the stipple, the hatch, the NaN / Inf
/// style, a composed colour): parity_contract §4's ~1e-5 starting figure for one GPU evaluation (R-85).
const TOL: f64 = 1e-5;

fn rgb8(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

/// IEC 61966-2-1's sRGB decode of an 8-bit channel.
fn decode8(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear(h: u32) -> Rgb {
    rgb8(h).map(decode8)
}

/// Ottosson's linear sRGB to OKLab.
fn oklab([r, g, b]: Rgb) -> Rgb {
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let [l, m, s] = [l.cbrt(), m.cbrt(), s.cbrt()];
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_808_380_0 * m - 0.808_675_766_0 * s,
    ]
}

fn dist(a: Rgb, b: Rgb) -> f64 {
    let (p, q) = (oklab(a), oklab(b));
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// τ, the OKLab distance from `#FF00FF` to `#E034C6` (R-429).
fn tau() -> f64 {
    dist(linear(0xFF00FF), linear(0xE034C6))
}

fn frag(x: u32, y: u32) -> [f64; 2] {
    [f64::from(x) + 0.5, f64::from(y) + 0.5]
}

fn hatch(x: u32, y: u32) -> Rgb {
    render::present::debug_invalid(frag(x, y))
}

/// R-434's stipple at pixel `(x, y)`, typed in.
fn stipple(x: u32, y: u32) -> Rgb {
    if (x | y) & 2 == 0 {
        linear(DOT)
    } else {
        linear(GREY)
    }
}

// ── The scene ──────────────────────────────────────────────────────────────────────────────────────────────────

/// `theta`'s stored value: finite, or non-finite by its bits.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Theta {
    Finite(f32),
    Bits(u32),
}

/// A sample: `(state, detail, forbidden, d_min f16 bits, length, theta)`.
#[derive(Clone, Copy, Debug)]
struct Row {
    state: u32,
    detail: u32,
    forbidden: bool,
    d_min: u32,
    length: u32,
    theta: Theta,
}

const fn row(state: u32, detail: u32, forbidden: bool) -> Row {
    Row {
        state,
        detail,
        forbidden,
        d_min: F16_HALF,
        length: 20,
        theta: Theta::Finite(1.0),
    }
}

const fn with(r: Row, d_min: u32, length: u32, theta: Theta) -> Row {
    Row {
        d_min,
        length,
        theta,
        ..r
    }
}

const NAN: Theta = Theta::Bits(0x7FC0_0000);
const NEG_INF: Theta = Theta::Bits(0xFF80_0000);
const POS_INF: Theta = Theta::Bits(0x7F80_0000);

/// QA's rows: systems, `running`, `sim_failed` and `decode_failed` of each `detail` holding valid values, the reserved
/// codes, systems holding NaN, `−∞`, `+∞` and the word's sentinel, the forbidden lane over each kind of payload, a
/// `sim_failed` holding `d_min`'s unset `+∞` and a running sample holding it.
const ROWS: [Row; 24] = [
    row(ESCAPE, 0, false),
    row(BOUNDED, 2, false),
    row(COLLISION, 1, false),
    row(RUNNING, 0, false),
    row(SIM_FAILED, 0, false),
    row(SIM_FAILED, 1, false),
    row(SIM_FAILED, 2, false),
    row(SIM_FAILED, 3, false),
    row(DECODE_FAILED, 0, false),
    row(DECODE_FAILED, 1, false),
    row(DECODE_FAILED, 2, false),
    row(DECODE_FAILED, 3, false),
    row(6, 1, false),
    row(7, 3, false),
    with(row(ESCAPE, 1, false), F16_NAN, LENGTH_SENTINEL, NAN),
    with(
        row(BOUNDED, 0, false),
        F16_NEG_INF,
        LENGTH_SENTINEL,
        NEG_INF,
    ),
    with(row(ESCAPE, 2, false), F16_INF, 30, POS_INF),
    row(ESCAPE, 1, true),
    with(row(SIM_FAILED, 0, true), F16_NAN, 20, NAN),
    row(DECODE_FAILED, 2, true),
    with(row(RUNNING, 1, true), F16_INF, 20, Theta::Finite(0.5)),
    with(
        row(BOUNDED, 3, true),
        F16_HALF,
        LENGTH_SENTINEL,
        Theta::Finite(2.0),
    ),
    with(row(SIM_FAILED, 1, false), F16_INF, 20, Theta::Finite(1.5)),
    with(row(RUNNING, 3, false), F16_INF, 40, Theta::Finite(0.25)),
];

const TILE: u32 = 8;
const N: u32 = ROWS.len() as u32;

fn grid() -> Grid {
    Grid::new([N, 1], 1, 0, TILE).expect("the scene's grid")
}

fn sample_at(g: &Grid, x: u32) -> u32 {
    g.cell(x * TILE + TILE / 2, TILE / 2).sample
}

fn payload() -> Synthetic {
    let g = grid();
    let mut set = Synthetic::flat(g, 6);
    for (x, r) in ROWS.iter().enumerate() {
        let i = sample_at(&g, x as u32);
        let theta = match r.theta {
            Theta::Finite(v) => v,
            Theta::Bits(b) => f32::from_bits(b),
        };
        set.sample(i)
            .state(r.state)
            .detail(r.detail)
            .times(40, 10)
            .word([0x1234_5678, 0, 0, 0], r.length)
            .theta(theta);
        let w = set.simstate(i).packed_a;
        set.sample(i).packed_a((w & 0xFFFF) | (r.d_min << 16));
        set.set_forbidden(i, r.forbidden);
    }
    set
}

fn context() -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid: grid(),
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: 1,
    }
}

/// A stain: source → colour, brightness → Replace-L → (post) → OUT.
fn stain(colour: Occupant, brightness: Occupant, post: Option<String>) -> Stain {
    let node = |kind, occupant, inputs: Vec<Option<usize>>| Node {
        kind,
        occupant,
        inputs,
    };
    let mut nodes = vec![
        node(
            Kind::Source,
            Occupant::Custom(ZERO_SOURCE.to_owned()),
            vec![],
        ),
        node(Kind::Colour, colour, vec![Some(0)]),
        node(Kind::Brightness, brightness, vec![Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("replace_l".to_owned()),
            vec![Some(1), Some(2)],
        ),
    ];
    let mut last = 3;
    if let Some(p) = post {
        nodes.push(node(Kind::Post, Occupant::Custom(p), vec![Some(3)]));
        last = 4;
    }
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"))
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `stain` rendered as `view` with the background `bg`, the override on where `override_on`: its pixels and counts.
fn render(
    h: &GpuHarness,
    stain: &Stain,
    view: View,
    bg: [f32; 3],
    override_on: bool,
) -> (Vec<[f32; 4]>, Option<[u32; 3]>) {
    let r = render_stain_with(
        h.device(),
        h.queue(),
        stain,
        &payload(),
        &context(),
        &|_, name| (override_on && name == "INVALID_OVERRIDE").then(|| vec![1.0]),
        Inputs {
            tier: Tier::FULL,
            view,
            background: bg,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    (r.pixels, r.counts)
}

/// Row `x`'s pixels.
fn tile(x: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..TILE).flat_map(move |py| (x * TILE..(x + 1) * TILE).map(move |px| (px, py)))
}

fn at(image: &[[f32; 4]], (px, py): (u32, u32)) -> [f32; 4] {
    image[(py * N * TILE + px) as usize]
}

fn near(got: [f32; 4], want: Rgb) -> bool {
    (0..3).all(|c| (f64::from(got[c]) - want[c]).abs() <= TOL)
}

// ── REQ-COL-108: what a presentation view draws ─────────────────────────────────────────────────────────────────

/// What a presentation pixel must be.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    /// The canvas background uniform, bit for bit.
    Background,
    /// `#141418`, its 8-bit code's decode as the f32 nearest, bit for bit.
    Black,
    /// The node's override colour, R-16's magenta, bit for bit.
    Override,
    /// R-434's stipple.
    Stipple,
    /// A system's colour, which other tests place: here only neither the background nor the hatch.
    System,
}

/// Which field a ramp reads.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Field {
    Length,
    DMin,
    Theta,
}

/// Whether row `r`'s value of `field` is invalid for its ramp (R-467: NaN, a `±∞` that is no declared sentinel, a
/// stored sentinel), and whether it is `d_min`'s declared unset `+∞` (R-271).
fn value_kind(field: Field, r: &Row) -> (bool, bool) {
    match field {
        Field::Length => (r.length == LENGTH_SENTINEL, false),
        Field::DMin => (
            r.d_min == F16_NAN || r.d_min == F16_NEG_INF,
            r.d_min == F16_INF,
        ),
        Field::Theta => (matches!(r.theta, Theta::Bits(_)), false),
    }
}

/// REQ-COL-108 for a field ramp of `field` at row `r`, overridden where `override_on`: the lane and `decode_failed`
/// the background; `sim_failed` and the reserved codes the invalid colour whatever the payload (R-399, R-511 Z3); then
/// the unset value the stipple, an invalid value the invalid colour; every other sample a system's.
fn ramp_want(field: Field, r: &Row, override_on: bool) -> Want {
    let invalid = if override_on {
        Want::Override
    } else {
        Want::Black
    };
    if r.forbidden || r.state == DECODE_FAILED {
        return Want::Background;
    }
    if r.state == SIM_FAILED || r.state > DECODE_FAILED {
        return invalid;
    }
    match value_kind(field, r) {
        (_, true) => Want::Stipple,
        (true, _) => invalid,
        _ => Want::System,
    }
}

/// REQ-COL-108 and R-446 for the outcome view at row `r`: the lane and `decode_failed` the background; bounded,
/// `running`, `sim_failed` and the reserved codes `#141418`; an escape or a collision its swatch, a system's.
fn outcome_want(r: &Row) -> Want {
    if r.forbidden || r.state == DECODE_FAILED {
        Want::Background
    } else if r.state == ESCAPE || r.state == COLLISION {
        Want::System
    } else {
        Want::Black
    }
}

/// Checks a presentation render, `what`, against `want` per row, with the background `bg`: each pixel alpha 1, bit
/// for bit or within [`TOL`] as its want is, and none the hatch at its position (R-467).
fn check(what: &str, image: &[[f32; 4]], bg: [f32; 3], want: &dyn Fn(&Row) -> Want) {
    let black = linear(BLACK).map(|c| c as f32);
    let magenta = linear(MAGENTA).map(|c| c as f32);
    for (x, r) in ROWS.iter().enumerate() {
        let w = want(r);
        for p in tile(x as u32) {
            let got = at(image, p);
            let rgb = [got[0], got[1], got[2]];
            let ok = got[3] == 1.0
                && match w {
                    Want::Background => rgb == bg,
                    Want::Black => rgb == black,
                    Want::Override => rgb == magenta,
                    Want::Stipple => near(got, stipple(p.0, p.1)),
                    Want::System => rgb != bg,
                };
            assert!(
                ok,
                "{what}: row {x} ({r:?}) pixel {p:?} is {got:?}, not {w:?}"
            );
            assert!(
                !near(got, hatch(p.0, p.1)),
                "{what}: row {x} pixel {p:?} is the hatch's"
            );
        }
    }
    // The background is no palette colour and not `#141418`, so the checks above tell it apart.
    assert!(
        bg != black && bg != magenta,
        "the test's background is a want's"
    );
}

/// The outcome occupant's WGSL with `from` replaced by `to`, which must occur once: a control's occupant.
#[cfg(feature = "controls")]
fn altered_outcome(from: &str, to: &str) -> Occupant {
    assert_eq!(
        outcome::WGSL.matches(from).count(),
        1,
        "the outcome occupant has no single `{from}`"
    );
    Occupant::Custom(outcome::WGSL.replace(from, to))
}

/// The outcome view, its occupant `colour`, rendered with each background and checked; every pixel but the
/// background's the same under both.
fn check_outcome(colour: Occupant) {
    let h = gpu();
    let st = stain(colour, Occupant::None, None);
    let (a, _) = render(&h, &st, View::Presentation, BG, false);
    let (b, _) = render(&h, &st, View::Presentation, BG2, false);
    check("the outcome view", &a, BG, &outcome_want);
    check(
        "the outcome view, the second background",
        &b,
        BG2,
        &outcome_want,
    );
    for (x, r) in ROWS.iter().enumerate() {
        if outcome_want(r) != Want::Background {
            for p in tile(x as u32) {
                assert_eq!(
                    at(&a, p),
                    at(&b, p),
                    "the outcome view: row {x} pixel {p:?} follows the background"
                );
            }
        }
    }
}

#[test]
fn qa_m2_34_presentation_outcome_view() {
    check_outcome(Occupant::BuiltIn(outcome::ID.to_owned()));
}

/// The outcome occupant's first test, the lane and `decode_failed` beside it, as REQ-COL-108 and R-511 put it.
#[cfg(feature = "controls")]
const OUTCOME_BLANK: &str = "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {\n        return canvas_background();";

#[cfg(feature = "controls")]
const BLACK_WGSL: &str = "vec3<f32>(0.00699541, 0.00699541, 0.009134059)";

negative_control!(
    qa_m2_34_presentation_outcome_view,
    "an outcome view that hatches the forbidden lane fails",
    expected = "the outcome view: row 17",
    check_outcome(altered_outcome(
        OUTCOME_BLANK,
        "if (ctx.validity.forbidden) {\n        return debug_invalid(ctx.frag_xy);\n    }\n    if (ctx.sample.state == STATE_DECODE_FAILED) {\n        return canvas_background();"
    ))
);

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_outcome_lane_black {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_outcome_view,
        "an outcome view that draws the forbidden lane #141418 fails",
        expected = "the outcome view: row 17",
        check_outcome(altered_outcome(
            OUTCOME_BLANK,
            &format!("if (ctx.validity.forbidden) {{\n        return {BLACK_WGSL};\n    }}\n    if (ctx.sample.state == STATE_DECODE_FAILED) {{\n        return canvas_background();")
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_outcome_from_detail {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_outcome_view,
        "an outcome view that decides the background from decode_failed's detail fails",
        expected = "the outcome view: row 8",
        check_outcome(altered_outcome(
            "ctx.sample.state == STATE_DECODE_FAILED",
            "(ctx.sample.state == STATE_DECODE_FAILED && ctx.sample.detail != 0u)"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_outcome_decode_failed_black {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_outcome_view,
        "an outcome view that draws decode_failed #141418, as before R-511, fails",
        expected = "the outcome view: row 8",
        check_outcome(altered_outcome(
            "ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED",
            "ctx.validity.forbidden"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_outcome_sim_failed_blank {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_outcome_view,
        "an outcome view that draws sim_failed blank fails",
        expected = "the outcome view: row 4",
        check_outcome(altered_outcome(
            "ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED",
            "ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED || ctx.sample.state == STATE_SIM_FAILED"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_outcome_constant_background {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_outcome_view,
        "an outcome view that draws a constant, the harness's background, not the uniform, fails",
        expected = "row 8",
        check_outcome(altered_outcome(
            OUTCOME_BLANK,
            "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {\n        return vec3<f32>(0.125, 0.25, 0.5);"
        ))
    );
}

/// `field`'s colour ramp on `lut`: `length` and `d_min` their ledger ramps, `theta` lin over [0, 4]; twilight is hue
/// mode's form (colour_composition §4.1, as the task applies it to a field ramp).
fn ramp(field: Field, map: Map) -> FieldRamp {
    let base = match field {
        Field::Length => FieldRamp::length(),
        Field::DMin => FieldRamp::d_min(),
        Field::Theta => Ok(FieldRamp::new(
            ScalarField::payload("theta").unwrap_or_else(|e| panic!("{e}")),
            Compaction::Lin { lo: 0.0, hi: 4.0 },
            Map::ramp(Ramp::Lut(Lut::Viridis)),
        )),
    }
    .unwrap_or_else(|e| panic!("{e}"));
    FieldRamp::new(base.field.clone(), base.compaction, map)
}

/// The three forms of a field ramp of `field`: its colour ramp, its lightness under its closing post (greyscale), and
/// its ramp on twilight (hue), each WGSL edited by `edit` where given.
fn ramp_stains(field: Field, edit: Option<(&str, &str)>) -> Vec<(String, Stain)> {
    let e = |w: String| match edit {
        Some((from, to)) => {
            assert!(w.contains(from), "the ramp has no `{from}`");
            w.replace(from, to)
        }
        None => w,
    };
    let colour = ramp(field, Map::ramp(Ramp::Lut(Lut::Viridis)));
    let light = ramp(field, Map::Lightness);
    let hue = ramp(field, Map::ramp(Ramp::Lut(Lut::Twilight)));
    vec![
        (
            format!("{field:?}'s ramp"),
            stain(Occupant::Custom(e(colour.wgsl())), Occupant::None, None),
        ),
        (
            format!("{field:?}'s greyscale"),
            stain(
                Occupant::None,
                Occupant::Custom(light.wgsl()),
                Some(e(light.style_post())),
            ),
        ),
        (
            format!("{field:?}'s hue"),
            stain(Occupant::Custom(e(hue.wgsl())), Occupant::None, None),
        ),
    ]
}

/// Each form of each field's ramp, overridden where `override_on`, checked with both backgrounds.
fn check_ramps(fields: &[Field], edit: Option<(&str, &str)>, override_on: bool) {
    check_ramp_forms(fields, edit, override_on, "");
}

/// [`check_ramps`] over the forms whose name holds `only`.
fn check_ramp_forms(fields: &[Field], edit: Option<(&str, &str)>, override_on: bool, only: &str) {
    let h = gpu();
    for &field in fields {
        for (what, st) in ramp_stains(field, edit) {
            if !what.contains(only) {
                continue;
            }
            let (a, _) = render(&h, &st, View::Presentation, BG, override_on);
            let (b, _) = render(&h, &st, View::Presentation, BG2, override_on);
            let want = |r: &Row| ramp_want(field, r, override_on);
            check(&what, &a, BG, &want);
            check(&format!("{what}, the second background"), &b, BG2, &want);
        }
    }
}

const FIELDS: [Field; 3] = [Field::Length, Field::DMin, Field::Theta];

#[test]
fn qa_m2_34_presentation_field_ramps() {
    check_ramps(&FIELDS, None, false);
}

#[test]
fn qa_m2_34_presentation_field_ramps_overridden() {
    check_ramps(&FIELDS, None, true);
}

negative_control!(
    qa_m2_34_presentation_field_ramps,
    "a field ramp that hatches its invalid lane fails",
    expected = "Length's ramp: row 4",
    check_ramps(
        &[Field::Length],
        Some(("PRESENT_INVALID", "debug_invalid(ctx.frag_xy)")),
        false
    )
);

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_field_ramps_lane_black {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_field_ramps,
        "a field ramp that draws its forbidden lane #141418 fails",
        expected = "DMin's ramp: row 8",
        check_ramps(
            &[Field::DMin],
            Some(("return canvas_background();", "return PRESENT_INVALID;")),
            false
        )
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_field_ramps_failure_read {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_field_ramps,
        "a field ramp that reads a sim_failed sample's payload, as before physics review P1, fails",
        expected = "DMin's ramp: row 4",
        check_ramps(
            &[Field::DMin],
            Some((
                "ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED",
                "false"
            )),
            false
        )
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_presentation_field_ramps_post_hatches {
    use super::*;
    negative_control!(
        qa_m2_34_presentation_field_ramps,
        "a greyscale post that hatches the invalid lane fails",
        expected = "Theta's greyscale: row 4",
        check_ramp_forms(
            &[Field::Theta],
            Some(("PRESENT_INVALID", "debug_invalid(ctx.frag_xy)")),
            false,
            "greyscale"
        )
    );
}

negative_control!(
    qa_m2_34_presentation_field_ramps_overridden,
    "an override that also recolours the background fails",
    expected = "row 8",
    check_ramps(
        &[Field::Length],
        Some((
            "return canvas_background();",
            "return select(canvas_background(), uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);"
        )),
        true
    )
);

/// The test stain: length's colour ramp as the colour, `d_min`'s lightness as the brightness, Replace-L, `d_min`'s
/// closing post. Only `d_min`'s post's styles are placed: elsewhere a system's composed colour.
fn check_test_stain(edit: Option<(&str, &str)>) {
    let h = gpu();
    let post = ramp(Field::DMin, Map::Lightness).style_post();
    let post = match edit {
        Some((from, to)) => {
            assert!(post.contains(from), "the post has no `{from}`");
            post.replace(from, to)
        }
        None => post,
    };
    let st = stain(
        Occupant::Custom(ramp(Field::Length, Map::ramp(Ramp::Lut(Lut::Viridis))).wgsl()),
        Occupant::Custom(ramp(Field::DMin, Map::Lightness).wgsl()),
        Some(post),
    );
    let (a, _) = render(&h, &st, View::Presentation, BG, false);
    check("the test stain", &a, BG, &|r| {
        ramp_want(Field::DMin, r, false)
    });
}

#[test]
fn qa_m2_34_presentation_test_stain() {
    check_test_stain(None);
}

negative_control!(
    qa_m2_34_presentation_test_stain,
    "a test stain whose post draws sim_failed in the hatch fails",
    expected = "the test stain: row 4",
    check_test_stain(Some(("PRESENT_INVALID", "debug_invalid(ctx.frag_xy)")))
);

/// A `debug/` field view keeps its own treatment (REQ-COL-108's last clause): `debug/generated/<field>` over the same
/// payload, a presentation variant, hatches the row's NaN and `−∞`.
fn check_debug_field(field: &str, rows: &[usize], edit: Option<(&str, &str)>) {
    let id = format!("debug/generated/{field}");
    let mut src = registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no `{id}`"))
        .source;
    if let Some((from, to)) = edit {
        assert!(src.contains(from), "`{id}` has no `{from}`");
        src = src.replace(from, to);
    }
    let h = gpu();
    let st = stain(Occupant::Custom(src), Occupant::None, None);
    let (a, _) = render(&h, &st, View::Presentation, BG, false);
    for &x in rows {
        for p in tile(x as u32) {
            assert!(
                near(at(&a, p), hatch(p.0, p.1)),
                "`{id}`: row {x} pixel {p:?} is {:?}, not the hatch",
                at(&a, p)
            );
        }
    }
}

#[test]
fn qa_m2_34_presentation_debug_field_view_hatches() {
    check_debug_field("d_min", &[14, 15], None);
    check_debug_field("theta", &[14, 15, 16], None);
}

negative_control!(
    qa_m2_34_presentation_debug_field_view_hatches,
    "a debug field view drawing its invalid samples #141418, as a presentation view does, fails",
    expected = "`debug/generated/d_min`: row 14",
    check_debug_field(
        "d_min",
        &[14, 15],
        Some(("debug_invalid(ctx.frag_xy)", BLACK_WGSL))
    )
);

// ── REQ-COL-109: the debug view ─────────────────────────────────────────────────────────────────────────────────

/// A debug-view category (render_gui_spec §G6's "Debug" key), in the counts' order.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cat {
    Forbidden = 0,
    Failure = 1,
    NanInf = 2,
}

/// REQ-COL-109 at row `r` of a stain that reads `reads` (the outcome view reads no scalar): the lane first; then a
/// non-finite sample (`sim_failed` 0–2, `decode_failed` 0, a non-finite value the stain reads that is no declared
/// sentinel); then any other failure; `None` for a system, drawn as the stain draws it. The ramps here read no value of
/// a failed sample (R-399), so a non-finite value read is a system's.
fn category(r: &Row, reads: Option<Field>) -> Option<Cat> {
    let failed = !matches!(r.state, ESCAPE | BOUNDED | COLLISION | RUNNING);
    let payload_non_finite =
        (r.state == SIM_FAILED && r.detail <= 2) || (r.state == DECODE_FAILED && r.detail == 0);
    let read_non_finite = !failed
        && match reads {
            Some(Field::DMin) => r.d_min == F16_NAN || r.d_min == F16_NEG_INF,
            Some(Field::Theta) => matches!(r.theta, Theta::Bits(_)),
            _ => false,
        };
    if r.forbidden {
        Some(Cat::Forbidden)
    } else if payload_non_finite || read_non_finite {
        Some(Cat::NanInf)
    } else if failed {
        Some(Cat::Failure)
    } else {
        None
    }
}

/// Checks the debug render `debug` of a stain reading `reads` against its presentation render `plain`, and its
/// `counts` against the pixels it drew in each category.
fn check_debug(
    what: &str,
    debug: &[[f32; 4]],
    plain: &[[f32; 4]],
    counts: [u32; 3],
    reads: Option<Field>,
) {
    let mut want = [0u32; 3];
    let mut nan_inf: Vec<[f32; 3]> = Vec::new();
    for (x, r) in ROWS.iter().enumerate() {
        let c = category(r, reads);
        for p in tile(x as u32) {
            let got = at(debug, p);
            let ok = match c {
                None => got == at(plain, p),
                Some(Cat::Forbidden | Cat::Failure) => near(got, hatch(p.0, p.1)),
                Some(Cat::NanInf) => {
                    let rgb = [got[0], got[1], got[2]];
                    if !nan_inf.contains(&rgb) {
                        nan_inf.push(rgb);
                    }
                    near(got, render::present::debug_nan_inf(frag(p.0, p.1)))
                        && !near(got, hatch(p.0, p.1))
                }
            };
            assert!(
                ok && got[3] == 1.0,
                "{what}: row {x} ({r:?}) pixel {p:?} is {got:?}, not {c:?}"
            );
            if let Some(c) = c {
                want[c as usize] += 1;
            }
        }
    }
    assert!(
        want.iter().all(|&n| n > 0),
        "{what}: the scene draws no pixel of some category: {want:?}"
    );
    // R-505: each of the style's colours farther than τ from `#FF00FF`; R-467 D6: not one flat colour.
    assert!(
        nan_inf.len() >= 2,
        "{what}: the NaN / Inf style is one flat colour"
    );
    for c in &nan_inf {
        let d = dist(c.map(f64::from), linear(0xFF00FF));
        assert!(
            d > tau(),
            "{what}: the NaN / Inf colour {c:?} is {d} from #FF00FF, within τ"
        );
    }
    assert_eq!(
        counts, want,
        "{what}: the debug counts are not the pixels drawn in each"
    );
}

/// The stains the debug view is checked over: the outcome view, and the field ramps of `d_min` and `theta`.
fn debug_stains() -> Vec<(&'static str, Stain, Option<Field>)> {
    vec![
        (
            "the outcome view",
            stain(
                Occupant::BuiltIn(outcome::ID.to_owned()),
                Occupant::None,
                None,
            ),
            None,
        ),
        (
            "d_min's ramp",
            stain(
                Occupant::Custom(ramp(Field::DMin, Map::ramp(Ramp::Lut(Lut::Viridis))).wgsl()),
                Occupant::None,
                None,
            ),
            Some(Field::DMin),
        ),
        (
            "theta's ramp",
            stain(
                Occupant::Custom(ramp(Field::Theta, Map::ramp(Ramp::Lut(Lut::Viridis))).wgsl()),
                Occupant::None,
                None,
            ),
            Some(Field::Theta),
        ),
    ]
}

/// The debug view, its arm `arm` (the shipped one where `None`), over each stain, its counts mapped by `count`.
fn check_debug_view(arm: Option<&str>, count: &dyn Fn([u32; 3]) -> [u32; 3]) {
    let h = gpu();
    for (what, st, reads) in debug_stains() {
        let (plain, none) = render(&h, &st, View::Presentation, BG, false);
        assert_eq!(
            none, None,
            "{what}: a presentation render returned debug counts"
        );
        let r = match arm {
            None => {
                let (p, c) = render(&h, &st, View::Debug, BG, false);
                (p, c)
            }
            Some(a) => {
                let f = assemble::assemble_arm(&st, Tier::FULL, Some(a))
                    .unwrap_or_else(|e| panic!("{e}"));
                let r = render_fragment(
                    h.device(),
                    h.queue(),
                    &f,
                    Tier::FULL,
                    &payload(),
                    &context(),
                    &|_, _| None,
                    BG,
                )
                .unwrap_or_else(|e| panic!("{e}"));
                (r.pixels, r.counts)
            }
        };
        let counts =
            r.1.unwrap_or_else(|| panic!("{what}: the debug view returned no counts"));
        check_debug(what, &r.0, &plain, count(counts), reads);
    }
}

#[test]
fn qa_m2_34_debug_view_styles_and_counts() {
    check_debug_view(None, &|c| c);
}

/// The arm with `from` replaced by `to`, which must occur once.
#[cfg(feature = "controls")]
fn arm_with(edits: &[(&str, &str)]) -> String {
    let mut a = render::assemble::DEBUG_ARM.to_owned();
    for (from, to) in edits {
        assert_eq!(a.matches(from).count(), 1, "the arm has no single `{from}`");
        a = a.replace(from, to);
    }
    a
}

/// The arm's lane test, as the arm writes it.
#[cfg(feature = "controls")]
const LANE_TEST: &str = "    if (forbidden) {\n        atomicAdd(&debug_counts[0], 1u);\n        return debug_invalid(frag_xy);\n    }\n";

negative_control!(
    qa_m2_34_debug_view_styles_and_counts,
    "a debug view that reads the payload before the lane draws a forbidden sim_failed as a failure",
    expected = "the outcome view: row 18",
    check_debug_view(
        Some(&arm_with(&[
            (LANE_TEST, ""),
            (
                "    return rgb;\n",
                &format!("{LANE_TEST}    return rgb;\n")
            ),
        ])),
        &|c| c
    )
);

#[cfg(feature = "controls")]
mod qa_m2_34_debug_view_nan_hatched {
    use super::*;
    negative_control!(
        qa_m2_34_debug_view_styles_and_counts,
        "a debug view that draws a NaN sim_failed with debug_invalid fails",
        expected = "the outcome view: row 4",
        check_debug_view(
            Some(&arm_with(&[(
                "(s.state == STATE_SIM_FAILED && s.detail <= 2u) || ",
                ""
            )])),
            &|c| c
        )
    );
}

#[cfg(feature = "controls")]
mod qa_m2_34_debug_view_counts_per_sample {
    use super::*;
    negative_control!(
        qa_m2_34_debug_view_styles_and_counts,
        "counts taken per sample, as an area statistic over samples is, not per pixel, fail",
        expected = "the debug counts are not the pixels drawn in each",
        check_debug_view(None, &|c| c.map(|n| n / (TILE * TILE)))
    );
}

#[test]
fn qa_m2_34_debug_view_presentation_has_no_arm() {
    // R-470 G4: the presentation variant is assembled without the debug arm, so it can draw no debug style.
    for (what, st, _) in debug_stains() {
        let p = assemble::assemble_view(&st, Tier::FULL, View::Presentation)
            .unwrap_or_else(|e| panic!("{e}"));
        let d =
            assemble::assemble_view(&st, Tier::FULL, View::Debug).unwrap_or_else(|e| panic!("{e}"));
        check_arms(what, &p.source, &d.source);
    }
}

fn check_arms(what: &str, presentation: &str, debug: &str) {
    assert!(
        !presentation.contains("debug_counts") && !presentation.contains("debug_nan_inf(frag_xy)"),
        "{what}: the presentation variant carries the debug arm"
    );
    assert!(
        debug.contains("debug_counts"),
        "{what}: the debug variant carries no counts"
    );
}

negative_control!(
    qa_m2_34_debug_view_presentation_has_no_arm,
    "a presentation variant assembled with the arm fails",
    expected = "the presentation variant carries the debug arm",
    {
        let (what, st, _) = debug_stains().remove(0);
        let d =
            assemble::assemble_view(&st, Tier::FULL, View::Debug).unwrap_or_else(|e| panic!("{e}"));
        check_arms(what, &d.source, &d.source);
    }
);
