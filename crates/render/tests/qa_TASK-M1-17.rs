//! QA tests for TASK-M1-17 on the render side, written from R-399, R-400 and the requirements they add, not from the
//! implementation. Each renders a scene of its own through the render harness (`validation::golden_scene`) and reads
//! the pixels:
//! - REQ-TOOL-162 / REQ-COL-064 (R-399): an unset `d_min` (forced failures and unstepped samples, f16 +∞) in its
//!   debug view and in its `FieldRamp`, and running samples in the outcome palette, are drawn in one style, the same
//!   colour at the same pixel in all three; over every 4 × 4 pixel block it takes at least two colours (never flat, so
//!   no flat ramp colour, viridis, twilight, magma, grey or cool-warm, reproduces a block), its two colours differ in
//!   luminance (visible without colour vision), it is dots, not stripes (no block is invariant along a row, a column,
//!   a diagonal or an anti-diagonal, as `debug_invalid`'s and REQ-TOOL-124's pending hatch's stripes are), it differs
//!   from `debug_invalid` at every pixel, and it never shows the pending hatch's blue `#0000FF`
//!   (debug_tooling_plan §F) (`qa_not_yet_style_*`).
//! - REQ-RENDER-084 (R-400, R-122): `E_0`, whose declared range spans zero, renders `ramp_coolwarm` from Moreland's
//!   table: values placed at the table's stops render each stop decoded to linear RGB, values halfway between render
//!   the stops' sRGB mean decoded (render_gui_spec §10.1's interpolation), 0 renders the table's neutral stop 16, the
//!   negative values cool (blue over red) and the positive warm (`qa_ramp_coolwarm_*`). The checked-in table is checked
//!   against matplotlib 3.8.0's `_coolwarm_data` at its ends and midpoint, typed in here.
//! - REQ-RENDER-084 (R-403): `K_0` and `V_0`, physical values, render on viridis in the default (auto) mode, increasing
//!   in the value, `K_0`'s minimum and `V_0`'s most negative value at viridis's dark end (`qa_k0_v0_*`).
//!
//! Tolerance: 1e-5 per linear-RGB channel, parity_contract §4's ~1e-5 starting figure for one GPU evaluation against
//! its f64 reference (R-85). Each test has a registered negative control (R-176).

use render::colour::field_ramp::FieldRamp;
use render::present::{self, Rgb};
use validation::golden_scene::{scene, Colouring, Scene, STATE_SIM_FAILED};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

const TOL: f64 = 1e-5;

/// Payload §2's running state.
const STATE_RUNNING: u32 = 3;

/// REQ-TOOL-124's pending hatch line colour, debug_tooling_plan §F: blue `#0000FF`, as linear RGB.
const PENDING_BLUE: Rgb = [0.0, 0.0, 1.0];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// A rendered image: its width, height and linear RGB pixels, row-major.
struct Image {
    w: u32,
    h: u32,
    px: Vec<Rgb>,
}

impl Image {
    fn at(&self, x: u32, y: u32) -> Rgb {
        self.px[(y * self.w + x) as usize]
    }
}

fn render(h: &GpuHarness, s: &Scene) -> Image {
    let (w, hh) = s.size();
    let px = s
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect();
    Image { w, h: hh, px }
}

/// The eight-sample row of the harness, every sample's `d_min` unset: even samples forced failures holding +∞,
/// odd ones unstepped (R-271).
fn unset_d_min(colouring: Colouring) -> Scene {
    let mut s = scene("length_view").unwrap_or_else(|e| panic!("{e}"));
    s.set = Synthetic::flat(s.context.grid, 0);
    for i in 0..s.context.grid.sample_count() as u32 {
        if i % 2 == 0 {
            s.set
                .sample(i)
                .state(STATE_SIM_FAILED)
                .times(40, 0)
                .d_min(f32::INFINITY);
        } else {
            s.set.sample(i).times(0, 0);
        }
    }
    s.colouring = colouring;
    s
}

/// A row of running samples in the outcome palette, `detail` and `t_end_step` varied.
fn running() -> Scene {
    let mut s = scene("detail_view").unwrap_or_else(|e| panic!("{e}"));
    for i in 0..s.context.grid.sample_count() as u32 {
        s.set
            .sample(i)
            .state(STATE_RUNNING)
            .detail(i % 4)
            .times(i * 7, 0);
    }
    s.colouring = Colouring::Outcome { edit: None };
    s
}

/// Relative luminance of a linear RGB colour (Rec. 709 / sRGB primaries).
fn luminance(c: Rgb) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Checks that every pixel of `img`, `what`, is drawn in the "not yet" style (R-399; REQ-TOOL-162, REQ-COL-064).
fn check_style(img: &Image, what: &str) {
    for y in 0..img.h {
        for x in 0..img.w {
            let c = img.at(x, y);
            let hatch = present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
            assert!(
                !near(c, hatch),
                "{what}: pixel ({x}, {y}) is the invalid hatch's colour there"
            );
            assert!(
                !near(c, PENDING_BLUE),
                "{what}: pixel ({x}, {y}) is the pending hatch's blue"
            );
        }
    }
    let mut colours: Vec<Rgb> = Vec::new();
    for y0 in 0..=img.h - 4 {
        for x0 in 0..=img.w - 4 {
            let block: Vec<(u32, u32)> = (y0..y0 + 4)
                .flat_map(|y| (x0..x0 + 4).map(move |x| (x, y)))
                .collect();
            let mut distinct: Vec<Rgb> = Vec::new();
            for &(x, y) in &block {
                let c = img.at(x, y);
                if !distinct.iter().any(|d| near(*d, c)) {
                    distinct.push(c);
                }
                if !colours.iter().any(|d| near(*d, c)) {
                    colours.push(c);
                }
            }
            assert!(
                distinct.len() >= 2,
                "{what}: the 4 × 4 block at ({x0}, {y0}) is one flat colour {:?}",
                distinct[0]
            );
            // Dots, not lines: along each direction a line or stripe repeats, some pixel of the block differs from
            // its neighbour.
            for (dx, dy, dir) in [
                (1i64, 0i64, "rows"),
                (0, 1, "columns"),
                (1, 1, "diagonals"),
                (1, -1, "anti-diagonals"),
            ] {
                let varies = block.iter().any(|&(x, y)| {
                    let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                    (i64::from(x0)..i64::from(x0) + 4).contains(&nx)
                        && (i64::from(y0)..i64::from(y0) + 4).contains(&ny)
                        && !near(img.at(x, y), img.at(nx as u32, ny as u32))
                });
                assert!(
                    varies,
                    "{what}: the 4 × 4 block at ({x0}, {y0}) is stripes along its {dir}, not dots"
                );
            }
        }
    }
    let ys: Vec<f64> = colours.iter().map(|c| luminance(*c)).collect();
    let (lo, hi) = ys
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), y| {
            (l.min(*y), h.max(*y))
        });
    assert!(
        hi - lo > TOL,
        "{what}: the style's colours {colours:?} share one luminance, invisible without colour vision"
    );
}

/// The three renders, the same style at the same pixel in each (R-399: one style for an unset `d_min` and running
/// samples alike).
fn check_three(view: &Image, ramp: &Image, outcome: &Image) {
    check_style(view, "d_min's debug view");
    check_style(ramp, "d_min's FieldRamp");
    check_style(outcome, "the outcome palette's running class");
    for (other, what) in [(ramp, "d_min's FieldRamp"), (outcome, "the running class")] {
        for y in 0..view.h.min(other.h) {
            for x in 0..view.w.min(other.w) {
                assert!(
                    near(view.at(x, y), other.at(x, y)),
                    "pixel ({x}, {y}): {what} is {:?}, d_min's view {:?}: not one style",
                    other.at(x, y),
                    view.at(x, y)
                );
            }
        }
    }
}

fn three(h: &GpuHarness) -> (Image, Image, Image) {
    let ramp = FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}"));
    (
        render(h, &unset_d_min(Colouring::View("d_min"))),
        render(
            h,
            &unset_d_min(Colouring::Ramp {
                ramp,
                override_on: false,
            }),
        ),
        render(h, &running()),
    )
}

#[test]
fn qa_not_yet_style_one_stipple_in_every_view() {
    let h = gpu();
    let (v, r, o) = three(&h);
    check_three(&v, &r, &o);
}

negative_control!(
    qa_not_yet_style_one_stipple_in_every_view,
    "the flat DBG_NOT_YET grey of before R-399 is caught",
    expected = "is one flat colour",
    {
        let h = gpu();
        let (v, r, o) = three(&h);
        let flat = Image {
            w: v.w,
            h: v.h,
            px: vec![present::not_yet(); v.px.len()],
        };
        check_three(&flat, &r, &o)
    }
);

#[cfg(feature = "controls")]
mod qa_not_yet_style_stripes {
    use super::*;
    negative_control!(
        qa_not_yet_style_one_stipple_in_every_view,
        "a two-grey diagonal stripe pattern is caught as stripes, not dots",
        expected = "is stripes along its diagonals",
        {
            let h = gpu();
            let (v, ..) = three(&h);
            let mut striped = Image {
                w: v.w,
                h: v.h,
                px: Vec::new(),
            };
            for y in 0..v.h {
                for x in 0..v.w {
                    let on = (i64::from(x) - i64::from(y)).rem_euclid(4) < 2;
                    striped.px.push(if on {
                        present::not_yet()
                    } else {
                        present::srgb8([0x80, 0x80, 0x80])
                    });
                }
            }
            check_style(&striped, "a striped style")
        }
    );
}

#[cfg(feature = "controls")]
mod qa_not_yet_style_hatched {
    use super::*;
    negative_control!(
        qa_not_yet_style_one_stipple_in_every_view,
        "the running class drawn in the invalid hatch is caught",
        expected = "the invalid hatch's colour there",
        {
            let h = gpu();
            let (v, r, mut o) = three(&h);
            for y in 0..o.h {
                for x in 0..o.w {
                    o.px[(y * o.w + x) as usize] =
                        present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
                }
            }
            check_three(&v, &r, &o)
        }
    );
}

#[cfg(feature = "controls")]
mod qa_not_yet_style_two_styles {
    use super::*;
    negative_control!(
        qa_not_yet_style_one_stipple_in_every_view,
        "a running class whose stipple is shifted by one pixel is caught as a second style",
        expected = "not one style",
        {
            let h = gpu();
            let (v, r, o) = three(&h);
            let shifted = Image {
                w: o.w,
                h: o.h,
                px: (0..o.h)
                    .flat_map(|y| (0..o.w).map(move |x| (x, y)))
                    .map(|(x, y)| o.at((x + 1) % o.w, y))
                    .collect(),
            };
            check_three(&v, &r, &shifted)
        }
    );
}

// ── REQ-RENDER-084: ramp_coolwarm, Moreland's table ────────────────────────────────────────────────────────────────

/// The checked-in table's stops, sRGB-encoded, as the file lists them.
fn table() -> Vec<[f64; 3]> {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ledger/data/lut/coolwarm.txt");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let v: Vec<f64> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
            assert_eq!(v.len(), 3, "a stop is not three channels: {l}");
            [v[0], v[1], v[2]]
        })
        .collect()
}

/// matplotlib 3.8.0's `_coolwarm_data` (lib/matplotlib/_cm.py), Moreland's CoolWarmFloat33, at x = 0, ½ and 1.
const MPL_ENDS: [(usize, [f64; 3]); 3] = [
    (0, [0.2298057, 0.298717966, 0.753683153]),
    (16, [0.865395197, 0.86541021, 0.865395561]),
    (32, [0.705673158, 0.01555616, 0.150232812]),
];

/// IEC 61966-2-1's sRGB decode.
fn decode(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear(srgb: [f64; 3]) -> Rgb {
    srgb.map(decode)
}

/// `E_0`'s scene with values `xs` (the measured range `[−1, 1]` included, so `M = 1` and `t = (x + 1)/2`).
fn e0_scene(xs: &[f32; 8]) -> Scene {
    let mut s = scene("length_view").unwrap_or_else(|e| panic!("{e}"));
    s.set = Synthetic::flat(s.context.grid, 0);
    for (i, &x) in xs.iter().enumerate() {
        s.set.sample(i as u32).E_0(x);
    }
    s.colouring = Colouring::View("E_0");
    s
}

/// Checks every pixel of sample `i` of `s`'s `img` is `want`.
fn check_tile(s: &Scene, img: &Image, i: u32, want: Rgb, what: &str) {
    for (x, y) in s.pixels(i) {
        let got = img.at(x, y);
        assert!(
            near(got, want),
            "sample {i} pixel ({x}, {y}) is {got:?}, not {what} {want:?}"
        );
    }
}

/// Stops `k`, placed at `x = 2k/32 − 1`, and the midpoints `k + ½`.
const AT_STOPS: [u32; 8] = [0, 5, 11, 16, 21, 27, 32, 8];
const BETWEEN: [u32; 6] = [0, 7, 15, 16, 24, 31];

fn check_coolwarm(stops: &[[f64; 3]], at: &Image, at_s: &Scene, mid: &Image, mid_s: &Scene) {
    assert_eq!(stops.len(), 33, "Moreland's CoolWarmFloat33 has 33 stops");
    for (k, c) in MPL_ENDS {
        assert_eq!(
            stops[k], c,
            "the table's stop {k} is not matplotlib 3.8.0's"
        );
    }
    for (i, k) in AT_STOPS.into_iter().enumerate() {
        check_tile(
            at_s,
            at,
            i as u32,
            linear(stops[k as usize]),
            &format!("stop {k}"),
        );
    }
    for (i, k) in BETWEEN.into_iter().enumerate() {
        let (a, b) = (stops[k as usize], stops[k as usize + 1]);
        let want = linear([0, 1, 2].map(|c| (a[c] + b[c]) / 2.0));
        check_tile(
            mid_s,
            mid,
            i as u32 + 2,
            want,
            &format!("the midpoint of stops {k} and {}", k + 1),
        );
    }
    // 0 renders stop 16 (checked above), the table's neutral: the stop whose channels are closest to equal.
    let spread = |c: &[f64; 3]| {
        c.iter().fold(f64::NEG_INFINITY, |m, v| m.max(*v))
            - c.iter().fold(f64::INFINITY, |m, v| m.min(*v))
    };
    let neutral = (0..stops.len())
        .min_by(|&a, &b| spread(&stops[a]).total_cmp(&spread(&stops[b])))
        .unwrap();
    assert_eq!(
        neutral, 16,
        "the table's neutral is stop {neutral}, not its midpoint t = ½"
    );
    // The two signs on either side.
    for (i, cool) in [
        (0u32, true),
        (1, true),
        (2, true),
        (4, false),
        (5, false),
        (6, false),
    ] {
        let (x, y) = at_s.pixels(i)[0];
        let c = at.at(x, y);
        assert_eq!(
            c[2] > c[0],
            cool,
            "sample {i} ({c:?}) is not on the {} side",
            if cool { "cool" } else { "warm" }
        );
    }
}

fn xs_at_stops() -> [f32; 8] {
    AT_STOPS.map(|k| 2.0 * k as f32 / 32.0 - 1.0)
}

fn xs_between() -> [f32; 8] {
    let mut xs = [-1.0f32, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    for (i, k) in BETWEEN.into_iter().enumerate() {
        xs[i + 2] = (2 * k + 1) as f32 / 32.0 - 1.0;
    }
    xs
}

#[test]
fn qa_ramp_coolwarm_stops_midpoints_and_neutral() {
    let h = gpu();
    let (a, b) = (e0_scene(&xs_at_stops()), e0_scene(&xs_between()));
    check_coolwarm(&table(), &render(&h, &a), &a, &render(&h, &b), &b);
}

negative_control!(
    qa_ramp_coolwarm_stops_midpoints_and_neutral,
    "the most negative value drawn warm, as a ramp read backwards would draw it, fails",
    expected = "not stop 0",
    {
        let h = gpu();
        let (a, b) = (e0_scene(&xs_at_stops()), e0_scene(&xs_between()));
        let mut flipped = render(&h, &a);
        // Paint stop 0's sample warm, as a ramp read backwards would.
        for (x, y) in a.pixels(0) {
            flipped.px[(y * flipped.w + x) as usize] = linear(MPL_ENDS[2].1);
        }
        check_coolwarm(&table(), &flipped, &a, &render(&h, &b), &b)
    }
);

#[cfg(feature = "controls")]
mod qa_ramp_coolwarm_interpolated_linear {
    use super::*;
    negative_control!(
        qa_ramp_coolwarm_stops_midpoints_and_neutral,
        "midpoints read as interpolated in linear RGB rather than in the table's sRGB fail",
        expected = "not the midpoint of stops",
        {
            let h = gpu();
            let (a, b) = (e0_scene(&xs_at_stops()), e0_scene(&xs_between()));
            let stops = table();
            let (img_b, img_a) = (render(&h, &b), render(&h, &a));
            // Check the midpoints against the mean of the decoded stops.
            for (i, k) in BETWEEN.into_iter().enumerate() {
                let (p, q) = (linear(stops[k as usize]), linear(stops[k as usize + 1]));
                let want = [0, 1, 2].map(|c| (p[c] + q[c]) / 2.0);
                check_tile(
                    &b,
                    &img_b,
                    i as u32 + 2,
                    want,
                    &format!("the midpoint of stops {k} and {}", k + 1),
                );
            }
            check_coolwarm(&stops, &img_a, &a, &img_b, &b)
        }
    );
}

// ── REQ-RENDER-084 (R-403): K_0 and V_0 on viridis, increasing ────────────────────────────────────────────────────

/// Physical values: `K₀ ≥ 0`, `V₀ < 0` (dd_decoder §3.6), in an order unlike the index.
const K0: [f32; 8] = [0.9, 0.05, 1.6, 0.36, 0.0, 1.2, 0.2, 0.6];
const V0: [f32; 8] = [-0.8, -2.4, -0.12, -1.5, -0.55, -1.9, -0.3, -1.12];

fn ic_scene(field: &'static str, xs: &[f32; 8]) -> Scene {
    let mut s = scene("length_view").unwrap_or_else(|e| panic!("{e}"));
    s.set = Synthetic::flat(s.context.grid, 0);
    for (i, &x) in xs.iter().enumerate() {
        let ic = s.set.ic(i as u32);
        match field {
            "K_0" => ic.K_0 = x,
            _ => ic.V_0 = x,
        }
    }
    s.colouring = Colouring::View(field);
    s
}

/// Each sample is viridis at `(x − min)/(max − min)` (the default auto mode), so the minimum, `V_0`'s most negative
/// value, is viridis's dark end.
fn check_one_signed(field: &str, xs: &[f32; 8], s: &Scene, img: &Image) {
    let v: Vec<f64> = xs.iter().map(|&x| f64::from(x)).collect();
    let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for (i, x) in v.iter().enumerate() {
        let t = (x - lo) / (hi - lo);
        check_tile(
            s,
            img,
            i as u32,
            present::ramp_viridis(t),
            &format!("`{field}` = {x}'s viridis place {t}"),
        );
    }
}

#[test]
fn qa_k0_v0_viridis_increasing_dark_minimum() {
    let h = gpu();
    for (f, xs) in [("K_0", &K0), ("V_0", &V0)] {
        let s = ic_scene(f, xs);
        check_one_signed(f, xs, &s, &render(&h, &s));
    }
}

negative_control!(
    qa_k0_v0_viridis_increasing_dark_minimum,
    "V_0 read with its most negative value at the bright end (|V_0| increasing) fails",
    expected = "not `V_0` = ",
    {
        let h = gpu();
        let s = ic_scene("V_0", &V0);
        let img = render(&h, &s);
        check_one_signed("V_0", &V0.map(|x| -x), &s, &img)
    }
);
