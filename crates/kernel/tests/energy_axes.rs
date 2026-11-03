//! Physical-quantity axes warped by default, a linear mapping an option per axis, the warp per pixel (R-471, R-506,
//! R-508; chart_reference §2.1, §5.4; `kernel::chart::energy`, `kernel::chart::physical`):
//! - `energy_axes_warped_*` (REQ-CHART-067): every chart with an energy-type axis validates every pixel of a 65 × 65
//!   grid over `[0,1]²`, its decode feasible; the existing charts are bitwise their own under the default mapping; a
//!   lone `E` axis decodes `E = U + K_max·t^γ_K`; the warp's `t` axis continues past `t = 1` and is bounded below
//!   `t = 0`, its `s` axis bounded; a mass-ratio axis keeps every pixel's masses positive and summing to 1.
//! - `energy_axis_linear_option_*` (REQ-CHART-068): under the linear option `(L_z, E)`'s corners past the parabola are
//!   not a system, never integrated, never warned; the row `t = 1`, the point `(½, 0)` and `L_z = 0` on the column
//!   `s = ½` agree with the warp; R-408's count makes those pixels forbidden; a mass-ratio axis's non-positive ratio is
//!   not a system.
//! - `energy_warp_per_pixel_*` (REQ-CHART-069): `(L_z, K)` and `(L_z, E)` tilted into a mass logit and into a shape
//!   control validate every pixel, at its own `U`, `I` and masses; untilted, bitwise the chart's.
//! - `pe_axis_*` (REQ-CHART-070): no chart offers `PE`.
//! - `physical_axis_warps_*` (REQ-CHART-070): the virial ratio and the mass ratio warp as chart_reference §2.1 writes,
//!   the mass ratio under `stick_breaking` and under `softmax_tanh` at both ends of `μ_max`'s range.
//!
//! `K_max` and `γ_K` are REQ-CHART-044's placeholders (R-71), `validation::{K_MAX, GAMMA_K}`. Each test registers its
//! negative control (R-176).

use kernel::chart::energy::{
    count, decode_rule, decoder_bug_warning, extension, written, Count, ExtensionType, Mapped,
    Mapping, Quantity, Refused, Rule, Sides,
};
use kernel::chart::invariant::{self, Invariant};
use kernel::chart::mixed::{Alpha, Quantity as _, ZAlpha, ZBeta, ZMu1};
#[cfg(feature = "controls")]
use kernel::chart::physical::ratio_masses;
use kernel::chart::physical::{
    ratio_of, AxisE, AxisK, AxisKE, AxisLz, AxisVirial, EnergyAxis, MixedEnergy, MixedMassRatio,
};
#[cfg(feature = "controls")]
use kernel::chart::tilt::shift_controls;
use kernel::chart::tilt::Tiltable;
use kernel::chart::{
    Burrau, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, InvariantLE, InvariantLK, Tilt,
    TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::decode::invariant::infeasible;
use kernel::decode::{mass, Decoded, Degenerate, Latent};
use kernel::generated::links::{
    Defaults, Link, LinkParams, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    StickBreaking,
};
use validation::{negative_control, tunables, GAMMA_K, K_MAX};

/// The grid side: 65 × 65 points `i/64` over `[0,1]²`, its edges included.
const N: u32 = 65;
/// `|K(p) − K*|` in units of `eps·K*`, at most (REQ-DEC-046, proposed; `chart_invariant.rs`).
const TOL_K: f64 = 8.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))`, at most (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// A mass ratio's decode against its axis value, relative: the realisation's `ε_μ` (chart_reference §0.1;
/// `MixedMassRatio::realised`).
const RATIO_TOL: f64 = 1e-6;

/// The selection with `stick_breaking` on the mass block.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

fn k_max() -> f64 {
    K_MAX.number()
}

fn gamma_k() -> f64 {
    GAMMA_K.number()
}

/// The 65 × 65 grid over `[lo, hi]²`, edges included.
fn grid(lo: f64, hi: f64) -> Vec<[f64; 2]> {
    let at = |i: u32| lo + (hi - lo) * f64::from(i) / f64::from(N - 1);
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [at(i), at(j)]))
        .collect()
}

/// The decode's bits, as the Debug form of every float.
fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

/// `L_z(p) = Σ (xᵢ p_yᵢ − yᵢ p_xᵢ)` and `K(p) = Σ |pᵢ|²/2mᵢ` of a decode.
fn lz_k(d: &Decoded<f64>) -> (f64, f64) {
    let ic = &d.ic;
    let lz = (0..3)
        .map(|i| ic.r[i][0] * ic.p[i][1] - ic.r[i][1] * ic.p[i][0])
        .sum();
    let k = (0..3)
        .map(|i| (ic.p[i][0].powi(2) + ic.p[i][1].powi(2)) / (2.0 * ic.m[i]))
        .sum();
    (lz, k)
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: k_max(),
        gamma_k: gamma_k(),
    }
}

/// The slice centre `z₀` of the mixed charts.
const Z0: [f64; 8] = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];

/// A lone energy-type axis beside `α` on `[0.2, 1.3]`, held value `held`, under `mapping`.
fn lone<Q: EnergyAxis>(held: f64, mapping: Mapping) -> MixedEnergy<Alpha, Q, Defaults, f64> {
    MixedEnergy::new(Z0, [0.2, 1.3], k_max(), gamma_k(), held, mapping)
}

// ── REQ-CHART-067: every pixel of [0,1]² a system ─────────────────────────────────────────────────────────────────────

/// Every point of the 65 × 65 grid over `[0,1]²` validates and decodes to a system: `at(u, v)` gives `validate`'s
/// result and the decode's `DEGENERATE` tag.
fn check_every_pixel(name: &str, at: impl Fn(f64, f64) -> (ValidationResult, Option<Degenerate>)) {
    for [u, v] in grid(0.0, 1.0) {
        let (valid, tag) = at(u, v);
        assert!(
            valid == ValidationResult::Valid && tag.is_none(),
            "pixel ({u}, {v}) of {name} is not a system: {valid:?}, {tag:?}"
        );
    }
}

/// `chart`'s check and decode tag at `(u, v)`.
fn system<C: Chart<Param = f64>>(
    chart: &C,
) -> impl Fn(f64, f64) -> (ValidationResult, Option<Degenerate>) + '_ {
    move |u, v| {
        (
            chart.validate(u, v),
            chart.decode::<Defaults, f64>(u, v).degenerate,
        )
    }
}

#[test]
fn energy_axes_warped_every_pixel() {
    let inv = invariant();
    check_every_pixel("(L_z, E)", system(&InvariantLE(inv)));
    check_every_pixel("(L_z, K)", system(&InvariantLK(inv)));
    let burrau = Burrau {
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
        k_max: k_max(),
        gamma_k: gamma_k(),
    };
    check_every_pixel("(ν, K)", system(&burrau));
    let theta = [0.1, core::f64::consts::FRAC_PI_4];
    let theta_k = BurrauThetaK {
        theta_range: theta,
        k_max: k_max(),
        gamma_k: gamma_k(),
    };
    check_every_pixel("Φ_θ,K", system(&theta_k));
    let theta_lz = BurrauThetaLz {
        theta_range: theta,
        k: k_max(),
        lz_fraction: 1.0,
    };
    check_every_pixel("Φ_θ,L_z", system(&theta_lz));
    check_every_pixel("α × E", system(&lone::<AxisE>(0.0, Mapping::Warped)));
    check_every_pixel("α × K", system(&lone::<AxisK>(0.0, Mapping::Warped)));
    check_every_pixel("α × KE", system(&lone::<AxisKE>(0.0, Mapping::Warped)));
    check_every_pixel("α × L_z", system(&lone::<AxisLz>(k_max(), Mapping::Warped)));
    check_every_pixel(
        "α × L_z, K = 0",
        system(&lone::<AxisLz>(0.0, Mapping::Warped)),
    );
    let mass_e: MixedEnergy<ZBeta, AxisE, Defaults, f64> =
        MixedEnergy::new(Z0, [-2.0, 2.0], k_max(), gamma_k(), 0.0, Mapping::Warped);
    check_every_pixel("z_β × E", system(&mass_e));
    let mu_e: MixedEnergy<ZMu1, AxisE, Defaults, f64> =
        MixedEnergy::new(Z0, [-2.0, 2.0], k_max(), gamma_k(), 0.0, Mapping::Warped);
    check_every_pixel("z_μ1 × E", system(&mu_e));
}

negative_control!(
    energy_axes_warped_every_pixel,
    "a lone E axis mapped linearly onto [U₀, U₀ + K_max] leaves the pixels whose own U lies above E not a system, so it \
     must fail",
    expected = "is not a system",
    check_every_pixel("α × E, linear", system(&lone::<AxisE>(0.0, Mapping::Linear)))
);

/// A lone `L_z` axis's line of the warp at the held `K`, its `L_max` taken at `k_line`: the warp's when `k_line` is the
/// held `K`.
fn lone_lz_line(k_line: f64) -> impl Fn(f64, f64) -> (ValidationResult, Option<Degenerate>) {
    let chart = lone::<AxisLz>(k_max(), Mapping::Warped);
    move |u, v| {
        let mut z = Z0;
        z[0] = Alpha::control_at::<Defaults, f64>(0.2 + 1.1 * u, &LinkParams::DEFAULT);
        let rule = Rule {
            k0: k_max(),
            weight: 0.0,
            u_ref: 0.0,
            s2: 0.0,
            lz: (v + v - 1.0) * invariant::l_max(k_line),
            lz_on_warp: false,
        };
        let (d, _) = decode_rule::<Defaults, f64>(rule, &Latent::new(z), &LinkParams::DEFAULT);
        (chart.validate(u, v), d.degenerate)
    }
}

#[test]
fn energy_axes_warped_held_k() {
    check_every_pixel("α × L_z's line", lone_lz_line(k_max()));
}

negative_control!(
    energy_axes_warped_held_k,
    "an L_z line whose L_max is taken at a K above the held one puts its edge pixels outside the feasible region, so \
     it must fail",
    expected = "is not a system",
    check_every_pixel("α × L_z's line, wrong K", lone_lz_line(2.0 * k_max()))
);

/// The default mapping is the chart's own, bit for bit: decode, `map` and `validate` over the grid and past it.
fn check_own<C, W>(name: &str, chart: &C, mapped: &W)
where
    C: ChartDecode<Param = f64>,
    W: ChartDecode<Param = f64>,
{
    for [u, v] in grid(-0.5, 1.5) {
        assert!(
            bits(&mapped.decode::<Defaults, f64>(u, v))
                == bits(&chart.decode::<Defaults, f64>(u, v))
                && bits(&mapped.decode_at::<Defaults, f64>(u, v))
                    == bits(&chart.decode_at::<Defaults, f64>(u, v))
                && format!("{:?}", mapped.map::<f64>(u, v))
                    == format!("{:?}", chart.map::<f64>(u, v))
                && mapped.validate(u, v) == chart.validate(u, v),
            "{name} at ({u}, {v}) is not bitwise its own chart"
        );
    }
}

fn mapped_all(mapping: Mapping) -> [Mapping; 2] {
    [mapping, mapping]
}

#[test]
fn energy_axes_warped_bitwise_own() {
    let inv = invariant();
    let warped = mapped_all(Mapping::Warped);
    check_own(
        "(L_z, E)",
        &InvariantLE(inv),
        &Mapped::<_, Defaults>::new(InvariantLE(inv), warped),
    );
    check_own(
        "(L_z, K)",
        &InvariantLK(inv),
        &Mapped::<_, Defaults>::new(InvariantLK(inv), warped),
    );
    let burrau = Burrau {
        nu_range: [0.2, 0.9],
        k_max: k_max(),
        gamma_k: gamma_k(),
    };
    check_own(
        "(ν, K)",
        &burrau,
        &Mapped::<_, Defaults>::new(burrau, warped),
    );
    let theta_k = BurrauThetaK {
        theta_range: [0.1, 0.7],
        k_max: k_max(),
        gamma_k: gamma_k(),
    };
    check_own(
        "Φ_θ,K",
        &theta_k,
        &Mapped::<_, Defaults>::new(theta_k, warped),
    );
}

negative_control!(
    energy_axes_warped_bitwise_own,
    "a chart under the linear option is not the warped chart, so it must fail",
    expected = "is not bitwise its own chart",
    {
        let inv = invariant();
        let linear =
            Mapped::<_, Defaults>::new(InvariantLK(inv), [Mapping::Warped, Mapping::Linear]);
        check_own("(L_z, K)", &InvariantLK(inv), &linear);
    }
);

/// The warp's rule (`invariant_rule` through `decode_rule`) is the invariant chart's own decode, bit for bit, over the
/// grid, on `inv`: the rule's machinery adds nothing to the warp. `through_e` forms `K*` as `(U + K(t)) − U` (the
/// control), rounding off the chart's `K* = K(t)`.
fn check_rule(inv: Invariant<f64>, through_e: bool) {
    let u_pot = kernel::chart::energy::centre_potential::<Defaults, f64>(
        &inv.controls(),
        &LinkParams::DEFAULT,
    );
    for [u, v] in grid(0.0, 1.0) {
        for energy in [false, true] {
            let mut rule = kernel::chart::energy::invariant_rule(
                &inv,
                mapped_all(Mapping::Warped),
                energy,
                0.0,
                u,
                v,
            );
            if through_e {
                rule.k0 = (u_pot + rule.k0) - u_pot;
            }
            let (d, _) = decode_rule::<Defaults, f64>(rule, &inv.frozen(), &LinkParams::DEFAULT);
            let own = InvariantLK(inv).decode::<Defaults, f64>(u, v);
            assert!(
                bits(&d) == bits(&own),
                "the warp's rule at ({u}, {v}) is not the chart's decode"
            );
        }
    }
}

/// An invariant chart off the placeholders, `K_max = 0.3`, `γ_K = 1.5`, whose `K(t)` is not a short dyadic.
fn off_placeholders() -> Invariant<f64> {
    Invariant {
        k_max: 0.3,
        gamma_k: 1.5,
        ..invariant()
    }
}

#[test]
fn energy_axes_warped_rule_is_the_warp() {
    check_rule(invariant(), false);
    check_rule(off_placeholders(), false);
}

negative_control!(
    energy_axes_warped_rule_is_the_warp,
    "a rule reading the potential on the warp (E = U + K(t) then K* = E − U) rounds off the chart's K*, so it must fail",
    expected = "is not the chart's decode",
    check_rule(off_placeholders(), true)
);

/// A lone `E` axis decodes `E = U + K_max·t^γ_K`, `U` the pixel's own potential (which varies along `α`): `K(p)` is
/// `K_max·t^γ_K` and `L_z(p)` the held 0, each within REQ-DEC-046's tolerances.
fn check_lone_e(chart: &MixedEnergy<Alpha, AxisE, Defaults, f64>) {
    let eps = f64::EPSILON;
    let mut potentials = Vec::new();
    for [u, v] in grid(0.0, 1.0) {
        let d = chart.decode::<Defaults, f64>(u, v);
        let (lz, k) = lz_k(&d);
        let want = k_max() * v.powf(gamma_k());
        let e = d.descriptor.V_0 + want;
        assert!(
            (k - want).abs() <= TOL_K * eps * want.max(f64::MIN_POSITIVE) + f64::MIN_POSITIVE
                && lz.abs() <= TOL_L * eps * (2.0 * want).sqrt().max(f64::MIN_POSITIVE)
                && (d.descriptor.V_0 + d.descriptor.K_0 - e).abs() <= 1e-12 * (1.0 + e.abs()),
            "the lone E axis at ({u}, {v}) does not decode E = U + K_max·t^γ_K: K = {k}, want {want}, L_z = {lz}"
        );
        potentials.push(d.descriptor.V_0);
    }
    let spread = potentials.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - potentials.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(
        spread > 1e-3,
        "the lone E axis's U does not vary across the window"
    );
}

#[test]
fn energy_axes_warped_lone_e() {
    check_lone_e(&lone::<AxisE>(0.0, Mapping::Warped));
}

negative_control!(
    energy_axes_warped_lone_e,
    "E mapped linearly from the centre's U puts K* = E − U off K_max·t^γ_K, so it must fail",
    expected = "does not decode E = U + K_max·t^γ_K",
    check_lone_e(&lone::<AxisE>(0.0, Mapping::Linear))
);

// ── R-508: the t axis continues past t = 1, bounded below t = 0; s bounded ────────────────────────────────────────────

/// The `t` axis declared `sides` against what its formula `k(t)` decodes past each edge: past an affine side every
/// pixel is a system the square does not draw, `K` finite and above `K_max` (past `t = 1`) or outside `[0, K_max]`
/// (below `t = 0`); past a bounded side `validate(t)` refuses it.
fn check_t_sides(name: &str, sides: Sides, k: impl Fn(f64) -> f64, valid: impl Fn(f64) -> bool) {
    for i in 1..=32 {
        let past = f64::from(i) / 64.0;
        for (t, side) in [(1.0 + past, sides.above), (-past, sides.below)] {
            let kt = k(t);
            match side {
                ExtensionType::Affine => assert!(
                    kt.is_finite() && !(0.0..=k_max()).contains(&kt) && kt >= 0.0 && valid(t),
                    "{name} past its affine side at t = {t} decodes K = {kt}: not a system, or one the square draws"
                ),
                _ => assert!(!valid(t), "{name} past its bounded side at t = {t} is a system"),
            }
        }
    }
}

/// R-508 on `(L_z, K)`, `(L_z, E)` and `(ν, K)` at the warp's `γ_K = gamma`: declared types, the `t > 1` side
/// decoding `K = K_max·t^γ_K` and validating, the `s` axis refused past either edge. `below` overrides the `t` axis's
/// declared below side (the control).
fn check_r508(gamma: f64, below: Option<ExtensionType>) {
    let inv = Invariant {
        gamma_k: gamma,
        ..invariant()
    };
    let le = Mapped::<_, Defaults>::new(InvariantLE(inv), mapped_all(Mapping::Warped));
    let lk = Mapped::<_, Defaults>::new(InvariantLK(inv), mapped_all(Mapping::Warped));
    let burrau = Mapped::<_, Defaults>::new(
        Burrau {
            nu_range: [0.2, 0.9],
            k_max: k_max(),
            gamma_k: gamma,
        },
        mapped_all(Mapping::Warped),
    );
    let t_sides = Sides {
        below: ExtensionType::Bounded,
        above: ExtensionType::Affine,
    };
    for sides in [le.extension(), lk.extension(), burrau.extension()] {
        assert_eq!(
            sides[1], t_sides,
            "the warp's t axis is not bounded below 0 and affine past 1"
        );
    }
    assert_eq!(
        le.extension()[0],
        Sides::both(ExtensionType::Bounded),
        "the warp's s axis is not bounded"
    );
    let declared = Sides {
        below: below.unwrap_or(t_sides.below),
        ..t_sides
    };
    let k_of = |d: Decoded<f64>| lz_k(&d).1;
    check_t_sides(
        "(L_z, K)",
        declared,
        |t| k_of(lk.decode::<Defaults, f64>(0.5, t)),
        |t| lk.validate(0.5, t) == ValidationResult::Valid,
    );
    check_t_sides(
        "(L_z, E)",
        declared,
        |t| k_of(le.decode::<Defaults, f64>(0.25, t)),
        |t| le.validate(0.25, t) == ValidationResult::Valid,
    );
    check_t_sides(
        "(ν, K)",
        declared,
        |t| k_of(burrau.decode::<Defaults, f64>(0.5, t)),
        |t| burrau.validate(0.5, t) == ValidationResult::Valid,
    );
    for [u, t] in grid(0.0, 1.0) {
        let v = 1.5 * t;
        let (_, k) = lz_k(&lk.decode::<Defaults, f64>(u, v));
        let want = k_max() * v.powf(gamma);
        assert!(
            (k - want).abs() <= TOL_K * f64::EPSILON * want + f64::MIN_POSITIVE
                && lk.validate(u, v) == ValidationResult::Valid
                && le.validate(u, v) == ValidationResult::Valid,
            "(L_z, K) at ({u}, {v}) is not the warp's formula continued"
        );
        for s in [-u / 8.0 - 1e-9, 1.0 + u / 8.0 + 1e-9] {
            assert!(
                lk.validate(s, v) != ValidationResult::Valid
                    && le.validate(s, v) != ValidationResult::Valid,
                "the s axis past its edge at ({s}, {v}) is a system"
            );
        }
    }
}

#[test]
fn energy_axes_warped_t_continues() {
    check_r508(gamma_k(), None);
}

negative_control!(
    energy_axes_warped_t_continues,
    "the t axis declared affine below t = 0 at the even γ_K = 2 draws the t > 0 side's systems again (the fold), so it \
     must fail",
    expected = "past its affine side",
    check_r508(2.0, Some(ExtensionType::Affine))
);

#[test]
fn energy_axes_warped_t_continues_non_integer() {
    check_r508(1.5, None);
}

negative_control!(
    energy_axes_warped_t_continues_non_integer,
    "the t axis declared affine below t = 0 at a non-integer γ_K decodes no real K there, so it must fail",
    expected = "past its affine side",
    check_r508(1.5, Some(ExtensionType::Affine))
);

// ── R-506: the mass ratio through the mass link ───────────────────────────────────────────────────────────────────────

/// Every pixel of the grid over `[lo, hi]²` decodes masses positive and summing to 1, `masses(u, v)` the decoded masses.
fn check_simplex(name: &str, lo: f64, hi: f64, masses: impl Fn(f64, f64) -> [f64; 3]) {
    for [u, v] in grid(lo, hi) {
        let m = masses(u, v);
        assert!(
            m.iter().all(|&x| x > 0.0) && (m.iter().sum::<f64>() - 1.0).abs() <= 4.0 * f64::EPSILON,
            "{name} at ({u}, {v}) has masses {m:?}, not positive or not summing to 1"
        );
    }
}

fn ratio_chart(asked: [f64; 2], mapping: Mapping) -> MixedMassRatio<ZAlpha, Defaults, f64> {
    MixedMassRatio::new(
        Z0,
        [-2.0, 2.0],
        asked,
        [0, 1],
        mapping,
        &LinkParams::DEFAULT,
    )
    .expect("a ratio range")
}

#[test]
fn energy_axes_warped_mass_ratio() {
    for mapping in [Mapping::Warped, Mapping::Linear] {
        let chart = ratio_chart([0.25, 4.0], mapping);
        check_simplex("the mass-ratio axis", -0.5, 1.5, |u, v| {
            chart.decode::<Defaults, f64>(u, v).ic.m
        });
    }
}

negative_control!(
    energy_axes_warped_mass_ratio,
    "masses set from a ratio range reaching below 0 directly, not through the link, go negative, so it must fail",
    expected = "not positive or not summing to 1",
    {
        let chart = ratio_chart([-0.5, 4.0], Mapping::Linear);
        check_simplex("the ratio's masses", 0.0, 1.0, |_, v| ratio_masses([0, 1], chart.held, chart.ratio_at(v)));
    }
);

// ── REQ-CHART-068: the linear option ──────────────────────────────────────────────────────────────────────────────────

/// On `(L_z, E)` under the linear option: a pixel is not a system exactly where `|L_z| > √(2I(E − U))`, there are
/// some (the parabola's corners), each is never integrated (its decode `DEGENERATE(INVARIANT_INFEASIBLE)`,
/// `decode_failed`) and none warns; `decode` gives the decode the classification reads.
fn check_linear_corners(decode: impl Fn(f64, f64) -> Decoded<f64>) {
    let inv = invariant();
    let chart = Mapped::<_, Defaults>::new(InvariantLE(inv), mapped_all(Mapping::Linear));
    let lin = invariant::l_max(k_max());
    let mut blank = 0;
    for [u, v] in grid(0.0, 1.0) {
        let (k, lz) = (k_max() * v, (u + u - 1.0) * lin);
        let outside = infeasible(1.0, k, lz);
        let valid = chart.validate(u, v) == ValidationResult::Valid;
        assert!(
            valid != outside,
            "({u}, {v}) is classified against the parabola wrongly"
        );
        if outside {
            blank += 1;
            assert!(
                decode(u, v).degenerate == Some(Degenerate::InvariantInfeasible),
                "the corner pixel ({u}, {v}) is integrated"
            );
        }
    }
    assert!(blank > 0, "the linear option leaves no corner");
    assert!(
        !decoder_bug_warning(&chart.mapping),
        "a linear chart's corners warn"
    );
    assert!(
        decoder_bug_warning(&mapped_all(Mapping::Warped)),
        "a warped chart warns of nothing"
    );
}

#[test]
fn energy_axis_linear_option_corners() {
    let chart = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear));
    check_linear_corners(|u, v| chart.decode::<Defaults, f64>(u, v));
}

negative_control!(
    energy_axis_linear_option_corners,
    "corner pixels decoded as the warp's feasible states would be integrated, so it must fail",
    expected = "is integrated",
    {
        let warped = InvariantLE(invariant());
        check_linear_corners(|u, v| warped.decode::<Defaults, f64>(u, v));
    }
);

/// The pixels both mappings share, untilted (R-471 H4): the row `t = 1` and the point `(½, 0)` decode to the same
/// state, bit for bit, and the column `s = ½` has `L_z = 0` under both; `extra` adds points the check asserts equal
/// too (the control's centre).
fn check_shared(extra: &[[f64; 2]]) {
    let inv = invariant();
    for (energy, name) in [(true, "(L_z, E)"), (false, "(L_z, K)")] {
        let decode = |mapping: Mapping, u: f64, v: f64| {
            if energy {
                Mapped::<_, Defaults>::new(InvariantLE(inv), mapped_all(mapping))
                    .decode::<Defaults, f64>(u, v)
            } else {
                Mapped::<_, Defaults>::new(InvariantLK(inv), mapped_all(mapping))
                    .decode::<Defaults, f64>(u, v)
            }
        };
        let row = (0..N).map(|i| [f64::from(i) / f64::from(N - 1), 1.0]);
        for [u, v] in row.chain([[0.5, 0.0]]).chain(extra.iter().copied()) {
            assert!(
                bits(&decode(Mapping::Warped, u, v)) == bits(&decode(Mapping::Linear, u, v)),
                "{name} at ({u}, {v}) differs between the mappings"
            );
        }
        for j in 0..N {
            let v = f64::from(j) / f64::from(N - 1);
            for mapping in [Mapping::Warped, Mapping::Linear] {
                let (lz, k) = lz_k(&decode(mapping, 0.5, v));
                assert!(
                    lz.abs() <= TOL_L * f64::EPSILON * (2.0 * k).sqrt().max(f64::MIN_POSITIVE),
                    "{name} on the column s = ½ at t = {v} has L_z = {lz}"
                );
            }
        }
    }
}

#[test]
fn energy_axis_linear_option_shared() {
    check_shared(&[]);
}

negative_control!(
    energy_axis_linear_option_shared,
    "the centre (½, ½) decodes K_max·2^(−γ_K) warped and K_max/2 linear, equal only at γ_K = 1, so asserting it equal \
     must fail",
    expected = "differs between the mappings",
    check_shared(&[[0.5, 0.5]])
);

/// R-408's count over the window `[−¼, 1¼]²` of `(L_z, E)` under the linear option, through `sides`: every pixel is
/// counted (both axes affine) and exactly those `validate` refuses are forbidden (B1).
fn check_linear_count(sides: [Sides; 2]) {
    let chart = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear));
    let (mut forbidden, mut refused, mut total) = (0, 0, 0);
    for [u, v] in grid(-0.25, 1.25) {
        let valid = chart.validate(u, v) == ValidationResult::Valid;
        refused += u32::from(!valid);
        match count(sides, [u, v], valid) {
            Count::Forbidden => {
                forbidden += 1;
                total += 1;
            }
            Count::System => total += 1,
            Count::DomainEnd | Count::NotCounted => {}
        }
    }
    assert!(
        forbidden == refused && total == N * N && forbidden > 0,
        "R-408's count of the linear chart is not B1's: {forbidden} forbidden of {total}, {refused} refused"
    );
}

#[test]
fn energy_axis_linear_option_count() {
    let chart = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear));
    assert_eq!(chart.extension(), [Sides::both(ExtensionType::Affine); 2]);
    check_linear_count(chart.extension());
}

negative_control!(
    energy_axis_linear_option_count,
    "the warp's types on the linear chart make its pixels past the edges the domain's end, out of the count, so it must \
     fail",
    expected = "is not B1's",
    {
        let warped = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Warped));
        check_linear_count(warped.extension());
    }
);

/// A mass-ratio axis under the linear option on `[−½, 2]`: every pixel whose ratio is at or below 0 is refused by
/// `chart`'s check, and the rest validate.
fn check_ratio_blank(chart: &MixedMassRatio<ZAlpha, Defaults, f64>) {
    let linear = ratio_chart([-0.5, 2.0], Mapping::Linear);
    let mut blank = 0;
    for [u, v] in grid(0.0, 1.0) {
        let q = linear.ratio_at(v);
        let valid = chart.validate(u, v) == ValidationResult::Valid;
        assert!(
            valid == (q > 0.0),
            "the ratio {q} at ({u}, {v}) is not blank exactly where it is not positive"
        );
        blank += u32::from(!valid);
    }
    assert!(blank > 0, "no ratio at or below 0");
}

#[test]
fn energy_axis_linear_option_ratio() {
    check_ratio_blank(&ratio_chart([-0.5, 2.0], Mapping::Linear));
}

negative_control!(
    energy_axis_linear_option_ratio,
    "the warped axis, clamped to the ratios the link realises, draws no ratio at or below 0, so it must fail",
    expected = "is not blank exactly where it is not positive",
    check_ratio_blank(&ratio_chart([-0.5, 2.0], Mapping::Warped))
);

/// The linear option on the Burrau `(ν, K)` chart: `K = K_max·v`, every pixel a system, `v < 0` refused.
#[test]
fn energy_axis_linear_option_burrau() {
    let burrau = Burrau {
        nu_range: [0.2, 0.9],
        k_max: k_max(),
        gamma_k: gamma_k(),
    };
    let chart = Mapped::<_, Defaults>::new(burrau, [Mapping::Warped, Mapping::Linear]);
    check_burrau_linear(&chart, |v| k_max() * v);
}

/// Every pixel of the grid a system with `K(p) = want(v)`, and `v < 0` not.
fn check_burrau_linear<C: Chart<Param = f64>>(chart: &C, want: impl Fn(f64) -> f64) {
    for [u, v] in grid(0.0, 1.0) {
        let (_, k) = lz_k(&chart.decode::<Defaults, f64>(u, v));
        assert!(
            chart.validate(u, v) == ValidationResult::Valid
                && (k - want(v)).abs() <= TOL_K * f64::EPSILON * want(v) + f64::MIN_POSITIVE,
            "the linear K axis at ({u}, {v}) decodes K = {k}, not K_max·v"
        );
    }
    assert!(
        chart.validate(0.5, -0.25) != ValidationResult::Valid,
        "K < 0 is a system"
    );
}

negative_control!(
    energy_axis_linear_option_burrau,
    "the warped (ν, K) chart decodes K_max·v^γ_K, not K_max·v, so it must fail",
    expected = "not K_max·v",
    {
        let burrau = Burrau {
            nu_range: [0.2, 0.9],
            k_max: k_max(),
            gamma_k: gamma_k(),
        };
        check_burrau_linear(&burrau, |v| k_max() * v);
    }
);

// ── REQ-CHART-069: the warp per pixel under a tilt ────────────────────────────────────────────────────────────────────

/// The view's centre, in the chart's own parameters.
const CENTRE: [f64; 2] = [0.5, 0.5];
/// The targets' slider spans.
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

/// How the check decodes a tilted invariant chart's pixel: through the chart (per pixel), or with the warp computed
/// once at the centre (the control): `E = U₀ + K(t)` and `L_max(t)` from the centre, `K* = E − U` at the pixel.
#[derive(Clone, Copy, PartialEq)]
enum Warp {
    PerPixel,
    #[cfg(feature = "controls")]
    Centre,
}

/// `chart` tilted by `spec`: every pixel of the 65 × 65 grid over `[0,1]²` whose `(a, b)` lie in the square validates,
/// and its decode meets the targets of its own `U`, `I` and masses: feasible, `K(p) = K(b)`, `L_z(p) = (2a − 1)·L_max`.
fn check_tilted<C>(name: &str, chart: C, spec: &TiltSpec, warp: Warp)
where
    C: Tiltable<Param = f64> + ChartDecode + Copy + InvariantOf,
{
    let view = Tilted::new(chart, Tilt::new(spec, CENTRE, SPAN)).expect("a hidden target");
    let inv = chart.inv();
    for [u, v] in grid(0.0, 1.0) {
        let ([a, b], shift) = view.at(u, v);
        if !((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)) {
            continue;
        }
        assert!(shift.is_some(), "{name}: no tilt at ({u}, {v})");
        let d = match warp {
            Warp::PerPixel => view.decode::<Defaults, f64>(u, v),
            #[cfg(feature = "controls")]
            Warp::Centre => {
                let shift = shift.unwrap();
                let u0 = kernel::chart::energy::centre_potential::<Defaults, f64>(
                    &inv.controls(),
                    &LinkParams::DEFAULT,
                );
                let k = invariant::kinetic(inv.k_max, inv.gamma_k, b);
                let rule = Rule {
                    k0: u0 + k,
                    weight: 1.0,
                    u_ref: 0.0,
                    s2: 0.0,
                    lz: (a + a - 1.0) * invariant::l_max(k),
                    lz_on_warp: false,
                };
                let z = Latent::new(shift_controls(inv.controls(), &shift));
                decode_rule::<Defaults, f64>(rule, &z, &LinkParams::DEFAULT).0
            }
        };
        let (lz, k) = lz_k(&d);
        let want_k = invariant::kinetic(inv.k_max, inv.gamma_k, b);
        let want_lz = (a + a - 1.0) * invariant::l_max(want_k);
        assert!(
            view.validate(u, v) == ValidationResult::Valid
                && d.degenerate.is_none()
                && (k - want_k).abs() <= TOL_K * f64::EPSILON * want_k + f64::MIN_POSITIVE
                && (lz - want_lz).abs()
                    <= TOL_L * f64::EPSILON * (want_lz.abs() + (2.0 * want_k).sqrt()) + f64::MIN_POSITIVE,
            "{name} tilted ({}°, {}°) into z[{}] at ({u}, {v}) leaves the feasible region: K = {k}, want {want_k}",
            spec.tau1,
            spec.tau2,
            spec.x
        );
    }
}

/// The frozen part of an invariant chart.
trait InvariantOf {
    fn inv(&self) -> Invariant<f64>;
}

impl InvariantOf for InvariantLE<f64> {
    fn inv(&self) -> Invariant<f64> {
        self.0
    }
}

impl InvariantOf for InvariantLK<f64> {
    fn inv(&self) -> Invariant<f64> {
        self.0
    }
}

/// The tilts REQ-CHART-069 names: a single `τ` into each target and `(30°, 20°)` under both maps.
fn tilts(x: u32) -> Vec<TiltSpec> {
    let mut all: Vec<TiltSpec> = [10.0, 45.0, 80.0]
        .iter()
        .flat_map(|&t| {
            [
                spec(t, 0.0, TiltMap::Sheet, x),
                spec(0.0, t, TiltMap::Sheet, x),
            ]
        })
        .collect();
    all.push(spec(30.0, 20.0, TiltMap::Lean, x));
    all.push(spec(30.0, 20.0, TiltMap::Sheet, x));
    all
}

#[test]
fn energy_warp_per_pixel_tilted() {
    let inv = invariant();
    for x in [6, 7, 0, 1] {
        for s in tilts(x) {
            check_tilted("(L_z, E)", InvariantLE(inv), &s, Warp::PerPixel);
            check_tilted("(L_z, K)", InvariantLK(inv), &s, Warp::PerPixel);
        }
    }
}

negative_control!(
    energy_warp_per_pixel_tilted,
    "(L_z, E)'s warp computed once at the centre leaves the pixels whose own U lies above the centre's with \
     K < L_z²/2I, so it must fail",
    expected = "leaves the feasible region",
    {
        let inv = invariant();
        for x in [0, 1, 6, 7] {
            for s in tilts(x) {
                check_tilted("(L_z, E)", InvariantLE(inv), &s, Warp::Centre);
            }
        }
    }
);

/// A tilted lone `E` axis beside `z_μ1`, tilted into a shape control, and the linear `(L_z, E)` chart tilted into a
/// mass logit: the pixel's own `U` read at every pixel, `validate` and the decode agreeing pixel by pixel.
#[test]
fn energy_warp_per_pixel_mixed_and_linear() {
    let mixed: MixedEnergy<ZMu1, AxisE, Defaults, f64> =
        MixedEnergy::new(Z0, [-2.0, 2.0], k_max(), gamma_k(), 0.0, Mapping::Warped);
    for s in tilts(0) {
        let view = Tilted::new(mixed, Tilt::new(&s, CENTRE, SPAN)).expect("a hidden target");
        check_every_pixel("z_μ1 × E, tilted", |u, v| {
            let ([a, b], _) = view.at(u, v);
            let inside = (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b);
            if inside {
                (
                    view.validate(u, v),
                    view.decode::<Defaults, f64>(u, v).degenerate,
                )
            } else {
                (ValidationResult::Valid, None)
            }
        });
    }
    let linear = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear));
    check_agree(&linear, &spec(45.0, 0.0, TiltMap::Sheet, 6));
}

/// The tilted view's `validate` refuses exactly the pixels whose decode the construction finds infeasible.
fn check_agree<C: Tiltable<Param = f64> + Copy>(chart: &C, s: &TiltSpec) {
    let view = Tilted::new(*chart, Tilt::new(s, CENTRE, SPAN)).expect("a hidden target");
    let mut blank = 0;
    for [u, v] in grid(0.0, 1.0) {
        let infeasible =
            view.decode::<Defaults, f64>(u, v).degenerate == Some(Degenerate::InvariantInfeasible);
        let valid = view.validate(u, v) == ValidationResult::Valid;
        assert!(
            valid != infeasible,
            "the tilted linear chart's check at ({u}, {v}) is not its decode's"
        );
        blank += u32::from(infeasible);
    }
    assert!(blank > 0, "the tilted linear chart has no corner");
}

negative_control!(
    energy_warp_per_pixel_mixed_and_linear,
    "the tilted linear chart's check read at (a, b) without its hidden offsets misses the pixels the tilt moves, so it \
     must fail",
    expected = "is not its decode's",
    {
        struct Unshifted(Mapped<InvariantLE<f64>, Defaults>);
        impl Clone for Unshifted {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl Copy for Unshifted {}
        impl Chart for Unshifted {
            type Param = f64;
            fn map<F: kernel::chart::Float + From<f64>>(&self, u: F, v: F) -> kernel::chart::ChartOut<F> {
                self.0.map(u, v)
            }
            fn name(&self) -> &str {
                self.0.name()
            }
            fn validate(&self, u: f64, v: f64) -> ValidationResult {
                self.0.validate(u, v)
            }
            fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
            where
                S: kernel::generated::links::Selection,
                R: kernel::generated::links::LinkReal + kernel::payload::PayloadReal + From<f64>,
            {
                self.0.decode::<S, R>(u, v)
            }
        }
        impl kernel::chart::tilt::Hidden for Unshifted {
            fn hidden(&self, k: u32) -> bool {
                self.0.hidden(k)
            }
            fn sliced(&self, k: u32, value: f64) -> Option<Self> {
                self.0.sliced(k, value).map(Unshifted)
            }
        }
        impl Tiltable for Unshifted {
            fn decode_shifted<S, R>(
                &self,
                u: R,
                v: R,
                shift: &kernel::chart::tilt::Shift<R>,
                p: &LinkParams<R>,
            ) -> Decoded<R>
            where
                S: kernel::generated::links::Selection,
                R: kernel::generated::links::LinkReal + kernel::payload::PayloadReal + From<f64>,
            {
                self.0.decode_shifted::<S, R>(u, v, shift, p)
            }
        }
        let linear = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear));
        check_agree(&Unshifted(linear), &spec(45.0, 0.0, TiltMap::Sheet, 6));
    }
);

/// Untilted, a tilted view of each invariant chart under each mapping is the chart's own, bit for bit.
#[test]
fn energy_warp_per_pixel_untilted_bitwise() {
    let inv = invariant();
    for mapping in [mapped_all(Mapping::Warped), mapped_all(Mapping::Linear)] {
        check_untilted(Mapped::<_, Defaults>::new(InvariantLE(inv), mapping), false);
        check_untilted(Mapped::<_, Defaults>::new(InvariantLK(inv), mapping), false);
    }
}

/// At `τ = 0` no tilt path and every entry the chart's own; `identity_path` forces the path on (the control).
fn check_untilted<C: Tiltable<Param = f64> + ChartDecode + Copy>(chart: C, identity_path: bool) {
    let mut view = Tilted::new(
        chart,
        Tilt::new(&spec(0.0, 0.0, TiltMap::Sheet, 6), CENTRE, SPAN),
    )
    .unwrap();
    if identity_path {
        view = Tilted::new(
            chart,
            Tilt {
                on: true,
                ..*view.tilt()
            },
        )
        .unwrap();
    }
    for [u, v] in grid(0.0, 1.0) {
        assert!(
            bits(&view.decode::<Defaults, f64>(u, v)) == bits(&chart.decode::<Defaults, f64>(u, v))
                && view.validate(u, v) == chart.validate(u, v),
            "{} at ({u}, {v}) is not bitwise the untilted chart's",
            chart.name()
        );
    }
}

negative_control!(
    energy_warp_per_pixel_untilted_bitwise,
    "a tilt path run with an identity rotation refuses what the untilted chart projects, so it must fail",
    expected = "is not bitwise the untilted chart's",
    check_untilted(
        Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear)),
        true
    )
);

// ── REQ-CHART-070: PE, the virial ratio and the mass ratio ────────────────────────────────────────────────────────────

/// No chart offers `PE` (`written` refuses it, and its extension with it); every other quantity is written.
fn check_pe(written: impl Fn(Quantity) -> Result<(), Refused>) {
    for q in Quantity::ALL {
        let offered = written(q).is_ok();
        assert!(
            offered == (q != Quantity::PE),
            "{q:?} is offered: {offered}"
        );
        if q == Quantity::PE {
            assert_eq!(
                written(q),
                Err(Refused::NoWarpWritten(Quantity::PE)),
                "PE is offered"
            );
        }
    }
    for m in [Mapping::Warped, Mapping::Linear] {
        assert!(
            extension(Quantity::PE, m).is_err(),
            "PE has an extension type"
        );
    }
}

#[test]
fn pe_axis_refused() {
    check_pe(written);
    let energy_axes = [
        AxisE::QUANTITY,
        AxisK::QUANTITY,
        AxisKE::QUANTITY,
        AxisLz::QUANTITY,
        AxisVirial::QUANTITY,
    ];
    assert!(
        energy_axes.iter().all(|q| written(*q).is_ok()),
        "a chart offers an axis with no warp written"
    );
    assert!(Quantity::PE.energy_type() && !Quantity::VirialRatio.energy_type());
}

negative_control!(
    pe_axis_refused,
    "an axis mapped with no written warp is accepted, so it must fail",
    expected = "is offered",
    check_pe(|_| Ok(()))
);

/// The virial-ratio axis beside `α`: every pixel validates and decodes `2K/|U| = Q_max·t^γ_K` on its own `U`; `per`
/// gives the decode's `K*` from the pixel's potential (the chart's own, or the control's, the centre's `U₀`).
fn check_virial(per_pixel: bool) {
    let chart = lone::<AxisVirial>(0.0, Mapping::Warped);
    let u0 = kernel::chart::energy::centre_potential::<Defaults, f64>(&Z0, &LinkParams::DEFAULT);
    for [u, v] in grid(0.0, 1.0) {
        let q = k_max() * v.powf(gamma_k());
        let d = if per_pixel {
            chart.decode::<Defaults, f64>(u, v)
        } else {
            let mut z = Z0;
            z[0] = Alpha::control_at::<Defaults, f64>(0.2 + 1.1 * u, &LinkParams::DEFAULT);
            let rule = Rule {
                k0: q * u0.abs() / 2.0,
                weight: 0.0,
                u_ref: 0.0,
                s2: 0.0,
                lz: 0.0,
                lz_on_warp: false,
            };
            decode_rule::<Defaults, f64>(rule, &Latent::new(z), &LinkParams::DEFAULT).0
        };
        let (_, k) = lz_k(&d);
        let ratio = 2.0 * k / d.descriptor.V_0.abs();
        assert!(
            chart.validate(u, v) == ValidationResult::Valid
                && d.degenerate.is_none()
                && (ratio - q).abs() <= 1e-12 * (1.0 + q),
            "the virial axis at ({u}, {v}) decodes 2K/|U| = {ratio}, not {q}"
        );
    }
    assert_eq!(chart.extension(), Sides::both(ExtensionType::Bounded));
}

#[test]
fn physical_axis_warps_virial() {
    check_virial(true);
}

negative_control!(
    physical_axis_warps_virial,
    "K* = Q·|U₀|/2 from the centre's potential misses the pixel's own virial ratio along α, so it must fail",
    expected = "decodes 2K/|U|",
    check_virial(false)
);

/// How a mass-ratio arm forms its mass controls: the chart's own (its link's inverse at the ratio's masses), or the
/// control's formula written for softmax_tanh at `μ_max = 5` alone, `m_i/m₀ = exp(5·tanh z_i)` solved for `z_i`.
#[derive(Clone, Copy, PartialEq)]
enum Formula {
    Own,
    #[cfg(feature = "controls")]
    Softmax5,
}

/// The mass-ratio axis on `[0.05, 20]` of each body pair, under `stick_breaking` and under `softmax_tanh` at both ends
/// of `μ_max`'s range (REQ-GEN-038's, `validation::tunables`) and at its default: every pixel of `[0,1]²` validates and
/// decodes masses positive, summing to 1, with the ratio `q(t)`.
fn check_mass_ratio(formula: Formula) {
    let range = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's range");
    for pair in [[0, 1], [0, 2], [1, 2], [2, 0]] {
        check_ratio_arm::<Stick>("stick_breaking", pair, &LinkParams::DEFAULT, formula);
        for mu_max in [range.lo, range.default, range.hi] {
            let p = LinkParams {
                mu_max,
                ..LinkParams::DEFAULT
            };
            check_ratio_arm::<Defaults>("softmax_tanh", pair, &p, formula);
        }
    }
}

/// One arm of [`check_mass_ratio`] under the selection `S` at `p`.
fn check_ratio_arm<S: Selection>(
    link: &str,
    pair: [u32; 2],
    p: &LinkParams<f64>,
    formula: Formula,
) {
    let chart = MixedMassRatio::<ZBeta, S, f64>::new(
        Z0,
        [-2.0, 2.0],
        [0.05, 20.0],
        pair,
        Mapping::Warped,
        p,
    )
    .expect("a realised range");
    for [u, v] in grid(0.0, 1.0) {
        let q = chart.ratio_at(v);
        let z = match formula {
            Formula::Own => {
                let z = chart.controls_from(Z0, u, v, p);
                [z[6], z[7]]
            }
            #[cfg(feature = "controls")]
            Formula::Softmax5 => {
                let m = ratio_masses(pair, chart.held, q);
                let z = |i: usize| ((m[i] / m[0]).ln() / 5.0).atanh();
                [z(1), z(2)]
            }
        };
        let m = <S::MassLink as Link<2, 3>>::forward_at(z, p);
        let ratio = ratio_of(pair, m);
        assert!(
            chart.validate_params(u, v, p) == ValidationResult::Valid
                && m.iter().all(|&x| x > 0.0)
                && (m.iter().sum::<f64>() - 1.0).abs() <= 4.0 * f64::EPSILON
                && (ratio - q).abs() <= RATIO_TOL * q,
            "the mass ratio {pair:?} under {link} at μ_max = {} at ({u}, {v}) decodes {ratio}, not {q}",
            p.mu_max
        );
    }
}

#[test]
fn physical_axis_warps_mass_ratio() {
    check_mass_ratio(Formula::Own);
    let chart = ratio_chart([0.25, 4.0], Mapping::Warped);
    assert_eq!(chart.extension(), Sides::both(ExtensionType::Bounded));
    let decoded = chart.decode::<Defaults, f64>(0.3, 0.6);
    let q = chart.ratio_at(0.6);
    assert!(
        (ratio_of([0, 1], decoded.ic.m) - q).abs() <= RATIO_TOL * q,
        "the chart's decode is not its ratio"
    );
}

negative_control!(
    physical_axis_warps_mass_ratio,
    "a mass-ratio warp written for softmax_tanh at μ_max = 5 alone decodes another ratio under stick_breaking or at an \
     end of μ_max's range, so it must fail",
    expected = "decodes",
    check_mass_ratio(Formula::Softmax5)
);

/// `held` is the third body's mass at the centre under the chart's link.
#[test]
fn physical_axis_warps_mass_ratio_held() {
    let chart = ratio_chart([0.25, 4.0], Mapping::Warped);
    let m = mass::decode::<Defaults, f64>([Z0[6], Z0[7]]).m;
    assert_eq!(
        chart.held, m[2],
        "the held mass is not the third body's at z₀"
    );
    assert!(MixedMassRatio::<ZAlpha, Defaults, f64>::new(
        Z0,
        [-2.0, 2.0],
        [4.0, 0.25],
        [0, 1],
        Mapping::Warped,
        &LinkParams::DEFAULT
    )
    .is_none());
    assert!(MixedMassRatio::<ZAlpha, Defaults, f64>::new(
        Z0,
        [-2.0, 2.0],
        [0.25, 4.0],
        [1, 1],
        Mapping::Warped,
        &LinkParams::DEFAULT
    )
    .is_none());
}

negative_control!(
    physical_axis_warps_mass_ratio_held,
    "the first body's mass is not the held third's, so it must fail",
    expected = "is not the third body's",
    {
        let chart = ratio_chart([0.25, 4.0], Mapping::Warped);
        let m = mass::decode::<Defaults, f64>([Z0[6], Z0[7]]).m;
        assert_eq!(
            chart.held, m[0],
            "the held mass is not the third body's at z₀"
        );
    }
);
