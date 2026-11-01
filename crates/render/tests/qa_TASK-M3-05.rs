//! QA tests for TASK-M3-05, from REQ-TOOL-149, R-379, R-381 and render contract Part 5, "The drift views and the suspect
//! styling", not from the implementation. The diverging ramp, its 8-bit colours, the placements and the styling are
//! written out here; only the prior tasks' viridis and hatch (`present::ramp_viridis`, `present::debug_invalid`) draw a
//! reference colour. They cover what the definition states beyond the default `symlog` and `lin`:
//! - a drift of exactly 0 draws the neutral `#DDDDDD` (`qa_drift_zero_is_the_neutral`);
//! - `lin` through the palette swap is on viridis, and `log` ignores `PALETTE` (`qa_drift_lin_swap_and_log_palette`);
//! - each view's floor and range are its own uniforms: `EPS_L` places the L_z view, `EPS_E` only its energy predicate
//!   (`qa_drift_uniforms_are_the_fields`);
//! - the hatch under every compaction and palette, in all three views, never styled (`qa_drift_hatch_every_mode`);
//! - the rows and columns on every ramp, in ink by luminance, both a grid (`qa_drift_styling_every_mode`);
//! - in the shader, the energy predicate reads the relative drift at the view's `EPS_E`, the L_z one the absolute latch
//!   (`qa_drift_predicates_in_the_shader`).
//!
//! **Domain** (pitfalls §10): dyadic states, so `L_z` is exact, and energy drifts of magnitude at least 1/16, where the
//! f32 rounding of `H` moves a colour by less than the tolerance.

use ledger::constants::{
    DRIFT_LIN_R_E, DRIFT_LIN_R_LZ, EPS_E_DEFAULT, EPS_L_DEFAULT, SUSPECT_ENERGY_THRESHOLD,
    SUSPECT_LZ_THRESHOLD,
};
use render::present::{self, Rgb};
use render::registry;
use validation::golden_scene::{row_scene, Colouring, DebugCase, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

const TOL: f64 = 1e-5;
const R: [[f32; 2]; 3] = [[0.625, -0.25], [-0.5, 0.5], [-0.125, -0.25]];
const P: [[f32; 2]; 3] = [[0.125, 0.25], [-0.25, -0.0625], [0.125, -0.1875]];

/// The sRGB decode of an 8-bit colour, written out.
fn hex(c: [u8; 3]) -> Rgb {
    c.map(|v| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// `diverging(c−, c0, c+)` at `t`, mixed in linear RGB, `c0` at 1/2: the contract's three colours.
fn diverging(t: f64) -> Rgb {
    let (lo, mid, hi) = (
        hex([0x3B, 0x4C, 0xC0]),
        hex([0xDD, 0xDD, 0xDD]),
        hex([0xB4, 0x04, 0x26]),
    );
    let (a, b, u) = if t < 0.5 {
        (lo, mid, 2.0 * t)
    } else {
        (mid, hi, 2.0 * t - 1.0)
    };
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * u)
}

fn symlog_place(x: f64, eps: f64) -> f64 {
    let s = (1.0 + x.abs() / eps).ln().copysign(x);
    0.5 + 0.5 * s / (1.0 + s.abs())
}

fn lin_place(x: f64, r: f64) -> f64 {
    ((x + r) / (2.0 * r)).clamp(0.0, 1.0)
}

fn log_place(x: f64, eps: f64) -> f64 {
    1.0 - 1.0 / (1.0 + (1.0 + x.abs() / eps).ln())
}

/// Rows for `energy`, columns for `lz`, each every fourth pixel, in ink by the linear Rec. 709 luminance of `rgb`.
fn styled(rgb: Rgb, energy: bool, lz: bool, x: u32, y: u32) -> Rgb {
    if (energy && y.is_multiple_of(4)) || (lz && x.is_multiple_of(4)) {
        let l = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
        [if l >= 0.18 { 0.0 } else { 1.0 }; 3]
    } else {
        rgb
    }
}

const ENERGY: &str = "debug/generated/energy_drift";
const LZ: &str = "debug/generated/Lz_drift";
const DERIVED: &str = "debug/derived/energy_drift";

fn params(p: &[(&str, f64)]) -> Vec<(String, Vec<f64>)> {
    p.iter().map(|(k, v)| ((*k).to_owned(), vec![*v])).collect()
}

fn view(id: &str, p: &[(&str, f64)]) -> Colouring {
    Colouring::Debug(Box::leak(Box::new(DebugCase {
        name: format!("qa {id}"),
        id: id.to_owned(),
        params: params(p),
        nudge: 0,
    })))
}

fn source(id: &str) -> String {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
        .source
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn fill(s: &mut Scene, i: u32, e_0: f32, lz_0: f32, de_max: f32, dlz_max: f32) {
    s.set
        .sample(i)
        .r(R)
        .p(P)
        .E_0(e_0)
        .Lz_0(lz_0)
        .times(1000, 500)
        .drift_max(de_max, dlz_max);
}

/// `(H, L_z)` of the tile's state, by the scene's masses; `L_z` is `Σ(x p_y − y p_x)` written out, exact in dyadics.
fn invariants(s: &mut Scene, i: u32) -> (f32, f32) {
    fill(s, i, 0.0, 0.0, 0.0, 0.0);
    let read = s.read(i);
    let lz: f32 = (0..3).map(|k| R[k][0] * P[k][1] - R[k][1] * P[k][0]).sum();
    assert_eq!(read.Lz_drift, lz, "the read side's L_z is not the hand one");
    (read.energy_drift, lz)
}

/// A scene of tiles with the drifts `xs` of the view `id` shows, no predicate lit.
fn drifts(id: &str, xs: &[f64], colouring: Colouring) -> Scene {
    let mut s = row_scene("qa drift", xs.len() as u32, colouring).unwrap_or_else(|e| panic!("{e}"));
    for (i, x) in (0u32..).zip(xs) {
        let (h, lz) = invariants(&mut s, i);
        if id == LZ {
            fill(&mut s, i, h, lz - *x as f32, 0.0, 0.0);
        } else {
            fill(&mut s, i, h - *x as f32, lz, 0.0, 0.0);
        }
    }
    s
}

fn image(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

fn check(s: &Scene, img: &[Rgb], i: u32, want: &dyn Fn(u32, u32) -> Rgb, what: &str) {
    let width = s.size().0;
    for (x, y) in s.pixels(i) {
        let (got, c) = (img[(y * width + x) as usize], want(x, y));
        assert!(
            got.iter().zip(&c).all(|(a, b)| (a - b).abs() <= TOL),
            "pixel ({x}, {y}) of sample {i} is {got:?}, not {what} {c:?}"
        );
    }
}

// ── Zero, the neutral ────────────────────────────────────────────────────────────────────────────────────────────

fn check_zero(h: &GpuHarness, colouring: Colouring, want: Rgb) {
    let s = drifts(LZ, &[0.0], colouring);
    check(&s, &image(h, &s), 0, &|_, _| want, "the place of 0");
}

#[test]
fn qa_drift_zero_is_the_neutral() {
    let h = gpu();
    let c0 = hex([0xDD, 0xDD, 0xDD]);
    check_zero(&h, view(LZ, &[]), c0);
    check_zero(&h, view(LZ, &[("COMPACTION", 1.0)]), c0);
    check_zero(
        &h,
        view(LZ, &[("PALETTE", 1.0)]),
        present::ramp_viridis(0.5),
    );
    check_zero(
        &h,
        view(LZ, &[("COMPACTION", 2.0)]),
        present::ramp_viridis(0.0),
    );
}

negative_control!(
    qa_drift_zero_is_the_neutral,
    "the neutral isn't viridis's middle",
    expected = "the place of 0",
    check_zero(&gpu(), view(LZ, &[]), present::ramp_viridis(0.5))
);

// ── `lin` under the swap; `log` under either palette ─────────────────────────────────────────────────────────────

fn the_view_r(id: &str) -> f64 {
    if id == LZ {
        DRIFT_LIN_R_LZ.number()
    } else {
        DRIFT_LIN_R_E.number()
    }
}

fn check_lin_swap(h: &GpuHarness, id: &str, colouring: Colouring) {
    let r = the_view_r(id);
    let xs = [-2.0 * r, -0.5 * r, 0.5 * r, 2.0 * r];
    let s = drifts(id, &xs, colouring);
    let img = image(h, &s);
    for (i, x) in (0u32..).zip(xs) {
        let want = present::ramp_viridis(lin_place(x, r));
        check(&s, &img, i, &|_, _| want, "`lin` on viridis");
    }
}

#[test]
fn qa_drift_lin_swap_and_log_palette() {
    let h = gpu();
    for id in [ENERGY, LZ, DERIVED] {
        check_lin_swap(&h, id, view(id, &[("COMPACTION", 1.0), ("PALETTE", 1.0)]));
        let r = the_view_r(id);
        let xs = [-r, r / 2.0];
        let eps = if id == LZ {
            EPS_L_DEFAULT.number()
        } else {
            EPS_E_DEFAULT.number()
        };
        for palette in [0.0, 1.0] {
            let s = drifts(
                id,
                &xs,
                view(id, &[("COMPACTION", 2.0), ("PALETTE", palette)]),
            );
            let img = image(&h, &s);
            for (i, x) in (0u32..).zip(xs) {
                let want = present::ramp_viridis(log_place(x, eps));
                check(&s, &img, i, &|_, _| want, "`log` on viridis");
            }
        }
    }
}

negative_control!(
    qa_drift_lin_swap_and_log_palette,
    "`lin` with no swap draws diverging",
    expected = "`lin` on viridis",
    check_lin_swap(&gpu(), LZ, view(LZ, &[("COMPACTION", 1.0)]))
);

// ── The fields' own uniforms ─────────────────────────────────────────────────────────────────────────────────────

fn check_lz_floor(h: &GpuHarness, colouring: Colouring, eps: f64) {
    let xs = [-1.0 / 64.0, 1.0 / 256.0];
    let s = drifts(LZ, &xs, colouring);
    let img = image(h, &s);
    for (i, x) in (0u32..).zip(xs) {
        let want = diverging(symlog_place(x, eps));
        check(&s, &img, i, &|_, _| want, "`symlog` at the field's floor");
    }
}

#[test]
fn qa_drift_uniforms_are_the_fields() {
    let h = gpu();
    // `ε_L` places the L_z view; `ε_E` doesn't.
    check_lz_floor(&h, view(LZ, &[("EPS_L", 1e-3)]), 1e-3);
    check_lz_floor(&h, view(LZ, &[("EPS_E", 1e-3)]), EPS_L_DEFAULT.number());
    // `ε_E` places the energy views.
    for id in [ENERGY, DERIVED] {
        let xs = [-0.25, 0.125];
        let s = drifts(id, &xs, view(id, &[("EPS_E", 1e-3)]));
        let img = image(&h, &s);
        for (i, x) in (0u32..).zip(xs) {
            let want = diverging(symlog_place(x, 1e-3));
            check(&s, &img, i, &|_, _| want, "`symlog` at the field's floor");
        }
    }
    // `LIN_R` is the range, each view's default the register's.
    for id in [ENERGY, LZ, DERIVED] {
        let r = the_view_r(id);
        let xs = [-r / 2.0, r / 4.0];
        let (dflt, wide) = (
            drifts(id, &xs, view(id, &[("COMPACTION", 1.0)])),
            drifts(
                id,
                &xs,
                view(id, &[("COMPACTION", 1.0), ("LIN_R", 2.0 * r)]),
            ),
        );
        let (a, b) = (image(&h, &dflt), image(&h, &wide));
        for (i, x) in (0u32..).zip(xs) {
            let want = diverging(lin_place(x, r));
            check(&dflt, &a, i, &|_, _| want, "`lin` at the register's R");
            let want = diverging(lin_place(x, 2.0 * r));
            check(&wide, &b, i, &|_, _| want, "`lin` at the view's LIN_R");
        }
    }
}

negative_control!(
    qa_drift_uniforms_are_the_fields,
    "the L_z view at another floor than it was given",
    expected = "the field's floor",
    check_lz_floor(&gpu(), view(LZ, &[("EPS_L", 1e-3)]), EPS_L_DEFAULT.number())
);

// ── The hatch ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The view's WGSL with its drift the absence NaN, by bits the shader can't fold.
fn absent(id: &str) -> String {
    let bits = format!(
        "bitcast<f32>({}u + uniforms.COMPACTION * 0u)",
        present::ABSENT_NAN_BITS
    );
    let src = source(id);
    let from = match id {
        ENERGY => "let x = ctx.sample.energy_drift;",
        LZ => "let x = ctx.sample.Lz_drift;",
        _ => "let x = value(ctx);",
    };
    assert!(src.contains(from), "`{id}` has no `{from}`");
    src.replace(from, &format!("let x = {bits};"))
}

/// Both predicates lit, so a styled hatch shows.
fn lit_row(colouring: Colouring) -> Scene {
    let mut s = row_scene("qa drift", 2, colouring).unwrap_or_else(|e| panic!("{e}"));
    for i in 0..2 {
        let (h, lz) = invariants(&mut s, i);
        fill(&mut s, i, h - 0.25, lz - 0.25, 1024.0, 1024.0);
    }
    s
}

fn check_hatch(h: &GpuHarness, s: &Scene) {
    let img = image(h, s);
    for i in 0..2 {
        check(
            s,
            &img,
            i,
            &|x, y| present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]),
            "the hatch",
        );
    }
}

#[test]
fn qa_drift_hatch_every_mode() {
    let h = gpu();
    for id in [ENERGY, LZ, DERIVED] {
        for c in [0.0, 1.0, 2.0] {
            for p in [0.0, 1.0] {
                let s = lit_row(Colouring::Probe(
                    absent(id),
                    params(&[("COMPACTION", c), ("PALETTE", p)]),
                ));
                check_hatch(&h, &s);
            }
        }
    }
}

negative_control!(
    qa_drift_hatch_every_mode,
    "a drift that is a value draws the ramp, styled",
    expected = "the hatch",
    check_hatch(&gpu(), &lit_row(view(ENERGY, &[("COMPACTION", 1.0)])))
);

// ── The styling on every ramp ────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Mode {
    Symlog,
    Swap,
    Lin,
    LinSwap,
    Log,
}

impl Mode {
    const ALL: [Mode; 5] = [
        Mode::Symlog,
        Mode::Swap,
        Mode::Lin,
        Mode::LinSwap,
        Mode::Log,
    ];

    fn params(self) -> &'static [(&'static str, f64)] {
        match self {
            Mode::Symlog => &[],
            Mode::Swap => &[("PALETTE", 1.0)],
            Mode::Lin => &[("COMPACTION", 1.0)],
            Mode::LinSwap => &[("COMPACTION", 1.0), ("PALETTE", 1.0)],
            Mode::Log => &[("COMPACTION", 2.0)],
        }
    }

    fn rgb(self, x: f64, r: f64, eps: f64) -> Rgb {
        match self {
            Mode::Symlog => diverging(symlog_place(x, eps)),
            Mode::Swap => present::ramp_viridis(symlog_place(x, eps)),
            Mode::Lin => diverging(lin_place(x, r)),
            Mode::LinSwap => present::ramp_viridis(lin_place(x, r)),
            Mode::Log => present::ramp_viridis(log_place(x, eps)),
        }
    }
}

/// Four tiles at a drift beyond `−R` (dark on viridis, so white ink) lit as energy, L_z, both and neither.
const LITS: [(bool, bool); 4] = [(true, false), (false, true), (true, true), (false, false)];

fn styling_scene(id: &str, x: f64, colouring: Colouring) -> Scene {
    let (tau_e, tau_l) = (
        SUSPECT_ENERGY_THRESHOLD.number() as f32,
        SUSPECT_LZ_THRESHOLD.number() as f32,
    );
    let mut s = row_scene("qa drift", 4, colouring).unwrap_or_else(|e| panic!("{e}"));
    for (i, (e, l)) in (0u32..).zip(LITS) {
        let (h, lz) = invariants(&mut s, i);
        let (e_0, lz_0) = if id == LZ {
            (h, lz - x as f32)
        } else {
            (h - x as f32, lz)
        };
        let de = e_0.abs().max(1e-6) * tau_e * if e { 4.0 } else { 0.25 };
        let dlz = tau_l * if l { 4.0 } else { 0.25 };
        fill(&mut s, i, e_0, lz_0, de, dlz);
    }
    s
}

fn check_styling(h: &GpuHarness, id: &str, mode: Mode, x: f64, swap: bool) {
    let (r, eps) = if id == LZ {
        (DRIFT_LIN_R_LZ.number(), EPS_L_DEFAULT.number())
    } else {
        (DRIFT_LIN_R_E.number(), EPS_E_DEFAULT.number())
    };
    let s = styling_scene(id, x, view(id, mode.params()));
    let img = image(h, &s);
    let rgb = mode.rgb(x, r, eps);
    for (i, (e, l)) in (0u32..).zip(LITS) {
        let (e, l) = if swap { (l, e) } else { (e, l) };
        check(
            &s,
            &img,
            i,
            &|px, py| styled(rgb, e, l, px, py),
            "the styled ramp",
        );
    }
}

#[test]
fn qa_drift_styling_every_mode() {
    let h = gpu();
    for id in [ENERGY, LZ, DERIVED] {
        let r = the_view_r(id);
        for mode in Mode::ALL {
            for x in [-2.0 * r, r] {
                check_styling(&h, id, mode, x, false);
            }
        }
    }
}

negative_control!(
    qa_drift_styling_every_mode,
    "rows read as the L_z predicate's",
    expected = "the styled ramp",
    check_styling(&gpu(), ENERGY, Mode::Log, 0.125, true)
);

// ── The predicates in the shader ─────────────────────────────────────────────────────────────────────────────────

/// `(E_0, dE_max, EPS_E, lit)` on the L_z view, whose placement doesn't read `E_0`, so only the style moves. `R` of the
/// row at the threshold's own scale: `τ = 2⁻⁸`.
fn check_energy_cases(h: &GpuHarness, cases: &[(f32, f32, f64, bool)]) {
    for &(e_0, de, eps, want) in cases {
        let x = -1.0 / 64.0;
        let mut s =
            row_scene("qa drift", 1, view(LZ, &[("EPS_E", eps)])).unwrap_or_else(|e| panic!("{e}"));
        let (_, lz) = invariants(&mut s, 0);
        fill(&mut s, 0, e_0, lz - x as f32, de, 0.0);
        let rgb = diverging(symlog_place(x, EPS_L_DEFAULT.number()));
        let img = image(h, &s);
        check(
            &s,
            &img,
            0,
            &|px, py| styled(rgb, want, false, px, py),
            "the energy predicate's style",
        );
    }
}

#[test]
fn qa_drift_predicates_in_the_shader() {
    let h = gpu();
    let t = SUSPECT_ENERGY_THRESHOLD.number() as f32;
    let eps = EPS_E_DEFAULT.number();
    check_energy_cases(
        &h,
        &[
            // One latch, lit at a small |E₀| and unlit at a large one, either sign: the relative drift.
            (1.0, 8.0 * t, eps, true),
            (-1.0, 8.0 * t, eps, true),
            (64.0, 8.0 * t, eps, false),
            (-64.0, 8.0 * t, eps, false),
            // A rest start's `E₀ = 0` reads the view's floor.
            (0.0, 8.0 * t * 1e-3, 1e-3, true),
            (0.0, t / 8.0 * 1e-3, 1e-3, false),
        ],
    );
    // The L_z predicate reads the latch alone: the same `dLz_max` lights any `Lz_0`.
    for lz_0_shift in [0.0f32, -1.0 / 64.0, 1.0 / 16.0] {
        let tau = SUSPECT_LZ_THRESHOLD.number() as f32;
        let mut s = row_scene("qa drift", 2, view(ENERGY, &[])).unwrap_or_else(|e| panic!("{e}"));
        for (i, (dlz, lit)) in (0u32..).zip([(4.0 * tau, true), (tau / 4.0, false)]) {
            let (h0, lz) = invariants(&mut s, i);
            fill(&mut s, i, h0 - 0.25, lz + lz_0_shift, 0.0, dlz);
            let rgb = diverging(symlog_place(0.25, eps));
            let img = image(&h, &s);
            check(
                &s,
                &img,
                i,
                &|px, py| styled(rgb, false, lit, px, py),
                "the L_z predicate's style",
            );
        }
    }
}

negative_control!(
    qa_drift_predicates_in_the_shader,
    "the case that is unlit read as lit",
    expected = "the energy predicate's style",
    check_energy_cases(
        &gpu(),
        &[(
            64.0,
            8.0 * SUSPECT_ENERGY_THRESHOLD.number() as f32,
            EPS_E_DEFAULT.number(),
            true
        )]
    )
);
