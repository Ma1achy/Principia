//! Physical-quantity axes warped by default, a linear mapping an option per axis, the warp per pixel (R-471, R-506,
//! R-508; chart_reference §2.1, §5.4; `kernel::chart::energy`, `kernel::chart::physical`):
//! - `energy_axes_warped_*` (REQ-CHART-067): every chart with an energy-type axis validates every pixel of a 65 × 65
//!   grid over `[0,1]²`, its decode feasible; the existing charts are bitwise their own under the default mapping; a
//!   lone `E` axis decodes `E = U + K_max·t^γ_K`; the warp's `t` axis continues past `t = 1` and is bounded below
//!   `t = 0`, its `s` axis bounded, and R-408 counts it so; a mass-ratio axis keeps every pixel's masses positive and
//!   summing to 1.
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
use kernel::chart::physical::ratio_masses;
use kernel::chart::physical::{
    ratio_of, AxisE, AxisK, AxisKE, AxisLz, AxisVirial, EnergyAxis, MixedEnergy, MixedMassRatio,
};
#[cfg(feature = "controls")]
use kernel::chart::tilt::shift_controls;
use kernel::chart::tilt::{Hidden, Shift, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, Constraint, InvariantLE, InvariantLK,
    Tilt, TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::decode::invariant::{infeasible, Outcome};
use kernel::decode::{mass, Decoded, Degenerate, Latent};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Defaults, Link, LinkParams, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    SoftmaxTanh, StickBreaking,
};
use validation::{negative_control, tunables, GAMMA_K, K_MAX};

/// The grid side: 65 × 65 points `i/64` over `[0,1]²`, its edges included.
const N: u32 = 65;
/// `|K(p) − K*|` in units of `eps·K*`, at most (REQ-DEC-046, proposed; `chart_invariant.rs`).
const TOL_K: f64 = 8.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))`, at most (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// A mass ratio's decode against its axis value, relative: the realisation's `ε_μ`, the link clamp
/// (chart_decoder_contract § "Three hard requirements on any registered link", item 2; `MixedMassRatio::realised`).
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

/// A lone `L_z` axis decodes its line of the warp at the held `K`: `K(p) = K` and `L_z(p) = (2t − 1)·L_max(K)`, each
/// within REQ-DEC-046's tolerances, at every pixel of the grid; `line(t)` is the line's `2t − 1` as the check reads it.
fn check_lone_lz(line: fn(f64) -> f64) {
    let held = k_max();
    let chart = lone::<AxisLz>(held, Mapping::Warped);
    let eps = f64::EPSILON;
    let l_max = invariant::l_max(held);
    for [u, v] in grid(0.0, 1.0) {
        let (lz, k) = lz_k(&chart.decode::<Defaults, f64>(u, v));
        let want = line(v) * l_max;
        assert!(
            (k - held).abs() <= TOL_K * eps * held && (lz - want).abs() <= TOL_L * eps * l_max,
            "the lone L_z axis at ({u}, {v}) decodes L_z = {lz}, K = {k}, not (2t − 1)·L_max(K) = {want} at K = {held}"
        );
    }
}

#[test]
fn energy_axes_warped_lone_lz() {
    check_lone_lz(|t| t + t - 1.0);
}

negative_control!(
    energy_axes_warped_lone_lz,
    "the line read as 1 − 2t, L_z's sign flipped, must fail",
    expected = "not (2t − 1)·L_max(K)",
    check_lone_lz(|t| 1.0 - t - t)
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

/// A mapped chart is its chart and its mappings, and no more: equal where both are, unequal where either differs, and
/// its debug form names both.
fn check_mapped_value(other: Invariant<f64>) {
    let inv = invariant();
    let at = |i: Invariant<f64>, m: [Mapping; 2]| Mapped::<_, Defaults>::new(InvariantLK(i), m);
    let warped = mapped_all(Mapping::Warped);
    let linear = [Mapping::Warped, Mapping::Linear];
    assert!(
        at(inv, linear) == at(inv, linear)
            && at(inv, linear) != at(inv, warped)
            && at(inv, linear) != at(other, linear),
        "a mapped chart's equality is not its chart's and its mappings'"
    );
    let text = format!("{:?}", at(inv, linear));
    assert!(
        text.starts_with("Mapped {")
            && text.contains(&format!("{:?}", InvariantLK(inv)))
            && text.contains("[Warped, Linear]"),
        "the debug form {text}"
    );
}

#[test]
fn energy_axis_linear_option_mapped_value() {
    check_mapped_value(off_placeholders());
}

negative_control!(
    energy_axis_linear_option_mapped_value,
    "the same chart read as another must fail",
    expected = "is not its chart's and its mappings'",
    check_mapped_value(invariant())
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

/// A lone `E` axis under the linear option decodes `E = U₀ + K_max·t`, `U₀` the slice centre's potential, wherever
/// `validate` takes the pixel: `K* = E − U` at the pixel's own `U`, which varies along `α`, so the decoded `K` is not
/// `K_max·t` there.
fn check_linear_lone_e(chart: &MixedEnergy<Alpha, AxisE, Defaults, f64>) {
    let u0 = kernel::chart::energy::centre_potential::<Defaults, f64>(
        &chart.frozen,
        &LinkParams::DEFAULT,
    );
    let (mut valid, mut off) = (0, 0);
    for [u, v] in grid(0.0, 1.0) {
        if chart.validate(u, v) != ValidationResult::Valid {
            continue;
        }
        valid += 1;
        let d = chart.decode::<Defaults, f64>(u, v);
        let e = d.descriptor.V_0 + d.descriptor.K_0;
        let want = u0 + k_max() * v;
        assert!(
            (e - want).abs() <= 1e-12 * (1.0 + want.abs()),
            "the linear E axis at ({u}, {v}) decodes E = {e}, not U₀ + K_max·t = {want}"
        );
        off += u32::from((d.descriptor.K_0 - k_max() * v).abs() > 1e-6);
    }
    assert!(
        valid > 0 && off > 0,
        "{valid} valid pixels, {off} with K* ≠ K_max·t"
    );
}

#[test]
fn energy_axis_linear_option_lone_e() {
    check_linear_lone_e(&lone::<AxisE>(0.0, Mapping::Linear));
}

negative_control!(
    energy_axis_linear_option_lone_e,
    "the warped E axis decodes U + K_max·t^γ_K, not U₀ + K_max·t, so it must fail",
    expected = "not U₀ + K_max·t",
    check_linear_lone_e(&lone::<AxisE>(0.0, Mapping::Warped))
);

/// A lone `K` axis beside `α`.
type LoneK = MixedEnergy<Alpha, AxisK, Defaults, f64>;
/// A mass-ratio axis beside `z_α`.
type RatioAlpha = MixedMassRatio<ZAlpha, Defaults, f64>;

/// A physical-quantity mixed-axis chart is its parameters and no more: equal where every one is, unequal where any
/// one differs; its debug form names the chart.
fn check_physical_values(differ: bool) {
    let e = lone::<AxisK>(0.0, Mapping::Warped);
    let edit = |f: &dyn Fn(&mut LoneK)| {
        let mut c = e;
        f(&mut c);
        c
    };
    let changed = [
        edit(&|c| c.frozen = [0.0; 8]),
        edit(&|c| c.a_range = [0.0, 1.0]),
        edit(&|c| c.top = 2.0),
        edit(&|c| c.bottom = 0.5),
        edit(&|c| c.gamma_k = 1.0),
        edit(&|c| c.held = 0.5),
        edit(&|c| c.mapping = Mapping::Linear),
    ];
    for c in changed {
        let copy = c;
        assert!(
            (c != e) == differ && c == copy,
            "a lone K chart's equality is not its parameters': {c:?}"
        );
    }
    let r = ratio_chart([0.1, 10.0], Mapping::Warped);
    let edit = |f: &dyn Fn(&mut RatioAlpha)| {
        let mut c = r;
        f(&mut c);
        c
    };
    let changed = [
        edit(&|c| c.frozen = [0.0; 8]),
        edit(&|c| c.a_range = [0.0, 1.0]),
        edit(&|c| c.ratio = [0.2, 5.0]),
        edit(&|c| c.bodies = [1, 2]),
        edit(&|c| c.held = 0.25),
        edit(&|c| c.mapping = Mapping::Linear),
    ];
    for c in changed {
        let copy = c;
        assert!(
            (c != r) == differ && c == copy,
            "a mass-ratio chart's equality is not its parameters': {c:?}"
        );
    }
    let (te, tr) = (format!("{e:?}"), format!("{r:?}"));
    assert!(
        te.starts_with("MixedEnergy {")
            && te.contains("held: 0.0")
            && tr.starts_with("MixedMassRatio {")
            && tr.contains("bodies: [0, 1]"),
        "the debug forms {te} and {tr}"
    );
}

#[test]
fn physical_axis_warps_values() {
    check_physical_values(true);
}

negative_control!(
    physical_axis_warps_values,
    "charts that differ in one parameter read as equal must fail",
    expected = "equality is not its parameters'",
    check_physical_values(false)
);

/// A lone physical axis's check over `[−¼, 1¼]²` (R-26; R-407, R-508): the vertical axis clamped past each bounded
/// side, as its declared types say, the horizontal `α` clamped to its domain; a point that moved is `Clamp`, naming
/// `α`'s domain where `α` moved and `vertical` otherwise, onto a system; a warped chart refuses nothing in the window, a
/// linear one only points whose construction fails. A non-finite coordinate is refused, naming its axis.
fn check_past<Q: EnergyAxis>(chart: &MixedEnergy<Alpha, Q, Defaults, f64>, vertical: Constraint) {
    let name = format!("α × {:?} {:?}", Q::QUANTITY, chart.mapping);
    let p = LinkParams::DEFAULT;
    let feasible = |u, v| chart.decode_from(chart.frozen, u, v, &p).1 != Outcome::Infeasible;
    let sides = chart.extension();
    let bounded = |t| t == ExtensionType::Bounded;
    let warped = chart.mapping == Mapping::Warped;
    let (mut across, mut up) = (0, 0);
    for [u, v] in grid(-0.25, 1.25) {
        let mut cv = v;
        if bounded(sides.below) {
            cv = cv.max(0.0);
        }
        if bounded(sides.above) {
            cv = cv.min(1.0);
        }
        match chart.validate(u, v) {
            ValidationResult::Valid => assert!(
                cv == v && feasible(u, v),
                "{name}: ({u}, {v}) is valid past a bounded side, or not a system"
            ),
            ValidationResult::Clamp {
                u: cu,
                v: got,
                moved,
                failed,
            } => {
                assert!(
                    got == cv
                        && moved == [cu != u, cv != v]
                        && (cu, cv) != (u, v)
                        && feasible(cu, cv)
                        && failed
                            == if cu != u {
                                Constraint::AxisDomain
                            } else {
                                vertical
                            },
                    "{name}: ({u}, {v}) is clamped to ({cu}, {got}), {moved:?}, naming {failed:?}"
                );
                across += u32::from(cu != u);
                up += u32::from(cv != v);
            }
            ValidationResult::Reject { failed } => assert!(
                !warped && failed == Constraint::KAboveLzBound,
                "{name}: ({u}, {v}) is refused, naming {failed:?}"
            ),
            other => panic!("{name}: ({u}, {v}) gives {other:?}"),
        }
    }
    let clamps_up = bounded(sides.below) || bounded(sides.above);
    assert!(
        across > 0 && (up > 0) == clamps_up,
        "{name}: {across} clamped across, {up} up"
    );
    for (u, v, failed) in [
        (f64::NAN, 0.5, Constraint::AxisDomain),
        (f64::NEG_INFINITY, f64::NAN, Constraint::AxisDomain),
        (0.5, f64::NAN, vertical),
        (0.5, f64::INFINITY, vertical),
    ] {
        assert_eq!(
            chart.validate(u, v),
            ValidationResult::Reject { failed },
            "{name}: ({u}, {v})"
        );
    }
}

/// [`check_past`] on each lone axis under each mapping, `lz` the constraint read for the `L_z` axis.
fn check_lone_past(lz: Constraint) {
    let k = Constraint::KNonNegative;
    for mapping in [Mapping::Warped, Mapping::Linear] {
        check_past(&lone::<AxisK>(0.0, mapping), k);
        check_past(&lone::<AxisE>(0.0, mapping), k);
        check_past(&lone::<AxisVirial>(0.0, mapping), k);
        check_past(&lone::<AxisLz>(k_max(), mapping), lz);
    }
    let mass: MixedEnergy<ZMu1, AxisK, Defaults, f64> =
        MixedEnergy::new(Z0, [-2.0, 2.0], k_max(), gamma_k(), 0.0, Mapping::Warped);
    let alpha = lone::<AxisK>(0.0, Mapping::Warped);
    assert!(
        mass.forbids_energy_normalisation()
            && alpha.forbids_energy_normalisation()
            && mass.requires_per_pixel_mass()
            && !alpha.requires_per_pixel_mass(),
        "the lone K charts' flags"
    );
}

#[test]
fn energy_axes_warped_past_edges() {
    check_lone_past(Constraint::KAboveLzBound);
}

negative_control!(
    energy_axes_warped_past_edges,
    "an L_z axis read as naming K ≥ 0 where it is clamped must fail",
    expected = "naming",
    check_lone_past(Constraint::KNonNegative)
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

/// R-408's count through the warp's types on `(L_z, E)` (R-508; R-506 W4), at each edge and just past it: `s` past
/// either edge and `t < 0` are the domain's end, `t > 1` is counted, a system or forbidden by `validate`, and the edges
/// themselves are inside; a periodic or pole-crossing side alone is not counted, and a bounded one beside it wins.
fn check_warped_count(sides: [Sides; 2]) {
    let past = 1e-9;
    let cases = [
        ([0.5, 0.0], true, Count::System),
        ([0.0, 1.0], true, Count::System),
        ([1.0, 0.5], false, Count::Forbidden),
        ([0.5, -past], true, Count::DomainEnd),
        ([-past, 0.5], true, Count::DomainEnd),
        ([1.0 + past, 0.5], true, Count::DomainEnd),
        ([0.5, 1.0 + past], true, Count::System),
        ([0.5, 1.25], false, Count::Forbidden),
    ];
    for (at, valid, want) in cases {
        assert_eq!(
            count(sides, at, valid),
            want,
            "the warp's count at {at:?}, valid {valid}"
        );
    }
    let periodic = Sides::both(ExtensionType::Periodic);
    let pole = Sides::both(ExtensionType::PoleCrossing);
    let affine = Sides::both(ExtensionType::Affine);
    for (types, want) in [
        ([periodic, affine], Count::NotCounted),
        ([affine, pole], Count::NotCounted),
        ([periodic, sides[0]], Count::DomainEnd),
    ] {
        assert_eq!(
            count(types, [1.5, 1.5], true),
            want,
            "the count past {types:?}"
        );
    }
}

#[test]
fn energy_axes_warped_count() {
    let le = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Warped));
    check_warped_count(le.extension());
}

negative_control!(
    energy_axes_warped_count,
    "the linear option's types count the pixels past s's edges, so the warp's count must fail on them",
    expected = "the warp's count at",
    {
        let le = Mapped::<_, Defaults>::new(InvariantLE(invariant()), mapped_all(Mapping::Linear));
        check_warped_count(le.extension());
    }
);

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

/// How far past a projected `s` the parabola's outside must begin: `10⁻⁹`, a hair, as `s`'s ulps near the edge are
/// finer than `L_z`'s.
const OFF_EDGE: f64 = 1e-9;

/// How a mapped invariant chart's check reads at each point: `validate`, the construction's verdict at a point, and
/// whether its `L_z` axis is warped, with the constraints its own chart names below `t = 0` and past the parabola.
struct Projection<'a> {
    name: &'a str,
    validate: &'a dyn Fn(f64, f64) -> ValidationResult,
    feasible: &'a dyn Fn(f64, f64) -> bool,
    warped_s: bool,
    low: Constraint,
    wide: Constraint,
}

/// Over `[−¼, 1¼]²`, the check of a mapped invariant chart with a linear axis (R-26): a valid point is feasible; any
/// other is projected to `t = max(t, 0)` and, past the parabola, a warped `L_z` clamped to `s`'s edge and a linear one
/// moved to a feasible `s` on its own side on the parabola, [`OFF_EDGE`] further out infeasible (at `t = 0`, `K = 0`,
/// that is `½`); the constraint named is `low` where `t` moved, else `wide`. A non-finite coordinate is refused,
/// naming its axis.
fn check_projection(c: &Projection) {
    let name = c.name;
    let (mut moved, mut valid) = (0, 0);
    for [u, v] in grid(-0.25, 1.25) {
        match (c.validate)(u, v) {
            ValidationResult::Valid => {
                valid += 1;
                assert!(
                    v >= 0.0 && (!c.warped_s || (0.0..=1.0).contains(&u)) && (c.feasible)(u, v),
                    "{name}: ({u}, {v}) is valid and not a system"
                );
            }
            ValidationResult::Project { u: s, v: t, failed } => {
                assert!(
                    t == v.max(0.0) && (s, t) != (u, v) && (c.feasible)(s, t),
                    "{name}: ({u}, {v}) is projected to ({s}, {t}), not a system on the edge"
                );
                assert_eq!(
                    failed,
                    if t != v { c.low } else { c.wide },
                    "{name}: the constraint ({u}, {v}) names"
                );
                if c.warped_s {
                    assert_eq!(s, u.clamp(0.0, 1.0), "{name}: s at ({u}, {v})");
                } else if s != u {
                    moved += 1;
                    let out = if u > 0.5 { s + OFF_EDGE } else { s - OFF_EDGE };
                    assert!(
                        (s - 0.5) * (u - 0.5) >= 0.0 && !(c.feasible)(out, t),
                        "{name}: ({u}, {v}) moves to {s}, not the nearest feasible s"
                    );
                }
            }
            other => panic!("{name}: ({u}, {v}) gives {other:?}"),
        }
    }
    assert!(
        valid > 0 && (c.warped_s || moved > 0),
        "{name}: {valid} valid, {moved} moved"
    );
    for (u, v, failed) in [
        (f64::NAN, 0.5, c.wide),
        (f64::INFINITY, 0.5, c.wide),
        (0.5, f64::NAN, c.low),
        (0.5, f64::NEG_INFINITY, c.low),
    ] {
        assert_eq!(
            (c.validate)(u, v),
            ValidationResult::Reject { failed },
            "{name}: ({u}, {v})"
        );
    }
}

/// [`check_projection`] on `(L_z, K)` and `(L_z, E)` under each mapping with a linear axis, `check` wrapping each
/// chart's `validate`.
fn check_projections(check: fn(ValidationResult) -> ValidationResult) {
    let inv = invariant();
    let z = inv.frozen();
    let p = LinkParams::DEFAULT;
    let linear_s = [Mapping::Linear, Mapping::Warped];
    let linear_t = [Mapping::Warped, Mapping::Linear];
    for mapping in [mapped_all(Mapping::Linear), linear_s, linear_t] {
        let warped_s = mapping[0] == Mapping::Warped;
        let lk = Mapped::<_, Defaults>::new(InvariantLK(inv), mapping);
        check_projection(&Projection {
            name: &format!("(L_z, K) {mapping:?}"),
            validate: &|u, v| check(lk.validate(u, v)),
            feasible: &|s, t| lk.decode_from(s, t, &z, &p).1 != Outcome::Infeasible,
            warped_s,
            low: Constraint::KNonNegative,
            wide: Constraint::KAboveLzBound,
        });
        let le = Mapped::<_, Defaults>::new(InvariantLE(inv), mapping);
        check_projection(&Projection {
            name: &format!("(L_z, E) {mapping:?}"),
            validate: &|u, v| check(le.validate(u, v)),
            feasible: &|s, t| le.decode_from(s, t, &z, &p).1 != Outcome::Infeasible,
            warped_s,
            low: Constraint::LzEParabola,
            wide: Constraint::LzEParabola,
        });
    }
}

/// The check as the chart gives it.
fn as_given(r: ValidationResult) -> ValidationResult {
    r
}

#[test]
fn energy_axis_linear_option_projection() {
    check_projections(as_given);
}

negative_control!(
    energy_axis_linear_option_projection,
    "a check that projects a refused linear L_z 10⁻⁶ inside the parabola lands off it, so it must fail",
    expected = "not the nearest feasible s",
    check_projections(|r| match r {
        ValidationResult::Project { u: s, v, failed } if s != 0.5 => ValidationResult::Project {
            u: if s > 0.5 { s - 1e-6 } else { s + 1e-6 },
            v,
            failed,
        },
        other => other,
    })
);

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

/// The hidden coordinates a tilt may target on the physical-quantity mixed-axis charts (R-473 J1): every control but
/// the other axis's and the physical axis's block, the momenta for an energy axis, the masses for the mass ratio; a
/// slice sets one only where it is hidden. `momenta` and `masses` are the blocks the check reads as written.
fn check_hidden(momenta: [u32; 4], masses: [u32; 2]) {
    let e = lone::<AxisK>(0.0, Mapping::Warped);
    let r = ratio_chart([0.1, 10.0], Mapping::Warped);
    let alpha = Alpha::COORDINATE as u32;
    let z_alpha = ZAlpha::COORDINATE as u32;
    for k in 0..9 {
        let want_e = k < 8 && k != alpha && !momenta.contains(&k);
        let want_r = k < 8 && k != z_alpha && !masses.contains(&k);
        assert!(
            e.hidden(k) == want_e && r.hidden(k) == want_r,
            "control {k} is hidden {} on the lone K chart and {} on the ratio chart",
            e.hidden(k),
            r.hidden(k)
        );
        assert_eq!(
            e.sliced(k, 0.25).map(|c| c.frozen[k as usize]),
            want_e.then_some(0.25),
            "the lone K chart sliced at control {k}"
        );
        assert_eq!(
            r.sliced(k, 0.25).map(|c| c.frozen[k as usize]),
            want_r.then_some(0.25),
            "the ratio chart sliced at control {k}"
        );
    }
}

#[test]
fn energy_warp_per_pixel_hidden() {
    check_hidden([2, 3, 4, 5], [6, 7]);
}

negative_control!(
    energy_warp_per_pixel_hidden,
    "a check that reads the masses as written by the lone K chart must fail",
    expected = "is hidden",
    check_hidden([2, 3, 4, 6], [6, 7])
);

/// A mapped `(L_z, K)` chart's check at a tilt's `(a, b)`, a mass logit shifted (R-473 J1, E3): `want` at each point,
/// under each mapping with one axis linear. A warped axis is refused past its bounded side, naming its constraint;
/// past a linear one the construction's exact test decides; a non-finite coordinate names the axis it is on.
fn check_shifted(want: &[([Mapping; 2], [f64; 2], Option<Constraint>)]) {
    let lk = |m| Mapped::<_, Defaults>::new(InvariantLK(invariant()), m);
    let shift = Shift {
        x: 6,
        dx: 0.1,
        y: 0,
        dy: 0.0,
        two: false,
    };
    for &(mapping, [a, b], failed) in want {
        let got = lk(mapping).validate_shifted(a, b, &shift, &LinkParams::DEFAULT);
        let want = failed.map_or(ValidationResult::Valid, |failed| ValidationResult::Reject {
            failed,
        });
        assert_eq!(got, want, "{mapping:?} at ({a}, {b})");
    }
}

/// The shifted check's cases: `(mapping, (a, b), the constraint refused or none)`.
const SHIFTED: [([Mapping; 2], [f64; 2], Option<Constraint>); 13] = {
    let (lin, war) = (Mapping::Linear, Mapping::Warped);
    let (low, wide) = (
        Some(Constraint::KNonNegative),
        Some(Constraint::KAboveLzBound),
    );
    [
        ([lin, war], [0.5, -0.1], low),
        ([lin, war], [0.5, 0.0], None),
        ([lin, war], [0.5, 0.5], None),
        ([lin, war], [1.2, 1.5], None),
        ([lin, war], [1.2, 0.5], wide),
        ([lin, war], [f64::NAN, 0.5], wide),
        ([lin, war], [0.5, f64::NAN], low),
        ([war, lin], [0.5, -0.1], wide),
        ([war, lin], [0.5, 0.5], None),
        ([war, lin], [1.0, 0.5], None),
        ([war, lin], [1.2, 0.5], wide),
        ([war, lin], [-0.2, 0.5], wide),
        ([war, lin], [f64::INFINITY, f64::NAN], low),
    ]
};

#[test]
fn energy_warp_per_pixel_shifted_check() {
    check_shifted(&SHIFTED);
}

negative_control!(
    energy_warp_per_pixel_shifted_check,
    "a linear K below t = 0 read as the warp's bounded side, refused by K ≥ 0, must fail",
    expected = "at (0.5, -0.1)",
    check_shifted(&[(
        [Mapping::Warped, Mapping::Linear],
        [0.5, -0.1],
        Some(Constraint::KNonNegative)
    )])
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

/// Tilted, the default mapping is still the chart's own, bit for bit: `chart` and `mapped` under one tilt into a mass
/// logit and one into a shape control decode and validate alike over `[−¼, 1¼]²`.
fn check_tilted_own<C, W>(chart: C, mapped: W)
where
    C: Tiltable<Param = f64> + ChartDecode + Copy,
    W: Tiltable<Param = f64> + ChartDecode + Copy,
{
    for (x, tau) in [(6, 30.0), (1, 45.0)] {
        let tilt = || Tilt::new(&spec(tau, 20.0, TiltMap::Lean, x), CENTRE, SPAN);
        let a = Tilted::new(chart, tilt()).expect("a hidden target");
        let b = Tilted::new(mapped, tilt()).expect("a hidden target");
        for [u, v] in grid(-0.25, 1.25) {
            assert!(
                bits(&a.decode::<Defaults, f64>(u, v)) == bits(&b.decode::<Defaults, f64>(u, v))
                    && a.validate(u, v) == b.validate(u, v),
                "{} tilted into {x} at ({u}, {v}) is not bitwise its own chart under the mapping",
                chart.name()
            );
        }
    }
}

#[test]
fn energy_warp_per_pixel_tilted_own() {
    let inv = invariant();
    let warped = mapped_all(Mapping::Warped);
    check_tilted_own(
        InvariantLE(inv),
        Mapped::<_, Defaults>::new(InvariantLE(inv), warped),
    );
    check_tilted_own(
        InvariantLK(inv),
        Mapped::<_, Defaults>::new(InvariantLK(inv), warped),
    );
}

negative_control!(
    energy_warp_per_pixel_tilted_own,
    "the linear option tilted is not the warped chart tilted, so it must fail",
    expected = "is not bitwise its own chart under the mapping",
    check_tilted_own(
        InvariantLK(invariant()),
        Mapped::<_, Defaults>::new(InvariantLK(invariant()), [Mapping::Warped, Mapping::Linear])
    )
);

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

/// The virial-ratio axis beside `α` on the asked range `asked`, its held `L_z = 0`, under `mapping`.
fn virial(
    asked: [f64; 2],
    mapping: Mapping,
) -> Option<MixedEnergy<Alpha, AxisVirial, Defaults, f64>> {
    MixedEnergy::virial(Z0, [0.2, 1.3], asked, gamma_k(), 0.0, mapping)
}

/// The virial ratio over its own asked range `[Q_lo, Q_hi]` (R-471 H4; chart_reference §2.1): warped, every pixel of
/// `[0,1]²` validates and decodes `2K/|U| = Q₀ + (Q_hi − Q₀)·t^γ_K`, `Q₀ = max(Q_lo, 0)`, a range reaching below 0
/// decoding as its feasible part, bit for bit; linear, `Q_lo + (Q_hi − Q_lo)·t`, its pixels with `Q < 0` refused and
/// `Q = 0` a system; a range not increasing and finite, or warped with `Q_hi ≤ 0`, builds no chart. `want` gives the
/// ratio the test expects at `(asked, mapping, t)`: the section's, or the control's.
fn check_virial_range(want: fn([f64; 2], Mapping, f64) -> f64) {
    for (asked, mapping) in [
        ([0.2, 1.5], Mapping::Warped),
        ([-0.5, 1.5], Mapping::Warped),
        ([0.2, 1.5], Mapping::Linear),
        ([-0.5, 1.5], Mapping::Linear),
    ] {
        let chart = virial(asked, mapping).expect("a virial range");
        assert_eq!(
            chart.extension(),
            sides_of(mapping),
            "the virial axis's extension type"
        );
        for [u, v] in grid(0.0, 1.0) {
            let q = want(asked, mapping, v);
            let d = chart.decode::<Defaults, f64>(u, v);
            let valid = chart.validate(u, v);
            if q < 0.0 {
                assert_eq!(
                    valid,
                    ValidationResult::Reject {
                        failed: Constraint::KAboveLzBound
                    },
                    "the virial pixel ({u}, {v}) at Q = {q} < 0 is a system"
                );
                continue;
            }
            let (_, k) = lz_k(&d);
            let ratio = 2.0 * k / d.descriptor.V_0.abs();
            assert!(
                valid == ValidationResult::Valid
                    && d.degenerate.is_none()
                    && (ratio - q).abs() <= 1e-12 * (1.0 + q),
                "the virial axis on {asked:?} {mapping:?} at ({u}, {v}) decodes 2K/|U| = {ratio}, not {q}"
            );
        }
    }
    // A range reaching below 0, warped, is its feasible part's: [−0.5, 1.5] is [0, 1.5], MixedEnergy::new's.
    let below = virial([-0.5, 1.5], Mapping::Warped).expect("a virial range");
    let from_zero = lone_virial(1.5);
    for [u, v] in grid(-0.25, 1.25) {
        assert_eq!(
            bits(&below.decode::<Defaults, f64>(u, v)),
            bits(&from_zero.decode::<Defaults, f64>(u, v)),
            "[−0.5, 1.5] warped is not [0, 1.5] at ({u}, {v})"
        );
    }
    let linear = virial([-0.5, 1.5], Mapping::Linear).expect("a virial range");
    assert_eq!(
        linear.validate(0.5, 0.25),
        ValidationResult::Valid,
        "Q = 0 is not a system"
    );
    for (asked, mapping, builds) in [
        ([1.0, 1.0], Mapping::Linear, false),
        ([1.0, 0.5], Mapping::Linear, false),
        ([f64::NAN, 1.0], Mapping::Linear, false),
        ([0.0, f64::INFINITY], Mapping::Linear, false),
        ([f64::NEG_INFINITY, 1.0], Mapping::Warped, false),
        ([-1.0, 0.0], Mapping::Warped, false),
        ([-1.0, 0.0], Mapping::Linear, true),
        ([-1.0, f64::MIN_POSITIVE], Mapping::Warped, true),
    ] {
        assert_eq!(
            virial(asked, mapping).is_some(),
            builds,
            "the virial range {asked:?} {mapping:?}"
        );
    }
}

/// The extension type of a virial axis under `mapping` (chart_reference §2.1).
fn sides_of(mapping: Mapping) -> Sides {
    match mapping {
        Mapping::Warped => Sides::both(ExtensionType::Bounded),
        Mapping::Linear => Sides::both(ExtensionType::Affine),
    }
}

/// A virial axis beside `α` from [`MixedEnergy::new`], its range `[0, top]`.
fn lone_virial(top: f64) -> MixedEnergy<Alpha, AxisVirial, Defaults, f64> {
    MixedEnergy::new(Z0, [0.2, 1.3], top, gamma_k(), 0.0, Mapping::Warped)
}

/// The ratio §2.1 writes at `t` on the asked range `[lo, hi]`.
fn virial_want([lo, hi]: [f64; 2], mapping: Mapping, t: f64) -> f64 {
    match mapping {
        Mapping::Warped => {
            let q0 = lo.max(0.0);
            q0 + (hi - q0) * t.powf(gamma_k())
        }
        Mapping::Linear => lo + (hi - lo) * t,
    }
}

#[test]
fn physical_axis_warps_virial_range() {
    check_virial_range(virial_want);
}

negative_control!(
    physical_axis_warps_virial_range,
    "the range fixed at [0, Q_hi], its bottom dropped, is not the asked range's axis, so it must fail",
    expected = "decodes 2K/|U|",
    check_virial_range(|[_, hi], mapping, t| virial_want([0.0, hi], mapping, t))
);

// ── A warped `t` axis at a held `L_z ≠ 0`: the floor (the owner's ruling of 4 Nov 2026 on RQ-683) ───────────────────

/// The held values of the floor's tests, either sign, a tiny one and one past `K_max`'s own `L_max`.
const HELD_LZ: [f64; 6] = [0.3, -0.3, 1.0e-300, 0.1, 1.7, 40.0];

/// `n` values of `|L_z|` across `10^−12` to `10^4`, from a fixed generator, for the floor's rounding: `l²/2` lands on
/// both sides of its float.
fn held_values(n: u32) -> Vec<f64> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unit = (state >> 11) as f64 / (1u64 << 53) as f64;
            10f64.powf(-12.0 + 16.0 * unit)
        })
        .collect()
}

/// `floor` is the least float the exact test finds feasible at each `L_z`, whichever way `l²/2` rounded, and some of
/// them rounded below.
fn check_floor_least(floor: fn(f64) -> f64) {
    let mut raised = 0;
    for l in held_values(4096) {
        let k = floor(l);
        assert!(
            !infeasible(1.0, k, l),
            "the floor at L_z = {l} rounds below L_z²/2"
        );
        assert!(
            infeasible(1.0, invariant::WarpReal::below(k), l),
            "the floor at L_z = {l} is not the least feasible float"
        );
        raised += usize::from(k != l * l / 2.0);
    }
    assert!(raised > 0, "no L_z rounded below: the sweep tests nothing");
}

#[test]
fn energy_axes_warped_floor_rounding() {
    check_floor_least(invariant::k_floor);
}

negative_control!(
    energy_axes_warped_floor_rounding,
    "the rounded L_z²/2 alone, not raised, falls below the exact bound on about half the sweep",
    expected = "rounds below L_z²/2",
    check_floor_least(|l| l * l / 2.0)
);

/// `floor`'s edges at f64: 0 at `±0`, the smallest float where `l²` underflows, either sign alike, and `+∞` and NaN
/// where `l²/2` has no float.
fn check_floor_edges(floor: fn(f64) -> f64) {
    assert_eq!(floor(0.0).to_bits(), 0, "the floor at +0");
    assert_eq!(floor(-0.0).to_bits(), 0, "the floor at −0");
    assert_eq!(
        floor(1.0e-300).to_bits(),
        1,
        "the floor where l² underflows"
    );
    assert_eq!(
        floor(-1.0e-300).to_bits(),
        1,
        "the floor where l² underflows"
    );
    assert_eq!(floor(1.0e200), f64::INFINITY, "the floor past f64");
    assert!(floor(f64::NAN).is_nan(), "the floor of NaN");
}

/// The floor's edges at both precisions, and the step up's own, which leaves `+∞` and NaN where they are.
#[test]
fn energy_axes_warped_floor_edges() {
    use invariant::{k_floor, WarpReal};
    check_floor_edges(k_floor);
    assert_eq!(k_floor(0.0_f32).to_bits(), 0);
    assert_eq!(k_floor(1.0e-30_f32).to_bits(), 1);
    assert_eq!(k_floor(1.0e30_f32), f32::INFINITY);
    assert!(k_floor(f32::NAN).is_nan());
    assert_eq!(0.0_f64.above().to_bits(), 1);
    assert_eq!(0.0_f32.above().to_bits(), 1);
    assert_eq!(1.0_f64.above(), 1.0 + f64::EPSILON);
    assert_eq!(1.0_f32.above(), 1.0 + f32::EPSILON);
    assert_eq!(f64::MAX.above(), f64::INFINITY);
    assert_eq!(f32::MAX.above(), f32::INFINITY);
    assert_eq!(f64::INFINITY.above(), f64::INFINITY);
    assert_eq!(f32::INFINITY.above(), f32::INFINITY);
    assert!(f64::NAN.above().is_nan() && f32::NAN.above().is_nan());
}

negative_control!(
    energy_axes_warped_floor_edges,
    "the rounded l²/2 alone leaves an underflowed spin's bottom row at K* = 0, below any spin",
    expected = "the floor where l² underflows",
    check_floor_edges(|l| l * l / 2.0)
);

/// Every pixel of a warped lone `Q` axis at every held `L_z` is a system at that `L_z`, under `mapping`: the bottom row
/// included, with the rounding at the floor (the sweep's values at `u = ½`, `t = 0`, on the whole grid for
/// [`HELD_LZ`]).
fn check_held_axis<Q: EnergyAxis>(mapping: Mapping) {
    for held in HELD_LZ {
        check_every_pixel(
            &format!("{:?} at L_z = {held}", Q::QUANTITY),
            system(&lone::<Q>(held, mapping)),
        );
    }
    for held in held_values(256) {
        let chart = lone::<Q>(held, mapping);
        assert!(
            chart.validate(0.5, 0.0) == ValidationResult::Valid,
            "the bottom row of {:?} at L_z = {held} is not a system",
            Q::QUANTITY
        );
    }
}

fn check_held_axes(mapping: Mapping) {
    check_held_axis::<AxisK>(mapping);
    check_held_axis::<AxisKE>(mapping);
    check_held_axis::<AxisE>(mapping);
    check_held_axis::<AxisVirial>(mapping);
}

#[test]
fn energy_axes_warped_held_lz_every_pixel() {
    check_held_axes(Mapping::Warped);
}

negative_control!(
    energy_axes_warped_held_lz_every_pixel,
    "the linear option at the same held L_z adds no floor, so its low pixels are not systems",
    expected = "is not a system",
    check_held_axes(Mapping::Linear)
);

/// Each warped lone axis decodes `K* = floor(held) + line` and the held `L_z`; `E` reads `U + K*` at the pixel's own `U`
/// and the virial ratio `Q(t)·|U|/2` above the floor; past the top row `K*` goes on and exceeds `K_max`. `floor` is the
/// floor the test expects.
fn check_held_targets(floor: fn(f64) -> f64) {
    fn axis<Q: EnergyAxis>(held: f64, floor: f64, line: &dyn Fn(f64, f64) -> f64) {
        let chart = lone::<Q>(held, Mapping::Warped);
        for [u, t] in grid(0.0, 1.0) {
            let d = chart.decode::<Defaults, f64>(u, t);
            let (lz, k) = lz_k(&d);
            let u_pot = kernel::decode::descriptor::potential(&d.ic);
            let want = floor + line(t, u_pot);
            let eps = f64::EPSILON;
            assert!(
                (k - want).abs() <= TOL_K * eps * want.max(f64::MIN_POSITIVE)
                    && (lz - held).abs() <= TOL_L * eps * (held.abs() + (2.0 * want).sqrt()),
                "{:?} at L_z = {held}, ({u}, {t}): K = {k}, L_z = {lz}, not K* = {want}",
                Q::QUANTITY
            );
        }
    }
    let (top, g) = (k_max(), gamma_k());
    for held in [0.3, -0.3, 1.7] {
        let f = floor(held);
        let kinetic = move |t: f64, _: f64| top * t.powf(g);
        axis::<AxisK>(held, f, &kinetic);
        axis::<AxisKE>(held, f, &kinetic);
        axis::<AxisE>(held, f, &kinetic);
        axis::<AxisVirial>(held, f, &move |t, u| top * t.powf(g) * u.abs() / 2.0);
    }
}

#[test]
fn energy_axes_warped_held_lz_targets() {
    check_held_targets(|l| l * l / 2.0);
}

negative_control!(
    energy_axes_warped_held_lz_targets,
    "the unfloored line K_max·t^γ is the warp of a held L_z = 0 only",
    expected = "not K* =",
    check_held_targets(|_| 0.0)
);

/// The bottom row under `mapping`, at the precision `$r`, across the sweep's held values: the construction feasible on
/// every axis's pixel, the chart built at that precision.
macro_rules! check_bottom_row {
    ($r:ty, $axis:ty, $mapping:expr) => {
        for l in held_values(1024) {
            let frozen = Z0.map(|z| z as $r);
            let chart: MixedEnergy<Alpha, $axis, Defaults, $r> = MixedEnergy::new(
                frozen,
                [0.2, 1.3],
                k_max() as $r,
                gamma_k() as $r,
                l as $r,
                $mapping,
            );
            let (_, outcome) = chart.decode_from(frozen, 0.5, 0.0, &LinkParams::DEFAULT);
            assert!(
                outcome == Outcome::Feasible,
                "the bottom row at L_z = {l} is {outcome:?}"
            );
        }
    };
}

/// The bottom row, `t = 0`, is a system at both precisions on every warped axis.
fn check_bottom_rows(mapping: Mapping) {
    check_bottom_row!(f64, AxisK, mapping);
    check_bottom_row!(f64, AxisKE, mapping);
    check_bottom_row!(f64, AxisE, mapping);
    check_bottom_row!(f64, AxisVirial, mapping);
    check_bottom_row!(f32, AxisK, mapping);
    check_bottom_row!(f32, AxisKE, mapping);
    check_bottom_row!(f32, AxisE, mapping);
    check_bottom_row!(f32, AxisVirial, mapping);
}

#[test]
fn energy_axes_warped_bottom_row() {
    check_bottom_rows(Mapping::Warped);
}

negative_control!(
    energy_axes_warped_bottom_row,
    "the linear option's row t = 0 is K* = 0, below any spin, so it must fail",
    expected = "the bottom row at L_z",
    check_bottom_rows(Mapping::Linear)
);

/// Past the square: below `t = 0` a warped `t` axis clamps to the floor, past `t = 1` it continues on `K`, `KE` and `E`
/// (R-508), and the virial ratio is bounded past both edges.
fn check_held_sides(held: f64, want: fn(f64) -> f64) {
    let moved = |v: f64| ValidationResult::Clamp {
        u: 0.5,
        v,
        moved: [false, true],
        failed: Constraint::KNonNegative,
    };
    let k = lone::<AxisK>(held, Mapping::Warped);
    assert_eq!(k.validate(0.5, -0.25), moved(0.0), "t < 0, K");
    assert_eq!(k.validate(0.5, 1.25), ValidationResult::Valid, "t > 1, K");
    let q = lone::<AxisVirial>(held, Mapping::Warped);
    assert_eq!(q.validate(0.5, -0.25), moved(0.0), "t < 0, virial");
    assert_eq!(q.validate(0.5, 1.25), moved(1.0), "t > 1, virial");
    let floor = lz_k(&k.decode::<Defaults, f64>(0.5, 0.0)).1;
    assert!(
        (floor - want(held)).abs() <= TOL_K * f64::EPSILON * floor,
        "the clamp's t = 0 reads K = {floor}, not the floor"
    );
}

#[test]
fn energy_axes_warped_held_lz_sides() {
    check_held_sides(0.3, |l| l * l / 2.0);
    check_held_sides(-1.7, |l| l * l / 2.0);
}

negative_control!(
    energy_axes_warped_held_lz_sides,
    "a clamp read at K = 0 is the unfloored warp's, so it must fail",
    expected = "not the floor",
    check_held_sides(0.3, |_| 0.0)
);

/// A warped axis at a held `L_z ≠ 0` stays a system under a tilt into a mass logit and `z_β`: the pixel's own
/// `U` and masses move, the floor, at `I = 1`, does not.
fn check_held_tilted(mapping: Mapping) {
    let chart = lone::<AxisE>(0.3, mapping);
    let shift = Shift {
        x: 6,
        dx: 0.4,
        y: 1,
        dy: 0.3,
        two: true,
    };
    for [u, t] in grid(0.0, 1.0) {
        let valid = chart.validate_shifted(u, t, &shift, &LinkParams::DEFAULT);
        assert!(
            valid == ValidationResult::Valid,
            "({u}, {t}) tilted is not a system: {valid:?}"
        );
    }
}

#[test]
fn energy_axes_warped_held_lz_tilted() {
    check_held_tilted(Mapping::Warped);
}

negative_control!(
    energy_axes_warped_held_lz_tilted,
    "the linear option under the same tilt keeps its fixed values, so its low pixels are not systems",
    expected = "tilted is not a system",
    check_held_tilted(Mapping::Linear)
);

/// A held value with no floor, or none at all, is refused at every point: never a system, never a panic.
fn check_unrepresentable(held: f64) {
    for [u, v] in grid(0.0, 1.0) {
        assert_eq!(
            lone::<AxisK>(held, Mapping::Warped).validate(u, v),
            ValidationResult::Reject {
                failed: Constraint::KAboveLzBound
            },
            "the warped axis at L_z = {held}: ({u}, {v}) is not refused"
        );
    }
}

#[test]
fn energy_axes_warped_held_lz_unrepresentable() {
    check_unrepresentable(f64::NAN);
    check_unrepresentable(f64::INFINITY);
    check_unrepresentable(1.0e200);
    check_unrepresentable(-1.0e200);
}

negative_control!(
    energy_axes_warped_held_lz_unrepresentable,
    "a finite spin with a representable floor is a system at every pixel",
    expected = "is not refused",
    check_unrepresentable(0.3)
);

/// The linear option at a held `L_z ≠ 0` keeps its fixed values (the owner's ruling, E2): `K* = K_max·t`, no floor, the
/// row `t = 0` not a system (blank, R-470) and the top row one, on `K`, `KE`, `E` and, at a held `L_z` its top row
/// clears, the virial ratio; `mapping` is the chart's. A warped `L_z` axis is as it was: `K` the held value, no floor added.
fn check_linear_held(mapping: Mapping) {
    fn axis<Q: EnergyAxis>(held: f64, mapping: Mapping) {
        let chart = lone::<Q>(held, mapping);
        assert_eq!(
            chart.validate(0.5, 0.0),
            ValidationResult::Reject {
                failed: Constraint::KAboveLzBound
            },
            "{:?} linear: the row t = 0 is a system",
            Q::QUANTITY
        );
        assert_eq!(
            chart.validate(0.5, 1.0),
            ValidationResult::Valid,
            "{:?} linear: the top row is not a system",
            Q::QUANTITY
        );
    }
    axis::<AxisK>(0.3, mapping);
    axis::<AxisKE>(0.3, mapping);
    axis::<AxisE>(0.3, mapping);
    axis::<AxisVirial>(0.05, mapping);
    let lz = lone::<AxisLz>(0.3, Mapping::Warped);
    let (_, k) = lz_k(&lz.decode::<Defaults, f64>(0.5, 0.75));
    assert!(
        (k - 0.3).abs() <= TOL_K * f64::EPSILON * 0.3,
        "a warped L_z axis's K = {k} is not the held 0.3"
    );
}

#[test]
fn energy_axis_linear_option_held_lz_blanks() {
    check_linear_held(Mapping::Linear);
}

negative_control!(
    energy_axis_linear_option_held_lz_blanks,
    "the warped axis at the same held L_z has the floor, so its row t = 0 is a system",
    expected = "the row t = 0 is a system",
    check_linear_held(Mapping::Warped)
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
/// decodes masses positive, summing to 1, with the ratio `q(t)` and the third body's mass at the held `m_c`.
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

/// The third body of the pair `(a, b)`, whose mass a mass-ratio axis holds.
fn third([a, b]: [u32; 2]) -> usize {
    (3 - a - b) as usize
}

/// One arm of [`check_mass_ratio`] under the selection `S` at `p`, the decoded `m_c` at the held value too.
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
        let m_c = m[third(pair)];
        assert!(
            chart.validate_params(u, v, p) == ValidationResult::Valid
                && m.iter().all(|&x| x > 0.0)
                && (m.iter().sum::<f64>() - 1.0).abs() <= 4.0 * f64::EPSILON
                && (ratio - q).abs() <= RATIO_TOL * q
                && (m_c - chart.held).abs() <= RATIO_TOL * chart.held,
            "the mass ratio {pair:?} under {link} at μ_max = {} at ({u}, {v}) decodes {ratio} and m_c = {m_c}, not {q} \
             and the held {}",
            p.mu_max,
            chart.held
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

/// The mass ratio's formula, read off the chart's own range `[q₀, q₁]`: warped `q₀·(q₁/q₀)^t`, geometric, its
/// midpoint `√(q₀q₁)`; linear `q₀ + (q₁ − q₀)·t`, its midpoint the mean. `warped` names which the check reads.
fn check_ratio_formula(chart: &MixedMassRatio<ZAlpha, Defaults, f64>, warped: bool) {
    let [q0, q1] = chart.ratio;
    let mid = if warped {
        (q0 * q1).sqrt()
    } else {
        0.5 * (q0 + q1)
    };
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b;
    assert!(
        close(chart.ratio_at(0.0), q0)
            && close(chart.ratio_at(1.0), q1)
            && close(chart.ratio_at(0.5), mid),
        "the ratio's formula on [{q0}, {q1}]: {}, {}, {}",
        chart.ratio_at(0.0),
        chart.ratio_at(0.5),
        chart.ratio_at(1.0)
    );
}

#[test]
fn physical_axis_warps_ratio_formula() {
    check_ratio_formula(&ratio_chart([0.25, 4.0], Mapping::Warped), true);
    check_ratio_formula(&ratio_chart([0.25, 4.0], Mapping::Linear), false);
}

negative_control!(
    physical_axis_warps_ratio_formula,
    "the warped ratio read as linear must fail",
    expected = "the ratio's formula",
    check_ratio_formula(&ratio_chart([0.25, 4.0], Mapping::Warped), false)
);

/// Warped, a mass-ratio chart asked for more than the link reaches at `μ_max` keeps the part it realises: both ends
/// realised, and `growth` past either one not; asked for a range wholly past the reach, there is no chart.
fn check_reach(growth: f64) {
    let range = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's range");
    let p = LinkParams {
        mu_max: range.lo,
        ..LinkParams::DEFAULT
    };
    let at = |asked| {
        MixedMassRatio::<ZAlpha, Defaults, f64>::new(
            Z0,
            [-2.0, 2.0],
            asked,
            [0, 1],
            Mapping::Warped,
            &p,
        )
    };
    let chart = at([1e-3, 1e3]).expect("a realised part");
    let [q0, q1] = chart.ratio;
    assert!(
        1e-3 < q0 && q1 < 1e3 && chart.realised(q0, &p) && chart.realised(q1, &p),
        "the reach [{q0}, {q1}] at μ_max = {} is not realised",
        p.mu_max
    );
    assert!(
        !chart.realised(q0 / growth, &p) && !chart.realised(q1 * growth, &p),
        "the reach [{q0}, {q1}] at μ_max = {} stops short of the link's",
        p.mu_max
    );
    assert!(
        at([1e4, 1e5]).is_none() && at([1e-5, 1e-4]).is_none(),
        "a range past the reach gives a chart"
    );
}

#[test]
fn physical_axis_warps_ratio_reach() {
    check_reach(1.0 + 10.0 * <f64 as ChartConstants>::EPS_MU);
}

negative_control!(
    physical_axis_warps_ratio_reach,
    "a ratio 10⁻⁹ past the reach, inside ε_μ, is realised, so it must fail",
    expected = "stops short",
    check_reach(1.0 + 1e-9)
);

/// A mass-ratio chart's check over `[−¼, 1¼]²` (R-26; R-506 W3, W4): warped, `t` clamped to `[0, 1]`; linear, a ratio
/// the link does not realise, or at or below 0, refused; `α` clamped to its domain; a point that moved is `Clamp` onto a
/// realised ratio, naming the axis domain. A non-finite coordinate is refused.
fn check_ratio_past(mapping: Mapping, clamps_up: bool) {
    let p = LinkParams::DEFAULT;
    let chart = MixedMassRatio::<Alpha, Defaults, f64>::new(
        Z0,
        [0.2, 1.3],
        [0.05, 20.0],
        [0, 1],
        mapping,
        &p,
    )
    .expect("a chart");
    let (mut across, mut up) = (0, 0);
    for [u, v] in grid(-0.25, 1.25) {
        let cv = if clamps_up { v.clamp(0.0, 1.0) } else { v };
        match chart.validate(u, v) {
            ValidationResult::Valid => assert!(
                cv == v && chart.realised(chart.ratio_at(v), &p),
                "{mapping:?}: ({u}, {v}) is valid past the ratio's edge, or unrealised"
            ),
            ValidationResult::Clamp {
                u: cu,
                v: got,
                moved,
                failed,
            } => {
                assert!(
                    got == cv
                        && moved == [cu != u, cv != v]
                        && (cu, cv) != (u, v)
                        && failed == Constraint::AxisDomain
                        && chart.realised(chart.ratio_at(cv), &p),
                    "{mapping:?}: ({u}, {v}) is clamped to ({cu}, {got}), {moved:?}"
                );
                across += u32::from(cu != u);
                up += u32::from(cv != v);
            }
            ValidationResult::Reject { failed } => assert!(
                !clamps_up
                    && failed == Constraint::AxisDomain
                    && !chart.realised(chart.ratio_at(v), &p),
                "{mapping:?}: ({u}, {v}) is refused"
            ),
            other => panic!("{mapping:?}: ({u}, {v}) gives {other:?}"),
        }
    }
    assert!(
        across > 0 && (up > 0) == clamps_up,
        "{mapping:?}: {across} across, {up} up"
    );
    for [u, v] in [[f64::NAN, 0.5], [0.5, f64::NAN], [0.5, f64::INFINITY]] {
        assert_eq!(
            chart.validate(u, v),
            ValidationResult::Reject {
                failed: Constraint::AxisDomain
            },
            "{mapping:?}: ({u}, {v})"
        );
    }
    assert!(
        chart.requires_per_pixel_mass(),
        "the ratio chart's masses are per pixel"
    );
}

#[test]
fn physical_axis_warps_ratio_past_edges() {
    check_ratio_past(Mapping::Warped, true);
    check_ratio_past(Mapping::Linear, false);
}

negative_control!(
    physical_axis_warps_ratio_past_edges,
    "the linear ratio read as clamped past its edges must fail",
    expected = "Linear",
    check_ratio_past(Mapping::Linear, true)
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

// ── Fix round 2: a ratio is realised only at the held m_c (chart_reference §2.1) ─────────────────────────────────────────

/// A slice whose mass logit for `m_c` saturates before the ratio's does: `mass` the frozen mass controls, `pair` the
/// bodies, `mu_max` the link's parameter, `asked` the asked range.
struct Saturating {
    mass: [f64; 2],
    pair: [u32; 2],
    mu_max: f64,
    asked: [f64; 2],
    /// A ratio past the `m_c` logit's clamp, which the link's round trip keeps.
    past: f64,
}

/// Codex's input (`μ_max = 1`, mass controls `(0, 3)`, bodies `(0, 1)`), the same at the default `μ_max = 5`, where
/// the `m_c` logit clamps at the range's top, and mass controls `(0, −3)` at `μ_max = 5`, where it clamps at its bottom
/// with `m_c` small.
const SATURATING: [Saturating; 3] = [
    Saturating {
        mass: [0.0, 3.0],
        pair: [0, 1],
        mu_max: 1.0,
        asked: [0.5, 4.0],
        past: 2.0,
    },
    Saturating {
        mass: [0.0, 3.0],
        pair: [0, 1],
        mu_max: 5.0,
        asked: [0.5, 20.0],
        past: 2.0,
    },
    Saturating {
        mass: [0.0, -3.0],
        pair: [0, 1],
        mu_max: 5.0,
        asked: [0.05, 2.0],
        past: 0.875,
    },
];

/// The slice centre `Z0` with the mass controls `mass`.
fn with_mass(mass: [f64; 2]) -> [f64; 8] {
    let mut z = Z0;
    z[6] = mass[0];
    z[7] = mass[1];
    z
}

/// The mass-ratio chart under `S` at `p` at the slice centre `Z0` with the mass controls `mass`.
fn saturating_chart<S: Selection>(
    mass: [f64; 2],
    pair: [u32; 2],
    asked: [f64; 2],
    mapping: Mapping,
    p: &LinkParams<f64>,
) -> Option<MixedMassRatio<ZBeta, S, f64>> {
    MixedMassRatio::<ZBeta, S, f64>::new(with_mass(mass), [-2.0, 2.0], asked, pair, mapping, p)
}

fn mu(mu_max: f64) -> LinkParams<f64> {
    LinkParams {
        mu_max,
        ..LinkParams::DEFAULT
    }
}

/// The predicate without the held mass's test: the round trip's masses positive, summing to 1 and with ratio `q`, to
/// within `ε_μ`, relative (the negative controls' predicate).
fn ratio_only<S: Selection>(
    chart: &MixedMassRatio<ZBeta, S, f64>,
    q: f64,
    p: &LinkParams<f64>,
) -> bool {
    let m = ratio_masses(chart.bodies, chart.held, q);
    let link = <S::MassLink as Link<2, 3>>::inverse_at(m, p);
    let back = <S::MassLink as Link<2, 3>>::forward_at(link, p);
    let sum: f64 = back.iter().sum();
    back.iter().all(|&x| x > 0.0)
        && (sum - 1.0).abs() <= RATIO_TOL
        && (ratio_of(chart.bodies, back) - q).abs() <= RATIO_TOL * q
}

/// The predicate the chart's check applies: `realised` where `own`, else [`ratio_only`].
fn predicate<S: Selection>(
    own: bool,
) -> impl Fn(&MixedMassRatio<ZBeta, S, f64>, f64, &LinkParams<f64>) -> bool {
    move |chart, q, p| {
        if own {
            chart.realised(q, p)
        } else {
            ratio_only(chart, q, p)
        }
    }
}

/// A ratio past the `m_c` logit's clamp is not realised where the round trip moves `m_c`: under `softmax_tanh` on
/// [`SATURATING`] (`q = 2` on the first two, 0.875 below the bottom's clamp on the third), and under `stick_breaking`,
/// `q = 2`, with the held mass the reference body's at mass controls `(8, 0)`, `m₀ ≈ 6·10⁻⁷`, whose first coordinate
/// the inverse clamps (bodies `(1, 2)`). The linear chart refuses its pixel at that ratio, and `stick_breaking`'s
/// warped chart, realising no ratio at that `m_c`, is none.
fn check_held_refused(own: bool) {
    let refused = |name: &str, realised: bool, q: f64, m: [f64; 3], held: f64| {
        assert!(
            !realised,
            "{name} realises q = {q}, its round trip at {m:?} moving m_c off the held {held}"
        );
    };
    for s in &SATURATING {
        let p = mu(s.mu_max);
        let chart = saturating_chart::<Defaults>(s.mass, s.pair, s.asked, Mapping::Warped, &p)
            .expect("a realised range");
        let m = ratio_masses(s.pair, chart.held, s.past);
        let back = <SoftmaxTanh as Link<2, 3>>::forward_at(SoftmaxTanh::inverse_at(m, &p), &p);
        assert!(
            (ratio_of(s.pair, back) - s.past).abs() <= RATIO_TOL * s.past,
            "the round trip does not keep the ratio {}",
            s.past
        );
        refused(
            &format!(
                "softmax_tanh at μ_max = {}, mass controls {:?}",
                s.mu_max, s.mass
            ),
            predicate::<Defaults>(own)(&chart, s.past, &p),
            s.past,
            back,
            chart.held,
        );
        let around = [0.5 * s.past, 1.5 * s.past];
        let linear = saturating_chart::<Defaults>(s.mass, s.pair, around, Mapping::Linear, &p)
            .expect("a linear chart");
        assert_eq!(linear.ratio_at(0.5), s.past);
        assert!(
            matches!(
                linear.validate_params(0.5, 0.5, &p),
                ValidationResult::Reject { .. }
            ),
            "the linear chart takes q = {} at μ_max = {}, mass controls {:?}",
            s.past,
            s.mu_max,
            s.mass
        );
    }
    let p = LinkParams::DEFAULT;
    let stick = |mapping| saturating_chart::<Stick>([8.0, 0.0], [1, 2], [1.0, 3.0], mapping, &p);
    assert!(
        stick(Mapping::Warped).is_none(),
        "stick_breaking's warped chart realises a ratio"
    );
    let linear = stick(Mapping::Linear).expect("a linear chart");
    let m = ratio_masses([1, 2], linear.held, 2.0);
    let back = <StickBreaking as Link<2, 3>>::forward_at(StickBreaking::inverse_at(m, &p), &p);
    refused(
        "stick_breaking at mass controls (8, 0)",
        predicate::<Stick>(own)(&linear, 2.0, &p),
        2.0,
        back,
        linear.held,
    );
    assert_eq!(linear.ratio_at(0.5), 2.0);
    assert!(
        matches!(
            linear.validate_params(0.5, 0.5, &p),
            ValidationResult::Reject { .. }
        ),
        "stick_breaking's linear chart takes q = 2"
    );
}

#[test]
fn physical_axis_warps_mass_ratio_held_refused() {
    check_held_refused(true);
}

negative_control!(
    physical_axis_warps_mass_ratio_held_refused,
    "the ratio-only predicate, which never compares m_c with the held mass, admits q = 2, so it must fail",
    expected = "realises q =",
    check_held_refused(false)
);

/// The warped range's end where the `m_c` logit clamps: bodies `(0, 1)` under `softmax_tanh`,
/// `ln(m_c/m₀) = ln(m_c·(1 + q)/(1 − m_c))` reaches `±(1 − ε_μ)·μ_max` at `q* = e^{±(1 − ε_μ)μ_max}·(1 − m_c)/m_c − 1`,
/// the top on [`SATURATING`]'s first two, the bottom on its third, the other end the asked one, the end past `q*` by
/// no more than `ε_μ` over the round trip's slope there. `at_ratio` names the
/// formula the check expects instead: the end where the ratio's own logit `ln q` clamps, `e^{±(1 − ε_μ)μ_max}`, or the
/// asked end, what a predicate blind to `m_c` gives.
fn check_held_edge(at_ratio: bool) {
    let eps = <f64 as ChartConstants>::EPS_MU;
    for (k, s) in SATURATING.iter().enumerate() {
        let p = mu(s.mu_max);
        let chart = saturating_chart::<Defaults>(s.mass, s.pair, s.asked, Mapping::Warped, &p)
            .expect("a realised range");
        let h = chart.held;
        let up = k < 2;
        let sign = if up { 1.0 } else { -1.0 };
        let clamp = (sign * (1.0 - eps) * s.mu_max).exp();
        let want = if at_ratio {
            if up {
                clamp.min(s.asked[1])
            } else {
                clamp.max(s.asked[0])
            }
        } else {
            clamp * (1.0 - h) / h - 1.0
        };
        let [lo, hi] = chart.ratio;
        let (end, other, asked) = if up {
            (hi, lo, s.asked[0])
        } else {
            (lo, hi, s.asked[1])
        };
        // Past q*, the round trip's m_c moves as d ln m_c/d ln q = −(1 − m_c)·q/(1 + q), so the bisection's end, where
        // m_c's error reaches ε_μ/2, lies past q* by about ε_μ/2 over that slope: at most ε_μ over it, relative.
        let slope = (1.0 - h) * want / (1.0 + want);
        let past = if up {
            end / want - 1.0
        } else {
            want / end - 1.0
        };
        assert!(
            (-1e-9..=eps / slope).contains(&past) && other == asked,
            "the range [{lo}, {hi}] at μ_max = {}, mass controls {:?}: its end {end} is not at {want}",
            s.mu_max,
            s.mass
        );
    }
}

#[test]
fn physical_axis_warps_mass_ratio_held_edge() {
    check_held_edge(false);
}

negative_control!(
    physical_axis_warps_mass_ratio_held_edge,
    "the range's end at the ratio logit's clamp, or the asked end, is where a predicate blind to m_c puts it, so it \
     must fail",
    expected = "is not at",
    check_held_edge(true)
);

/// Every pixel of `[0,1]²` on [`SATURATING`]'s warped charts decodes the third body's mass at the held `m_c`, to within
/// `ε_μ`, relative; `chart` builds the chart from the slice, its mass controls and `μ_max`.
fn check_held_pixels(
    chart: impl Fn(&Saturating, &LinkParams<f64>) -> MixedMassRatio<ZBeta, Defaults, f64>,
) {
    for s in &SATURATING {
        let p = mu(s.mu_max);
        let chart = chart(s, &p);
        for [u, v] in grid(0.0, 1.0) {
            let m_c = chart.decode_params::<Defaults, f64>(u, v, &p).ic.m[third(s.pair)];
            assert!(
                (m_c - chart.held).abs() <= RATIO_TOL * chart.held,
                "μ_max = {}, mass controls {:?}: the pixel ({u}, {v}) at q = {} decodes m_c = {m_c}, not the held {}",
                s.mu_max,
                s.mass,
                chart.ratio_at(v),
                chart.held
            );
        }
    }
}

#[test]
fn physical_axis_warps_mass_ratio_held_pixels() {
    check_held_pixels(|s, p| {
        saturating_chart(s.mass, s.pair, s.asked, Mapping::Warped, p).expect("a realised range")
    });
}

negative_control!(
    physical_axis_warps_mass_ratio_held_pixels,
    "the linear chart over the asked range draws ratios past the m_c logit's clamp, whose decode moves m_c, so it must \
     fail",
    expected = "not the held",
    check_held_pixels(|s, p| {
        saturating_chart(s.mass, s.pair, s.asked, Mapping::Linear, p).expect("a linear chart")
    })
);

/// The ratios `realised(chart, q)` takes on the grid `q = 10^{k/100}`, `k ∈ [−600, 600]`, are one run, not empty, on
/// [`SATURATING`] and `Z0` at each body pair under `softmax_tanh` at both ends of `μ_max`'s range and its default, and
/// under `stick_breaking`: the realised set is one interval in `ln q`, which `edge`'s bisection needs.
fn check_interval<S: Selection>(
    link: &str,
    realised: &impl Fn(&MixedMassRatio<ZBeta, S, f64>, f64, &LinkParams<f64>) -> bool,
    mass: [f64; 2],
    pair: [u32; 2],
    p: &LinkParams<f64>,
) {
    let chart =
        saturating_chart::<S>(mass, pair, [0.5, 2.0], Mapping::Linear, p).expect("a linear chart");
    let taken: Vec<bool> = (-600..=600)
        .map(|k| realised(&chart, 10f64.powf(f64::from(k) / 100.0), p))
        .collect();
    let starts = taken.windows(2).filter(|w| !w[0] && w[1]).count() + usize::from(taken[0]);
    assert!(
        starts == 1,
        "{link} at μ_max = {}, mass controls {mass:?}, bodies {pair:?}: the realised ratios are {starts} runs, not one \
         interval",
        p.mu_max
    );
}

/// [`check_interval`] on each slice, the chart's own predicate, with the band `|q − 1| < 0.1` refused where `gap`.
fn check_intervals(gap: bool) {
    let softmax = |c: &MixedMassRatio<ZBeta, Defaults, f64>, q: f64, p: &LinkParams<f64>| {
        c.realised(q, p) && !(gap && (q - 1.0).abs() < 0.1)
    };
    let stick = |c: &MixedMassRatio<ZBeta, Stick, f64>, q: f64, p: &LinkParams<f64>| {
        c.realised(q, p) && !(gap && (q - 1.0).abs() < 0.1)
    };
    for s in &SATURATING {
        check_interval("softmax_tanh", &softmax, s.mass, s.pair, &mu(s.mu_max));
    }
    let range = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's range");
    for pair in [[0, 1], [0, 2], [1, 2], [2, 0]] {
        let z = [Z0[6], Z0[7]];
        check_interval("stick_breaking", &stick, z, pair, &LinkParams::DEFAULT);
        check_interval(
            "stick_breaking",
            &stick,
            [-3.0, 2.0],
            pair,
            &LinkParams::DEFAULT,
        );
        for mu_max in [range.lo, range.default, range.hi] {
            check_interval("softmax_tanh", &softmax, z, pair, &mu(mu_max));
        }
    }
}

#[test]
fn physical_axis_warps_mass_ratio_held_interval() {
    check_intervals(false);
}

negative_control!(
    physical_axis_warps_mass_ratio_held_interval,
    "a predicate refusing the band |q − 1| < 0.1 inside the realised set splits it in two runs, so it must fail",
    expected = "not one interval",
    check_intervals(true)
);
