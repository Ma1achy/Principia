//! The outcome view as R-444 to R-448 leave it (TASK-M2-32), with R-467 and R-511 (TASK-M2-34): physical outcomes
//! only, failures in black, a pixel that is not a system and a `decode_failed` sample blank, and the OKLab-uniform
//! variant.
//! - REQ-COL-101 (`outcome_view_failures_*`), as REQ-COL-108 changes it: running, `sim_failed` of each `detail`, a
//!   reserved `state` code and bounded draw exactly bounded / nothing yet's `#141418`, on the CPU twin
//!   (`render::colour::outcome::shown`) and on the GPU; on the latent chart, whose `validate` is `Valid` everywhere, a
//!   sample carrying each decoder tag (`ENERGY_UNSCALABLE`, `ENERGY_INFEASIBLE`, `M01_TINY`) and a non-finite decode,
//!   each shown by the decoder at its chart point, keeps REQ-COL-102's field clear and draws the canvas background, as
//!   every `decode_failed` sample does (R-511); a sample the field marks forbidden draws exactly the background uniform,
//!   whatever its payload (R-467); in greyscale and hue mode running and failures sit at rank 0 and a forbidden or
//!   `decode_failed` sample draws the background. The forbidden rows read the background, a uniform this file sets to
//!   a value of its own ([`BG`]), so a pixel equal to it is the uniform's. REQ-COL-102's CPU fill
//!   (`engine::chart_class`) is checked at the unit square's edges, outside it, where `Φ` fails to evaluate and where
//!   `validate` rejects.
//! - REQ-COL-103 (`outcome_oklab_variant_*`): the variant renders exactly its eight swatches and `#141418`; each
//!   chromatic swatch measures OKLab L 0.72 ± 0.002, C 0.123 ± 0.002 and its hue within 0.5° of its class's own (R-447;
//!   R-455); failures and forbidden samples as in the default; the default is the sRGB set; editing one set leaves the
//!   other unchanged.
//! - REQ-COL-104 and REQ-COL-105 (`outcome_palette_separation_*`): the separation evidence R-444, R-445 and R-447
//!   measured, recomputed and printed for the PR (`--nocapture`), measured as REQ-COL-055 measures: Euclidean distance
//!   in OKLab, from 8-bit sRGB through dd_colouring §3.1's transforms (`render::present`). Proposals for the M2
//!   gate, R-454 (R-71), not values: the swatches are the human's.
//!
//! The classes are named as shown, 1-based (R-449): R-449's names, fixed strings until TASK-M2-33's conversion function
//! lands. Each GPU check takes a render, so its control runs it on the render of an altered occupant, payload or field
//! and shows it fails (pitfalls §9). Each test registers its negative control (R-176).

use render::assemble::Occupant;
use render::bind::Context;
use render::colour::outcome::{self, Shown, OKLAB_UNIFORM, SWATCHES};
use render::present::{self, Rgb};
use render::raster::Grid;
use validation::chart::{self, Chart, ChartOut, Float, Latent, ValidationResult};
use validation::chart_class;
use validation::golden_scene::{render_stain_with, Inputs, STATE_DECODE_FAILED, STATE_SIM_FAILED};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

#[path = "support/outcome_modes.rs"]
mod modes;
use modes::{event, greyscale, hue, stain, Mode, K};

/// Each sample's tile, in pixels.
const TILE: u32 = 8;

/// The `state` codes (payload §2).
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;

/// Bounded / nothing yet's swatch, `#141418` (R-446).
const BLACK: [u8; 3] = [0x14, 0x14, 0x18];

/// The canvas background this file's renders take, linear RGB: a value of its own, no palette's, exact in f32, so a
/// pixel equal to it is the uniform's (R-467 D2; REQ-COL-108).
const BG: [f32; 3] = [0.25, 0.125, 0.0625];

/// The largest per-channel difference, linear RGB, allowed between a computed colour and its CPU value.
const TOL: f64 = 1e-5;

// ── The chart points REQ-COL-102's field is classified from ──────────────────────────────────────────────────────

#[path = "support/decoder_cases.rs"]
mod decoder_cases;
use decoder_cases::{case_chart, pinned, raised, Case, CASES};

/// A test chart that rejects past `from` along `u`, as `result`, and is the latent chart pinned at `z0` otherwise: the
/// layer-2 or layer-3 failure REQ-COL-102 marks forbidden.
struct Walled {
    latent: Latent<f64>,
    from: f64,
    result: ValidationResult,
}

impl Chart for Walled {
    type Param = f64;
    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        self.latent.map(u, v)
    }
    fn name(&self) -> &str {
        "walled"
    }
    fn validate(&self, u: f64, _v: f64) -> ValidationResult {
        if u > self.from {
            self.result
        } else {
            ValidationResult::Valid
        }
    }
}

/// A rejection, layer 2's mass simplex.
const REJECT: ValidationResult = ValidationResult::Reject {
    failed: chart::Constraint::MassPositive,
};

/// The walled chart rejecting past `u = ½`, the latent chart at `z0 = 0` before it.
fn walled(result: ValidationResult) -> Walled {
    Walled {
        latent: pinned([0.0; 8]),
        from: 0.5,
        result,
    }
}

/// Where a row's sample sits, which REQ-COL-102's field is classified from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum At {
    /// The latent chart at `z0 = 0`, `(½, ½)`: `Valid`, `Φ` finite.
    Plain,
    /// `case`'s latent chart at `(½, ½)`.
    Decoder(Case),
    /// The walled chart at `(¾, ½)`, past its wall: `validate` rejects.
    Rejected,
    /// The latent chart with `z0` NaN, at `(½, ½)`: `Φ` fails to evaluate.
    Unevaluated,
}

/// The field's value at `at`, from the chart alone ([`chart_class::forbidden`]).
fn classified(at: At) -> bool {
    let f = match at {
        At::Plain => chart_class::forbidden(&pinned([0.0; 8]), 0.5, 0.5),
        At::Decoder(case) => chart_class::forbidden(&case_chart(case), 0.5, 0.5),
        At::Rejected => chart_class::forbidden(&walled(REJECT), 0.75, 0.5),
        At::Unevaluated => chart_class::forbidden(&pinned([f64::NAN; 8]), 0.5, 0.5),
    };
    f.unwrap_or_else(|| panic!("{at:?} is inside [0,1]²"))
}

// ── The failures scene ───────────────────────────────────────────────────────────────────────────────────────────

/// What a sample must show.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    /// Bounded / nothing yet's `#141418`, in event mode; rank 0 in the others.
    Black,
    /// The canvas background, the uniform [`BG`], exactly (R-467, R-511).
    Background,
}

/// A sample of the failures scene: its name, payload `(state, detail, t_end_step)`, where it sits and its want.
#[derive(Clone, Copy, Debug)]
struct Row {
    name: &'static str,
    payload: (u32, u32, u32),
    at: At,
    want: Want,
}

/// The failures scene: bounded, running, `sim_failed` of each `detail` (NaN in state, Inf/overflow in state, a
/// non-finite derived quantity, reserved), `decode_failed` of each `detail` and the reserved codes 6 and 7, each on a
/// plain chart point; each decoder case under each `decode_failed` detail at its own chart point (REQ-DEC-038 gives no
/// map from tag to detail, so every detail is drawn); then samples in a forbidden region, the walled chart's and the
/// unevaluated latent chart's, with a physical, a running, a failed and a reserved payload each. Every `decode_failed`
/// sample and every forbidden one wants the background (R-467, R-511).
fn rows() -> Vec<Row> {
    let row = |name, payload, at, want| Row {
        name,
        payload,
        at,
        want,
    };
    let mut out = vec![
        row("bounded", (BOUNDED, 0, 1000), At::Plain, Want::Black),
        row("running", (RUNNING, 0, 20), At::Plain, Want::Black),
    ];
    for d in 0..4 {
        out.push(row(
            "sim_failed",
            (STATE_SIM_FAILED, d, 40),
            At::Plain,
            Want::Black,
        ));
    }
    for d in 0..4 {
        out.push(row(
            "decode_failed",
            (STATE_DECODE_FAILED, d, 0),
            At::Plain,
            Want::Background,
        ));
    }
    out.push(row("reserved 6", (6, 0, 40), At::Plain, Want::Black));
    out.push(row("reserved 7", (7, 0, 40), At::Plain, Want::Black));
    for case in CASES {
        for d in 0..4 {
            out.push(row(
                "decoder case",
                (STATE_DECODE_FAILED, d, 0),
                At::Decoder(case),
                Want::Background,
            ));
        }
    }
    for at in [At::Rejected, At::Unevaluated] {
        out.extend([
            row(
                "forbidden bounded",
                (BOUNDED, 0, 1000),
                at,
                Want::Background,
            ),
            row(
                "forbidden collision",
                (COLLISION, 2, 40),
                at,
                Want::Background,
            ),
            row("forbidden escape", (ESCAPE, 1, 120), at, Want::Background),
            row("forbidden running", (RUNNING, 0, 20), at, Want::Background),
            row(
                "forbidden sim_failed",
                (STATE_SIM_FAILED, 0, 40),
                at,
                Want::Background,
            ),
            row("forbidden reserved", (6, 0, 40), at, Want::Background),
        ]);
        for d in 0..4 {
            out.push(row(
                "forbidden decode_failed",
                (STATE_DECODE_FAILED, d, 0),
                at,
                Want::Background,
            ));
        }
    }
    out
}

/// The grid of `n` samples in one row, one quad each, each tile [`TILE`] px.
fn grid(n: usize) -> Grid {
    Grid::new([n as u32, 1], 1, 0, TILE).expect("the scene's grid")
}

/// The sample of column `x`.
fn sample_at(g: &Grid, x: u32) -> u32 {
    g.cell(x * TILE + TILE / 2, TILE / 2).sample
}

/// The payload of `rows`, each sample's field from `lane`.
fn payload(rows: &[Row], lane: &dyn Fn(&Row) -> bool) -> Synthetic {
    let g = grid(rows.len());
    let mut set = Synthetic::flat(g, 3);
    for (x, r) in rows.iter().enumerate() {
        let i = sample_at(&g, x as u32);
        let (state, detail, t) = r.payload;
        set.sample(i).state(state).detail(detail).times(t, 0);
        set.set_forbidden(i, lane(r));
    }
    set
}

/// The validity lane's `forbidden` as REQ-COL-102 defines it: the chart's classification of the row's point.
fn validity_lane(r: &Row) -> bool {
    classified(r.at)
}

fn context(n: usize) -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid: grid(n),
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: 1,
    }
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `set` rendered in `mode`, `swatch_set` at `set_param` where given, the canvas background [`BG`].
fn render(
    h: &GpuHarness,
    set: &Synthetic,
    n: usize,
    mode: &Mode,
    param: &dyn Fn(&str) -> Option<Vec<f64>>,
) -> Vec<[f32; 4]> {
    render_stain_with(
        h.device(),
        h.queue(),
        &stain(mode),
        set,
        &context(n),
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

/// Event mode with the colour occupant `colour`.
fn event_with(colour: String) -> Mode {
    Mode {
        colour: Occupant::Custom(colour),
        ..event()
    }
}

/// The outcome occupant's WGSL with `from` replaced by `to`: a control's altered occupant.
#[cfg(feature = "controls")]
fn altered(from: &str, to: &str) -> String {
    assert!(outcome::WGSL.contains(from), "the occupant has no `{from}`");
    outcome::WGSL.replace(from, to)
}

/// Column `x`'s pixels in an image of `n` tiles, with their positions.
fn tile(image: &[[f32; 4]], n: usize, x: u32) -> Vec<((u32, u32), [f32; 4])> {
    let width = n as u32 * TILE;
    let mut out = Vec::new();
    for py in 0..TILE {
        for px in x * TILE..(x + 1) * TILE {
            out.push(((px, py), image[(py * width + px) as usize]));
        }
    }
    out
}

fn black() -> [f32; 3] {
    present::srgb8(BLACK).map(|c| c as f32)
}

/// Checks the failures scene's event-mode render `image` against the want of each row `which` picks: `#141418`
/// exactly, or the background uniform [`BG`] exactly; alpha 1.
fn check_event(rows: &[Row], which: Which, image: &[[f32; 4]]) {
    for (x, r) in rows.iter().enumerate().filter(|(_, r)| which(r)) {
        for ((px, py), got) in tile(image, rows.len(), x as u32) {
            let ok = got[3] == 1.0
                && match r.want {
                    Want::Black => got[..3] == black(),
                    Want::Background => got[..3] == BG,
                };
            assert!(
                ok,
                "{} {:?} at {:?}: pixel ({px}, {py}) is {got:?}, not {:?}",
                r.name, r.payload, r.at, r.want
            );
        }
    }
}

/// Checks each row's field, as `lane` sets it, against REQ-COL-102: clear on the plain and decoder points, each
/// decoder case raised by the decoder there; set where the chart rejects or `Φ` fails to evaluate.
fn check_lane(rows: &[Row], lane: &dyn Fn(&Row) -> bool) {
    for r in rows {
        if let At::Decoder(case) = r.at {
            let c = case_chart(case);
            assert!(raised(case, &c), "the decoder does not raise {case:?}");
            assert_eq!(c.validate(0.5, 0.5), ValidationResult::Valid);
        }
        let want = matches!(r.at, At::Rejected | At::Unevaluated);
        assert_eq!(
            lane(r),
            want,
            "{} {:?} at {:?}: REQ-COL-102's field",
            r.name,
            r.payload,
            r.at
        );
    }
}

/// Which rows a check reads.
type Which = fn(&Row) -> bool;

/// Renders the failures scene in event mode, the outcome occupant `colour`, the field from `lane`, and checks the rows
/// `which` picks. `lane` is first checked against REQ-COL-102's field: since R-511 a presentation render draws a
/// forbidden and a `decode_failed` sample alike, so a field set from the decoder's tags shows only where it is set.
fn failures_event(which: Which, colour: Option<String>, lane: &dyn Fn(&Row) -> bool) {
    let h = gpu();
    let rows = rows();
    check_lane(&rows, lane);
    let set = payload(&rows, lane);
    let mode = colour.map_or_else(event, event_with);
    let image = render(&h, &set, rows.len(), &mode, &defaults);
    check_event(&rows, which, &image);
}

fn every(_: &Row) -> bool {
    true
}

fn state_is(r: &Row, state: u32) -> bool {
    r.payload.0 == state && r.at == At::Plain
}

#[test]
fn outcome_view_failures_render_black_or_the_background() {
    failures_event(every, None, &validity_lane);
}

negative_control!(
    outcome_view_failures_render_black_or_the_background,
    "an occupant drawing blank from the payload alone, not the chart, fails",
    expected = "forbidden bounded (1, 0, 1000) at Rejected",
    failures_event(
        every,
        Some(altered(
            "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {",
            "if (ctx.sample.state == STATE_DECODE_FAILED) {",
        )),
        &validity_lane,
    )
);

#[cfg(feature = "controls")]
mod outcome_view_failures_render_black_or_the_background_hatch {
    use super::*;
    negative_control!(
        outcome_view_failures_render_black_or_the_background,
        "an occupant still drawing the forbidden region with the hatch fails (R-467)",
        expected = "forbidden bounded (1, 0, 1000) at Rejected",
        failures_event(
            |r| matches!(r.at, At::Rejected | At::Unevaluated),
            Some(altered(
                "return canvas_background();",
                "return debug_invalid(ctx.frag_xy);",
            )),
            &validity_lane,
        )
    );
}

#[test]
fn outcome_view_failures_running_not_stippled() {
    failures_event(|r| state_is(r, RUNNING), None, &validity_lane);
}

negative_control!(
    outcome_view_failures_running_not_stippled,
    "an occupant drawing running in the \"not yet\" stipple fails",
    expected = "running (3, 0, 20) at Plain",
    failures_event(
        |r| state_is(r, RUNNING),
        Some(altered(
            "    return select(uniforms.bounded, uniforms.oklab_bounded, v);",
            "    if (s == STATE_RUNNING) { return debug_not_yet(ctx.frag_xy); }\n    \
             return select(uniforms.bounded, uniforms.oklab_bounded, v);",
        )),
        &validity_lane,
    )
);

#[test]
fn outcome_view_failures_sim_failed_not_hatched() {
    failures_event(|r| state_is(r, STATE_SIM_FAILED), None, &validity_lane);
}

negative_control!(
    outcome_view_failures_sim_failed_not_hatched,
    "an occupant drawing sim_failed in the hatch fails",
    expected = "sim_failed (4, 0, 40) at Plain",
    failures_event(
        |r| state_is(r, STATE_SIM_FAILED),
        Some(altered(
            "    return select(uniforms.bounded, uniforms.oklab_bounded, v);",
            "    if (s == STATE_SIM_FAILED) { return debug_invalid(ctx.frag_xy); }\n    \
             return select(uniforms.bounded, uniforms.oklab_bounded, v);",
        )),
        &validity_lane,
    )
);

#[test]
fn outcome_view_failures_decode_failed_blank() {
    failures_event(|r| state_is(r, STATE_DECODE_FAILED), None, &validity_lane);
}

negative_control!(
    outcome_view_failures_decode_failed_blank,
    "an occupant drawing decode_failed black, as R-446 had it before R-511, fails",
    expected = "decode_failed (5, 0, 0) at Plain",
    failures_event(
        |r| state_is(r, STATE_DECODE_FAILED),
        Some(altered(
            "if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {",
            "if (ctx.validity.forbidden) {",
        )),
        &validity_lane,
    )
);

/// The rows on a decoder case's chart point.
fn decoder_rows(r: &Row) -> bool {
    matches!(r.at, At::Decoder(_))
}

#[test]
fn outcome_view_failures_field_not_from_the_decoder() {
    failures_event(decoder_rows, None, &validity_lane);
}

negative_control!(
    outcome_view_failures_field_not_from_the_decoder,
    "a field set from the decoder's tags marks the tagged samples forbidden, and fails",
    expected = "decoder case (5, 0, 0) at Decoder(Unscalable): REQ-COL-102's field",
    {
        let from_tags = |r: &Row| match r.at {
            At::Decoder(case) => case != Case::NonFinite,
            _ => validity_lane(r),
        };
        failures_event(decoder_rows, None, &from_tags)
    }
);

/// Checks the failures scene's render in `mode`, `image`, against the rank: every black row the colour of rank 0,
/// `rank0`, within [`TOL`], and every forbidden or `decode_failed` row the background uniform [`BG`] exactly.
fn check_ranked(mode: &str, rows: &[Row], image: &[[f32; 4]], rank0: Rgb) {
    for (x, r) in rows.iter().enumerate() {
        for ((px, py), got) in tile(image, rows.len(), x as u32) {
            let ok = match r.want {
                Want::Black => (0..3).all(|c| (f64::from(got[c]) - rank0[c]).abs() <= TOL),
                Want::Background => got[..3] == BG,
            };
            assert!(
                ok,
                "{mode}: {} {:?} at {:?}: pixel ({px}, {py}) is {got:?}, not {:?}",
                r.name, r.payload, r.at, r.want
            );
        }
    }
}

/// Renders the failures scene in greyscale and hue mode (R-448: `K = 9`), the field from `lane`, and checks running and
/// the failures at rank 0 and the forbidden and `decode_failed` rows in the background: greyscale's rank 0 the
/// None-colour row at `B = ½/K`, hue's the hue wheel at `t = 0`.
fn failures_ranked(lane: &dyn Fn(&Row) -> bool) {
    let h = gpu();
    let rows = rows();
    let set = payload(&rows, lane);
    let grey0 = present::oklab_to_linear([0.5 / f64::from(K), 0.0, 0.0]);
    for (mode, rank0) in [(greyscale(), grey0), (hue(), present::hue_wheel(0.0))] {
        let image = render(&h, &set, rows.len(), &mode, &defaults);
        check_ranked(mode.name, &rows, &image, rank0);
    }
}

#[test]
fn outcome_view_failures_rank_zero_in_every_mode() {
    failures_ranked(&validity_lane);
}

negative_control!(
    outcome_view_failures_rank_zero_in_every_mode,
    "a field that marks nothing forbidden ranks the forbidden samples in greyscale, and fails",
    expected = "greyscale: forbidden bounded (1, 0, 1000) at Rejected",
    failures_ranked(&|_: &Row| false)
);

/// Checks the CPU twin `shown` (`render::colour::outcome::shown`): every `state` code 0 to 7 and `detail` 0 to 3, the
/// field set or clear, a forbidden sample always `Forbidden`, a clear `decode_failed` one `DecodeFailed` (R-511), and a
/// clear one the class §1.4 gives it: a collision its pair's or the triple collision's, an escape its body's or the
/// triple ejection's, and every other state, bounded, running, `sim_failed` and the reserved codes, bounded / nothing
/// yet. The failures scene's rows against their wants, under the field from the chart.
fn check_twin(shown: &dyn Fn(u32, u32, bool) -> Shown) {
    assert_eq!(SWATCHES[0].srgb8, BLACK, "bounded / nothing yet's swatch");
    for state in 0..8 {
        for detail in 0..4 {
            assert_eq!(
                shown(state, detail, true),
                Shown::Forbidden,
                "a forbidden sample, state {state} detail {detail}"
            );
            let want = match (state, detail) {
                (STATE_DECODE_FAILED, _) => Shown::DecodeFailed,
                (COLLISION, 3) => Shown::Swatch(4),
                (COLLISION, d) => Shown::Swatch(3 - d as usize),
                (ESCAPE, d) => Shown::Swatch(5 + d as usize),
                _ => Shown::Swatch(0),
            };
            assert_eq!(
                shown(state, detail, false),
                want,
                "state {state} detail {detail}, the field clear"
            );
        }
    }
    for r in rows() {
        let (state, detail, _) = r.payload;
        let got = shown(state, detail, validity_lane(&r));
        let want = match r.want {
            Want::Black => got == Shown::Swatch(0),
            Want::Background => matches!(got, Shown::Forbidden | Shown::DecodeFailed),
        };
        assert!(want, "{} {:?} at {:?}: {got:?}", r.name, r.payload, r.at);
    }
}

#[test]
fn outcome_view_failures_cpu_twin() {
    check_twin(&outcome::shown);
}

negative_control!(
    outcome_view_failures_cpu_twin,
    "a twin drawing sim_failed blank fails (R-511: sim_failed keeps #141418)",
    expected = "state 4 detail 0, the field clear",
    check_twin(&|s, d, f| if s == STATE_SIM_FAILED && !f {
        Shown::DecodeFailed
    } else {
        outcome::shown(s, d, f)
    })
);

#[cfg(feature = "controls")]
mod outcome_view_failures_cpu_twin_decode_failed {
    use super::*;
    negative_control!(
        outcome_view_failures_cpu_twin,
        "a twin drawing decode_failed black, the class R-446 gave it, fails (R-511)",
        expected = "state 5 detail 0, the field clear",
        check_twin(&|s, d, f| if s == STATE_DECODE_FAILED && !f {
            Shown::Swatch(0)
        } else {
            outcome::shown(s, d, f)
        })
    );
}

// ── REQ-COL-102's CPU fill ───────────────────────────────────────────────────────────────────────────────────────

/// Checks a classifier, `classify`, against REQ-COL-102 on walled charts: inside `[0,1]²` its edges included, the
/// point forbidden exactly where `validate` returns anything but `Valid` (a rejection, a projection or a clamp) or `Φ`
/// fails to evaluate (a NaN or an infinite `z`); `None` just outside each edge, where the extension types decide.
fn check_classifier(classify: &dyn Fn(&Walled, f64, f64) -> Option<bool>) {
    let open = walled(REJECT);
    for (u, v) in [(0.0, 0.0), (0.0, 1.0), (0.5, 0.5), (0.5, 1.0), (0.5, 0.0)] {
        assert_eq!(
            classify(&open, u, v),
            Some(false),
            "({u}, {v}) before the wall"
        );
    }
    for (u, v) in [(1.0, 1.0), (1.0, 0.0), (0.5f64.next_up(), 0.5)] {
        assert_eq!(
            classify(&open, u, v),
            Some(true),
            "({u}, {v}) past the wall"
        );
    }
    let below = 0.0f64.next_down();
    let above = 1.0f64.next_up();
    for (u, v) in [(below, 0.5), (above, 0.5), (0.25, below), (0.25, above)] {
        assert_eq!(classify(&open, u, v), None, "({u}, {v}) outside [0,1]²");
    }
    let remedies = [
        ValidationResult::Project {
            u: 0.5,
            v: 0.5,
            failed: chart::Constraint::LzEParabola,
        },
        ValidationResult::Clamp {
            u: 0.5,
            v: 0.5,
            moved: [true, false],
            failed: chart::Constraint::BurrauNu,
        },
    ];
    for result in remedies {
        assert_eq!(
            classify(&walled(result), 0.75, 0.5),
            Some(true),
            "{result:?} is not Valid"
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut z0 = [0.0; 8];
        z0[5] = bad;
        let chart = Walled {
            latent: pinned(z0),
            from: 1.0,
            result: REJECT,
        };
        assert_eq!(
            classify(&chart, 0.25, 0.25),
            Some(true),
            "Φ gives z₅ = {bad}"
        );
    }
}

#[test]
fn outcome_view_failures_chart_class_classifies() {
    check_classifier(&|c, u, v| chart_class::forbidden(c, u, v));
}

negative_control!(
    outcome_view_failures_chart_class_classifies,
    "a classifier reading validate alone misses where Φ fails to evaluate",
    expected = "Φ gives z₅ = NaN",
    check_classifier(&|c, u, v| chart_class::forbidden(c, u, v)
        .map(|_| c.validate(u, v) != ValidationResult::Valid))
);

/// Checks [`chart_class::fill`] on a set of three samples: each sample's field its chart point's classification, and
/// a point outside `[0,1]²` refused by name; `points` the samples' chart points.
fn check_fill(points: [[f64; 2]; 3]) {
    let mut set = Synthetic::flat(grid(3), 3);
    chart_class::fill(&mut set, &walled(REJECT), |i| points[i as usize])
        .unwrap_or_else(|e| panic!("{e}"));
    let got: Vec<bool> = (0..3).map(|i| set.forbidden(i)).collect();
    assert_eq!(got, [false, true, false], "the filled field");
    let refused = chart_class::fill(&mut set, &walled(REJECT), |i| {
        [points[i as usize][0] + 1.0, 0.5]
    });
    assert_eq!(
        refused.map_err(|e| e.contains("outside [0,1]²")),
        Err(true),
        "a point outside the square"
    );
}

#[test]
fn outcome_view_failures_chart_class_fills() {
    check_fill([[0.25, 0.5], [0.75, 0.5], [0.5, 0.5]]);
}

negative_control!(
    outcome_view_failures_chart_class_fills,
    "points past the wall in another order fill another field",
    expected = "the filled field",
    check_fill([[0.75, 0.5], [0.25, 0.5], [0.5, 0.5]])
);

// ── REQ-COL-103: the OKLab-uniform variant ───────────────────────────────────────────────────────────────────────

/// R-447's hues, degrees, in the chromatic swatches' order (`OKLAB_UNIFORM[1..]`).
const HUES: [f64; 8] = [27.0, 147.0, 265.0, 65.0, 103.0, 335.0, 208.0, 305.0];

/// A colour's OKLCh: L, C and the hue in degrees, in `[0, 360)`.
fn lch(rgb: Rgb) -> [f64; 3] {
    let [l, a, b] = present::linear_to_oklab(rgb);
    [l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0)]
}

/// The smaller angle between two hues, degrees.
fn hue_gap(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Checks the variant's swatches, `variant`: bounded `#141418`, and each chromatic swatch at OKLab L 0.72 ± 0.002, C
/// 0.123 ± 0.002 and within 0.5° of its class's own hue, the sRGB swatch's (R-447: "each chromatic class keeps its own
/// OKLab hue"). Against the table's rounded hues, the one departure past 0.5° is escape body 3's `#09B9CD`, 0.54° from
/// its row's 208° (R-455); the check pins it, so a swatch that moves past 0.5° of its row fails.
fn check_measured(variant: &[outcome::Swatch]) {
    assert_eq!(variant[0].srgb8, BLACK, "the variant's bounded");
    let mut past_row = Vec::new();
    for ((s, own), row) in variant[1..].iter().zip(&SWATCHES[1..]).zip(HUES) {
        let [l, c, h] = lch(present::srgb8(s.srgb8));
        let own = lch(present::srgb8(own.srgb8))[2];
        assert!(
            (l - 0.72).abs() <= 0.002 && (c - 0.123).abs() <= 0.002 && hue_gap(h, own) <= 0.5,
            "{} {:02X?}: L {l:.4}, C {c:.4}, hue {h:.2}°, not L 0.72, C 0.123, hue {own:.2}°",
            s.label,
            s.srgb8
        );
        if hue_gap(h, row) > 0.5 {
            past_row.push(format!("{} {:.2}°", s.label, hue_gap(h, row)));
        }
    }
    assert_eq!(
        past_row,
        ["escape body 3 0.54°"],
        "the swatches past 0.5° of their row's hue"
    );
}

/// The variant scene: one sample per class, then running, a `sim_failed`, a `decode_failed` and a forbidden sample.
fn variant_rows() -> Vec<Row> {
    let row = |name, payload, at, want| Row {
        name,
        payload,
        at,
        want,
    };
    let mut out: Vec<Row> = [
        (BOUNDED, 0),
        (COLLISION, 2),
        (COLLISION, 1),
        (COLLISION, 0),
        (COLLISION, 3),
        (ESCAPE, 0),
        (ESCAPE, 1),
        (ESCAPE, 2),
        (ESCAPE, 3),
    ]
    .into_iter()
    .map(|(s, d)| row("class", (s, d, 40), At::Plain, Want::Black))
    .collect();
    out.extend([
        row("running", (RUNNING, 0, 20), At::Plain, Want::Black),
        row(
            "sim_failed",
            (STATE_SIM_FAILED, 1, 40),
            At::Plain,
            Want::Black,
        ),
        row(
            "decode_failed",
            (STATE_DECODE_FAILED, 1, 0),
            At::Plain,
            Want::Background,
        ),
        row(
            "forbidden",
            (COLLISION, 0, 40),
            At::Rejected,
            Want::Background,
        ),
    ]);
    out
}

/// The 8-bit sRGB each variant row must show under the set `set`: the class's swatch, `#141418` for running and
/// `sim_failed`; `None`, the background, for the `decode_failed` and forbidden rows (R-511, R-467).
fn variant_wants(set: &[outcome::Swatch]) -> Vec<Option<[u8; 3]>> {
    let mut out: Vec<Option<[u8; 3]>> = set.iter().map(|s| Some(s.srgb8)).collect();
    out.extend([Some(BLACK); 2]);
    out.extend([None; 2]);
    out
}

/// Checks a render of the variant scene, `image`, against `wants`: each class's swatch exactly, the f32 nearest its
/// sRGB decode, and the `decode_failed` and forbidden rows the background uniform [`BG`] exactly; alpha 1.
fn check_swatches(what: &str, image: &[[f32; 4]], wants: &[Option<[u8; 3]>]) {
    for (x, want) in wants.iter().enumerate() {
        for ((px, py), got) in tile(image, wants.len(), x as u32) {
            let ok = got[3] == 1.0
                && match want {
                    Some(c) => got[..3] == present::srgb8(*c).map(|v| v as f32),
                    None => got[..3] == BG,
                };
            assert!(
                ok,
                "{what}: sample {x} pixel ({px}, {py}) is {got:?}, not {want:02X?}"
            );
        }
    }
}

/// The variant's params: `swatch_set` at `set`, and the param `edit`, if any, at `#FFFFFF`.
fn variant_params(set: u32, edit: Option<&'static str>) -> impl Fn(&str) -> Option<Vec<f64>> {
    move |name| {
        if name == outcome::SET_PARAM {
            Some(vec![f64::from(set)])
        } else if Some(name) == edit {
            Some(present::srgb8([0xFF; 3]).to_vec())
        } else {
            None
        }
    }
}

/// The variant scene and a render of it through `mode`'s occupant under params.
struct VariantScene {
    h: GpuHarness,
    set: Synthetic,
    n: usize,
}

impl VariantScene {
    fn new() -> Self {
        let h = gpu();
        let rows = variant_rows();
        let set = payload(&rows, &validity_lane);
        VariantScene {
            h,
            set,
            n: rows.len(),
        }
    }

    fn at(&self, mode: &Mode, params: &dyn Fn(&str) -> Option<Vec<f64>>) -> Vec<[f32; 4]> {
        render(&self.h, &self.set, self.n, mode, params)
    }
}

/// Renders the variant scene through `mode`'s occupant: the sRGB set by default, the variant at `swatch_set = 1`, each
/// class exactly, running and `sim_failed` `#141418` and the `decode_failed` and forbidden samples the background in
/// both; the variant with escape body 2's
/// swatch edited, that class alone changed.
fn check_variant(mode: &Mode) {
    let v = VariantScene::new();
    check_swatches(
        "the default",
        &v.at(mode, &defaults),
        &variant_wants(&SWATCHES),
    );
    check_swatches(
        "the variant",
        &v.at(mode, &variant_params(1, None)),
        &variant_wants(&OKLAB_UNIFORM),
    );
    let mut edited = OKLAB_UNIFORM;
    edited[6].srgb8 = [0xFF; 3];
    check_swatches(
        "the variant, edited",
        &v.at(mode, &variant_params(1, Some("oklab_escape_body_1"))),
        &variant_wants(&edited),
    );
}

/// Checks that editing escape body 2's swatch in one set leaves the other set's render of the variant scene, through
/// `mode`'s occupant, unchanged.
fn check_edits_apart(mode: &Mode) {
    let v = VariantScene::new();
    assert!(
        v.at(mode, &variant_params(0, Some("oklab_escape_body_1"))) == v.at(mode, &defaults),
        "editing the variant changed the sRGB set's render"
    );
    assert!(
        v.at(mode, &variant_params(1, Some("escape_body_1")))
            == v.at(mode, &variant_params(1, None)),
        "editing the sRGB set changed the variant's render"
    );
}

#[test]
fn outcome_oklab_variant_measures_r_447() {
    check_measured(&OKLAB_UNIFORM);
}

negative_control!(
    outcome_oklab_variant_measures_r_447,
    "the sRGB palette measures neither L 0.72 nor C 0.123",
    expected = "not L 0.72, C 0.123",
    check_measured(&SWATCHES)
);

#[test]
fn outcome_oklab_variant_renders_its_swatches() {
    check_variant(&event());
}

negative_control!(
    outcome_oklab_variant_renders_its_swatches,
    "an occupant that never selects the variant fails",
    expected = "the variant: sample 1",
    check_variant(&event_with(altered(
        "let v = uniforms.swatch_set == 1u;",
        "let v = uniforms.swatch_set == 2u;",
    )))
);

#[test]
fn outcome_oklab_variant_edits_stay_in_their_set() {
    check_edits_apart(&event());
}

negative_control!(
    outcome_oklab_variant_edits_stay_in_their_set,
    "an occupant whose variant reads the sRGB set's escape body 2 where it is light fails",
    expected = "editing the sRGB set changed the variant's render",
    check_edits_apart(&event_with(altered(
        "select(uniforms.escape_body_1, uniforms.oklab_escape_body_1, v)",
        "select(uniforms.escape_body_1, max(uniforms.oklab_escape_body_1, uniforms.escape_body_1 - vec3<f32>(0.5)), v)",
    )))
);

// ── REQ-COL-104 and REQ-COL-105: the separation evidence ─────────────────────────────────────────────────────────

/// The OKLab distance between two 8-bit sRGB colours.
fn distance(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (a, b) = (
        present::linear_to_oklab(present::srgb8(a)),
        present::linear_to_oklab(present::srgb8(b)),
    );
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// Every pair of `set`'s swatches with its distance, nearest first.
fn pairs(set: &[outcome::Swatch]) -> Vec<(f64, &'static str, &'static str)> {
    let mut out = Vec::new();
    for (i, a) in set.iter().enumerate() {
        for b in &set[i + 1..] {
            out.push((distance(a.srgb8, b.srgb8), a.label, b.label));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The colours beside the palette that a swatch must stand apart from: the hatch's two (REQ-COL-055), the stipple's
/// grey and dot (REQ-COL-053, REQ-COL-064), the pending hatch's blue and the fallback tint's pink (REQ-TOOL-124).
fn others() -> Vec<(String, [u8; 3])> {
    let [violet, cyan] = ledger::gen::prelude::hatch().colours;
    vec![
        ("the hatch's violet".to_owned(), violet),
        ("the hatch's cyan".to_owned(), cyan),
        ("the stipple's grey".to_owned(), present::NOT_YET_SRGB8),
        ("the stipple's dot".to_owned(), present::NOT_YET_DOT_SRGB8),
        ("the pending hatch".to_owned(), [0x00, 0x00, 0xFF]),
        ("the fallback tint".to_owned(), [0xFF, 0x69, 0xFF]),
    ]
}

/// Whether `(L, C, hue)` is in the sRGB gamut: every linear channel in `[0, 1]`.
fn in_gamut(l: f64, c: f64, hue: f64) -> bool {
    present::oklch_to_linear(l, c, hue / 360.0)
        .iter()
        .all(|x| (0.0..=1.0).contains(x))
}

/// The largest in-gamut chroma at `(L, hue)`, by bisection to 1e-9.
fn max_chroma(l: f64, hue: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 0.5);
    while hi - lo > 1e-9 {
        let mid = 0.5 * (lo + hi);
        if in_gamut(l, mid, hue) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// A distance as the rulings quote it, three decimals.
fn r3(x: f64) -> String {
    format!("{x:.3}")
}

/// The sRGB palette's evidence (REQ-COL-104; R-444, R-445): its pairwise distances, its closest pair, triple collision
/// against escape body 1 at 0.175 (the human's 0.185 is collision 1–3 against escape body 3, the next; RQ-331), and
/// purple's distance from every entry, the nearest the "not yet" grey at 0.192, every one at least 0.19. Printed, then
/// checked against the figures R-444 and R-445 record.
fn check_srgb(set: &[outcome::Swatch]) {
    println!("REQ-COL-104, the sRGB palette's pairwise OKLab distances, nearest first:");
    let p = pairs(set);
    for (d, a, b) in &p {
        println!("  {d:.3}  {a} / {b}");
    }
    let purple = set[8];
    let mut from: Vec<(f64, String)> = set[..8]
        .iter()
        .map(|s| {
            (
                distance(purple.srgb8, s.srgb8),
                format!("{} {:02X?}", s.label, s.srgb8),
            )
        })
        .chain(
            others()
                .into_iter()
                .map(|(n, c)| (distance(purple.srgb8, c), format!("{n} {c:02X?}"))),
        )
        .collect();
    from.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!(
        "  triple ejection {:02X?}'s distance from every entry:",
        purple.srgb8
    );
    for (d, n) in &from {
        println!("    {d:.4}  {n}");
    }
    assert_eq!(
        (r3(p[0].0), p[0].1, p[0].2),
        (r3(0.175), "triple collision", "escape body 1"),
        "the closest pair"
    );
    assert_eq!(
        (r3(p[1].0), p[1].1, p[1].2),
        (r3(0.185), "collision 1–3", "escape body 3"),
        "the next pair, the human's 0.185"
    );
    assert_eq!(
        (
            format!("{:.4}", from[0].0),
            from[0].1.starts_with("the stipple's grey")
        ),
        ("0.1915".to_owned(), true),
        "purple's nearest"
    );
    assert!(
        from.iter().all(|(d, _)| *d >= 0.19),
        "purple within 0.19 of an entry"
    );
    let [l, c, h] = lch(present::srgb8(purple.srgb8));
    assert_eq!(
        (r3(l), r3(c), format!("{h:.1}")),
        (r3(0.401), r3(0.190), "305.1".to_owned())
    );
}

#[test]
fn outcome_palette_separation_srgb() {
    check_srgb(&SWATCHES);
}

negative_control!(
    outcome_palette_separation_srgb,
    "the palette with triple ejection back in R-433's navy #000097 measures another nearest to it",
    expected = "purple's nearest",
    {
        let mut old = SWATCHES;
        old[8].srgb8 = [0x00, 0x00, 0x97];
        check_srgb(&old)
    }
);

/// The OKLab-uniform variant's evidence (REQ-COL-105; R-447): its pairwise distances, its closest pair escape body 2
/// against triple ejection at 0.063 and the four more under 0.101; each swatch's measured L, C and hue; C = 0.123 in
/// gamut at every listed hue, the narrowest escape body 3's 208° at 0.1238; and the largest chroma in gamut at every
/// hue at L = 0.72, 0.1224 near 199.8°, under 0.123 (RQ-332); and each chromatic swatch's distance from the entries
/// beside it, as R-447 records. Printed, then checked.
fn check_variant_evidence(set: &[outcome::Swatch]) {
    println!("REQ-COL-105, the OKLab-uniform variant's pairwise OKLab distances, nearest first:");
    let p = pairs(set);
    for (d, a, b) in &p {
        println!("  {d:.3}  {a} / {b}");
    }
    println!("  each chromatic swatch measured, and C = 0.123 at its hue and L = 0.72:");
    let mut narrowest = (f64::INFINITY, 0.0);
    for (s, hue) in set[1..].iter().zip(HUES) {
        let [l, c, h] = lch(present::srgb8(s.srgb8));
        let most = max_chroma(0.72, hue);
        println!(
            "    {} {:02X?}: L {l:.4}, C {c:.4}, hue {h:.2}° (R-447 {hue}°); C 0.123 in gamut: {}, largest {most:.4}",
            s.label,
            s.srgb8,
            in_gamut(0.72, 0.123, hue)
        );
        assert!(in_gamut(0.72, 0.123, hue), "C 0.123 out of gamut at {hue}°");
        if most < narrowest.0 {
            narrowest = (most, hue);
        }
    }
    let every = (0..3600)
        .map(|k| (max_chroma(0.72, f64::from(k) / 10.0), f64::from(k) / 10.0))
        .fold((f64::INFINITY, 0.0), |a, b| if b.0 < a.0 { b } else { a });
    println!(
        "  the narrowest listed hue {}° at {:.4}; over every hue (0.1° steps) the largest in-gamut chroma at L = 0.72 \
         is {:.4}, at {:.1}°",
        narrowest.1, narrowest.0, every.0, every.1
    );
    let mut nearest: Vec<(String, f64)> = Vec::new();
    for (name, c) in others() {
        let d = set[1..]
            .iter()
            .map(|s| distance(s.srgb8, c))
            .fold(f64::INFINITY, f64::min);
        println!("  the nearest chromatic swatch to {name} {c:02X?}: {d:.3}");
        nearest.push((name, d));
    }
    assert_eq!(
        (r3(p[0].0), p[0].1, p[0].2),
        (r3(0.063), "escape body 2", "triple ejection"),
        "the closest pair"
    );
    let under: Vec<String> = p[1..]
        .iter()
        .filter(|x| x.0 < 0.101)
        .map(|(d, a, b)| format!("{a} / {b} {d:.3}"))
        .collect();
    assert_eq!(
        under,
        [
            "collision 1–2 / triple collision 0.079",
            "triple collision / escape body 1 0.081",
            "collision 2–3 / triple ejection 0.084",
            "collision 1–3 / escape body 1 0.091",
        ],
        "the pairs under 0.101"
    );
    assert_eq!(
        (format!("{:.4}", narrowest.0), narrowest.1),
        ("0.1238".to_owned(), 208.0),
        "the narrowest listed hue"
    );
    assert_eq!(
        (format!("{:.4}", every.0), (every.1 - 199.8).abs() < 0.5),
        ("0.1224".to_owned(), true),
        "the largest in-gamut chroma at every hue"
    );
    let floors: Vec<(String, String)> = nearest.iter().map(|(n, d)| (n.clone(), r3(*d))).collect();
    for (name, floor) in [
        ("the stipple's dot", 0.139),
        ("the hatch's violet", 0.196),
        ("the fallback tint", 0.132),
        ("the stipple's grey", 0.320),
    ] {
        let got = floors
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d.clone());
        assert!(
            got.as_deref()
                .is_some_and(|d| d.parse::<f64>().is_ok_and(|d| d >= floor)),
            "{name}: {got:?}, under R-447's {floor}"
        );
    }
}

#[test]
fn outcome_palette_separation_variant() {
    check_variant_evidence(&OKLAB_UNIFORM);
}

negative_control!(
    outcome_palette_separation_variant,
    "a variant with escape body 2 moved onto triple ejection's hue measures another closest pair",
    expected = "the closest pair",
    {
        let mut moved = OKLAB_UNIFORM;
        moved[6].srgb8 = OKLAB_UNIFORM[8].srgb8;
        check_variant_evidence(&moved)
    }
);
