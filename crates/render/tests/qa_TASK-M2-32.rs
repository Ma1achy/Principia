//! QA's tests for TASK-M2-32, written from the requirements it closes and the rulings it builds (R-444 to R-448,
//! R-449's names), not from the implementation. Every expected colour is typed in here from R-444's and R-447's tables;
//! OKLab is Ottosson's, typed in here (dd_colouring §3.1), with IEC 61966-2-1's sRGB transfer.
//!
//! - REQ-COL-101 (`qa_m2_32_outcome_view_*`): on the GPU, a scene of QA's own, 40 samples in one row with the
//!   descriptor's other bits contaminated (PIT-9): every class draws its swatch exactly, collisions and a triple
//!   collision at `t_end_step == 0` included (R-445); running, `sim_failed` and `decode_failed` of every `detail`, the
//!   reserved codes 6 and 7 and bounded draw `#141418` exactly; a sample REQ-COL-102's field marks forbidden draws the
//!   hatch, pixel-identical to a render of `debug_invalid` alone and within 1e-5 of its CPU mirror, whatever its
//!   payload (a valid escape, `decode_failed` of every `detail`, running, bounded, `sim_failed`, a collision at t = 0);
//!   a `decode_failed` sample beside it with the same payload, not forbidden, draws `#141418`. The forbidden samples
//!   straddle the lane's word boundary (samples 31 and 32). Controls: running in the stipple, `sim_failed` in the
//!   hatch, `decode_failed` white, a reserved code hatched, the hatch decided from `decode_failed`'s detail, the field
//!   ignored, and a collision at t = 0 drawn orange each fail. The CPU twin (`outcome::shown`) and the rank greyscale and
//!   hue mode take (`outcome::class`, R-448, `K = 9`) are checked over every `state`, `detail` and field value.
//! - REQ-COL-102 (`qa_m2_32_chart_class_*`): `engine::chart_class::forbidden` is `Some(true)` exactly where the chart's
//!   `validate` returns anything but `Valid` (layers 2 and 3: `Project`, `Clamp`, `Reject`) or `Φ`, `Chart::map`, gives
//!   a non-finite component (NaN or ±∞), on the closed square `[0,1]²` its edges included, and `None` outside it; on the
//!   latent chart, `validate` `Valid` everywhere, it is `Some(false)` on every point, the points where the shared decoder
//!   raises `M01_TINY` and `ENERGY_UNSCALABLE` among them (the decoder's own outcomes never set it). `fill` writes it per
//!   sample, refuses a point outside the square naming the sample, and the bound bytes hold bit `i mod 32` of word
//!   `i / 32`, little-endian.
//! - REQ-COL-103 (`qa_m2_32_oklab_variant_*`): `swatch_set = 1` renders R-447's swatches exactly and the failures and
//!   the hatch as the default does; the default and `swatch_set = 0` render the sRGB palette, bit-identically; an edit of
//!   one set leaves the other set's render bit-identical; each chromatic swatch measures L 0.72 ± 0.002, C 0.123 ± 0.002
//!   and its hue within 0.5° of its class's own (R-447: "each chromatic class keeps its own OKLab hue"), and within 0.5°
//!   of the table's rounded row but for escape body 3, whose departure (0.53°) is RQ-366's.
//! - REQ-COL-104 / REQ-COL-105 (`qa_m2_32_separation_*`): the figures R-445 and R-447 record, recomputed: the sRGB
//!   palette's closest pair 0.175 (triple collision / escape body 1), 0.185 with the orange left out (collision 1–3 /
//!   escape body 3), purple at least 0.19 from every entry, the hatch, the stipple, the pending hatch and the fallback
//!   tint (nearest the grey, 0.192), every swatch farther than τ = 0.101 from the dot; the variant's closest pair 0.063
//!   (escape body 2 / triple ejection), four more pairs under τ and no others, each chromatic swatch at least 0.139 from
//!   the dot, 0.196 from the hatch, 0.132 from the tint and 0.320 from the grey; C = 0.123 in gamut at L = 0.72 at each
//!   listed hue, and the largest chroma in gamut at every hue 0.1224, near 199.8°.
//! - REQ-COL-002 (`qa_m2_32_goldens_*`): the re-rendered goldens decoded and read tile by tile against the typed-in
//!   table: `m1-outcome/outcome` holds every row REQ-COL-002's verify names, each in its colour, the forbidden one in the
//!   hatch, and no white, orange-at-t = 0, stipple or navy; `outcome_edited` differs from it only in escape body 2's
//!   tiles; `m1-synthetic/known_escaper` draws the triple ejection purple and the running sample `#141418`.
//!
//! Each test registers its negative control (R-176).

// The file name `qa_TASK-M2-32` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;

use render::assemble::{Kind, Node, Occupant, Stain};
use render::bind::Context;
use render::colour::outcome::{self, Shown};
use render::raster::Grid;
use render::registry::ZERO_SOURCE;
use validation::chart::{Chart, ChartOut, Constraint, Float, Latent, ValidationResult};
use validation::golden_scene::render_stain;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;
use validation::{chart_class, energy_norm, Defaults, Degenerate, StickBreakingLinks};

type Rgb = [f64; 3];

// ── The rulings' values, typed in ───────────────────────────────────────────────────────────────────────────────

/// Payload §2's `state` codes.
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;
const SIM_FAILED: u32 = 4;
const DECODE_FAILED: u32 = 5;

/// R-444's nine classes, in colour_composition §1.4's order and R-448's rank: `(param, label as shown (R-449), sRGB,
/// R-447's variant, R-447's hue)`. Pair `k` is the side opposite body `k` (R-22): pair 2 is bodies 1 and 2 as shown.
const CLASSES: [(&str, &str, u32, u32, f64); 9] = [
    (
        "bounded",
        "bounded / nothing yet",
        0x141418,
        0x141418,
        f64::NAN,
    ),
    (
        "collision_pair_2",
        "collision 1–2",
        0xDE2D2D,
        0xE7857A,
        27.0,
    ),
    (
        "collision_pair_1",
        "collision 1–3",
        0x2EBC4E,
        0x6EB976,
        147.0,
    ),
    (
        "collision_pair_0",
        "collision 2–3",
        0x3462E0,
        0x7FA2F2,
        265.0,
    ),
    (
        "triple_collision",
        "triple collision",
        0xF29620,
        0xD99349,
        65.0,
    ),
    ("escape_body_0", "escape body 1", 0xF0DE32, 0xB3A841, 103.0),
    ("escape_body_1", "escape body 2", 0xE034C6, 0xD486C3, 335.0),
    ("escape_body_2", "escape body 3", 0x30C8DC, 0x09B9CD, 208.0),
    (
        "triple_ejection",
        "triple ejection",
        0x611597,
        0xB790E2,
        305.0,
    ),
];

/// The class rank of `(state, detail)` as R-444, R-446 and R-448 define it: a collision's pair (pair 2 rank 1, pair 1
/// rank 2, pair 0 rank 3) or the triple collision (4), an escape's body (5, 6, 7) or the triple ejection (8); every other
/// state, bounded, running, the failures and the reserved codes, bounded / nothing yet, 0.
fn rank(state: u32, detail: u32) -> usize {
    match (state, detail) {
        (COLLISION, 3) => 4,
        (COLLISION, d) => 3 - d as usize,
        (ESCAPE, 3) => 8,
        (ESCAPE, d) => 5 + d as usize,
        _ => 0,
    }
}

/// The colours the outcome view must not show any more, or that the separation counts: the removed degenerate white
/// and navy, the "not yet" grey and dot (R-433, R-434), the hatch (REQ-COL-055), the pending hatch's blue and the
/// fallback tint (debug_tooling_plan §G).
const WHITE: u32 = 0xECECF0;
const NAVY: u32 = 0x000097;
const GREY: u32 = 0x4E4E4E;
const DOT: u32 = 0x909090;
const VIOLET: u32 = 0x9B00FF;
const CYAN: u32 = 0x48FFFF;
const PENDING: u32 = 0x0000FF;
const TINT: u32 = 0xFF69FF;

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

/// IEC 61966-2-1's sRGB encode of a linear channel, to 8 bits, rounded.
fn encode8(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let v = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round() as u8
}

/// Ottosson's linear sRGB to OKLab.
fn oklab_linear([r, g, b]: Rgb) -> Rgb {
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

fn oklab(h: u32) -> Rgb {
    oklab_linear(linear(h))
}

/// Ottosson's OKLab to linear sRGB.
fn linear_of_oklab([l, a, b]: Rgb) -> Rgb {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let [l, m, s] = [l_.powi(3), m_.powi(3), s_.powi(3)];
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
    ]
}

fn dist(a: u32, b: u32) -> f64 {
    let (p, q) = (oklab(a), oklab(b));
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// OKLCh: L, C and the hue in degrees, in `[0, 360)`.
fn lch(h: u32) -> Rgb {
    let [l, a, b] = oklab(h);
    [l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0)]
}

fn hue_gap(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// τ, the OKLab distance from `#FF00FF` to `#E034C6` (R-429, R-132).
fn tau() -> f64 {
    dist(0xFF00FF, 0xE034C6)
}

// ── REQ-COL-101: the CPU twin and the rank ──────────────────────────────────────────────────────────────────────

/// Checks a twin `shown` and a rank `class` over every `state` code 0–7, `detail` 0–3 and field value against R-444
/// to R-448: forbidden first, the hatch whatever the payload; else the class of [`rank`], whose swatch is the typed-in
/// table's.
fn check_twin(shown: &dyn Fn(u32, u32, bool) -> Shown, class: &dyn Fn(u32, u32) -> usize) {
    assert_eq!(outcome::K, 9, "K is not R-448's 9");
    for state in 0..8 {
        for detail in 0..4 {
            let want = rank(state, detail);
            assert_eq!(
                class(state, detail),
                want,
                "state {state}, detail {detail}: rank is not R-448's"
            );
            assert_eq!(
                shown(state, detail, true),
                Shown::Forbidden,
                "state {state}, detail {detail}, forbidden: not the hatch"
            );
            match shown(state, detail, false) {
                Shown::Swatch(k) => assert_eq!(
                    outcome::SWATCHES[k].srgb8,
                    rgb8(CLASSES[want].2),
                    "state {state}, detail {detail}: swatch {k}, not {}",
                    CLASSES[want].1
                ),
                other => panic!("state {state}, detail {detail}, not forbidden: {other:?}"),
            }
        }
    }
}

#[test]
fn qa_m2_32_outcome_view_twin_and_rank() {
    check_twin(&outcome::shown, &outcome::class);
}

negative_control!(
    qa_m2_32_outcome_view_twin_and_rank,
    "a twin that hatches decode_failed of detail 1 from its payload fails (R-446)",
    expected = "state 5, detail 1, not forbidden",
    check_twin(
        &|s, d, f| if s == DECODE_FAILED && d == 1 {
            Shown::Forbidden
        } else {
            outcome::shown(s, d, f)
        },
        &outcome::class
    )
);

#[cfg(feature = "controls")]
mod qa_m2_32_outcome_view_twin_and_rank_eleven {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_twin_and_rank,
        "a rank that puts decode_failed at the old degenerate's 4 fails (R-446, R-448)",
        expected = "state 5, detail 0: rank is not R-448's",
        check_twin(&outcome::shown, &|s, d| if s == DECODE_FAILED {
            4
        } else {
            outcome::class(s, d)
        })
    );
}

/// The declared swatch sets are R-444's and R-447's, in §1.4's order, with R-449's labels, none a proposal.
fn check_declared(srgb: &[outcome::Swatch], variant: &[outcome::Swatch]) {
    assert_eq!(srgb.len(), 9);
    assert_eq!(variant.len(), 9);
    for (k, &(param, label, s, v, _)) in CLASSES.iter().enumerate() {
        assert_eq!(
            (srgb[k].param, srgb[k].label, srgb[k].srgb8),
            (param, label, rgb8(s)),
            "the sRGB set's class {k}"
        );
        assert_eq!(
            (variant[k].label, variant[k].srgb8),
            (label, rgb8(v)),
            "the variant's class {k}"
        );
        assert_ne!(variant[k].param, srgb[k].param, "the sets share a param");
        assert!(
            !srgb[k].proposed && !variant[k].proposed,
            "class {k} proposed"
        );
    }
}

#[test]
fn qa_m2_32_outcome_view_declared_sets() {
    check_declared(&outcome::SWATCHES, &outcome::OKLAB_UNIFORM);
}

negative_control!(
    qa_m2_32_outcome_view_declared_sets,
    "the sets swapped are not R-444's and R-447's",
    expected = "the sRGB set's class 0",
    check_declared(&outcome::OKLAB_UNIFORM, &outcome::SWATCHES)
);

// ── REQ-COL-101 / REQ-COL-103: the GPU scene ────────────────────────────────────────────────────────────────────

/// What a row must draw: the class of this rank, flat, or the hatch.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    Class(usize),
    Hatch,
}

/// QA's rows: `(state, detail, t_end_step, forbidden)`; 40 samples, the forbidden ones at 30–38, across the lane's
/// word boundary.
const ROWS: [(u32, u32, u32, bool); 40] = [
    (BOUNDED, 2, 1000, false),
    (COLLISION, 2, 40, false),
    (COLLISION, 1, 40, false),
    (COLLISION, 0, 40, false),
    (COLLISION, 3, 40, false),
    (ESCAPE, 0, 120, false),
    (ESCAPE, 1, 120, false),
    (ESCAPE, 2, 120, false),
    (ESCAPE, 3, 120, false),
    // R-445: a collision at step 0 is its pair's, a triple collision there the triple collision's.
    (COLLISION, 2, 0, false),
    (COLLISION, 1, 0, false),
    (COLLISION, 0, 0, false),
    (COLLISION, 3, 0, false),
    // R-446: running, the failures of every detail and the reserved codes are bounded / nothing yet.
    (RUNNING, 0, 0, false),
    (RUNNING, 3, 77, false),
    (SIM_FAILED, 0, 40, false),
    (SIM_FAILED, 1, 40, false),
    (SIM_FAILED, 2, 40, false),
    (SIM_FAILED, 3, 40, false),
    (DECODE_FAILED, 0, 0, false),
    (DECODE_FAILED, 1, 0, false),
    (DECODE_FAILED, 2, 0, false),
    (DECODE_FAILED, 3, 0, false),
    (6, 1, 40, false),
    (7, 3, 40, false),
    (ESCAPE, 2, 0, false),
    (BOUNDED, 0, 5, false),
    (RUNNING, 1, 3, false),
    (ESCAPE, 1, 500, false),
    (COLLISION, 0, 9, false),
    // REQ-COL-102's field set: the hatch, whatever the payload.
    (ESCAPE, 1, 120, true),
    (DECODE_FAILED, 0, 0, true),
    (DECODE_FAILED, 1, 0, true),
    (DECODE_FAILED, 2, 0, true),
    (DECODE_FAILED, 3, 0, true),
    (RUNNING, 2, 4, true),
    (COLLISION, 3, 0, true),
    (BOUNDED, 1, 1000, true),
    (SIM_FAILED, 1, 40, true),
    // The twin of row 32, not forbidden: black, so only the field tells them apart.
    (DECODE_FAILED, 1, 0, false),
];

fn want(row: (u32, u32, u32, bool)) -> Want {
    let (state, detail, _, forbidden) = row;
    if forbidden {
        Want::Hatch
    } else {
        Want::Class(rank(state, detail))
    }
}

const TILE: u32 = 8;

fn grid() -> Grid {
    Grid::new([ROWS.len() as u32, 1], 1, 0, TILE).expect("the scene's grid")
}

fn sample_at(g: &Grid, x: u32) -> u32 {
    g.cell(x * TILE + TILE / 2, TILE / 2).sample
}

/// The scene's payload, the descriptor's other bits contaminated, REQ-COL-102's field from the rows.
fn payload() -> Synthetic {
    let g = grid();
    let mut set = Synthetic::flat(g, 6);
    for (x, &(state, detail, t, forbidden)) in ROWS.iter().enumerate() {
        let i = sample_at(&g, x as u32);
        set.sample(i)
            .state(state)
            .detail(detail)
            .times(t, t / 3)
            .saturated(i % 2 == 1)
            .dmin_pair(i % 4)
            .last_symbol((i / 2) % 4)
            .d_min(0.02 * (i + 1) as f32);
        set.set_forbidden(i, forbidden);
    }
    let forbidden: Vec<u32> = (0..ROWS.len() as u32)
        .map(|x| sample_at(&g, x))
        .filter(|&i| set.forbidden(i))
        .collect();
    assert!(
        forbidden.iter().any(|&i| i < 32) && forbidden.iter().any(|&i| i >= 32),
        "the forbidden samples {forbidden:?} do not straddle the lane's word boundary"
    );
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

/// Event mode's stain with `colour` in the colour slot: source → colour, brightness None → Replace-L → OUT.
fn stain(colour: Occupant) -> Stain {
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
    .unwrap_or_else(|e| panic!("{e}"))
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// The scene rendered with `colour`, the params `params` gives by name.
fn render(
    h: &GpuHarness,
    colour: Occupant,
    params: &dyn Fn(&str) -> Option<Vec<f64>>,
) -> Vec<[f32; 4]> {
    render_stain(
        h.device(),
        h.queue(),
        &stain(colour),
        &payload(),
        &context(),
        &|_, name| params(name),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn built_in() -> Occupant {
    Occupant::BuiltIn(outcome::ID.to_owned())
}

fn no_params(_: &str) -> Option<Vec<f64>> {
    None
}

/// The hatch alone, the GPU's `debug_invalid`: the forbidden rows' reference.
fn hatch_probe(h: &GpuHarness) -> Vec<[f32; 4]> {
    render(
        h,
        Occupant::Custom(
            "fn colour(ctx: Ctx) -> vec3<f32> { return debug_invalid(ctx.frag_xy); }\n".to_owned(),
        ),
        &no_params,
    )
}

/// The outcome occupant with `to` inserted after its `detail` read: a control's altered occupant.
#[cfg(feature = "controls")]
fn inserted(to: &str) -> Occupant {
    replaced(
        "let d = ctx.sample.detail;",
        &format!("let d = ctx.sample.detail;\n    {to}"),
    )
}

#[cfg(feature = "controls")]
fn replaced(from: &str, to: &str) -> Occupant {
    assert_eq!(
        outcome::WGSL.matches(from).count(),
        1,
        "`{from}` in the occupant"
    );
    Occupant::Custom(outcome::WGSL.replace(from, to))
}

/// Row `x`'s pixels and their positions.
fn tile(image: &[[f32; 4]], x: u32) -> Vec<((u32, u32), [f32; 4])> {
    let width = ROWS.len() as u32 * TILE;
    let mut out = Vec::new();
    for py in 0..TILE {
        for px in x * TILE..(x + 1) * TILE {
            out.push(((px, py), image[(py * width + px) as usize]));
        }
    }
    out
}

/// Checks `image` against the rows: a class's swatch from `set` (the sRGB column or the variant's) bit-exact, as the
/// f32 of its linear decode the fragment returns unchanged; the hatch bit-identical to `probe` and within 1e-5 of the
/// CPU's `debug_invalid`; alpha 1.
fn check_scene(what: &str, image: &[[f32; 4]], probe: &[[f32; 4]], variant: bool) {
    for (x, &row) in ROWS.iter().enumerate() {
        let w = want(row);
        for (((px, py), got), (_, hatch)) in
            tile(image, x as u32).into_iter().zip(tile(probe, x as u32))
        {
            let ok = got[3] == 1.0
                && match w {
                    Want::Class(k) => {
                        let hex = if variant { CLASSES[k].3 } else { CLASSES[k].2 };
                        let lin = linear(hex);
                        (0..3).all(|c| got[c] == lin[c] as f32)
                            && [got[0], got[1], got[2]].map(|c| encode8(f64::from(c))) == rgb8(hex)
                    }
                    Want::Hatch => {
                        let cpu = render::present::debug_invalid([
                            f64::from(px) + 0.5,
                            f64::from(py) + 0.5,
                        ]);
                        got == hatch && (0..3).all(|c| (f64::from(got[c]) - cpu[c]).abs() <= 1e-5)
                    }
                };
            let (s, d, t, f) = row;
            assert!(
                ok,
                "{what}: row {x} (state {s}, detail {d}, t {t}, forbidden {f}) pixel ({px}, {py}) is {got:?}, not {w:?}"
            );
        }
    }
}

#[test]
fn qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched() {
    let h = gpu();
    check_scene(
        "event",
        &render(&h, built_in(), &no_params),
        &hatch_probe(&h),
        false,
    );
}

/// A control: the scene rendered with `colour`, checked as the default.
#[cfg(feature = "controls")]
fn control(colour: Occupant) {
    let h = gpu();
    check_scene(
        "event",
        &render(&h, colour, &no_params),
        &hatch_probe(&h),
        false,
    );
}

negative_control!(
    qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
    "running drawn in the \"not yet\" stipple fails (R-446)",
    expected = "row 13 (state 3, detail 0",
    control(inserted(
        "if (s == STATE_RUNNING) { return debug_not_yet(ctx.frag_xy); }"
    ))
);

#[cfg(feature = "controls")]
mod qa_m2_32_failures_sim_failed_hatched {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
        "sim_failed drawn in the hatch fails (R-446)",
        expected = "row 15 (state 4, detail 0",
        control(inserted(
            "if (s == STATE_SIM_FAILED) { return debug_invalid(ctx.frag_xy); }"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_32_failures_decode_failed_white {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
        "decode_failed drawn in the old degenerate white fails (R-446)",
        expected = "row 19 (state 5, detail 0",
        control(inserted(
            "if (s == STATE_DECODE_FAILED) { return vec3<f32>(0.838799, 0.838799, 0.8713671); }"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_32_failures_reserved_hatched {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
        "a reserved code drawn in the hatch, as before R-446, fails",
        expected = "row 23 (state 6, detail 1",
        control(inserted(
            "if (s > 5u) { return debug_invalid(ctx.frag_xy); }"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_32_failures_hatch_from_detail {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
        "the hatch decided from decode_failed's detail, not the chart, fails (R-446)",
        expected = "row 20 (state 5, detail 1",
        control(replaced(
            "if (ctx.chart.forbidden) {",
            "if (ctx.chart.forbidden || (ctx.sample.state == STATE_DECODE_FAILED && ctx.sample.detail == 1u)) {"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_32_failures_field_ignored {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
        "an occupant that ignores REQ-COL-102's field draws a forbidden escape in its swatch",
        expected = "row 30 (state 0, detail 1, t 120, forbidden true)",
        control(replaced("if (ctx.chart.forbidden) {", "if (false) {"))
    );
}

#[cfg(feature = "controls")]
mod qa_m2_32_failures_orange_at_start {
    use super::*;
    negative_control!(
        qa_m2_32_outcome_view_renders_failures_black_and_forbidden_hatched,
        "a collision at t_end_step == 0 drawn in the orange, as before R-445, fails",
        expected = "row 9 (state 2, detail 2, t 0",
        control(inserted(
            "if (s == STATE_COLLISION && ctx.sample.t_end_step == 0u) { return uniforms.triple_collision; }"
        ))
    );
}

// ── REQ-COL-103: the OKLab-uniform variant on the GPU ───────────────────────────────────────────────────────────

/// The params: `swatch_set` at `set` where given, and `edit`'s param at linear `#80FF10`.
fn variant_params(
    set: Option<u32>,
    edit: Option<&'static str>,
) -> impl Fn(&str) -> Option<Vec<f64>> {
    move |name: &str| {
        if Some(name) == edit {
            return Some(linear(0x80FF10).to_vec());
        }
        match set {
            Some(s) if name == outcome::SET_PARAM => Some(vec![f64::from(s)]),
            _ => None,
        }
    }
}

#[test]
fn qa_m2_32_oklab_variant_renders_its_swatches() {
    let h = gpu();
    let probe = hatch_probe(&h);
    let default = render(&h, built_in(), &no_params);
    check_scene("default", &default, &probe, false);
    let zero = render(&h, built_in(), &variant_params(Some(0), None));
    assert_eq!(zero, default, "swatch_set 0 is not the default render");
    let one = render(&h, built_in(), &variant_params(Some(1), None));
    check_scene("swatch_set 1", &one, &probe, true);
}

negative_control!(
    qa_m2_32_oklab_variant_renders_its_swatches,
    "an occupant that selects the variant at swatch_set 2 shows the sRGB set at 1",
    expected = "swatch_set 1: row 1 (state 2, detail 2",
    {
        let h = gpu();
        let altered = replaced(
            "let v = uniforms.swatch_set == 1u;",
            "let v = uniforms.swatch_set == 2u;",
        );
        let one = render(&h, altered, &variant_params(Some(1), None));
        check_scene("swatch_set 1", &one, &hatch_probe(&h), true);
    }
);

/// Checks that `edited`, a render with `param` edited, differs from `base` exactly on the rows of class `k`, each
/// pixel there the edit, when `moves`; and is bit-identical to `base` otherwise.
fn check_edit(what: &str, base: &[[f32; 4]], edited: &[[f32; 4]], k: usize, moves: bool) {
    let lin = linear(0x80FF10);
    let mut changed = 0;
    for (x, &row) in ROWS.iter().enumerate() {
        let mine = moves && want(row) == Want::Class(k);
        for (((px, py), a), (_, b)) in tile(base, x as u32).into_iter().zip(tile(edited, x as u32))
        {
            if mine {
                assert!(
                    (0..3).all(|c| b[c] == lin[c] as f32),
                    "{what}: row {x} pixel ({px}, {py}) is {b:?}, not the edit"
                );
                changed += 1;
            } else {
                assert_eq!(a, b, "{what}: row {x} pixel ({px}, {py}) changed");
            }
        }
    }
    assert_eq!(
        changed > 0,
        moves,
        "{what}: the edit's class changed {changed} pixels"
    );
}

#[test]
fn qa_m2_32_oklab_variant_edits_stay_in_their_set() {
    let h = gpu();
    let k = 6; // escape body 2
    let (srgb, variant) = (CLASSES[k].0, outcome::OKLAB_UNIFORM[k].param);
    assert_eq!(outcome::SWATCHES[k].param, srgb);
    let base = |set| render(&h, built_in(), &variant_params(Some(set), None));
    let edited = |set, p| render(&h, built_in(), &variant_params(Some(set), Some(p)));
    let (b0, b1) = (base(0), base(1));
    check_edit(
        "the variant's edit, shown at 0",
        &b0,
        &edited(0, variant),
        k,
        false,
    );
    check_edit(
        "the variant's edit, shown at 1",
        &b1,
        &edited(1, variant),
        k,
        true,
    );
    check_edit("the sRGB edit, shown at 1", &b1, &edited(1, srgb), k, false);
    check_edit("the sRGB edit, shown at 0", &b0, &edited(0, srgb), k, true);
}

negative_control!(
    qa_m2_32_oklab_variant_edits_stay_in_their_set,
    "the sRGB set's edit, shown at swatch_set 0, does change the render",
    expected = "the sRGB edit, claimed apart: row 6",
    {
        let h = gpu();
        let b0 = render(&h, built_in(), &variant_params(Some(0), None));
        let e0 = render(
            &h,
            built_in(),
            &variant_params(Some(0), Some("escape_body_1")),
        );
        check_edit("the sRGB edit, claimed apart", &b0, &e0, 6, false);
    }
);

/// Checks the variant `set` measures as R-447 rules: bounded `#141418`; each chromatic swatch at L 0.72 ± 0.002 and
/// C 0.123 ± 0.002, its hue within 0.5° of its class's own sRGB hue, and within 0.5° of the table's row but for the
/// departures `past_row` names (RQ-366).
fn check_measured(set: &[u32; 9], past_row: &[usize]) {
    assert_eq!(set[0], 0x141418, "the variant's bounded");
    for k in 1..9 {
        let [l, c, h] = lch(set[k]);
        let own = lch(CLASSES[k].2)[2];
        let row = CLASSES[k].4;
        println!(
            "{}: #{:06X} L {l:.4} C {c:.4} hue {h:.2}°, its class's own {own:.2}° ({:.2}° off), its row {row}° ({:.2}° off)",
            CLASSES[k].1,
            set[k],
            hue_gap(h, own),
            hue_gap(h, row)
        );
        assert!(
            (l - 0.72).abs() <= 0.002 && (c - 0.123).abs() <= 0.002 && hue_gap(h, own) <= 0.5,
            "{}: L {l:.4}, C {c:.4}, hue {h:.2}°: not L 0.72, C 0.123 at its own hue {own:.2}°",
            CLASSES[k].1
        );
        assert_eq!(
            hue_gap(h, row) > 0.5,
            past_row.contains(&k),
            "{}: hue {h:.2}° against its row's {row}°",
            CLASSES[k].1
        );
    }
}

fn variant_hexes() -> [u32; 9] {
    CLASSES.map(|c| c.3)
}

#[test]
fn qa_m2_32_oklab_variant_measures_r447() {
    // Escape body 3's `#09B9CD` is 0.53° from its row's 208°, within 0.5° of its class's own 208.48° (RQ-366).
    check_measured(&variant_hexes(), &[7]);
}

negative_control!(
    qa_m2_32_oklab_variant_measures_r447,
    "the sRGB palette taken as the variant is not at L 0.72, C 0.123",
    expected = "collision 1–2: L",
    check_measured(&CLASSES.map(|c| c.2), &[7])
);

// ── REQ-COL-104 / REQ-COL-105: the separation evidence ──────────────────────────────────────────────────────────

/// Every pair of `set`'s entries with its distance, nearest first.
fn pairs(set: &[(&str, u32)]) -> Vec<(f64, &'static str, &'static str)> {
    let mut out = Vec::new();
    for i in 0..set.len() {
        for j in i + 1..set.len() {
            out.push((dist(set[i].1, set[j].1), name(set[i].0), name(set[j].0)));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

fn name(n: &str) -> &'static str {
    CLASSES
        .iter()
        .map(|c| c.1)
        .find(|l| *l == n)
        .unwrap_or_else(|| panic!("no class {n}"))
}

fn near(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() < 5e-4,
        "{what}: {got:.4}, recorded {want}"
    );
}

/// Checks R-445's figures for the sRGB palette `set`.
fn check_srgb_evidence(set: &[u32; 9]) {
    let named: Vec<(&str, u32)> = CLASSES.iter().zip(set).map(|(c, &h)| (c.1, h)).collect();
    let p = pairs(&named);
    println!("sRGB palette, nearest pairs: {:?}", &p[..4]);
    near(p[0].0, 0.175, "the closest pair");
    assert_eq!((p[0].1, p[0].2), ("triple collision", "escape body 1"));
    let no_orange: Vec<_> = p
        .iter()
        .filter(|q| q.1 != "triple collision" && q.2 != "triple collision")
        .collect();
    near(
        no_orange[0].0,
        0.185,
        "the closest pair, the orange left out",
    );
    assert_eq!(
        (no_orange[0].1, no_orange[0].2),
        ("collision 1–3", "escape body 3")
    );
    let purple = set[8];
    let mut from: Vec<(f64, String)> = named[..8]
        .iter()
        .map(|&(n, h)| (dist(purple, h), n.to_owned()))
        .chain(
            [
                ("grey", GREY),
                ("dot", DOT),
                ("violet", VIOLET),
                ("cyan", CYAN),
                ("pending", PENDING),
                ("tint", TINT),
            ]
            .map(|(n, h)| (dist(purple, h), n.to_owned())),
        )
        .collect();
    from.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("purple's distances: {from:?}");
    assert!(
        from[0].0 >= 0.19,
        "purple is {:.4} from {}",
        from[0].0,
        from[0].1
    );
    near(from[0].0, 0.192, "purple's nearest");
    assert_eq!(from[0].1, "grey");
    let t = tau();
    near(t, 0.101, "τ");
    for (n, h) in &named {
        let d = dist(*h, DOT);
        assert!(d > t, "{n} is {d:.4} from the dot, within τ");
    }
}

#[test]
fn qa_m2_32_separation_srgb() {
    check_srgb_evidence(&CLASSES.map(|c| c.2));
}

negative_control!(
    qa_m2_32_separation_srgb,
    "the M1 palette's navy in place of purple is not R-445's evidence",
    expected = "purple is 0.1780 from pending",
    {
        let mut set = CLASSES.map(|c| c.2);
        set[8] = NAVY;
        check_srgb_evidence(&set)
    }
);

/// Whether OKLab `(l, c, hue°)` is inside the sRGB gamut.
fn in_gamut(l: f64, c: f64, hue: f64) -> bool {
    let h = hue.to_radians();
    linear_of_oklab([l, c * h.cos(), c * h.sin()])
        .iter()
        .all(|&x| (-1e-12..=1.0 + 1e-12).contains(&x))
}

/// The largest chroma in gamut at `(l, hue°)`, by bisection.
fn max_chroma(l: f64, hue: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 0.5);
    for _ in 0..60 {
        let m = 0.5 * (lo + hi);
        if in_gamut(l, m, hue) {
            lo = m;
        } else {
            hi = m;
        }
    }
    lo
}

/// Checks R-447's figures for the variant `set`.
fn check_variant_evidence(set: &[u32; 9]) {
    let named: Vec<(&str, u32)> = CLASSES.iter().zip(set).map(|(c, &h)| (c.1, h)).collect();
    let chromatic = &named[1..];
    let p = pairs(chromatic);
    println!("the variant, pairs under τ and the next: {:?}", &p[..6]);
    near(p[0].0, 0.063, "the variant's closest pair");
    assert_eq!((p[0].1, p[0].2), ("escape body 2", "triple ejection"));
    let t = tau();
    let under: Vec<(&str, &str)> = p.iter().filter(|q| q.0 < t).map(|q| (q.1, q.2)).collect();
    assert_eq!(
        under,
        [
            ("escape body 2", "triple ejection"),
            ("collision 1–2", "triple collision"),
            ("triple collision", "escape body 1"),
            ("collision 2–3", "triple ejection"),
            ("collision 1–3", "escape body 1"),
        ],
        "the pairs under τ"
    );
    for (want, d) in [
        (0.079, p[1].0),
        (0.081, p[2].0),
        (0.084, p[3].0),
        (0.091, p[4].0),
    ] {
        near(d, want, "a pair under τ");
    }
    let least = |others: &[u32]| {
        chromatic
            .iter()
            .flat_map(|&(_, h)| others.iter().map(move |&o| dist(h, o)))
            .fold(f64::INFINITY, f64::min)
    };
    for (what, others, at_least) in [
        ("the dot", vec![DOT], 0.139),
        ("the hatch", vec![VIOLET, CYAN], 0.196),
        ("the tint", vec![TINT], 0.132),
        ("the grey", vec![GREY], 0.320),
    ] {
        let d = least(&others);
        println!("the variant's nearest to {what}: {d:.4}");
        assert!(
            d >= at_least - 5e-4,
            "a chromatic swatch is {d:.4} from {what}, under {at_least}"
        );
    }
    // C = 0.123 at L = 0.72: in gamut at each listed hue; the largest in-gamut chroma at every hue 0.1224, near 199.8°.
    for c in &CLASSES[1..] {
        assert!(
            in_gamut(0.72, 0.123, c.4),
            "C 0.123 is out of gamut at {}°",
            c.4
        );
    }
    let (worst, at) = (0..3600)
        .map(|k| f64::from(k) / 10.0)
        .map(|h| (max_chroma(0.72, h), h))
        .fold((f64::INFINITY, 0.0), |a, b| if b.0 < a.0 { b } else { a });
    println!(
        "the largest chroma in gamut at every hue, L 0.72: {worst:.5}, at {at:.1}°; at 208°: {:.5}",
        max_chroma(0.72, 208.0)
    );
    near(worst, 0.1224, "the largest chroma in gamut at every hue");
    assert!(
        (at - 199.8).abs() <= 1.0,
        "the narrowest hue is {at}°, not near 199.8°"
    );
    near(
        max_chroma(0.72, 208.0),
        0.1238,
        "the largest chroma at escape body 3's 208°",
    );
}

#[test]
fn qa_m2_32_separation_variant() {
    check_variant_evidence(&variant_hexes());
}

negative_control!(
    qa_m2_32_separation_variant,
    "the sRGB palette taken as the variant is not R-447's evidence",
    expected = "the variant's closest pair",
    check_variant_evidence(&CLASSES.map(|c| c.2))
);

// ── REQ-COL-102: the chart's classification ─────────────────────────────────────────────────────────────────────

/// QA's chart: the latent chart pinned at the origin, which `validate` rejects at `u > 0.6` (layer 2), projects at
/// `v > 0.8`, clamps at `v < 0.05`, and whose `Φ` gives a NaN component at `u < 0.1` and +∞ at `0.1 ≤ u < 0.15`.
struct QaChart {
    latent: Latent<f64>,
}

impl Chart for QaChart {
    type Param = f64;
    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        let mut out = self.latent.map(u, v);
        if u < <F as From<f64>>::from(0.1) {
            out.z.z[3] = F::nan();
        } else if u < <F as From<f64>>::from(0.15) {
            out.z.z[5] = F::infinity();
        }
        out
    }
    fn name(&self) -> &str {
        "qa"
    }
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        if u > 0.6 {
            ValidationResult::Reject {
                failed: Constraint::MassPositive,
            }
        } else if v > 0.8 {
            ValidationResult::Project {
                u,
                v: 0.8,
                failed: Constraint::MassSum,
            }
        } else if v < 0.05 {
            ValidationResult::Clamp {
                u,
                v: 0.05,
                moved: [false, true],
                failed: Constraint::MassBuffer,
            }
        } else {
            ValidationResult::Valid
        }
    }
}

fn qa_chart() -> QaChart {
    QaChart {
        latent: Latent {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
    }
}

/// The points of `[0,1]²` checked: a 41 × 41 lattice, its edges at exactly 0 and 1.
fn lattice() -> Vec<[f64; 2]> {
    (0..=40)
        .flat_map(|j| (0..=40).map(move |i| [f64::from(i) / 40.0, f64::from(j) / 40.0]))
        .collect()
}

/// Checks a classifier against REQ-COL-102 on QA's chart: forbidden exactly where `validate` rejects or `Φ` is not
/// finite, `None` outside the square.
fn check_classifier(classify: &dyn Fn(&QaChart, f64, f64) -> Option<bool>) {
    let c = qa_chart();
    for [u, v] in lattice() {
        let want = c.validate(u, v) != ValidationResult::Valid
            || !c.map(u, v).z.z.iter().all(|x: &f64| x.is_finite());
        assert_eq!(classify(&c, u, v), Some(want), "QA's chart at ({u}, {v})");
    }
    for [u, v] in [
        [-1e-9, 0.5],
        [1.0 + 1e-9, 0.5],
        [0.5, -0.25],
        [0.5, 1.5],
        [2.0, -1.0],
    ] {
        assert_eq!(classify(&c, u, v), None, "({u}, {v}), outside the square");
    }
}

#[test]
fn qa_m2_32_chart_class_validate_and_map() {
    check_classifier(&|c, u, v| chart_class::forbidden(c, u, v));
}

negative_control!(
    qa_m2_32_chart_class_validate_and_map,
    "a classifier that reads only validate leaves Φ's NaN unhatched",
    expected = "QA's chart at (0, 0.05)",
    check_classifier(&|c, u, v| {
        let inside = (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v);
        inside.then(|| c.validate(u, v) != ValidationResult::Valid)
    })
);

/// A latent chart from a seed, its centre and plane's directions in `[-3, 3)` (a small LCG).
fn latent(seed: u64) -> Latent<f64> {
    let mut s = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let mut next = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((s >> 11) as f64 / (1u64 << 53) as f64) * 6.0 - 3.0
    };
    Latent {
        z0: std::array::from_fn(|_| next()),
        q1: std::array::from_fn(|_| next()),
        q2: std::array::from_fn(|_| next()),
    }
}

/// The latent chart pinned at `z0`.
fn pinned(z0: [f64; 8]) -> Latent<f64> {
    Latent {
        z0,
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

/// A classifier of the latent chart's points: REQ-COL-102's field at `(u, v)`.
type LatentClassifier = dyn Fn(&Latent<f64>, f64, f64) -> Option<bool>;

/// Checks a classifier on the latent chart, `validate` `Valid` everywhere: no point forbidden, the points where the
/// shared decoder raises `M01_TINY` and `ENERGY_UNSCALABLE` among them (R-446: the decoder's own outcomes never set the
/// field).
fn check_latent(classify: &LatentClassifier) {
    let tiny = pinned([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 30.0, 30.0]);
    assert_eq!(
        validation::chart::decode::<StickBreakingLinks, f64, _>(&tiny, 0.5, 0.5).degenerate,
        Some(Degenerate::M01Tiny),
        "the decoder does not raise M01_TINY there"
    );
    let rest = pinned([0.0; 8]);
    assert_eq!(
        energy_norm::decode_chart::<Defaults, f64, _>(&rest, 0.5, 0.5, Some(0.0))
            .0
            .degenerate,
        Some(Degenerate::EnergyUnscalable),
        "the decoder does not raise ENERGY_UNSCALABLE there"
    );
    for (what, c) in [("M01_TINY", tiny), ("ENERGY_UNSCALABLE", rest)] {
        assert_eq!(
            classify(&c, 0.5, 0.5),
            Some(false),
            "the {what} point is forbidden"
        );
    }
    for seed in 0..16 {
        let c = latent(seed);
        for [u, v] in lattice().into_iter().step_by(7) {
            assert_eq!(
                classify(&c, u, v),
                Some(false),
                "latent chart {seed} at ({u}, {v})"
            );
        }
    }
}

#[test]
fn qa_m2_32_chart_class_latent_never_forbidden() {
    check_latent(&|c, u, v| chart_class::forbidden(c, u, v));
}

negative_control!(
    qa_m2_32_chart_class_latent_never_forbidden,
    "a field set from the decoder's tag fails (R-446)",
    expected = "the M01_TINY point is forbidden",
    check_latent(&|c, u, v| {
        let tag = validation::chart::decode::<StickBreakingLinks, f64, _>(c, u, v).degenerate;
        chart_class::forbidden(c, u, v).map(|f| f || tag.is_some())
    })
);

/// Checks `fill` on a 5 × 8 grid of QA's chart: each sample's field its point's classification, the bound bytes bit
/// `i mod 32` of little-endian word `i / 32`; a point outside the square refused, naming its sample.
fn check_fill(bit: &dyn Fn(&[u8], usize) -> bool) {
    let g = Grid::new([5, 8], 1, 0, 4).expect("a 5 × 8 grid");
    let n = g.sample_count() as usize;
    assert!(n > 32, "{n} samples do not reach a second word");
    let point = |i: u32| [f64::from(i % 7) / 6.0, f64::from(i / 7) / 6.0];
    let mut set = Synthetic::flat(g, 4);
    let c = qa_chart();
    chart_class::fill(&mut set, &c, point).unwrap_or_else(|e| panic!("{e}"));
    let bytes = set.bytes().chart_forbidden;
    assert_eq!(bytes.len() % 4, 0, "the lane is not whole words");
    assert!(
        bytes.len() >= n.div_ceil(32) * 4,
        "the lane holds {} bytes for {n} samples",
        bytes.len()
    );
    let mut forbidden = 0;
    for i in 0..n as u32 {
        let [u, v] = point(i);
        let want = chart_class::forbidden(&c, u, v) == Some(true);
        assert_eq!(set.forbidden(i), want, "sample {i}'s field");
        assert_eq!(bit(&bytes, i as usize), want, "sample {i}'s bit");
        forbidden += usize::from(want);
    }
    assert!(
        forbidden > 0 && forbidden < n,
        "{forbidden} of {n} forbidden: no contrast"
    );
    let err = chart_class::fill(
        &mut set,
        &c,
        |i| if i == 17 { [1.25, 0.5] } else { point(i) },
    )
    .expect_err("a point outside the square is classified");
    assert!(
        err.contains("17"),
        "the refusal does not name sample 17: {err}"
    );
}

#[test]
fn qa_m2_32_chart_class_fill_and_bytes() {
    check_fill(&|b, i| {
        (u32::from_le_bytes(b[i / 32 * 4..i / 32 * 4 + 4].try_into().unwrap()) >> (i % 32)) & 1 == 1
    });
    assert_eq!(
        render::bind::chart_words(&[]),
        [0],
        "no sample: one empty word"
    );
    let mut f = vec![false; 33];
    (f[0], f[31], f[32]) = (true, true, true);
    assert_eq!(render::bind::chart_words(&f), [0x8000_0001, 1]);
}

negative_control!(
    qa_m2_32_chart_class_fill_and_bytes,
    "the bytes read big-endian within a word fail",
    expected = "bit",
    check_fill(
        &|b, i| (u32::from_be_bytes(b[i / 32 * 4..i / 32 * 4 + 4].try_into().unwrap()) >> (i % 32))
            & 1
            == 1
    )
);

// ── REQ-COL-002: the re-rendered goldens ────────────────────────────────────────────────────────────────────────

/// A golden reference: its width and pixels, RGBA8, opaque where the file holds RGB. The harness stores the linear
/// colour, 8-bit.
fn golden(case: &str) -> (u32, Vec<[u8; 4]>) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(case)
        .join("reference.png");
    let f = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let mut r = png::Decoder::new(std::io::Cursor::new(f))
        .read_info()
        .unwrap_or_else(|e| panic!("{e}"));
    let mut buf = vec![0; r.output_buffer_size().expect("a buffer size")];
    let info = r.next_frame(&mut buf).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "{case}: not 8-bit");
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => panic!("{case}: {other:?}, not RGB or RGBA"),
    };
    let px = buf[..info.buffer_size()]
        .chunks(channels)
        .map(|c| [c[0], c[1], c[2], c.get(3).copied().unwrap_or(255)])
        .collect();
    (info.width, px)
}

/// The 8-bit store of linear `c`.
fn store(c: Rgb) -> [u8; 3] {
    c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// What a golden tile must show.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tile {
    Flat(u32),
    Hatch,
}

/// Checks a golden's tiles, 8 px square in one row, against `want`, and that no tile shows a colour R-444 to R-446
/// removed from the outcome view.
fn check_golden(case: &str, width: u32, px: &[[u8; 4]], want: &[Tile]) {
    assert_eq!(width, want.len() as u32 * 8, "{case}: {width} px wide");
    let removed = [WHITE, NAVY, GREY, DOT, 0xD6A1FF].map(|h| store(linear(h)));
    for (k, w) in want.iter().enumerate() {
        for y in 0..8u32 {
            for x in k as u32 * 8..(k as u32 + 1) * 8 {
                let p = px[(y * width + x) as usize];
                let got = [p[0], p[1], p[2]];
                let ok = p[3] == 255
                    && match *w {
                        Tile::Flat(h) => got == store(linear(h)),
                        Tile::Hatch => {
                            got == store(render::present::debug_invalid([
                                f64::from(x) + 0.5,
                                f64::from(y) + 0.5,
                            ]))
                        }
                    };
                assert!(
                    ok,
                    "{case}: tile {k} pixel ({x}, {y}) is {p:02X?}, not {w:?}"
                );
                assert!(
                    *w == Tile::Hatch || !removed.contains(&got),
                    "{case}: tile {k} shows a colour R-444 to R-446 removed"
                );
            }
        }
    }
}

/// The `outcome` scene's tiles, from its samples as the scene names them, and the rows REQ-COL-002's verify asks for.
fn outcome_tiles(samples: &[(&str, u32, u32)], forbidden: usize) -> Vec<Tile> {
    let code = |name: &str| match name {
        "escape" => ESCAPE,
        "bounded" => BOUNDED,
        "collision" => COLLISION,
        "running" => RUNNING,
        "sim_failed" => SIM_FAILED,
        "decode_failed" => DECODE_FAILED,
        "reserved 6" => 6,
        "reserved 7" => 7,
        other => panic!("no state {other}"),
    };
    let rows: Vec<(u32, u32, u32)> = samples.iter().map(|&(s, d, t)| (code(s), d, t)).collect();
    let has = |f: &dyn Fn(&(u32, u32, u32)) -> bool, what: &str| {
        assert!(
            rows.iter().enumerate().any(|(k, r)| k != forbidden && f(r)),
            "the outcome scene has no {what} row"
        );
    };
    for d in 0..4 {
        has(
            &|r| r.0 == COLLISION && r.1 == d && r.2 > 0,
            "collision with t_end_step > 0",
        );
        has(&|r| r.0 == ESCAPE && r.1 == d, "escape");
        has(
            &|r| r.0 == SIM_FAILED && r.1 == d,
            "sim_failed of each detail",
        );
        has(
            &|r| r.0 == DECODE_FAILED && r.1 == d,
            "decode_failed of each detail",
        );
    }
    has(&|r| r.0 == BOUNDED, "bounded");
    has(&|r| r.0 == RUNNING, "running");
    has(
        &|r| r.0 == COLLISION && r.1 < 3 && r.2 == 0,
        "pair collision at t_end_step == 0",
    );
    has(
        &|r| r.0 == COLLISION && r.1 == 3 && r.2 == 0,
        "triple collision at t_end_step == 0",
    );
    rows.iter()
        .enumerate()
        .map(|(k, &(s, d, _))| {
            if k == forbidden {
                Tile::Hatch
            } else {
                Tile::Flat(CLASSES[rank(s, d)].2)
            }
        })
        .collect()
}

#[test]
fn qa_m2_32_goldens_outcome() {
    use validation::golden_scene::{OUTCOME_FORBIDDEN, OUTCOME_SAMPLES};
    let tiles = outcome_tiles(&OUTCOME_SAMPLES, OUTCOME_FORBIDDEN as usize);
    let (w, base) = golden("m1-outcome/outcome");
    check_golden("m1-outcome/outcome", w, &base, &tiles);
    // Edited: escape body 2's swatch (`escape_body_1`) to #808020, every other tile bit-identical.
    let edited_tiles: Vec<Tile> = tiles
        .iter()
        .map(|&t| {
            if t == Tile::Flat(CLASSES[6].2) {
                Tile::Flat(0x808020)
            } else {
                t
            }
        })
        .collect();
    assert_ne!(edited_tiles, tiles, "the scene has no escape body 2 tile");
    let (we, edited) = golden("m1-outcome/outcome_edited");
    check_golden("m1-outcome/outcome_edited", we, &edited, &edited_tiles);
}

negative_control!(
    qa_m2_32_goldens_outcome,
    "the reference read as the M1 table, decode_failed in the white, fails",
    expected = "m1-outcome/outcome: tile",
    {
        use validation::golden_scene::{OUTCOME_FORBIDDEN, OUTCOME_SAMPLES};
        let tiles: Vec<Tile> = outcome_tiles(&OUTCOME_SAMPLES, OUTCOME_FORBIDDEN as usize)
            .into_iter()
            .zip(OUTCOME_SAMPLES)
            .map(|(t, s)| {
                if s.0 == "decode_failed" && t != Tile::Hatch {
                    Tile::Flat(WHITE)
                } else {
                    t
                }
            })
            .collect();
        let (w, base) = golden("m1-outcome/outcome");
        check_golden("m1-outcome/outcome", w, &base, &tiles);
    }
);

/// `known_escaper`'s tiles, from `engine::fixtures`' table as the fixture's doc gives it: escapes of bodies 1, 2, 3,
/// the triple ejection, the earliest escape (body 2), bounded, a collision of pair 0, running.
fn known_escaper_tiles(triple_ejection: u32, running: u32) -> Vec<Tile> {
    vec![
        Tile::Flat(CLASSES[5].2),
        Tile::Flat(CLASSES[6].2),
        Tile::Flat(CLASSES[7].2),
        Tile::Flat(triple_ejection),
        Tile::Flat(CLASSES[6].2),
        Tile::Flat(CLASSES[0].2),
        Tile::Flat(CLASSES[3].2),
        Tile::Flat(running),
    ]
}

#[test]
fn qa_m2_32_goldens_known_escaper() {
    let (w, px) = golden("m1-synthetic/known_escaper");
    check_golden(
        "m1-synthetic/known_escaper",
        w,
        &px,
        &known_escaper_tiles(0x611597, 0x141418),
    );
}

negative_control!(
    qa_m2_32_goldens_known_escaper,
    "the M1 reference's navy triple ejection is not the re-rendered golden",
    expected = "m1-synthetic/known_escaper: tile 3",
    {
        let (w, px) = golden("m1-synthetic/known_escaper");
        check_golden(
            "m1-synthetic/known_escaper",
            w,
            &px,
            &known_escaper_tiles(NAVY, 0x141418),
        );
    }
);
