//! The shared prelude (`shaders/wgsl/lib/prelude.wgsl`, generated) and the presentation layer
//! (`shaders/wgsl/lib/present.wgsl`, hand-written), compiled for the GPU as a fragment stage and evaluated there against
//! their CPU mirror, `render::present`:
//! - REQ-RENDER-020: the prelude is the layout build step's output, not a hand file, and a custom node calling
//!   `ramp_viridis` compiles and runs (`prelude_generated_*`); the LUT ramps match matplotlib's published tables at
//!   their stops (R-122), and the grey ramp and hue wheel their definitions (`prelude_luts_*`).
//! - REQ-RENDER-021: `range_norm` against the CPU reference, in and out of range, with `auto_range` on and off; out of
//!   range clamps under the fixed range (`range_norm_*`).
//! - REQ-TOOL-009, REQ-TOOL-122: each `dbg_*` helper returns its defined colour for fixture inputs; `dbg_sentinel(-1.0,
//!   p)` is the ramp's colour for −1.0; the absence NaN's bits draw `debug_invalid(p)`, the hatch (R-136)
//!   (`dbg_helpers_*`).
//! - REQ-COL-055: the proposed hatch's pattern and colours, each colour farther in OKLab from every palette entry, every
//!   LUT of colour_composition §7.1 included, than the flat magenta R-16 kept (`dbg_helpers_hatch_*`).
//!
//! Each GPU check takes the WGSL as text, so its control runs the same check on the text with one mutation and shows
//! it fails (pitfalls §9). Each CPU check takes the mirror's function, so its control runs it on a wrong one.

use std::path::Path;

use ledger::gen::{self, prelude};
use render::present::{self, Rgb};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

/// The test entry appended to the prelude and the presentation layer.
const ENTRY: &str = include_str!("prelude_entry.wgsl");

/// `prelude_entry.wgsl`'s target width, in cases per row.
const CASE_WIDTH: usize = 64;

/// A custom colour node (render contract Part 2: custom is an occupant, through the one compile path): it calls the
/// prelude's `ramp_viridis`, as any node may (render_gui_spec §10.1).
const CUSTOM_NODE: &str = r"
fn colour_custom(t: f32) -> vec3<f32> { return ramp_viridis(t); }

@group(2) @binding(0) var<storage, read> custom_t: array<f32>;

@fragment
fn t_custom(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    return vec4<u32>(bitcast<vec3<u32>>(colour_custom(custom_t[u32(pos.x)])), 0u);
}
";

/// The f32 unit roundoff, `2⁻²⁴`.
const U: f64 = f32::EPSILON as f64 / 2.0;

/// The worst-case error of an f32 `srgb_to_linear` on the GPU against the f64 transfer, absolute, for an encoded value
/// in [0, 1], from WGSL's stated accuracies (WGSL § "Floating Point Accuracy"): `pow(x, 2.4)` is inherited from
/// `exp2(2.4 · log2(x))`; for `x = (c + 0.055)/1.055` in [0.04, 1], `log2(x)` lies in [−4.7, 0] and is within
/// `3 · 2⁻²¹` (3 ULP, or `2⁻²¹` absolute on [0.5, 2]), so the exponent is within `7.2e-6` and the result within
/// `ln 2 · 7.2e-6 = 5.0e-6` relative from it; `exp2` adds `3 + 2·|2.4 · log2 x|` ≤ 26 ULP, `3.1e-6`. The input's own
/// rounding (the stop's f32, the interpolation, `+ 0.055` and `/ 1.055`, a few `U`) moves the result by less than
/// `2.4 · 4U = 5.7e-7` relative. The linear segment, `c / 12.92`, is within 2.5 ULP. The value is at most 1, so the
/// absolute bound is the relative one: `8.7e-6`, rounded up to `1e-5`.
const SRGB_DECODE_BOUND: f64 = 1e-5;

/// WGSL's absolute error bound on `cos` and `sin` over [−π, π] (WGSL § "Floating Point Accuracy"): `2⁻¹¹`.
const TRIG_BOUND: f64 = 1.0 / 2048.0;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The checked-in prelude, then the presentation layer: the shared library a fragment node is assembled after.
fn library() -> String {
    format!(
        "{}\n{}",
        read(prelude::PATH),
        read("crates/render/shaders/wgsl/lib/present.wgsl")
    )
}

/// The WGSL the GPU checks read, with at most one mutation, a control's: `from` replaced by `to`, which the text must
/// contain.
#[derive(Clone, Copy, Debug)]
struct Text {
    from: &'static str,
    to: &'static str,
}

/// The WGSL as it is.
fn as_is() -> Text {
    Text { from: "", to: "" }
}

/// The WGSL with `from` replaced by `to`: a control's one mutation.
#[cfg(feature = "controls")]
fn mutated(from: &'static str, to: &'static str) -> Text {
    Text { from, to }
}

impl Text {
    fn apply(self, text: String) -> String {
        if self.from.is_empty() {
            return text;
        }
        assert!(
            text.contains(self.from),
            "the mutation's target `{}` is not in the WGSL",
            self.from
        );
        text.replace(self.from, self.to)
    }
}

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

/// `source` parsed and validated by naga with every capability; an error panics, rendered against the source.
fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
}

/// One case of `prelude_entry.wgsl`: its operation and up to seven argument words.
#[derive(Clone, Copy, Debug)]
struct Case {
    op: u32,
    args: [u32; 7],
}

const RANGE_NORM: u32 = 1;
const RAMP_VIRIDIS: u32 = 2;
const RAMP_TWILIGHT: u32 = 3;
const RAMP_GREY: u32 = 4;
const HUE_WHEEL: u32 = 5;
const DEBUG_INVALID: u32 = 6;
const DBG_CAT: u32 = 7;
const DBG_LIN: u32 = 8;
const DBG_LOG: u32 = 9;
const DBG_FLAG: u32 = 10;
const DBG_HASH: u32 = 11;
const DBG_SENTINEL: u32 = 12;
const SRGB_TO_LINEAR: u32 = 13;
const LINEAR_TO_SRGB: u32 = 14;

fn f(x: f32) -> u32 {
    x.to_bits()
}

fn case(op: u32, args: &[u32]) -> Case {
    let mut a = [0; 7];
    a[..args.len()].copy_from_slice(args);
    Case { op, args: a }
}

/// Each case evaluated on the GPU by `prelude_entry.wgsl` after the library, `text` applied to the whole module: one
/// draw, a pixel per case, its four result words.
fn run(gpu: &GpuHarness, text: Text, cases: &[Case]) -> Vec<[u32; 4]> {
    let module = text.apply(format!("{}\n{ENTRY}", library()));
    validate(&module);
    let words: Vec<u32> = cases
        .iter()
        .flat_map(|c| {
            [
                c.op, c.args[0], c.args[1], c.args[2], c.args[3], c.args[4], c.args[5], c.args[6],
            ]
        })
        .collect();
    let rows = cases.len().div_ceil(CASE_WIDTH);
    let kernel = gpu
        .fragment(
            &module,
            "t_prelude",
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

/// The pixel centre of case `k`'s pixel, its `@builtin(position).xy`.
fn centre(k: usize) -> [f64; 2] {
    [(k % CASE_WIDTH) as f64 + 0.5, (k / CASE_WIDTH) as f64 + 0.5]
}

fn rgb_of(p: [u32; 4]) -> Rgb {
    [0, 1, 2].map(|c| f64::from(f32::from_bits(p[c])))
}

/// Each channel of the GPU's `got` within `bound[c]` of the CPU's `want`, or a panic naming `what`.
fn check_rgb(what: &str, got: [u32; 4], want: Rgb, bound: Rgb) {
    let g = rgb_of(got);
    for c in 0..3 {
        assert!(
            (g[c] - want[c]).abs() <= bound[c],
            "{what}: the shader's channel {c} is {} and the CPU mirror's {}, beyond the bound {:e}",
            g[c],
            want[c],
            bound[c]
        );
    }
}

/// The largest change in a linear-RGB channel of a LUT ramp per unit `t`: the steepest step between stops times
/// `N − 1`, times the sRGB decode's steepest slope, `2.4/1.055`.
fn ramp_slope(stops: &[Rgb]) -> f64 {
    let step = stops
        .windows(2)
        .flat_map(|w| (0..3).map(move |c| (w[1][c] - w[0][c]).abs()))
        .fold(0.0, f64::max);
    step * (stops.len() - 1) as f64 * 2.4 / 1.055
}

/// The worst-case error of an f32 OKLCH colour on the GPU (`oklch_to_linear(l, c, turns)`) against the f64 one, per
/// channel, its hue already off by up to `dturns` turns: `a` and `b` are off by `c·(2⁻¹¹ + 2π·dturns)` (WGSL's `cos`
/// and `sin` over [−π, π], and the hue); `lms'` by its row of `M₂⁻¹` on them, plus `4U` of its rounding; cubing
/// multiplies that by `3·lms'²` (taken at its largest, the f64 value plus the error), plus `3U` relatively; `M₁⁻¹`
/// sums the three with its absolute coefficients, plus `6U` of each term; and the result's own rounding.
fn oklch_bound(l: f64, c: f64, turns: f64, dturns: f64) -> Rgb {
    let lab_to_lms = [
        [1.0, 0.3963377774, 0.2158037573],
        [1.0, -0.1055613458, -0.0638541728],
        [1.0, -0.0894841775, -1.2914855480],
    ];
    let h = std::f64::consts::TAU * turns;
    let (a, b) = (c * h.cos(), c * h.sin());
    let dab = c * (TRIG_BOUND + std::f64::consts::TAU * dturns) + 2.0 * U;
    let dlms: Vec<f64> = lab_to_lms
        .iter()
        .map(|row| {
            let v = row[0] * l + row[1] * a + row[2] * b;
            let dv = (row[1].abs() + row[2].abs()) * dab + 4.0 * U * (l + a.abs() + b.abs());
            let top = v.abs() + dv;
            3.0 * top * top * dv + 3.0 * U * top * top * top
        })
        .collect();
    present::LMS_TO_LINEAR.map(|row| {
        let lms_top = 1.5;
        (0..3)
            .map(|j| row[j].abs() * (dlms[j] + 6.0 * U * lms_top))
            .sum::<f64>()
            + 4.0 * U
    })
}

fn same(x: f64) -> Rgb {
    [x; 3]
}

// ── prelude_generated (REQ-RENDER-020) ────────────────────────────────────────────────────────────────────────────

/// The checked-in prelude `on_disk` is the layout build step's output: `cargo xtask codegen`'s emitters write it at
/// `prelude::PATH` from the layout table, and its first line says so.
fn check_generated(on_disk: &str) {
    let files = gen::generate(&ledger::layout(), gen::EMITTERS).expect("the layout generates");
    let emitted = files
        .iter()
        .find(|g| g.path.ends_with(prelude::PATH))
        .expect("the emitters write the prelude");
    assert!(
        emitted.contents == on_disk,
        "the checked-in prelude is not the emitters' output: run `cargo xtask codegen`"
    );
    assert!(
        on_disk.starts_with("// Generated by `cargo xtask codegen` from the layout table"),
        "the prelude does not say it is generated"
    );
}

#[test]
fn prelude_generated_by_the_layout_step() {
    check_generated(&read(prelude::PATH));
}

negative_control!(
    prelude_generated_by_the_layout_step,
    "a hand edit to the checked-in prelude must fail the check",
    expected = "is not the emitters' output",
    check_generated(&read(prelude::PATH).replace("ramp_viridis", "ramp_viridis_hand"))
);

/// A custom node calling `ramp_viridis`, appended to the library, `text` applied, compiles (naga and the GPU) and
/// returns the CPU mirror's colour at each of `ts`.
fn check_custom_node(gpu: &GpuHarness, text: Text, ts: &[f32]) {
    let module = text.apply(format!("{}\n{CUSTOM_NODE}", library()));
    validate(&module);
    let kernel = gpu
        .fragment(
            &module,
            "t_custom",
            &[&[], &[], &[BindingKind::Storage]],
            ts.len() as u32,
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let words: Vec<u32> = ts.iter().map(|t| t.to_bits()).collect();
    let pixels = kernel
        .draw(&[&[], &[], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    let slope = ramp_slope(&present::viridis_stops());
    for (&t, &p) in ts.iter().zip(&pixels) {
        let want = present::ramp_viridis(f64::from(t));
        check_rgb(
            &format!("custom node at t = {t}"),
            p,
            want,
            same(SRGB_DECODE_BOUND + slope * 4.0 * U),
        );
    }
}

const CUSTOM_TS: [f32; 6] = [0.0, 0.1, 0.25, 0.5, 0.875, 1.0];

#[test]
fn prelude_generated_custom_node_calling_ramp_viridis_compiles() {
    check_custom_node(&gpu(), as_is(), &CUSTOM_TS);
}

negative_control!(
    prelude_generated_custom_node_calling_ramp_viridis_compiles,
    "a prelude without ramp_viridis must fail to compile the custom node",
    expected = "ramp_viridis",
    check_custom_node(
        &gpu(),
        mutated("fn ramp_viridis(", "fn ramp_viridis_gone("),
        &CUSTOM_TS
    )
);

// ── range_norm (REQ-RENDER-021) ───────────────────────────────────────────────────────────────────────────────────

/// One `range_norm` case: `(x, lo, hi, auto_range, meas)`.
type RangeCase = (f32, f32, f32, bool, [f32; 2]);

/// In and out of the range, auto off and on, an inverted range and a degenerate one.
const RANGE_CASES: [RangeCase; 14] = [
    (5.0, 0.0, 10.0, false, [0.0, 0.0]),
    (2.5, 0.0, 10.0, false, [100.0, 200.0]),
    (0.0, 0.0, 10.0, false, [0.0, 0.0]),
    (10.0, 0.0, 10.0, false, [0.0, 0.0]),
    (-3.0, 0.0, 10.0, false, [0.0, 0.0]),
    (25.0, 0.0, 10.0, false, [-1.0, 100.0]),
    (0.3, 0.0, 1.0, false, [0.0, 0.0]),
    (150.0, 0.0, 10.0, true, [100.0, 200.0]),
    (5.0, 0.0, 10.0, true, [100.0, 200.0]),
    (250.0, 0.0, 10.0, true, [100.0, 200.0]),
    (-0.75, 0.0, 1.0, true, [-1.0, -0.5]),
    (2.0, 10.0, 0.0, false, [0.0, 0.0]),
    (3.0, 4.0, 4.0, false, [0.0, 0.0]),
    (3.0, 0.0, 10.0, true, [7.0, 7.0]),
];

/// The f32 `range_norm`'s worst-case error against the f64 reference: the two subtractions within `U` of their
/// operands' magnitudes, the division within 2.5 ULP (`5U` relative), the clamp exact.
fn range_norm_bound(x: f32, l: f32, h: f32) -> f64 {
    let (x, l, h) = (f64::from(x), f64::from(l), f64::from(h));
    let d = (h - l).abs();
    if d == 0.0 {
        return 0.0;
    }
    (2.0 * U * (x.abs() + l.abs()) + 2.0 * U * (h.abs() + l.abs())) / d + 5.0 * U
}

/// `range_norm` on the GPU, `text` applied, against `reference` for every case.
fn check_range_norm(
    gpu: &GpuHarness,
    text: Text,
    reference: fn(f64, f64, f64, bool, [f64; 2]) -> f64,
) {
    let cases: Vec<Case> = RANGE_CASES
        .iter()
        .map(|&(x, lo, hi, a, m)| {
            case(
                RANGE_NORM,
                &[f(x), f(lo), f(hi), u32::from(a), f(m[0]), f(m[1])],
            )
        })
        .collect();
    let got = run(gpu, text, &cases);
    for (&(x, lo, hi, a, m), p) in RANGE_CASES.iter().zip(got) {
        let want = reference(
            f64::from(x),
            f64::from(lo),
            f64::from(hi),
            a,
            m.map(f64::from),
        );
        let (l, h) = if a { (m[0], m[1]) } else { (lo, hi) };
        let g = f64::from(f32::from_bits(p[0]));
        assert!(
            (g - want).abs() <= range_norm_bound(x, l, h),
            "range_norm({x}, {lo}, {hi}, {a}, {m:?}): the shader gives {g}, the reference {want}"
        );
    }
}

#[test]
fn range_norm_matches_the_cpu_reference() {
    check_range_norm(&gpu(), as_is(), present::range_norm);
}

negative_control!(
    range_norm_matches_the_cpu_reference,
    "a range_norm that ignores auto_range must disagree with the reference",
    expected = "the shader gives",
    check_range_norm(
        &gpu(),
        mutated("let l = select(lo, meas.x, auto_range);", "let l = lo;"),
        present::range_norm
    )
);

/// `range_norm`'s arguments in f64: `(x, lo, hi, auto_range, meas)`.
type RangeArgs = (f64, f64, f64, bool, [f64; 2]);

/// The CPU reference's values on fixture inputs (render_gui_spec §10.1): the fixed range maps linearly and clamps out
/// of range; auto uses `meas` and ignores `lo` and `hi`; a degenerate range reads 0 (R-72).
fn check_range_norm_fixtures(range_norm: fn(f64, f64, f64, bool, [f64; 2]) -> f64) {
    let fixtures: [(RangeArgs, f64); 10] = [
        ((5.0, 0.0, 10.0, false, [100.0, 200.0]), 0.5),
        ((2.5, 0.0, 10.0, false, [0.0, 0.0]), 0.25),
        ((-3.0, 0.0, 10.0, false, [-10.0, 0.0]), 0.0),
        ((25.0, 0.0, 10.0, false, [0.0, 100.0]), 1.0),
        ((150.0, 0.0, 10.0, true, [100.0, 200.0]), 0.5),
        ((5.0, 0.0, 10.0, true, [100.0, 200.0]), 0.0),
        ((250.0, 0.0, 10.0, true, [100.0, 200.0]), 1.0),
        ((2.0, 10.0, 0.0, false, [0.0, 0.0]), 0.8),
        ((3.0, 4.0, 4.0, false, [0.0, 1.0]), 0.0),
        ((3.0, 0.0, 10.0, true, [7.0, 7.0]), 0.0),
    ];
    for ((x, lo, hi, a, m), want) in fixtures {
        let got = range_norm(x, lo, hi, a, m);
        assert!(
            (got - want).abs() < 1e-12,
            "range_norm({x}, {lo}, {hi}, {a}, {m:?}) is {got}, not {want}"
        );
    }
}

#[test]
fn range_norm_fixed_clamps_and_auto_uses_meas() {
    check_range_norm_fixtures(present::range_norm);
}

negative_control!(
    range_norm_fixed_clamps_and_auto_uses_meas,
    "a range_norm that does not clamp must miss the fixtures",
    expected = "not 0",
    check_range_norm_fixtures(|x, lo, hi, _, _| (x - lo) / (hi - lo))
);

// ── prelude_luts (R-122; REQ-RENDER-020) ──────────────────────────────────────────────────────────────────────────

/// A published table: its source, and its length, first and last stops, a middle stop and the sum of every component,
/// as the source writes them.
struct Published {
    source: &'static str,
    len: usize,
    first: Rgb,
    middle: (usize, Rgb),
    last: Rgb,
    sum: f64,
}

/// matplotlib 3.8.0's published tables (`_cm_listed.py`).
const MATPLOTLIB: &str = "matplotlib's published table";

const VIRIDIS: Published = Published {
    source: MATPLOTLIB,
    len: 256,
    first: [0.267004, 0.004874, 0.329415],
    middle: (127, [0.128729, 0.563265, 0.551229]),
    last: [0.993248, 0.906157, 0.143936],
    sum: 331.7005179999998,
};

const TWILIGHT: Published = Published {
    source: MATPLOTLIB,
    len: 510,
    first: [0.8857501584075443, 0.8500092494306783, 0.8879736506427196],
    middle: (
        127,
        [0.38407269378943537, 0.46139018782416635, 0.7309466543290268],
    ),
    last: [0.8857115512284565, 0.8500218611585632, 0.8857253899008712],
    sum: 767.1852642362104,
};

/// `stops` are the published table `p`.
fn check_published(name: &str, stops: &[Rgb], p: &Published) {
    assert_eq!(stops.len(), p.len, "{name}: the data file's stop count");
    let sum: f64 = stops.iter().flatten().sum();
    assert!(
        stops[0] == p.first
            && stops[p.middle.0] == p.middle.1
            && stops[p.len - 1] == p.last
            && (sum - p.sum).abs() < 1e-9,
        "{name}: the data file is not {}",
        p.source
    );
}

/// `lut`'s ramp on the GPU, `text` applied, at every stop `k/(N − 1)`: within the decode's bound of the published
/// stop, decoded; and the CPU mirror's ramp at the stop is the stop, decoded.
fn check_lut_stops(gpu: &GpuHarness, text: Text, op: u32, stops: &[Rgb], name: &str) {
    let last = (stops.len() - 1) as f64;
    let ts: Vec<f32> = (0..stops.len()).map(|k| (k as f64 / last) as f32).collect();
    let cases: Vec<Case> = ts.iter().map(|&t| case(op, &[f(t)])).collect();
    let got = run(gpu, text, &cases);
    let slope = ramp_slope(stops);
    for (k, (&t, p)) in ts.iter().zip(got).enumerate() {
        let want = present::srgb_to_linear3(stops[k]);
        // t = k/(N − 1) rounded to f32 is within U of it, and the shader's x = t·(N − 1) within 2U·(N − 1) of k.
        let dt = 2.0 * U * last;
        check_rgb(
            &format!("{name} at stop {k}"),
            p,
            want,
            same(SRGB_DECODE_BOUND + slope * dt),
        );
        let cpu = present::ramp(stops, f64::from(t));
        assert!(
            (0..3).all(|c| (cpu[c] - want[c]).abs() <= slope * U * last + 1e-15),
            "{name}: the CPU mirror at stop {k} is not the stop"
        );
    }
}

#[test]
fn prelude_luts_viridis_matches_the_published_table_at_its_stops() {
    let stops = present::viridis_stops();
    check_published("viridis", &stops, &VIRIDIS);
    check_lut_stops(&gpu(), as_is(), RAMP_VIRIDIS, &stops, "viridis");
}

negative_control!(
    prelude_luts_viridis_matches_the_published_table_at_its_stops,
    "a viridis stop one off must miss the table",
    expected = "viridis at stop",
    check_lut_stops(
        &gpu(),
        mutated(
            "vec3<f32>(0.128729, 0.563265, 0.551229)",
            "vec3<f32>(0.128729, 0.563265, 0.561229)"
        ),
        RAMP_VIRIDIS,
        &present::viridis_stops(),
        "viridis"
    )
);

#[test]
fn prelude_luts_twilight_matches_the_published_table_at_its_stops() {
    let stops = present::twilight_stops();
    check_published("twilight", &stops, &TWILIGHT);
    check_lut_stops(&gpu(), as_is(), RAMP_TWILIGHT, &stops, "twilight");
}

negative_control!(
    prelude_luts_twilight_matches_the_published_table_at_its_stops,
    "a twilight ramp that skips a stop must miss the table",
    expected = "twilight at stop",
    check_lut_stops(
        &gpu(),
        mutated(
            "let x = clamp(t, 0.0, 1.0) * 509.0;",
            "let x = clamp(t, 0.0, 1.0) * 508.0;"
        ),
        RAMP_TWILIGHT,
        &present::twilight_stops(),
        "twilight"
    )
);

#[test]
fn prelude_luts_data_files_are_the_published_tables() {
    check_published("viridis", &present::viridis_stops(), &VIRIDIS);
    check_published("twilight", &present::twilight_stops(), &TWILIGHT);
}

negative_control!(
    prelude_luts_data_files_are_the_published_tables,
    "a table with one stop changed must not pass as the published one",
    expected = "is not matplotlib's published table",
    {
        let mut stops = present::viridis_stops();
        stops[200][1] += 1e-6;
        check_published("viridis", &stops, &VIRIDIS)
    }
);

/// Between stops the ramps interpolate in sRGB, then decode: the CPU mirror's midpoint between two stops is their
/// mean, decoded; and its ends clamp.
fn check_ramp_between(ramp: fn(&[Rgb], f64) -> Rgb) {
    let stops = present::viridis_stops();
    let last = (stops.len() - 1) as f64;
    for k in [0usize, 63, 200, 254] {
        let got = ramp(&stops, (k as f64 + 0.5) / last);
        let want =
            present::srgb_to_linear3([0, 1, 2].map(|c| (stops[k][c] + stops[k + 1][c]) / 2.0));
        assert!(
            (0..3).all(|c| (got[c] - want[c]).abs() < 1e-12),
            "the ramp between stops {k} and {} is not their sRGB mean, decoded",
            k + 1
        );
    }
    let ends = [
        (ramp(&stops, -0.5), present::srgb_to_linear3(stops[0])),
        (ramp(&stops, 1.5), present::srgb_to_linear3(stops[255])),
    ];
    for (got, want) in ends {
        assert!(got == want, "the ramp does not clamp t to [0, 1]");
    }
}

#[test]
fn prelude_luts_interpolate_in_srgb_and_clamp() {
    check_ramp_between(present::ramp);
}

negative_control!(
    prelude_luts_interpolate_in_srgb_and_clamp,
    "a ramp interpolating in linear RGB must miss the sRGB mean",
    expected = "is not their sRGB mean",
    check_ramp_between(|stops, t| {
        let x = t.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
        let k = (x.floor() as usize).min(stops.len() - 2);
        let (a, b) = (
            present::srgb_to_linear3(stops[k]),
            present::srgb_to_linear3(stops[k + 1]),
        );
        [0, 1, 2].map(|c| a[c] + (b[c] - a[c]) * (x - k as f64))
    })
);

/// The mirror's named ramps each read their own table: `viridis` and `twilight` are the mirror's `ramp_viridis` and
/// `ramp_twilight`, or a control's stand-ins.
fn check_named_ramps(viridis: fn(f64) -> Rgb, twilight: fn(f64) -> Rgb) {
    let (v, w) = (present::viridis_stops(), present::twilight_stops());
    for t in [0.0, 0.3, 0.5, 0.77, 1.0] {
        assert!(
            close(viridis(t), present::ramp(&v, t)),
            "ramp_viridis({t}) does not read the viridis table"
        );
        assert!(
            close(twilight(t), present::ramp(&w, t)),
            "ramp_twilight({t}) does not read the twilight table"
        );
    }
}

#[test]
fn prelude_luts_named_ramps_read_their_own_tables() {
    check_named_ramps(present::ramp_viridis, present::ramp_twilight);
}

negative_control!(
    prelude_luts_named_ramps_read_their_own_tables,
    "a twilight ramp that reads the viridis table must fail",
    expected = "does not read the twilight table",
    check_named_ramps(present::ramp_viridis, present::ramp_viridis)
);

/// `ramp_grey` and `hue_wheel` on the GPU, `text` applied, against the CPU mirror; and the mirror against their
/// definitions: the grey is OKLab `(t, 0, 0)`, `t³` on every channel, `hue_wheel` OKLCH (0.75, 0.12) at `t` turns,
/// periodic in `t` and in gamut at every hue.
fn check_grey_and_wheel(gpu: &GpuHarness, text: Text) {
    let ts: [f32; 9] = [0.0, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0];
    let mut cases: Vec<Case> = ts.iter().map(|&t| case(RAMP_GREY, &[f(t)])).collect();
    cases.extend(ts.iter().map(|&t| case(HUE_WHEEL, &[f(t)])));
    cases.extend([1.25f32, -0.3].map(|t| case(HUE_WHEEL, &[f(t)])));
    let got = run(gpu, text, &cases);
    let (l, c) = prelude::hue_wheel_lc();
    for (k, p) in got.into_iter().enumerate() {
        let t = f64::from(f32::from_bits(cases[k].args[0]));
        if cases[k].op == RAMP_GREY {
            let want = present::ramp_grey(t);
            check_rgb(&format!("ramp_grey({t})"), p, want, same(1e-5));
            assert!(
                want.iter().all(|&v| (v - t * t * t).abs() < 1e-9),
                "ramp_grey({t}) is not OKLab ({t}, 0, 0)"
            );
        } else {
            let want = present::hue_wheel(t);
            check_rgb(
                &format!("hue_wheel({t})"),
                p,
                want,
                oklch_bound(l, c, t, 2.0 * U),
            );
        }
    }
    for k in 0..360 {
        let t = f64::from(k) / 360.0;
        let w = present::hue_wheel(t);
        assert!(
            w.iter().all(|&v| (0.0..=1.0).contains(&v)),
            "hue_wheel({t}) is out of gamut: {w:?}"
        );
        let again = present::hue_wheel(t + 1.0);
        assert!(
            (0..3).all(|ch| (again[ch] - w[ch]).abs() < 1e-9),
            "hue_wheel is not periodic at {t}"
        );
    }
}

#[test]
fn prelude_luts_grey_ramp_and_hue_wheel() {
    check_grey_and_wheel(&gpu(), as_is());
}

negative_control!(
    prelude_luts_grey_ramp_and_hue_wheel,
    "a hue wheel at another chroma must miss the mirror",
    expected = "hue_wheel(",
    check_grey_and_wheel(
        &gpu(),
        mutated(
            "return oklch_to_linear(0.75, 0.12, t);",
            "return oklch_to_linear(0.75, 0.10, t);"
        )
    )
);

/// The colour-space maps the ramps are built on (dd_colouring §3.1). On the GPU, `text` applied: `srgb_to_linear` and
/// `linear_to_srgb` against the CPU mirror over both segments of the transfer, within [`SRGB_DECODE_BOUND`] (the
/// encode's `pow(c, 1/2.4)` is within the same: its `log2` within `3 · 2⁻²⁰` on [0.003, 1], over 2.4, and `exp2`'s
/// 10 ULP). On the CPU, `mirror`: the transfer's fixed points and its value at ½, and OKLab's published values for
/// white and the three primaries, each round-tripping.
fn check_colour_space(gpu: &GpuHarness, text: Text, mirror: fn(f64) -> f64) {
    let encoded: [[f32; 3]; 4] = [
        [0.0, 0.01, 0.04045],
        [0.05, 0.2, 0.5],
        [0.73, 0.9, 1.0],
        [0.03, 0.3, 0.6],
    ];
    let linear: [[f32; 3]; 4] = [
        [0.0, 0.001, 0.0031308],
        [0.004, 0.05, 0.21404114],
        [0.5, 0.8, 1.0],
        [0.002, 0.1, 0.7],
    ];
    let mut cases: Vec<Case> = encoded
        .iter()
        .map(|c| case(SRGB_TO_LINEAR, &c.map(f)))
        .collect();
    cases.extend(linear.iter().map(|c| case(LINEAR_TO_SRGB, &c.map(f))));
    let got = run(gpu, text, &cases);
    for (k, p) in got.into_iter().enumerate() {
        let (input, op): ([f32; 3], fn(f64) -> f64) = if k < encoded.len() {
            (encoded[k], mirror)
        } else {
            (linear[k - encoded.len()], present::linear_to_srgb)
        };
        let want = input.map(|x| op(f64::from(x)));
        check_rgb(
            &format!("the transfer of {input:?}"),
            p,
            want,
            same(SRGB_DECODE_BOUND),
        );
    }
    let fixtures = [
        (0.0, 0.0),
        (1.0, 1.0),
        (0.5, 0.214_041_140_482_232_55),
        (0.04045, 0.04045 / 12.92),
    ];
    for (c, want) in fixtures {
        assert!(
            (mirror(c) - want).abs() < 1e-12,
            "srgb_to_linear({c}) is {}, not {want}",
            mirror(c)
        );
        assert!(
            // The transfer's two segments meet at 0.04045 only to about 1e-8 (sRGB's published constants).
            (present::linear_to_srgb(want) - c).abs() < 1e-7,
            "linear_to_srgb({want}) is not {c}"
        );
    }
    // Ottosson's published OKLab values ("A perceptual color space for image processing", 2020), to their 4 digits.
    let oklab: [(Rgb, Rgb); 4] = [
        ([1.0, 1.0, 1.0], [1.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.6280, 0.2249, 0.1258]),
        ([0.0, 1.0, 0.0], [0.8664, -0.2339, 0.1795]),
        ([0.0, 0.0, 1.0], [0.4520, -0.0325, -0.3115]),
    ];
    for (rgb, lab) in oklab {
        let got = present::linear_to_oklab(rgb);
        assert!(
            (0..3).all(|c| (got[c] - lab[c]).abs() < 1e-4),
            "linear_to_oklab({rgb:?}) is {got:?}, not {lab:?}"
        );
        let back = present::oklab_to_linear(got);
        assert!(
            (0..3).all(|c| (back[c] - rgb[c]).abs() < 1e-6),
            "oklab_to_linear does not invert linear_to_oklab at {rgb:?}: {back:?}"
        );
    }
}

#[test]
fn prelude_luts_colour_space_maps() {
    check_colour_space(&gpu(), as_is(), present::srgb_to_linear);
}

negative_control!(
    prelude_luts_colour_space_maps,
    "an encode with the decode's exponent must miss the mirror",
    expected = "the transfer of",
    check_colour_space(
        &gpu(),
        mutated("pow(c, vec3<f32>(1.0 / 2.4))", "pow(c, vec3<f32>(2.4))"),
        present::srgb_to_linear
    )
);

// ── dbg_helpers (REQ-TOOL-009, REQ-TOOL-122) ──────────────────────────────────────────────────────────────────────

/// The six helpers and `debug_invalid` on the GPU, `text` applied, against the CPU mirror, on fixture inputs.
fn check_helpers_gpu(gpu: &GpuHarness, text: Text) {
    let mut cases = Vec::new();
    let mut want: Vec<(String, Rgb, Rgb)> = Vec::new();
    let viridis = present::viridis_stops();
    let slope = ramp_slope(&viridis);
    let push = |cases: &mut Vec<Case>,
                want: &mut Vec<(String, Rgb, Rgb)>,
                c: Case,
                what: String,
                w: Rgb,
                b: Rgb| {
        cases.push(c);
        want.push((what, w, b));
    };
    let (gl, gc) = prelude::hue_wheel_lc();
    for (i, n) in [
        (0u32, 6u32),
        (5, 6),
        (7, 8),
        (9, 8),
        (0, 9),
        (1, 9),
        (2, 12),
        (8, 12),
        (1000, 1001),
    ] {
        let w = present::dbg_cat(i, n);
        let b = if n <= 8 {
            same(SRGB_DECODE_BOUND)
        } else {
            // The shader's hue is i·round(φ_g·2³²) mod 2³² over 2³², rounded to f32: within (i + 1)·2⁻³² + U turns.
            oklch_bound(
                gl,
                gc,
                present::golden_turns(i),
                (f64::from(i) + 1.0) / 4294967296.0 + U,
            )
        };
        push(
            &mut cases,
            &mut want,
            case(DBG_CAT, &[i, n]),
            format!("dbg_cat({i}, {n})"),
            w,
            b,
        );
    }
    for (x, lo, hi) in [
        (5.0f32, 0.0f32, 10.0f32),
        (-2.0, 0.0, 10.0),
        (12.0, 0.0, 10.0),
        (0.3, 0.25, 0.5),
    ] {
        let w = present::dbg_lin(f64::from(x), f64::from(lo), f64::from(hi));
        let b = same(SRGB_DECODE_BOUND + slope * range_norm_bound(x, lo, hi));
        push(
            &mut cases,
            &mut want,
            case(DBG_LIN, &[f(x), f(lo), f(hi)]),
            format!("dbg_lin({x}, {lo}, {hi})"),
            w,
            b,
        );
    }
    for (x, eps) in [
        (0.0f32, 1e-3f32),
        (1e-3, 1e-3),
        (-0.5, 1e-3),
        (1e6, 1e-6),
        (3e38, 1.0),
    ] {
        let w = present::dbg_log(f64::from(x), f64::from(eps));
        // |x|/eps, 1 + ·, log (3 ULP or 2⁻²¹ absolute), 1 + s, the division and 1 − ·: t within 2⁻¹⁹.
        let b = same(SRGB_DECODE_BOUND + slope * 2f64.powi(-19));
        push(
            &mut cases,
            &mut want,
            case(DBG_LOG, &[f(x), f(eps)]),
            format!("dbg_log({x}, {eps})"),
            w,
            b,
        );
    }
    for v in [false, true] {
        push(
            &mut cases,
            &mut want,
            case(DBG_FLAG, &[u32::from(v)]),
            format!("dbg_flag({v})"),
            present::dbg_flag(v),
            same(SRGB_DECODE_BOUND),
        );
    }
    for v in [0u32, 1, 0xdead_beef, u32::MAX] {
        push(
            &mut cases,
            &mut want,
            case(DBG_HASH, &[v]),
            format!("dbg_hash_u32({v:#x})"),
            present::dbg_hash_u32(v),
            same(SRGB_DECODE_BOUND),
        );
    }
    let nan = f32::from_bits(present::ABSENT_NAN_BITS);
    for x in [-1.0f32, 0.0, 1.0, 3.5, -1e6, nan, 3e38, nan, nan, nan, nan] {
        let k = cases.len();
        let w = present::dbg_sentinel(x, centre(k));
        let b = same(SRGB_DECODE_BOUND + slope * 4.0 * U);
        push(
            &mut cases,
            &mut want,
            case(DBG_SENTINEL, &[f(x)]),
            format!("dbg_sentinel({x}) at {:?}", centre(k)),
            w,
            b,
        );
    }
    for _ in 0..16 {
        let k = cases.len();
        push(
            &mut cases,
            &mut want,
            case(DEBUG_INVALID, &[]),
            format!("debug_invalid({:?})", centre(k)),
            present::debug_invalid(centre(k)),
            same(SRGB_DECODE_BOUND),
        );
    }
    let got = run(gpu, text, &cases);
    for (p, (what, w, b)) in got.into_iter().zip(want) {
        check_rgb(&what, p, w, b);
    }
}

#[test]
fn dbg_helpers_match_the_cpu_mirror_on_the_gpu() {
    check_helpers_gpu(&gpu(), as_is());
}

negative_control!(
    dbg_helpers_match_the_cpu_mirror_on_the_gpu,
    "a dbg_sentinel that tests NaN by value must not draw the hatch",
    expected = "dbg_sentinel(NaN)",
    check_helpers_gpu(
        &gpu(),
        mutated("if (is_absent_nan(x)) {", "if (x == 12345.0) {")
    )
);

/// Each helper's colour for fixture inputs, from its definition (render contract Part 5's renderings, R-72): `mirror`
/// is the CPU mirror, or a control's stand-in for one helper.
struct Mirror {
    dbg_cat: fn(u32, u32) -> Rgb,
    dbg_lin: fn(f64, f64, f64) -> Rgb,
    dbg_log: fn(f64, f64) -> Rgb,
    dbg_flag: fn(bool) -> Rgb,
    pcg: fn(u32) -> u32,
    dbg_hash_u32: fn(u32) -> Rgb,
    dbg_literal: fn(f64) -> f64,
    dbg_sentinel: fn(f32, [f64; 2]) -> Rgb,
}

fn mirror() -> Mirror {
    Mirror {
        dbg_cat: present::dbg_cat,
        dbg_lin: present::dbg_lin,
        dbg_log: present::dbg_log,
        dbg_flag: present::dbg_flag,
        pcg: present::pcg,
        dbg_hash_u32: present::dbg_hash_u32,
        dbg_literal: present::dbg_literal,
        dbg_sentinel: present::dbg_sentinel,
    }
}

fn hex(h: u32) -> Rgb {
    present::srgb8([(h >> 16) as u8, (h >> 8) as u8, h as u8])
}

fn close(a: Rgb, b: Rgb) -> bool {
    (0..3).all(|c| (a[c] - b[c]).abs() < 1e-12)
}

fn check_helper_fixtures(m: &Mirror) {
    // dbg_cat: Okabe–Ito for n ≤ 8, cycling; the golden angle beyond, OKLCH (0.75, 0.12).
    let okabe_ito = [
        0x000000, 0xE69F00, 0x56B4E9, 0x009E73, 0xF0E442, 0x0072B2, 0xD55E00, 0xCC79A7,
    ];
    for (i, &h) in okabe_ito.iter().enumerate() {
        assert!(
            close((m.dbg_cat)(i as u32, 8), hex(h)),
            "dbg_cat({i}, 8) is not Okabe–Ito's #{h:06X}"
        );
        assert!(
            close((m.dbg_cat)(i as u32 + 8, 6), hex(h)),
            "dbg_cat({}, 6) does not cycle to #{h:06X}",
            i + 8
        );
    }
    let phi = (5f64.sqrt() - 1.0) / 2.0;
    for i in [0u32, 1, 2, 9, 100] {
        let want = present::oklch_to_linear(0.75, 0.12, (f64::from(i) * phi).fract());
        assert!(
            close((m.dbg_cat)(i, 9), want),
            "dbg_cat({i}, 9) is not the golden angle's"
        );
    }
    // dbg_lin: viridis over [lo, hi], clamped.
    for (x, t) in [(5.0, 0.5), (-1.0, 0.0), (11.0, 1.0)] {
        assert!(
            close((m.dbg_lin)(x, 0.0, 10.0), present::ramp_viridis(t)),
            "dbg_lin({x}, 0, 10) is not viridis({t})"
        );
    }
    // dbg_log: 1 − 1/(1 + ln(1 + |x|/eps)) on viridis: 0 at 0, ½ at |x| = eps·(e − 1), the end at ∞.
    let e1 = std::f64::consts::E - 1.0;
    for (x, eps, t) in [
        (0.0, 1e-3, 0.0),
        (1e-3 * e1, 1e-3, 0.5),
        (-2.0 * e1, 2.0, 0.5),
        (f64::INFINITY, 1.0, 1.0),
    ] {
        assert!(
            close((m.dbg_log)(x, eps), present::ramp_viridis(t)),
            "dbg_log({x}, {eps}) is not viridis({t})"
        );
    }
    // dbg_flag: bluish green #009E73 for true, vermillion #D55E00 for false.
    assert!(
        close((m.dbg_flag)(true), hex(0x009E73)),
        "dbg_flag(true) is not #009E73"
    );
    assert!(
        close((m.dbg_flag)(false), hex(0xD55E00)),
        "dbg_flag(false) is not #D55E00"
    );
    // dbg_hash_u32: PCG (Jarzynski and Olano 2020), its low three bytes as sRGB.
    for (v, h) in [
        (0u32, 0x07bb_2fe2u32),
        (1, 0xa8be_ea3c),
        (0xdead_beef, 0x6729_9972),
        (u32::MAX, 0xe62a_4902),
    ] {
        assert_eq!((m.pcg)(v), h, "pcg({v:#x})");
        let bytes = h.to_le_bytes();
        let want = present::srgb8([bytes[0], bytes[1], bytes[2]]);
        assert!(
            close((m.dbg_hash_u32)(v), want),
            "dbg_hash_u32({v:#x}) is not its hash's low bytes"
        );
    }
    // dbg_literal and dbg_sentinel: −1 a quarter along, 0 the middle, 1 three quarters, ±∞ the ends; the absence NaN
    // the hatch, −1.0 its literal value on the ramp.
    for (x, t) in [
        (-1.0, 0.25),
        (0.0, 0.5),
        (1.0, 0.75),
        (3.0, 0.875),
        (f64::INFINITY, 1.0),
        (f64::NEG_INFINITY, 0.0),
    ] {
        assert!(
            ((m.dbg_literal)(x) - t).abs() < 1e-12,
            "dbg_literal({x}) is not {t}"
        );
    }
    for p in [[0.5, 0.5], [4.5, 0.5], [2.5, 3.5], [9.5, 20.5]] {
        let nan = f32::from_bits(present::ABSENT_NAN_BITS);
        assert!(
            close((m.dbg_sentinel)(nan, p), present::debug_invalid(p)),
            "dbg_sentinel(NaN, {p:?}) is not the hatch"
        );
        assert!(
            close((m.dbg_sentinel)(-1.0, p), present::ramp_viridis(0.25)),
            "dbg_sentinel(-1.0, {p:?}) is not the ramp's colour for −1.0"
        );
    }
}

#[test]
fn dbg_helpers_return_their_defined_colours() {
    check_helper_fixtures(&mirror());
}

negative_control!(
    dbg_helpers_return_their_defined_colours,
    "a dbg_sentinel that hatches −1.0 as well must fail",
    expected = "is not the ramp's colour for −1.0",
    check_helper_fixtures(&Mirror {
        dbg_sentinel: |x, p| if x < 0.0 || x.to_bits() == present::ABSENT_NAN_BITS {
            present::debug_invalid(p)
        } else {
            present::ramp_viridis(present::dbg_literal(f64::from(x)))
        },
        ..mirror()
    })
);

/// `golden` is `frac(i·φ_g)`, `φ_g = (√5 − 1)/2` (dd_colouring §3.7), to the rounding of evaluating it in f64:
/// `golden` is the mirror's `golden_turns`, or a control's stand-in. The reference is exact to `i·2⁻⁶⁴`:
/// `frac(i·K/2⁶⁴)` in integers, `K = ⌊φ_g·2⁶⁴⌋`. The bound is the f64 evaluation's error: `i·|φ̂ − φ_g|` from the
/// rounded `φ̂`, half an ulp of the product, and the reference's own error, its rounding to f64 included.
/// `frac(i/φ_g)` and `frac(i·(φ_g + 1))` equal `frac(i·φ_g)` in exact arithmetic (`1/φ_g = 1 + φ_g`), so only a bound
/// this tight tells them apart, and only at some `i`: 11, 22 and 44 separate the second, the larger three the first.
fn check_golden_turns(golden: fn(u32) -> f64) {
    const K: u128 = 0x9E37_79B9_7F4A_7C15;
    let two64 = 2f64.powi(64);
    let phi = (5f64.sqrt() - 1.0) / 2.0;
    // φ̂ ∈ [½, 1) has 53 significant bits, so φ̂·2⁶⁴ is an integer, exactly.
    let phi_err = ((phi * two64) as u128).abs_diff(K) as f64 / two64 + 1.0 / two64;
    for i in [11u32, 22, 44, 1000, 12345, 1_000_003] {
        let reference = ((u128::from(i) * K) % (1u128 << 64)) as f64 / two64;
        let x = f64::from(i) * phi;
        let bound = f64::from(i) * phi_err
            + 2f64.powi(x.log2().floor() as i32 - 53)
            + f64::from(i) / two64
            + 2f64.powi(-54);
        let got = golden(i);
        assert!(
            (got - reference).abs() <= bound,
            "golden_turns({i}) = {got} is not frac(i·φ_g) = {reference} within {bound:e}"
        );
    }
}

#[test]
fn dbg_helpers_golden_turns_is_frac_i_phi() {
    check_golden_turns(present::golden_turns);
}

negative_control!(
    dbg_helpers_golden_turns_is_frac_i_phi,
    "frac(i/φ_g), equal only in exact arithmetic, must miss the f64 bound",
    expected = "is not frac(i·φ_g)",
    check_golden_turns(|i| (f64::from(i) / ((5f64.sqrt() - 1.0) / 2.0)).fract())
);

// ── the hatch (REQ-COL-055; R-132, R-136) ─────────────────────────────────────────────────────────────────────────

/// The proposed hatch: violet `#9B00FF` and cyan `#48FFFF` in diagonal stripes 4 px wide across `x + y`, at the
/// pixel centre; `stripe` is the mirror's stripe function, or a control's.
fn check_hatch_pattern(stripe: fn([f64; 2]) -> usize) {
    let h = prelude::hatch();
    assert_eq!(
        h.colours,
        [[0x9b, 0x00, 0xff], [0x48, 0xff, 0xff]],
        "the hatch colours"
    );
    assert_eq!(h.half_period, 4, "the stripe width");
    for y in -8i32..24 {
        for x in -8i32..24 {
            let want = ((x + y).div_euclid(4)).rem_euclid(2) as usize;
            let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            assert_eq!(stripe(p), want, "the stripe at {p:?}");
        }
    }
}

#[test]
fn dbg_helpers_hatch_pattern() {
    check_hatch_pattern(present::hatch_stripe);
}

negative_control!(
    dbg_helpers_hatch_pattern,
    "stripes along x alone must miss the pattern",
    expected = "the stripe at",
    check_hatch_pattern(|p| (p[0].floor() as i64).div_euclid(4).rem_euclid(2) as usize)
);

// ── the hatch's collision evidence (REQ-COL-055): every LUT of colour_composition §7.1 ─────────────────────────────

/// The §7.1 LUTs the prelude does not carry, as published tables (R-16, R-122, R-139): each one's name, its data file
/// under `tests/data/lut/` (each naming its source), its fingerprint, and the scale that takes a component to [0, 1].
const SECTION_7_1_TABLES: [(&str, &str, Published, f64); 7] = [
    (
        "cividis",
        include_str!("data/lut/cividis.txt"),
        Published {
            source: MATPLOTLIB,
            len: 256,
            first: [0.0, 0.135112, 0.304751],
            middle: (127, [0.485141, 0.482451, 0.470384]),
            last: [0.995737, 0.909344, 0.217772],
            sum: 357.2571600000007,
        },
        1.0,
    ),
    (
        "plasma",
        include_str!("data/lut/plasma.txt"),
        Published {
            source: MATPLOTLIB,
            len: 256,
            first: [0.050383, 0.029803, 0.527975],
            middle: (127, [0.794549, 0.27577, 0.473117]),
            last: [0.940015, 0.975158, 0.131326],
            sum: 377.82052599999986,
        },
        1.0,
    ),
    (
        "magma",
        include_str!("data/lut/magma.txt"),
        Published {
            source: MATPLOTLIB,
            len: 256,
            first: [0.001462, 0.000466, 0.013866],
            middle: (127, [0.709962, 0.212797, 0.477201]),
            last: [0.987053, 0.991438, 0.749504],
            sum: 353.4014399999998,
        },
        1.0,
    ),
    (
        "inferno",
        include_str!("data/lut/inferno.txt"),
        Published {
            source: MATPLOTLIB,
            len: 256,
            first: [0.001462, 0.000466, 0.013866],
            middle: (127, [0.729909, 0.212759, 0.333861]),
            last: [0.988362, 0.998364, 0.644924],
            sum: 317.14938400000005,
        },
        1.0,
    ),
    (
        "turbo",
        include_str!("data/lut/turbo.txt"),
        Published {
            source: "Google's published Turbo table (matplotlib's `_turbo_data`)",
            len: 256,
            first: [0.18995, 0.07176, 0.23217],
            middle: (127, [0.63323, 0.99195, 0.23937]),
            last: [0.4796, 0.01583, 0.01055],
            sum: 392.57500000000016,
        },
        1.0,
    ),
    (
        "cool-warm",
        include_str!("data/lut/coolwarm.txt"),
        Published {
            source: "Moreland's 33-stop Cool-warm table",
            len: 33,
            first: [0.2298057, 0.298717966, 0.753683153],
            middle: (16, [0.865395197, 0.86541021, 0.865395561]),
            last: [0.705673158, 0.01555616, 0.150232812],
            sum: 66.45168609999999,
        },
        1.0,
    ),
    (
        "principia",
        include_str!("data/lut/principia.txt"),
        Published {
            source: "the explorer's eight Principia stops",
            len: 8,
            first: [6.0, 4.0, 35.0],
            middle: (3, [22.0, 120.0, 130.0]),
            last: [240.0, 148.0, 10.0],
            sum: 2281.0,
        },
        255.0,
    ),
];

/// A data file's stops, as written: `#` lines are comments, every other line three numbers; a missing or unparsable
/// number reads as NaN, so no stop is dropped silently.
fn table_stops(data: &str) -> Vec<Rgb> {
    data.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split_whitespace().map(|x| x.parse().unwrap_or(f64::NAN));
            [0; 3].map(|_| it.next().unwrap_or(f64::NAN))
        })
        .collect()
}

/// Cubehelix, sRGB-encoded, at `t` (colour_composition §7.1; dd_colouring §3.8's analytic form, the reference, with
/// s = 0.5, λ = 1.5, h = 1): `φ = 2π(s/3 − λt)`, `a = h·t(1 − t)/2`, each channel clamped to [0, 1] for display.
fn cubehelix(t: f64) -> Rgb {
    let (s, lambda, h) = (0.5, 1.5, 1.0);
    let phi = std::f64::consts::TAU * (s / 3.0 - lambda * t);
    let a = h * t * (1.0 - t) / 2.0;
    let (c, n) = (phi.cos(), phi.sin());
    [
        t + a * (-0.14861 * c + 1.78277 * n),
        t + a * (-0.29227 * c - 0.90649 * n),
        t + a * (1.97294 * c),
    ]
    .map(|x| x.clamp(0.0, 1.0))
}

#[test]
fn dbg_helpers_hatch_lut_tables_are_the_published_tables() {
    for (name, data, published, _) in &SECTION_7_1_TABLES {
        check_published(name, &table_stops(data), published);
    }
}

negative_control!(
    dbg_helpers_hatch_lut_tables_are_the_published_tables,
    "a Turbo table with one stop changed must not pass as the published one",
    expected = "turbo: the data file is not Google's published Turbo table",
    {
        let (name, data, published, _) = &SECTION_7_1_TABLES[4];
        let mut stops = table_stops(data);
        stops[91][1] += 1e-5;
        check_published(name, &stops, published)
    }
);

/// Samples per interval between a LUT's stops: the ramp interpolates in sRGB (`present::ramp`), and a colour may sit
/// nearer a point between two stops than either stop.
const PER_INTERVAL: usize = 16;

/// `stops` (sRGB-encoded, in [0, 1]) as the ramp draws them, `PER_INTERVAL` samples per interval, in linear RGB, each
/// named by its position in stops.
fn sampled(name: &str, stops: &[Rgb]) -> Vec<(String, Rgb)> {
    let n = PER_INTERVAL * (stops.len() - 1);
    (0..=n)
        .map(|j| {
            (
                format!("{name} near stop {:.2}", j as f64 / PER_INTERVAL as f64),
                present::ramp(stops, j as f64 / n as f64),
            )
        })
        .collect()
}

/// Every finite palette the hatch must not collide with, as linear RGB: the outcome palette (colour_composition §1.4),
/// the `dbg_*` palettes (Okabe–Ito, the flag pair), every LUT of colour_composition §7.1 (Viridis, Cividis, Plasma,
/// Magma, Inferno, Twilight, Cool-warm, Principia, Cubehelix and Turbo; R-16, R-139) as its ramp draws it, and the
/// grey ramp and the OKLCH hue circle the hue wheel and the golden angle draw from, sampled finely.
fn palettes() -> Vec<(String, Rgb)> {
    let mut out = Vec::new();
    let outcome = [
        0xDE2D2D, 0x2EBC4E, 0x3462E0, 0x141418, 0xECECF0, 0xF0DE32, 0xE034C6, 0x30C8DC, 0xF29620,
    ];
    out.extend(outcome.map(|h| (format!("outcome #{h:06X}"), hex(h))));
    out.extend((0..8).map(|i| (format!("Okabe–Ito {i}"), present::dbg_cat(i, 8))));
    out.extend([true, false].map(|b| (format!("dbg_flag({b})"), present::dbg_flag(b))));
    out.extend(sampled("viridis", &present::viridis_stops()));
    out.extend(sampled("twilight", &present::twilight_stops()));
    for (name, data, _, scale) in &SECTION_7_1_TABLES {
        let stops: Vec<Rgb> = table_stops(data)
            .into_iter()
            .map(|s| s.map(|x| x / scale))
            .collect();
        out.extend(sampled(name, &stops));
    }
    out.extend((0..=4096).map(|k| {
        let t = f64::from(k) / 4096.0;
        (
            format!("cubehelix at {t:.4}"),
            present::srgb_to_linear3(cubehelix(t)),
        )
    }));
    out.extend((0..=1000).map(|k| {
        (
            format!("grey {k}"),
            present::ramp_grey(f64::from(k) / 1000.0),
        )
    }));
    out.extend((0..3600).map(|k| {
        (
            format!("hue {k}"),
            present::hue_wheel(f64::from(k) / 3600.0),
        )
    }));
    out
}

/// The OKLab distance from `c` to the nearest of `palettes`, and its name.
fn nearest(c: Rgb, palettes: &[(String, Rgb)]) -> (f64, String) {
    let lab = present::linear_to_oklab(c);
    palettes
        .iter()
        .map(|(name, p)| {
            let q = present::linear_to_oklab(*p);
            let d = ((lab[0] - q[0]).powi(2) + (lab[1] - q[1]).powi(2) + (lab[2] - q[2]).powi(2))
                .sqrt();
            (d, name.clone())
        })
        .fold((f64::INFINITY, String::new()), |a, b| {
            if b.0 < a.0 {
                b
            } else {
                a
            }
        })
}

/// Each of the hatch's colours `colours` is farther in OKLab from every palette entry than R-16's flat magenta,
/// `#FF00FF`, is from its nearest; the distances are printed, the PR's evidence.
fn check_no_collision(colours: [[u8; 3]; 2]) {
    let palettes = palettes();
    let (magenta, magenta_near) = nearest(hex(0xFF00FF), &palettes);
    eprintln!("flat magenta #FF00FF: {magenta:.3} from {magenta_near}");
    for c in colours {
        let (d, near) = nearest(present::srgb8(c), &palettes);
        eprintln!(
            "hatch #{:02X}{:02X}{:02X}: {d:.3} from {near}",
            c[0], c[1], c[2]
        );
        assert!(
            d > magenta,
            "the hatch colour #{:02X}{:02X}{:02X} is {d:.3} from {near}, no farther than the flat magenta's {magenta:.3}",
            c[0],
            c[1],
            c[2]
        );
    }
}

#[test]
fn dbg_helpers_hatch_collides_with_no_palette_entry() {
    check_no_collision(prelude::hatch().colours);
}

negative_control!(
    dbg_helpers_hatch_collides_with_no_palette_entry,
    "a hatch of body-1 escape's magenta must collide",
    expected = "no farther than the flat magenta's",
    check_no_collision([[0xE0, 0x34, 0xC6], [0x48, 0xFF, 0xFF]])
);

/// The ten LUTs of colour_composition §7.1 (R-16, R-139).
const SECTION_7_1: [&str; 10] = [
    "viridis",
    "cividis",
    "plasma",
    "magma",
    "inferno",
    "twilight",
    "cool-warm",
    "principia",
    "cubehelix",
    "turbo",
];

/// `palettes` samples every LUT of colour_composition §7.1, so the collision check measures against each.
fn check_every_lut(palettes: &[(String, Rgb)]) {
    for lut in SECTION_7_1 {
        let n = palettes
            .iter()
            .filter(|(name, _)| name.starts_with(&format!("{lut} ")))
            .count();
        assert!(
            n > 0,
            "the collision check samples the §7.1 LUT {lut} {n} times"
        );
    }
}

#[test]
fn dbg_helpers_hatch_measured_against_every_section_7_1_lut() {
    check_every_lut(&palettes());
}

negative_control!(
    dbg_helpers_hatch_measured_against_every_section_7_1_lut,
    "a palette set without Turbo, where the first proposal's aquamarine collided, must fail",
    expected = "the §7.1 LUT turbo 0 times",
    check_every_lut(
        &palettes()
            .into_iter()
            .filter(|(name, _)| !name.starts_with("turbo "))
            .collect::<Vec<_>>()
    )
);
