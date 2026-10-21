//! View › Debug view and its styles (R-467 clause 3, D5, D6; R-470 G4; REQ-COL-109, REQ-COL-110; TASK-M2-34):
//! - REQ-COL-109 (`debug_view_styles_over_the_stains`): on the CPU twin (`render::debug_view`) and on the GPU, a
//!   payload with valid outcomes, a forbidden-lane sample of each kind of payload, `sim_failed` of each `detail`,
//!   `decode_failed` of each `detail`, the reserved `state` codes and non-finite values the stain reads, through the
//!   outcome view and `d_min`'s field ramp: system pixels equal the stain's output with the view off, bit for bit;
//!   forbidden-lane pixels equal `debug_invalid` at their positions; non-finite samples the NaN / Inf style at theirs;
//!   other failures `debug_invalid`; the three counts the numbers of pixels drawn in each. Controls: a debug view that
//!   reads the payload before the lane, one that draws a NaN `sim_failed` with `debug_invalid`, and one counting as
//!   R-408's statistic does, its forbidden pixels out of the count, each fail. The twin's order and its key's labels
//!   (`debug_view_styles_twin_order`).
//! - REQ-COL-110 (`debug_view_styles_nan_inf_mirror`, `debug_view_styles_nan_inf_proposal`): `debug_nan_inf` on the
//!   GPU is its CPU mirror at every pixel, its colours `present.wgsl`'s constants, and `PRESENT_INVALID` is
//!   `#141418`'s; the proposal's measurement, against every palette entry REQ-COL-055 measures (the outcome palette's
//!   two sets, the `dbg_*` palettes, the LUTs, the grey ramp and the hue circle), the hatch, the stipple and `#FF00FF`,
//!   unsimulated and under each colour-vision simulation, the tightest distances printed for the gui reviewer and the
//!   M2 gate (run with `--nocapture`).
//!
//! The debug variant is a baked pipeline variant (R-41; R-470 G4), assembled with `render::assemble::View::Debug`; the
//! controls render an altered arm through `render::assemble::assemble_arm`. Each test registers its negative control
//! (R-176).

use std::path::Path;

use ledger::gen::prelude;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier, View};
use render::colour::field_ramp::{FieldRamp, Reads, Shown};
use render::colour::outcome;
use render::debug_view::{self, Category, Sample};
use render::display::cvd::{self, CvdMode};
use render::present::{self, Rgb};
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{
    render_fragment, render_stain_with, row_scene, Colouring, Inputs, Scene, STATE_DECODE_FAILED,
    STATE_SIM_FAILED,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

#[path = "support/palettes.rs"]
mod palettes;

/// The canvas background the renders take, linear RGB, a value of this file's own.
const BG: [f32; 3] = [0.25, 0.125, 0.0625];

/// The largest per-channel difference, linear RGB, between a computed style and its CPU mirror.
const TOL: f64 = 1e-5;

/// The `state` codes (payload §2).
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;

/// `d_min`'s f16 bits: the quiet NaN and `−∞`.
const F16_QNAN: u32 = 0x7e00;
const F16_NEG_INF: u32 = 0xfc00;

// ── The scene ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A sample of the scene: its name, payload `(state, detail, t_end_step)`, `d_min`'s f16 bits (`None` for unset) and
/// its lane.
#[derive(Clone, Copy, Debug)]
struct Row {
    name: &'static str,
    payload: (u32, u32, u32),
    d_min: Option<u32>,
    forbidden: bool,
}

/// `d_min` 0.6, as f16 bits.
const D_MIN: u32 = 0x38cd;

/// The scene: valid outcomes, `running` with `d_min` unset, valid outcomes whose `d_min` is NaN and `−∞`; `sim_failed`
/// and `decode_failed` of each `detail` and the reserved codes, `d_min` finite; and forbidden samples of each kind of
/// payload, a NaN `d_min` among them.
fn rows() -> Vec<Row> {
    let row = |name, payload, d_min, forbidden| Row {
        name,
        payload,
        d_min,
        forbidden,
    };
    let mut out = vec![
        row("escape", (ESCAPE, 0, 120), Some(D_MIN), false),
        row("bounded", (BOUNDED, 0, 1000), Some(D_MIN), false),
        row("collision", (COLLISION, 2, 40), Some(D_MIN), false),
        row("running", (RUNNING, 0, 20), None, false),
        row("escape, d_min NaN", (ESCAPE, 1, 120), Some(F16_QNAN), false),
        row(
            "bounded, d_min −∞",
            (BOUNDED, 0, 1000),
            Some(F16_NEG_INF),
            false,
        ),
    ];
    for d in 0..4 {
        out.push(row(
            "sim_failed",
            (STATE_SIM_FAILED, d, 40),
            Some(D_MIN),
            false,
        ));
    }
    for d in 0..4 {
        out.push(row(
            "decode_failed",
            (STATE_DECODE_FAILED, d, 0),
            Some(D_MIN),
            false,
        ));
    }
    out.push(row("reserved 6", (6, 0, 40), Some(D_MIN), false));
    out.push(row("reserved 7", (7, 0, 40), Some(D_MIN), false));
    out.extend([
        row(
            "forbidden sim_failed",
            (STATE_SIM_FAILED, 3, 40),
            Some(D_MIN),
            true,
        ),
        row(
            "forbidden sim_failed NaN",
            (STATE_SIM_FAILED, 0, 40),
            Some(D_MIN),
            true,
        ),
        row("forbidden escape", (ESCAPE, 2, 120), Some(D_MIN), true),
        row("forbidden running", (RUNNING, 0, 20), None, true),
        row(
            "forbidden decode_failed",
            (STATE_DECODE_FAILED, 0, 0),
            Some(D_MIN),
            true,
        ),
        row(
            "forbidden decode_failed",
            (STATE_DECODE_FAILED, 2, 0),
            Some(D_MIN),
            true,
        ),
        row("forbidden reserved", (6, 0, 40), Some(D_MIN), true),
        row(
            "forbidden, d_min NaN",
            (BOUNDED, 0, 1000),
            Some(F16_QNAN),
            true,
        ),
    ]);
    out
}

/// The scene, its field `d_min`'s ramp.
fn scene(rows: &[Row]) -> Scene {
    let ramp = FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}"));
    let mut s = row_scene(
        "debug view",
        rows.len() as u32,
        Colouring::Ramp {
            ramp,
            override_on: false,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    for (i, r) in (0u32..).zip(rows) {
        let (state, detail, t) = r.payload;
        s.set.sample(i).state(state).detail(detail).times(t, 0);
        if let Some(bits) = r.d_min {
            let w = s.set.simstate(i).packed_a;
            s.set.sample(i).packed_a((w & 0xffff) | (bits << 16));
        }
        s.set.set_forbidden(i, r.forbidden);
    }
    s
}

// ── The two stains, presentation and debug ──────────────────────────────────────────────────────────────────────

/// The stains the view is drawn over: the outcome view in event mode and `d_min`'s field ramp.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Over {
    Outcome,
    DMin,
}

/// The stain `over`: the zero source into its colour, the brightness None, Replace L and `OUT`.
fn stain(over: Over) -> Stain {
    let colour = match over {
        Over::Outcome => Occupant::BuiltIn(outcome::ID.to_owned()),
        Over::DMin => Occupant::Custom(FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}")).wgsl()),
    };
    let node = |kind, occupant, inputs: Vec<Option<usize>>| Node {
        kind,
        occupant,
        inputs,
    };
    Stain::new(vec![
        node(
            Kind::Source,
            Occupant::Custom(ZERO_SOURCE.to_owned()),
            vec![],
        ),
        node(Kind::Colour, colour, vec![Some(0)]),
        node(Kind::Brightness, Occupant::None, vec![Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("replace_l".to_owned()),
            vec![Some(1), Some(2)],
        ),
        node(Kind::Out, Occupant::None, vec![Some(3)]),
    ])
    .unwrap_or_else(|e| panic!("{over:?}: {e}"))
}

/// Whether the stain `over` read a non-finite value at sample `i` of `s`, noting it for the debug view: a field ramp
/// whose invalid lane takes a non-finite value (R-436), after its blank and unset tests.
fn stain_non_finite(over: Over, s: &Scene, i: u32) -> bool {
    if over == Over::Outcome {
        return false;
    }
    let ramp = FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}"));
    let (v, gate) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
    let shown = ramp.shown_in(v, gate, Reads::HEADER, s.set.forbidden(i), s.read(i).state);
    shown == Shown::Invalid && ramp.field.is_non_finite(v, gate)
}

/// The twin's sample `i` of `s` under the stain `over`.
fn sample(over: Over, s: &Scene, i: u32) -> Sample {
    let read = s.read(i);
    Sample {
        forbidden: s.set.forbidden(i),
        state: read.state,
        detail: read.detail,
        stain_non_finite: stain_non_finite(over, s, i),
    }
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `s` drawn over `over` with the debug view's `arm` (`None` for the view as built, [`View::Debug`]), and the counts.
fn render_debug(
    h: &GpuHarness,
    s: &Scene,
    over: Over,
    arm: Option<&str>,
) -> (Vec<[f32; 4]>, [u32; 3]) {
    let stain = stain(over);
    let r = match arm {
        None => render_stain_with(
            h.device(),
            h.queue(),
            &stain,
            &s.set,
            &s.context,
            &|_, _| None,
            Inputs {
                view: View::Debug,
                background: BG,
                ..Inputs::default()
            },
        ),
        Some(arm) => {
            let f = assemble::assemble_arm(&stain, Tier::FULL, Some(arm))
                .unwrap_or_else(|e| panic!("{e}"));
            render_fragment(
                h.device(),
                h.queue(),
                &f,
                Tier::FULL,
                &s.set,
                &s.context,
                &|_, _| None,
                BG,
            )
        }
    }
    .unwrap_or_else(|e| panic!("{e}"));
    (r.pixels, r.counts.expect("the debug view's counts"))
}

/// `s` drawn over `over`, the view off: the presentation variant.
fn render_off(h: &GpuHarness, s: &Scene, over: Over) -> Vec<[f32; 4]> {
    let r = render_stain_with(
        h.device(),
        h.queue(),
        &stain(over),
        &s.set,
        &s.context,
        &|_, _| None,
        Inputs {
            background: BG,
            ..Inputs::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.counts, None, "the presentation variant counts nothing");
    r.pixels
}

/// The debug view over each stain, its arm `arm`, against its twin: each pixel its category's style, or, a system's,
/// the view-off render's bit for bit; and the counts the twin's over every pixel.
fn check_over(arm: Option<&str>) {
    let h = gpu();
    let rows = rows();
    let s = scene(&rows);
    let width = s.size().0;
    for over in [Over::Outcome, Over::DMin] {
        let off = render_off(&h, &s, over);
        let (on, counts) = render_debug(&h, &s, over, arm);
        let mut twin = Vec::new();
        for (i, r) in (0u32..).zip(&rows) {
            let smp = sample(over, &s, i);
            for (x, y) in s.pixels(i) {
                twin.push(smp);
                let k = (y * width + x) as usize;
                let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                let ok = on[k][3] == 1.0
                    && match debug_view::category(smp) {
                        None => on[k].map(f32::to_bits) == off[k].map(f32::to_bits),
                        Some(c) => {
                            (0..3).all(|j| (f64::from(on[k][j]) - c.colour(frag)[j]).abs() <= TOL)
                        }
                    };
                assert!(
                    ok,
                    "{over:?}: {} {:?} (sample {i}) pixel ({x}, {y}) is {:?}, not {:?} (the view off: {:?})",
                    r.name,
                    r.payload,
                    on[k],
                    debug_view::category(smp),
                    off[k]
                );
            }
        }
        assert_eq!(counts, debug_view::counts(twin), "{over:?}: the counts");
        assert!(
            counts.iter().all(|&n| n > 0),
            "{over:?}: every category drawn"
        );
    }
}

/// The debug arm with `from` replaced by `to`: a control's altered arm.
#[cfg(feature = "controls")]
fn arm_with(from: &str, to: &str) -> String {
    assert!(
        assemble::DEBUG_ARM.contains(from),
        "the arm has no `{from}`"
    );
    assemble::DEBUG_ARM.replace(from, to)
}

#[test]
fn debug_view_styles_over_the_stains() {
    check_over(None);
}

negative_control!(
    debug_view_styles_over_the_stains,
    "a debug view reading the payload before the lane draws a forbidden sim_failed as a failure, and fails",
    expected = "Outcome: forbidden sim_failed NaN (4, 0, 40)",
    {
        let lane = "    if (forbidden) {\n        atomicAdd(&debug_counts[0], 1u);\n        return debug_invalid(frag_xy);\n    }\n";
        let last = arm_with(lane, "");
        assert!(last.contains("    return rgb;\n}"), "the arm has no last return");
        check_over(Some(&last.replace("    return rgb;\n}", &format!("{lane}    return rgb;\n}}"))))
    }
);

#[cfg(feature = "controls")]
mod debug_view_styles_over_the_stains_controls {
    use super::*;

    mod nan_as_failure {
        use super::*;
        negative_control!(
            debug_view_styles_over_the_stains,
            "a debug view drawing a NaN sim_failed with debug_invalid fails",
            expected = "Outcome: sim_failed (4, 0, 40)",
            check_over(Some(&arm_with(
                "return debug_nan_inf(frag_xy);",
                "return debug_invalid(frag_xy);",
            )))
        );
    }

    mod r408_count {
        use super::*;
        negative_control!(
            debug_view_styles_over_the_stains,
            "a count taken as R-408's statistic, the forbidden pixels out of it, fails",
            expected = "Outcome: the counts",
            check_over(Some(&arm_with(
                "        atomicAdd(&debug_counts[0], 1u);\n",
                ""
            )))
        );
    }
}

// ── The twin's order and the key ────────────────────────────────────────────────────────────────────────────────

/// Checks `category` over every `state` 0 to 7, `detail` 0 to 3, the lane and the stain's note: the lane first, then
/// non-finiteness (`sim_failed` 0, 1 and 2, `decode_failed` 0, or the stain's note), then any other failure; and the
/// key's labels.
fn check_order(category: fn(Sample) -> Option<Category>) {
    for state in 0..8 {
        for detail in 0..4 {
            for forbidden in [false, true] {
                for stain_non_finite in [false, true] {
                    let s = Sample {
                        forbidden,
                        state,
                        detail,
                        stain_non_finite,
                    };
                    let non_finite = stain_non_finite
                        || (state == STATE_SIM_FAILED && detail <= 2)
                        || (state == STATE_DECODE_FAILED && detail == 0);
                    let want = if forbidden {
                        Some(Category::Forbidden)
                    } else if non_finite {
                        Some(Category::NanInf)
                    } else if state >= STATE_SIM_FAILED {
                        Some(Category::Failure)
                    } else {
                        None
                    };
                    assert_eq!(category(s), want, "{s:?}");
                }
            }
        }
    }
    let labels = Category::ALL.map(Category::label);
    assert_eq!(
        labels,
        ["forbidden (chart)", "decode or sim failure", "NaN / Inf"]
    );
    assert_eq!(Category::ALL.map(|c| c as usize), [0, 1, 2]);
}

#[test]
fn debug_view_styles_twin_order() {
    check_order(debug_view::category);
}

negative_control!(
    debug_view_styles_twin_order,
    "a twin reading the failure before non-finiteness fails",
    expected = "state: 4, detail: 0",
    check_order(|s| {
        if !s.forbidden && debug_view::payload_failed(s.state) {
            Some(Category::Failure)
        } else {
            debug_view::category(s)
        }
    })
);

// ── The NaN / Inf style's mirror ────────────────────────────────────────────────────────────────────────────────

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The constant `name`'s three channels, as `present.wgsl` declares it.
fn wgsl_constant(name: &str) -> [f32; 3] {
    let source = read("crates/render/shaders/wgsl/lib/present.wgsl");
    let head = format!("const {name}: vec3<f32> = vec3<f32>(");
    let line = source
        .lines()
        .find(|l| l.starts_with(&head))
        .unwrap_or_else(|| panic!("present.wgsl declares no {name}"));
    let v: Vec<f32> = line[head.len()..]
        .trim_end_matches(");")
        .split(',')
        .map(|x| x.trim().parse().unwrap_or_else(|e| panic!("`{x}`: {e}")))
        .collect();
    [v[0], v[1], v[2]]
}

/// The side of the square the style is rendered on: four of its squares across, every phase.
const SIDE: u32 = 16;

/// `debug_nan_inf` at every pixel of a `SIDE` × `SIDE` target, rows from the top, after the prelude and the
/// presentation layer.
fn rendered() -> Vec<Rgb> {
    let module = format!(
        "{}\n{}\n@fragment\nfn t_nan_inf(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {{\n    \
         return vec4<u32>(bitcast<vec3<u32>>(debug_nan_inf(pos.xy)), 0u);\n}}\n",
        read(prelude::PATH),
        read("crates/render/shaders/wgsl/lib/present.wgsl"),
    );
    let gpu = gpu();
    let kernel = gpu
        .fragment(&module, "t_nan_inf", &[], SIDE, SIDE)
        .unwrap_or_else(|e| panic!("{e}"));
    kernel
        .draw(&[])
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [0, 1, 2].map(|c| f64::from(f32::from_bits(p[c]))))
        .collect()
}

/// Checks `present.wgsl`'s constants against the CPU mirror, each channel the f32 nearest the mirror's, and the
/// shader's `debug_nan_inf` against `mirror` at every pixel.
fn check_mirror(mirror: fn([f64; 2]) -> Rgb) {
    let f32s = |c: Rgb| c.map(|v| v as f32);
    let [indigo, green] = present::NAN_INF_SRGB8.map(present::srgb8);
    assert_eq!(wgsl_constant("DBG_NAN_INF_INDIGO"), f32s(indigo), "indigo");
    assert_eq!(wgsl_constant("DBG_NAN_INF_GREEN"), f32s(green), "green");
    assert_eq!(
        wgsl_constant("PRESENT_INVALID"),
        f32s(present::present_invalid()),
        "PRESENT_INVALID"
    );
    assert_eq!(
        present::PRESENT_INVALID_SRGB8,
        outcome::SWATCHES[0].srgb8,
        "#141418 is bounded's"
    );
    let image = rendered();
    let mut seen = [false; 2];
    for (k, got) in image.iter().enumerate() {
        let frag = [
            (k % SIDE as usize) as f64 + 0.5,
            (k / SIDE as usize) as f64 + 0.5,
        ];
        seen[present::nan_inf_square(frag)] = true;
        let want = mirror(frag);
        assert!(
            got.iter().zip(&want).all(|(g, w)| (g - w).abs() < 1e-7),
            "pixel {frag:?}: the shader's {got:?} is not the mirror's {want:?}"
        );
    }
    assert_eq!(seen, [true; 2], "both squares drawn");
}

#[test]
fn debug_view_styles_nan_inf_mirror() {
    check_mirror(present::debug_nan_inf);
}

negative_control!(
    debug_view_styles_nan_inf_mirror,
    "a mirror with squares of 2 px, not 4, is not the shader's",
    expected = "is not the mirror's",
    check_mirror(|frag| {
        let [x, y] = frag.map(|c| (c.floor() as i64).div_euclid(2));
        present::srgb8(present::NAN_INF_SRGB8[(x + y).rem_euclid(2) as usize])
    })
);

// ── REQ-COL-110's measurement ───────────────────────────────────────────────────────────────────────────────────

/// The OKLab distance between two linear colours.
fn distance(a: Rgb, b: Rgb) -> f64 {
    let (p, q) = (present::linear_to_oklab(a), present::linear_to_oklab(b));
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// What the screen shows of `c` under `mode`: simulated, clamped to [0, 1], as the pass writes; as it is unsimulated.
fn seen(mode: Option<CvdMode>, c: Rgb) -> Rgb {
    mode.map_or(c, |m| cvd::simulate(m, c).map(|x| x.clamp(0.0, 1.0)))
}

/// REQ-COL-055's τ (R-429): the OKLab distance from `#FF00FF` to escape body 2's `#E034C6`, 0.101.
fn tau() -> f64 {
    distance(palettes::hex(0xFF00FF), palettes::hex(0xE034C6))
}

/// The modes measured: unsimulated, then each colour-vision simulation.
const MODES: [Option<CvdMode>; 5] = [
    None,
    Some(CvdMode::Deuteranopia),
    Some(CvdMode::Protanopia),
    Some(CvdMode::Tritanopia),
    Some(CvdMode::Achromatopsia),
];

/// The colours the style is measured against, linear: every palette entry REQ-COL-055 measures, the hatch's two, the
/// stipple's grey and dot, and `#FF00FF`.
fn against() -> Vec<(String, Rgb)> {
    let mut out = palettes::entries();
    let [violet, cyan] = prelude::hatch().colours;
    out.push((format!("the hatch {violet:02X?}"), present::srgb8(violet)));
    out.push((format!("the hatch {cyan:02X?}"), present::srgb8(cyan)));
    out.push(("the stipple's grey".to_owned(), present::not_yet()));
    out.push(("the stipple's dot".to_owned(), present::not_yet_dot()));
    out.push(("#FF00FF".to_owned(), palettes::hex(0xFF00FF)));
    out
}

/// REQ-COL-110's measurement of the style `colours` (8-bit sRGB), printed: under each mode, each colour's tightest
/// distance and its entry, its distance from `#FF00FF`, and the two colours' distance from each other, the pattern's
/// contrast. Each colour must be farther than τ from `#FF00FF` under every mode (R-505) and from every entry
/// unsimulated; under a simulation, a colour within τ of an entry is distinct by its pattern only if the two colours,
/// simulated, are farther than τ apart, as REQ-COL-061 holds the hatch.
fn check_proposal(colours: [[u8; 3]; 2]) {
    let tau = tau();
    let entries = against();
    let magenta = palettes::hex(0xFF00FF);
    let style = colours.map(present::srgb8);
    let hexes = colours.map(|c| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));
    eprintln!(
        "REQ-COL-110: the NaN / Inf style {} / {}, τ = {tau:.4}",
        hexes[0], hexes[1]
    );
    for mode in MODES {
        let label = mode.map_or("unsimulated", |m| m.label());
        let nearest = style.map(|c| {
            let q = seen(mode, c);
            entries
                .iter()
                .map(|(name, p)| (distance(seen(mode, *p), q), name.clone()))
                .fold((f64::INFINITY, String::new()), |a, b| {
                    if b.0 < a.0 {
                        b
                    } else {
                        a
                    }
                })
        });
        let from_magenta = style.map(|c| distance(seen(mode, c), seen(mode, magenta)));
        let contrast = distance(seen(mode, style[0]), seen(mode, style[1]));
        eprintln!(
            "{label}: {} {:.4} from {}, {:.4} from #FF00FF; {} {:.4} from {}, {:.4} from #FF00FF; squares {contrast:.4} \
             apart",
            hexes[0], nearest[0].0, nearest[0].1, from_magenta[0], hexes[1], nearest[1].0, nearest[1].1, from_magenta[1]
        );
        for k in 0..2 {
            assert!(
                from_magenta[k] > tau,
                "{label}: {} is within τ of #FF00FF, {:.4}",
                hexes[k],
                from_magenta[k]
            );
            let clear = nearest[k].0 > tau;
            assert!(
                clear || (mode.is_some() && contrast > tau),
                "{label}: {} is {:.4} from {}, within τ, its squares {contrast:.4} apart",
                hexes[k],
                nearest[k].0,
                nearest[k].1
            );
        }
    }
}

#[test]
fn debug_view_styles_nan_inf_proposal() {
    check_proposal(present::NAN_INF_SRGB8);
}

negative_control!(
    debug_view_styles_nan_inf_proposal,
    "a magenta style, R-16's #FF00FF, fails (R-505)",
    expected = "#FF00FF is within τ of #FF00FF",
    check_proposal([[0xFF, 0x00, 0xFF], [0x00, 0x40, 0x00]])
);
