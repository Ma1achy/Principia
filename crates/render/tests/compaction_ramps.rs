//! The field ramp's compactions, ramps and defaults (`render::colour::field_ramp`, `render::colour::defaults`;
//! colour_composition §1.2, §4.1; dd_colouring §3.6, §4 seam 5, §5 test 7; TASK-M7-05):
//! - REQ-COL-039: each compaction form is monotone on its domain; the symlog has `b(x) + b(−x) = 1` and `b(0) = ½`
//!   exactly; `log` styles the −1.0 sentinel instead of ramping it; the forms are R-411's, the debug views' `dbg_log`
//!   and the drift views' symlog; and `compaction.wgsl` computes them on the GPU (`compaction_forms_*`); the ramp kinds
//!   on the GPU agree with their CPU twins (`ramp_kinds_*`);
//! - REQ-COL-016: the default ramp registry returns the stated ramp and polarity per field role, "signed" read from the
//!   declared range as R-400 reads it (`default_ramps_*`);
//! - REQ-COL-032: a field's ledger scale changes, and the view re-styles while the sim buffer stays as it was, read
//!   back (`ledger_scale_restyle_*`): the measure of "the dispatch counter is unchanged" here, as the render loop has
//!   no integrate path whose frame-record counters could move;
//! - REQ-COL-024: FTLE bound as the colour, through a ramp, and as the brightness, each compiles and renders as its CPU
//!   twin has it (`field_either_role_*`).
//!
//! Each test registers its negative control (R-176).

use ledger::constants::{DIVERGING_X0, LOG_FLOORS};
use ledger::gen::numeric::{cyclic_period, End};
use ledger::schema::{Bound, Declaration, Entry, Range, Scale};
use render::assemble::{Kind, Node, Occupant, Stain, Tier};
use render::codegen::Slot;
use render::colour::combine::{combine, Combiner, LRange};
use render::colour::defaults::{self, DefaultRamp, Resolve, Role};
use render::colour::field_ramp::{
    Compaction, FieldRamp, Invalid, Lut, Map, Polarity, Ramp, Reads, ScalarField, Shown,
    COMPACTION_WGSL, RAMP_WGSL,
};
use render::compositor::LAYER_FORMAT;
use render::frame_record::RenderLoop;
use render::pipeline_cache::FrameInputs;
use render::present::{self, Rgb};
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{render_stain, row_scene, scene, Colouring, Scene};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

/// The ledger's validated entries.
fn entries() -> Vec<Entry> {
    let l = ledger::payload::ledger();
    ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"))
}

/// The entry `name`.
fn entry(name: &str) -> Entry {
    entries()
        .into_iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"))
}

/// The sim key the tests read with: `eps_E` and `eps_L` at 2⁻²⁰, a value of the tests' own.
fn sim_key(name: &str) -> Option<f64> {
    match name {
        "eps_E" | "eps_L" => Some(EPS),
        _ => None,
    }
}

const EPS: f64 = 1.0 / 1_048_576.0;

fn resolve() -> Resolve<'static> {
    Resolve { sim_key: &sim_key }
}

// ── REQ-COL-039: the forms ───────────────────────────────────────────────────────────────────────────────────────

/// `n + 1` points from `-hi` to `hi`, logarithmically spaced in magnitude from `lo`, with 0 and both signs.
fn symmetric(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let mut xs = vec![0.0];
    for k in 0..=n {
        let m = lo * (hi / lo).powf(k as f64 / n as f64);
        xs.push(m);
        xs.push(-m);
    }
    xs.sort_by(f64::total_cmp);
    xs
}

/// `n + 1` points from `lo` to `hi`, evenly spaced.
fn even(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..=n)
        .map(|k| lo + (hi - lo) * k as f64 / n as f64)
        .collect()
}

/// Each form's domain, as dd_colouring §3.6 places it, and whether it is strictly increasing there.
fn forms() -> Vec<(Compaction, Vec<f64>, bool)> {
    let period = cyclic_period();
    let mut log = vec![0.0];
    log.extend(symmetric(1e-9, 1e9, 400).into_iter().filter(|x| *x > 0.0));
    vec![
        // lin: increasing on [lo, hi], clamped beyond, so non-decreasing on ℝ.
        (
            Compaction::Lin { lo: -1.0, hi: 3.0 },
            even(-1.0, 3.0, 400),
            true,
        ),
        (
            Compaction::Lin { lo: -1.0, hi: 3.0 },
            even(-9.0, 9.0, 400),
            false,
        ),
        // log: its domain is x ≥ 0, x < 0 being the styled sentinel.
        (
            Compaction::Log {
                eps: 1.0 / 16_384.0,
            },
            log,
            true,
        ),
        // cyclic: one period, [0, period).
        (
            Compaction::Cyclic { period },
            even(0.0, period * (1.0 - 1e-9), 400),
            true,
        ),
        // diverging: ℝ, at two scales.
        (
            Compaction::Diverging { x0: EPS },
            symmetric(1e-12, 1e9, 400),
            true,
        ),
        (
            Compaction::Diverging { x0: 1.0 },
            symmetric(1e-9, 1e9, 400),
            true,
        ),
        // flag: {0, 1}.
        (Compaction::Flag, vec![0.0, 1.0], true),
    ]
}

/// Each form `place` gives is monotone on its domain, within [0, 1], strictly where `strict`.
fn check_monotone(place: impl Fn(&Compaction, f64) -> f64) {
    for (c, xs, strict) in forms() {
        let bs: Vec<f64> = xs.iter().map(|&x| place(&c, x)).collect();
        for (k, w) in bs.windows(2).enumerate() {
            let ok = if strict { w[1] > w[0] } else { w[1] >= w[0] };
            assert!(
                ok,
                "{c:?} is not monotone at {} → {}: {} then {}",
                xs[k],
                xs[k + 1],
                w[0],
                w[1]
            );
        }
        for (x, b) in xs.iter().zip(&bs) {
            assert!((0.0..=1.0).contains(b), "{c:?} places {x} at {b}");
        }
    }
}

#[test]
fn compaction_forms_monotone_on_their_domains() {
    check_monotone(Compaction::place);
}

negative_control!(
    compaction_forms_monotone_on_their_domains,
    "a form reversed, 1 − b, is caught",
    expected = "is not monotone",
    check_monotone(|c, x| 1.0 - c.place(x))
);

/// The ends and the boundaries of each form: `lin` 0 at `lo` and 1 at `hi`, clamped beyond; `log` 0 at 0 and below
/// its top; `cyclic` 0 at 0 and again at one period; `flag` 1 for any non-zero value.
fn check_ends(c: impl Fn(Compaction, f64) -> f64) {
    let lin = Compaction::Lin { lo: -1.0, hi: 3.0 };
    for (x, b) in [(-1.0, 0.0), (3.0, 1.0), (-2.0, 0.0), (4.0, 1.0), (1.0, 0.5)] {
        assert_eq!(c(lin, x), b, "lin places {x}");
    }
    let log = Compaction::Log { eps: 0.5 };
    assert_eq!(c(log, 0.0), 0.0, "log places 0 at 0");
    assert_eq!(c(log, -0.0), 0.0, "log places −0 at 0");
    assert_eq!(c(log, -1.0), 0.0, "log's x < 0 maps to 0");
    assert!(c(log, 1e300) < 1.0, "log stays below 1");
    let cyclic = Compaction::Cyclic { period: 4.0 };
    for (x, b) in [
        (0.0, 0.0),
        (4.0, 0.0),
        (1.0, 0.25),
        (-1.0, 0.75),
        (9.0, 0.25),
    ] {
        assert_eq!(c(cyclic, x), b, "cyclic places {x}");
    }
    for (x, b) in [
        (0.0, 0.0),
        (-0.0, 0.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1e-30, 1.0),
    ] {
        assert_eq!(c(Compaction::Flag, x), b, "flag places {x}");
    }
}

#[test]
fn compaction_forms_ends_and_boundaries() {
    check_ends(|c, x| c.place(x));
}

negative_control!(
    compaction_forms_ends_and_boundaries,
    "a cyclic form that truncates, Rust's `fract` (x − trunc x), places −1 at −0.25",
    expected = "cyclic places -1",
    check_ends(|c, x| match c {
        Compaction::Cyclic { period } => (x / period).fract(),
        _ => c.place(x),
    })
);

/// The symlog's `b(x) + b(−x)` is 1 and `b(0)` ½, both exactly, at each scale and over every magnitude.
fn check_symmetric(place: impl Fn(f64, f64) -> f64) {
    for x0 in [EPS, 1.0, 3.0] {
        assert_eq!(place(x0, 0.0), 0.5, "b(0) at x₀ = {x0}");
        assert_eq!(place(x0, -0.0), 0.5, "b(−0) at x₀ = {x0}");
        for x in symmetric(1e-30, 1e30, 600) {
            let (a, b) = (place(x0, x), place(x0, -x));
            assert_eq!(a + b, 1.0, "b({x}) + b({}) = {a} + {b} at x₀ = {x0}", -x);
        }
    }
}

#[test]
fn compaction_forms_symlog_symmetric_and_centred() {
    check_symmetric(|x0, x| Compaction::Diverging { x0 }.place(x));
}

negative_control!(
    compaction_forms_symlog_symmetric_and_centred,
    "a symlog centred off ½ (½ + ½·g without the sign) is caught",
    expected = "b(",
    check_symmetric(|x0, x| {
        let s = (1.0 + f64::abs(x) / x0).ln();
        0.5 + 0.5 * (1.0 - 1.0 / (1.0 + s)) * if x == 0.0 { 0.0 } else { 1.0 }
    })
);

/// The forms are R-411's: `log` is the debug views' `dbg_log` placement, `1 − 1/(1 + ln(1 + |x|/ε))`, with the
/// field's own ε; `diverging` the drift views' symlog, `s = sign(x)·ln(1 + |x|/x₀)` at `½ + ½·s/(1 + |s|)`, which
/// [`Compaction::place`] forms as `1 − u` below zero, the same value to the last bit or one rounding of 1.
fn check_r411(log: impl Fn(f64, f64) -> f64) {
    for eps in [1.0 / 16_384.0, 1.0 / 1_048_576.0, 0.5] {
        for x in even(0.0, 4.0, 200)
            .into_iter()
            .chain(symmetric(1e-9, 1e9, 100))
        {
            let x = x.abs();
            assert_eq!(
                log(eps, x),
                present::dbg_log_t(x, eps),
                "log at {x}, ε = {eps}: not the debug views' placement"
            );
        }
    }
    for x0 in [EPS, 1.0] {
        for x in symmetric(1e-9, 1e9, 200) {
            let s = x.signum() * (1.0 + x.abs() / x0).ln();
            let want = if x == 0.0 {
                0.5
            } else {
                0.5 + 0.5 * s / (1.0 + s.abs())
            };
            let got = Compaction::Diverging { x0 }.place(x);
            assert!(
                (got - want).abs() <= f64::EPSILON,
                "symlog at {x}, x₀ = {x0}: {got}, the drift views' {want}"
            );
        }
    }
}

#[test]
fn compaction_forms_are_r411s() {
    check_r411(|eps, x| Compaction::Log { eps }.place(x));
}

negative_control!(
    compaction_forms_are_r411s,
    "§3.6's old ranged log, (ln x − ln lo)/(ln hi − ln lo) over [ε, 10⁶], is not R-411's",
    expected = "not the debug views' placement",
    check_r411(|eps, x| ((x.max(eps).ln() - eps.ln()) / (1e6f64.ln() - eps.ln())).clamp(0.0, 1.0))
);

/// The `closure_min` ramp `log` over the given values: −1.0, the sentinel, then the values of [`LOG_VALUES`].
fn log_ramp(c: Compaction) -> FieldRamp {
    FieldRamp::new(
        ScalarField::payload("closure_min").unwrap_or_else(|e| panic!("{e}")),
        c,
        Map::ramp(Ramp::Lut(Lut::Viridis)),
    )
}

/// `closure_min`'s stored values in the sentinel scene: the −1.0 sentinel first, then 0, the floor, and above.
const LOG_VALUES: [f32; 8] = [-1.0, 0.0, 1.1920929e-7, 1e-5, 1e-3, 0.1, 1.0, 2.0];

/// `closure_min`'s floor, its register entry.
fn closure_floor() -> f64 {
    LOG_FLOORS
        .iter()
        .find(|(n, _)| *n == "closure_min")
        .map(|(_, k)| k.number())
        .expect("closure_min's floor")
}

/// The ramp styles the −1.0 sentinel, in its WGSL's invalid test and in its CPU twin, and on the GPU draws it as the
/// invalid colour, `#141418` (R-467), while every other value is on the ramp.
fn check_sentinel(h: &GpuHarness, ramp: &FieldRamp) {
    let wgsl = ramp.wgsl();
    assert!(
        wgsl.contains("|| v < 0.0) {"),
        "the ramp's invalid test does not style a negative value:\n{wgsl}"
    );
    assert_eq!(ramp.shown(-1.0, true), Shown::Invalid, "−1.0 is ramped");
    let mut s = row_scene(
        "log_sentinel",
        LOG_VALUES.len() as u32,
        Colouring::View("closure_min"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in LOG_VALUES.into_iter().enumerate() {
        s.set.sample(i as u32).closure_min(v);
    }
    let image = render_with(h, &s, ramp, None);
    check_image(&s, &image, ramp, "the log ramp");
}

#[test]
fn compaction_forms_log_styles_the_sentinel() {
    let ramp = log_ramp(Compaction::Log {
        eps: closure_floor(),
    });
    assert_eq!(
        ramp.shown(0.0, true),
        Shown::Ramp(0.0),
        "0 is the ramp's start"
    );
    check_sentinel(&gpu(), &ramp);
}

negative_control!(
    compaction_forms_log_styles_the_sentinel,
    "a lin compaction, which ramps −1.0 to its start, is caught",
    expected = "the ramp's invalid test does not style a negative value",
    check_sentinel(&gpu(), &log_ramp(Compaction::Lin { lo: 0.0, hi: 2.0 }))
);

// ── The forms and ramps on the GPU ───────────────────────────────────────────────────────────────────────────────

/// The entry the GPU checks append to the prelude, the compactions and the ramps: each pixel of a `CASE_WIDTH`-wide
/// target evaluates case `i`, `(op, x, k, 0)`, and returns its result's bits.
const ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> cases: array<vec4<u32>>;

const CASE_WIDTH: u32 = 64u;

fn rgb(c: vec3<f32>) -> vec4<u32> { return vec4<u32>(bitcast<vec3<u32>>(c), 0u); }

const C0: vec3<f32> = vec3<f32>(0.1, 0.2, 0.3);
const C1: vec3<f32> = vec3<f32>(0.9, 0.5, 0.0);
const C2: vec3<f32> = vec3<f32>(0.25, 0.75, 1.0);

@fragment
fn t_compaction(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let a = cases[u32(pos.y) * CASE_WIDTH + u32(pos.x)];
    let x = bitcast<f32>(a.y);
    let k = bitcast<f32>(a.z);
    switch a.x {
        case 1u: { return vec4<u32>(bitcast<u32>(compact_log(x, k)), 0u, 0u, 0u); }
        case 2u: { return vec4<u32>(bitcast<u32>(compact_cyclic(x, k)), 0u, 0u, 0u); }
        case 3u: {
            let b = compact_symlog(x, k);
            let c = compact_symlog(-x, k);
            return vec4<u32>(bitcast<u32>(b), bitcast<u32>(c), bitcast<u32>(b + c), 0u);
        }
        case 4u: { return vec4<u32>(bitcast<u32>(compact_flag(x)), 0u, 0u, 0u); }
        case 5u: { return rgb(ramp_lerp(x, C0, C1)); }
        case 6u: { return rgb(ramp_diverging(x, C0, C2, C1)); }
        case 7u: { return rgb(ramp_bands(x, k, 0.25, C1, C0)); }
        default: { return vec4<u32>(0xffffffffu); }
    }
}
";

const CASE_WIDTH: usize = 64;
const LOG: u32 = 1;
const CYCLIC: u32 = 2;
const SYMLOG: u32 = 3;
const FLAG: u32 = 4;
const LERP: u32 = 5;
const DIVERGING: u32 = 6;
const BANDS: u32 = 7;

/// The colours the entry's ramps take, as f32 values.
const C0: Rgb = [0.1f32 as f64, 0.2f32 as f64, 0.3f32 as f64];
const C1: Rgb = [0.9f32 as f64, 0.5, 0.0];
const C2: Rgb = [0.25, 0.75, 1.0];

/// Each case `(op, x, k)` evaluated on the GPU after the prelude, `compaction` and the ramps: its four result words.
fn run(h: &GpuHarness, compaction: &str, cases: &[(u32, f32, f32)]) -> Vec<[u32; 4]> {
    let module = format!(
        "{}\n{compaction}\n{RAMP_WGSL}\n{ENTRY}",
        ledger::gen::prelude::wgsl(Tier::FULL)
    );
    let words: Vec<u32> = cases
        .iter()
        .flat_map(|&(op, x, k)| [op, x.to_bits(), k.to_bits(), 0])
        .collect();
    let rows = cases.len().div_ceil(CASE_WIDTH);
    let kernel = h
        .fragment(
            &module,
            "t_compaction",
            &[&[], &[], &[BindingKind::Storage]],
            CASE_WIDTH as u32,
            rows as u32,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let pixels = kernel
        .draw(&[&[], &[], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    pixels[..cases.len()].to_vec()
}

/// The bound on a GPU form's difference from its f64 twin at f32: WGSL's `log` is within 2⁻²¹ absolute on
/// [0.5, 2] and 3 ULP relative elsewhere (WGSL § "Floating Point Accuracy"), and `b = 1 − 1/(1 + s)` has
/// `|db/ds| ≤ 1`, so the form's error is the log's, a few f32 roundings of `|x|/k` and of the division, each at most
/// 2⁻²⁴ relative: 2⁻¹⁸ covers them with room. Above `|x| = k·2⁶⁴`, where `s` is formed as `ln|x| − ln k`, each log's
/// 3 ULP is of a value up to ~89 in size, but there `s > 44` and `|db/ds| = 1/(1 + s)² < 2⁻¹¹`.
const FORM_BOUND: f64 = 1.0 / 262_144.0;

/// The forms on the GPU: within [`FORM_BOUND`] of their CPU twins; the symlog's `b(x) + b(−x)` exactly 1 and `b(0)`
/// exactly ½ in f32 too.
fn check_gpu_forms(h: &GpuHarness, compaction: &str) {
    let mut cases = Vec::new();
    for x in symmetric(1e-7, 1e7, 60) {
        let x = x as f32;
        cases.push((SYMLOG, x, 1.0));
        cases.push((SYMLOG, x, EPS as f32));
        if x >= 0.0 {
            cases.push((LOG, x, 1.0 / 16_384.0));
        }
        // `x/period` at f32 has a spacing of 2⁻²⁴ of its own size, so `fract` is placed to FORM_BOUND only while
        // the quotient stays within 2⁶, some sixty turns: beyond, the f32 quotient, not the form, decides.
        if x.abs() <= 400.0 {
            cases.push((CYCLIC, x, cyclic_period() as f32));
        }
        cases.push((FLAG, x, 0.0));
    }
    let got = run(h, compaction, &cases);
    for (&(op, x, k), p) in cases.iter().zip(&got) {
        let b = f64::from(f32::from_bits(p[0]));
        let (x, k) = (f64::from(x), f64::from(k));
        let want = match op {
            LOG => Compaction::Log { eps: k }.place(x),
            CYCLIC => Compaction::Cyclic { period: k }.place(x),
            SYMLOG => {
                let (lo, sum) = (f32::from_bits(p[1]), f32::from_bits(p[2]));
                assert_eq!(sum, 1.0, "on the GPU, b({x}) + b({}) = {b} + {lo}", -x);
                if x == 0.0 {
                    assert_eq!(b, 0.5, "on the GPU, b(0) = {b}");
                }
                Compaction::Diverging { x0: k }.place(x)
            }
            _ => Compaction::Flag.place(x),
        };
        // `cyclic` wraps at the period: a value one rounding from it may land at either end.
        let d = (b - want).abs();
        let d = if op == CYCLIC { d.min(1.0 - d) } else { d };
        assert!(
            d <= FORM_BOUND,
            "op {op} at x = {x}, k = {k}: the GPU's {b}, the twin's {want}"
        );
    }
}

#[test]
fn compaction_forms_on_the_gpu_match_the_twins() {
    check_gpu_forms(&gpu(), COMPACTION_WGSL);
}

negative_control!(
    compaction_forms_on_the_gpu_match_the_twins,
    "a GPU symlog formed as ½ + ½·sign(x)·g, not 1 − u below zero, does not sum to 1 exactly",
    expected = "on the GPU, b(",
    check_gpu_forms(
        &gpu(),
        &COMPACTION_WGSL.replace(
            "return select(1.0 - u, u, x >= 0.0);",
            "return 0.5 + sign(x) * (u - 0.5) + 1e-7;"
        )
    )
);

/// `log` and the symlog at the f32 extremes of `|x|/k` (the forms take any finite value): `|x|` near `f32::MAX` over a
/// scale of 2⁻²⁰, where `|x|/k` itself would overflow to +∞, and `|x|` at `k·2⁶⁴` and one f32 step either side of
/// it, where `compaction.wgsl` switches from `ln(1 + |x|/k)` to `ln|x| − ln k`: each GPU value finite, below 1 and
/// within [`FORM_BOUND`] of its twin, the symlog's sum still exactly 1.
fn check_gpu_forms_extremes(h: &GpuHarness, compaction: &str) {
    let mut cases = Vec::new();
    for k in [1.0f32 / 1_048_576.0, 1.0 / 16_384.0, 1.0] {
        let edge = k * 18_446_744_073_709_551_616.0;
        let at = [edge.next_down(), edge, edge.next_up()];
        for x in at.into_iter().chain([1e20, 4e32, 1e36, f32::MAX]) {
            cases.push((LOG, x, k));
            cases.push((SYMLOG, x, k));
            cases.push((SYMLOG, -x, k));
        }
    }
    let got = run(h, compaction, &cases);
    for (&(op, x, k), p) in cases.iter().zip(&got) {
        let b = f32::from_bits(p[0]);
        let (xd, kd) = (f64::from(x), f64::from(k));
        let want = if op == LOG {
            Compaction::Log { eps: kd }.place(xd)
        } else {
            let sum = f32::from_bits(p[2]);
            assert_eq!(sum, 1.0, "on the GPU, b({x}) + b({}) = {sum}", -x);
            Compaction::Diverging { x0: kd }.place(xd)
        };
        assert!(
            b.is_finite() && (0.0..1.0).contains(&b),
            "op {op} at x = {x}, k = {k}: the GPU's {b} is not in [0, 1)"
        );
        assert!(
            (f64::from(b) - want).abs() <= FORM_BOUND,
            "op {op} at x = {x}, k = {k}: the GPU's {b}, the twin's {want}"
        );
    }
}

#[test]
fn compaction_forms_on_the_gpu_hold_at_the_f32_extremes() {
    check_gpu_forms_extremes(&gpu(), COMPACTION_WGSL);
}

negative_control!(
    compaction_forms_on_the_gpu_hold_at_the_f32_extremes,
    "a GPU log formed as ln(1 + |x|/k), whose quotient overflows to +∞ for a huge x, is caught",
    expected = "op 1 at x = ",
    check_gpu_forms_extremes(
        &gpu(),
        &COMPACTION_WGSL.replace(
            "return select(below, above, huge);",
            "return log(1.0 + a / s);"
        )
    )
);

/// The ramp kinds on the GPU agree with their CPU twins (`Ramp::colour`) within f32 rounding of a mix, including at
/// the ends, the diverging ramp's neutral at ½ and the bands' edges.
fn check_gpu_ramps(h: &GpuHarness, ramp: &str) {
    let mut cases = Vec::new();
    for t in even(-0.25, 1.25, 120)
        .into_iter()
        .chain([0.0, 0.5, 1.0, 0.24, 0.26, 0.49])
    {
        let t = t as f32;
        cases.push((LERP, t, 0.0));
        cases.push((DIVERGING, t, 0.0));
        cases.push((BANDS, t, 4.0));
    }
    let module_cases = run_ramps(h, ramp, &cases);
    for (&(op, t, k), p) in cases.iter().zip(&module_cases) {
        let got = [0, 1, 2].map(|c| f64::from(f32::from_bits(p[c])));
        let r = match op {
            LERP => Ramp::Lerp { c0: C0, c1: C1 },
            DIVERGING => Ramp::Diverging {
                lo: C0,
                mid: C2,
                hi: C1,
            },
            _ => Ramp::Bands {
                n: f64::from(k),
                width: 0.25,
                line: C1,
                base: C0,
            },
        };
        let want = r.colour(f64::from(t));
        for c in 0..3 {
            assert!(
                (got[c] - want[c]).abs() <= 1e-6,
                "{} at t = {t}: the GPU's {got:?}, the twin's {want:?}",
                r.name()
            );
        }
    }
    let neutral = run_ramps(h, ramp, &[(DIVERGING, 0.5, 0.0)])[0];
    assert_eq!(
        [0, 1, 2].map(|c| f64::from(f32::from_bits(neutral[c]))),
        C2,
        "the diverging ramp is not its neutral at ½"
    );
}

/// [`run`] with the ramps' WGSL `ramp` in place of `ramp.wgsl`.
fn run_ramps(h: &GpuHarness, ramp: &str, cases: &[(u32, f32, f32)]) -> Vec<[u32; 4]> {
    let module = format!(
        "{}\n{COMPACTION_WGSL}\n{ramp}\n{ENTRY}",
        ledger::gen::prelude::wgsl(Tier::FULL)
    );
    let words: Vec<u32> = cases
        .iter()
        .flat_map(|&(op, x, k)| [op, x.to_bits(), k.to_bits(), 0])
        .collect();
    let rows = cases.len().div_ceil(CASE_WIDTH);
    let kernel = h
        .fragment(
            &module,
            "t_compaction",
            &[&[], &[], &[BindingKind::Storage]],
            CASE_WIDTH as u32,
            rows as u32,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let pixels = kernel
        .draw(&[&[], &[], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    pixels[..cases.len()].to_vec()
}

#[test]
fn ramp_kinds_on_the_gpu_match_the_twins() {
    check_gpu_ramps(&gpu(), RAMP_WGSL);
}

negative_control!(
    ramp_kinds_on_the_gpu_match_the_twins,
    "a diverging ramp whose halves meet at ⅓, not ½, is caught",
    expected = "diverging at t",
    check_gpu_ramps(
        &gpu(),
        &RAMP_WGSL
            .replace("mix(mid, hi, 2.0 * u - 1.0)", "mix(mid, hi, 1.5 * u - 0.5)")
            .replace("mix(lo, mid, 2.0 * u)", "mix(lo, mid, 3.0 * u)")
            .replace("u < 0.5", "u < 0.33333334")
    )
);

// ── REQ-COL-016: the default ramp per field role ─────────────────────────────────────────────────────────────────

/// A field's expected default: its role, ramp, polarity and compaction.
type Want = (Role, Lut, Polarity, Compaction);

/// The run-time values the registry's `lin` ranges are placed at in [`check_placed`]: the measured `(−3, 5)` and a
/// horizon of 1000 steps.
const READS: Reads = Reads {
    u_range: [-3.0, 5.0],
    horizon_steps: 1000,
};

/// A `lin` range read at run time, `lo` and `hi` its ends, `symmetric` for a signed field.
fn read(lo: End, hi: End, symmetric: bool) -> Compaction {
    Compaction::LinRead { lo, hi, symmetric }
}

/// `e` with its name, scale and range replaced: a test's ledger entry.
fn edited(name: &'static str, scale: Scale, lo: Bound, hi: Bound) -> Entry {
    let mut e = entry("ftle");
    e.name = name;
    e.scale = scale;
    e.range = Range { lo, hi };
    e.declaration = Some(Declaration::Cited("a test's range"));
    e
}

/// The defaults the registry must give, by field.
fn wants() -> Vec<(Entry, Want)> {
    use Bound::{Closed, Unbounded};
    let floor = |n: &str| {
        LOG_FLOORS
            .iter()
            .find(|(f, _)| *f == n)
            .map(|(_, k)| k.number())
            .expect("a floor")
    };
    use End::{Fixed, Horizon, Measured};
    let sym = read(Measured, Measured, true);
    let x0 = |n: &str| {
        DIVERGING_X0
            .iter()
            .find(|(f, _)| *f == n)
            .map(|(_, k)| k.number())
            .expect("x₀")
    };
    let tau = cyclic_period();
    let mut out = vec![
        // magnitude / diagnostic: the greyscale, FTLE and diffusion white = high, t_end_step white = low.
        (
            entry("ftle"),
            (
                Role::Magnitude(Polarity::High),
                Lut::Grey,
                Polarity::High,
                read(Measured, Measured, false),
            ),
        ),
        (
            entry("diffusion"),
            (
                Role::Magnitude(Polarity::High),
                Lut::Grey,
                Polarity::High,
                read(Measured, Measured, false),
            ),
        ),
        (
            edited("ensemble_spread", Scale::Lin, Closed(0.0), Unbounded),
            (
                Role::Magnitude(Polarity::High),
                Lut::Grey,
                Polarity::High,
                read(Fixed(0.0), Measured, false),
            ),
        ),
        (
            entry("t_end_step"),
            (
                Role::Magnitude(Polarity::Low),
                Lut::Grey,
                Polarity::Low,
                read(Fixed(0.0), Horizon, false),
            ),
        ),
        // signed: diverging through the neutral; the drifts' x₀ their sim-key floors, E_0's and Lz_0's their own.
        (
            entry("energy_drift"),
            (
                Role::Signed,
                Lut::Coolwarm,
                Polarity::High,
                Compaction::Diverging { x0: EPS },
            ),
        ),
        (
            entry("Lz_drift"),
            (
                Role::Signed,
                Lut::Coolwarm,
                Polarity::High,
                Compaction::Diverging { x0: EPS },
            ),
        ),
        (
            entry("E_0"),
            (
                Role::Signed,
                Lut::Coolwarm,
                Polarity::High,
                Compaction::Diverging { x0: x0("E_0") },
            ),
        ),
        (
            entry("Lz_0"),
            (
                Role::Signed,
                Lut::Coolwarm,
                Polarity::High,
                Compaction::Diverging { x0: x0("Lz_0") },
            ),
        ),
        (
            entry("S"),
            (Role::Signed, Lut::Coolwarm, Polarity::High, sym),
        ),
        (
            entry("C_ty"),
            (Role::Signed, Lut::Coolwarm, Polarity::High, sym),
        ),
        // angles: cyclic, whatever the range (R-423).
        (
            entry("theta"),
            (
                Role::Angle,
                Lut::Twilight,
                Polarity::High,
                Compaction::Cyclic { period: tau },
            ),
        ),
        (
            entry("rho_angle"),
            (
                Role::Angle,
                Lut::Twilight,
                Polarity::High,
                Compaction::Cyclic { period: tau },
            ),
        ),
        // one-signed: sequential, V_0 included (R-403).
        (
            entry("K_0"),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                read(Fixed(0.0), Measured, false),
            ),
        ),
        (
            entry("V_0"),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                read(Measured, Fixed(0.0), false),
            ),
        ),
        (
            entry("q_mass"),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                Compaction::Lin { lo: 0.0, hi: 1.0 },
            ),
        ),
        // declared [0, 4] from `y`'s definition (REQ-PAY-098; R-426), no longer provisional: one-signed.
        (
            entry("mean_y"),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                Compaction::Lin { lo: 0.0, hi: 4.0 },
            ),
        ),
        (
            entry("total_substeps"),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                Compaction::Lin {
                    lo: 0.0,
                    hi: 4_294_967_295.0,
                },
            ),
        ),
        (
            entry("saturated"),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                Compaction::Flag,
            ),
        ),
    ];
    for f in [
        "d_min",
        "dE_max",
        "dLz_max",
        "closure_min",
        "rho_ratio",
        "r_min_pair_0",
    ] {
        out.push((
            entry(f),
            (
                Role::Sequential,
                Lut::Viridis,
                Polarity::High,
                Compaction::Log { eps: floor(f) },
            ),
        ));
    }
    // An infinite declared end is no end: the measured one stands in, read at run time, as for an unbounded end.
    out.push((
        edited(
            "infinite_hi",
            Scale::Lin,
            Closed(0.0),
            Bound::Open(f64::INFINITY),
        ),
        (
            Role::Sequential,
            Lut::Viridis,
            Polarity::High,
            read(Fixed(0.0), Measured, false),
        ),
    ));
    out.push((
        edited(
            "infinite_ends",
            Scale::Lin,
            Closed(f64::NEG_INFINITY),
            Closed(f64::INFINITY),
        ),
        (Role::Signed, Lut::Coolwarm, Polarity::High, sym),
    ));
    out
}

/// The registry `registry` gives each field its stated default.
fn check_defaults(registry: impl Fn(&Entry, &Resolve) -> Result<DefaultRamp, String>) {
    for (e, (role, lut, polarity, compaction)) in wants() {
        let d = registry(&e, &resolve()).unwrap_or_else(|why| panic!("`{}`: {why}", e.name));
        let got = (d.role, d.ramp, d.polarity, d.compaction);
        let want = (role, Ramp::Lut(lut), polarity, compaction);
        assert!(
            got == want,
            "`{}`'s default is {got:?}, not {want:?}",
            e.name
        );
    }
}

#[test]
fn default_ramps_follow_field_role() {
    check_defaults(defaults::default_ramp);
}

/// The fields whose `lin` range the registry leaves open, each with the fixed range it is placed on at [`READS`]: a
/// measured end `u_range`'s, a step index's top the horizon, a signed field's `[−M, M]`.
const PLACED: [(&str, [f64; 2]); 6] = [
    ("ftle", [-3.0, 5.0]),
    ("diffusion", [-3.0, 5.0]),
    ("S", [-5.0, 5.0]),
    ("K_0", [0.0, 5.0]),
    ("V_0", [-3.0, 0.0]),
    ("t_end_step", [0.0, 1000.0]),
];

/// Each of [`PLACED`]'s defaults, by `registry`, reads its open ends at run time: its WGSL reads `uniforms.u_range` or
/// `ctx.params.horizon_steps` and its header declares `u_range` where it measures, no literal of those values (lowering
/// contract Part 3: no gesture reaches a compiler); its twin places each value as the fixed range would at [`READS`],
/// and, at other reads, where those reads put it.
fn check_placed(registry: impl Fn(&Entry) -> Compaction) {
    let other = Reads {
        u_range: [-1.0, 1.0],
        horizon_steps: 10,
    };
    for (name, [lo, hi]) in PLACED {
        let c = registry(&entry(name));
        let text = c.wgsl("v");
        let measured = text.contains("uniforms.u_range");
        assert!(
            measured || text.contains("ctx.params.horizon_steps"),
            "`{name}`'s range is not read at run time: {text}"
        );
        assert_eq!(
            c.uniforms().contains("@uniform u_range: vec2<f32>"),
            measured,
            "`{name}`'s header: {}",
            c.uniforms()
        );
        let fixed = Compaction::Lin { lo, hi };
        for x in [-4.0, -1.0, 0.0, 0.5, 2.0, 7.0, 600.0] {
            assert_eq!(
                c.place_at(x, READS),
                fixed.place(x),
                "`{name}` at {x} with the reads (−3, 5), 1000"
            );
        }
        assert_ne!(
            (c.place_at(-2.0, other), c.place_at(0.5, other)),
            (fixed.place(-2.0), fixed.place(0.5)),
            "`{name}` is placed the same at other reads"
        );
    }
}

#[test]
fn default_ramps_read_their_open_ends_at_run_time() {
    check_placed(|e| {
        defaults::default_ramp(e, &resolve())
            .unwrap_or_else(|why| panic!("{why}"))
            .compaction
    });
}

negative_control!(
    default_ramps_read_their_open_ends_at_run_time,
    "a registry that bakes the measured range into the source is caught",
    expected = "is not read at run time",
    check_placed(|e| {
        let c = defaults::default_ramp(e, &resolve())
            .unwrap_or_else(|why| panic!("{why}"))
            .compaction;
        let [lo, hi] = PLACED
            .iter()
            .find(|(n, _)| *n == e.name)
            .map(|(_, r)| *r)
            .unwrap_or([0.0, 1.0]);
        match c {
            Compaction::LinRead { .. } => Compaction::Lin { lo, hi },
            c => c,
        }
    })
);

negative_control!(
    default_ramps_follow_field_role,
    "a registry that reads every field's role from its range alone puts FTLE on the diverging ramp",
    expected = "`ftle`'s default is",
    check_defaults(|e, r| {
        let mut e = e.clone();
        if e.name == "ftle" {
            e.name = "ftle_unnamed";
        }
        defaults::default_ramp(&e, r)
    })
);

/// "Signed" is R-400's: a declared range that spans zero, `lo < 0 < hi`, either end possibly unbounded; an end at 0,
/// open or closed, does not span it; a gap is no range and is refused.
fn check_signed(role: impl Fn(&Entry) -> Role) {
    use Bound::{Closed, Open, Unbounded};
    for (lo, hi, want) in [
        (Closed(-1.0), Closed(1.0), Role::Signed),
        (Unbounded, Unbounded, Role::Signed),
        (Unbounded, Closed(1e-30), Role::Signed),
        (Open(-1e-30), Unbounded, Role::Signed),
        (Closed(0.0), Unbounded, Role::Sequential),
        (Open(0.0), Closed(1.0), Role::Sequential),
        (Unbounded, Closed(0.0), Role::Sequential),
        (Unbounded, Open(0.0), Role::Sequential),
        (Closed(1.0), Closed(2.0), Role::Sequential),
    ] {
        let e = edited("signed_probe", Scale::Lin, lo, hi);
        assert_eq!(role(&e), want, "the range {}", e.range);
    }
    let mut gap = edited("gap_probe", Scale::Lin, Unbounded, Unbounded);
    gap.declaration = None;
    assert_eq!(role(&gap), Role::Sequential, "a gap does not span zero");
    let why = defaults::default_ramp(&gap, &resolve()).expect_err("a gap is refused");
    assert!(why.contains("`gap_probe`"), "{why}");
}

#[test]
fn default_ramps_signed_is_the_declared_range() {
    check_signed(defaults::role);
}

negative_control!(
    default_ramps_signed_is_the_declared_range,
    "a role that reads a range ending at 0 as signed is caught",
    expected = "the range",
    check_signed(|e| {
        let touches = |b: Bound| matches!(b, Bound::Closed(v) | Bound::Open(v) if v == 0.0);
        if touches(e.range.lo) || touches(e.range.hi) {
            Role::Signed
        } else {
            defaults::role(e)
        }
    })
);

/// A signed field whose declared ends are both fixed is placed on `[−M, M]`, `M` the larger magnitude, so zero sits at
/// the ramp's neutral: `[−2, 5]` on `[−5, 5]`, `[−5, 2]` on `[−5, 5]`; a sequential one keeps its own ends.
fn check_signed_fixed(compaction: impl Fn(&Entry) -> Compaction) {
    use Bound::Closed;
    for (lo, hi, want) in [
        (-2.0, 5.0, Compaction::Lin { lo: -5.0, hi: 5.0 }),
        (-5.0, 2.0, Compaction::Lin { lo: -5.0, hi: 5.0 }),
        (1.0, 2.0, Compaction::Lin { lo: 1.0, hi: 2.0 }),
    ] {
        let e = edited("fixed_probe", Scale::Lin, Closed(lo), Closed(hi));
        assert_eq!(compaction(&e), want, "the range [{lo}, {hi}]");
    }
}

#[test]
fn default_ramps_signed_fixed_range_is_symmetric() {
    check_signed_fixed(|e| {
        defaults::default_ramp(e, &resolve())
            .unwrap_or_else(|why| panic!("{why}"))
            .compaction
    });
}

negative_control!(
    default_ramps_signed_fixed_range_is_symmetric,
    "a signed range placed on its declared ends, zero off the neutral, is caught",
    expected = "the range [-2, 5]",
    check_signed_fixed(|e| match (e.range.lo, e.range.hi) {
        (Bound::Closed(lo), Bound::Closed(hi)) => Compaction::Lin { lo, hi },
        _ => unreachable!("both ends are closed"),
    })
);

/// The registry refuses a field it can't place, naming it: a categorical field, a vector, a `diverging` field whose
/// sim-key floor the sim key does not give or with no `x₀`, a `log` field with no floor, and an inverted declared
/// `lin` range.
fn check_refusals(sim: &'static dyn Fn(&str) -> Option<f64>) {
    let r = Resolve { sim_key: sim };
    let refused = |e: &Entry, r: &Resolve, why: &str| {
        let got = defaults::default_ramp(e, r)
            .err()
            .unwrap_or_else(|| panic!("`{}` was given a default", e.name));
        assert!(
            got.contains(&format!("`{}`", e.name)) && got.contains(why),
            "`{}` refused for {got}, not {why}",
            e.name
        );
    };
    refused(&entry("state"), &r, "categorical");
    refused(&entry("n"), &r, "vector");
    let mut drift = entry("energy_drift");
    drift.floor = Some("eps_unknown");
    refused(&drift, &r, "sim key does not give");
    refused(
        &edited(
            "no_x0",
            Scale::Diverging,
            Bound::Unbounded,
            Bound::Unbounded,
        ),
        &r,
        "no x₀ of its own",
    );
    refused(
        &edited("no_floor", Scale::Log, Bound::Closed(0.0), Bound::Unbounded),
        &r,
        "no log floor of its own",
    );
    refused(
        &edited(
            "inverted",
            Scale::Lin,
            Bound::Closed(2.0),
            Bound::Closed(1.0),
        ),
        &r,
        "is inverted",
    );
    let mut zero = entry("energy_drift");
    zero.floor = Some("eps_zero");
    refused(&zero, &r, "not a finite positive value");
    let mut inf = entry("energy_drift");
    inf.floor = Some("eps_inf");
    refused(&inf, &r, "not a finite positive value");
    // A degenerate declared range is a range: `range_norm` reads it as 0 (R-72).
    let one = edited(
        "degenerate",
        Scale::Lin,
        Bound::Closed(2.0),
        Bound::Closed(2.0),
    );
    let d = defaults::default_ramp(&one, &r).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(d.compaction, Compaction::Lin { lo: 2.0, hi: 2.0 });
}

/// The sim key of [`check_refusals`]: `eps_zero` is 0, `eps_inf` +∞.
fn refusing_key(name: &str) -> Option<f64> {
    match name {
        "eps_zero" => Some(0.0),
        "eps_inf" => Some(f64::INFINITY),
        _ => None,
    }
}

/// A sim key that gives every parameter, 1: the refusals' control.
#[cfg(feature = "controls")]
fn knows_everything(_: &str) -> Option<f64> {
    Some(1.0)
}

#[test]
fn default_ramps_refuse_what_they_cannot_place() {
    check_refusals(&refusing_key);
}

negative_control!(
    default_ramps_refuse_what_they_cannot_place,
    "a sim key that gives every parameter places the drift whose floor it should not know",
    expected = "`energy_drift` was given a default",
    check_refusals(&knows_everything)
);

// ── Rendering a field ramp ───────────────────────────────────────────────────────────────────────────────────────

/// The stain of `ramp` in its slot: the zero source → the ramp → Replace-L → its style post, if a brightness ramp
/// and `post`, → `OUT`.
fn ramp_stain(ramp: &FieldRamp, post: bool) -> Stain {
    let (kind, occupant) = ramp.occupant();
    let post = (post && ramp.slot() == Slot::Brightness).then(|| ramp.style_post());
    custom_stain(kind, occupant, post)
}

/// The run-time values `s` is drawn with: the measured range of its valid, finite values, written to `u_range` as
/// the measurement does, and its context's horizon.
fn reads(s: &Scene) -> Reads {
    Reads {
        u_range: measured(s),
        horizon_steps: s.context.horizon_steps,
    }
}

/// `ramp` rendered over `s`'s payload with its style post where `post` is `Some(true)` (a colour ramp has none),
/// `u_range` written with [`reads`]' measured range.
fn render_with(h: &GpuHarness, s: &Scene, ramp: &FieldRamp, post: Option<bool>) -> Vec<[f32; 4]> {
    let u_range = reads(s).u_range.to_vec();
    render_stain(
        h.device(),
        h.queue(),
        &ramp_stain(ramp, post.unwrap_or(true)),
        &s.set,
        &s.context,
        &|_, name| (name == "u_range").then(|| u_range.clone()),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The colour `ramp` shows for sample `i` of `s` at the pixel `(x, y)`: a colour ramp's twin, or, for a brightness
/// ramp, Replace-L on white at its lightness (colour_composition §4.1's None-colour row), its styles its post's.
fn want_at(s: &Scene, i: u32, ramp: &FieldRamp, (x, y): (u32, u32)) -> Rgb {
    let (v, gate) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
    let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
    ramp.colour_at_reads(v, gate, frag, Invalid::Default, reads(s))
        .unwrap_or_else(|| {
            let Shown::Ramp(b) = ramp.shown_at(v, gate, reads(s)) else {
                unreachable!("a placed sample")
            };
            combine(Combiner::ReplaceL(LRange::DEFAULT), None, Some(b))
        })
}

/// The tolerance of a render against its CPU twin, per linear channel: the LUTs' and OKLab's f32 evaluation against
/// f64, a few 10⁻⁵ (render/tests/prelude.rs bounds each), and the field's own f32 read; 10⁻³ is far below the 1/255
/// step that would show.
const RENDER_TOL: f64 = 1e-3;

/// Each pixel of `image` is `ramp`'s twin for its sample, a styled sample's style included.
fn check_image(s: &Scene, image: &[[f32; 4]], ramp: &FieldRamp, what: &str) {
    let (w, _) = s.size();
    let n = s.set.grid().sample_count();
    let mut checked = 0;
    for i in 0..n {
        for (x, y) in s.pixels(i) {
            let got = image[(y * w + x) as usize];
            let want = want_at(s, i, ramp, (x, y));
            for c in 0..3 {
                assert!(
                    (f64::from(got[c]) - want[c]).abs() <= RENDER_TOL,
                    "{what}: sample {i} ({:?}) at ({x}, {y}) is {got:?}, its twin {want:?}",
                    s.value(i),
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "{what}: no pixel checked");
}

// ── REQ-COL-024: FTLE in either role ─────────────────────────────────────────────────────────────────────────────

/// The FTLE scene: an unstepped sample (NaN), a failed one and six on the field (golden_scene's `ftle`).
fn ftle_scene() -> Scene {
    scene("ftle").unwrap_or_else(|e| panic!("{e}"))
}

/// The measured range of `s`'s valid, finite FTLE values.
fn measured(s: &Scene) -> [f64; 2] {
    let n = s.set.grid().sample_count();
    (0..n)
        .filter_map(|i| s.value(i).ok())
        .filter(|(v, g)| *g && v.is_finite())
        .fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], (v, _)| {
            [lo.min(f64::from(v)), hi.max(f64::from(v))]
        })
}

/// FTLE bound as the colour, by its default (the greyscale) and through viridis, and as the brightness, with its
/// style post when `post`: each assembles, compiles, renders, and is its CPU twin at every pixel, the invalid samples
/// `#141418` in both roles (R-467).
fn check_either_role(h: &GpuHarness, post: bool) {
    let s = ftle_scene();
    let e = entry("ftle");
    let r = resolve();
    let grey = defaults::field_ramp(&e, Slot::Colour, &r).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(
        grey.map,
        Map::ramp(Ramp::Lut(Lut::Grey)),
        "FTLE's default colour is the greyscale"
    );
    let viridis = FieldRamp::new(
        grey.field.clone(),
        grey.compaction,
        Map::ramp(Ramp::Lut(Lut::Viridis)),
    );
    let bright =
        defaults::field_ramp(&e, Slot::Brightness, &r).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(bright.map, Map::Lightness);
    assert!(
        bright.wgsl().contains("fn brightness(ctx: Ctx) -> f32 {"),
        "{}",
        bright.wgsl()
    );
    let invalid = (0..s.set.grid().sample_count())
        .filter(|&i| {
            bright.shown(
                s.value(i).expect("a value").0,
                s.value(i).expect("a value").1,
            ) == Shown::Invalid
        })
        .count();
    assert!(invalid > 0, "the scene has no invalid FTLE sample");
    for (ramp, what) in [
        (&grey, "FTLE as colour, grey"),
        (&viridis, "FTLE as colour, viridis"),
    ] {
        check_image(&s, &render_with(h, &s, ramp, None), ramp, what);
    }
    check_image(
        &s,
        &render_with(h, &s, &bright, Some(post)),
        &bright,
        "FTLE as brightness",
    );
}

#[test]
fn field_either_role_ftle_as_colour_and_brightness() {
    check_either_role(&gpu(), true);
}

negative_control!(
    field_either_role_ftle_as_colour_and_brightness,
    "the brightness without its style post draws an invalid FTLE as black, not #141418",
    expected = "FTLE as brightness: sample 0",
    check_either_role(&gpu(), false)
);

// ── REQ-COL-032: a ledger scale change re-styles the view with zero recompute (seam 5) ───────────────────────────

/// The re-style scene's `E_0` values, one sample per pixel, a NaN among them.
const E0: [f32; 8] = [-4.0, -1.0, -0.25, 0.0, 0.25, 1.0, 4.0, f32::NAN];

/// The layer's width, one pixel per sample.
const W: u32 = E0.len() as u32;

/// The frame's uniforms.
const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.0,
    masses: [1.0, 1.0, 1.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 16,
    horizon_steps: 1000,
};

/// The render loop's rig: the loop, its target, and the sim buffers it reads.
struct Rig {
    h: GpuHarness,
    rl: RenderLoop,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    simstate: wgpu::Buffer,
    bytes: Vec<u8>,
}

fn rig(s: &Scene) -> Rig {
    use wgpu::util::DeviceExt;
    let h = gpu();
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, W, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d {
            width: W,
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
    let buffer = |label: &str, contents: &[u8]| {
        h.device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
    };
    let simstate = buffer("simstate", &bytes.simstate);
    let word = buffer("word", &bytes.word);
    rl.set_sim(simstate.clone(), Some(word.clone()));
    Rig {
        h,
        rl,
        target,
        view,
        simstate,
        bytes: bytes.simstate,
    }
}

/// The colour node's key in [`custom_stain`]'s stain: the source is 0, the colour 1.
const COLOUR_KEY: u64 = 1;

impl Rig {
    /// `ramp`'s stain requested and compiled, its `u_range` written with `u_range` where it declares one, then one
    /// frame: its pixels, 8-bit sRGB.
    fn frame(&mut self, ramp: &FieldRamp, u_range: [f64; 2]) -> Vec<[u8; 4]> {
        let stain = ramp_stain(ramp, true);
        let keys: Vec<u64> = (0..stain.nodes().len() as u64).collect();
        self.rl
            .cache()
            .request(&stain, &keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
        self.rl.cache().wait();
        assert!(
            self.rl.cache().errors().is_empty(),
            "{:?}",
            self.rl.cache().errors()
        );
        self.again(ramp, u_range)
    }

    /// One more frame of the current stain, `u_range` written where `ramp` declares it, as the measurement writes it
    /// after a pan or zoom: a uniform write, no request and no compile.
    fn again(&mut self, ramp: &FieldRamp, u_range: [f64; 2]) -> Vec<[u8; 4]> {
        if ramp.compaction.uniforms().contains("u_range") {
            self.rl
                .set_param(COLOUR_KEY, "u_range", &u_range)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        self.rl.frame(&INPUTS, &self.view);
        read_texture(&self.h, &self.target)
    }
}

/// `buffer`'s bytes, read back.
fn read_buffer(h: &GpuHarness, buffer: &wgpu::Buffer) -> Vec<u8> {
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: buffer.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = h.device().create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, buffer.size());
    h.queue().submit([encoder.finish()]);
    map(h, &staging)
}

fn map(h: &GpuHarness, staging: &wgpu::Buffer) -> Vec<u8> {
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let out = slice.get_mapped_range().expect("range").to_vec();
    staging.unmap();
    out
}

/// The target's pixels, 8-bit sRGB.
fn read_texture(h: &GpuHarness, texture: &wgpu::Texture) -> Vec<[u8; 4]> {
    let row = 256u32;
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("texture readback"),
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
            width: W,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    h.queue().submit([encoder.finish()]);
    let bytes = map(h, &staging);
    (0..W as usize)
        .map(|x| {
            [
                bytes[x * 4],
                bytes[x * 4 + 1],
                bytes[x * 4 + 2],
                bytes[x * 4 + 3],
            ]
        })
        .collect()
}

/// The 8-bit sRGB encoding of linear `v`.
fn srgb8(v: f64) -> i32 {
    (present::linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as i32
}

/// `pixels` are `ramp`'s twin for [`E0`], one sample per pixel, to one 8-bit step (the layer and the target are 8-bit
/// sRGB).
fn check_pixels(pixels: &[[u8; 4]], ramp: &FieldRamp, u_range: [f64; 2], what: &str) {
    let reads = Reads {
        u_range,
        horizon_steps: INPUTS.horizon_steps,
    };
    for (i, (&v, p)) in E0.iter().zip(pixels).enumerate() {
        let frag = [i as f64 + 0.5, 0.5];
        let want = ramp
            .colour_at_reads(v, true, frag, Invalid::Default, reads)
            .expect("a colour ramp");
        for c in 0..3 {
            assert!(
                (i32::from(p[c]) - srgb8(want[c])).abs() <= 1,
                "{what}: E_0 = {v} draws {p:?}, its twin {want:?}"
            );
        }
    }
}

/// The measured range of [`E0`]'s finite values, `u_range` as the measurement writes it.
fn e0_measured() -> [f64; 2] {
    let finite: Vec<f64> = E0
        .iter()
        .filter(|v| v.is_finite())
        .map(|&v| f64::from(v))
        .collect();
    [
        finite.iter().copied().fold(f64::INFINITY, f64::min),
        finite.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ]
}

/// The `E_0` view built from `e`, its ledger entry, by the default registry.
fn e0_view(e: &Entry) -> FieldRamp {
    defaults::field_ramp(e, Slot::Colour, &resolve()).unwrap_or_else(|why| panic!("{why}"))
}

/// Seam 5: `E_0`'s ledger scale changes from `diverging` to `to`, which R-403 admits for its range (−∞, ∞); the view
/// built from the changed entry re-styles (another compaction, other pixels, each its twin), while the sim buffer
/// stays as it was, read back byte for byte: nothing re-integrates. With `recompute`, the change also rewrites the sim
/// buffer, as a re-integration would. Then a pan or zoom, a new measured range: a `u_range` write re-places the `lin`
/// view with no compile (lowering contract Part 3); with `bake`, the view carries the measured range as literals
/// instead, and the write cannot move it.
///
/// This shows the render side only: the changed entry is a clone of `E_0`'s, built into a view without regenerating
/// the ledger, so the payload schema version, which hashes each entry's `scale` (`ledger::version`), is not consulted.
/// Whether a real ledger scale edit moves that version, and so invalidates every cached payload (caching contract
/// Part 1, "Validity"), against seam 5's "zero recompute", is RQ-297's open question.
fn check_restyle(to: Scale, recompute: bool, bake: bool) {
    let mut s = row_scene("restyle", W, Colouring::View("E_0")).unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in E0.into_iter().enumerate() {
        s.set.sample(i as u32).E_0(v);
    }
    let mut rig = rig(&s);
    let before = entry("E_0");
    assert_eq!(before.scale, Scale::Diverging);
    let m = e0_measured();
    let a = e0_view(&before);
    let pa = rig.frame(&a, m);
    check_pixels(&pa, &a, m, "before");

    let mut after = before.clone();
    after.scale = to;
    let mut b = e0_view(&after);
    if bake {
        if let Compaction::LinRead { .. } = b.compaction {
            let r = m[0].abs().max(m[1].abs());
            b.compaction = Compaction::Lin { lo: -r, hi: r };
        }
    }
    if recompute {
        let fresh: Vec<u8> = rig.bytes.iter().map(|b| b.wrapping_add(1)).collect();
        rig.h.queue().write_buffer(&rig.simstate, 0, &fresh);
    }
    let pb = rig.frame(&b, m);
    assert!(
        a.compaction != b.compaction && pa != pb,
        "the view did not re-style: {:?} → {:?}",
        a.compaction,
        b.compaction
    );
    assert!(
        read_buffer(&rig.h, &rig.simstate) == rig.bytes,
        "the sim buffer changed across the re-style"
    );
    check_pixels(&pb, &b, m, "after");
    assert_eq!(
        rig.rl.cache().compiles(),
        2,
        "the re-style compiled other than its one fragment"
    );
    if to == Scale::Lin {
        let zoomed = [-1.0, 1.0];
        let pc = rig.again(&b, zoomed);
        assert!(pc != pb, "the u_range write did not re-place the view");
        if !bake {
            check_pixels(&pc, &b, zoomed, "after the u_range write");
        }
        assert_eq!(
            rig.rl.cache().compiles(),
            2,
            "the u_range write reached a compiler"
        );
    }
}

#[test]
fn ledger_scale_restyle_zero_recompute() {
    check_restyle(Scale::Lin, false, false);
}

negative_control!(
    ledger_scale_restyle_zero_recompute,
    "a re-style that rewrites the sim buffer, as a re-integration does, is caught",
    expected = "the sim buffer changed across the re-style",
    check_restyle(Scale::Lin, true, false)
);

#[cfg(feature = "controls")]
mod ledger_scale_restyle_unchanged {
    use super::*;
    negative_control!(
        ledger_scale_restyle_zero_recompute,
        "a scale \"change\" to the scale it had does not re-style the view",
        expected = "the view did not re-style",
        check_restyle(Scale::Diverging, false, false)
    );
}

#[cfg(feature = "controls")]
mod ledger_scale_restyle_baked {
    use super::*;
    negative_control!(
        ledger_scale_restyle_zero_recompute,
        "a view whose measured range is baked into its source is not re-placed by a u_range write",
        expected = "the u_range write did not re-place the view",
        check_restyle(Scale::Lin, false, true)
    );
}

// ── The twins at their edges, and every form and ramp assembled in either slot ──────────────────────────────────

/// `closure_min`'s ramp by `c`, mapped by `map`, at `polarity`: a float field with no unset value.
fn closure_ramp(c: Compaction, map: Map, polarity: Polarity) -> FieldRamp {
    FieldRamp::new(
        ScalarField::payload("closure_min").unwrap_or_else(|e| panic!("{e}")),
        c,
        map,
    )
    .with_polarity(polarity)
}

/// The CPU twins at their edges: a non-finite value is invalid, both signs; `lin` does not style a negative value,
/// only `log` does; low polarity reverses the value, in the twin and the WGSL; the bands' edges; the override colour
/// and a lightness's placed sample.
fn check_twin_edges(bands: Ramp) {
    let grey = Map::ramp(Ramp::Lut(Lut::Grey));
    let lin = closure_ramp(
        Compaction::Lin { lo: -2.0, hi: 2.0 },
        grey.clone(),
        Polarity::High,
    );
    for x in [f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(lin.shown(x, true), Shown::Invalid, "{x} is invalid");
    }
    assert_eq!(lin.shown(-1.0, true), Shown::Ramp(0.25), "lin places −1.0");
    let low = closure_ramp(Compaction::Lin { lo: -2.0, hi: 2.0 }, grey, Polarity::Low);
    assert_eq!(low.polarity, Polarity::Low);
    assert_eq!(
        low.shown(-1.0, true),
        Shown::Ramp(0.75),
        "low polarity reverses −1.0"
    );
    assert!(
        low.wgsl().contains(
            "return ramp_grey(1.0 - range_norm(v, -2.0, 2.0, false, vec2<f32>(0.0, 0.0)));"
        ),
        "{}",
        low.wgsl()
    );
    for (t, want, what) in [
        (0.0, [1.0, 0.0, 0.0], "a band's start"),
        (0.0624, [1.0, 0.0, 0.0], "inside a band"),
        (0.0625, [0.0, 0.0, 1.0], "a band's end, fract(n·t) = width"),
        (0.2, [0.0, 0.0, 1.0], "between bands"),
        (0.25, [1.0, 0.0, 0.0], "the next band's start"),
        (1.0, [1.0, 0.0, 0.0], "t = 1, fract 0"),
        (-1.0, [1.0, 0.0, 0.0], "t clamped to 0"),
    ] {
        assert_eq!(bands.colour(t), want, "bands at t = {t}, {what}");
    }
    let magenta = [1.0, 0.0, 1.0];
    let nan = f32::from_bits(0x7fc0_0000);
    assert_eq!(
        lin.colour_at(nan, true, [0.5, 0.5], Invalid::Colour(magenta)),
        Some(magenta),
        "the override colour"
    );
    assert_eq!(
        lin.colour_at(nan, true, [0.5, 0.5], Invalid::Default),
        Some(present::present_invalid()),
        "the default invalid colour, #141418 (R-467)"
    );
    let light = closure_ramp(
        Compaction::Lin { lo: -2.0, hi: 2.0 },
        Map::Lightness,
        Polarity::High,
    );
    assert_eq!(
        light.colour_at(1.0, true, [0.5, 0.5], Invalid::Default),
        None
    );
    assert_eq!(light.slot(), Slot::Brightness);
}

/// Four bands over [0, 1], each the first quarter of its period: red on blue.
const BANDS_4: Ramp = Ramp::Bands {
    n: 4.0,
    width: 0.25,
    line: [1.0, 0.0, 0.0],
    base: [0.0, 0.0, 1.0],
};

#[test]
fn field_ramp_twins_at_their_edges() {
    check_twin_edges(BANDS_4);
}

negative_control!(
    field_ramp_twins_at_their_edges,
    "bands whose width is a third of the period draw the band's end as line",
    expected = "a band's end",
    check_twin_edges(Ramp::Bands {
        n: 4.0,
        width: 0.3,
        line: [1.0, 0.0, 0.0],
        base: [0.0, 0.0, 1.0],
    })
);

/// Each ramp kind's settings are header uniforms (lowering contract Part 3, the swatches; Part 4: no gesture reaches a
/// compiler): declared with the ramp's own values as their defaults, and read by name, so a write of one, here
/// through the draw's params, re-colours FTLE's ramp to the twin of the ramp with that setting, one compile. With
/// `write` off, the setting keeps its default, and the image is the default ramp's, not the twin's.
fn check_ramp_uniforms(h: &GpuHarness, write: bool) {
    let s = ftle_scene();
    let field = ScalarField::payload("ftle").unwrap_or_else(|e| panic!("{e}"));
    // A fixed range wider than the scene's FTLE values, its ends off round numbers, so no sample sits at a band's edge
    // (at t = 1, f32's `n·t` may fall just short of the next band).
    let lin = Compaction::Lin { lo: -9.7, hi: 10.3 };
    let bands = |n: f64, width: f64| Ramp::Bands {
        n,
        width,
        line: [1.0, 0.0, 0.0],
        base: [0.0, 0.0, 1.0],
    };
    let cases: [(Ramp, &str, Vec<f64>, Ramp); 4] = [
        (
            Ramp::Lerp { c0: C0, c1: C1 },
            "RAMP_C1",
            C2.to_vec(),
            Ramp::Lerp { c0: C0, c1: C2 },
        ),
        (
            Ramp::Diverging {
                lo: C0,
                mid: C2,
                hi: C1,
            },
            "RAMP_MID",
            C1.to_vec(),
            Ramp::Diverging {
                lo: C0,
                mid: C1,
                hi: C1,
            },
        ),
        (bands(4.0, 0.25), "RAMP_WIDTH", vec![0.75], bands(4.0, 0.75)),
        (bands(4.0, 0.25), "RAMP_N", vec![1.0], bands(1.0, 0.25)),
    ];
    let u_range = reads(&s).u_range.to_vec();
    for (ramp, name, value, twin) in cases {
        let a = FieldRamp::new(field.clone(), lin, Map::ramp(ramp));
        let text = a.wgsl();
        let d = render::assemble::Declaration::parse(&text).unwrap_or_else(|e| panic!("{e}"));
        let declared = d
            .uniforms
            .iter()
            .find(|u| u.name == name)
            .unwrap_or_else(|| panic!("{} declares no `{name}`:\n{text}", ramp.name()));
        let own = match (ramp, name) {
            (Ramp::Lerp { c1, .. }, _) => c1.to_vec(),
            (Ramp::Diverging { mid, .. }, _) => mid.to_vec(),
            (Ramp::Bands { width, .. }, "RAMP_WIDTH") => vec![width],
            (Ramp::Bands { n, .. }, _) => vec![n],
            (Ramp::Lut(_), _) => unreachable!("a LUT has no settings"),
        };
        // The header writes each as its f32's shortest literal, so the two agree as f32s.
        let f32s = |v: &[f64]| v.iter().map(|&x| x as f32).collect::<Vec<_>>();
        assert_eq!(
            f32s(&declared.default),
            f32s(&own),
            "`{name}`'s default is not the ramp's own"
        );
        assert!(
            text.contains(&format!("uniforms.{name}")),
            "{} does not read `{name}`:\n{text}",
            ramp.name()
        );
        let draw = |write: bool| {
            render_stain(
                h.device(),
                h.queue(),
                &ramp_stain(&a, true),
                &s.set,
                &s.context,
                &|_, n| {
                    if n == "u_range" {
                        Some(u_range.clone())
                    } else {
                        (write && n == name).then(|| value.clone())
                    }
                },
            )
            .unwrap_or_else(|e| panic!("{e}"))
        };
        let image = draw(write);
        assert!(
            image != draw(false) || !write,
            "writing `{name}` changed no pixel of {}",
            ramp.name()
        );
        let b = FieldRamp::new(field.clone(), lin, Map::ramp(twin));
        check_image(
            &s,
            &image,
            &b,
            &format!("{} with `{name}` written", ramp.name()),
        );
    }
}

#[test]
fn ramp_kinds_settings_are_uniforms() {
    check_ramp_uniforms(&gpu(), true);
}

negative_control!(
    ramp_kinds_settings_are_uniforms,
    "a draw that leaves each setting at its default is not the written ramp's twin",
    expected = "with `RAMP_C1` written",
    check_ramp_uniforms(&gpu(), false)
);

/// Every compaction, every ramp kind and the lightness, each polarity: the ramp's WGSL names its form, its ramp and
/// its polarity in its header, and assembles through the one assembler, a lightness with its style post, its text
/// changed by `edit` first.
fn check_every_kind(edit: impl Fn(String) -> String) {
    let compactions = [
        Compaction::Lin { lo: 0.0, hi: 1.0 },
        Compaction::Log { eps: 0.5 },
        Compaction::Cyclic { period: 2.0 },
        Compaction::Diverging { x0: 0.25 },
        Compaction::Flag,
    ];
    let maps = [
        Map::ramp(Ramp::Lut(Lut::Viridis)),
        Map::ramp(Ramp::Lut(Lut::Magma)),
        Map::ramp(Ramp::Lut(Lut::Twilight)),
        Map::ramp(Ramp::Lut(Lut::Coolwarm)),
        Map::ramp(Ramp::Lut(Lut::Grey)),
        Map::ramp(Ramp::Lerp { c0: C0, c1: C1 }),
        Map::ramp(Ramp::Diverging {
            lo: C0,
            mid: C2,
            hi: C1,
        }),
        Map::ramp(BANDS_4),
        Map::Lightness,
    ];
    for c in compactions {
        for map in &maps {
            for p in [Polarity::High, Polarity::Low] {
                let ramp = closure_ramp(c, map.clone(), p);
                let text = ramp.wgsl();
                let head: String = text.lines().take(2).collect::<Vec<_>>().join("\n");
                let polarity = if p == Polarity::High { "high" } else { "low" };
                assert!(
                    head.contains(&format!("the `{}`", c.name()))
                        && head.contains(&format!("polarity {polarity}")),
                    "the header does not name the form {} and polarity {polarity}:\n{head}",
                    c.name()
                );
                if let Map::Ramp(r) = map {
                    assert!(
                        head.contains(&format!("`{}`", r.name())),
                        "the header does not name {}:\n{head}",
                        r.name()
                    );
                }
                let (kind, _) = ramp.occupant();
                let occupant = Occupant::Custom(edit(text));
                let stain = custom_stain(
                    kind,
                    occupant,
                    (kind == Kind::Brightness).then(|| ramp.style_post()),
                );
                render::assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| {
                    panic!(
                        "{} · {:?} · {polarity} did not assemble: {e}",
                        c.name(),
                        map
                    )
                });
            }
        }
    }
}

/// The stain of a colour or brightness `occupant` of `kind`, with `post` after Replace-L.
fn custom_stain(kind: Kind, occupant: Occupant, post: Option<String>) -> Stain {
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
    let last = match post {
        Some(p) => {
            nodes.push(node(Kind::Post, Occupant::Custom(p), vec![Some(3)]));
            4
        }
        None => 3,
    };
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn field_either_role_every_form_and_ramp_assembles() {
    check_every_kind(|t| t);
}

negative_control!(
    field_either_role_every_form_and_ramp_assembles,
    "a ramp calling a library function that does not exist does not assemble",
    expected = "did not assemble",
    check_every_kind(|t| t.replace("compact_symlog(", "compact_symlg("))
);
