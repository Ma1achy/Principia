//! QA tests for TASK-M1-20, written from REQ-RENDER-085 and R-423, not from the implementation. `theta`, the unwrapped
//! longitude `θ̃` in radians (dd_integrator §3.7), is an angle, so its generated numeric view is cyclic: raw =
//! fract(θ̃ / 2π) with fract(x) = x − ⌊x⌋ (WGSL's `fract`, not Rust's trunc-based `f32::fract`), on the fixed [0, 1],
//! drawn with `ramp_twilight` (the prelude's LUT index 1), not R-400's `ramp_coolwarm`.
//!
//! Each GPU test renders a row of samples through the render harness (`validation::golden_scene::Scene::render`),
//! `theta`'s checked-in generated view with its header's defaults and the scene's measured `u_range`, and reads the
//! pixels. The reference colour is computed here: matplotlib's published twilight table (the checked-in transcription,
//! `crates/ledger/data/lut/twilight.txt`, its first stop checked against matplotlib 3.8.0's `_twilight_data[0]` typed
//! in here), interpolated in sRGB between stops and decoded by IEC 61966-2-1 (render_gui_spec §10.1's interpolation);
//! the render crate's own `present::ramp_*` mirrors are not used.
//! - `qa_theta_view_is_twilight_at_fract_of_a_turn`: positive and negative `θ̃`, below and beyond one turn, each draws
//!   twilight at `θ̃/2π − ⌊θ̃/2π⌋`; `θ̃ = −π/2` draws twilight at 0.75. Controls (rendered): the view with a trunc-based
//!   fract, the view on `ramp_coolwarm`, and the view in auto range (`RANGE_AUTO` 1) each fail.
//! - `qa_theta_view_one_turn_apart_same_colour`: `θ̃`, `θ̃ + 2π` and `θ̃ − 2π`, either sign, draw one colour. Control:
//!   a half turn apart does not.
//! - `qa_theta_view_tiny_negative_wraps_to_the_top`: a tiny negative `θ̃` draws twilight at `1 − ε` (or at 1.0, which
//!   WGSL's `fract` may return there, the same point modulo one turn); never the twilight colour of a negative raw
//!   clamped. Since twilight's two ends differ by under one 8-bit step, 1.0 and 0 are never told apart by pixel
//!   equality here: the band around the wrap admits both ends.
//! - `qa_theta_generated_header_is_cyclic_fixed`: the checked-in view's header says `scale cyclic` and defaults to
//!   `RANGE_AUTO = 0`, and its ramp is `ramp_twilight`; `q_mass`, `ftle`, `mean_y`, `S` and `diffusion` keep
//!   `ramp_coolwarm` (R-423 leaves them). Control: R-400's `theta` view (coolwarm) fails.
//!
//! Tolerance: 1e-5 per linear-RGB channel, parity_contract §4's ~1e-5 starting figure for one GPU evaluation against
//! its f64 reference (R-85), plus the colour span of the twilight table over the raw's f32 error band: WGSL's f32
//! division is accurate to 2.5 ULP (WGSL § "Floating Point Accuracy"), rounded up to 3, plus the relative error of the
//! f32 constant 2π the view divides by, plus one unit roundoff for `fract`'s result. Each test has a registered negative
//! control (R-176).

use render::registry;
use validation::golden_scene::{row_scene, Colouring, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

const TOL: f64 = 1e-5;

const TAU: f64 = std::f64::consts::TAU;

/// matplotlib 3.8.0's `_twilight_data[0]` (lib/matplotlib/_cm_listed.py).
/// Typed as the source writes them, parsed for the comparison.
const MPL_TWILIGHT_0: [&str; 3] = [
    "0.88575015840754434",
    "0.85000924943067835",
    "0.8879736506427196",
];

type Rgb = [f64; 3];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// A published table's stops, sRGB-encoded, from the ledger's data file `name`.
fn table(name: &str) -> Vec<Rgb> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ledger/data/lut")
        .join(name);
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

fn twilight() -> Vec<Rgb> {
    let t = table("twilight.txt");
    assert_eq!(t.len(), 510, "matplotlib's twilight has 510 stops");
    assert_eq!(
        t[0],
        MPL_TWILIGHT_0.map(|x| x.parse::<f64>().unwrap()),
        "twilight's stop 0 is not matplotlib 3.8.0's"
    );
    t
}

/// IEC 61966-2-1's sRGB decode.
fn decode(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The table `stops` at `t` on [0, 1] (clamped): linear interpolation in sRGB between neighbouring stops, decoded.
fn lookup(stops: &[Rgb], t: f64) -> Rgb {
    let n = stops.len() - 1;
    let x = t.clamp(0.0, 1.0) * n as f64;
    let k = (x.floor() as usize).min(n - 1);
    let f = x - k as f64;
    let (a, b) = (stops[k], stops[k + 1]);
    [0, 1, 2].map(|c| decode(a[c] + (b[c] - a[c]) * f))
}

/// The per-channel `[min, max]` of the table over `t ∈ [a, b] ⊂ [0, 1]`: the interpolant is monotone in each channel
/// between stops, so its extremes are at the ends or at a stop between them.
fn span(stops: &[Rgb], a: f64, b: f64) -> (Rgb, Rgb) {
    let n = (stops.len() - 1) as f64;
    let mut ts = vec![a, b];
    let (k0, k1) = ((a * n).ceil() as i64, (b * n).floor() as i64);
    ts.extend((k0..=k1).map(|k| k as f64 / n));
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for t in ts {
        let c = lookup(stops, t);
        for i in 0..3 {
            lo[i] = lo[i].min(c[i]);
            hi[i] = hi[i].max(c[i]);
        }
    }
    (lo, hi)
}

/// The f32 ULP at `x`.
fn ulp32(x: f64) -> f64 {
    let a = (x as f32).abs();
    f64::from(f32::from_bits(a.to_bits() + 1)) - f64::from(a)
}

/// The f32 error band of the raw the GPU computes for a quotient `q = θ̃/2π`: WGSL's f32 division within 2.5 ULP
/// (3 here), the f32 constant 2π's relative error times `|q|`, and one unit roundoff for `fract`'s result.
fn delta(q: f64) -> f64 {
    let c = f64::from(std::f32::consts::TAU);
    3.0 * ulp32(q) + q.abs() * ((c - TAU) / TAU).abs() + f64::from(f32::EPSILON) / 2.0
}

/// Checks that `got` lies within `TOL` of the table's colours over raw ∈ `[q − d, q + d]` modulo 1 (fract(x) = x − ⌊x⌋,
/// so a band crossing a whole turn takes both ends of the table).
fn check_band(stops: &[Rgb], got: Rgb, q: f64, d: f64, what: &str) {
    let (lo, hi) = (q - d, q + d);
    let bands: Vec<(f64, f64)> = if lo.floor() == hi.floor() {
        vec![(lo - lo.floor(), hi - lo.floor())]
    } else {
        vec![(lo - lo.floor(), 1.0), (0.0, hi - hi.floor())]
    };
    let ok = bands.iter().any(|&(a, b)| {
        let (l, h) = span(stops, a, b);
        (0..3).all(|i| got[i] >= l[i] - TOL && got[i] <= h[i] + TOL)
    });
    let t = q - q.floor();
    assert!(
        ok,
        "{what}: {got:?} is not twilight at fract(θ̃/2π) = {t} ({:?}, raw band ±{d})",
        lookup(stops, t)
    );
}

/// A row of `thetas`, coloured by `colouring`.
fn theta_scene(thetas: &[f32], colouring: Colouring) -> Scene {
    let mut s = row_scene("qa TASK-M1-20", thetas.len() as u32, colouring)
        .unwrap_or_else(|e| panic!("{e}"));
    for (i, &t) in (0u32..).zip(thetas) {
        s.set.sample(i).theta(t);
    }
    s
}

/// Every pixel of every sample of `s`, rendered.
fn render_tiles(s: &Scene) -> Vec<Vec<Rgb>> {
    let h = gpu();
    let img = s
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"));
    let w = s.size().0;
    (0..s.context.grid.sample_count())
        .map(|i| {
            s.pixels(i)
                .into_iter()
                .map(|(x, y)| {
                    let p = img[(y * w + x) as usize];
                    [p[0], p[1], p[2]].map(f64::from)
                })
                .collect()
        })
        .collect()
}

/// `theta`'s checked-in generated view.
fn theta_source() -> String {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == "debug/generated/theta")
        .unwrap_or_else(|| panic!("the registry has no `debug/generated/theta`"))
        .source
}

// ── raw = fract(θ̃/2π) on twilight ──────────────────────────────────────────────────────────────────────────────────

/// Positive and negative `θ̃`, below and beyond one turn, none near a whole turn; `−π/2` first. Their raws span
/// [0.04, 0.995], so an auto range would move every colour.
const THETAS: [f32; 8] = [
    -std::f32::consts::FRAC_PI_2,
    0.3,
    2.0,
    -2.0,
    15.9,
    -22.7,
    100.5,
    -100.25,
];

/// Checks the row `thetas` rendered as `tiles`: each sample twilight at `fract(θ̃/2π)`, `fract = x − ⌊x⌋` the
/// reference `wrap` stands for.
fn check_twilight(thetas: &[f32], tiles: &[Vec<Rgb>], wrap: fn(f64) -> f64) {
    let stops = twilight();
    for (i, (&theta, tile)) in thetas.iter().zip(tiles).enumerate() {
        let q = f64::from(theta) / TAU;
        let r = wrap(q);
        for &got in tile {
            check_band(
                &stops,
                got,
                r,
                delta(q),
                &format!("sample {i}, θ̃ = {theta}"),
            );
        }
    }
}

#[test]
fn qa_theta_view_is_twilight_at_fract_of_a_turn() {
    let s = theta_scene(&THETAS, Colouring::View("theta"));
    check_twilight(&THETAS, &render_tiles(&s), |q| q - q.floor());
    // θ̃ = −π/2 is a quarter turn back: raw 0.75 (REQ-RENDER-085).
    let q = f64::from(-std::f32::consts::FRAC_PI_2) / TAU;
    assert!((q - q.floor() - 0.75).abs() < 1e-7, "−π/2 is not raw 0.75");
}

negative_control!(
    qa_theta_view_is_twilight_at_fract_of_a_turn,
    "the view with a trunc-based fract (x − trunc x, Rust's f32::fract) fails at negative θ̃",
    expected = "sample 0, θ̃ = -1.5707964",
    {
        let src = theta_source();
        let trunc = src.replace(
            "fract(raw / 6.2831855)",
            "(raw / 6.2831855 - trunc(raw / 6.2831855))",
        );
        assert_ne!(src, trunc, "the view has no fract(raw / 2π) to replace");
        let s = theta_scene(&THETAS, Colouring::Probe(trunc, Vec::new()));
        check_twilight(&THETAS, &render_tiles(&s), |q| q - q.floor())
    }
);

#[cfg(feature = "controls")]
mod qa_theta_view_coolwarm {
    use super::*;
    negative_control!(
        qa_theta_view_is_twilight_at_fract_of_a_turn,
        "the view drawn on ramp_coolwarm, as R-400 left it, fails",
        expected = "is not twilight at fract",
        {
            let src = theta_source();
            let cw = src.replace("ramp_twilight(", "ramp_coolwarm(");
            assert_ne!(src, cw, "the view has no ramp_twilight to replace");
            let s = theta_scene(&THETAS, Colouring::Probe(cw, Vec::new()));
            check_twilight(&THETAS, &render_tiles(&s), |q| q - q.floor())
        }
    );
}

#[cfg(feature = "controls")]
mod qa_theta_view_auto_range {
    use super::*;
    negative_control!(
        qa_theta_view_is_twilight_at_fract_of_a_turn,
        "the view in auto range over the measured raws, not the fixed [0, 1], fails",
        expected = "is not twilight at fract",
        {
            let raws: Vec<f64> = THETAS
                .iter()
                .map(|&t| {
                    let q = f64::from(t) / TAU;
                    q - q.floor()
                })
                .collect();
            let lo = raws.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = raws.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let params = vec![
                ("RANGE_AUTO".to_owned(), vec![1.0]),
                ("u_range".to_owned(), vec![lo, hi]),
            ];
            let s = theta_scene(&THETAS, Colouring::Probe(theta_source(), params));
            check_twilight(&THETAS, &render_tiles(&s), |q| q - q.floor())
        }
    );
}

// ── one turn apart, one colour ─────────────────────────────────────────────────────────────────────────────────────

/// Base values, either sign, each paired with itself one turn on and one turn back.
const BASES: [f32; 4] = [0.7, -0.7, 4.0, -3.0];

/// Checks that each base `θ̃` and `θ̃ + k·step` (k = ±1) draw twilight at `fract(θ̃/2π)`, the partner's f32 rounding
/// (its distance from `θ̃ + k·step`) added to its band: with `step` one turn, the claim of REQ-RENDER-085.
fn check_one_turn(step: f64) {
    let stops = twilight();
    let mut thetas = Vec::new();
    let mut refs = Vec::new();
    for &b in &BASES {
        for k in [0.0, 1.0, -1.0] {
            let t = (f64::from(b) + k * step) as f32;
            thetas.push(t);
            // The reference is the base's raw: one whole turn on or back is the same raw.
            refs.push((b, k, (f64::from(t) - f64::from(b) - k * step).abs() / TAU));
        }
    }
    let s = theta_scene(&thetas, Colouring::View("theta"));
    let tiles = render_tiles(&s);
    for ((&t, &(b, k, rounding)), tile) in thetas.iter().zip(&refs).zip(&tiles) {
        let qb = f64::from(b) / TAU;
        let q = f64::from(t) / TAU;
        for &got in tile {
            check_band(
                &stops,
                got,
                qb,
                delta(q) + rounding,
                &format!("θ̃ = {t}, {k:+} turn from {b}"),
            );
        }
    }
}

#[test]
fn qa_theta_view_one_turn_apart_same_colour() {
    check_one_turn(TAU);
}

negative_control!(
    qa_theta_view_one_turn_apart_same_colour,
    "values half a turn apart are not one colour",
    expected = "is not twilight at fract",
    check_one_turn(TAU / 2.0)
);

// ── a tiny negative θ̃ wraps to the top of the turn ─────────────────────────────────────────────────────────────────

/// Tiny negative `θ̃`, each just short of a whole turn back, and one well inside (`−π/2`) to fail a clamp.
const TINY: [f32; 4] = [-1e-7, -1e-30, -1e-3, -std::f32::consts::FRAC_PI_2];

fn check_tiny(src: String) {
    let stops = twilight();
    let s = theta_scene(&TINY, Colouring::Probe(src, Vec::new()));
    let tiles = render_tiles(&s);
    for (i, (&theta, tile)) in TINY.iter().zip(&tiles).enumerate() {
        let q = f64::from(theta) / TAU;
        for &got in tile {
            check_band(
                &stops,
                got,
                q,
                delta(q),
                &format!("tiny sample {i}, θ̃ = {theta}"),
            );
        }
    }
}

#[test]
fn qa_theta_view_tiny_negative_wraps_to_the_top() {
    check_tiny(theta_source());
}

negative_control!(
    qa_theta_view_tiny_negative_wraps_to_the_top,
    "a view that takes |θ̃|, mirroring the retrograde phases, fails at −10⁻³ and −π/2",
    expected = "is not twilight at fract",
    {
        let src = theta_source();
        let mirrored = src.replace("fract(raw / 6.2831855)", "fract(abs(raw) / 6.2831855)");
        assert_ne!(src, mirrored, "the view has no fract(raw / 2π) to replace");
        check_tiny(mirrored)
    }
);

// ── the checked-in view and its neighbours ─────────────────────────────────────────────────────────────────────────

fn generated(field: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("frag/debug/generated")
        .join(format!("{field}.wgsl"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The one `return ramp_…(` line of a generated view's body.
fn ramp_of(src: &str) -> &str {
    let lines: Vec<&str> = src
        .lines()
        .filter(|l| l.trim_start().starts_with("return ramp_"))
        .collect();
    assert_eq!(lines.len(), 1, "not one ramp line: {lines:?}");
    let l = lines[0].trim_start().trim_start_matches("return ");
    &l[..l.find('(').unwrap()]
}

fn check_theta_header(theta: &str) {
    let header: String = theta
        .lines()
        .take_while(|l| l.starts_with("//"))
        .map(|l| l.trim_start_matches("//").trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        header.contains("scale cyclic"),
        "`theta`'s view does not declare the cyclic scale: {header}"
    );
    assert!(
        theta
            .lines()
            .any(|l| l.trim() == "// @uniform RANGE_AUTO: u32 = 0 [0, 1]"),
        "`theta`'s view does not default to RANGE_AUTO = 0"
    );
    assert_eq!(ramp_of(theta), "ramp_twilight", "`theta`'s ramp");
    for f in ["q_mass", "ftle", "mean_y", "S", "diffusion"] {
        assert_eq!(
            ramp_of(&generated(f)),
            "ramp_coolwarm",
            "`{f}` left ramp_coolwarm"
        );
    }
}

#[test]
fn qa_theta_generated_header_is_cyclic_fixed() {
    check_theta_header(&generated("theta"));
    // The registry serves the checked-in file.
    assert_eq!(theta_source(), generated("theta"));
}

negative_control!(
    qa_theta_generated_header_is_cyclic_fixed,
    "R-400's theta view, lin on ramp_coolwarm in auto range, fails",
    expected = "does not declare the cyclic scale",
    check_theta_header(
        "// The debug view of `theta`: scalar index 25, f32, scale lin, range\n\
         // (−∞, ∞).\n\
         // @uniform RANGE_AUTO: u32 = 1 [0, 1]\n\
         // @uniform u_range: vec2<f32> = (0.0, 1.0)\n\
         fn colour(ctx: Ctx) -> vec3<f32> {\n\
         \x20   let m = max(abs(uniforms.u_range.x), abs(uniforms.u_range.y));\n\
         \x20   return ramp_coolwarm(range_norm(raw, -m, m, uniforms.RANGE_AUTO != 0u, vec2<f32>(-m, m)));\n\
         }\n"
    )
);
