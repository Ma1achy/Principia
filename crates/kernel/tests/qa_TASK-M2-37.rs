//! qa's tests for TASK-M2-37, written from the four requirements it closes and chart_reference §2.1, each with its
//! negative control (R-176). Every decode is read back through the physics, not the chart's own targets: the masses,
//! `U = −Σ mᵢmⱼ/rᵢⱼ`, `I`, `K = Σ|pᵢ|²/2mᵢ` and `L_z` about the centre of mass are recomputed here from the decoded
//! `(m, r, p)` ([`phys`]), and each is held to the formula §2.1 writes for the axis.
//!
//! - `qa_energy_axes_warped_*` (REQ-CHART-067): every pixel of a 65 × 65 grid over `[0,1]²` validates and decodes to
//!   the system §2.1's warp names, on `(L_z, K)`, `(L_z, E)` (bare and under a mapping with both axes warped), the
//!   Burrau family's `(ν, K)` and `(θ, K)`, and a lone `K`, `KE`, `E`, `L_z` and virial axis beside a mass logit
//!   that moves `U`; a lone `E` decodes `E = U + K_max·t^γ_K` at the pixel's own `U`; R-508: on `(L_z, K)`, `(L_z, E)`
//!   and `(ν, K)` a window reaching `t = 1.5` decodes the warp continued (`K > K_max`) and validates, the `t` axis
//!   affine past 1 and bounded below 0, `s` bounded past both edges; a mass-ratio axis keeps every pixel's masses
//!   positive and summing to 1; under a mapping with every axis warped, an invariant chart decodes and checks as the
//!   chart, bitwise, over `[−¼, 1¼]²` at `γ_K = 2` and `3/2` ("bit-identical to before"; its control reads the mapped
//!   chart's rule path in place, which differs below `t = 0` at `γ_K = 3/2`). Controls (the verify's): a lone `E`
//!   mapped linearly onto `[U₀, U₀ + K_max]` fails at the corners; a lone `L_z` warp taken at the wrong held `K`
//!   leaves the feasible region; a `t` axis declared affine below 0 decodes a non-system there (γ_K = 3/2) or the fold
//!   of the `t > 0` side (γ_K = 2).
//! - `qa_energy_axis_linear_option_*` (REQ-CHART-068): `(L_z, E)` under the linear option: exactly the corners past
//!   `|L_z| = √(2I(E − U))` are not a system, their decodes `DEGENERATE` (never integrated), none warns, and each is
//!   projected by `validate` onto a feasible point (the navigation stop); untilted, the row `t = 1` and the point
//!   `(½, 0)` decode to the same state under both mappings, `L_z = 0` on `s = ½` under both, and the centre differs
//!   (γ_K = 2); R-408's count makes the corners forbidden and the linear axis affine past its edge; a linear mass ratio
//!   draws its non-positive ratios blank. Controls: a warped chart's would-be corners are integrated, a warning that
//!   fires on a linear chart, the centre asserted equal, and the warped extension types.
//! - `qa_energy_warp_per_pixel_*` (REQ-CHART-069): `(L_z, K)`, `(L_z, E)` and a lone `E` tilted into a mass logit and
//!   into a shape control at a single `τ ∈ {10°, 45°, 80°}` and at `(30°, 20°)` under Lean each axis validate every
//!   pixel and meet each pixel's own targets; under Tip as sheet at `(30°, 20°)` every pixel whose `(a, b)` lie in the
//!   square does; untilted the tilted chart is the chart, bitwise. Control: the warp computed once at the centre
//!   (`E = U₀ + K(t)`, `K* = E − U`) leaves pixels with `K < L_z²/2I`.
//! - `qa_pe_axis_*`, `qa_physical_axis_warps_*` (REQ-CHART-070): `PE` has no written warp and no extension type; the
//!   virial ratio warps as `Q_max·t^γ_K`, `K* = Q·|U|/2` at the pixel's `U`, bounded past both edges; the mass ratio
//!   warps geometrically through the active link, every pixel validating under `stick_breaking` and under
//!   `softmax_tanh` at both ends of `μ_max`'s allowed range. Controls: a virial `K*` read at the centre's `U`, and a
//!   ratio warp written for `softmax_tanh` at `μ_max = 5` alone. (The validator's refusal is the engine's qa file.)
//!
//! **Values.** `K_max` and `γ_K` are REQ-CHART-044's placeholders (R-71), `validation::{K_MAX, GAMMA_K}`. The decode's
//! `K` and `L_z` residuals are REQ-DEC-046's proposed tolerances (8 and 4 eps, as `qa_TASK-M2-09.rs` states them),
//! plus [`RND`], the rounding of this file's own f64 re-evaluation of `K` and `L_z` (a few products and a three-term
//! sum per quantity, bounded by 16 eps of the terms' absolute sum). A mass ratio is held to `ε_μ`, relative
//! (chart_reference §2.1: "realised … to within `ε_μ`"), a mass sum to 4 eps. Every slice centre, range, `Q_max`, held
//! value and angle here is test data.

use kernel::chart::energy::{
    count, decoder_bug_warning, extension, written, Count, ExtensionType, Mapped, Mapping,
    Quantity, Refused, Sides,
};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{ZBeta, ZMu1};
use kernel::chart::physical::{
    AxisE, AxisK, AxisKE, AxisLz, AxisVirial, EnergyAxis, MixedEnergy, MixedMassRatio,
};
use kernel::chart::tilt::Tiltable;
use kernel::chart::{
    Burrau, BurrauThetaK, Chart, InvariantLE, InvariantLK, Tilt, TiltMap, TiltSpec, Tilted,
    ValidationResult,
};
use kernel::decode::Decoded;
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Defaults, Link, LinkParams, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    StickBreaking,
};
use validation::{negative_control, tunables, GAMMA_K, K_MAX};

const EPS: f64 = f64::EPSILON;
/// `|K(p) − K*|` in units of `eps·K*` (REQ-DEC-046, proposed).
const TOL_K: f64 = 8.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))` (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// This file's own re-evaluation rounding, eps of the terms' absolute sum (the module's note).
const RND: f64 = 16.0;
/// The grid side, 65 points `i/64` per axis, edges included (the verify's 65 × 65).
const N: u32 = 65;

/// The selection with `stick_breaking` on the mass block.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

fn k_max() -> f64 {
    K_MAX.number()
}

fn gamma_k() -> f64 {
    GAMMA_K.number()
}

/// §2.1's `K(t) = K_max·t^γ`.
fn warp_k(gamma: f64, t: f64) -> f64 {
    k_max() * t.powf(gamma)
}

/// The 65 × 65 grid over `[u0, u1] × [v0, v1]`, edges included.
fn grid(u: [f64; 2], v: [f64; 2]) -> Vec<[f64; 2]> {
    let at = |r: [f64; 2], i: u32| r[0] + (r[1] - r[0]) * f64::from(i) / f64::from(N - 1);
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [at(u, i), at(v, j)]))
        .collect()
}

fn unit() -> Vec<[f64; 2]> {
    grid([0.0, 1.0], [0.0, 1.0])
}

/// A decoded system's physics, recomputed here from `(m, r, p)` about the centre of mass.
#[derive(Clone, Copy, Debug)]
struct Phys {
    m: [f64; 3],
    /// `U = −Σ mᵢmⱼ/rᵢⱼ`, `G = 1`.
    u: f64,
    /// `I = Σ mᵢ|rᵢ − r_cm|²`.
    i: f64,
    /// `K = Σ |pᵢ|²/2mᵢ`.
    k: f64,
    /// `L_z = Σ (xᵢ − x_cm) p_yᵢ − (yᵢ − y_cm) p_xᵢ`.
    lz: f64,
    /// `Σ |xᵢ p_yᵢ| + |yᵢ p_xᵢ|`, `L_z`'s rounding scale.
    lz_scale: f64,
    finite: bool,
}

fn phys(d: &Decoded<f64>) -> Phys {
    let ic = &d.ic;
    let m = ic.m;
    let total: f64 = m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|j| m[j] * ic.r[j][a]).sum::<f64>() / total);
    let rel = |j: usize| [ic.r[j][0] - c[0], ic.r[j][1] - c[1]];
    let i = (0..3)
        .map(|j| {
            let r = rel(j);
            m[j] * (r[0] * r[0] + r[1] * r[1])
        })
        .sum();
    let lz = (0..3)
        .map(|j| {
            let r = rel(j);
            r[0] * ic.p[j][1] - r[1] * ic.p[j][0]
        })
        .sum();
    let lz_scale = (0..3)
        .map(|j| {
            let r = rel(j);
            (r[0] * ic.p[j][1]).abs() + (r[1] * ic.p[j][0]).abs()
        })
        .sum();
    let k = (0..3)
        .map(|j| (ic.p[j][0].powi(2) + ic.p[j][1].powi(2)) / (2.0 * m[j]))
        .sum();
    let mut u = 0.0;
    for a in 0..3 {
        for b in (a + 1)..3 {
            let d = ((ic.r[a][0] - ic.r[b][0]).powi(2) + (ic.r[a][1] - ic.r[b][1]).powi(2)).sqrt();
            u -= m[a] * m[b] / d;
        }
    }
    let finite = m
        .iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .all(|x| x.is_finite());
    Phys {
        m,
        u,
        i,
        k,
        lz,
        lz_scale,
        finite,
    }
}

/// The targets a pixel must decode: `K*` and `L_z`.
#[derive(Clone, Copy, Debug)]
struct Want {
    k: f64,
    lz: f64,
}

/// One chart's pixels: its check, its decode, and the targets §2.1 gives a pixel from its own physics.
struct Pixels<'a> {
    name: &'a str,
    validate: &'a dyn Fn(f64, f64) -> ValidationResult,
    decode: &'a dyn Fn(f64, f64) -> Decoded<f64>,
    want: &'a dyn Fn(f64, f64, &Phys) -> Want,
}

/// Every point of `points` validates, decodes to a finite, non-`DEGENERATE` system, its targets are feasible
/// (`K* ≥ 0`, `L_z² ≤ 2IK*` at the pixel's own `I`) and the decode meets them to REQ-DEC-046's tolerance.
fn check_pixels(c: &Pixels, points: &[[f64; 2]]) {
    let name = c.name;
    let decoded: Vec<_> = points
        .iter()
        .map(|&[u, v]| {
            let d = (c.decode)(u, v);
            let p = phys(&d);
            ([u, v], d, p, (c.want)(u, v, &p))
        })
        .collect();
    // The targets first, over every point: a warp that leaves the feasible region anywhere is caught as that.
    for ([u, v], _, p, w) in &decoded {
        let slack = (TOL_K + RND) * EPS * w.k.abs();
        assert!(
            w.k >= -slack && w.lz * w.lz <= 2.0 * p.i * w.k * (1.0 + 64.0 * EPS) + slack,
            "{name}: pixel ({u}, {v}) is not its warp's system: its targets leave the feasible region, K* = {}, \
             L_z = {}, I = {}",
            w.k,
            w.lz,
            p.i
        );
    }
    for ([u, v], d, p, w) in &decoded {
        let valid = (c.validate)(*u, *v);
        assert!(
            valid == ValidationResult::Valid && d.degenerate.is_none() && p.finite,
            "{name}: pixel ({u}, {v}) is not its warp's system: {valid:?}, {:?}",
            d.degenerate
        );
        let k_err = (p.k - w.k).abs();
        let lz_err = (p.lz - w.lz).abs();
        assert!(
            k_err <= (TOL_K + RND) * EPS * w.k.abs() + f64::MIN_POSITIVE
                && lz_err
                    <= TOL_L * EPS * (w.lz.abs() + (2.0 * w.k.abs()).sqrt())
                        + RND * EPS * p.lz_scale
                        + f64::MIN_POSITIVE,
            "{name}: pixel ({u}, {v}) is not its warp's system: K = {}, want {}; L_z = {}, want {}",
            p.k,
            w.k,
            p.lz,
            w.lz
        );
    }
}

/// The invariant charts' slice centre (test data).
fn inv(gamma: f64) -> Invariant<f64> {
    Invariant {
        z_cfg: [-0.2, 0.4],
        z_mass: [0.5, -0.3],
        k_max: k_max(),
        gamma_k: gamma,
    }
}

/// The mixed charts' slice centre (test data): its `z_β = 0.3` and `z_μ₁ = 0.5` inside the horizontal range `[−1, 1]`
/// (the mass ratio beside `z_β`, the energy axes beside `z_μ₁`), so `U` rises and
/// falls about the centre's across the window.
const Z0: [f64; 8] = [-0.2, 0.3, 0.1, -0.2, 0.05, 0.15, 0.5, -0.3];
const RANGE: [f64; 2] = [-1.0, 1.0];
/// The virial axis's `Q_max` (test data).
const Q_MAX: f64 = 1.5;

/// A lone `Q` axis beside `z_μ₁` on [`RANGE`], its held value `held`, under `mapping`.
fn lone<Q: EnergyAxis>(
    held: f64,
    mapping: Mapping,
    gamma: f64,
) -> MixedEnergy<ZMu1, Q, Defaults, f64> {
    let top = if Q::QUANTITY == Quantity::VirialRatio {
        Q_MAX
    } else {
        k_max()
    };
    MixedEnergy::new(Z0, RANGE, top, gamma, held, mapping)
}

/// The `(L_z, K)` / `(L_z, E)` targets of §2.1's warp at `(s, t)`: `K(t)`, `(2s − 1)·√(2IK)` at the pixel's own `I`.
fn invariant_want(gamma: f64) -> impl Fn(f64, f64, &Phys) -> Want {
    move |s, t, p| {
        let k = warp_k(gamma, t);
        Want {
            k,
            lz: (2.0 * s - 1.0) * (2.0 * p.i * k).sqrt(),
        }
    }
}

/// A `t` axis's targets at the held `L_z = 0`: `K(t)`.
fn t_want(gamma: f64) -> impl Fn(f64, f64, &Phys) -> Want {
    move |_, t, _| Want {
        k: warp_k(gamma, t),
        lz: 0.0,
    }
}

/// `check_pixels` over `points` on `chart`'s own check and decode.
fn check_chart<C: Chart<Param = f64>>(
    name: &str,
    chart: &C,
    want: &dyn Fn(f64, f64, &Phys) -> Want,
    points: &[[f64; 2]],
) {
    let validate = |u, v| chart.validate(u, v);
    let decode = |u, v| chart.decode::<Defaults, f64>(u, v);
    check_pixels(
        &Pixels {
            name,
            validate: &validate,
            decode: &decode,
            want,
        },
        points,
    );
}

fn burrau(gamma: f64) -> Burrau<f64> {
    Burrau {
        nu_range: [0.2, 0.8],
        k_max: k_max(),
        gamma_k: gamma,
    }
}

fn theta_k(gamma: f64) -> BurrauThetaK<f64> {
    BurrauThetaK {
        theta_range: [0.15, 0.75],
        k_max: k_max(),
        gamma_k: gamma,
    }
}

// ── REQ-CHART-067 ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Every chart with an energy-type axis, warped, its every pixel of `[0,1]²` its warp's system. `lone_e` is the
/// mapping of the lone `E` axis (the default `Warped`; the control's `Linear`, mapped onto `[U₀, U₀ + K_max]`).
fn check_warped_every_pixel(lone_e: Mapping) {
    let g = gamma_k();
    let pts = unit();
    let iw = invariant_want(g);
    let tw = t_want(g);
    check_chart("(L_z, K)", &InvariantLK(inv(g)), &iw, &pts);
    check_chart("(L_z, E)", &InvariantLE(inv(g)), &iw, &pts);
    let warped = [Mapping::Warped; 2];
    check_chart(
        "(L_z, K) mapped warped",
        &Mapped::<_, Defaults>::new(InvariantLK(inv(g)), warped),
        &iw,
        &pts,
    );
    check_chart(
        "(L_z, E) mapped warped",
        &Mapped::<_, Defaults>::new(InvariantLE(inv(g)), warped),
        &iw,
        &pts,
    );
    check_chart("(ν, K)", &burrau(g), &tw, &pts);
    check_chart("(θ, K)", &theta_k(g), &tw, &pts);
    check_chart("lone K", &lone::<AxisK>(0.0, Mapping::Warped, g), &tw, &pts);
    check_chart(
        "lone KE",
        &lone::<AxisKE>(0.0, Mapping::Warped, g),
        &tw,
        &pts,
    );
    // A lone E: K* = E − U = K(t) at the pixel's own U, whatever the mapping claims.
    check_chart("lone E", &lone::<AxisE>(0.0, lone_e, g), &tw, &pts);
    // A lone L_z at the held K: L_z = (2t − 1)·√(2IK).
    let held = 0.6 * k_max();
    check_chart(
        "lone L_z",
        &lone::<AxisLz>(held, Mapping::Warped, g),
        &lone_lz_want(held, held),
        &pts,
    );
}

/// The lone `L_z` axis's targets: `K* = held`, `L_z = (2t − 1)·√(2·I·k_line)`, §2.1's line at `k_line`, the chart's
/// held `K` where right.
fn lone_lz_want(held: f64, k_line: f64) -> impl Fn(f64, f64, &Phys) -> Want {
    move |_, t, p| Want {
        k: held,
        lz: (2.0 * t - 1.0) * (2.0 * p.i * k_line).sqrt(),
    }
}

#[test]
fn qa_energy_axes_warped_every_pixel() {
    check_warped_every_pixel(Mapping::Warped);
}

negative_control!(
    qa_energy_axes_warped_every_pixel,
    "a lone E axis mapped linearly onto [U₀, U₀ + K_max] leaves pixels where the pixel's U exceeds U₀ at t = 0: not \
     its warp's system",
    expected = "is not its warp's system",
    check_warped_every_pixel(Mapping::Linear)
);

/// The lone `L_z` warp taken at the held-fixed `K` `k_line`.
fn check_lone_lz(k_line_factor: f64) {
    let held = 0.6 * k_max();
    check_chart(
        "lone L_z",
        &lone::<AxisLz>(held, Mapping::Warped, gamma_k()),
        &lone_lz_want(held, held * k_line_factor),
        &unit(),
    );
}

#[test]
fn qa_energy_axes_warped_lone_lz_held_k() {
    check_lone_lz(1.0);
}

negative_control!(
    qa_energy_axes_warped_lone_lz_held_k,
    "a lone L_z warp computed at twice the held K puts the edge pixels outside the feasible region",
    expected = "leave the feasible region",
    check_lone_lz(2.0)
);

/// A lone `E` decodes `E = U + K_max·t^γ_K`, `U` the pixel's own (`centre = false`) or, the control, the slice
/// centre's `U₀`; `E` is read from the decode as `K + U`.
fn check_lone_e(centre: bool) {
    let g = gamma_k();
    let chart = lone::<AxisE>(0.0, Mapping::Warped, g);
    let u0 = phys(&chart.decode::<Defaults, f64>(0.75, 0.0)).u; // z_μ₁ = 0.5, the centre's.
    let mut spread = (f64::INFINITY, f64::NEG_INFINITY);
    for [u, t] in unit() {
        let p = phys(&chart.decode::<Defaults, f64>(u, t));
        spread = (spread.0.min(p.u), spread.1.max(p.u));
        let e = p.k + p.u;
        let u_ref = if centre { u0 } else { p.u };
        let want = u_ref + warp_k(g, t);
        assert!(
            (e - want).abs() <= (TOL_K + RND) * EPS * (warp_k(g, t) + p.u.abs() + u_ref.abs()),
            "the lone E at ({u}, {t}) decodes E = {e}, not U + K_max·t^γ_K = {want}"
        );
    }
    assert!(
        spread.1 > spread.0,
        "U does not move across the window: {spread:?}"
    );
}

#[test]
fn qa_energy_axes_warped_lone_e() {
    check_lone_e(false);
}

negative_control!(
    qa_energy_axes_warped_lone_e,
    "E read against the slice centre's U₀, not the pixel's own U, is not U + K_max·t^γ_K",
    expected = "not U + K_max",
    check_lone_e(true)
);

/// R-508 on `(L_z, K)`, `(L_z, E)` and `(ν, K)`: a window reaching `t = 1.5` decodes the warp's formula continued and
/// validates; the declared types are §2.1's (`t` bounded below 0 and affine past 1, `s` bounded); past `s`'s edges and
/// below `t = 0` the check does not pass the point, and R-408 counts the affine side's pixels as systems. `cap` caps
/// the expected `t` at 1, the control's bounded `t > 1` side.
fn check_t_continues(cap: bool) {
    let g = gamma_k();
    let pts = grid([0.0, 1.0], [0.0, 1.5]);
    let t_of = move |t: f64| if cap { t.min(1.0) } else { t };
    let iw = move |s: f64, t: f64, p: &Phys| invariant_want(g)(s, t_of(t), p);
    let tw = move |s: f64, t: f64, p: &Phys| t_want(g)(s, t_of(t), p);
    check_chart("(L_z, K) to t = 1.5", &InvariantLK(inv(g)), &iw, &pts);
    check_chart("(L_z, E) to t = 1.5", &InvariantLE(inv(g)), &iw, &pts);
    check_chart("(ν, K) to t = 1.5", &burrau(g), &tw, &pts);
    let warped = [Mapping::Warped; 2];
    let t_side = Sides {
        below: ExtensionType::Bounded,
        above: ExtensionType::Affine,
    };
    let s_side = Sides::both(ExtensionType::Bounded);
    assert_eq!(
        Mapped::<_, Defaults>::new(InvariantLK(inv(g)), warped).extension(),
        [s_side, t_side]
    );
    assert_eq!(
        Mapped::<_, Defaults>::new(InvariantLE(inv(g)), warped).extension(),
        [s_side, t_side]
    );
    assert_eq!(
        Mapped::<_, Defaults>::new(burrau(g), warped).extension()[1],
        t_side
    );
    for q in [Quantity::K, Quantity::KE, Quantity::E] {
        assert_eq!(
            extension(q, Mapping::Warped),
            Ok(t_side),
            "{q:?}'s warped sides"
        );
    }
    assert_eq!(extension(Quantity::Lz, Mapping::Warped), Ok(s_side));
    let lk = InvariantLK(inv(g));
    let le = InvariantLE(inv(g));
    for &[u, v] in &[
        [-0.05, 0.5],
        [1.05, 0.5],
        [0.5, -0.05],
        [-0.2, 1.2],
        [1.3, 1.4],
    ] {
        assert!(
            lk.validate(u, v) != ValidationResult::Valid,
            "(L_z, K): ({u}, {v}) past a bounded side passes"
        );
        assert!(
            le.validate(u, v) != ValidationResult::Valid,
            "(L_z, E): ({u}, {v}) past a bounded side passes"
        );
    }
    for t in [-0.05, -0.5] {
        assert!(
            burrau(g).validate(0.5, t) != ValidationResult::Valid,
            "(ν, K): t = {t} passes"
        );
    }
    let sides = [s_side, t_side];
    assert_eq!(
        count(sides, [0.5, 1.4], true),
        Count::System,
        "t > 1 is not counted a system"
    );
    assert_eq!(count(sides, [1.1, 0.5], true), Count::DomainEnd);
    assert_eq!(count(sides, [-0.1, 1.2], true), Count::DomainEnd);
    assert_eq!(count(sides, [0.5, -0.1], true), Count::DomainEnd);
}

#[test]
fn qa_energy_axes_warped_t_continues() {
    check_t_continues(false);
}

negative_control!(
    qa_energy_axes_warped_t_continues,
    "a t > 1 side capped at K_max (bounded, not continuing) is not the decode past t = 1",
    expected = "is not its warp's system",
    check_t_continues(true)
);

/// Below `t = 0` on a lone `K` axis, the `(L_z, K)` chart and `(ν, K)` at `γ_K = gamma`: the side `declared` claims.
/// Declared bounded, the check does not pass the point. Declared affine, the formula continued past `t = 0` must give
/// a new system there: finite, not `DEGENERATE`, and not the system of `|t|`, which the `t > 0` side already draws.
/// A chart's name, check, decode and vertical axis's sides, for [`check_below_zero`].
type Arm<'a> = (
    &'a str,
    &'a dyn Fn(f64, f64) -> ValidationResult,
    &'a dyn Fn(f64, f64) -> Decoded<f64>,
    Sides,
);

fn check_below_zero(gamma: f64, declared: Option<ExtensionType>) {
    let lk = InvariantLK(inv(gamma));
    let lone_k = lone::<AxisK>(0.0, Mapping::Warped, gamma);
    let bu = burrau(gamma);
    let charts: [Arm; 3] = [
        (
            "(L_z, K)",
            &|u, v| lk.validate(u, v),
            &|u, v| lk.decode::<Defaults, f64>(u, v),
            Mapped::<_, Defaults>::new(lk, [Mapping::Warped; 2]).extension()[1],
        ),
        (
            "lone K",
            &|u, v| lone_k.validate(u, v),
            &|u, v| lone_k.decode::<Defaults, f64>(u, v),
            lone_k.extension(),
        ),
        (
            "(ν, K)",
            &|u, v| bu.validate(u, v),
            &|u, v| bu.decode::<Defaults, f64>(u, v),
            Mapped::<_, Defaults>::new(bu, [Mapping::Warped; 2]).extension()[1],
        ),
    ];
    for (name, validate, decode, sides) in charts {
        let below = declared.unwrap_or(sides.below);
        for &[u, t] in &[[0.3, -0.25], [0.5, -0.5], [0.7, -0.1]] {
            match below {
                ExtensionType::Bounded => assert!(
                    validate(u, t) != ValidationResult::Valid,
                    "{name}: t = {t} below the bounded side passes"
                ),
                _ => {
                    let d = decode(u, t);
                    let p = phys(&d);
                    assert!(
                        p.finite && d.degenerate.is_none() && p.k.is_finite(),
                        "{name}: affine below t = 0 decodes a non-system at ({u}, {t}): {:?}",
                        d.degenerate
                    );
                    let mirror = decode(u, -t);
                    assert!(
                        format!("{:?}", d.ic) != format!("{:?}", mirror.ic),
                        "{name}: affine below t = 0 folds ({u}, {t}) onto the t > 0 side's system"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_energy_axes_warped_below_zero() {
    check_below_zero(gamma_k(), None);
    check_below_zero(1.5, None);
}

negative_control!(
    qa_energy_axes_warped_below_zero,
    "a t axis declared affine below t = 0 folds onto the t > 0 side at an even integer γ_K",
    expected = "affine below t = 0 folds",
    check_below_zero(2.0, Some(ExtensionType::Affine))
);

negative_control!(
    qa_energy_axes_warped_below_zero_non_integer,
    "a t axis declared affine below t = 0 decodes a non-system there at a non-integer γ_K",
    expected = "affine below t = 0 decodes a non-system",
    check_below_zero(1.5, Some(ExtensionType::Affine))
);

/// A mass-ratio chart beside `z_β` (the ratio writes the mass block, so its partner is not a mass control).
fn ratio<S: Selection>(
    asked: [f64; 2],
    pair: [u32; 2],
    mapping: Mapping,
    p: &LinkParams<f64>,
) -> Option<MixedMassRatio<ZBeta, S, f64>> {
    MixedMassRatio::<ZBeta, S, f64>::new(Z0, RANGE, asked, pair, mapping, p)
}

/// A builder mass-ratio axis under `mapping` over the asked `[lo, hi]`: every pixel's masses positive and summing to 1.
/// `link` reads the masses the chart decodes, through the active link's inputs (§2.1); not `link`, the control, writes
/// the ratio's masses `m_a = (1 − m_c)/(1 + q)`, `m_b = q·m_a` straight into the physical masses.
fn check_ratio_simplex(mapping: Mapping, asked: [f64; 2], link: bool) {
    for pair in [[0, 1], [1, 2], [2, 0]] {
        let chart = ratio::<Defaults>(asked, pair, mapping, &LinkParams::DEFAULT)
            .expect("a realised range");
        for [u, v] in unit() {
            let m = if link {
                chart.decode::<Defaults, f64>(u, v).ic.m
            } else {
                let q = chart.ratio_at::<f64>(v);
                let m_a = (1.0 - chart.held) / (1.0 + q);
                let mut m = [chart.held; 3];
                m[pair[0] as usize] = m_a;
                m[pair[1] as usize] = q * m_a;
                m
            };
            assert!(
                m.iter().all(|&x| x > 0.0) && (m.iter().sum::<f64>() - 1.0).abs() <= 4.0 * EPS,
                "the mass ratio {pair:?} at ({u}, {v}) decodes masses {m:?}, not positive summing to 1"
            );
        }
    }
}

#[test]
fn qa_energy_axes_warped_mass_ratio() {
    check_ratio_simplex(Mapping::Warped, [0.05, 20.0], true);
    check_ratio_simplex(Mapping::Warped, [-3.0, 1e6], true);
    // Under the linear option too the decode's masses stay in the simplex: the link's inputs carry them.
    check_ratio_simplex(Mapping::Linear, [-3.0, 4.0], true);
}

negative_control!(
    qa_energy_axes_warped_mass_ratio,
    "the ratio written into the physical masses, not the link's inputs, over a range through zero reaches a \
     non-positive mass",
    expected = "not positive summing to 1",
    check_ratio_simplex(Mapping::Linear, [-3.0, 4.0], false)
);

// ── REQ-CHART-068 ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `(L_z, E)` under the mapping `mapping`, read as the linear option: `E = U₀ + K_max·t`, `L_z = (2s − 1)·√(2I₀K_max)`,
/// `I₀ = 1`, and a pixel is past the parabola where `L_z² > 2I₀(E − U₀)`. Each such corner pixel is not a system:
/// `validate` refuses it, its decode is `DEGENERATE` (never integrated), and `validate`'s projection (the navigation
/// stop) lands on a point it passes; every other pixel validates; there are corners; none warns (`warns`).
fn check_corners(mapping: [Mapping; 2], warns: &dyn Fn(&[Mapping]) -> bool) {
    let chart = Mapped::<_, Defaults>::new(InvariantLE(inv(gamma_k())), mapping);
    let lin = (2.0 * k_max()).sqrt();
    let mut corners = 0;
    for [s, t] in unit() {
        let (k, lz) = (k_max() * t, (2.0 * s - 1.0) * lin);
        // The parabola itself (`|L_z| = L_max`, the rigid rotation) is a system; a grid point on it, to the rounding
        // of this test's own `L_z²` and `2K`, is not classified here.
        let outside = lz * lz > 2.0 * k * (1.0 + 64.0 * EPS) + f64::MIN_POSITIVE;
        let inside = lz * lz < 2.0 * k * (1.0 - 64.0 * EPS);
        let valid = chart.validate(s, t);
        if outside {
            corners += 1;
            let d = chart.decode::<Defaults, f64>(s, t);
            assert!(
                valid != ValidationResult::Valid && d.degenerate.is_some(),
                "the corner pixel ({s}, {t}) is integrated: {valid:?}, {:?}",
                d.degenerate
            );
            let ValidationResult::Project { u, v, .. } = valid else {
                panic!("the corner pixel ({s}, {t}) has no navigation stop: {valid:?}");
            };
            assert!(
                chart.validate(u, v) == ValidationResult::Valid,
                "the navigation stop from ({s}, {t}) at ({u}, {v}) is not feasible"
            );
        } else if inside {
            assert!(
                valid == ValidationResult::Valid,
                "the feasible pixel ({s}, {t}) is refused: {valid:?}"
            );
        }
    }
    assert!(corners > 0, "the linear option leaves no corner");
    assert!(!warns(&mapping), "the linear chart's corners warn");
}

#[test]
fn qa_energy_axis_linear_option_corners() {
    check_corners([Mapping::Linear; 2], &decoder_bug_warning);
    for one in [
        [Mapping::Linear, Mapping::Warped],
        [Mapping::Warped, Mapping::Linear],
    ] {
        assert!(!decoder_bug_warning(&one), "{one:?} warns");
    }
    assert!(
        decoder_bug_warning(&[Mapping::Warped; 2]),
        "an all-warped chart's in-square failure does not warn"
    );
}

negative_control!(
    qa_energy_axis_linear_option_corners,
    "the warped chart's would-be corners are systems, integrated",
    expected = "is integrated",
    check_corners([Mapping::Warped; 2], &decoder_bug_warning)
);

negative_control!(
    qa_energy_axis_linear_option_corners_warn,
    "a warning that fires on the linear chart",
    expected = "corners warn",
    check_corners([Mapping::Linear; 2], &|_| true)
);

/// Untilted, `(L_z, E)` and `(L_z, K)` under both mappings decode the same state on the row `t = 1` and at `(½, 0)`,
/// and `L_z = 0` on the column `s = ½` under both; `extra` names further points asserted the same (the control's
/// centre).
fn check_shared(extra: &[[f64; 2]]) {
    let g = gamma_k();
    let mut points: Vec<[f64; 2]> = (0..N)
        .map(|i| [f64::from(i) / f64::from(N - 1), 1.0])
        .collect();
    points.push([0.5, 0.0]);
    points.extend_from_slice(extra);
    let same = |name: &str,
                w: &dyn Fn(f64, f64) -> Decoded<f64>,
                l: &dyn Fn(f64, f64) -> Decoded<f64>| {
        for &[s, t] in &points {
            let (a, b) = (phys(&w(s, t)), phys(&l(s, t)));
            let scale = TOL_L * EPS * (a.lz_scale + b.lz_scale)
                + RND * EPS * (a.k + b.k)
                + f64::MIN_POSITIVE;
            assert!(
                a.m == b.m && (a.u - b.u).abs() <= RND * EPS * a.u.abs() && (a.k - b.k).abs() <= scale
                    && (a.lz - b.lz).abs() <= scale + (2.0 * a.k).sqrt() * TOL_L * EPS,
                "{name}: ({s}, {t}) decodes another state under the two mappings: {a:?} against {b:?}"
            );
        }
        for j in 0..N {
            let t = f64::from(j) / f64::from(N - 1);
            for d in [w(0.5, t), l(0.5, t)] {
                let p = phys(&d);
                assert!(
                    p.lz.abs()
                        <= TOL_L * EPS * (2.0 * p.k).sqrt()
                            + RND * EPS * p.lz_scale
                            + f64::MIN_POSITIVE,
                    "{name}: L_z = {} on s = ½ at t = {t}",
                    p.lz
                );
            }
        }
    };
    let le = InvariantLE(inv(g));
    let le_lin = Mapped::<_, Defaults>::new(le, [Mapping::Linear; 2]);
    same(
        "(L_z, E)",
        &|s, t| le.decode::<Defaults, f64>(s, t),
        &|s, t| le_lin.decode::<Defaults, f64>(s, t),
    );
    let lk = InvariantLK(inv(g));
    let lk_lin = Mapped::<_, Defaults>::new(lk, [Mapping::Linear; 2]);
    same(
        "(L_z, K)",
        &|s, t| lk.decode::<Defaults, f64>(s, t),
        &|s, t| lk_lin.decode::<Defaults, f64>(s, t),
    );
}

#[test]
fn qa_energy_axis_linear_option_shared() {
    check_shared(&[]);
}

negative_control!(
    qa_energy_axis_linear_option_shared,
    "the centre (½, ½) is K_max·2^(−γ_K) warped against K_max/2 linear, another state at γ_K = 2",
    expected = "decodes another state",
    check_shared(&[[0.5, 0.5]])
);

/// R-408's count over a window `[−¼, 1¼]²` on `(L_z, E)` under the linear option, with the sides `sides`: every pixel
/// past the parabola is forbidden (in the count and the total), every other a system, none the domain's end.
fn check_linear_count(sides: [Sides; 2]) {
    let chart = Mapped::<_, Defaults>::new(InvariantLE(inv(gamma_k())), [Mapping::Linear; 2]);
    let lin = (2.0 * k_max()).sqrt();
    let mut forbidden = 0;
    for [s, t] in grid([-0.25, 1.25], [-0.25, 1.25]) {
        let (k, lz) = (k_max() * t, (2.0 * s - 1.0) * lin);
        let outside = k < 0.0 || lz * lz > 2.0 * k;
        let valid = chart.validate(s, t) == ValidationResult::Valid;
        let want = if outside {
            Count::Forbidden
        } else {
            Count::System
        };
        let got = count(sides, [s, t], valid);
        assert_eq!(
            got, want,
            "the linear (L_z, E) pixel ({s}, {t}) counts {got:?}, not {want:?}"
        );
        forbidden += usize::from(got == Count::Forbidden);
    }
    assert!(forbidden > 0);
}

#[test]
fn qa_energy_axis_linear_option_count() {
    let chart = Mapped::<_, Defaults>::new(InvariantLE(inv(gamma_k())), [Mapping::Linear; 2]);
    assert_eq!(chart.extension(), [Sides::both(ExtensionType::Affine); 2]);
    for q in Quantity::ALL.into_iter().filter(|&q| q != Quantity::PE) {
        assert_eq!(
            extension(q, Mapping::Linear),
            Ok(Sides::both(ExtensionType::Affine)),
            "{q:?} linear"
        );
    }
    check_linear_count(chart.extension());
}

negative_control!(
    qa_energy_axis_linear_option_count,
    "the warped sides make the linear axis's pixels past its edge the domain's end, not forbidden",
    expected = "counts DomainEnd",
    check_linear_count(
        Mapped::<_, Defaults>::new(InvariantLE(inv(gamma_k())), [Mapping::Warped; 2]).extension()
    )
);

/// A builder mass ratio under the linear option over `[−1, 3]`: `q(t) = −1 + 4t`, each pixel with `q ≤ 0` not a
/// system, each with `q > 0` a system with that ratio; some are blank.
fn check_ratio_blank(mapping: Mapping) {
    let chart =
        ratio::<Defaults>([-1.0, 3.0], [0, 1], mapping, &LinkParams::DEFAULT).expect("a chart");
    let mut blank = 0;
    for [u, v] in unit() {
        let q = chart.ratio_at(v);
        let valid = chart.validate(u, v) == ValidationResult::Valid;
        if q <= 0.0 {
            blank += 1;
            assert!(
                !valid,
                "the non-positive ratio {q} at ({u}, {v}) is a system"
            );
        } else {
            let m = chart.decode::<Defaults, f64>(u, v).ic.m;
            assert!(
                valid && (m[1] / m[0] - q).abs() <= <f64 as ChartConstants>::EPS_MU * q,
                "the ratio {q} at ({u}, {v}) is not drawn: {m:?}"
            );
        }
    }
    if mapping == Mapping::Linear {
        for (i, v) in [0.0, 0.1, 0.25, 0.5, 1.0].into_iter().enumerate() {
            let want: f64 = -1.0 + 4.0 * v;
            assert!(
                (chart.ratio_at::<f64>(v) - want).abs() <= 4.0 * EPS * want.abs().max(1.0),
                "linear q at {i}"
            );
        }
    }
    assert!(blank > 0, "no non-positive ratio is drawn blank");
}

#[test]
fn qa_energy_axis_linear_option_ratio() {
    check_ratio_blank(Mapping::Linear);
}

negative_control!(
    qa_energy_axis_linear_option_ratio,
    "the warped ratio over the same asked range reaches no non-positive ratio, so none is blank",
    expected = "no non-positive ratio",
    check_ratio_blank(Mapping::Warped)
);

// ── REQ-CHART-069 ─────────────────────────────────────────────────────────────────────────────────────────────────────

const CENTRE: [f64; 2] = [0.5, 0.5];
const SPAN: [f64; 2] = [2.0, 2.0];

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y: x,
    }
}

/// The tilts the verify names, into the target `x`: a single `τ ∈ {10°, 45°, 80°}` (as `τ₁` and as `τ₂`), and
/// `(30°, 20°)` under Lean each axis and under Tip as sheet.
fn tilts(x: u32) -> Vec<TiltSpec> {
    let mut all = Vec::new();
    for t in [10.0, 45.0, 80.0] {
        all.push(spec(t, 0.0, TiltMap::Sheet, x));
        all.push(spec(0.0, t, TiltMap::Sheet, x));
        all.push(spec(t, 0.0, TiltMap::Lean, x));
    }
    all.push(spec(30.0, 20.0, TiltMap::Lean, x));
    all.push(spec(30.0, 20.0, TiltMap::Sheet, x));
    all
}

/// `chart` tilted by `s`: every pixel of the 65 × 65 grid over `[0,1]²` whose tilted `(a, b)` lie in the square (all
/// of them but under Tip as sheet with both angles set) validates and decodes its targets at its own `U`, `I` and
/// masses; `energy` whether the vertical axis is `E`, `lz_axis` whether the horizontal is `L_z`.
fn check_tilted<C>(name: &str, chart: C, s: &TiltSpec, energy: bool, lz_axis: bool, centre: bool)
where
    C: Tiltable<Param = f64> + Copy,
{
    let g = gamma_k();
    let view = Tilted::new(chart, Tilt::new(s, CENTRE, SPAN)).expect("a hidden target");
    let u0 = phys(&chart.decode::<Defaults, f64>(0.5, 0.0)).u;
    let both = s.tau1 != 0.0 && s.tau2 != 0.0 && s.map == TiltMap::Sheet;
    let mut inside = 0;
    let mut points = Vec::new();
    for [u, v] in unit() {
        let ([a, b], shift) = view.at(u, v);
        assert!(shift.is_some(), "{name}: untilted at ({u}, {v})");
        let in_square = (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b);
        assert!(
            in_square || both,
            "{name}: ({u}, {v}) leaves the square at ({a}, {b}) under {s:?}"
        );
        if in_square {
            inside += 1;
            points.push([u, v]);
        }
    }
    assert!(
        inside > N * N / 4,
        "{name}: only {inside} pixels in the square"
    );
    let want = |u: f64, v: f64, p: &Phys| {
        let ([a, b], _) = view.at(u, v);
        let k = warp_k(g, b);
        // The control's warp computed once at the centre: its E, read against the pixel's U; L_max is the centre's
        // √(2I₀K(t)), I₀ = 1.
        let k_star = if centre { u0 + k - p.u } else { k };
        let k_star = if energy { k_star } else { k };
        let lz = if lz_axis {
            (2.0 * a - 1.0) * (2.0 * k).sqrt()
        } else {
            0.0
        };
        Want { k: k_star, lz }
    };
    let validate = |u, v| view.validate(u, v);
    let decode = |u, v| view.decode::<Defaults, f64>(u, v);
    let label = format!("{name} tilted {s:?}");
    check_pixels(
        &Pixels {
            name: &label,
            validate: &validate,
            decode: &decode,
            want: &want,
        },
        &points,
    );
}

/// The tilted charts, their targets read at each pixel's own potential, or (`centre`, the control) with the warp
/// computed once at the centre: `E = U₀ + K(t)` and `L_max = √(2I₀K(t))`, `K* = E − U` at the pixel.
fn check_per_pixel(centre: bool) {
    let g = gamma_k();
    // Into a mass logit (z_μ₁, z_μ₂) and into a shape control (z_α, z_β).
    for x in [6, 7, 0, 1] {
        for s in tilts(x) {
            check_tilted("(L_z, E)", InvariantLE(inv(g)), &s, true, true, centre);
            if !centre {
                check_tilted("(L_z, K)", InvariantLK(inv(g)), &s, false, true, centre);
            }
        }
    }
    // A lone E beside z_μ₁, tilted into z_α and z_μ₂: its hidden coordinates.
    for x in [0, 7] {
        for s in tilts(x) {
            check_tilted(
                "lone E",
                lone::<AxisE>(0.0, Mapping::Warped, g),
                &s,
                true,
                false,
                centre,
            );
        }
    }
}

#[test]
fn qa_energy_warp_per_pixel_tilted() {
    check_per_pixel(false);
}

negative_control!(
    qa_energy_warp_per_pixel_tilted,
    "the warp computed once at the centre leaves pixels with K < L_z²/2I",
    expected = "leave the feasible region",
    check_per_pixel(true)
);

/// Untilted (both angles 0), the tilted chart is the chart, bitwise, its check and its decode; `nudge` tilts by a
/// hair instead (the control).
fn check_untilted(nudge: f64) {
    let g = gamma_k();
    fn same<C: Tiltable<Param = f64> + Copy>(name: &str, chart: C, nudge: f64) {
        for map in [TiltMap::Sheet, TiltMap::Lean] {
            let view = Tilted::new(chart, Tilt::new(&spec(nudge, 0.0, map, 6), CENTRE, SPAN))
                .expect("a view");
            for [u, v] in unit() {
                assert!(
                    format!("{:?}", view.decode::<Defaults, f64>(u, v))
                        == format!("{:?}", chart.decode::<Defaults, f64>(u, v))
                        && view.validate(u, v) == chart.validate(u, v),
                    "{name}: untilted at ({u}, {v}) is not the chart bitwise"
                );
            }
        }
    }
    same("(L_z, E)", InvariantLE(inv(g)), nudge);
    same("(L_z, K)", InvariantLK(inv(g)), nudge);
    same("lone E", lone::<AxisE>(0.0, Mapping::Warped, g), nudge);
}

#[test]
fn qa_energy_warp_per_pixel_untilted() {
    check_untilted(0.0);
}

negative_control!(
    qa_energy_warp_per_pixel_untilted,
    "a tilt of 1e-6 degrees is not the chart bitwise",
    expected = "is not the chart bitwise",
    check_untilted(1e-6)
);

// ── REQ-CHART-070 ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `PE` has no warp written and no extension type under either mapping; every other quantity has both.
fn check_pe(offered: Quantity) {
    assert_eq!(
        written(offered),
        Err(Refused::NoWarpWritten(offered)),
        "{offered:?} is offered"
    );
    for m in [Mapping::Warped, Mapping::Linear] {
        assert!(
            extension(offered, m).is_err(),
            "{offered:?} has an extension type"
        );
    }
    for q in Quantity::ALL.into_iter().filter(|&q| q != Quantity::PE) {
        assert_eq!(written(q), Ok(()), "{q:?} has no written warp");
    }
}

#[test]
fn qa_pe_axis_refused() {
    check_pe(Quantity::PE);
    assert!(Quantity::PE.energy_type(), "PE is kind 3");
}

negative_control!(
    qa_pe_axis_refused,
    "E, whose warp §2.1 writes, is offered",
    expected = "is offered",
    check_pe(Quantity::E)
);

/// The virial ratio at the held `L_z = 0`, warped: every pixel validates and decodes `K* = Q_max·t^γ_K·|U|/2` at the
/// pixel's own `U` (`centre = false`) or, the control, the centre's.
fn check_virial(centre: bool) {
    let g = gamma_k();
    let chart = lone::<AxisVirial>(0.0, Mapping::Warped, g);
    assert_eq!(chart.extension(), Sides::both(ExtensionType::Bounded));
    let u0 = phys(&chart.decode::<Defaults, f64>(0.75, 0.0)).u;
    let want = move |_: f64, t: f64, p: &Phys| {
        let u = if centre { u0 } else { p.u };
        assert!(p.u < 0.0, "U = {} at positive masses", p.u);
        Want {
            k: Q_MAX * t.powf(g) * u.abs() / 2.0,
            lz: 0.0,
        }
    };
    check_chart("virial", &chart, &want, &unit());
    for t in [-0.1, 1.1] {
        assert!(
            chart.validate(0.5, t) != ValidationResult::Valid,
            "the virial axis passes t = {t}"
        );
    }
}

#[test]
fn qa_physical_axis_warps_virial() {
    check_virial(false);
}

negative_control!(
    qa_physical_axis_warps_virial,
    "K* read against the centre's U is not the pixel's virial system",
    expected = "is not its warp's system",
    check_virial(true)
);

/// The virial ratio under the linear option at a held `L_z ≠ 0` (chart_reference §2.1: "Under the linear option the
/// axis is `Q = Q_max·t`, affine, and its pixels with `K* < L_z²/2I` are not a system"), the chart built under
/// `mapping` (`Linear`; the control's `Warped`, whose floor makes those pixels systems): those pixels are
/// refused and the others decode `K* = Q_max·t·|U|/2`; a pixel on the boundary, to this test's rounding, is not judged.
fn check_virial_linear(mapping: Mapping) {
    let held = 0.3;
    let chart = lone::<AxisVirial>(held, mapping, gamma_k());
    let mut blank = 0;
    for [u, t] in unit() {
        let valid = chart.validate(u, t) == ValidationResult::Valid;
        let d = chart.decode::<Defaults, f64>(u, t);
        // U from the configuration, which the momenta do not move.
        let p = phys(&d);
        let k = Q_MAX * t * p.u.abs() / 2.0;
        if held * held > 2.0 * p.i * k * (1.0 + 64.0 * EPS) {
            blank += 1;
            assert!(
                !valid,
                "the virial pixel ({u}, {t}) with K* < L_z²/2I is a system"
            );
        } else if held * held < 2.0 * p.i * k * (1.0 - 64.0 * EPS) {
            assert!(
                valid && d.degenerate.is_none() && (p.k - k).abs() <= (TOL_K + RND) * EPS * k,
                "the linear virial pixel ({u}, {t}) is not its system: {valid}, K = {} not {k}",
                p.k
            );
        }
    }
    assert!(blank > 0, "no virial pixel is blank");
    assert_eq!(chart.extension(), Sides::both(ExtensionType::Affine));
}

#[test]
fn qa_physical_axis_warps_virial_linear() {
    check_virial_linear(Mapping::Linear);
}

negative_control!(
    qa_physical_axis_warps_virial_linear,
    "the warped virial axis stands on the floor, so its pixels with K* < L_z²/2I are systems",
    expected = "with K* < L_z²/2I is a system",
    check_virial_linear(Mapping::Warped)
);

/// One arm: under `S` at `p`, the warped ratio chart over `[0.05, 20]` validates every pixel of `[0,1]²`, its masses
/// positive, summing to 1 and with ratio `q₀·(q₁/q₀)^t` to `ε_μ`.
fn check_ratio_arm<S: Selection>(link: &str, pair: [u32; 2], p: &LinkParams<f64>, softmax5: bool) {
    let chart = ratio::<S>([0.05, 20.0], pair, Mapping::Warped, p).expect("a realised range");
    let [q0, q1] = chart.ratio;
    assert!(
        0.05 <= q0 && q0 < q1 && q1 <= 20.0,
        "{link}: the realised range [{q0}, {q1}]"
    );
    let eps_mu = <f64 as ChartConstants>::EPS_MU;
    for [u, v] in unit() {
        let q = q0 * (q1 / q0).powf(v);
        let m = if !softmax5 {
            chart.decode_params::<S, f64>(u, v, p).ic.m
        } else {
            let m_a = (1.0 - chart.held) / (1.0 + q);
            let mut want = [chart.held; 3];
            want[pair[0] as usize] = m_a;
            want[pair[1] as usize] = q * m_a;
            let five = LinkParams {
                mu_max: 5.0,
                ..LinkParams::DEFAULT
            };
            let z = <<Defaults as Selection>::MassLink as Link<2, 3>>::inverse_at(want, &five);
            <S::MassLink as Link<2, 3>>::forward_at(z, p)
        };
        let got = m[pair[1] as usize] / m[pair[0] as usize];
        assert!(
            chart.validate_params(u, v, p) == ValidationResult::Valid
                && m.iter().all(|&x| x > 0.0)
                && (m.iter().sum::<f64>() - 1.0).abs() <= 4.0 * EPS
                && (got - q).abs() <= eps_mu * q,
            "the mass ratio {pair:?} under {link} at μ_max = {} at ({u}, {v}) decodes {got}, not q = {q}",
            p.mu_max
        );
    }
    for v in [-0.1, 1.1] {
        assert!(
            chart.validate_params(0.5, v, p) != ValidationResult::Valid,
            "{link}: the ratio passes v = {v}"
        );
    }
}

/// Every arm: the masses the chart decodes, or (`softmax5`, the control) the masses of a warp written for
/// `softmax_tanh` at `μ_max = 5` alone, its controls `L⁻¹` at `μ_max = 5` read through the arm's link.
fn check_mass_ratio(softmax5: bool) {
    let mu = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's allowed range");
    for pair in [[0, 1], [1, 2], [2, 0]] {
        check_ratio_arm::<Stick>("stick_breaking", pair, &LinkParams::DEFAULT, softmax5);
        for mu_max in [mu.lo, mu.hi] {
            let p = LinkParams {
                mu_max,
                ..LinkParams::DEFAULT
            };
            check_ratio_arm::<Defaults>("softmax_tanh", pair, &p, softmax5);
        }
    }
}

#[test]
fn qa_physical_axis_warps_mass_ratio() {
    check_mass_ratio(false);
}

negative_control!(
    qa_physical_axis_warps_mass_ratio,
    "a mass-ratio warp written for softmax_tanh at μ_max = 5 alone decodes another ratio under stick_breaking or at an \
     end of μ_max's range",
    expected = "decodes",
    check_mass_ratio(true)
);

/// Which decode of a mapped chart [`check_mapped_is_own`] holds against the chart.
#[derive(Clone, Copy, PartialEq)]
enum MappedRead {
    /// [`Chart::decode`].
    Own,
    /// The mapped chart's own rule path, `decode_from` on the frozen controls.
    Rule,
    /// [`Chart::decode`] one float up the `v` axis (the control): a difference no tolerance would see.
    UlpOff,
}

/// Every axis warped, a mapped invariant chart is the chart, bitwise, over a window past every edge of the square, at
/// `γ_K = gamma`, through its decode and its rule path (REQ-CHART-067: "the existing charts' decodes are bit-identical
/// to before untilted").
fn check_mapped_is_own(gamma: f64, read: MappedRead) {
    let warped = [Mapping::Warped; 2];
    let lk = InvariantLK(inv(gamma));
    let le = InvariantLE(inv(gamma));
    let mlk = Mapped::<_, Defaults>::new(lk, warped);
    let mle = Mapped::<_, Defaults>::new(le, warped);
    let p = LinkParams::DEFAULT;
    for [u, v] in grid([-0.25, 1.25], [-0.25, 1.25]) {
        let (a, b) = if read == MappedRead::Rule {
            (
                mlk.decode_from(u, v, &lk.0.frozen(), &p).0,
                mle.decode_from(u, v, &le.0.frozen(), &p).0,
            )
        } else {
            let at = if read == MappedRead::UlpOff {
                f64::from_bits(v.to_bits() + 1)
            } else {
                v
            };
            (
                mlk.decode::<Defaults, f64>(u, at),
                mle.decode::<Defaults, f64>(u, at),
            )
        };
        assert!(
            format!("{a:?}") == format!("{:?}", lk.decode::<Defaults, f64>(u, v))
                && format!("{b:?}") == format!("{:?}", le.decode::<Defaults, f64>(u, v)),
            "the mapped warped chart at ({u}, {v}), γ_K = {gamma}, is not the chart bitwise"
        );
        assert!(
            mlk.validate(u, v) == lk.validate(u, v) && mle.validate(u, v) == le.validate(u, v),
            "the mapped warped chart's check at ({u}, {v}) is not the chart's"
        );
    }
}

#[test]
fn qa_energy_axes_warped_mapped_is_own() {
    for read in [MappedRead::Own, MappedRead::Rule] {
        check_mapped_is_own(gamma_k(), read);
        check_mapped_is_own(1.5, read);
    }
}

negative_control!(
    qa_energy_axes_warped_mapped_is_own,
    "the mapped chart read one float up the v axis is a different system, which a bitwise comparison must see",
    expected = "is not the chart bitwise",
    check_mapped_is_own(1.5, MappedRead::UlpOff)
);

// ── Fix round 1: a warped lone `t` axis at a held `L_z ≠ 0`; the virial ratio's own asked range ───────────────────────

/// The held `L_z` of the refusal tests (test data).
const HELD_LZ: f64 = 0.3;

/// The configuration's `I` at `(u, t)` on a lone `Q` axis: read from the held `L_z = 0` chart's decode, whose
/// configuration is the held-`L_z` chart's (the energy axis solves into the momenta alone, kind 3).
fn config_i<Q: EnergyAxis>(u: f64, t: f64) -> f64 {
    let d = lone::<Q>(0.0, Mapping::Warped, gamma_k()).decode::<Defaults, f64>(u, t);
    phys(&d).i
}

/// A lone `Q` axis at the held `L_z = held` (chart_reference §2.1, "A warped `t` axis at a held `L_z ≠ 0` stands on the
/// floor"; the owner's ruling of 4 Nov 2026 on RQ-683; R-471 E1, R-506 W1), under `mapping` at `gamma`: warped, every
/// pixel of `[0,1]²` validates, decodes §2.1's `K* = L_z²/2I + line`, the pixel's own `I`, and the held `L_z`, and R-408's
/// count makes each a system. `line` is §2.1's line at `(t, U)`, which alone reaches `K* < L_z²/2I` inside the square,
/// so the floor has ground.
fn check_floored_axis<Q: EnergyAxis>(
    held: f64,
    mapping: Mapping,
    gamma: f64,
    line: &dyn Fn(f64, f64) -> f64,
) {
    let chart = lone::<Q>(held, mapping, gamma);
    let name = format!("{:?}", Q::QUANTITY);
    let mut nonsystems = 0;
    for [u, t] in unit() {
        let base = phys(&lone::<Q>(0.0, Mapping::Warped, gamma).decode::<Defaults, f64>(u, t));
        let k = line(t, base.u);
        nonsystems += usize::from(held * held > 2.0 * base.i * k * (1.0 + 64.0 * EPS));
        let valid = chart.validate(u, t);
        let d = chart.decode::<Defaults, f64>(u, t);
        assert!(
            valid == ValidationResult::Valid && d.degenerate.is_none(),
            "{name} warped at the held L_z = {held}: ({u}, {t}) is refused: {valid:?}"
        );
        let p = phys(&d);
        let want = held * held / (2.0 * base.i) + k;
        assert!(
            (p.k - want).abs() <= (TOL_K + RND) * EPS * want
                && (p.lz - held).abs()
                    <= TOL_L * EPS * (held.abs() + (2.0 * want).sqrt()) + RND * EPS * p.lz_scale,
            "{name} warped at the held L_z = {held}: ({u}, {t}) is not its system: K = {} not {want}, L_z = {}",
            p.k,
            p.lz
        );
        assert_eq!(
            count([Sides::both(ExtensionType::Bounded); 2], [u, t], true),
            Count::System,
            "{name}: an in-square pixel is not a system"
        );
    }
    assert!(
        nonsystems > 0,
        "{name} at the held L_z = {held} has no pixel its line alone leaves below L_z²/2I: the floor has no ground"
    );
}

/// Every lone `K`, `KE`, `E` and virial axis at the held `L_z = held` on the floor (the section above).
fn check_floored(held: f64, mapping: Mapping, g: f64) {
    let kt = move |t: f64, _: f64| warp_k(g, t);
    check_floored_axis::<AxisK>(held, mapping, g, &kt);
    check_floored_axis::<AxisKE>(held, mapping, g, &kt);
    check_floored_axis::<AxisE>(held, mapping, g, &kt);
    check_floored_axis::<AxisVirial>(held, mapping, g, &move |t, u| {
        Q_MAX * t.powf(g) * u.abs() / 2.0
    });
}

#[test]
fn qa_energy_axes_warped_held_lz_floored() {
    check_floored(HELD_LZ, Mapping::Warped, gamma_k());
    check_floored(-HELD_LZ, Mapping::Warped, gamma_k());
}

negative_control!(
    qa_energy_axes_warped_held_lz_floored,
    "the line with no floor, K_max·t, the linear chart at γ_K = 1, blanks its bottom rows",
    expected = "is refused",
    check_floored(HELD_LZ, Mapping::Linear, 1.0)
);

/// The linear option at the same held `L_z = 0.3` is offered (chart_reference §2.1: "Under the linear option it is
/// offered, its pixels with `K* < L_z²/2I` not a system"), on a lone `K` and `KE` axis, `K* = K_max·t`: a pixel with
/// `K* < L_z²/2I` at its own `I` is refused, one above it validates and decodes `K*` and the held `L_z`, and both kinds
/// occur. `mapping` is the chart's (`Linear`; the control's `Warped`, whose floor makes those pixels systems).
fn check_linear_held(mapping: Mapping) {
    fn axis<Q: EnergyAxis>(mapping: Mapping) {
        let chart = lone::<Q>(HELD_LZ, mapping, gamma_k());
        let (mut blank, mut systems) = (0, 0);
        for [u, t] in unit() {
            let i = config_i::<Q>(u, t);
            let k = k_max() * t;
            let bound = HELD_LZ * HELD_LZ / (2.0 * i);
            let valid = chart.validate(u, t);
            if k < bound * (1.0 - 64.0 * EPS) {
                blank += 1;
                assert!(
                    valid != ValidationResult::Valid,
                    "{:?} linear: ({u}, {t}) with K* < L_z²/2I is a system",
                    Q::QUANTITY
                );
            } else if k > bound * (1.0 + 64.0 * EPS) {
                systems += 1;
                let d = chart.decode::<Defaults, f64>(u, t);
                let p = phys(&d);
                assert!(
                    valid == ValidationResult::Valid
                        && d.degenerate.is_none()
                        && (p.k - k).abs() <= (TOL_K + RND) * EPS * k
                        && (p.lz - HELD_LZ).abs()
                            <= TOL_L * EPS * (HELD_LZ + (2.0 * k).sqrt()) + RND * EPS * p.lz_scale,
                    "{:?} linear at the held L_z = {HELD_LZ}: ({u}, {t}) is not its system: {valid:?}, K = {} not {k}, \
                     L_z = {}",
                    Q::QUANTITY,
                    p.k,
                    p.lz
                );
            }
        }
        assert!(
            blank > 0 && systems > 0,
            "{:?}: {blank} blank, {systems} systems",
            Q::QUANTITY
        );
    }
    axis::<AxisK>(mapping);
    axis::<AxisKE>(mapping);
}

#[test]
fn qa_energy_axis_linear_option_held_lz() {
    check_linear_held(Mapping::Linear);
}

negative_control!(
    qa_energy_axis_linear_option_held_lz,
    "the warped axis at the held L_z = 0.3 stands on the floor, so its pixels with K_max·t < L_z²/2I are systems",
    expected = "with K* < L_z²/2I is a system",
    check_linear_held(Mapping::Warped)
);

/// A virial-ratio axis beside `z_μ₁` over the asked range `asked`, at the held `L_z = 0`, under `mapping`.
fn virial_on(
    asked: [f64; 2],
    mapping: Mapping,
) -> Option<MixedEnergy<ZMu1, AxisVirial, Defaults, f64>> {
    MixedEnergy::virial(Z0, RANGE, asked, gamma_k(), 0.0, mapping)
}

/// The virial ratio keeps its own asked range `[Q_lo, Q_hi]` (R-471 H4; chart_reference §2.1's paragraph): warped,
/// `Q(t) = Q₀ + (Q_hi − Q₀)·t^γ_K`, `Q₀ = max(Q_lo, 0)`, every pixel of `[0,1]²` validating and decoding
/// `K* = Q·|U|/2` at the pixel's own `U`, bounded past both edges; linear, `Q_lo + (Q_hi − Q_lo)·t`, affine, its
/// pixels with `Q < 0` refused and R-408-forbidden, the others decoding their `K*`; a warped range with `Q_hi ≤ 0` is
/// offered by no chart. `want` is the ratio the test asserts at `(asked, mapping, t)` (§2.1's; the control's).
fn check_virial_asked(want: fn([f64; 2], Mapping, f64) -> f64) {
    for (asked, mapping) in [
        ([0.4, 1.5], Mapping::Warped),
        ([-0.5, 1.5], Mapping::Warped),
        ([0.4, 1.5], Mapping::Linear),
        ([-0.5, 1.5], Mapping::Linear),
    ] {
        let name = format!("virial {asked:?} {mapping:?}");
        let chart = virial_on(asked, mapping).expect("an offered virial range");
        let sides = match mapping {
            Mapping::Warped => Sides::both(ExtensionType::Bounded),
            Mapping::Linear => Sides::both(ExtensionType::Affine),
        };
        assert_eq!(chart.extension(), sides, "{name}'s extension type");
        let feasible: Vec<_> = unit()
            .into_iter()
            .filter(|&[u, t]| {
                let q = want(asked, mapping, t);
                if q < 0.0 {
                    let valid = chart.validate(u, t);
                    assert!(
                        valid != ValidationResult::Valid
                            && count([Sides::both(ExtensionType::Affine), sides], [u, t], false)
                                == Count::Forbidden,
                        "{name}: ({u}, {t}) at Q = {q} < 0 is a system: {valid:?}"
                    );
                }
                q >= 0.0
            })
            .collect();
        assert!(
            mapping == Mapping::Warped || asked[0] >= 0.0 || feasible.len() < unit().len(),
            "{name}: no pixel at Q < 0"
        );
        let target = move |_: f64, t: f64, p: &Phys| {
            assert!(p.u < 0.0, "U = {} at positive masses", p.u);
            Want {
                k: want(asked, mapping, t) * p.u.abs() / 2.0,
                lz: 0.0,
            }
        };
        check_chart(&name, &chart, &target, &feasible);
        if mapping == Mapping::Warped {
            for t in [-0.1, 1.1] {
                assert!(
                    chart.validate(0.5, t) != ValidationResult::Valid,
                    "{name} passes t = {t}"
                );
            }
        }
    }
    assert!(
        virial_on([-1.0, -0.2], Mapping::Warped).is_none(),
        "a warped virial range with no feasible part is offered"
    );
}

/// §2.1's ratio at `t` on `asked` under `mapping`.
fn virial_asked_want([lo, hi]: [f64; 2], mapping: Mapping, t: f64) -> f64 {
    match mapping {
        Mapping::Warped => {
            let q0 = lo.max(0.0);
            q0 + (hi - q0) * t.powf(gamma_k())
        }
        Mapping::Linear => lo + (hi - lo) * t,
    }
}

#[test]
fn qa_physical_axis_warps_virial_asked() {
    check_virial_asked(virial_asked_want);
}

negative_control!(
    qa_physical_axis_warps_virial_asked,
    "the range fixed at [0, Q_hi], as before the fix, is not the asked range's axis",
    expected = "is not its warp's system",
    check_virial_asked(|[_, hi], mapping, t| virial_asked_want([0.0, hi], mapping, t))
);
