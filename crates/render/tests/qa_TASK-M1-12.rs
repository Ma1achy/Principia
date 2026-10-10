//! QA tests for TASK-M1-12, written from the requirements and their sources, not from the implementation. Each draws a
//! scene of its own (a row of samples, one 8 px tile each) through the render harness
//! (`validation::golden_scene::Scene::render`) and checks the debug view the registry lists against a value computed
//! here in f64 from the corpus's formula; a view's value is read through a probe (the view's WGSL with its `colour`
//! renamed and a `colour` writing its `value` in red). The read side's and the presentation layer's own CPU twins
//! (`kernel::shape`, `present::dbg_dircos*`, `present::dbg_ternary`, `present::word_hash`) are never the reference;
//! only the prior tasks' ramp and hatch mirrors (`present::ramp_*`, `present::debug_invalid`) draw a reference colour.
//! - REQ-TOOL-010: the live shape view's mode 0 is `½(n + 1)` of the shape-sphere point of chart_reference §3.1 (R-14's
//!   convention), mode 1 Twilight at `θ̃/2π mod 1`, negative `θ̃` included; the running `S/t` and the Welford slope
//!   `C_ty/C_tt(n)`, `C_tt = h²n(n² − 1)/12`, match payload §5, NaN where undefined; the drift running-max vs final
//!   view tells a secular drift (final = max) from recovered transients, whatever the final drift's sign
//!   (`qa_live_shape_*`, `qa_accumulator_*`); each live shape mode hatches its undefined value, a coincident `n` or a
//!   non-finite `θ̃` (`qa_live_shape_hatches_undefined_values`).
//! - REQ-TOOL-024: orbit_count `⌊|θ̃|/2π⌋` and retrograde `θ̃ < 0` exactly; the reduced crossing count exactly, from a
//!   free reduction done here, NaN and the hatch when truncated; `ftle = (S + ln(δ/δ₀))/(n·dt)` within
//!   `crates/kernel/tests/derived.rs`'s derived bound, NaN when `ftle_valid` fails; `H(r, p) − E_0` within that file's
//!   drift tolerance (`qa_derived_*`).
//! - REQ-TOOL-025, REQ-TOOL-155: the word-hash view draws the fold `pcg(w ^ pcg(z ^ pcg(y ^ pcg(x))))` of the render
//!   contract's PCG, its low three bytes as sRGB, and words differing in one bit of any limb take distinct colours; a
//!   truncated word draws the hatch (payload §3: every word derivation is invalid once truncated); the symbol at slot
//!   `k` is the freely reduced appended sequence's (`qa_word_*`). The `last_symbol` view draws the stored symbol only
//!   where payload §2's gate `length ≥ 1 && length ≠ 127` holds, the hatch elsewhere (`qa_last_symbol_*`).
//! - REQ-TOOL-156: a truncated word's reduced-length view draws the hatch, its raw `length` view 127 literally on the
//!   ramp; an untruncated one `len/76` on viridis (`qa_word_reduced_length_*`).
//! - REQ-TOOL-154: the masses view draws `(m0, m1, m2)/max mᵢ`: equal masses white, each vertex its primary
//!   (`qa_ternary_*`).
//! - REQ-VAL-010, REQ-TOOL-157: every vector field of the ledger has a '‖·‖ as scalar' view whose colour depends on
//!   `‖v‖` alone and an 'as direction-cosines' view whose colour depends on `v/‖v‖` alone; `r`'s and `p`'s draw each
//!   body's squared share over the largest; `‖n‖ − 1` stays within REQ-VAL-122's proposed tolerance on the degenerate
//!   shapes (collinear, near-collision, Lagrange) (`qa_vector_*`, `qa_norm_n_*`).
//! - REQ-VAL-011: `t_dmin_step` read in the fragment from a packed `times` comes back exactly over every bit, its
//!   `t_end_step` beside it (`qa_t_dmin_*`).
//!
//! Colour tolerance: 1e-5 per linear-RGB channel, parity_contract §4's ~1e-5 for one GPU evaluation against its f64
//! reference (R-85), as qa_TASK-M1-09 takes it. Each test has a registered negative control (R-176).

use ledger::schema::FieldType;
use render::present::{self, Rgb};
use render::registry;
use validation::golden_scene::{
    appended, row_scene, Colouring, DebugCase, Scene, STATE_SIM_FAILED,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

const TOL: f64 = 1e-5;

/// The f32 unit roundoff, `2⁻²⁴`, as `crates/kernel/tests/derived.rs` takes it.
const U: f64 = f32::EPSILON as f64 / 2.0;

/// REQ-VAL-122's proposed tolerance on `‖n‖ − 1` at f32, 64 U (a calibration, R-71: pending the human's confirmation
/// at the M1 gate). Its derivation (first-order rounding of `a`, `b`, `p`, `q`, `I` and the divisions) bounds each
/// component of `n` as well, so the same figure holds `n` against its f64 reference.
const NORM_TOL: f64 = 64.0 * U;

/// `crates/kernel/tests/derived.rs`'s drift tolerance: 1e-6 relative to `|K| + |V| + |E_0|`.
const DRIFT_TOL: f64 = 1e-6;

/// The scene context's `dt_macro` as the fragment reads it, an f32, its `δ₀` and renormalisation interval
/// (`validation::golden_scene`'s `context`).
const DT: f64 = 0.01f32 as f64;
const DELTA_0: f64 = 1e-6;
const N_RENORM: u32 = 16;

const TWO_PI: f64 = std::f64::consts::TAU;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<[f32; 4]> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn source(id: &str) -> String {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("the registry has no `{id}`"))
        .source
}

fn params(p: &[(&str, f64)]) -> Vec<(String, Vec<f64>)> {
    p.iter().map(|(k, v)| ((*k).to_owned(), vec![*v])).collect()
}

/// The view `id` itself, at `p` over its header's defaults.
fn view(id: &str, p: &[(&str, f64)]) -> Colouring {
    Colouring::Debug(Box::leak(Box::new(DebugCase {
        name: format!("qa {id}"),
        id: id.to_owned(),
        params: params(p),
        nudge: 0,
    })))
}

/// The view `id`'s `value`, written exactly in red.
fn probe(id: &str, p: &[(&str, f64)]) -> Colouring {
    let wgsl = format!(
        "{}\nfn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(value(ctx), 0.0, 0.0); }}\n",
        source(id).replace("fn colour(", "fn qa_shown(")
    );
    Colouring::Probe(wgsl, params(p))
}

/// A WGSL expression over `ctx`, written in red.
fn expr(e: &str) -> Colouring {
    Colouring::Probe(
        format!("fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({e}, 0.0, 0.0); }}\n"),
        Vec::new(),
    )
}

fn scene(n: u32, c: Colouring) -> Scene {
    row_scene("qa TASK-M1-12", n, c).unwrap_or_else(|e| panic!("{e}"))
}

fn rgb(p: [f32; 4]) -> Rgb {
    [p[0], p[1], p[2]].map(f64::from)
}

/// Sample `i`'s first pixel's red channel.
fn red(s: &Scene, img: &[[f32; 4]], i: u32) -> f32 {
    let (x, y) = s.pixels(i)[0];
    img[(y * s.size().0 + x) as usize][0]
}

/// Sample `i`'s first pixel's colour.
fn colour_at(s: &Scene, img: &[[f32; 4]], i: u32) -> Rgb {
    let (x, y) = s.pixels(i)[0];
    rgb(img[(y * s.size().0 + x) as usize])
}

/// Every pixel of sample `i` is `want` at that pixel within `tol`.
fn tile(s: &Scene, img: &[[f32; 4]], i: u32, tol: f64, want: &dyn Fn([f64; 2]) -> Rgb, what: &str) {
    let w = s.size().0;
    for (x, y) in s.pixels(i) {
        let got = rgb(img[(y * w + x) as usize]);
        let c = want([f64::from(x) + 0.5, f64::from(y) + 0.5]);
        assert!(
            got.iter().zip(&c).all(|(a, b)| (a - b).abs() <= tol),
            "sample {i}, pixel ({x}, {y}): {got:?} is not {what} {c:?}"
        );
    }
}

/// The literal place of a stored value on viridis, render contract Part 5's `dbg_sentinel`: `0.5 + 0.5·x/(1 + |x|)`.
fn literal(x: f64) -> Rgb {
    present::ramp_viridis(0.5 + 0.5 * x / (1.0 + x.abs()))
}

/// A seeded uniform draw on [0, 1) (splitmix64).
fn rng(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
        s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── The shape-sphere point, chart_reference §3.1 (R-14), in f64 ─────────────────────────────────────────────────────

/// `n = ((a − b)/I, 2p/I, 2q/I)` from the mass-weighted Jacobi vectors `ρ̃ = √μ_ρ (r₁ − r₀)`,
/// `λ̃ = √μ_λ (r₂ − r₀₁)`, `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁` (M = 1), `q` the standard cross `ρ̃ₓλ̃_y − ρ̃_yλ̃ₓ`. `cross`
/// is the sign the reference gives `q`: +1 is R-14's; −1 the superseded convention, for the control.
fn shape_f64(r: [[f32; 2]; 3], m: [f32; 3], cross: f64) -> [f64; 3] {
    let r = r.map(|b| b.map(f64::from));
    let m = m.map(f64::from);
    let m01 = m[0] + m[1];
    let (mr, ml) = (m[0] * m[1] / m01, m[2] * m01);
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]].map(|x| x * mr.sqrt());
    let c = [0, 1].map(|k| (m[0] * r[0][k] + m[1] * r[1][k]) / m01);
    let lam = [r[2][0] - c[0], r[2][1] - c[1]].map(|x| x * ml.sqrt());
    let a = rho[0] * rho[0] + rho[1] * rho[1];
    let b = lam[0] * lam[0] + lam[1] * lam[1];
    let i = a + b;
    let p = rho[0] * lam[0] + rho[1] * lam[1];
    let q = cross * (rho[0] * lam[1] - rho[1] * lam[0]);
    [(a - b) / i, 2.0 * p / i, 2.0 * q / i]
}

/// A random configuration in [−2, 2)² per body and positive masses summing to 1.
fn config(next: &mut dyn FnMut() -> f64) -> ([[f32; 2]; 3], [f32; 3]) {
    let mut r = [[0f32; 2]; 3];
    r.iter_mut()
        .flatten()
        .for_each(|x| *x = (4.0 * next() - 2.0) as f32);
    let raw = [0, 1, 2].map(|_| 0.05 + next());
    let sum: f64 = raw.iter().sum();
    let m = raw.map(|x| (x / sum) as f32);
    (r, m)
}

fn set_masses(s: &mut Scene, i: u32, m: [f32; 3]) {
    let ic = s.set.ic(i);
    (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
}

/// 64 random configurations plus the corpus's landmarks: the equilateral `L⁺` (`n = (0, 0, 1)`, bodies 0 → 1 → 2
/// anticlockwise), its mirror `L⁻`, the 0–1 near-collision (`n → (−1, 0, 0)`), a collinear (Euler-type) line (`w = 0`).
fn shape_cases() -> Vec<([[f32; 2]; 3], [f32; 3])> {
    let mut next = rng(0x0012_5ea5);
    let mut out: Vec<_> = (0..64).map(|_| config(&mut next)).collect();
    let h = (3f32).sqrt() / 2.0;
    let eq = [1.0 / 3.0; 3];
    out.push(([[0.0, 0.0], [1.0, 0.0], [0.5, h]], eq));
    out.push(([[0.0, 0.0], [1.0, 0.0], [0.5, -h]], eq));
    out.push((
        [[0.3, 0.2], [0.3 + 1e-3, 0.2], [-0.7, 0.4]],
        [0.5, 0.3, 0.2],
    ));
    out.push(([[-1.0, 0.0], [0.25, 0.0], [1.5, 0.0]], [0.2, 0.5, 0.3]));
    out
}

fn shape_scene(c: Colouring, cases: &[([[f32; 2]; 3], [f32; 3])]) -> Scene {
    let mut s = scene(cases.len() as u32, c);
    for (i, (r, m)) in (0u32..).zip(cases) {
        s.set.sample(i).r(*r);
        set_masses(&mut s, i, *m);
    }
    s
}

/// The live shape view at `u_mode` 0 draws `½(n + 1)` of the f64 shape point (with `cross`'s sign), each channel within
/// half of [`NORM_TOL`] plus one rounding, and `debug/reductions/n_dircos` draws the same; the landmarks land where
/// chart_reference §3.1 puts them.
fn check_live_shape_mode0(cross: f64) {
    let cases = shape_cases();
    let h = gpu();
    for id in ["debug/live_shape", "debug/reductions/n_dircos"] {
        let p: &[(&str, f64)] = if id == "debug/live_shape" {
            &[("u_mode", 0.0)]
        } else {
            &[]
        };
        let s = shape_scene(view(id, p), &cases);
        let img = render(&h, &s);
        for (i, (r, m)) in (0u32..).zip(&cases) {
            let n = shape_f64(*r, *m, cross);
            let want = n.map(|x| 0.5 * (x + 1.0));
            tile(
                &s,
                &img,
                i,
                0.5 * NORM_TOL + U,
                &|_| want,
                &format!("`{id}`: ½(n + 1) of the shape point"),
            );
        }
    }
    let n = shape_f64(cases[64].0, cases[64].1, 1.0);
    assert!(
        (n[2] - 1.0).abs() < 1e-6,
        "L⁺ is not at w = +1 by this reference: {n:?}"
    );
    let n = shape_f64(cases[66].0, cases[66].1, 1.0);
    assert!(
        n[0] < -0.999,
        "the 0–1 near-collision is not near (−1, 0, 0): {n:?}"
    );
}

#[test]
fn qa_live_shape_mode0_is_half_n_plus_one_of_the_shape_point() {
    check_live_shape_mode0(1.0);
}

negative_control!(
    qa_live_shape_mode0_is_half_n_plus_one_of_the_shape_point,
    "the superseded negative-cross convention, which puts L⁺ at the other pole",
    expected = "½(n + 1) of the shape point",
    check_live_shape_mode0(-1.0)
);

/// `‖n‖ − 1`, `norm` read in the fragment from `ctx.sample.n`, within REQ-VAL-122's proposed tolerance on every shape
/// case, the landmarks included.
fn check_norm(norm: &str) {
    let tol = NORM_TOL;
    let cases = shape_cases();
    let s = shape_scene(expr(&format!("{norm} - 1.0")), &cases);
    let img = render(&gpu(), &s);
    for i in 0..cases.len() as u32 {
        let e = f64::from(red(&s, &img, i));
        assert!(
            e.abs() <= tol,
            "sample {i}: ‖n‖ − 1 = {e:e}, past {tol:e} ({:?})",
            cases[i as usize]
        );
    }
}

#[test]
fn qa_norm_n_within_tolerance_on_the_landmarks() {
    check_norm("length(ctx.sample.n)");
}

negative_control!(
    qa_norm_n_within_tolerance_on_the_landmarks,
    "a norm that drops n's third component, which L⁺ (n = (0, 0, 1)) exposes",
    expected = "‖n‖ − 1 =",
    check_norm("length(ctx.sample.n.xy)")
);

/// A live shape case: whether it is coincident, its `θ̃`, its positions and its masses.
type UndefinedCase = (bool, f32, [[f32; 2]; 3], [f32; 3]);

/// Every live shape mode maps an undefined value to the invalid colour (payload §1; render contract Part 4, Part 5's
/// live shape views): a coincident configuration (`I = 0`, `n = 0/0`) in modes 0 and 2, a NaN or infinite `θ̃` in
/// mode 1; each defined value draws its mode's colour. `undefined(mode, coincident, θ̃)` says where the hatch is due.
fn check_live_shape_undefined(undefined: fn(u32, bool, f32) -> bool) {
    let third = [1.0f32 / 3.0; 3];
    let h3 = (3f32).sqrt() / 2.0;
    let lagrange = [[1.0f32, 0.0], [-0.5, h3], [-0.5, -h3]];
    // Each coincident case exactly so in f32: at the origin, and at a dyadic point with dyadic masses, whose pair
    // centre `(m0·r0 + m1·r1)/m01` is exact, so `ρ = λ = 0` and `I = 0`.
    let cases: [UndefinedCase; 6] = [
        (true, 1.0, [[0.0; 2]; 3], third),
        (false, f32::NAN, lagrange, third),
        (false, f32::INFINITY, lagrange, third),
        (false, f32::NEG_INFINITY, lagrange, third),
        (false, 1.0, lagrange, third),
        (true, -2.0, [[0.5, -0.25]; 3], [0.25, 0.25, 0.5]),
    ];
    let h = gpu();
    for mode in 0u32..3 {
        let mut s = scene(
            cases.len() as u32,
            view("debug/live_shape", &[("u_mode", f64::from(mode))]),
        );
        for (i, &(_, theta, r, m)) in (0u32..).zip(&cases) {
            s.set.sample(i).r(r).theta(theta);
            set_masses(&mut s, i, m);
        }
        let img = render(&h, &s);
        for (i, &(coincident, theta, r, m)) in (0u32..).zip(&cases) {
            let what = format!("mode {mode}, coincident {coincident}, θ̃ {theta}");
            if undefined(mode, coincident, theta) {
                tile(
                    &s,
                    &img,
                    i,
                    TOL,
                    &present::debug_invalid,
                    &format!("the hatch ({what})"),
                );
                continue;
            }
            let (want, name) = match mode {
                0 => (shape_f64(r, m, 1.0).map(|x| 0.5 * (x + 1.0)), "½(n + 1)"),
                1 => (
                    present::ramp_twilight((f64::from(theta) / TWO_PI).rem_euclid(1.0)),
                    "Twilight",
                ),
                _ => (literal(0.0), "‖n‖ − 1's literal place"),
            };
            tile(
                &s,
                &img,
                i,
                0.5 * NORM_TOL + U + TOL,
                &|_| want,
                &format!("{name} ({what})"),
            );
        }
    }
}

#[test]
fn qa_live_shape_hatches_undefined_values() {
    check_live_shape_undefined(|mode, coincident, theta| {
        if mode == 1 {
            !theta.is_finite()
        } else {
            coincident
        }
    });
}

negative_control!(
    qa_live_shape_hatches_undefined_values,
    "Twilight expected of a NaN θ̃, as if the phase were never undefined",
    expected = "is not Twilight (mode 1, coincident false, θ̃ NaN)",
    check_live_shape_undefined(|mode, coincident, _| mode != 1 && coincident)
);

/// Mode 1: Twilight at `fract(θ̃/2π)`, negative `θ̃` wrapping into [0, 1).
fn check_twilight(wrap: fn(f64) -> f64) {
    let thetas = [0.3f32, 3.5, -0.4, -8.2, 15.9, -22.7, 40.1, 100.5];
    let mut s = scene(8, view("debug/live_shape", &[("u_mode", 1.0)]));
    for (i, t) in (0u32..).zip(thetas) {
        s.set.sample(i).theta(t);
    }
    let img = render(&gpu(), &s);
    for (i, t) in (0u32..).zip(thetas) {
        let want = present::ramp_twilight(wrap(f64::from(t) / TWO_PI));
        tile(&s, &img, i, TOL, &|_| want, "Twilight at θ̃/2π mod 1");
    }
}

#[test]
fn qa_live_shape_mode1_is_twilight_of_theta() {
    check_twilight(|x| x - x.floor());
}

negative_control!(
    qa_live_shape_mode1_is_twilight_of_theta,
    "a wrap of |θ̃|, which mirrors the retrograde phases",
    expected = "Twilight at θ̃/2π mod 1",
    check_twilight(|x| x.abs() - x.abs().floor())
);

// ── REQ-TOOL-024: the derived views ─────────────────────────────────────────────────────────────────────────────────

/// `θ̃` values kept off every multiple of 2π: below one turn, just past one, many turns, and negative.
const THETAS: [f32; 12] = [
    0.1, 6.0, 6.5, 12.7, 1000.3, 31.0, -0.1, -6.5, -13.0, -77.7, -1000.3, 3.0,
];

/// orbit_count and retrograde, each through its view's `value` and its colour: the count `⌊|θ̃|/2π⌋` exactly, on
/// viridis at its literal place; retrograde `θ̃ < 0`, green true and red false (`dbg_flag`).
fn check_winding(count: fn(f64) -> f64) {
    let h = gpu();
    let fill = |c: Colouring| {
        let mut s = scene(THETAS.len() as u32, c);
        for (i, t) in (0u32..).zip(THETAS) {
            s.set.sample(i).theta(t);
        }
        s
    };
    let (sp, sv) = (
        fill(probe("debug/derived/orbit_count", &[])),
        fill(view("debug/derived/orbit_count", &[])),
    );
    let (ip, iv) = (render(&h, &sp), render(&h, &sv));
    for (i, t) in (0u32..).zip(THETAS) {
        let want = count(f64::from(t));
        let got = f64::from(red(&sp, &ip, i));
        assert_eq!(got, want, "θ̃ = {t}: orbit_count reads {got}, not {want}");
        tile(&sv, &iv, i, TOL, &|_| literal(want), "orbit_count's place");
    }
    let (sp, sv) = (
        fill(probe("debug/derived/retrograde", &[])),
        fill(view("debug/derived/retrograde", &[])),
    );
    let (ip, iv) = (render(&h, &sp), render(&h, &sv));
    let green = present::srgb8([0, 158, 115]);
    let vermillion = present::srgb8([213, 94, 0]);
    for (i, t) in (0u32..).zip(THETAS) {
        let retro = t < 0.0;
        assert_eq!(
            red(&sp, &ip, i),
            if retro { 1.0 } else { 0.0 },
            "θ̃ = {t}: retrograde"
        );
        tile(
            &sv,
            &iv,
            i,
            TOL,
            &|_| if retro { green } else { vermillion },
            "retrograde's flag",
        );
    }
}

#[test]
fn qa_derived_orbit_count_and_retrograde_exact() {
    check_winding(|t| (t.abs() / TWO_PI).floor());
}

negative_control!(
    qa_derived_orbit_count_and_retrograde_exact,
    "a count that rounds rather than floors",
    expected = "orbit_count reads",
    check_winding(|t| (t.abs() / TWO_PI).round())
);

/// The free reduction of `seq` (codes `a = 0, A = 1, b = 2, B = 3`, each `s`'s inverse `s ^ 1`), and whether it was
/// truncated: a push onto 76 symbols (payload §3, `fgw_capacity`).
fn reduce(seq: &[u32]) -> (Vec<u32>, bool) {
    let mut out: Vec<u32> = Vec::new();
    for &s in seq {
        if out.last() == Some(&(s ^ 1)) {
            out.pop();
        } else if out.len() == 76 {
            return (out, true);
        } else {
            out.push(s);
        }
    }
    (out, false)
}

/// Sequences with and without cancellation, one at the cap, one past it.
fn sequences() -> Vec<Vec<u32>> {
    let cyc = |n: usize| (0..n).map(|k| [0u32, 2, 1, 3][k % 4]).collect::<Vec<_>>();
    vec![
        vec![],
        vec![0, 1],
        vec![2, 0, 1, 3],
        vec![0, 2, 3, 2],
        vec![3, 3, 0, 2, 1],
        cyc(20),
        [cyc(30), vec![3, 2], cyc(5)].concat(),
        cyc(76),
        [cyc(76), vec![1]].concat(),
        [cyc(76), vec![2]].concat(),
        vec![2; 77],
        vec![0; 90],
    ]
}

/// The reduced crossing count: its view's value is the reduced length exactly, NaN when truncated; its colour
/// `len/76` on viridis, the hatch when truncated (REQ-TOOL-156); the raw `length` view shows 127 literally there.
fn check_reduced(reference: fn(&[u32]) -> (Vec<u32>, bool)) {
    let seqs = sequences();
    let h = gpu();
    let fill = |c: Colouring| {
        let mut s = scene(seqs.len() as u32, c);
        for (i, q) in (0u32..).zip(&seqs) {
            s.set.sample(i).word_raw(appended(q).0);
        }
        s
    };
    let sp = fill(probe("debug/word/reduced_length", &[]));
    let sv = fill(view("debug/word/reduced_length", &[]));
    let sl = fill(Colouring::View("length"));
    let (ip, iv, il) = (render(&h, &sp), render(&h, &sv), render(&h, &sl));
    for (i, q) in (0u32..).zip(&seqs) {
        let (red_seq, truncated) = reference(q);
        let got = red(&sp, &ip, i);
        if truncated {
            assert!(
                got.is_nan(),
                "{q:?}: a truncated word's reduced count reads {got}, not NaN"
            );
            tile(&sv, &iv, i, TOL, &present::debug_invalid, "the hatch");
            tile(&sl, &il, i, TOL, &|_| literal(127.0), "127's literal place");
        } else {
            let len = red_seq.len() as f32;
            assert_eq!(got, len, "{q:?}: the reduced count reads {got}, not {len}");
            tile(
                &sv,
                &iv,
                i,
                TOL,
                &|_| present::ramp_viridis(f64::from(len) / 76.0),
                "len/76 on viridis",
            );
        }
    }
}

#[test]
fn qa_derived_reduced_crossing_count_exact() {
    check_reduced(reduce);
}

negative_control!(
    qa_derived_reduced_crossing_count_exact,
    "a count of every push, with no cancellation",
    expected = "the reduced count reads",
    check_reduced(|q| ((q.iter().copied().take(76).collect()), q.len() > 76))
);

/// The ftle cases: `(n, S, δ/δ₀, failed)`.
const FTLE_CASES: [(u32, f32, f32, bool); 9] = [
    (16, 0.5, 3.0, false),
    (30, 1.2, 0.7, false),
    (100, 2.4, 40.0, false),
    (999, 6.0, 1.0, false),
    (17, -0.3, 2.5, false),
    (0, 0.0, 1.0, false),
    (10, 0.2, 2.0, false),
    (15, 0.2, 2.0, false),
    (64, 1.0, 2.0, true),
];

/// `ftle = S_final/(n·dt)`, `S_final = S + ln(δ/δ₀)`, valid iff not failed, `n > 0` and `n/16 > 0` (payload §5, §6),
/// against its view's value, within `derived.rs`'s `ftle_bound`; NaN and the hatch otherwise.
fn check_ftle(finalise: bool) {
    let h = gpu();
    let fill = |c: Colouring| {
        let mut s = scene(FTLE_CASES.len() as u32, c);
        for (i, &(n, big_s, ratio, failed)) in (0u32..).zip(&FTLE_CASES) {
            let r = [[0.3f32, -0.2], [-0.5, 0.4], [0.2, -0.2]];
            let p = [[0.1f32, 0.2], [-0.15, 0.05], [0.05, -0.25]];
            // The shadow off by δ split over two components, a 3-4-5 split.
            let d = (DELTA_0 * f64::from(ratio)) as f32;
            let mut rs = r;
            rs[0][0] += 0.6 * d;
            let mut ps = p;
            ps[2][1] -= 0.8 * d;
            let mut b = s.set.sample(i);
            b.r(r).p(p).r_sh(rs).p_sh(ps).S(big_s).times(n, 0);
            if failed {
                b.state(STATE_SIM_FAILED);
            }
        }
        s
    };
    let sp = fill(probe("debug/derived/ftle", &[]));
    let sv = fill(view("debug/derived/ftle", &[]));
    let (ip, iv) = (render(&h, &sp), render(&h, &sv));
    for (i, &(n, big_s, _, failed)) in (0u32..).zip(&FTLE_CASES) {
        let st = sp.set.simstate(i);
        let d2: f64 = (0..3)
            .flat_map(|b| (0..2).map(move |k| (b, k)))
            .map(|(b, k)| {
                let dr = f64::from(st.r_sh[b][k]) - f64::from(st.r[b][k]);
                let dp = f64::from(st.p_sh[b][k]) - f64::from(st.p[b][k]);
                dr * dr + dp * dp
            })
            .sum();
        let l = (d2.sqrt() / DELTA_0).ln();
        let got = red(&sp, &ip, i);
        let valid = !failed && n > 0 && n / N_RENORM > 0;
        if !valid {
            assert!(
                got.is_nan(),
                "n = {n}, failed {failed}: ftle reads {got}, not NaN"
            );
            tile(&sv, &iv, i, TOL, &present::debug_invalid, "the hatch");
            continue;
        }
        let s_final = f64::from(big_s) + if finalise { l } else { 0.0 };
        let n_dt = f64::from(n) * DT;
        let want = s_final / n_dt;
        let bound = U * (32.0 + 8.0 * l.abs() + 10.0 * s_final.abs()) / n_dt;
        assert!(
            (f64::from(got) - want).abs() <= bound,
            "n = {n}: ftle reads {got}, not {want} within {bound:e}"
        );
        tile(&sv, &iv, i, TOL, &|_| literal(want), "ftle's place");
    }
}

#[test]
fn qa_derived_ftle_finalised() {
    check_ftle(true);
}

negative_control!(
    qa_derived_ftle_finalised,
    "plain S/t, the partial interval not finalised (payload §5's rejected form)",
    expected = "ftle reads",
    check_ftle(false)
);

/// `H(r, p) − E_0` (`K = Σ‖pᵢ‖²/2mᵢ`, `V = −Σ mᵢmⱼ/‖rᵢ − rⱼ‖`, G = 1) with the sample's own masses, against the
/// current-drift view's value within [`DRIFT_TOL`] of `|K| + |V| + |E_0|`, and its colour at that value's place.
fn check_drift(kinetic_half: f64) {
    let mut next = rng(0xd71f);
    let cases: Vec<_> = (0..16)
        .map(|_| {
            let (r, m) = config(&mut next);
            let mut p = [[0f32; 2]; 3];
            p.iter_mut()
                .flatten()
                .for_each(|x| *x = (next() - 0.5) as f32);
            (r, p, m, (next() * 2.0 - 1.5) as f32)
        })
        .collect();
    let h = gpu();
    let fill = |c: Colouring| {
        let mut s = scene(cases.len() as u32, c);
        for (i, (r, p, m, e0)) in (0u32..).zip(&cases) {
            s.set.sample(i).r(*r).p(*p).E_0(*e0);
            set_masses(&mut s, i, *m);
        }
        s
    };
    let sp = fill(probe("debug/derived/energy_drift", &[]));
    let sv = fill(view("debug/derived/energy_drift", &[]));
    let (ip, iv) = (render(&h, &sp), render(&h, &sv));
    for (i, (r, p, m, e0)) in (0u32..).zip(&cases) {
        let f = |x: f32| f64::from(x);
        let k: f64 = (0..3)
            .map(|j| kinetic_half * (f(p[j][0]).powi(2) + f(p[j][1]).powi(2)) / f(m[j]))
            .sum();
        let v: f64 = [(0, 1), (0, 2), (1, 2)]
            .iter()
            .map(|&(a, b)| {
                -f(m[a]) * f(m[b]) / (f(r[a][0]) - f(r[b][0])).hypot(f(r[a][1]) - f(r[b][1]))
            })
            .sum();
        let want = k + v - f(*e0);
        let tol = DRIFT_TOL * (k.abs() + v.abs() + f(*e0).abs());
        let got = f64::from(red(&sp, &ip, i));
        assert!(
            (got - want).abs() <= tol,
            "sample {i}: the drift reads {got}, not {want} within {tol:e}"
        );
        tile(&sv, &iv, i, TOL, &|_| literal(want), "the drift's place");
    }
}

#[test]
fn qa_derived_current_drift_is_h_minus_e0() {
    check_drift(0.5);
}

negative_control!(
    qa_derived_current_drift_is_h_minus_e0,
    "a kinetic term of ‖p‖²/m, without the half",
    expected = "the drift reads",
    check_drift(1.0)
);

// ── REQ-TOOL-010: the accumulator views ─────────────────────────────────────────────────────────────────────────────

/// Running `S/t`, `t = n·dt`, within `7U` relative (`derived.rs`'s product and division), NaN at `n = 0`; and the
/// Welford slope `C_ty/C_tt(n)`, `C_tt = h²n(n² − 1)/12`, within `15U` relative (`derived.rs`'s `DIFFUSION_REL`), NaN
/// for `n < 2` (R-245).
fn check_accumulators(c_tt: fn(f64) -> f64) {
    let cases: [(u32, f32, f32); 7] = [
        (0, 0.0, 1e-4),
        (1, 0.3, 1e-4),
        (2, 0.4, 3e-5),
        (3, 0.9, -2e-4),
        (40, 1.7, 1.5e-3),
        (999, 6.2, -7e-3),
        (65535, 30.0, 0.25),
    ];
    let h = gpu();
    let fill = |c: Colouring| {
        let mut s = scene(cases.len() as u32, c);
        for (i, &(n, big_s, cty)) in (0u32..).zip(&cases) {
            s.set.sample(i).times(n, 0).S(big_s).C_ty(cty);
        }
        s
    };
    let sr = fill(probe("debug/accumulators/ftle_running", &[]));
    let sd = fill(probe("debug/accumulators/diffusion_slope", &[]));
    let (ir, id) = (render(&h, &sr), render(&h, &sd));
    for (i, &(n, big_s, cty)) in (0u32..).zip(&cases) {
        let nf = f64::from(n);
        let got = red(&sr, &ir, i);
        if n == 0 {
            assert!(got.is_nan(), "S/t at n = 0 reads {got}, not NaN");
        } else {
            let want = f64::from(big_s) / (nf * DT);
            assert!(
                (f64::from(got) - want).abs() <= 7.0 * U * want.abs(),
                "n = {n}: S/t reads {got}, not {want}"
            );
        }
        let got = red(&sd, &id, i);
        if n < 2 {
            assert!(got.is_nan(), "the slope at n = {n} reads {got}, not NaN");
        } else {
            let want = f64::from(cty) / c_tt(nf);
            assert!(
                (f64::from(got) - want).abs() <= 15.0 * U * want.abs(),
                "n = {n}: the slope reads {got}, not {want}"
            );
        }
    }
}

#[test]
fn qa_accumulator_running_ftle_and_welford_slope() {
    check_accumulators(|n| DT * DT * n * (n * n - 1.0) / 12.0);
}

negative_control!(
    qa_accumulator_running_ftle_and_welford_slope,
    "C_tt without the 1/12",
    expected = "the slope reads",
    check_accumulators(|n| DT * DT * n * (n * n - 1.0))
);

// ── REQ-TOOL-025, REQ-TOOL-155: the whole-word hash and symbol-at-k ─────────────────────────────────────────────────

/// Render contract Part 5's PCG (Jarzynski & Olano 2020, `pcg_hash`), wrapping u32.
fn pcg(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// The IEC 61966-2-1 sRGB decode.
fn decode(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// `h`'s low three bytes, low first, as 8-bit sRGB red, green, blue, in linear RGB.
fn bytes_rgb(h: u32) -> Rgb {
    [h & 255, (h >> 8) & 255, (h >> 16) & 255].map(|b| decode(f64::from(b) / 255.0))
}

/// The fixture words: each one-bit change of a base word in each limb, the length bits included, and real words.
fn hash_words() -> Vec<[u32; 4]> {
    let base = [0x1234_5678u32, 0x9abc_def0, 0x0f0f_0f0f, 0x0a00_0001];
    let mut out = vec![base, [0; 4]];
    for limb in 0..4 {
        for bit in [0, 7, 15, 24, 25, 31] {
            let mut w = base;
            w[limb] ^= 1 << bit;
            out.push(w);
        }
    }
    out.push([base[3], base[2], base[1], base[0]]);
    for q in sequences() {
        out.push(appended(&q).0);
    }
    let mut unique: Vec<[u32; 4]> = Vec::new();
    for w in out {
        if !unique.contains(&w) {
            unique.push(w);
        }
    }
    unique
}

/// The word-hash view draws `fold`'s colour for each fixture word, and the drawn colours are pairwise distinct; a
/// truncated word (stored length 127) draws the hatch, every word derivation being invalid for it (payload §3).
fn check_hash(fold: fn([u32; 4]) -> u32) {
    let words = hash_words();
    let truncated = |w: &[u32; 4]| w[3] >> 25 == 127;
    assert!(
        words.iter().any(truncated) && !words.iter().all(truncated),
        "the fixture holds both truncated and whole words"
    );
    let mut s = scene(words.len() as u32, view("debug/word/hash", &[]));
    for (i, w) in (0u32..).zip(&words) {
        s.set.sample(i).word_raw(*w);
    }
    let img = render(&gpu(), &s);
    for (i, w) in (0u32..).zip(&words) {
        if truncated(w) {
            tile(&s, &img, i, TOL, &present::debug_invalid, "the hatch");
        } else {
            tile(
                &s,
                &img,
                i,
                TOL,
                &|_| bytes_rgb(fold(*w)),
                &format!("the fold of {w:08x?}"),
            );
        }
    }
    let whole: Vec<usize> = (0..words.len())
        .filter(|&i| !truncated(&words[i]))
        .collect();
    for (j, &a) in whole.iter().enumerate() {
        for &b in &whole[j + 1..] {
            let (ca, cb) = (colour_at(&s, &img, a as u32), colour_at(&s, &img, b as u32));
            assert!(
                ca.iter().zip(&cb).any(|(x, y)| (x - y).abs() > TOL),
                "the words {:08x?} and {:08x?} draw one colour",
                words[a],
                words[b]
            );
        }
    }
}

#[test]
fn qa_word_hash_is_the_limb_fold() {
    check_hash(|w| pcg(w[3] ^ pcg(w[2] ^ pcg(w[1] ^ pcg(w[0])))));
}

negative_control!(
    qa_word_hash_is_the_limb_fold,
    "the fold taken from the top limb down",
    expected = "the fold of",
    check_hash(|w| pcg(w[0] ^ pcg(w[1] ^ pcg(w[2] ^ pcg(w[3])))))
);

/// The `last_symbol` view (payload §2: no in-band "none" code, meaningful iff `length ≥ 1 && length ≠ 127`): where
/// `meaningful(len, truncated)` holds it draws the stored symbol by `dbg_cat(·, 4)` (render contract Part 6, word
/// views), elsewhere the hatch. Each sample stores the reduced word's last symbol, or `1` (`A`) for the empty word,
/// whose stored value "may hold any value" (payload §3) and so must not show.
fn check_last_symbol(meaningful: fn(usize, bool) -> bool) {
    let seqs = sequences();
    let mut s = scene(seqs.len() as u32, Colouring::View("last_symbol"));
    let mut want = Vec::new();
    for (i, q) in (0u32..).zip(&seqs) {
        let (w, truncated) = reduce(q);
        let last = w.last().copied().unwrap_or(1);
        s.set.sample(i).word_raw(appended(q).0).last_symbol(last);
        want.push((meaningful(w.len(), truncated), last, w.len(), truncated));
    }
    assert!(
        want.iter().any(|&(_, _, n, t)| n == 0 && !t) && want.iter().any(|&(_, _, _, t)| t),
        "the fixture holds the empty and a truncated word"
    );
    let img = render(&gpu(), &s);
    for (i, &(shown, last, n, t)) in (0u32..).zip(&want) {
        if shown {
            tile(
                &s,
                &img,
                i,
                TOL,
                &|_| present::dbg_cat(last, 4),
                &format!("last symbol {last} (length {n}, truncated {t})"),
            );
        } else {
            tile(
                &s,
                &img,
                i,
                TOL,
                &present::debug_invalid,
                &format!("the hatch (length {n}, truncated {t})"),
            );
        }
    }
}

#[test]
fn qa_last_symbol_hatches_the_empty_and_truncated_word() {
    check_last_symbol(|len, truncated| len >= 1 && !truncated);
}

negative_control!(
    qa_last_symbol_hatches_the_empty_and_truncated_word,
    "a gate on truncation alone, showing the empty word's stored symbol",
    expected = "is not last symbol 1 (length 0",
    check_last_symbol(|_, truncated| !truncated)
);

/// The symbol at each slot `k` is the freely reduced appended sequence's `k`-th, exactly; none (NaN) past its length
/// or in a truncated word.
fn check_symbols(reference: fn(&[u32]) -> (Vec<u32>, bool)) {
    let seqs = sequences();
    let h = gpu();
    for k in [0u32, 1, 2, 4, 19, 33, 75] {
        let mut s = scene(
            seqs.len() as u32,
            probe("debug/word/symbol_at_k", &[("u_k", f64::from(k))]),
        );
        for (i, q) in (0u32..).zip(&seqs) {
            s.set.sample(i).word_raw(appended(q).0);
        }
        let img = render(&h, &s);
        for (i, q) in (0u32..).zip(&seqs) {
            let (w, truncated) = reference(q);
            let got = red(&s, &img, i);
            match w.get(k as usize).filter(|_| !truncated) {
                Some(&sym) => assert_eq!(
                    got, sym as f32,
                    "{q:?}: the symbol at {k} reads {got}, not {sym}"
                ),
                None => assert!(
                    got.is_nan(),
                    "{q:?}: the symbol at {k} reads {got}, not NaN"
                ),
            }
        }
    }
}

#[test]
fn qa_word_symbol_at_k_is_the_reduced_sequence() {
    check_symbols(reduce);
}

negative_control!(
    qa_word_symbol_at_k_is_the_reduced_sequence,
    "the sequence as pushed, with no cancellation",
    expected = "the symbol at",
    check_symbols(|q| (q.to_vec(), q.len() > 76))
);

// ── REQ-TOOL-154: the ternary masses ────────────────────────────────────────────────────────────────────────────────

fn check_ternary(scale: fn([f64; 3]) -> f64) {
    let masses: [[f32; 3]; 7] = [
        [1.0 / 3.0; 3],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.5, 0.3, 0.2],
        [0.2, 0.2, 0.6],
        [0.1, 0.45, 0.45],
    ];
    let id = format!(
        "debug/reductions/{}",
        ledger::gen::catalogue::MASSES_TERNARY
    );
    let mut s = scene(masses.len() as u32, view(&id, &[]));
    for (i, m) in (0u32..).zip(&masses) {
        set_masses(&mut s, i, *m);
    }
    let img = render(&gpu(), &s);
    for (i, m) in (0u32..).zip(&masses) {
        let m = m.map(f64::from);
        let k = scale(m);
        tile(
            &s,
            &img,
            i,
            TOL,
            &|_| m.map(|x| x / k),
            "(m0, m1, m2)/max mᵢ",
        );
    }
    for (i, c) in [[1.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        .into_iter()
        .enumerate()
    {
        tile(&s, &img, i as u32, TOL, &|_| c, "white or a primary");
    }
}

#[test]
fn qa_ternary_masses_scaled_by_max() {
    check_ternary(|m| m[0].max(m[1]).max(m[2]));
}

negative_control!(
    qa_ternary_masses_scaled_by_max,
    "the masses scaled by their sum",
    expected = "(m0, m1, m2)/max mᵢ",
    check_ternary(|m| m.iter().sum())
);

// ── REQ-VAL-010, REQ-TOOL-157: both reductions of every vector field ────────────────────────────────────────────────

fn vector_fields() -> Vec<&'static str> {
    ledger::gen::validate(&ledger::layout())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| matches!(e.ty, FieldType::Vector { .. }))
        .map(|e| e.name)
        .collect()
}

/// Four configurations: `A`, `A` rotated by 90° about the origin (each body's `(x, y)` to `(−y, x)`: the same norm,
/// another direction), `2A` (the same direction, another norm), and `B`, another of both.
fn vectors() -> [[[f32; 2]; 3]; 4] {
    let a = [[0.6f32, -0.2], [-0.45, 0.55], [-0.15, -0.35]];
    let rot = a.map(|[x, y]| [-y, x]);
    let dbl = a.map(|b| b.map(|x| 2.0 * x));
    let b = [[0.1f32, 0.9], [0.3, -0.05], [-0.7, 0.2]];
    [a, rot, dbl, b]
}

fn fill_vector(s: &mut Scene, i: u32, field: &str, v: [[f32; 2]; 3]) {
    let mut b = s.set.sample(i);
    match field {
        "r" => b.r(v),
        "p" => b.p(v),
        "r_sh" => b.r_sh(v),
        "p_sh" => b.p_sh(v),
        other => panic!("no setter for the vector field `{other}`"),
    };
}

fn differ(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).any(|(x, y)| (x - y).abs() > 1e-3)
}

fn same(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// For every vector field, its '‖·‖ as scalar' view's colour depends on the norm alone and its 'as direction-cosines'
/// view's on the direction alone; `r`'s, `p`'s and the shadows' cosines draw each body's squared share over the
/// largest, the bodies red, green, blue (REQ-TOOL-157); `n`'s scalar view is one colour over every shape, as `‖n‖ = 1`.
/// `share` is the k = 6 reference.
fn check_reductions(share: fn([[f32; 2]; 3]) -> Rgb) {
    let fields = vector_fields();
    for f in ["n", "r", "p"] {
        assert!(
            fields.contains(&f),
            "`{f}` is no vector field of the ledger"
        );
    }
    let h = gpu();
    for field in &fields {
        let [(_, scalar), (_, dircos)] = registry::vector_reductions(field);
        let (s_scalar, s_dir) = if *field == "n" {
            let cases = shape_cases();
            (
                shape_scene(view(&scalar, &[]), &cases),
                shape_scene(view(&dircos, &[]), &cases),
            )
        } else {
            let mut ss = scene(4, view(&scalar, &[]));
            let mut sd = scene(4, view(&dircos, &[]));
            for (i, v) in (0u32..).zip(vectors()) {
                fill_vector(&mut ss, i, field, v);
                fill_vector(&mut sd, i, field, v);
            }
            (ss, sd)
        };
        let (is, id) = (render(&h, &s_scalar), render(&h, &s_dir));
        let c = |s: &Scene, img: &[[f32; 4]], i: u32| colour_at(s, img, i);
        if *field == "n" {
            let first = c(&s_scalar, &is, 0);
            for i in 1..shape_cases().len() as u32 {
                assert!(
                    same(c(&s_scalar, &is, i), first),
                    "`n`'s '‖·‖ as scalar' differs at sample {i}"
                );
            }
            assert!(
                differ(c(&s_dir, &id, 0), c(&s_dir, &id, 1)),
                "`n`'s direction cosines do not vary"
            );
            continue;
        }
        assert!(
            same(c(&s_scalar, &is, 0), c(&s_scalar, &is, 1)),
            "`{field}`'s '‖·‖ as scalar' sees a rotation"
        );
        assert!(
            differ(c(&s_scalar, &is, 0), c(&s_scalar, &is, 2)),
            "`{field}`'s '‖·‖ as scalar' misses 2v"
        );
        assert!(
            same(c(&s_dir, &id, 0), c(&s_dir, &id, 2)),
            "`{field}`'s direction cosines see 2v"
        );
        assert!(
            differ(c(&s_dir, &id, 0), c(&s_dir, &id, 3)),
            "`{field}`'s direction cosines miss another direction"
        );
        for (i, v) in (0u32..).zip(vectors()) {
            let want = share(v);
            tile(
                &s_dir,
                &id,
                i,
                TOL,
                &|_| want,
                &format!("`{field}`'s per-body squared shares"),
            );
        }
    }
}

/// Each body's squared norm over the largest.
fn shares_over_max(v: [[f32; 2]; 3]) -> Rgb {
    let w = v.map(|b| f64::from(b[0]).powi(2) + f64::from(b[1]).powi(2));
    let top = w[0].max(w[1]).max(w[2]);
    w.map(|x| x / top)
}

#[test]
fn qa_vector_fields_offer_both_reductions() {
    check_reductions(shares_over_max);
}

negative_control!(
    qa_vector_fields_offer_both_reductions,
    "the shares over their sum",
    expected = "per-body squared shares",
    check_reductions(|v| {
        let w = v.map(|b| f64::from(b[0]).powi(2) + f64::from(b[1]).powi(2));
        let sum: f64 = w.iter().sum();
        w.map(|x| x / sum)
    })
);

// ── REQ-VAL-011: t_dmin_step through the fragment ───────────────────────────────────────────────────────────────────

/// `t_dmin_step` values over every bit, each beside a `t_end_step`, read in the fragment from the packed `times`.
fn check_t_dmin(mask: u32) {
    let mut vals: Vec<u32> = vec![
        0, 1, 0x7fff, 0x8000, 0xfffe, 0xffff, 0x00ff, 0xff00, 0x5555, 0xaaaa,
    ];
    vals.extend((0..16).map(|b| 1u32 << b));
    vals.extend((0..16).map(|b| 0xffff ^ (1u32 << b)));
    let h = gpu();
    for (what, e) in [
        ("t_dmin_step", "f32(ctx.sample.t_dmin_step)"),
        ("t_end_step", "f32(ctx.sample.t_end_step)"),
    ] {
        let mut s = scene(vals.len() as u32, expr(e));
        for (i, v) in (0u32..).zip(&vals) {
            s.set.sample(i).times(!v & 0xffff, *v & mask);
        }
        let img = render(&h, &s);
        for (i, v) in (0u32..).zip(&vals) {
            let want = if what == "t_dmin_step" {
                *v
            } else {
                !v & 0xffff
            };
            assert_eq!(
                red(&s, &img, i),
                want as f32,
                "{what} of {v:#06x} does not round-trip"
            );
        }
    }
}

#[test]
fn qa_t_dmin_step_roundtrips_in_the_fragment() {
    check_t_dmin(0xffff);
}

negative_control!(
    qa_t_dmin_step_roundtrips_in_the_fragment,
    "a 15-bit step index",
    expected = "does not round-trip",
    check_t_dmin(0x7fff)
);

// ── REQ-TOOL-010: drift running-max vs final ────────────────────────────────────────────────────────────────────────

/// The drift max-vs-final view, `u_quantity` 0 (energy), on states whose current drift `ΔE = H − E_0` is at most its
/// running maximum `dE_max`, as a running maximum of `|ΔE|` holds it (payload §5): a secular drift (final = max), two
/// recovered transients (final = max/2, max/8) and a full recovery (final = 0) draw four distinct colours, and a final
/// drift of either sign the same one. `fold` maps each fraction to the one set, for the control.
fn check_drift_max_vs_final(fold: fn(f64) -> f64) {
    let r = [[0.6f32, -0.2], [-0.45, 0.55], [-0.15, -0.35]];
    let p = [[0.1f32, 0.3], [-0.25, -0.05], [0.15, -0.25]];
    let m = [0.4f32, 0.35, 0.25];
    let f = |x: f32| f64::from(x);
    let k: f64 = (0..3)
        .map(|j| 0.5 * (f(p[j][0]).powi(2) + f(p[j][1]).powi(2)) / f(m[j]))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| {
            -f(m[a]) * f(m[b]) / (f(r[a][0]) - f(r[b][0])).hypot(f(r[a][1]) - f(r[b][1]))
        })
        .sum();
    let h = k + v;
    // dE_max 0.0625, an f16 exactly; each final drift a power-of-two fraction of it, each sign.
    let max = 0.0625f32;
    let fractions = [1.0, 0.5, 0.125, 0.0, -1.0, -0.5, -0.125];
    let mut s = scene(
        fractions.len() as u32,
        view(
            "debug/accumulators/drift_max_vs_final",
            &[("u_quantity", 0.0)],
        ),
    );
    for (i, fr) in (0u32..).zip(fractions) {
        let e0 = (h - fold(fr) * f64::from(max)) as f32;
        s.set
            .sample(i)
            .r(r)
            .p(p)
            .E_0(e0)
            .times(50, 0)
            .drift_max(max, max);
        set_masses(&mut s, i, m);
    }
    let img = render(&gpu(), &s);
    for a in 0..4u32 {
        for b in a + 1..4 {
            assert!(
                differ(colour_at(&s, &img, a), colour_at(&s, &img, b)),
                "final drifts {} and {} of the running maximum draw one colour",
                fractions[a as usize],
                fractions[b as usize]
            );
        }
    }
    for (a, b) in [(0u32, 4u32), (1, 5), (2, 6)] {
        assert!(
            !differ(colour_at(&s, &img, a), colour_at(&s, &img, b)),
            "final drifts {} and {} of the running maximum, one magnitude, draw two colours",
            fractions[a as usize],
            fractions[b as usize]
        );
    }
}

#[test]
fn qa_accumulator_drift_max_vs_final_tells_secular_from_transient() {
    check_drift_max_vs_final(|x| x);
}

negative_control!(
    qa_accumulator_drift_max_vs_final_tells_secular_from_transient,
    "every final drift at the running maximum, as the showcase's states saturate",
    expected = "draw one colour",
    check_drift_max_vs_final(|x| if x == 0.0 { 1.0 } else { x.signum() })
);
