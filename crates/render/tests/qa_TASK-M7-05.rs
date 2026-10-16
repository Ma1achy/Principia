//! QA's tests for TASK-M7-05, written from the requirements it closes, not from the implementation:
//! - REQ-COL-039 (dd_colouring §3.6, §5 test 7; R-411): the forms' known answers, each form monotone on its domain, the
//!   symlog's `b(x) + b(−x) = 1` and `b(0) = ½` exactly, on the CPU and in `compaction.wgsl` on the GPU, no `x_max`
//!   (no value ever reaches the ramp's end), `log` mapping 0 to 0 and styling the −1.0 sentinel instead of ramping it;
//! - REQ-COL-016 (colour_composition §1.2; R-399, R-400, R-426): every ledger field's default ramp by its role, signed
//!   by its declared range, through a neutral at zero; FTLE and diffusion white = high, `t_end` white = low / early;
//! - REQ-COL-032 (dd_colouring §2, §4 seam 5): a ledger scale change re-styles the view, and recomputes nothing: the sim
//!   buffer the frame reads is, read back, the bytes it was before the re-style. Whether the edit may move the payload
//!   schema version (R-36 hashes the whole ledger, `scale` included) is RQ-297's open question, so no test here asserts
//!   either side of it;
//! - REQ-COL-024 (colour_composition §4.1; gui_state_contract §4): FTLE bound as the colour through a ramp and as the
//!   brightness, each compiling and rendering its field; the only constraint is the output signature.
//!
//! Each test registers its negative control (R-176).

use ledger::constants::{DIVERGING_X0, LOG_FLOORS};
use ledger::gen;
use ledger::schema::{Entry, FieldType, Scale};
use render::assemble::{Kind, Node, Occupant, Stain, Tier};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::colour::field_ramp::{
    Compaction, FieldRamp, Lut, Map, Polarity, Ramp, Reads, ScalarField, Shown, COMPACTION_WGSL,
    RAMP_WGSL,
};
use render::compositor::LAYER_FORMAT;
use render::frame_record::RenderLoop;
use render::pipeline_cache::FrameInputs;
use render::present::{self, Rgb};
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{render_stain, row_scene, scene, Colouring, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

fn entries() -> Vec<Entry> {
    gen::validate(&ledger::payload::ledger()).unwrap_or_else(|e| panic!("{e}"))
}

fn entry(name: &str) -> Entry {
    entries()
        .into_iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"))
}

// ── REQ-COL-039: the forms, by the requirement's own formulas ────────────────────────────────────────────────────

/// The f64 rounding of a form's few operations, relative: four roundings of a unit-sized value. A wrong form is off by
/// far more than this (its controls by more than 10⁻³).
const F64_ROUNDING: f64 = 4.0 * f64::EPSILON;

/// R-411's `log`: `1 − 1/(1 + ln(1 + |x|/ε))`, for `x ≥ 0`.
fn log_form(x: f64, eps: f64) -> f64 {
    1.0 - 1.0 / (1.0 + (1.0 + x.abs() / eps).ln())
}

/// R-411's symlog: `½ + ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x₀)))`, `sign(0) = 0`.
fn symlog_form(x: f64, x0: f64) -> f64 {
    let sign = if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    };
    0.5 + 0.5 * sign * (1.0 - 1.0 / (1.0 + (1.0 + x.abs() / x0).ln()))
}

/// Magnitudes, relative to a form's scale, from 10⁻¹² (where `1 + |x|/scale` still resolves in f64) to 10³⁰, eight per
/// decade, and 0.
fn magnitudes() -> Vec<f64> {
    let mut xs = vec![0.0];
    xs.extend((-96..=240).map(|k| 10f64.powf(f64::from(k) / 8.0)));
    xs
}

/// The forms of `place` against the requirement's formulas: `lin` over its range, clamped; `log` and the symlog at
/// every magnitude, with their landmarks (`x = ε(e − 1)` gives `b = ½`, `x = x₀(e − 1)` gives `b = ¾`, `x = 0` gives
/// `b = 0` for `log` and `½` for the symlog, exactly); `cyclic` as `frac(x/period)`, negative `x` included; `flag`
/// `{0, 1}`.
fn check_known_answers(place: impl Fn(Compaction, f64) -> f64) {
    let near = |got: f64, want: f64, what: &str| {
        assert!(
            (got - want).abs() <= F64_ROUNDING * want.abs().max(1.0),
            "{what}: the form gives {got}, the requirement's formula {want}"
        );
    };
    let lin = Compaction::Lin { lo: -2.0, hi: 6.0 };
    for (x, want) in [
        (-2.0, 0.0),
        (0.0, 0.25),
        (2.0, 0.5),
        (6.0, 1.0),
        (-9.0, 0.0),
        (9.0, 1.0),
    ] {
        near(place(lin, x), want, &format!("lin [−2, 6] at {x}"));
    }
    for eps in [2f64.powi(-24), 1e-3, 1.0, 37.5] {
        let log = Compaction::Log { eps };
        for m in magnitudes() {
            let x = m * eps;
            near(
                place(log, x),
                log_form(x, eps),
                &format!("log ε = {eps} at {x}"),
            );
        }
        near(
            place(log, eps * (std::f64::consts::E - 1.0)),
            0.5,
            &format!("log ε = {eps} at ε(e − 1)"),
        );
        assert_eq!(
            place(log, 0.0),
            0.0,
            "log ε = {eps}: x = 0 is not b = 0 exactly"
        );
    }
    for x0 in [2f64.powi(-20), 1.0, 1e3] {
        let sym = Compaction::Diverging { x0 };
        for m in magnitudes() {
            for x in [m * x0, -m * x0] {
                near(
                    place(sym, x),
                    symlog_form(x, x0),
                    &format!("symlog x₀ = {x0} at {x}"),
                );
            }
        }
        near(
            place(sym, x0 * (std::f64::consts::E - 1.0)),
            0.75,
            &format!("symlog x₀ = {x0} at x₀(e − 1)"),
        );
        near(
            place(sym, -x0 * (std::f64::consts::E - 1.0)),
            0.25,
            &format!("symlog x₀ = {x0} at −x₀(e − 1)"),
        );
        assert_eq!(
            place(sym, 0.0),
            0.5,
            "symlog x₀ = {x0}: b(0) is not ½ exactly"
        );
        assert_eq!(
            place(sym, -0.0),
            0.5,
            "symlog x₀ = {x0}: b(−0) is not ½ exactly"
        );
    }
    let tau = std::f64::consts::TAU;
    let cyc = Compaction::Cyclic { period: tau };
    for (x, want) in [
        (0.0, 0.0),
        (0.25 * tau, 0.25),
        (1.5 * tau, 0.5),
        (-0.25 * tau, 0.75),
        (-2.5 * tau, 0.5),
    ] {
        near(place(cyc, x), want, &format!("cyclic over 2π at {x}"));
    }
    for (x, want) in [(0.0, 0.0), (1.0, 1.0)] {
        assert_eq!(place(Compaction::Flag, x), want, "flag at {x}");
    }
}

#[test]
fn qa_compaction_forms_known_answers() {
    check_known_answers(|c, x| c.place(x));
}

negative_control!(
    qa_compaction_forms_known_answers,
    "a symlog with log₁₀ in place of ln is not R-411's form",
    expected = "symlog x₀",
    check_known_answers(|c, x| match c {
        Compaction::Diverging { x0 } => {
            let s = (1.0 + x.abs() / x0).log10();
            0.5 + 0.5 * x.signum() * (1.0 - 1.0 / (1.0 + s)) * f64::from(u8::from(x != 0.0))
        }
        _ => c.place(x),
    })
);

/// Each form monotone on its domain (§5 test 7): `lin` non-decreasing everywhere and increasing inside its range;
/// `log` increasing on `[0, ∞)`; the symlog increasing on the whole line; `cyclic` increasing within one period; `flag`
/// 0 below 1. And no form has an `x_max` (R-411): `log` and the symlog are below 1 at 10²⁴.
fn check_monotone(place: impl Fn(Compaction, f64) -> f64) {
    let increasing = |c: Compaction, xs: &[f64], what: &str| {
        for w in xs.windows(2) {
            let (a, b) = (place(c, w[0]), place(c, w[1]));
            assert!(
                a < b,
                "{what} is not increasing: b({}) = {a}, b({}) = {b}",
                w[0],
                w[1]
            );
        }
    };
    let mags = magnitudes();
    let lin = Compaction::Lin { lo: -1.0, hi: 3.0 };
    let inside: Vec<f64> = (0..=64).map(|k| -1.0 + 4.0 * f64::from(k) / 64.0).collect();
    increasing(lin, &inside, "lin inside its range");
    for w in [-5.0, -1.0, 3.0, 7.0].windows(2) {
        assert!(
            place(lin, w[0]) <= place(lin, w[1]),
            "lin decreases outside its range"
        );
    }
    let scaled: Vec<f64> = mags.iter().map(|m| m * 1e-6).collect();
    increasing(Compaction::Log { eps: 1e-6 }, &scaled, "log on [0, ∞)");
    let mut line: Vec<f64> = mags.iter().rev().map(|m| -m).collect();
    line.extend(mags.iter().skip(1));
    increasing(
        Compaction::Diverging { x0: 1.0 },
        &line,
        "the symlog on the line",
    );
    let tau = std::f64::consts::TAU;
    let turn: Vec<f64> = (0..64).map(|k| tau * f64::from(k) / 64.0).collect();
    increasing(
        Compaction::Cyclic { period: tau },
        &turn,
        "cyclic within a turn",
    );
    let shifted: Vec<f64> = turn.iter().map(|x| x - 2.0 * tau).collect();
    increasing(
        Compaction::Cyclic { period: tau },
        &shifted,
        "cyclic within a turn two turns down",
    );
    increasing(Compaction::Flag, &[0.0, 1.0], "flag");
    for (c, what) in [
        (Compaction::Log { eps: 1e-6 }, "log"),
        (Compaction::Diverging { x0: 1.0 }, "the symlog"),
    ] {
        let top = place(c, 1e24);
        assert!(top < 1.0, "{what} reaches its end at 10²⁴: an x_max");
    }
}

#[test]
fn qa_compaction_forms_monotone_with_no_x_max() {
    check_monotone(|c, x| c.place(x));
}

negative_control!(
    qa_compaction_forms_monotone_with_no_x_max,
    "a symlog clamped at an x_max of 10⁶ stops increasing there",
    expected = "the symlog on the line is not increasing",
    check_monotone(|c, x| match c {
        Compaction::Diverging { .. } => c.place(x.clamp(-1e6, 1e6)),
        _ => c.place(x),
    })
);

// ── REQ-COL-039 on the GPU: `compaction.wgsl` itself ─────────────────────────────────────────────────────────────

/// A compute entry over `compaction.wgsl`: case `i` is `(op, x, k)`, op 0 `log`, 1 the symlog, 2 `cyclic`, 3 `flag`, and
/// writes the result's bits.
const ENTRY: &str = r"
@group(0) @binding(0) var<storage, read> q_op: array<u32>;
@group(0) @binding(1) var<storage, read> q_x: array<u32>;
@group(0) @binding(2) var<storage, read> q_k: array<u32>;
@group(0) @binding(3) var<storage, read_write> q_out: array<u32>;
@compute @workgroup_size(64)
fn q_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&q_out) { return; }
    let x = bitcast<f32>(q_x[i]);
    let k = bitcast<f32>(q_k[i]);
    var b = -1.0;
    switch q_op[i] {
        case 0u: { b = compact_log(x, k); }
        case 1u: { b = compact_symlog(x, k); }
        case 2u: { b = compact_cyclic(x, k); }
        default: { b = compact_flag(x); }
    }
    q_out[i] = bitcast<u32>(b);
}
";

/// `lib` evaluated on the GPU at each `(op, x, k)`.
fn gpu_forms(h: &GpuHarness, lib: &str, cases: &[(u32, f32, f32)]) -> Vec<f32> {
    let module = format!("{lib}\n{ENTRY}");
    let ops: Vec<u32> = cases.iter().map(|c| c.0).collect();
    let xs: Vec<u32> = cases.iter().map(|c| c.1.to_bits()).collect();
    let ks: Vec<u32> = cases.iter().map(|c| c.2.to_bits()).collect();
    h.run_wgsl(&module, "q_main", &[&ops, &xs, &ks])
        .into_iter()
        .map(f32::from_bits)
        .collect()
}

/// Every positive finite f32 scale: from the least subnormal through 10⁻³⁰ … 10³⁸ (eight per decade) to `f32::MAX`.
fn f32_magnitudes() -> Vec<f32> {
    let mut xs = vec![f32::from_bits(1), 1e-40];
    xs.extend((-240..=304).map(|k| 10f32.powf(k as f32 / 8.0)));
    xs.push(f32::MAX);
    xs.retain(|x| x.is_finite() && *x > 0.0);
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    xs
}

/// `compaction.wgsl` on the GPU (§5 test 7): the symlog's `b(x) + b(−x) = 1` and `b(±0) = ½` exactly in f32 at every
/// scale; the symlog increasing in order across the line and `log` across `[0, ∞)`, below 1 at every finite f32 up to
/// `f32::MAX`, a quotient `|x|/scale` past `f32::MAX` included (no `x_max`, and no form forms an infinity, which WGSL
/// may assume absent: gpu_determinism_note rule 4); `log`'s `b(0) = 0` exactly (R-411's note); `cyclic` and `flag` as
/// the formulas give.
fn check_gpu_forms(h: &GpuHarness, lib: &str) {
    // Every finite magnitude: the forms take any finite `x` (§3.6 gives them no range), so the sweep runs to
    // `f32::MAX` at every scale, past where a naive `|x|/scale` overflows.
    for x0 in [2f32.powi(-20), 1.0, 1e3] {
        let mags = f32_magnitudes();
        let mut cases = vec![(1, 0.0, x0), (1, -0.0, x0)];
        for &m in &mags {
            cases.push((1, m, x0));
            cases.push((1, -m, x0));
        }
        let b = gpu_forms(h, lib, &cases);
        assert_eq!(
            b[0].to_bits(),
            0.5f32.to_bits(),
            "x₀ = {x0}: the symlog's b(0) is {} not ½",
            b[0]
        );
        assert_eq!(
            b[1].to_bits(),
            0.5f32.to_bits(),
            "x₀ = {x0}: the symlog's b(−0) is {} not ½",
            b[1]
        );
        for (k, &m) in mags.iter().enumerate() {
            let (up, down) = (b[2 + 2 * k], b[3 + 2 * k]);
            assert!(
                up + down == 1.0 && f64::from(up) + f64::from(down) == 1.0,
                "x₀ = {x0}: the symlog's b({m}) + b(−{m}) = {up} + {down} ≠ 1"
            );
            assert!(
                (0.5..1.0).contains(&up),
                "x₀ = {x0}: b({m}) = {up} is outside [½, 1)"
            );
        }
        for w in (0..mags.len()).collect::<Vec<_>>().windows(2) {
            let (a, c) = (b[2 + 2 * w[0]], b[2 + 2 * w[1]]);
            assert!(
                a <= c,
                "x₀ = {x0}: the symlog decreases from {} to {}",
                mags[w[0]],
                mags[w[1]]
            );
        }
        let top = b[2 + 2 * (mags.len() - 1)];
        assert!(
            top < 1.0,
            "x₀ = {x0}: the symlog reaches 1 at {}: an x_max",
            mags[mags.len() - 1]
        );
        // Across the decades the placement keeps rising: 10³·x₀ is placed below 10³⁰, which an x_max would clamp.
        let at = |x: f32| b[2 + 2 * mags.iter().position(|m| *m >= x).expect("a sweep point")];
        assert!(
            at(1e3 * x0) < at(1e30),
            "x₀ = {x0}: the symlog is flat between 10³·x₀ and 10³⁰"
        );
    }
    for eps in [2f32.powi(-24), 1e-3, 1.0] {
        let mags = f32_magnitudes();
        let mut cases = vec![(0, 0.0, eps)];
        cases.extend(mags.iter().map(|&m| (0, m, eps)));
        let b = gpu_forms(h, lib, &cases);
        assert_eq!(
            b[0].to_bits(),
            0f32.to_bits(),
            "ε = {eps}: log's b(0) is {} not 0",
            b[0]
        );
        for w in b.windows(2) {
            assert!(
                w[0] <= w[1],
                "ε = {eps}: log decreases: {} then {}",
                w[0],
                w[1]
            );
        }
        assert!(
            *b.last().expect("a value") < 1.0,
            "ε = {eps}: log reaches 1 at {}: an x_max",
            mags[mags.len() - 1]
        );
    }
    let tau = std::f32::consts::TAU;
    let cyc = gpu_forms(
        h,
        lib,
        &[
            (2, 0.25 * tau, tau),
            (2, -0.25 * tau, tau),
            (2, 1.5 * tau, tau),
        ],
    );
    assert_eq!(cyc, [0.25, 0.75, 0.5], "cyclic is not frac(x/period)");
    let flag = gpu_forms(h, lib, &[(3, 0.0, 0.0), (3, 1.0, 0.0)]);
    assert_eq!(flag, [0.0, 1.0], "flag");
}

#[test]
fn qa_compaction_forms_on_the_gpu() {
    check_gpu_forms(&gpu(), COMPACTION_WGSL);
}

/// A library whose symlog is the requirement's formula written naively with `sign`, but with its negative half's `x₀`
/// off by one part in 10³: a symlog that is not symmetric.
#[cfg(feature = "controls")]
const ASYMMETRIC: &str = r"
fn compact_log(x: f32, eps: f32) -> f32 { return 1.0 - 1.0 / (1.0 + log(1.0 + max(x, 0.0) / eps)); }
fn compact_cyclic(x: f32, period: f32) -> f32 { return fract(x / period); }
fn compact_flag(x: f32) -> f32 { return select(0.0, 1.0, x != 0.0); }
fn compact_symlog(x: f32, x0: f32) -> f32 {
    let s = select(x0, 1.001 * x0, x < 0.0);
    return 0.5 + 0.5 * sign(x) * (1.0 - 1.0 / (1.0 + log(1.0 + abs(x) / s)));
}
";

negative_control!(
    qa_compaction_forms_on_the_gpu,
    "a symlog whose two halves take different x₀ is not symmetric",
    expected = "≠ 1",
    check_gpu_forms(&gpu(), ASYMMETRIC)
);

/// A library whose symlog is the requirement's formula written naively, `log(1 + |x|/x₀)`: symmetric, but its
/// quotient overflows to +∞ for a large finite `|x|` over a small `x₀`, and the form reaches 1 there.
#[cfg(feature = "controls")]
const OVERFLOWING: &str = r"
fn compact_log(x: f32, eps: f32) -> f32 { return 1.0 - 1.0 / (1.0 + log(1.0 + max(x, 0.0) / eps)); }
fn compact_cyclic(x: f32, period: f32) -> f32 { return fract(x / period); }
fn compact_flag(x: f32) -> f32 { return select(0.0, 1.0, x != 0.0); }
fn compact_symlog(x: f32, x0: f32) -> f32 {
    let u = 0.5 + 0.5 * (1.0 - 1.0 / (1.0 + log(1.0 + abs(x) / x0)));
    return select(1.0 - u, u, x >= 0.0);
}
";

#[cfg(feature = "controls")]
mod qa_compaction_forms_on_the_gpu_overflow {
    use super::*;
    negative_control!(
        qa_compaction_forms_on_the_gpu,
        "a symlog whose |x|/x₀ overflows at a large finite x reaches 1 there",
        expected = "is outside [½, 1)",
        check_gpu_forms(&gpu(), OVERFLOWING)
    );
}

// ── REQ-COL-039: `log` styles the −1.0 sentinel, never ramps it ──────────────────────────────────────────────────

/// `closure_min`, a `log` field (its ledger scale) with its display floor (R-401), on viridis.
fn closure_min_ramp(c: Compaction) -> FieldRamp {
    FieldRamp::new(
        ScalarField::payload("closure_min").unwrap_or_else(|e| panic!("{e}")),
        c,
        Map::ramp(Ramp::Lut(Lut::Viridis)),
    )
}

fn closure_min_floor() -> f64 {
    LOG_FLOORS
        .iter()
        .find(|(n, _)| *n == "closure_min")
        .map(|(_, k)| k.number())
        .expect("closure_min has a display floor")
}

/// The stain of `occupant` in `kind`'s slot (the other slot None), Replace-L, and `post` after it, if any.
fn stain(kind: Kind, occupant: Occupant, post: Option<String>) -> Result<Stain, String> {
    let node = |kind, occupant, inputs: Vec<Option<usize>>| Node {
        kind,
        occupant,
        inputs,
    };
    let (colour, brightness) = match kind {
        Kind::Colour => (occupant, Occupant::None),
        _ => (Occupant::None, occupant),
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
            Occupant::BuiltIn("replace_l".into()),
            vec![Some(1), Some(2)],
        ),
    ];
    let mut last = 3;
    if let Some(p) = post {
        nodes.push(node(Kind::Post, Occupant::Custom(p), vec![Some(3)]));
        last = 4;
    }
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    Stain::new(nodes).map_err(|e| e.to_string())
}

/// `stain` rendered over `s`, or why it did not render.
fn render(h: &GpuHarness, s: &Scene, stain: &Stain) -> Result<Vec<[f32; 4]>, String> {
    render_stain(h.device(), h.queue(), stain, &s.set, &s.context, &|_, _| {
        None
    })
}

/// Sample `i`'s first pixel in `image` and its fragment position.
fn pixel(s: &Scene, image: &[[f32; 4]], i: u32) -> (Rgb, [f64; 2]) {
    let (w, _) = s.size();
    let (x, y) = s.pixels(i)[0];
    let p = image[(y * w + x) as usize];
    (
        [0, 1, 2].map(|c| f64::from(p[c])),
        [f64::from(x) + 0.5, f64::from(y) + 0.5],
    )
}

/// The tolerance of a rendered linear channel against the colour the requirement gives: the GPU's f32 evaluation of
/// the LUT and the form, far below the 1/255 sRGB step at which two colours would read apart.
const RENDER_TOL: f64 = 1.0 / 1024.0;

fn same(a: Rgb, b: Rgb) -> bool {
    (0..3).all(|c| (a[c] - b[c]).abs() <= RENDER_TOL)
}

/// The values the sentinel scene stores: the −1.0 sentinel, another negative value, then 0 and values above the floor.
const SENTINEL_VALUES: [f32; 6] = [-1.0, -0.5, 0.0, 1e-4, 1e-2, 1.0];

/// `ramp` styles every negative value of `closure_min` as the invalid hatch, on the CPU and in its render, and ramps 0
/// to the ramp's start and the rest to their R-411 placements.
fn check_sentinel(h: &GpuHarness, ramp: &FieldRamp) {
    let eps = closure_min_floor();
    assert_eq!(
        ramp.shown(-1.0, true),
        Shown::Invalid,
        "the CPU twin ramps the −1.0 sentinel"
    );
    let mut s = row_scene(
        "qa_log_sentinel",
        SENTINEL_VALUES.len() as u32,
        Colouring::View("closure_min"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in SENTINEL_VALUES.into_iter().enumerate() {
        s.set.sample(i as u32).closure_min(v);
    }
    let (kind, occupant) = ramp.occupant();
    let image = render(
        h,
        &s,
        &stain(kind, occupant, None).unwrap_or_else(|e| panic!("{e}")),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in SENTINEL_VALUES.into_iter().enumerate() {
        let (got, frag) = pixel(&s, &image, i as u32);
        let want = if v < 0.0 {
            present::debug_invalid(frag)
        } else {
            present::ramp_viridis(log_form(f64::from(v), eps))
        };
        assert!(
            same(got, want),
            "closure_min = {v} draws {got:?}, not {want:?} ({})",
            if v < 0.0 {
                "the sentinel is ramped, not styled"
            } else {
                "R-411's placement on viridis"
            }
        );
    }
}

#[test]
fn qa_compaction_forms_log_styles_the_sentinel() {
    check_sentinel(
        &gpu(),
        &closure_min_ramp(Compaction::Log {
            eps: closure_min_floor(),
        }),
    );
}

negative_control!(
    qa_compaction_forms_log_styles_the_sentinel,
    "a flag compaction, which places −1.0 rather than styling it, is caught",
    expected = "the CPU twin ramps the −1.0 sentinel",
    check_sentinel(&gpu(), &closure_min_ramp(Compaction::Flag))
);

// ── REQ-COL-016: the default ramp per field role ─────────────────────────────────────────────────────────────────

/// The sim key's floors, `eps_E` and `eps_L` (R-263), each a positive energy or angular momentum.
fn sim_key(name: &str) -> Option<f64> {
    matches!(name, "eps_E" | "eps_L").then_some(1e-9)
}

fn resolve() -> Resolve<'static> {
    Resolve { sim_key: &sim_key }
}

/// The run-time values the registry's open ends are read against (the measured range a field's finite values span,
/// the read side's `horizon_steps`): a range straddling zero, unequal ends, and a run of 1000 steps.
const READS: Reads = Reads {
    u_range: [-3.0, 5.0],
    horizon_steps: 1000,
};

/// The fields colour_composition §1.2 gives a polarity, by their ledger names: FTLE and diffusion white = high; the
/// time to the terminal event, `t_end`, stored as `t_end_step` (payload §2; §1.4), white = low / early.
const NAMED: [(&str, Polarity); 3] = [
    ("ftle", Polarity::High),
    ("diffusion", Polarity::High),
    ("t_end_step", Polarity::Low),
];

/// What §1.2 asks of `e`'s default, as `(ramp, polarity)`: a named magnitude field the greyscale at its polarity; an
/// angle (a `cyclic` field) the cyclic ramp; a field whose declared range spans zero (R-400, R-426) diverging through a
/// neutral; every other field sequential; each but the named fields at polarity high.
fn want(e: &Entry) -> (Lut, Polarity) {
    if let Some((_, p)) = NAMED.iter().find(|(n, _)| *n == e.name) {
        (Lut::Grey, *p)
    } else if e.scale == Scale::Cyclic {
        (Lut::Twilight, Polarity::High)
    } else if e.range_state().spans_zero() {
        (Lut::Coolwarm, Polarity::High)
    } else {
        (Lut::Viridis, Polarity::High)
    }
}

/// The ledger's fields a ramp takes: scalar, not categorical.
fn rampable() -> Vec<Entry> {
    entries()
        .into_iter()
        .filter(|e| {
            !matches!(e.ty, FieldType::Vector { .. }) && !matches!(e.scale, Scale::Categorical(_))
        })
        .collect()
}

/// Every rampable field's default, but those in `skip`, is §1.2's for its role; a signed field's places zero at the
/// diverging ramp's centre, which is neutral (achromatic); and the role's polarity orders its ends: the named magnitude
/// fields' white end is their salient one.
fn check_defaults(
    registry: impl Fn(&Entry) -> Result<(Lut, Polarity, Compaction), String>,
    skip: &[&str],
) {
    let mut checked = 0;
    for e in rampable().iter().filter(|e| !skip.contains(&e.name)) {
        let Ok((lut, polarity, compaction)) = registry(e) else {
            continue;
        };
        let (want_lut, want_polarity) = want(e);
        assert_eq!(
            (lut, polarity),
            (want_lut, want_polarity),
            "`{}`'s default ramp and polarity (scale {:?}, range {:?})",
            e.name,
            e.scale,
            e.range_state()
        );
        if want_lut == Lut::Coolwarm && e.scale != Scale::Log {
            assert_eq!(
                compaction.place_at(0.0, READS),
                0.5,
                "`{}`'s zero is not at the diverging ramp's neutral",
                e.name
            );
        }
        checked += 1;
    }
    for name in ["E_0", "Lz_0", "energy_drift", "Lz_drift", "theta", "ftle"] {
        assert!(
            skip.contains(&name) || registry(&entry(name)).is_ok(),
            "`{name}` has no default ramp: {:?}",
            registry(&entry(name)).err()
        );
    }
    assert!(checked >= 10, "only {checked} fields have a default");
    // Moreland's cool-warm midpoint is its neutral, (221, 221, 221) in 8-bit sRGB.
    let mid = present::ramp_coolwarm(0.5).map(|c| (present::linear_to_srgb(c) * 255.0).round());
    assert!(
        mid[0] == mid[1] && mid[1] == mid[2],
        "the diverging ramp's centre {mid:?} is not neutral"
    );
}

fn registry(e: &Entry) -> Result<(Lut, Polarity, Compaction), String> {
    let d = defaults::default_ramp(e, &resolve())?;
    match d.ramp {
        Ramp::Lut(l) => Ok((l, d.polarity, d.compaction)),
        other => Err(format!("`{}`'s default {other:?} is no LUT", e.name)),
    }
}

#[test]
fn qa_default_ramps_by_field_role() {
    // `t_end_step` is checked on its own, below.
    check_defaults(registry, &["t_end_step"]);
}

negative_control!(
    qa_default_ramps_by_field_role,
    "a registry that reads `E_0` as one-signed gives it viridis",
    expected = "`E_0`'s default ramp and polarity",
    check_defaults(
        |e| registry(e).map(|(l, p, c)| if e.name == "E_0" {
            (Lut::Viridis, p, c)
        } else {
            (l, p, c)
        }),
        &["t_end_step"]
    )
);

/// The time to the terminal event, `t_end_step`, is a magnitude field (§1.2: "time-to-event (`t_end`) → white =
/// low / early"): the greyscale, polarity low, so its earliest value is drawn whiter than its latest.
fn check_t_end(registry: impl Fn(&Entry) -> Result<(Lut, Polarity, Compaction), String>) {
    let e = entry("t_end_step");
    let (lut, polarity, compaction) = registry(&e).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(
        (lut, polarity),
        (Lut::Grey, Polarity::Low),
        "`t_end_step`'s default is not the greyscale, white = early"
    );
    let lum = |x: f64| {
        present::linear_to_oklab(lut.colour(polarity.apply(compaction.place_at(x, READS))))[0]
    };
    assert!(
        lum(0.0) > lum(1000.0),
        "`t_end_step`'s early end is not the brighter"
    );
}

#[test]
fn qa_default_ramps_t_end_white_is_early() {
    check_t_end(registry);
}

negative_control!(
    qa_default_ramps_t_end_white_is_early,
    "a t_end at polarity high, white = late, is caught",
    expected = "white = early",
    check_t_end(|e| registry(e).map(|(_, _, c)| (Lut::Grey, Polarity::High, c)))
);

/// FTLE and diffusion are greyscale, white = high: their larger value draws whiter.
fn check_white_high(registry: impl Fn(&Entry) -> Result<(Lut, Polarity, Compaction), String>) {
    for name in ["ftle", "diffusion"] {
        let (lut, polarity, compaction) =
            registry(&entry(name)).unwrap_or_else(|why| panic!("{why}"));
        let lum = |x: f64| {
            present::linear_to_oklab(lut.colour(polarity.apply(compaction.place_at(x, READS))))[0]
        };
        assert!(
            lum(4.0) > lum(-2.0),
            "`{name}`'s high end is not the brighter"
        );
    }
}

#[test]
fn qa_default_ramps_ftle_white_is_high() {
    check_white_high(registry);
}

negative_control!(
    qa_default_ramps_ftle_white_is_high,
    "an FTLE at polarity low is caught",
    expected = "`ftle`'s high end is not the brighter",
    check_white_high(|e| registry(e).map(|(l, _, c)| (l, Polarity::Low, c)))
);

// ── REQ-COL-032: a ledger scale change re-styles the view, with zero recompute ───────────────────────────────────

/// `E_0`'s values the re-style draws.
const E0: [f32; 5] = [-4.0, -0.5, 0.0, 0.5, 4.0];

/// The measured range of [`E0`], `u_range` as the measurement writes it.
const E0_RANGE: [f64; 2] = [-4.0, 4.0];

/// `E_0`'s default colour view from its entry `e`.
fn e0_view(e: &Entry) -> FieldRamp {
    defaults::field_ramp(e, Slot::Colour, &resolve()).unwrap_or_else(|why| panic!("{why}"))
}

/// `E_0`'s entry with its ledger scale changed to `to`, nothing else.
fn e0_rescaled(to: Scale) -> (Entry, Entry) {
    let before = entry("E_0");
    assert_eq!(before.scale, Scale::Diverging, "`E_0` is diverging (R-403)");
    let mut after = before.clone();
    after.scale = to;
    (before, after)
}

/// `E_0`'s scale changes from `diverging` to `to` in its ledger entry, nothing else: the view built from the changed
/// entry places its values otherwise and draws other pixels, each as the new form gives it, its measured range
/// written to `u_range` where it reads one.
fn check_restyle(h: &GpuHarness, to: Scale) {
    let (before, after) = e0_rescaled(to);
    let (a, b) = (e0_view(&before), e0_view(&after));
    let mut s = row_scene("qa_restyle", E0.len() as u32, Colouring::View("E_0"))
        .unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in E0.into_iter().enumerate() {
        s.set.sample(i as u32).E_0(v);
    }
    let reads = Reads {
        u_range: E0_RANGE,
        horizon_steps: s.context.horizon_steps,
    };
    let draw = |r: &FieldRamp| {
        let (kind, occupant) = r.occupant();
        render_stain(
            h.device(),
            h.queue(),
            &stain(kind, occupant, None).unwrap_or_else(|e| panic!("{e}")),
            &s.set,
            &s.context,
            &|_, name| (name == "u_range").then(|| E0_RANGE.to_vec()),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    };
    let (ia, ib) = (draw(&a), draw(&b));
    let mut moved = 0;
    for i in 0..E0.len() as u32 {
        let (pa, _) = pixel(&s, &ia, i);
        let (pb, _) = pixel(&s, &ib, i);
        let x = f64::from(E0[i as usize]);
        let Map::Ramp(ra) = &a.map else {
            panic!("a colour view")
        };
        let Map::Ramp(rb) = &b.map else {
            panic!("a colour view")
        };
        assert!(
            same(pa, ra.colour(a.compaction.place_at(x, reads))),
            "before: E_0 = {x} draws {pa:?}"
        );
        assert!(
            same(pb, rb.colour(b.compaction.place_at(x, reads))),
            "after: E_0 = {x} draws {pb:?}"
        );
        moved += usize::from(!same(pa, pb));
    }
    assert!(
        moved > 0,
        "the view did not re-style: {:?} → {:?}",
        a.compaction,
        b.compaction
    );
}

#[test]
fn qa_ledger_scale_restyle_restyles() {
    check_restyle(&gpu(), Scale::Lin);
}

negative_control!(
    qa_ledger_scale_restyle_restyles,
    "a scale \"change\" to the scale it had draws the same pixels",
    expected = "the view did not re-style",
    check_restyle(&gpu(), Scale::Diverging)
);

/// `buffer`'s bytes, read back.
fn read_back(h: &GpuHarness, buffer: &wgpu::Buffer) -> Vec<u8> {
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa readback"),
        size: buffer.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = h.device().create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, buffer.size());
    h.queue().submit([encoder.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let out = slice.get_mapped_range().expect("range").to_vec();
    staging.unmap();
    out
}

/// `texture`'s one row of `w` 8-bit pixels, read back.
fn read_texture(h: &GpuHarness, texture: &wgpu::Texture, w: u32) -> Vec<u8> {
    let row = 256u32;
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa texture readback"),
        size: u64::from(row),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = h.device().create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    h.queue().submit([encoder.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let out = slice.get_mapped_range().expect("range")[..(4 * w) as usize].to_vec();
    staging.unmap();
    out
}

/// "Zero recompute" within the frame (REQ-COL-032; dd_colouring §2: compaction "never re-integrates"; §4 seam 5): the
/// render loop draws `E_0`'s view, its scale changes to `lin`, and the loop draws the view built from the changed entry,
/// a compile and a frame. The frame's pixels change (it re-styled), and the sim buffer the colour pass reads is, read
/// back, byte for byte what was uploaded: nothing between the two frames recomputed the payload. `recompute` stands in
/// for a re-style that re-integrates: it rewrites the sim buffer between the frames.
///
/// Whether the ledger edit itself moves the payload schema version, and so invalidates every cached payload (R-36;
/// caching contract Part 1, "Validity"), against seam 5's "zero recompute", is RQ-297's open question: no assertion
/// here takes either side of it.
fn check_sim_buffer(recompute: bool) {
    use wgpu::util::DeviceExt;
    let w = E0.len() as u32;
    let mut s =
        row_scene("qa_restyle_sim", w, Colouring::View("E_0")).unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in E0.into_iter().enumerate() {
        s.set.sample(i as u32).E_0(v);
    }
    let h = gpu();
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, w, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("qa target"),
        size: wgpu::Extent3d {
            width: w,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let bytes = s.set.bytes();
    let upload = |label: &str, contents: &[u8]| {
        h.device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
    };
    let simstate = upload("qa simstate", &bytes.simstate);
    let word = upload("qa word", &bytes.word);
    rl.set_sim(simstate.clone(), Some(word));
    let inputs = FrameInputs {
        ensemble: 0,
        ensemble_spread: 0.0,
        masses: [1.0, 1.0, 1.0],
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: s.context.horizon_steps,
    };
    let readback = read_back(&h, &simstate);
    assert!(
        readback == bytes.simstate,
        "the sim buffer does not read back as uploaded"
    );
    let frame = |rl: &mut RenderLoop, r: &FieldRamp| -> Vec<u8> {
        let (kind, occupant) = r.occupant();
        let st = stain(kind, occupant, None).unwrap_or_else(|e| panic!("{e}"));
        let keys: Vec<u64> = (0..st.nodes().len() as u64).collect();
        rl.cache()
            .request(&st, &keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
        rl.cache().wait();
        assert!(rl.cache().errors().is_empty(), "{:?}", rl.cache().errors());
        if r.compaction.uniforms().contains("u_range") {
            // The colour node is the stain's second, keyed 1.
            rl.set_param(1, "u_range", &E0_RANGE)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        rl.frame(&inputs, &view);
        read_texture(&h, &target, w)
    };
    let (before, after) = e0_rescaled(Scale::Lin);
    let pa = frame(&mut rl, &e0_view(&before));
    if recompute {
        let fresh: Vec<u8> = bytes.simstate.iter().map(|b| b.wrapping_add(1)).collect();
        h.queue().write_buffer(&simstate, 0, &fresh);
    }
    let pb = frame(&mut rl, &e0_view(&after));
    assert!(
        pa != pb,
        "the view did not re-style across the scale change"
    );
    assert!(
        read_back(&h, &simstate) == bytes.simstate,
        "the sim buffer changed across the re-style: a recompute"
    );
}

#[test]
fn qa_ledger_scale_restyle_leaves_the_sim_buffer() {
    check_sim_buffer(false);
}

negative_control!(
    qa_ledger_scale_restyle_leaves_the_sim_buffer,
    "a re-style that rewrites the sim buffer between its frames, as a re-integration does, is caught",
    expected = "the sim buffer changed across the re-style",
    check_sim_buffer(true)
);

// ── REQ-COL-024: FTLE in either role ─────────────────────────────────────────────────────────────────────────────

/// FTLE on the `ftle` scene, `lin` over `[0, 4]`, as a colour through viridis and as a brightness: each compiles and
/// renders its field, the colour as `viridis(b)`, the brightness under Replace-L with the colour None as the greyscale
/// of `b` (colour_composition §4.1's None-colour row), with `b` the requirement's `lin`; the invalid FTLE samples take
/// the hatch in the colour role. `swap` binds each to the other's slot instead.
fn check_either_role(h: &GpuHarness, swap: bool) {
    let s = scene("ftle").unwrap_or_else(|e| panic!("{e}"));
    let field = ScalarField::payload("ftle").unwrap_or_else(|e| panic!("{e}"));
    let lin = Compaction::Lin { lo: 0.0, hi: 4.0 };
    let colour = FieldRamp::new(field.clone(), lin, Map::ramp(Ramp::Lut(Lut::Viridis)));
    let bright = FieldRamp::new(field, lin, Map::Lightness);
    let n = s.set.grid().sample_count();
    for (ramp, what) in [(&colour, "colour"), (&bright, "brightness")] {
        let (kind, occupant) = ramp.occupant();
        let kind = match (swap, kind) {
            (false, k) => k,
            (true, Kind::Colour) => Kind::Brightness,
            (true, _) => Kind::Colour,
        };
        let image = stain(kind, occupant, None)
            .and_then(|st| render(h, &s, &st))
            .unwrap_or_else(|e| panic!("FTLE as {what} did not compile and render: {e}"));
        let mut placed = Vec::new();
        for i in 0..n {
            let (v, valid) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
            let (got, frag) = pixel(&s, &image, i);
            let usable = valid && v.is_finite();
            let b = ((f64::from(v) - 0.0) / 4.0).clamp(0.0, 1.0);
            let want = match (what, usable) {
                ("colour", false) => present::debug_invalid(frag),
                ("colour", true) => present::ramp_viridis(b),
                (_, true) => present::ramp_grey(b),
                (_, false) => continue,
            };
            assert!(
                same(got, want),
                "FTLE as {what}: sample {i}, ftle = {v}, draws {got:?}, not {want:?}"
            );
            if usable {
                placed.push(got);
            }
        }
        assert!(
            placed.len() >= 2 && placed.iter().any(|p| !same(*p, placed[0])),
            "FTLE as {what} renders no variation over the field"
        );
    }
}

#[test]
fn qa_field_either_role_ftle() {
    check_either_role(&gpu(), false);
}

negative_control!(
    qa_field_either_role_ftle,
    "a ramp bound to the slot its signature does not fit is refused: the one constraint is the output signature",
    expected = "did not compile and render",
    check_either_role(&gpu(), true)
);

/// Any field fills either slot by its default (gui_state_contract §4): each named field's default compiles in the
/// colour slot and in the brightness slot.
fn check_any_field(h: &GpuHarness, fields: &[&str]) {
    let s = scene("ftle").unwrap_or_else(|e| panic!("{e}"));
    for &name in fields {
        let e = entry(name);
        for slot in [Slot::Colour, Slot::Brightness] {
            let r = defaults::field_ramp(&e, slot, &resolve())
                .unwrap_or_else(|why| panic!("`{name}` in {slot:?}: {why}"));
            assert_eq!(
                r.slot(),
                slot,
                "`{name}`'s ramp is not for the {slot:?} slot"
            );
            let (kind, occupant) = r.occupant();
            stain(kind, occupant, None)
                .and_then(|st| render(h, &s, &st))
                .unwrap_or_else(|e| panic!("`{name}` in {slot:?} did not compile and render: {e}"));
        }
    }
}

#[test]
fn qa_field_either_role_any_field() {
    check_any_field(
        &gpu(),
        &[
            "ftle",
            "E_0",
            "energy_drift",
            "theta",
            "closure_min",
            "d_min",
            "t_end_step",
        ],
    );
}

negative_control!(
    qa_field_either_role_any_field,
    "a vector field has no scalar ramp in either slot",
    expected = "in Colour",
    check_any_field(&gpu(), &["r"])
);

/// The diverging fields with no floor read their `x₀` from the register (REQ-COL-065): `E_0`'s and `Lz_0`'s defaults
/// are the symlog at the register's `x₀`.
fn check_x0(table: &[(&str, f64)]) {
    for (name, x0) in table {
        let d =
            defaults::default_ramp(&entry(name), &resolve()).unwrap_or_else(|why| panic!("{why}"));
        assert_eq!(
            d.compaction,
            Compaction::Diverging { x0: *x0 },
            "`{name}`'s compaction is not the symlog at its register x₀"
        );
    }
}

#[test]
fn qa_default_ramps_diverging_x0_from_the_register() {
    let table: Vec<(&str, f64)> = DIVERGING_X0.iter().map(|(n, k)| (*n, k.number())).collect();
    assert_eq!(table.len(), 2, "E_0 and Lz_0");
    check_x0(&table);
}

negative_control!(
    qa_default_ramps_diverging_x0_from_the_register,
    "an x₀ the register does not hold is caught",
    expected = "not the symlog at its register x₀",
    check_x0(&[("E_0", 2.0)])
);

/// The ramp kinds of `ramp.wgsl` hold their ends: `lerp` c0 at 0 and c1 at 1; `diverging` its neutral at ½ exactly.
fn check_ramps(h: &GpuHarness, lib: &str) {
    let module = format!(
        "{lib}
@group(0) @binding(0) var<storage, read> q_t: array<u32>;
@group(0) @binding(1) var<storage, read_write> q_out: array<u32>;
@compute @workgroup_size(64)
fn q_main(@builtin(global_invocation_id) id: vec3<u32>) {{
    let i = id.x;
    if i >= arrayLength(&q_out) {{ return; }}
    let t = bitcast<f32>(q_t[i]);
    let c0 = vec3<f32>(0.25, 0.25, 0.25);
    let c1 = vec3<f32>(0.75, 0.75, 0.75);
    let mid = vec3<f32>(0.125, 0.125, 0.125);
    let l = ramp_lerp(t, c0, c1);
    let d = ramp_diverging(t, c0, mid, c1);
    q_out[i] = bitcast<u32>(select(l.x, d.x, i >= 2u));
}}
"
    );
    let ts = [0.0f32, 1.0, 0.5, 0.0, 1.0].map(f32::to_bits);
    let out: Vec<f32> = h
        .run_wgsl(&module, "q_main", &[&ts])
        .into_iter()
        .map(f32::from_bits)
        .collect();
    assert_eq!(out[..2], [0.25, 0.75], "lerp's ends");
    assert_eq!(out[2], 0.125, "the diverging ramp is not its neutral at ½");
    assert_eq!(out[3..], [0.25, 0.75], "the diverging ramp's ends");
}

#[test]
fn qa_ramp_kinds_hold_their_ends() {
    check_ramps(&gpu(), RAMP_WGSL);
}

negative_control!(
    qa_ramp_kinds_hold_their_ends,
    "a diverging ramp mixing end to end, past no neutral, is caught",
    expected = "not its neutral at ½",
    check_ramps(
        &gpu(),
        "fn ramp_lerp(t: f32, c0: vec3<f32>, c1: vec3<f32>) -> vec3<f32> { return mix(c0, c1, clamp(t, 0.0, 1.0)); }
fn ramp_diverging(t: f32, lo: vec3<f32>, mid: vec3<f32>, hi: vec3<f32>) -> vec3<f32> { return mix(lo, hi, clamp(t, 0.0, 1.0)); }"
    )
);
