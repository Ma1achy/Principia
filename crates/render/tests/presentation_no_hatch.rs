//! Presentation views draw no hatch (R-467, R-511; REQ-COL-108; TASK-M2-34): on the CPU twin and on the GPU, through
//! the outcome view, the word length's and `d_min`'s field ramps and a test stain, in the default, greyscale and hue
//! modes:
//! - a sample REQ-COL-102's lane marks forbidden draws exactly the background uniform whatever its payload, and so does
//!   a `decode_failed` sample of each `detail` (R-511), the lane decided before every other test, the unset value's
//!   included; `running` (in the outcome view), `sim_failed` of each `detail`, a reserved `state` code, a NaN, a `−∞`
//!   that is no declared sentinel and the length ramp's 127 draw exactly `#141418`, the outcome view's failures at
//!   rank 0 in greyscale and hue mode (R-446, R-448); a field ramp draws `sim_failed` and a reserved code `#141418`
//!   whatever the payload the packer wrote, `d_min`'s unset `+∞` or 0.0 and a valid word (R-467, R-511 Z3), and a
//!   `running` sample's unset `+∞` keeps the "not yet" stipple (`presentation_no_hatch_outcome_view`,
//!   `presentation_no_hatch_field_ramps`, `presentation_no_hatch_test_stain`);
//! - overridden, a ramp's invalid value draws the node's colour, and nothing else changes
//!   (`presentation_no_hatch_override`);
//! - no pixel of any presentation output equals `debug_invalid`'s pattern, and no not-a-system pixel draws a palette
//!   colour (C9, R-477): each equals the background uniform (every check above);
//! - a `decode_failed` sample of each decoder tag, at the chart point the decoder raises it at, has REQ-COL-102's lane
//!   unset, sits outside every share (its legend segment empty) and draws exactly the background
//!   (`presentation_no_hatch_decode_failed_tags`);
//! - a `debug/` field view of the same payload still draws the hatch (`presentation_no_hatch_debug_view_hatches`);
//! - a presentation fragment is assembled without the debug arm (R-470 G4; `presentation_no_hatch_no_debug_arm`).
//!
//! The renders take a background of this file's own, [`BG`], no palette's, so a pixel equal to it is the uniform's. The
//! render reads the lane alone, never the chart point, so a pixel past `[0,1]²` whose lane TASK-M8-05 sets draws as one
//! inside it does. Hue mode for a field ramp is the ramp on twilight, the cyclic LUT, as R-369 applies it: §4.1's hue
//! mode is the outcome class's. Each test registers its negative controls (R-176).

use render::assemble::{self, Occupant, Stain, Tier, View};
use render::colour::field_ramp::{FieldRamp, Invalid, Lut, Map, Ramp, Reads, Shown};
use render::colour::outcome;
use render::present::{self, Rgb};
use validation::chart_class;
use validation::golden_scene::{
    render_stain_with, row_scene, Colouring, Inputs, Look, Scene, STATE_DECODE_FAILED,
    STATE_SIM_FAILED,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

#[path = "support/outcome_modes.rs"]
mod modes;
use modes::{event, greyscale, hue, Mode, K};

#[path = "support/decoder_cases.rs"]
mod decoder_cases;
use decoder_cases::{case_chart, raised, CASES};

/// The canvas background the renders take, linear RGB: a value of this file's own, exact in f32 and no palette's.
const BG: [f32; 3] = [0.25, 0.125, 0.0625];

/// The largest per-channel difference, linear RGB, between a computed pixel and its CPU twin.
const TOL: f64 = 1e-5;

/// The `state` codes (payload §2).
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;

/// `d_min`'s f16 bits: the quiet NaN and `−∞`, which no declared sentinel is (R-436).
const F16_QNAN: u32 = 0x7e00;
const F16_NEG_INF: u32 = 0xfc00;

// ── The scene ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A sample's `d_min`: a value, its f16 bits written whole, or left unset, as a fresh sample is.
#[derive(Clone, Copy, Debug)]
enum DMin {
    Value(f32),
    Bits(u32),
    Unset,
}

/// A sample of the scene: its name, payload `(state, detail, t_end_step)`, word length, `d_min` and lane.
#[derive(Clone, Copy, Debug)]
struct Row {
    name: &'static str,
    payload: (u32, u32, u32),
    length: u32,
    d_min: DMin,
    forbidden: bool,
}

/// The scene: physical outcomes, `running`, `sim_failed` of each `detail` with the payload the packer writes (`d_min`
/// unset, `+∞`, or 0.0, payload §2, R-271, and a word), the reserved codes likewise, an escape with a NaN `d_min` and a
/// bounded sample with the length's 127 and a `−∞` `d_min`, `decode_failed` of each `detail` with both invalid and one
/// with `d_min` unset, and forbidden samples of each kind of payload, values valid, invalid or unset among them.
fn rows() -> Vec<Row> {
    let row = |name, payload, length, d_min, forbidden| Row {
        name,
        payload,
        length,
        d_min,
        forbidden,
    };
    let mut out = vec![
        row("escape", (ESCAPE, 0, 120), 38, DMin::Value(0.6), false),
        row("bounded", (BOUNDED, 0, 1000), 0, DMin::Value(1.7), false),
        row(
            "collision",
            (COLLISION, 2, 40),
            51,
            DMin::Value(0.07),
            false,
        ),
        row("running", (RUNNING, 0, 20), 76, DMin::Unset, false),
    ];
    for d in 0..4 {
        let p = (STATE_SIM_FAILED, d, 40);
        let d_min = if d % 2 == 0 {
            DMin::Unset
        } else {
            DMin::Bits(0)
        };
        out.push(row("sim_failed", p, 23, d_min, false));
    }
    out.push(row("reserved 6", (6, 0, 40), 23, DMin::Unset, false));
    out.push(row("reserved 7", (7, 0, 40), 9, DMin::Bits(0), false));
    out.push(row(
        "escape NaN",
        (ESCAPE, 1, 120),
        38,
        DMin::Bits(F16_QNAN),
        false,
    ));
    out.push(row(
        "bounded -inf",
        (BOUNDED, 0, 1000),
        127,
        DMin::Bits(F16_NEG_INF),
        false,
    ));
    for d in 0..4 {
        let p = (STATE_DECODE_FAILED, d, 0);
        out.push(row("decode_failed", p, 127, DMin::Bits(F16_QNAN), false));
    }
    out.push(row(
        "decode_failed unset",
        (STATE_DECODE_FAILED, 1, 0),
        64,
        DMin::Unset,
        false,
    ));
    out.extend([
        row(
            "forbidden escape",
            (ESCAPE, 1, 120),
            38,
            DMin::Value(0.6),
            true,
        ),
        row(
            "forbidden bounded",
            (BOUNDED, 0, 1000),
            0,
            DMin::Value(1.7),
            true,
        ),
        row("forbidden running", (RUNNING, 0, 20), 76, DMin::Unset, true),
        row(
            "forbidden sim_failed",
            (STATE_SIM_FAILED, 0, 40),
            23,
            DMin::Bits(F16_QNAN),
            true,
        ),
        row(
            "forbidden decode_failed",
            (STATE_DECODE_FAILED, 2, 0),
            127,
            DMin::Unset,
            true,
        ),
        row(
            "forbidden reserved",
            (6, 0, 40),
            127,
            DMin::Bits(F16_NEG_INF),
            true,
        ),
    ]);
    out
}

/// `rows` as a scene coloured by `colouring`, which [`Scene::value`] reads the field through.
fn scene(rows: &[Row], colouring: Colouring) -> Scene {
    let mut s =
        row_scene("presentation", rows.len() as u32, colouring).unwrap_or_else(|e| panic!("{e}"));
    for (i, r) in (0u32..).zip(rows) {
        let (state, detail, t) = r.payload;
        s.set
            .sample(i)
            .state(state)
            .detail(detail)
            .times(t, 0)
            .word([0; 4], r.length);
        match r.d_min {
            DMin::Value(v) => {
                s.set.sample(i).d_min(v);
            }
            DMin::Bits(b) => {
                let w = s.set.simstate(i).packed_a;
                s.set.sample(i).packed_a((w & 0xffff) | (b << 16));
            }
            DMin::Unset => {}
        }
        s.set.set_forbidden(i, r.forbidden);
    }
    s
}

// ── The views and their twins ───────────────────────────────────────────────────────────────────────────────────

/// The modes (colour_composition §4.1).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Look3 {
    Default,
    Greyscale,
    Hue,
}

const MODES: [Look3; 3] = [Look3::Default, Look3::Greyscale, Look3::Hue];

/// A presentation output's want at a pixel: a colour exactly, its f32 bits; one within [`TOL`]; or a system pixel
/// whose colour another test checks, here only not the background and not the hatch.
#[derive(Clone, Copy, Debug)]
enum Want {
    Exact([f32; 3]),
    Near(Rgb),
    System,
}

fn exact(c: Rgb) -> Want {
    Want::Exact(c.map(|v| v as f32))
}

/// A field ramp of `ramp`'s field in `mode`: the ramp itself by default, its lightness in greyscale mode, the ramp on
/// twilight in hue mode.
fn ramp_in(ramp: &FieldRamp, mode: Look3) -> FieldRamp {
    let map = match mode {
        Look3::Default => return ramp.clone(),
        Look3::Greyscale => Map::Lightness,
        Look3::Hue => Map::ramp(Ramp::Lut(Lut::Twilight)),
    };
    FieldRamp::new(ramp.field.clone(), ramp.compaction, map)
}

/// The stain of `ramp` in `mode`: a colour ramp in the colour slot, the brightness None; a lightness in the brightness
/// slot, the colour None, its style post closing the stain.
fn ramp_mode(ramp: &FieldRamp, edit: Option<(&str, &str)>) -> Mode {
    let wgsl = |w: String| match edit {
        Some((from, to)) => {
            assert!(w.contains(from), "the ramp has no `{from}`:\n{w}");
            w.replace(from, to)
        }
        None => w,
    };
    match ramp.map {
        Map::Lightness => Mode {
            name: "greyscale",
            colour: Occupant::None,
            brightness: Occupant::Custom(ramp.wgsl()),
            combiner: "replace_l",
            post: Some(wgsl(ramp.style_post())),
        },
        Map::Ramp(_) => Mode {
            name: "ramp",
            colour: Occupant::Custom(wgsl(ramp.wgsl())),
            brightness: Occupant::None,
            combiner: "replace_l",
            post: None,
        },
    }
}

/// The ramp's CPU twin at sample `i` of `s`, pixel `frag`: [`FieldRamp::shown_in`], then its colour, the invalid lane
/// `invalid`; a lightness's placed value the greyscale `OKLab(L = t, 0, 0)`.
fn ramp_want(ramp: &FieldRamp, s: &Scene, i: u32, frag: [f64; 2], invalid: Invalid) -> Want {
    let (v, gate) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
    let read = s.read(i);
    let shown = ramp.shown_in(v, gate, Reads::HEADER, s.set.forbidden(i), read.state);
    match (
        shown,
        ramp.colour_of(shown, frag, invalid, BG.map(f64::from)),
    ) {
        (Shown::NotYet, Some(c)) | (Shown::Ramp(_), Some(c)) => Want::Near(c),
        (Shown::Ramp(t), None) => Want::Near(present::ramp_grey(t)),
        (_, Some(c)) => exact(c),
        (_, None) => unreachable!("only a placed lightness has no colour"),
    }
}

/// The outcome view's CPU twin in `mode` at a sample of `state` and `detail` and lane `forbidden`: the background, or
/// the class's swatch, rank lightness or hue (R-448).
fn outcome_want(mode: Look3, state: u32, detail: u32, forbidden: bool) -> Want {
    match outcome::shown(state, detail, forbidden) {
        outcome::Shown::Forbidden | outcome::Shown::DecodeFailed => Want::Exact(BG),
        outcome::Shown::Swatch(k) => match mode {
            Look3::Default => exact(present::srgb8(outcome::SWATCHES[k].srgb8)),
            Look3::Greyscale => Want::Near(present::ramp_grey((k as f64 + 0.5) / f64::from(K))),
            Look3::Hue => Want::Near(present::hue_wheel(k as f64 / f64::from(K))),
        },
    }
}

/// The outcome view in `mode`, event mode's occupant `colour` where given.
fn outcome_mode(mode: Look3, colour: Option<String>) -> Mode {
    match (mode, colour) {
        (Look3::Default, Some(c)) => Mode {
            colour: Occupant::Custom(c),
            ..event()
        },
        (Look3::Default, None) => event(),
        (Look3::Greyscale, _) => greyscale(),
        (Look3::Hue, _) => hue(),
    }
}

/// The outcome occupant's WGSL with `from` replaced by `to`: a control's altered occupant.
#[cfg(feature = "controls")]
fn altered(from: &str, to: &str) -> String {
    assert!(outcome::WGSL.contains(from), "the occupant has no `{from}`");
    outcome::WGSL.replace(from, to)
}

/// The outcome occupant drawing the forbidden lane as `colour`, a `decode_failed` sample still the background.
#[cfg(feature = "controls")]
fn lane_drawn(colour: &str) -> String {
    altered(
        "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {",
        &format!(
            "if (ctx.validity.forbidden) {{ return {colour}; }}\n    if (ctx.sample.state == STATE_DECODE_FAILED) {{"
        ),
    )
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `s`'s payload rendered through `mode`'s stain, with the background [`BG`] and the params `param`.
fn render(
    h: &GpuHarness,
    s: &Scene,
    mode: &Mode,
    param: &dyn Fn(&str) -> Option<Vec<f64>>,
) -> Vec<[f32; 4]> {
    let stain: Stain = modes::stain(mode);
    render_stain_with(
        h.device(),
        h.queue(),
        &stain,
        &s.set,
        &s.context,
        &|_, name| param(name),
        Inputs {
            background: BG,
            ..Inputs::default()
        },
    )
    .unwrap_or_else(|e| panic!("{}: {e}", mode.name))
    .pixels
}

fn defaults(_: &str) -> Option<Vec<f64>> {
    None
}

/// Checks `image`, `s` rendered as `what`, against `want` at each pixel of each sample of `rows`, alpha 1: an exact
/// want bit for bit, a near one within [`TOL`], a system pixel neither the background nor the hatch; and no pixel the
/// hatch, `debug_invalid` at its position (R-467).
fn check(
    what: &str,
    rows: &[Row],
    s: &Scene,
    image: &[[f32; 4]],
    want: &dyn Fn(u32, [f64; 2]) -> Want,
) {
    let width = s.size().0;
    for (i, r) in (0u32..).zip(rows) {
        for (x, y) in s.pixels(i) {
            let got = image[(y * width + x) as usize];
            let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let rgb = [got[0], got[1], got[2]];
            let near = |c: Rgb| (0..3).all(|k| (f64::from(rgb[k]) - c[k]).abs() <= TOL);
            let w = want(i, frag);
            let ok = got[3] == 1.0
                && match w {
                    Want::Exact(c) => rgb == c,
                    Want::Near(c) => near(c),
                    Want::System => rgb != BG,
                };
            assert!(
                ok,
                "{what}: {} {:?} (sample {i}) pixel ({x}, {y}) is {got:?}, not {w:?}",
                r.name, r.payload
            );
            assert!(
                !near(present::debug_invalid(frag)),
                "{what}: {} {:?} (sample {i}) pixel ({x}, {y}) is the hatch",
                r.name,
                r.payload
            );
        }
    }
}

// ── The outcome view ────────────────────────────────────────────────────────────────────────────────────────────

/// The outcome view in every mode, event mode's occupant `colour` where given, against its twin.
fn check_outcome(colour: Option<String>) {
    let h = gpu();
    let rows = rows();
    let s = scene(
        &rows,
        Colouring::Ramp {
            ramp: FieldRamp::length().unwrap_or_else(|e| panic!("{e}")),
            override_on: false,
        },
    );
    for mode in MODES {
        let image = render(&h, &s, &outcome_mode(mode, colour.clone()), &defaults);
        check(
            &format!("the outcome view, {mode:?}"),
            &rows,
            &s,
            &image,
            &|i, _| {
                let (state, detail, _) = rows[i as usize].payload;
                outcome_want(mode, state, detail, s.set.forbidden(i))
            },
        );
    }
}

#[test]
fn presentation_no_hatch_outcome_view() {
    check_outcome(None);
}

negative_control!(
    presentation_no_hatch_outcome_view,
    "an outcome occupant drawing the forbidden lane in the hatch fails",
    expected = "the outcome view, Default: forbidden escape",
    check_outcome(Some(lane_drawn("debug_invalid(ctx.frag_xy)")))
);

#[cfg(feature = "controls")]
mod presentation_no_hatch_outcome_view_controls {
    use super::*;

    mod sim_failed_hatched {
        use super::*;
        negative_control!(
            presentation_no_hatch_outcome_view,
            "an outcome occupant drawing sim_failed in the hatch fails",
            expected = "the outcome view, Default: sim_failed (4, 0, 40)",
            check_outcome(Some(altered(
                "    return select(uniforms.bounded, uniforms.oklab_bounded, v);",
                "    if (s == STATE_SIM_FAILED) { return debug_invalid(ctx.frag_xy); }\n    \
                 return select(uniforms.bounded, uniforms.oklab_bounded, v);",
            )))
        );
    }

    mod lane_black {
        use super::*;
        negative_control!(
            presentation_no_hatch_outcome_view,
            "an outcome occupant drawing the forbidden lane #141418, not the background, fails",
            expected = "the outcome view, Default: forbidden escape",
            check_outcome(Some(lane_drawn("PRESENT_INVALID")))
        );
    }

    mod lane_palette {
        use super::*;
        negative_control!(
            presentation_no_hatch_outcome_view,
            "an outcome occupant mapping the forbidden lane to a palette entry fails (C9, R-477)",
            expected = "the outcome view, Default: forbidden escape",
            check_outcome(Some(lane_drawn("uniforms.escape_body_1")))
        );
    }

    mod decode_failed_by_detail {
        use super::*;
        negative_control!(
            presentation_no_hatch_outcome_view,
            "an outcome occupant deciding the background from decode_failed's detail fails",
            expected = "the outcome view, Default: decode_failed (5, 0, 0)",
            check_outcome(Some(altered(
                "ctx.sample.state == STATE_DECODE_FAILED) {",
                "(ctx.sample.state == STATE_DECODE_FAILED && ctx.sample.detail != 0u)) {",
            )))
        );
    }

    mod decode_failed_black {
        use super::*;
        negative_control!(
            presentation_no_hatch_outcome_view,
            "an outcome occupant drawing decode_failed #141418 fails (R-511)",
            expected = "the outcome view, Default: decode_failed (5, 0, 0)",
            check_outcome(Some(altered(
                "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {",
                "if (ctx.validity.forbidden) {",
            )))
        );
    }

    mod sim_failed_blank {
        use super::*;
        negative_control!(
            presentation_no_hatch_outcome_view,
            "an outcome occupant drawing sim_failed blank fails (R-511: sim_failed keeps #141418)",
            expected = "the outcome view, Default: sim_failed (4, 0, 40)",
            check_outcome(Some(altered(
                "ctx.sample.state == STATE_DECODE_FAILED) {",
                "ctx.sample.state == STATE_DECODE_FAILED || ctx.sample.state == STATE_SIM_FAILED) {",
            )))
        );
    }
}

// ── The field ramps ─────────────────────────────────────────────────────────────────────────────────────────────

/// The two field ramps, the word length's and `d_min`'s.
fn ramps() -> [FieldRamp; 2] {
    [FieldRamp::length(), FieldRamp::d_min()].map(|r| r.unwrap_or_else(|e| panic!("{e}")))
}

/// Each ramp in every mode, the ramp's WGSL (or its post's) edited by `edit` where given, against its twin; with
/// `override_on`, its invalid lane overridden to the node's colour, R-16's magenta by default.
fn check_ramps(edit: Option<(&str, &str)>, override_on: bool) {
    let h = gpu();
    let rows = rows();
    let invalid = match override_on {
        true => Invalid::Colour(render::colour::field_ramp::override_default()),
        false => Invalid::Default,
    };
    let param = |name: &str| (override_on && name == "INVALID_OVERRIDE").then(|| vec![1.0]);
    for base in ramps() {
        let s = scene(
            &rows,
            Colouring::Ramp {
                ramp: base.clone(),
                override_on,
            },
        );
        for mode in MODES {
            let ramp = ramp_in(&base, mode);
            let image = render(&h, &s, &ramp_mode(&ramp, edit), &param);
            let what = format!("`{}`'s ramp, {mode:?}", base.field.field);
            check(&what, &rows, &s, &image, &|i, frag| {
                ramp_want(&ramp, &s, i, frag, invalid)
            });
        }
    }
}

#[test]
fn presentation_no_hatch_field_ramps() {
    check_ramps(None, false);
}

negative_control!(
    presentation_no_hatch_field_ramps,
    "a ramp drawing its invalid values in the hatch fails",
    expected = "`length`'s ramp, Default: sim_failed (4, 0, 40)",
    check_ramps(
        Some((
            "return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);",
            "return debug_invalid(ctx.frag_xy);",
        )),
        false
    )
);

#[cfg(feature = "controls")]
mod presentation_no_hatch_field_ramps_controls {
    use super::*;

    mod lane_black {
        use super::*;
        negative_control!(
            presentation_no_hatch_field_ramps,
            "a ramp drawing the forbidden lane #141418, not the background, fails",
            expected = "`length`'s ramp, Default: decode_failed (5, 0, 0)",
            check_ramps(
                Some(("return canvas_background();", "return PRESENT_INVALID;")),
                false
            )
        );
    }

    mod decode_failed_black {
        use super::*;
        negative_control!(
            presentation_no_hatch_field_ramps,
            "a ramp deciding blank from the lane alone draws decode_failed's invalid value #141418, and fails (R-511)",
            expected = "`length`'s ramp, Default: decode_failed (5, 0, 0)",
            check_ramps(
                Some((
                    "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) { return canvas_background(); }",
                    "if (ctx.validity.forbidden && ctx.sample.state != STATE_DECODE_FAILED) { return canvas_background(); }",
                )),
                false
            )
        );
    }

    /// The ramp's failed-state test, as [`FieldRamp::wgsl`] writes it.
    const FAILED: &str =
        "if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) {";

    mod failed_on_the_ramp {
        use super::*;
        negative_control!(
            presentation_no_hatch_field_ramps,
            "a ramp without the failed-state test places sim_failed's payload, its word's length on the ramp, and fails",
            expected = "`length`'s ramp, Default: sim_failed (4, 0, 40)",
            check_ramps(Some((FAILED, "if (false) {")), false)
        );
    }

    mod failed_unset_stippled {
        use super::*;
        negative_control!(
            presentation_no_hatch_field_ramps,
            "a ramp drawing a sim_failed sample's unset +inf d_min in the \"not yet\" stipple fails (R-467, R-511 Z3)",
            expected = "`d_min`'s ramp, Default: sim_failed (4, 0, 40)",
            check_ramps(
                Some((
                    FAILED,
                    "if ((ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) \
                     && bitcast<u32>(v) != 0x7f800000u) {",
                )),
                false
            )
        );
    }

    mod reserved_on_the_ramp {
        use super::*;
        negative_control!(
            presentation_no_hatch_field_ramps,
            "a ramp whose failed-state test leaves out the reserved codes places them, and fails",
            expected = "`length`'s ramp, Default: reserved 6",
            check_ramps(
                Some((FAILED, "if (ctx.sample.state == STATE_SIM_FAILED) {")),
                false
            )
        );
    }
}

#[test]
fn presentation_no_hatch_override() {
    check_ramps(None, true);
}

negative_control!(
    presentation_no_hatch_override,
    "a ramp ignoring its node's override draws #141418, not the node's colour, and fails",
    expected = "`length`'s ramp, Default: sim_failed (4, 0, 40)",
    check_ramps(
        Some((
            "uniforms.INVALID_OVERRIDE != 0u);",
            "uniforms.INVALID_OVERRIDE == 2u);"
        )),
        true
    )
);

// ── A test stain ────────────────────────────────────────────────────────────────────────────────────────────────

/// The test stain: the outcome palette as the colour, `d_min`'s lightness as the brightness, Replace L, and the
/// lightness's style post, its WGSL edited by `edit` where given.
fn test_stain(edit: Option<(&str, &str)>) -> (FieldRamp, Mode) {
    let d_min = ramp_in(&ramps()[1], Look3::Greyscale);
    let post = ramp_mode(&d_min, edit).post;
    let mode = Mode {
        name: "test stain",
        colour: Occupant::BuiltIn(outcome::ID.to_owned()),
        brightness: Occupant::Custom(d_min.wgsl()),
        combiner: "replace_l",
        post,
    };
    (d_min, mode)
}

/// The test stain against its twin: its post's styles exactly, every other pixel a system's.
fn check_test_stain(edit: Option<(&str, &str)>) {
    let h = gpu();
    let rows = rows();
    let (d_min, mode) = test_stain(edit);
    let s = scene(
        &rows,
        Colouring::Ramp {
            ramp: ramps()[1].clone(),
            override_on: false,
        },
    );
    let image = render(&h, &s, &mode, &defaults);
    check("the test stain", &rows, &s, &image, &|i, frag| {
        let (v, gate) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
        let shown = d_min.shown_in(v, gate, Reads::HEADER, s.set.forbidden(i), s.read(i).state);
        match shown {
            Shown::Ramp(_) => Want::System,
            _ => ramp_want(&d_min, &s, i, frag, Invalid::Default),
        }
    });
}

#[test]
fn presentation_no_hatch_test_stain() {
    check_test_stain(None);
}

negative_control!(
    presentation_no_hatch_test_stain,
    "a post without the lane's test draws a forbidden sample's colour, and fails",
    expected = "the test stain: decode_failed (5, 0, 0)",
    check_test_stain(Some((
        "    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) { return canvas_background(); }\n",
        "",
    )))
);

// ── decode_failed of each decoder tag (R-511) ───────────────────────────────────────────────────────────────────

/// Each decoder case at the chart point the decoder raises it at, the lane from the chart there, under each
/// `decode_failed` detail (REQ-DEC-038 gives no map from tag to detail): the lane unset, the legend's segment for
/// `decode_failed` empty, so outside every share, and the outcome view and the length ramp drawing exactly the
/// background, event mode's occupant `colour` where given.
fn check_tags(colour: Option<String>) {
    assert!(
        outcome::segment(STATE_DECODE_FAILED).is_empty(),
        "decode_failed has a legend segment"
    );
    for state in [ESCAPE, BOUNDED, COLLISION, RUNNING, STATE_SIM_FAILED, 6, 7] {
        assert!(
            !outcome::segment(state).is_empty(),
            "state {state} has no segment"
        );
    }
    let mut rows = Vec::new();
    for case in CASES {
        let chart = case_chart(case);
        assert!(raised(case, &chart), "the decoder does not raise {case:?}");
        let forbidden = chart_class::forbidden(&chart, 0.5, 0.5)
            .unwrap_or_else(|| panic!("{case:?} is inside [0,1]²"));
        assert!(!forbidden, "{case:?}: REQ-COL-102's lane is set");
        for d in 0..4 {
            rows.push(Row {
                name: "decoder case",
                payload: (STATE_DECODE_FAILED, d, 0),
                length: 38,
                d_min: DMin::Value(0.6),
                forbidden,
            });
        }
    }
    let h = gpu();
    let s = scene(
        &rows,
        Colouring::Ramp {
            ramp: ramps()[0].clone(),
            override_on: false,
        },
    );
    let image = render(&h, &s, &outcome_mode(Look3::Default, colour), &defaults);
    check("the outcome view", &rows, &s, &image, &|_, _| {
        Want::Exact(BG)
    });
    let image = render(&h, &s, &ramp_mode(&ramps()[0], None), &defaults);
    check("the length ramp", &rows, &s, &image, &|_, _| {
        Want::Exact(BG)
    });
}

#[test]
fn presentation_no_hatch_decode_failed_tags() {
    check_tags(None);
}

negative_control!(
    presentation_no_hatch_decode_failed_tags,
    "an outcome occupant drawing decode_failed #141418 fails (R-511)",
    expected = "the outcome view: decoder case (5, 0, 0)",
    check_tags(Some(altered(
        "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {",
        "if (ctx.validity.forbidden) {",
    )))
);

// ── The debug views keep the hatch ──────────────────────────────────────────────────────────────────────────────

/// The scene's payload through `colouring`: the samples [`Scene::look`] gives the hatch drawn in it, there being some.
fn check_debug_hatches(colouring: Colouring) {
    let rows = rows();
    let s = scene(&rows, colouring);
    let h = gpu();
    let image: Vec<Rgb> = s
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect();
    let width = s.size().0;
    let mut hatched = 0;
    for i in 0..rows.len() as u32 {
        if s.look(i).unwrap_or_else(|e| panic!("{e}")) != Look::Invalid {
            continue;
        }
        hatched += 1;
        for (x, y) in s.pixels(i) {
            let want = present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
            let got = image[(y * width + x) as usize];
            assert!(
                (0..3).all(|k| (got[k] - want[k]).abs() <= TOL),
                "`{}` sample {i} pixel ({x}, {y}) is {got:?}, not the hatch {want:?}",
                s.field()
            );
        }
    }
    assert!(hatched > 0, "`{}` draws no sample in the hatch", s.field());
}

#[test]
fn presentation_no_hatch_debug_view_hatches() {
    check_debug_hatches(Colouring::View("d_min"));
}

negative_control!(
    presentation_no_hatch_debug_view_hatches,
    "d_min's presentation ramp, not its debug view, draws no sample in the hatch",
    expected = "`d_min` draws no sample in the hatch",
    check_debug_hatches(Colouring::Ramp {
        ramp: FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}")),
        override_on: false,
    })
);

// ── No debug arm in a presentation fragment (R-470 G4) ──────────────────────────────────────────────────────────

/// Every stain above assembled as `view`: none carries the debug arm, its counts' binding or its `debug_view` call.
fn check_no_arm(view: View) {
    let mut stains: Vec<Mode> = MODES.iter().map(|&m| outcome_mode(m, None)).collect();
    for base in ramps() {
        stains.extend(MODES.iter().map(|&m| ramp_mode(&ramp_in(&base, m), None)));
    }
    stains.push(test_stain(None).1);
    for mode in &stains {
        let f = assemble::assemble_view(&modes::stain(mode), Tier::FULL, view)
            .unwrap_or_else(|e| panic!("{}: {e}", mode.name));
        assert!(
            !f.debug_arm && !f.source.contains("debug_counts") && !f.source.contains("debug_view("),
            "{}: the presentation fragment carries the debug arm",
            mode.name
        );
    }
}

#[test]
fn presentation_no_hatch_no_debug_arm() {
    check_no_arm(View::Presentation);
}

negative_control!(
    presentation_no_hatch_no_debug_arm,
    "the debug variant carries its arm",
    expected = "the presentation fragment carries the debug arm",
    check_no_arm(View::Debug)
);
