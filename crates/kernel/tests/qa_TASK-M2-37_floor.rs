//! qa's tests for TASK-M2-37, the owner's ruling of 4 Nov 2026 on RQ-683 ("same rule for a single energy axis at held
//! spin", the rule being RQ-678's: warped, every pixel a system on the additive floor `K = L_z²/2I + K_max·t^γ`,
//! exceeding `K_max` at the top; linear, rows keep fixed physical values and infeasible cells are "not a system";
//! "the warped mode must not blank the bottom row"), as chart_reference §2.1 writes it. Written from the ruling and
//! that paragraph, not from the implementation; every decode is read back through the physics, recomputed here from
//! `(m, r, p)` ([`phys`]); each test has its negative control (R-176). `qa_TASK-M2-37.rs`'s
//! `qa_energy_axes_warped_held_lz_floored` holds the 65 × 65 grid at the held `L_z = ±0.3`; this file holds the rest.
//!
//! - `qa_energy_axes_warped_floor_bottom_row` and `_f32`: on a sweep of held `L_z` over every binade the floor's square
//!   reaches, subnormal squares included, at f64 and at f32, the bottom row of a lone `K`, `KE`, `E` and virial axis
//!   validates and decodes a system. The sweep holds values whose rounded `L_z²/2` is below `L_z²/2`, shown by the
//!   linear chart fed that rounded value, which refuses them; the controls run the check on it.
//! - `qa_energy_axes_warped_floor_rigid`: the bottom row is §2.2's rigid rotation, `a = 0`: the non-rigid part of the
//!   kinetic energy is `K`'s tolerance. Control: the row `t = ½`, above the floor.
//! - `qa_energy_axes_warped_floor_virial`: the pixel's own virial ratio is `Q(t) + L_z²/(I·|U|)` over asked ranges with
//!   `Q_lo` below, at and above 0. Control: the ratio with no floor.
//! - `qa_energy_axes_warped_floor_past_edges`: the top row exceeds `K_max`; past `t = 1` the additive line runs on,
//!   affine, each pixel a counted system; below `t = 0` the axis is bounded, its clamp landing on the floor, a system.
//!   Control: `K*` capped at the top row's.
//! - `qa_energy_axes_warped_floor_links`: every pixel a system beside `z_β` and `z_μ₁`, under `stick_breaking` and
//!   `softmax_tanh` at both ends of `μ_max`'s allowed range. Control: the line with no floor.
//! - `qa_energy_warp_per_pixel_floor_tilted`: the lone axes tilted into their hidden coordinates at REQ-CHART-069's
//!   angles: every pixel whose `(a, b)` lie in the square validates and meets `K* = L_z²/2I + line`, at its own `U`,
//!   `I` and masses. Control: the potential read once at the centre.
//! - `qa_energy_axis_linear_option_floor_by_value`: under the linear option at a held `L_z ≠ 0`, no floor: `K`, `KE`,
//!   `E` and virial pixels keep their fixed values, those with `K* < L_z²/2I` refused, `DEGENERATE` (never integrated),
//!   forbidden in R-408's count and unwarned, the rest systems at exactly those values. Control: the warped chart.
//! - `qa_energy_axis_linear_option_floor_edge`: on the boundary, exactly representable (`L_z = ½`, `K* = ⅛`), a linear
//!   pixel is a system and the float below it is not; the warped chart at `t = 0` decodes that same `K*`. Control: the
//!   float below read as a system.
//!
//! **Values.** `K_max` and `γ_K` are REQ-CHART-044's placeholders (R-71), `validation::{K_MAX, GAMMA_K}`. `K` and `L_z`
//! are held to REQ-DEC-046's proposed tolerances (8 and 4 eps, as `qa_TASK-M2-37.rs`), plus [`RND`], this file's own
//! re-evaluation's rounding. A pixel within 64 eps of the boundary is not judged by value (as `qa_TASK-M2-37.rs`);
//! [`qa_energy_axis_linear_option_floor_edge`] judges the boundary exactly. Every slice, range, angle and held value
//! is test data.

use kernel::chart::energy::{count, decoder_bug_warning, Count, ExtensionType, Mapping, Sides};
use kernel::chart::mixed::{ZBeta, ZMu1};
use kernel::chart::physical::{AxisE, AxisK, AxisKE, AxisVirial, EnergyAxis, MixedEnergy};
use kernel::chart::tilt::Tiltable;
use kernel::chart::{Chart, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult};
use kernel::decode::Decoded;
use kernel::generated::links::{
    Defaults, LinkParams, LinkReal, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use kernel::payload::PayloadReal;
use validation::{negative_control, tunables, GAMMA_K, K_MAX};

const EPS: f64 = f64::EPSILON;
/// `K`'s tolerance, REQ-DEC-046's proposed.
const TOL_K: f64 = 8.0;
/// `L_z`'s tolerance, REQ-DEC-046's proposed.
const TOL_L: f64 = 4.0;
/// The rounding of this file's own re-evaluation of `K` and `L_z`.
const RND: f64 = 16.0;
/// The boundary's margin: a pixel this close, relative, to `K* = L_z²/2I` is not judged by value.
const MARGIN: f64 = 64.0 * EPS;
/// The grid's side (REQ-CHART-067's).
const N: u32 = 65;
/// The held `L_z` (test data).
const HELD: f64 = 0.3;
/// The slice centre (test data; `qa_TASK-M2-37.rs`'s).
const Z0: [f64; 8] = [-0.2, 0.3, 0.1, -0.2, 0.05, 0.15, 0.5, -0.3];
const RANGE: [f64; 2] = [-1.0, 1.0];
/// The virial axis's `Q_max` (test data).
const Q_MAX: f64 = 1.5;
/// The `u` at which `z_μ₁`, on [`RANGE`], is the slice centre's.
const U_CENTRE: f64 = 0.75;

/// The selection with `stick_breaking` on the mass block.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

fn k_max() -> f64 {
    K_MAX.number()
}

fn gamma_k() -> f64 {
    GAMMA_K.number()
}

/// §2.1's `K_max·t^γ_K`.
fn warp_k(t: f64) -> f64 {
    k_max() * t.powf(gamma_k())
}

fn grid(v: [f64; 2]) -> Vec<[f64; 2]> {
    let at = |x: u32, a: f64, b: f64| a + (b - a) * f64::from(x) / f64::from(N - 1);
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [at(i, 0.0, 1.0), at(j, v[0], v[1])]))
        .collect()
}

fn unit() -> Vec<[f64; 2]> {
    grid([0.0, 1.0])
}

/// A decoded system's physics, recomputed from `(m, r, p)` about the centre of mass.
#[derive(Clone, Copy, Debug)]
struct Phys {
    u: f64,
    i: f64,
    k: f64,
    lz: f64,
    lz_scale: f64,
    /// The kinetic energy of the velocities' difference from the rigid rotation `ω = L_z/I`.
    non_rigid: f64,
    finite: bool,
}

fn phys(d: &Decoded<f64>) -> Phys {
    let ic = &d.ic;
    let m = ic.m;
    let total: f64 = m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|j| m[j] * ic.r[j][a]).sum::<f64>() / total);
    let rel = |j: usize| [ic.r[j][0] - c[0], ic.r[j][1] - c[1]];
    let sum = |f: &dyn Fn(usize) -> f64| (0..3).map(f).sum::<f64>();
    let i = sum(&|j| m[j] * (rel(j)[0].powi(2) + rel(j)[1].powi(2)));
    let lz = sum(&|j| rel(j)[0] * ic.p[j][1] - rel(j)[1] * ic.p[j][0]);
    let lz_scale = sum(&|j| (rel(j)[0] * ic.p[j][1]).abs() + (rel(j)[1] * ic.p[j][0]).abs());
    let k = sum(&|j| (ic.p[j][0].powi(2) + ic.p[j][1].powi(2)) / (2.0 * m[j]));
    let omega = lz / i;
    let non_rigid = sum(&|j| {
        let dv = [
            ic.p[j][0] / m[j] + omega * rel(j)[1],
            ic.p[j][1] / m[j] - omega * rel(j)[0],
        ];
        m[j] * (dv[0] * dv[0] + dv[1] * dv[1]) / 2.0
    });
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
        u,
        i,
        k,
        lz,
        lz_scale,
        non_rigid,
        finite,
    }
}

/// A lone `Q` axis beside `z_μ₁`, its held value `held`, under `mapping`, at the float `P`.
fn lone_at<Q: EnergyAxis, P: num_float::Precision>(
    held: P,
    mapping: Mapping,
) -> MixedEnergy<ZMu1, Q, Defaults, P> {
    let top = if Q::QUANTITY == kernel::chart::energy::Quantity::VirialRatio {
        Q_MAX
    } else {
        k_max()
    };
    MixedEnergy::new(
        Z0.map(P::of),
        RANGE.map(P::of),
        P::of(top),
        P::of(gamma_k()),
        held,
        mapping,
    )
}

/// The parameters at f32 or f64.
mod num_float {
    use spirv_std::num_traits::Float;

    /// A float the charts are held at.
    pub trait Precision: Float {
        /// `x` at this float.
        fn of(x: f64) -> Self;
    }

    impl Precision for f64 {
        fn of(x: f64) -> f64 {
            x
        }
    }

    impl Precision for f32 {
        fn of(x: f64) -> f32 {
            x as f32
        }
    }
}

fn lone<Q: EnergyAxis>(held: f64, mapping: Mapping) -> MixedEnergy<ZMu1, Q, Defaults, f64> {
    lone_at::<Q, f64>(held, mapping)
}

/// §2.1's line of the axis `Q`, warped, at `t` on the potential `u`: the `K*` above the floor.
fn warped_line<Q: EnergyAxis>(t: f64, u: f64) -> f64 {
    if Q::QUANTITY == kernel::chart::energy::Quantity::VirialRatio {
        Q_MAX * t.powf(gamma_k()) * u.abs() / 2.0
    } else {
        warp_k(t)
    }
}

/// The configuration of a lone `Q` chart at `(u, t)`, which the held `L_z` does not move (kind 3).
fn config<Q: EnergyAxis>(u: f64, t: f64) -> Phys {
    phys(&lone::<Q>(0.0, Mapping::Warped).decode::<Defaults, f64>(u, t))
}

/// The pixel `(u, v)` decodes `K* = want` and the held `L_z`, and is a system: `valid` passes, the decode is finite and
/// not `DEGENERATE`.
fn assert_system(name: &str, at: [f64; 2], d: &Decoded<f64>, valid: bool, want: f64, held: f64) {
    let p = phys(d);
    assert!(
        valid && d.degenerate.is_none() && p.finite,
        "{name}: {at:?} is refused: {:?}",
        d.degenerate
    );
    assert!(
        (p.k - want).abs() <= (TOL_K + RND) * EPS * want,
        "{name}: {at:?} is not its system: K = {} not {want}",
        p.k
    );
    assert!(
        (p.lz - held).abs()
            <= TOL_L * EPS * (held.abs() + (2.0 * want).sqrt()) + RND * EPS * p.lz_scale,
        "{name}: {at:?} is not its system: L_z = {} not {held}",
        p.lz
    );
}

// ── The bottom row ───────────────────────────────────────────────────────────────────────────────────────────────────

const COLUMNS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// The row `t = v` validates and decodes, at `R`, an undegenerate system, on `chart`.
fn check_row<C, R>(name: &str, chart: &C, held: impl core::fmt::LowerExp, v: f64, of: fn(f64) -> R)
where
    C: Chart,
    R: LinkReal + PayloadReal + From<C::Param>,
{
    for u in COLUMNS {
        let valid = chart.validate(u, v);
        let d = chart.decode::<Defaults, R>(of(u), of(v));
        assert!(
            valid == ValidationResult::Valid && d.degenerate.is_none(),
            "{name}: the row {v} blanks at u = {u}, held L_z = {held:e}: {valid:?}, {:?}",
            d.degenerate
        );
    }
}

/// A sweep of held `L_z` (test data): `n` pseudo-random mantissas, a splitmix64 sequence, each at its own binade in
/// `2^[lo, hi]`, alternating in sign, then `extra`.
fn sweep(n: u32, [lo, hi]: [i32; 2], extra: &[f64]) -> Vec<f64> {
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let span = (hi - lo + 1) as u64;
    let mut all: Vec<f64> = (0..n)
        .map(|k| {
            let e = lo + (next() % span) as i32;
            let m = 1.0 + (next() >> 12) as f64 * f64::EPSILON;
            let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
            sign * m * 2.0f64.powi(e)
        })
        .collect();
    all.extend_from_slice(extra);
    all
}

/// The linear chart whose `K*` at `t = 1` is `naive`: a held `L_z = held` fed a floor with no step up.
fn fed<P: num_float::Precision>(held: P, naive: P) -> MixedEnergy<ZMu1, AxisK, Defaults, P> {
    let mut chart = lone_at::<AxisK, P>(held, Mapping::Linear);
    chart.top = naive;
    chart
}

/// The bottom row of each warped axis at each held `L_z` of the sweep at f64, and just above it where `t^γ_K`
/// underflows; or (`naive`, the control) the row of the chart fed `L_z²/2` as rounded. The sweep must hold values whose
/// rounded `L_z²/2` the exact test refuses (it would hold no rounding hazard otherwise).
fn check_bottom_row(naive: bool) {
    let helds = sweep(
        4000,
        [-560, 8],
        &[
            1e-200,
            f64::MIN_POSITIVE,
            5e-324,
            1.5e-160,
            -3e-162,
            HELD,
            -HELD,
        ],
    );
    let of = |x: f64| x;
    let mut below = 0;
    for &l in &helds {
        let rounded = fed(l, l * l / 2.0);
        below += usize::from(rounded.validate(0.5, 1.0) != ValidationResult::Valid);
        if naive {
            check_row("the floor as rounded", &rounded, l, 1.0, of);
            continue;
        }
        for v in [0.0, 1e-200] {
            check_row("K", &lone::<AxisK>(l, Mapping::Warped), l, v, of);
            check_row("KE", &lone::<AxisKE>(l, Mapping::Warped), l, v, of);
            check_row("E", &lone::<AxisE>(l, Mapping::Warped), l, v, of);
            check_row("virial", &lone::<AxisVirial>(l, Mapping::Warped), l, v, of);
        }
    }
    assert!(
        below > helds.len() / 8,
        "the sweep has {below} held L_z whose rounded L_z²/2 falls short: the row is not at risk"
    );
}

#[test]
fn qa_energy_axes_warped_floor_bottom_row() {
    check_bottom_row(false);
}

negative_control!(
    qa_energy_axes_warped_floor_bottom_row,
    "the floor as rounded, with no step up, leaves K* below L_z²/2 for some held L_z, blanking the row",
    expected = "blanks",
    check_bottom_row(true)
);

/// [`check_bottom_row`] at f32, the decoder's float on the GPU, decoded at f32 and widened to f64.
fn check_bottom_row_f32(naive: bool) {
    let helds: Vec<f32> = sweep(4000, [-90, 8], &[1e-30, 1e-45, -1.2e-23, 0.3])
        .into_iter()
        .map(|l| l as f32)
        .collect();
    let of32 = |x: f64| x as f32;
    let of64 = |x: f64| x;
    let mut below = 0;
    for &l in &helds {
        let rounded = fed(l, l * l / 2.0);
        below += usize::from(
            rounded
                .decode::<Defaults, f32>(0.5, 1.0)
                .degenerate
                .is_some(),
        );
        if naive {
            check_row("the floor as rounded", &rounded, l, 1.0, of32);
            continue;
        }
        for v in [0.0, 1e-30] {
            let k = lone_at::<AxisK, f32>(l, Mapping::Warped);
            let ke = lone_at::<AxisKE, f32>(l, Mapping::Warped);
            let e = lone_at::<AxisE, f32>(l, Mapping::Warped);
            let virial = lone_at::<AxisVirial, f32>(l, Mapping::Warped);
            check_row("K", &k, l, v, of32);
            check_row("KE", &ke, l, v, of32);
            check_row("E", &e, l, v, of32);
            check_row("virial", &virial, l, v, of32);
            check_row("K widened", &k, l, v, of64);
            check_row("virial widened", &virial, l, v, of64);
        }
    }
    assert!(
        below > helds.len() / 8,
        "the sweep has {below} held L_z whose rounded L_z²/2 falls short: the row is not at risk"
    );
}

#[test]
fn qa_energy_axes_warped_floor_bottom_row_f32() {
    check_bottom_row_f32(false);
}

negative_control!(
    qa_energy_axes_warped_floor_bottom_row_f32,
    "the floor as rounded at f32, with no step up, blanks the row",
    expected = "blanks",
    check_bottom_row_f32(true)
);

/// The row `t = v` of each warped axis at the held `L_z = ±HELD`, across the window, is the rigid rotation (chart_reference
/// §2.1: "the bottom row `t = 0` is the floor itself, §2.2's rigid rotation with `a = 0`"): `K = L_z²/2I`, and the
/// velocities are `ω × r`, `ω = L_z/I`, their kinetic difference within `K`'s tolerance.
fn check_rigid(v: f64) {
    fn axis<Q: EnergyAxis>(held: f64, v: f64) {
        let chart = lone::<Q>(held, Mapping::Warped);
        for n in 0..N {
            let u = f64::from(n) / f64::from(N - 1);
            let d = chart.decode::<Defaults, f64>(u, v);
            let p = phys(&d);
            let tol = (TOL_K + RND) * EPS * p.k;
            assert!(
                p.non_rigid <= tol,
                "{:?} at the row {v}, u = {u}, is not the rigid rotation: {} of K = {} moves otherwise",
                Q::QUANTITY,
                p.non_rigid,
                p.k
            );
            assert!(
                (p.k - held * held / (2.0 * p.i)).abs() <= tol,
                "{:?} at the row {v}, u = {u}, is not the floor: K = {}",
                Q::QUANTITY,
                p.k
            );
        }
    }
    for held in [HELD, -HELD, 1.0, 1e-3] {
        axis::<AxisK>(held, v);
        axis::<AxisKE>(held, v);
        axis::<AxisE>(held, v);
        axis::<AxisVirial>(held, v);
    }
}

#[test]
fn qa_energy_axes_warped_floor_rigid() {
    check_rigid(0.0);
}

negative_control!(
    qa_energy_axes_warped_floor_rigid,
    "the row t = ½, above the floor, has motion other than the rotation",
    expected = "is not the rigid rotation",
    check_rigid(0.5)
);

// ── The virial axis ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Over asked ranges, every pixel of the warped virial axis at the held `L_z` validates and its own ratio `2K/|U|` is
/// §2.1's `Q(t) + L_z²/(I·|U|)`, `Q(t) = Q₀ + (Q_hi − Q₀)·t^γ_K`, `Q₀ = max(Q_lo, 0)`; `floor` the term's weight, 1 and
/// (the control) 0.
fn check_virial(floor: f64) {
    let g = gamma_k();
    for asked in [[0.0, Q_MAX], [0.4, Q_MAX], [-0.5, Q_MAX]] {
        let at = |held: f64| {
            MixedEnergy::<ZMu1, AxisVirial, Defaults, f64>::virial(
                Z0,
                RANGE,
                asked,
                g,
                held,
                Mapping::Warped,
            )
            .expect("an offered range")
        };
        let chart = at(HELD);
        let q0 = asked[0].max(0.0);
        for [u, t] in unit() {
            let d = chart.decode::<Defaults, f64>(u, t);
            let p = phys(&d);
            let q = q0 + (asked[1] - q0) * t.powf(g);
            let ratio = q + floor * HELD * HELD / (p.i * p.u.abs());
            let own = 2.0 * p.k / p.u.abs();
            assert!(
                chart.validate(u, t) == ValidationResult::Valid
                    && d.degenerate.is_none()
                    && (own - ratio).abs() <= 2.0 * (TOL_K + RND) * EPS * ratio
                    && (p.lz - HELD).abs()
                        <= TOL_L * EPS * (HELD + (2.0 * p.k).sqrt()) + RND * EPS * p.lz_scale,
                "the virial pixel ({u}, {t}) over {asked:?} has the ratio {own}, not {ratio}, L_z = {}",
                p.lz
            );
        }
    }
}

#[test]
fn qa_energy_axes_warped_floor_virial() {
    check_virial(1.0);
}

negative_control!(
    qa_energy_axes_warped_floor_virial,
    "the ratio read with no floor term",
    expected = "has the ratio",
    check_virial(0.0)
);

// ── Past the square ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The warped `t` axis on the floor past its edges (the ruling: "exceeding K_max at the top is fine; K_max is a range,
/// per R-508's t>1 continuation"). Its top row exceeds `K_max`; to `t = 1.5` every pixel validates, a counted system, at
/// `K* = L_z²/2I + K_max·t^γ_K` (or, the control, `t` capped at 1); below `t = 0` the axis is bounded, and its clamp
/// lands on the bottom row, itself a system.
fn check_past(cap: bool) {
    fn axis<Q: EnergyAxis>(cap: bool) {
        let name = format!("{:?}", Q::QUANTITY);
        let chart = lone::<Q>(HELD, Mapping::Warped);
        let sides = [Sides::both(ExtensionType::Bounded), chart.extension()];
        for [u, t] in grid([0.0, 1.5]) {
            let c = config::<Q>(u, t);
            let line = warped_line::<Q>(if cap { t.min(1.0) } else { t }, c.u);
            let want = HELD * HELD / (2.0 * c.i) + line;
            let valid = chart.validate(u, t) == ValidationResult::Valid;
            let d = chart.decode::<Defaults, f64>(u, t);
            assert_system(&name, [u, t], &d, valid, want, HELD);
            assert_eq!(
                count(sides, [u, t], valid),
                Count::System,
                "{name} ({u}, {t})"
            );
        }
        let top = phys(&chart.decode::<Defaults, f64>(0.5, 1.0));
        let line = warped_line::<Q>(1.0, top.u);
        assert!(
            top.k > line,
            "{name}: the top row does not exceed its line's top"
        );
        for t in [-1e-12, -0.05, -0.5] {
            for u in COLUMNS {
                let ValidationResult::Clamp { u: cu, v: cv, .. } = chart.validate(u, t) else {
                    panic!(
                        "{name}: ({u}, {t}) below t = 0 is not clamped: {:?}",
                        chart.validate(u, t)
                    );
                };
                assert!(
                    (cu, cv) == (u, 0.0) && chart.validate(cu, cv) == ValidationResult::Valid,
                    "{name}: ({u}, {t}) clamps to ({cu}, {cv})"
                );
                assert_eq!(
                    count(sides, [u, t], false),
                    Count::DomainEnd,
                    "{name} ({u}, {t})"
                );
            }
        }
    }
    axis::<AxisK>(cap);
    axis::<AxisKE>(cap);
    axis::<AxisE>(cap);
}

#[test]
fn qa_energy_axes_warped_floor_past_edges() {
    check_past(false);
}

negative_control!(
    qa_energy_axes_warped_floor_past_edges,
    "K* capped at the top row's, which R-508 and the ruling continue past",
    expected = "is not its system",
    check_past(true)
);

// ── Other horizontal axes and links ──────────────────────────────────────────────────────────────────────────────────

/// Every pixel of `chart` under `p` validates and decodes `K* = L_z²/2I + K_max·t^γ_K`, the pixel's own `I`, the held
/// `L_z`.
fn check_links_arm<C: Chart<Param = f64>>(name: &str, chart: &C, p: &LinkParams<f64>) {
    for [u, t] in unit() {
        let d = chart.decode_params::<Defaults, f64>(u, t, p);
        let valid = chart.validate_params(u, t, p) == ValidationResult::Valid;
        let want = HELD * HELD / (2.0 * phys(&d).i) + warp_k(t);
        assert_system(name, [u, t], &d, valid, want, HELD);
    }
}

/// The warped axes on the floor beside `z_β`, whose axis moves the configuration, and `z_μ₁`, under `softmax_tanh` and
/// `stick_breaking`, at both ends of `μ_max`'s allowed range. `mapping` is the chart's (`Warped`; the control's `Linear`
/// at `γ_K = 1`, the line with no floor).
fn check_links(mapping: Mapping) {
    let g = if mapping == Mapping::Warped {
        gamma_k()
    } else {
        1.0
    };
    let mu = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's allowed range");
    let params = [mu.lo, LinkParams::DEFAULT.mu_max, mu.hi].map(|mu_max| LinkParams {
        mu_max,
        ..LinkParams::DEFAULT
    });
    let top = k_max();
    for p in &params {
        let beta =
            MixedEnergy::<ZBeta, AxisK, Defaults, f64>::new(Z0, RANGE, top, g, HELD, mapping);
        check_links_arm("K beside z_β", &beta, p);
        let e = MixedEnergy::<ZBeta, AxisE, Defaults, f64>::new(Z0, RANGE, top, g, HELD, mapping);
        check_links_arm("E beside z_β", &e, p);
        let soft =
            MixedEnergy::<ZMu1, AxisKE, Defaults, f64>::new(Z0, RANGE, top, g, HELD, mapping);
        check_links_arm("KE beside z_μ₁", &soft, p);
    }
    let stick = MixedEnergy::<ZMu1, AxisK, Stick, f64>::new(Z0, RANGE, top, g, HELD, mapping);
    check_links_arm("K under stick_breaking", &stick, &LinkParams::DEFAULT);
    let stick = MixedEnergy::<ZBeta, AxisE, Stick, f64>::new(Z0, RANGE, top, g, HELD, mapping);
    check_links_arm("E under stick_breaking", &stick, &LinkParams::DEFAULT);
}

#[test]
fn qa_energy_axes_warped_floor_links() {
    check_links(Mapping::Warped);
}

negative_control!(
    qa_energy_axes_warped_floor_links,
    "the line with no floor, the linear chart at γ_K = 1, is refused at its bottom rows",
    expected = "is refused",
    check_links(Mapping::Linear)
);

// ── Tilted ───────────────────────────────────────────────────────────────────────────────────────────────────────────

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

/// REQ-CHART-069's angles, into the target `x`.
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

/// The lone axis `Q` at the held `L_z`, tilted by `s`: every pixel whose `(a, b)` lie in the square validates and
/// decodes `K* = L_z²/2I + line(b)` at its own `U` and `I`; `centre`, the control, reads the line's potential at the
/// slice centre: the virial `K*` there, and on `E` the energy `U₀ + K*`.
fn check_tilted_axis<Q: EnergyAxis>(s: &TiltSpec, centre: bool)
where
    MixedEnergy<ZMu1, Q, Defaults, f64>: Tiltable<Param = f64> + Copy,
{
    let chart = lone::<Q>(HELD, Mapping::Warped);
    let view = Tilted::new(chart, Tilt::new(s, CENTRE, SPAN)).expect("a hidden target");
    let u0 = config::<Q>(U_CENTRE, 0.0).u;
    let mut inside = 0;
    for [u, v] in unit() {
        let ([a, b], _) = view.at(u, v);
        if !((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)) {
            continue;
        }
        inside += 1;
        let valid = view.validate(u, v) == ValidationResult::Valid;
        let d = view.decode::<Defaults, f64>(u, v);
        let p = phys(&d);
        let line = warped_line::<Q>(b, if centre { u0 } else { p.u });
        let shift = if centre && Q::QUANTITY == kernel::chart::energy::Quantity::E {
            u0 - p.u
        } else {
            0.0
        };
        let want = HELD * HELD / (2.0 * p.i) + line + shift;
        assert_system(
            &format!("{:?} under {s:?}", Q::QUANTITY),
            [u, v],
            &d,
            valid,
            want,
            HELD,
        );
    }
    assert!(inside > N * N / 4, "only {inside} pixels in the square");
}

/// [`check_tilted_axis`] on `K`, `E` and the virial axis, into `z_α`, `z_β` and `z_μ₂`, their hidden coordinates.
fn check_tilted(centre: bool) {
    for x in [0, 1, 7] {
        for s in tilts(x) {
            check_tilted_axis::<AxisE>(&s, centre);
            check_tilted_axis::<AxisVirial>(&s, centre);
            check_tilted_axis::<AxisK>(&s, centre);
        }
    }
}

#[test]
fn qa_energy_warp_per_pixel_floor_tilted() {
    check_tilted(false);
}

negative_control!(
    qa_energy_warp_per_pixel_floor_tilted,
    "the potential read once at the centre is not the pixel's own (E3)",
    expected = "is not its system",
    check_tilted(true)
);

// ── The linear option ────────────────────────────────────────────────────────────────────────────────────────────────

/// A lone `Q` at the held `L_z = held` read as the linear option: `K*` is `value` at `(t, U, U₀)`, with no floor. Each
/// pixel with `K* < L_z²/2I`, at its own `I`, is not a system: refused, its decode `DEGENERATE`, forbidden in R-408's
/// count. Every other is a system at that `K*`, a counted one, so a row keeps its physical value. Both occur.
fn check_value_axis<Q: EnergyAxis>(
    chart: &MixedEnergy<ZMu1, Q, Defaults, f64>,
    held: f64,
    value: &dyn Fn(f64, f64, f64) -> f64,
) {
    let name = format!("{:?}", Q::QUANTITY);
    let u0 = config::<Q>(U_CENTRE, 0.0).u;
    let sides = [Sides::both(ExtensionType::Bounded), chart.extension()];
    let (mut blank, mut systems) = (0, 0);
    for [u, t] in unit() {
        let c = config::<Q>(u, t);
        let k = value(t, c.u, u0);
        let floor = held * held / (2.0 * c.i);
        let result = chart.validate(u, t);
        let d = chart.decode::<Defaults, f64>(u, t);
        if k < floor * (1.0 - MARGIN) {
            blank += 1;
            assert!(
                matches!(result, ValidationResult::Reject { .. }) && d.degenerate.is_some(),
                "{name} linear: ({u}, {t}) with K* = {k} below L_z²/2I = {floor} is a system: {result:?}, {:?}",
                d.degenerate
            );
            assert_eq!(count(sides, [u, t], false), Count::Forbidden, "{name}");
        } else if k > floor * (1.0 + MARGIN) {
            systems += 1;
            let valid = result == ValidationResult::Valid;
            assert_system(&format!("{name} linear"), [u, t], &d, valid, k, held);
            assert_eq!(count(sides, [u, t], valid), Count::System, "{name}");
        }
    }
    assert!(
        blank > 0 && systems > 0,
        "{name}: {blank} blank, {systems} systems"
    );
}

/// The linear option's values (chart_reference §2.1): `K` and `KE` `K_max·t`, `E` `U₀ + K_max·t`, the virial ratio
/// `Q_lo + (Q_hi − Q_lo)·t`, none warned. `mapping` is the chart's (`Linear`; the control's `Warped`).
fn check_by_value(mapping: Mapping) {
    for held in [HELD, -HELD] {
        let k = |t: f64, _: f64, _: f64| k_max() * t;
        check_value_axis(&lone::<AxisK>(held, mapping), held, &k);
        check_value_axis(&lone::<AxisKE>(held, mapping), held, &k);
        // K* = E − U at the pixel, which may be below 0 as well as below the floor.
        let e = |t: f64, u: f64, u0: f64| u0 + k_max() * t - u;
        check_value_axis(&lone::<AxisE>(held, mapping), held, &e);
        let asked = [-0.5, Q_MAX];
        let virial = MixedEnergy::<ZMu1, AxisVirial, Defaults, f64>::virial(
            Z0,
            RANGE,
            asked,
            gamma_k(),
            held,
            mapping,
        )
        .expect("an offered range");
        let q =
            move |t: f64, u: f64, _: f64| (asked[0] + (asked[1] - asked[0]) * t) * u.abs() / 2.0;
        check_value_axis(&virial, held, &q);
    }
    assert!(
        !decoder_bug_warning(&[mapping]) || mapping == Mapping::Warped,
        "the linear axis's blank pixels warn"
    );
}

#[test]
fn qa_energy_axis_linear_option_floor_by_value() {
    check_by_value(Mapping::Linear);
}

negative_control!(
    qa_energy_axis_linear_option_floor_by_value,
    "the warped axis stands on the floor, so its pixels below the fixed values' boundary are systems",
    expected = "is a system",
    check_by_value(Mapping::Warped)
);

/// The boundary judged exactly: at `L_z = ½` (so `L_z²/2 = ⅛`, exact), a linear `K` or `KE` axis's pixel at `t = 1` with
/// `K_max = ⅛` is a system, the rigid rotation, and with `K_max` the float below it (or, the control, the float below
/// read as feasible) is not; the warped chart at `t = 0` decodes that `⅛`.
fn check_edge(float_below_is_a_system: bool) {
    fn axis<Q: EnergyAxis>(float_below_is_a_system: bool) {
        let name = format!("{:?}", Q::QUANTITY);
        let held = 0.5;
        let edge = 0.125f64;
        let under = f64::from_bits(edge.to_bits() - 1);
        let at = |top: f64| {
            let mut chart = lone::<Q>(held, Mapping::Linear);
            chart.top = top;
            chart
        };
        for u in COLUMNS {
            let on = at(edge);
            let valid = on.validate(u, 1.0) == ValidationResult::Valid;
            let d = on.decode::<Defaults, f64>(u, 1.0);
            assert_system(
                &format!("{name} on the boundary"),
                [u, 1.0],
                &d,
                valid,
                edge,
                held,
            );
            let off = at(under);
            assert!(
                float_below_is_a_system || off.validate(u, 1.0) != ValidationResult::Valid,
                "{name}: the float below the boundary is a system"
            );
            assert!(
                !float_below_is_a_system || off.validate(u, 1.0) == ValidationResult::Valid,
                "{name}: the float below the boundary, read as a system, blanks"
            );
            let warped = lone::<Q>(held, Mapping::Warped);
            let valid = warped.validate(u, 0.0) == ValidationResult::Valid;
            let d = warped.decode::<Defaults, f64>(u, 0.0);
            assert_system(&format!("{name} warped"), [u, 0.0], &d, valid, edge, held);
        }
    }
    axis::<AxisK>(float_below_is_a_system);
    axis::<AxisKE>(float_below_is_a_system);
}

#[test]
fn qa_energy_axis_linear_option_floor_edge() {
    check_edge(false);
}

negative_control!(
    qa_energy_axis_linear_option_floor_edge,
    "the float below the boundary read as a system",
    expected = "read as a system, blanks",
    check_edge(true)
);
