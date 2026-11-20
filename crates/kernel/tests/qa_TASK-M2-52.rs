//! qa's tests for TASK-M2-52, written from REQ-CHART-062's R-633 arms and REQ-CHART-065 (chart_reference §4.5,
//! "Hidden coordinates under tilt and slice", and §2.2's construction) as R-708, R-709 and R-720 rule them, each with
//! its negative control (R-176). `I = 1` throughout.
//!
//! - `qa_tilt_every_chart_hidden_set`: each of the six construction charts offers exactly §4.5's table's hidden
//!   coordinates: the mass simplex `ν₀` and the four momentum controls, the Euclid plane and the δm strip the four
//!   momentum controls, `(ν, K)` and `Φ_θ,K` the held `L_z`, `Φ_θ,L_z` the held `K`.
//! - `qa_tilt_every_chart_held_lz`, warped, the default (R-708, R-720): `(ν, K)` and `Φ_θ,K` (`Φ_strip,K` the same map)
//!   tilted 30° into the held `L_z`, about a centre at 0 and at each sign, at a slider span of 2 and of 40: over a 33 × 33
//!   grid of the square's vertices, the bottom row `v = 0` among them, no cell is refused, and each is §2.2's construction
//!   at `L_z = L_z₀ + d`, by value at both signs of `d`, and `K = L_z²/2 + K_max·b^γ_K`, the floor at the pixel's own `L_z`,
//!   past `K_max` where the floor takes it there. At `L_z = 0` the held chart is the registered one, bit for bit.
//! - `qa_tilt_every_chart_held_lz_linear`, linear (R-708, R-633's refusal): rows at the fixed `K(b) = K_max·b`, the held
//!   `L_z + d` by value: where `K(b) ≥ L_z²/2` the momenta satisfy §2.2's three constraints, `Σp = 0`, `L_z(p)` and
//!   `K(p)`, to machine precision, the cell valid and weighted as the tilt; where `K(b) < L_z²/2` the cell is refused:
//!   `DEGENERATE(INVARIANT_INFEASIBLE)`, the view's check a rejection, area weight 0, and its momenta §2.2's `a = 0`
//!   ones. Both kinds occur on the window.
//! - `qa_tilt_every_chart_bottom_row`: `t = 0` at a held `L_z` whose `l²` rounds below the exact value is a system: the
//!   floor's rounding is handled (R-708's "must not blank the bottom row").
//! - `qa_tilt_every_chart_held_k`, linear: `Φ_θ,L_z` tilted into the held `K`, bare and as `HeldK` in linear mode, the
//!   refusal arm with `K* = K + d` and `L_z(b) = (2b − 1)·λ_L·√(2K)` at the centre's `K`.
//! - `qa_tilt_every_chart_held_k_warped`, warped (R-708, R-709): the held `K` moved in its warp coordinate,
//!   `K = K_max·t^γ_K`, `t = t₀ + d`, the band at the pixel's own `K`: no cell refused; past `t = 0` the pixel is clamped
//!   there, a real system at rest, flagged, not mirrored; the clamped-pixel share in the test output.
//! - `qa_tilt_every_chart_nu0`: the mass simplex tilted into `ν₀`: each pixel's masses are §4.5's ternary map at
//!   `(a, b)` and its triangle is the Burrau right triangle at `ν₀ + d` (side ratios `(1 − ν²) : 2ν : (1 + ν²)`
//!   opposite bodies 2, 1, 0), at rest; where `ν₀ + d ∉ (0, 1)` the cell is refused, weight 0.
//! - `qa_tilt_every_chart_rest_start`: each rest-start chart tilted into each momentum control: masses and positions
//!   the untilted chart's at `(a, b)`, momenta `D_mom` of the control at `d` (the others 0) under the pixel's masses,
//!   `Σp = 0`; every cell valid. The centre pixel of every arm is the untilted chart's, bitwise.
//! - `qa_slice_every_chart_construction_edge`: REQ-CHART-060's stop. A physical slice past the construction's edge
//!   (`ν₀ ∉ (0, 1)`, a held `K < 0`, and in linear mode a held `L_z` past `√(2K(v))`) leaves a centre that `validate`
//!   refuses; one inside passes. In warped mode a held `L_z` has no such edge: the floor keeps it a system at any `L_z`
//!   (R-720), and a held `K` stops at `K₀ = 0`.
//! - `qa_slice_every_chart_construction`: a slice of each hidden coordinate moves only that held value; the decode at
//!   the centre follows (the sliced `L_z` by value at both signs, its row's `K` by the mode; `K(p) = K₀ + d`, its band
//!   following, and the tilt's `t₀` recomputed from the new `K₀`; the triangle at the sliced `ν₀`; `D_mom` of the sliced
//!   control); a slice of any other index is not offered.
//!
//! - `qa_tilt_every_chart_shifted_energy_bound`, the fix round: `Φ_θ,L_z`'s `L_z` axis is affine (§5.4), so by value the
//!   bound on a row past the centre's band is `K + d`'s, the pixel's: a system exactly where `2(K + d) ≥ L_z(b)²`. Warped,
//!   the band moves with `K`, so a row past it is refused at every `K > 0`.
//! - `qa_no_construction_finite_2k` and `qa_decode_no_finite_2k`: a `K*` has a construction only with a finite `2K*`
//!   (§4.5, §2.2's `a = √(2K*)`), at each float: the predicate against `(k + k).is_finite()`, and the decode labelled, its
//!   momenta 0 and finite, the same for an `L_z` with no finite square; the last values below each edge stay systems.
//! - `qa_refusal_at_chart_precision`: a target with a construction at f64 and none at an f32 chart's own precision is
//!   refused by `validate`, weight 0, not just tagged by the decode; at one f32 target the label and the refusal agree.
//! - `qa_refusal_at_chart_precision_view`: the same through an f32 view, on the placement the f32 decode reads: the
//!   held-value charts, the held `L_z` and the held `K` by value at a non-zero `d`, have no labelled pixel left valid.
//! - `qa_refusal_at_placed_domain_end`: each chart's domain ends through an f32 view, walked float by float: a pixel placed
//!   outside at f32 is refused where the check at the f64 placement passes, and one inside at both stays a system. A
//!   domain end's weight is `qa_weight_reads_refusals_only`'s.
//! - `qa_view_check_reads_own_parameters`: REQ-DEC-048's active value on a view's check at the chart's own precision: at
//!   `μ_max` 1, 1.7 and 8, at f64 and f32, on a linear energy axis, the view accepts a pixel exactly where the chart's
//!   check passes and the decode at that `p`, at that precision, is a system; `p` changes the answer on each arm.
//! - `qa_construction_check_free_of_parameters`: the construction charts read no link for their systems, so on all of
//!   them, at f64 and f32, tilted, the check at any `μ_max` and `q_max` is the check at the defaults.
//! - `qa_weight_reads_refusals_only`: REQ-CHART-062's area weight 0 is the refusal's: on every chart, at f64 and f32, over
//!   a window past the square, the weight is 0 exactly where the chart refuses at f64 or at its own precision, and there
//!   the check rejects too; a rejection that is no refusal, a domain end or a linear energy axis' infeasible pixel, keeps
//!   the tilt's. The weight takes no parameters.
//!
//! Controls: the held `L_z` read without `d`; a floor at the centre's `L_z`, not the pixel's; a held `L_z` moved in a warp
//! coordinate, or held to one sign, tilted and sliced; a bottom row left to the rounding of `l²/2`; a refused cell
//! weighted as the tilt; a held `K` tilted by value; a mirrored `t < 0`; a held `K` slice moved in `t`, or a `t₀` not
//! recomputed; the triangle read at `ν₀` without `d`; a momentum control's `d` put on the next control; a `K` slice that
//! leaves the momenta's `K` unchanged; a warped `L_z` past the band expected refused; layer 3 read at the defaults, not
//! `p`; a chart that does read `p` put through the construction charts' check; a weight that reads the check, or layer 3
//! at the defaults; a refused pixel weighted as the tilt.

use std::io::Write;

use kernel::chart::burrau_acute::decode_theta_lz_at;
use kernel::chart::construction::{Held, LzChart, HELD_K, HELD_LZ, NU0};
use kernel::chart::invariant::Invariant;
use kernel::chart::tilt::{Hidden, Shift, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, EuclidPlane, Float, HeldK,
    HeldLz, InvariantLE, Mapped, Mapping, MassSimplex, RestStart, Tilt, TiltMap, TiltSpec, Tilted,
    ValidationResult,
};
use kernel::decode::invariant::{infeasible, no_construction};
use kernel::decode::{momentum, Decoded, Degenerate};
use kernel::generated::links::{Defaults, LinkParams, LinkReal};
use kernel::payload::PayloadReal;
use validation::negative_control;

/// The view's centre, off `(½, ½)`.
const C: [f64; 2] = [0.37, 0.61];
/// §2.2's "machine precision" on `O(1)` sums of a few products: a few hundred ulp.
const TOL: f64 = 1e-13;

fn grid(n: u32) -> Vec<[f64; 2]> {
    let at = |k: u32| (f64::from(k) + 0.5) / f64::from(n);
    (0..n)
        .flat_map(|j| (0..n).map(move |i| [at(i), at(j)]))
        .collect()
}

fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

/// The two single-angle tilts the arms take, with the closed form of `R·(ũ, ṽ, 0)` for each: Tip as sheet on `τ₁`,
/// `(a, b, d) = (c_u + cos τ·ũ, v, sin τ·ũ)`; Lean each axis on `τ₂`, `(u, c_v + cos τ·ṽ, sin τ·ṽ)`.
#[derive(Clone, Copy)]
enum Arm {
    SheetT1,
    LeanT2,
}

impl Arm {
    fn spec(self, x: u32) -> TiltSpec {
        let (tau1, tau2, map) = match self {
            Arm::SheetT1 => (30.0, 0.0, TiltMap::Sheet),
            Arm::LeanT2 => (0.0, 30.0, TiltMap::Lean),
        };
        TiltSpec {
            tau1,
            tau2,
            map,
            x,
            y: x,
        }
    }

    /// `(a, b, d)` for the pixel `(u, v)`, `d` in units of the slider span `span`.
    fn place(self, u: f64, v: f64, span: f64) -> (f64, f64, f64) {
        let (s, c) = 30f64.to_radians().sin_cos();
        match self {
            Arm::SheetT1 => (C[0] + c * (u - C[0]), v, span * s * (u - C[0])),
            Arm::LeanT2 => (u, C[1] + c * (v - C[1]), span * s * (v - C[1])),
        }
    }
}

fn view<Ch: Tiltable<Param = f64>>(chart: Ch, arm: Arm, x: u32, span: f64) -> Tilted<Ch, f64> {
    Tilted::new(chart, Tilt::new(&arm.spec(x), C, [span, span])).expect("a hidden target")
}

fn total_p(d: &Decoded<f64>) -> [f64; 2] {
    let p = d.ic.p;
    [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]]
}

fn lz_of(d: &Decoded<f64>) -> f64 {
    (0..3)
        .map(|i| d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0])
        .sum()
}

fn k_of(d: &Decoded<f64>) -> f64 {
    (0..3)
        .map(|i| (d.ic.p[i][0].powi(2) + d.ic.p[i][1].powi(2)) / (2.0 * d.ic.m[i]))
        .sum()
}

fn nu_k() -> Burrau<f64> {
    Burrau {
        nu_range: [0.1, 0.9],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

fn theta_k() -> BurrauThetaK<f64> {
    BurrauThetaK {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

fn theta_lz(k: f64) -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        k,
        lz_fraction: 1.0,
    }
}

fn simplex(nu0: f64) -> MassSimplex<f64> {
    MassSimplex { nu0, eps_m: 1e-4 }
}

fn euclid() -> EuclidPlane<f64> {
    EuclidPlane {
        m_range: [1.0, 32.0],
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
    }
}

fn delta_m() -> BurrauThetaDeltaM<f64> {
    BurrauThetaDeltaM {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        m_target: BurrauThetaDeltaM::equal_masses(),
    }
}

// ── The hidden set ───────────────────────────────────────────────────────────────────────────────────────────────────

fn hidden_set<Ch: Hidden>(c: &Ch) -> Vec<u32> {
    (0..16).filter(|&k| c.hidden(k)).collect()
}

/// `swap`: the control's wrong table, `ν₀` claimed on the Euclid plane and the held `K` on `(ν, K)`.
fn check_hidden_set(swap: bool) {
    let mom = vec![2, 3, 4, 5];
    let mut with_nu0 = mom.clone();
    with_nu0.push(NU0);
    let (euclid_set, nu_k_set) = if swap {
        (with_nu0.clone(), vec![HELD_K])
    } else {
        (mom.clone(), vec![HELD_LZ])
    };
    let cases: [(&str, Vec<u32>, Vec<u32>); 7] = [
        (
            "the mass simplex",
            hidden_set(&RestStart::new(simplex(0.5))),
            with_nu0,
        ),
        (
            "the Euclid plane",
            hidden_set(&RestStart::new(euclid())),
            euclid_set,
        ),
        ("the δm strip", hidden_set(&RestStart::new(delta_m())), mom),
        ("(ν, K)", hidden_set(&HeldLz::new(nu_k())), nu_k_set),
        ("Φ_θ,K", hidden_set(&HeldLz::new(theta_k())), vec![HELD_LZ]),
        ("Φ_θ,L_z", hidden_set(&theta_lz(0.5)), vec![HELD_K]),
        (
            "Φ_θ,L_z, held",
            hidden_set(&HeldK::new(theta_lz(0.5), 1.0, 1.5)),
            vec![HELD_K],
        ),
    ];
    for (name, got, want) in cases {
        assert_eq!(got, want, "the hidden set of {name} is not §4.5's table's");
    }
    // A physical index is distinct from every latent control.
    for k in [NU0, HELD_LZ, HELD_K] {
        assert!(k >= 8, "{k} collides with a latent control");
    }
}

#[test]
fn qa_tilt_every_chart_hidden_set() {
    check_hidden_set(false);
}

negative_control!(
    qa_tilt_every_chart_hidden_set,
    "a chart claimed to offer another chart's hidden coordinate must fail",
    expected = "is not §4.5's table's",
    {
        check_hidden_set(true);
    }
);

// ── The held L_z and the held K ──────────────────────────────────────────────────────────────────────────────────────

/// The square's 33 × 33 vertices, its edges included: the bottom row `v = 0` is itself a row (R-708).
fn vertices() -> Vec<[f64; 2]> {
    let at = |k: u32| f64::from(k) / 32.0;
    (0..33)
        .flat_map(|j| (0..33).map(move |i| [at(i), at(j)]))
        .collect()
}

/// A hair above `l²/2`, so that a control's wrong `L_z` is a construction and fails on its value, not on the rounding.
fn above_floor(l: f64) -> f64 {
    0.5 * l * l * (1.0 + 4.0 * f64::EPSILON)
}

#[derive(Clone, Copy, PartialEq)]
enum Ctl {
    None,
    /// The held value read without `d`.
    NoD,
    /// A refused cell weighted as the tilt.
    WeightRefused,
}

/// The linear arm, over a 33 × 33 grid of `view`: `target(a, b, d)` gives `(L_z, K*)`; the §2.2 constraints on each
/// feasible cell, refusal on each infeasible one.
fn check_construction<Ch: Tiltable<Param = f64>>(
    name: &str,
    view: &Tilted<Ch, f64>,
    arm: Arm,
    span: f64,
    target: impl Fn(f64, f64, f64) -> (f64, f64),
    centre: Decoded<f64>,
    ctl: Ctl,
) {
    let (mut valid, mut refused) = (0, 0);
    let w = view.tilt().weight;
    assert!(w > 0.0, "{name}: the tilt's weight is {w}");
    for [u, v] in grid(33) {
        let (a, b, d) = arm.place(u, v, span);
        let (lz, k) = target(a, b, if ctl == Ctl::NoD { 0.0 } else { d });
        let gap = 2.0 * k - lz * lz;
        if gap.abs() < 1e-9 {
            continue; // On §2.2's boundary: an ulp of d decides it.
        }
        let got = view.decode::<Defaults, f64>(u, v);
        let p = total_p(&got);
        assert!(
            p[0].abs() < TOL && p[1].abs() < TOL,
            "{name} at ({u}, {v}): Σp = {p:?}"
        );
        assert!(
            (lz_of(&got) - lz).abs() < TOL,
            "{name} at ({u}, {v}): L_z(p) = {} does not realise the tilted L_z {lz}",
            lz_of(&got)
        );
        let weight = if ctl == Ctl::WeightRefused {
            w
        } else {
            view.weight(u, v)
        };
        if gap >= 0.0 {
            valid += 1;
            assert!(
                (k_of(&got) - k).abs() < TOL,
                "{name} at ({u}, {v}): K(p) = {} is not K* = {k}",
                k_of(&got)
            );
            assert_eq!(got.degenerate, None, "{name} at ({u}, {v})");
            assert_eq!(
                view.validate(u, v),
                ValidationResult::Valid,
                "{name} at ({u}, {v})"
            );
            assert_eq!(view.weight(u, v), w, "{name} at ({u}, {v})");
        } else {
            refused += 1;
            assert_eq!(
                got.degenerate,
                Some(Degenerate::InvariantInfeasible),
                "{name} at ({u}, {v}): K* < L_z²/2I is not INVARIANT_INFEASIBLE"
            );
            assert!(
                (k_of(&got) - lz * lz / 2.0).abs() < TOL,
                "{name} at ({u}, {v}): a refused cell's momenta are not a = 0's"
            );
            assert!(
                matches!(view.validate(u, v), ValidationResult::Reject { .. }),
                "{name} at ({u}, {v}): a refused cell validates as {:?}",
                view.validate(u, v)
            );
            assert!(
                weight == 0.0,
                "{name} at ({u}, {v}): refused cell weighted {weight}"
            );
        }
    }
    assert!(
        valid > 100 && refused > 20,
        "{name}: the window must hold both kinds: {valid} valid, {refused} refused"
    );
    // The centre pixel is the untilted chart's, bit for bit (anchor-preserving).
    assert_eq!(
        bits(&view.decode::<Defaults, f64>(C[0], C[1])),
        bits(&centre),
        "{name}: the centre moved"
    );
}

/// What a warped arm decodes, for a control: the view's own pixel, or the wrong system a wrong rule would build.
#[derive(Clone, Copy, PartialEq)]
enum Warped {
    AsRuled,
    /// The held `L_z` without `d`.
    NoD,
    /// The floor at the centre's held `L_z`, not the pixel's.
    CentreFloor,
    /// The held `L_z` moved in the warp coordinate `s` of §2.1's `L_z = (2s − 1)·L_max`, `s` off its value by `d`.
    WarpLz,
    /// The held `L_z` held to one sign.
    OneSign,
}

/// R-708 and R-720 on one chart, its `K` axis `K_max·b^γ_K` and its held `L_z` about `centre`, in the mode a new
/// [`HeldLz`] takes: every vertex of the square a system, `L_z = L_z₀ + d`, `K = L_z²/2 + K_max·b^γ_K`.
fn check_warped_lz<C: LzChart<Param = f64>>(
    name: &str,
    chart: C,
    [k_max, gamma]: [f64; 2],
    (centre, arm, span): (f64, Arm, f64),
    alter: Warped,
) {
    let held = HeldLz {
        lz: centre,
        ..HeldLz::new(chart)
    };
    let tv = view(held, arm, HELD_LZ, span);
    let w = tv.tilt().weight;
    let (mut d_signs, mut lz_signs, mut above) = ([0, 0], [0, 0], 0);
    for [u, v] in vertices() {
        let (a, b, d) = arm.place(u, v, span);
        let lz = centre + d;
        let line = k_max * b.powf(gamma);
        let k = 0.5 * lz * lz + line;
        let got = if alter == Warped::NoD {
            chart.decode_lz(a, centre, above_floor(centre) + line)
        } else if alter == Warped::CentreFloor {
            chart.decode_lz(a, lz, above_floor(centre) + line)
        } else if alter == Warped::WarpLz {
            let moved = centre + 2.0 * d * (2.0 * k).sqrt();
            chart.decode_lz(a, moved, above_floor(moved) + line)
        } else if alter == Warped::OneSign {
            chart.decode_lz(a, lz.abs(), above_floor(lz) + line)
        } else {
            tv.decode::<Defaults, f64>(u, v)
        };
        assert_eq!(
            got.degenerate, None,
            "{name} at ({u}, {v}), L_z = {lz}: a cell refused under tilt"
        );
        assert_eq!(
            tv.validate(u, v),
            ValidationResult::Valid,
            "{name} at ({u}, {v})"
        );
        assert_eq!(tv.weight(u, v), w, "{name} at ({u}, {v})");
        let tol = TOL * (1.0 + k);
        let p = total_p(&got);
        assert!(
            p[0].abs() < tol && p[1].abs() < tol,
            "{name} at ({u}, {v}): Σp = {p:?}"
        );
        assert!(
            (lz_of(&got) - lz).abs() < tol,
            "{name} at ({u}, {v}): L_z(p) = {} does not realise the tilted L_z {lz}",
            lz_of(&got)
        );
        assert!(
            (k_of(&got) - k).abs() < tol,
            "{name} at ({u}, {v}): K(p) = {} is not the floor plus the warp's line, {k}",
            k_of(&got)
        );
        d_signs[usize::from(d > 0.0)] += 1;
        lz_signs[usize::from(lz > 0.0)] += 1;
        above += usize::from(k > k_max);
    }
    assert!(
        d_signs.iter().all(|&c| c > 100) && lz_signs.iter().all(|&c| c > 20),
        "{name}: both signs: d {d_signs:?}, L_z {lz_signs:?}"
    );
    // R-720's "a column above K_max at large tilt": the wide span reaches it on every arm.
    assert!(span < 40.0 || above > 0, "{name}: no cell above K_max");
    assert_eq!(
        bits(&tv.decode::<Defaults, f64>(C[0], C[1])),
        bits(&held.decode::<Defaults, f64>(C[0], C[1])),
        "{name}: the centre moved"
    );
    if centre == 0.0 {
        for [u, v] in vertices() {
            assert_eq!(
                bits(&held.decode::<Defaults, f64>(u, v)),
                bits(&chart.decode::<Defaults, f64>(u, v)),
                "{name}: L_z = 0 is not the registered chart at ({u}, {v})"
            );
        }
    }
}

fn check_held_lz(alter: Warped) {
    for centre in [0.0, 0.15, -0.3] {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            for span in [2.0, 40.0] {
                let at = (centre, arm, span);
                check_warped_lz("(ν, K)", nu_k(), [1.0, 1.5], at, alter);
                check_warped_lz("Φ_θ,K", theta_k(), [1.0, 1.5], at, alter);
            }
        }
    }
}

#[test]
fn qa_tilt_every_chart_held_lz() {
    check_held_lz(Warped::AsRuled);
}

negative_control!(
    qa_tilt_every_chart_held_lz,
    "the held L_z read without the tilt's d must fail",
    expected = "does not realise the tilted L_z",
    {
        check_held_lz(Warped::NoD);
    }
);

mod centre_floor {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_lz,
        "a floor at the centre's held L_z, not the pixel's, leaves no construction and must fail",
        expected = "a cell refused under tilt",
        {
            check_held_lz(Warped::CentreFloor);
        }
    );
}

mod warp_lz {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_lz,
        "a held L_z moved in a warp coordinate is not L_z0 + d and must fail",
        expected = "does not realise the tilted L_z",
        {
            check_held_lz(Warped::WarpLz);
        }
    );
}

mod one_sign {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_lz,
        "a held L_z held to one sign is not L_z0 + d and must fail",
        expected = "does not realise the tilted L_z",
        {
            check_held_lz(Warped::OneSign);
        }
    );
}

/// Linear mode: the rows `K(b) = K_max·b`, fixed, the held `L_z` by value, cells below the floor refused.
fn check_linear_lz(ctl: Ctl) {
    let span = 2.0;
    for centre_lz in [0.0, 0.15] {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            let a = HeldLz {
                lz: centre_lz,
                mapping: Mapping::Linear,
                ..HeldLz::new(nu_k())
            };
            let ka = a.chart.k_max;
            check_construction(
                "(ν, K), linear",
                &view(a, arm, HELD_LZ, span),
                arm,
                span,
                |_, b, d| (centre_lz + d, ka * b),
                a.decode::<Defaults, f64>(C[0], C[1]),
                ctl,
            );
            let t = HeldLz {
                lz: centre_lz,
                mapping: Mapping::Linear,
                ..HeldLz::new(theta_k())
            };
            let kt = t.chart.k_max;
            check_construction(
                "Φ_θ,K, linear",
                &view(t, arm, HELD_LZ, span),
                arm,
                span,
                |_, b, d| (centre_lz + d, kt * b),
                t.decode::<Defaults, f64>(C[0], C[1]),
                ctl,
            );
        }
    }
}

#[test]
fn qa_tilt_every_chart_held_lz_linear() {
    check_linear_lz(Ctl::None);
}

negative_control!(
    qa_tilt_every_chart_held_lz_linear,
    "the held L_z read without the tilt's d must fail",
    expected = "does not realise the tilted L_z",
    {
        check_linear_lz(Ctl::NoD);
    }
);

mod refused_weighted {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_lz_linear,
        "a refused cell counted with the tilt's area weight must fail",
        expected = "refused cell weighted",
        {
            check_linear_lz(Ctl::WeightRefused);
        }
    );
}

/// The bottom row at held `L_z` values, a good share of them with `l²` rounded below its exact value, so that `l²/2` is
/// under the floor: `t = 0` is a system on each. `naive` builds the construction at the rounded `l²/2` (the control).
fn check_bottom_row(naive: bool) {
    let mut rounded = 0;
    for i in 1..2048 {
        let x = (f64::from(i) * 0.754_877_666_246_692_7).fract();
        let l = 3.0 * x - 1.5;
        // `l·l − (l·l)_rounded`, exact: above 0 where the product rounded down.
        rounded += usize::from(l.mul_add(l, -(l * l)) > 0.0);
        let (a, t) = (
            HeldLz::new(nu_k()).sliced(HELD_LZ, l).unwrap(),
            HeldLz::new(theta_k()).sliced(HELD_LZ, l).unwrap(),
        );
        let (da, dt) = if naive {
            (
                nu_k().decode_lz(0.5, l, 0.5 * l * l),
                theta_k().decode_lz(0.5, l, 0.5 * l * l),
            )
        } else {
            (
                a.decode::<Defaults, f64>(0.5, 0.0),
                t.decode::<Defaults, f64>(0.5, 0.0),
            )
        };
        assert_eq!(
            da.degenerate, None,
            "the bottom row is blanked at L_z = {l}"
        );
        assert_eq!(
            dt.degenerate, None,
            "the bottom row is blanked at L_z = {l}"
        );
        assert_eq!(a.validate(0.5, 0.0), ValidationResult::Valid, "L_z = {l}");
        assert_eq!(t.validate(0.5, 0.0), ValidationResult::Valid, "L_z = {l}");
        assert!((lz_of(&da) - l).abs() < TOL, "L_z(p) at L_z = {l}");
        assert!((lz_of(&dt) - l).abs() < TOL, "L_z(p) at L_z = {l}");
    }
    assert!(
        rounded > 500,
        "{rounded} held values rounded below the floor"
    );
}

#[test]
fn qa_tilt_every_chart_bottom_row() {
    check_bottom_row(false);
}

negative_control!(
    qa_tilt_every_chart_bottom_row,
    "a bottom row left to the rounding of l²/2 must fail",
    expected = "the bottom row is blanked",
    {
        check_bottom_row(true);
    }
);

/// The linear arm of the held `K`: the bare chart's tilt, and [`HeldK`]'s in linear mode, by value, blank where
/// infeasible (R-708, R-709's "linear mode unchanged").
fn check_held_k(ctl: Ctl) {
    let span = 2.0;
    for centre_k in [0.5, 0.3] {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            let chart = theta_lz(centre_k);
            // L_z(b) = (2b − 1)·λ_L·√(2K) at the centre's K; K* = K + d.
            let target =
                |_: f64, b: f64, d: f64| ((2.0 * b - 1.0) * (2.0 * centre_k).sqrt(), centre_k + d);
            check_construction(
                "Φ_θ,L_z",
                &view(chart, arm, HELD_K, span),
                arm,
                span,
                target,
                chart.decode::<Defaults, f64>(C[0], C[1]),
                ctl,
            );
            let linear = HeldK {
                mapping: Mapping::Linear,
                ..HeldK::new(chart, 1.0, 1.5)
            };
            check_construction(
                "Φ_θ,L_z, linear",
                &view(linear, arm, HELD_K, span),
                arm,
                span,
                target,
                chart.decode::<Defaults, f64>(C[0], C[1]),
                ctl,
            );
        }
    }
}

#[test]
fn qa_tilt_every_chart_held_k() {
    check_held_k(Ctl::None);
}

mod held_k_controls {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_k,
        "the held K read without the tilt's d must fail",
        expected = "is not K* =",
        {
            check_held_k(Ctl::NoD);
        }
    );
}

/// What the warped held `K` arm decodes, for a control.
#[derive(Clone, Copy, PartialEq)]
enum WarpedK {
    AsRuled,
    /// The held `K` moved by value, as in linear mode.
    ByValue,
    /// `t < 0` mirrored onto `t > 0`.
    Mirror,
}

/// R-708 and R-709 on `Φ_θ,L_z`: every vertex a system at `K = K_max·t^γ_K`, `t = t₀ + d`, the band at it; a pixel past
/// `t = 0` clamped there at rest and flagged. The clamped pixels and the pixels counted.
fn check_warped_k(
    [k0, k_max, gamma]: [f64; 3],
    arm: Arm,
    span: f64,
    alter: WarpedK,
) -> (usize, usize) {
    let chart = HeldK::new(theta_lz(k0), k_max, gamma);
    let bare = chart.chart;
    let tv = view(chart, arm, HELD_K, span);
    let w = tv.tilt().weight;
    let t0 = (k0 / k_max).powf(1.0 / gamma);
    let (mut clamped, mut total) = (0, 0);
    for [u, v] in vertices() {
        let (a, b, d) = arm.place(u, v, span);
        let t = t0 + d;
        if t.abs() < 1e-12 {
            continue; // On the clamp: an ulp of d decides which side.
        }
        let k = if d == 0.0 {
            k0
        } else {
            k_max * t.max(0.0).powf(gamma)
        };
        let band = 2.0 * b - 1.0;
        let lz = band * (2.0 * k).sqrt();
        let got = if alter == WarpedK::ByValue {
            decode_theta_lz_at(&bare, a, band * (2.0 * k0).sqrt(), k0 + d).0
        } else if alter == WarpedK::Mirror {
            let k = k_max * t.abs().powf(gamma);
            decode_theta_lz_at(&bare, a, band * (2.0 * k).sqrt() * (1.0 - f64::EPSILON), k).0
        } else {
            tv.decode::<Defaults, f64>(u, v)
        };
        assert_eq!(
            got.degenerate, None,
            "Φ_θ,L_z at ({u}, {v}): a cell refused under tilt"
        );
        assert_eq!(
            tv.validate(u, v),
            ValidationResult::Valid,
            "Φ_θ,L_z at ({u}, {v})"
        );
        assert_eq!(tv.weight(u, v), w, "Φ_θ,L_z at ({u}, {v})");
        assert_eq!(
            tv.held_at_floor(u, v),
            t < 0.0,
            "Φ_θ,L_z at ({u}, {v}): the held-at-floor flag at t = {t}"
        );
        if t < 0.0 {
            clamped += 1;
            assert!(
                got.ic.p.iter().flatten().all(|&p| p == 0.0),
                "Φ_θ,L_z at ({u}, {v}): a clamped pixel is not at rest: {:?}",
                got.ic.p
            );
        }
        let tol = TOL * (1.0 + k);
        let p = total_p(&got);
        assert!(
            p[0].abs() < tol && p[1].abs() < tol,
            "Φ_θ,L_z at ({u}, {v}): Σp = {p:?}"
        );
        assert!(
            (k_of(&got) - k).abs() < tol && (lz_of(&got) - lz).abs() < tol,
            "Φ_θ,L_z at ({u}, {v}): (K, L_z)(p) = ({}, {}) is not the warp's ({k}, {lz})",
            k_of(&got),
            lz_of(&got)
        );
        total += 1;
    }
    assert_eq!(
        bits(&tv.decode::<Defaults, f64>(C[0], C[1])),
        bits(&bare.decode::<Defaults, f64>(C[0], C[1])),
        "Φ_θ,L_z: the centre moved"
    );
    (clamped, total)
}

/// A centre no tilt at the first span takes below `t = 0`, one that it does, under two warps, integer and not.
fn check_warped_k_all(alter: WarpedK) -> [(usize, usize); 2] {
    let mut sums = [(0, 0); 2];
    for arm in [Arm::SheetT1, Arm::LeanT2] {
        for (i, fixture) in [
            (0, [0.5, 1.0, 1.5]),
            (0, [0.5, 2.0, 2.0]),
            (1, [0.05, 1.0, 1.5]),
            (1, [0.05, 2.0, 2.5]),
        ] {
            let (clamped, total) = check_warped_k(fixture, arm, 1.0, alter);
            sums[i].0 += clamped;
            sums[i].1 += total;
            check_warped_k(fixture, arm, 8.0, alter);
        }
    }
    sums
}

#[test]
fn qa_tilt_every_chart_held_k_warped() {
    let [high, low] = check_warped_k_all(WarpedK::AsRuled);
    assert_eq!(high.0, 0, "no pixel clamped at the high centre");
    assert!(low.0 > 100, "the clamp arises: {low:?}");
    // R-709's report, past the harness's capture so that the test output shows it.
    writeln!(
        std::io::stderr(),
        "R-709 clamped-pixel share (qa): high centre {}/{}; low centre {}/{} = {:.4}",
        high.0,
        high.1,
        low.0,
        low.1,
        low.0 as f64 / low.1 as f64
    )
    .unwrap();
}

negative_control!(
    qa_tilt_every_chart_held_k_warped,
    "a held K tilted by value leaves cells with no construction and must fail",
    expected = "a cell refused under tilt",
    {
        check_warped_k_all(WarpedK::ByValue);
    }
);

mod mirror {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_k_warped,
        "t < 0 mirrored onto t > 0 is not the clamp and must fail",
        expected = "a clamped pixel is not at rest",
        {
            check_warped_k_all(WarpedK::Mirror);
        }
    );
}

// ── ν₀ on the mass simplex ───────────────────────────────────────────────────────────────────────────────────────────

fn dist(p: [f64; 2], q: [f64; 2]) -> f64 {
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
}

/// §4.5's ternary map at `(a, b)` with the interior buffer `ε_m`.
fn ternary(eps: f64, a: f64, b: f64) -> [f64; 3] {
    let (x, y) = (a, (1.0 - a) * b);
    [1.0 - x - y, x, y].map(|m| (1.0 - 3.0 * eps) * m + eps)
}

fn check_nu0(no_d: bool) {
    let nu0 = 0.5;
    for (span, refusing) in [(0.4, false), (2.0, true)] {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            let chart = RestStart::new(simplex(nu0));
            let tv = view(chart, arm, NU0, span);
            let w = tv.tilt().weight;
            let (mut valid, mut refused) = (0, 0);
            for [u, v] in grid(33) {
                let (a, b, d) = arm.place(u, v, span);
                let nu = nu0 + if no_d { 0.0 } else { d };
                if (nu - 1.0).abs() < 1e-9 || nu.abs() < 1e-9 {
                    continue;
                }
                if !(nu > 0.0 && nu < 1.0) {
                    refused += 1;
                    assert!(
                        matches!(tv.validate(u, v), ValidationResult::Reject { .. }),
                        "ν = {nu} at ({u}, {v}) is not refused"
                    );
                    assert_eq!(tv.weight(u, v), 0.0, "ν = {nu} at ({u}, {v}) weighted");
                    continue;
                }
                valid += 1;
                let got = tv.decode::<Defaults, f64>(u, v);
                let m = ternary(1e-4, a, b);
                for i in 0..3 {
                    assert!(
                        (got.ic.m[i] - m[i]).abs() < 1e-15,
                        "masses at ({u}, {v}): {:?} not {m:?}",
                        got.ic.m
                    );
                }
                let r = got.ic.r;
                let (s01, s02, s12) = (dist(r[0], r[1]), dist(r[0], r[2]), dist(r[1], r[2]));
                let c = 1.0 + nu * nu;
                assert!(
                    (s01 / s12 - (1.0 - nu * nu) / c).abs() < TOL
                        && (s02 / s12 - 2.0 * nu / c).abs() < TOL,
                    "at ({u}, {v}) the triangle is not the Burrau one at ν₀ + d = {nu}"
                );
                assert!(got.ic.p.iter().flatten().all(|&p| p == 0.0), "not at rest");
                assert_eq!(tv.validate(u, v), ValidationResult::Valid);
                assert_eq!(tv.weight(u, v), w);
            }
            assert!(valid > 100, "{valid} valid");
            if refusing {
                assert!(refused > 20, "{refused} refused");
            }
            // The centre is the untilted chart's, bit for bit.
            assert_eq!(
                bits(&tv.decode::<Defaults, f64>(C[0], C[1])),
                bits(&simplex(nu0).decode::<Defaults, f64>(C[0], C[1]))
            );
        }
    }
}

#[test]
fn qa_tilt_every_chart_nu0() {
    check_nu0(false);
}

negative_control!(
    qa_tilt_every_chart_nu0,
    "the triangle read at ν₀ without the tilt's d must fail",
    expected = "is not the Burrau one",
    {
        check_nu0(true);
    }
);

// ── A rest start's momentum controls from 0 ──────────────────────────────────────────────────────────────────────────

fn check_rest<Ch>(name: &str, own: Ch, wrong: bool)
where
    Ch: kernel::chart::construction::RestChart<Param = f64> + Chart<Param = f64> + Copy,
    RestStart<Ch, f64>: Tiltable<Param = f64>,
{
    let span = 1.0;
    for x in 2u32..6 {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            let tv = view(RestStart::new(own), arm, x, span);
            let w = tv.tilt().weight;
            let mut moving = 0;
            for [u, v] in grid(33) {
                let (a, b, d) = arm.place(u, v, span);
                let got = tv.decode::<Defaults, f64>(u, v);
                let at = own.decode::<Defaults, f64>(a, b);
                for i in 0..3 {
                    assert!((got.ic.m[i] - at.ic.m[i]).abs() < 1e-15, "{name}: masses");
                    assert!(
                        dist(got.ic.r[i], at.ic.r[i]) < TOL,
                        "{name}: positions at ({u}, {v})"
                    );
                }
                let mut z = [0.0; 4];
                let slot = (x - 2) as usize + usize::from(wrong);
                z[slot % 4] = d;
                let want =
                    momentum::decode_at::<Defaults, f64>(z, &got.masses, &LinkParams::DEFAULT);
                for i in 0..3 {
                    assert!(
                        dist(got.ic.p[i], want.p[i]) < TOL,
                        "{name} control {x} at ({u}, {v}): momenta {:?} are not D_mom of the control at d = {d}: {:?}",
                        got.ic.p,
                        want.p
                    );
                }
                let p = total_p(&got);
                assert!(p[0].abs() < TOL && p[1].abs() < TOL, "{name}: Σp = {p:?}");
                assert_eq!(
                    tv.validate(u, v),
                    own.validate(a, b),
                    "{name} at ({u}, {v})"
                );
                assert_eq!(
                    tv.weight(u, v),
                    w,
                    "{name}: a rest-start cell is never refused"
                );
                moving += usize::from(got.ic.p.iter().flatten().any(|&p| p != 0.0));
            }
            assert!(moving > 900, "{name} control {x}: {moving} pixels moving");
            assert_eq!(
                bits(&tv.decode::<Defaults, f64>(C[0], C[1])),
                bits(&own.decode::<Defaults, f64>(C[0], C[1])),
                "{name}: the centre moved"
            );
        }
    }
}

fn check_rest_all(wrong: bool) {
    check_rest("MassSimplex", simplex(0.5), wrong);
    check_rest("EuclidPlane", euclid(), wrong);
    check_rest("BurrauThetaDeltaM", delta_m(), wrong);
}

#[test]
fn qa_tilt_every_chart_rest_start() {
    check_rest_all(false);
}

negative_control!(
    qa_tilt_every_chart_rest_start,
    "the tilt's d expected on the next momentum control must fail",
    expected = "are not D_mom of the control",
    {
        check_rest_all(true);
    }
);

// ── REQ-CHART-065: slice ─────────────────────────────────────────────────────────────────────────────────────────────

/// A control of [`check_slice`].
#[derive(Clone, Copy, PartialEq)]
enum SliceCtl {
    None,
    /// The momenta's `K` expected unchanged by a slice of the held `K`.
    StaleK,
    /// The held `L_z` slid in the warp coordinate `s`, `L_z = (2s − 1)·L_max`.
    WarpLz,
    /// The held `K` slid in `t`, not by value.
    KInT,
    /// The tilt's `t₀` left at the unsliced `K₀`.
    StaleT0,
}

/// The held `L_z` slid by value, each sign, in each mode, the row's `K` the mode's (R-720: "the same rule governs slicing
/// the held L_z as well as tilting it"): warped, past the untilted band as well; linear, inside it.
fn slice_lz<C: LzChart<Param = f64> + PartialEq + std::fmt::Debug>(
    name: &str,
    chart: C,
    [k_max, gamma]: [f64; 2],
    ctl: SliceCtl,
) {
    let line = k_max * C[1].powf(gamma);
    for (mapping, values) in [
        (Mapping::Warped, [0.3, -0.2, 1.5, -1.5]),
        (Mapping::Linear, [0.3, -0.2, 0.9, -0.9]),
    ] {
        let held = HeldLz {
            mapping,
            ..HeldLz::new(chart)
        };
        for value in values {
            let s = held.sliced(HELD_LZ, value).expect("L_z is sliceable");
            assert_eq!(
                (s.chart, s.mapping, s.held(HELD_LZ)),
                (chart, mapping, Some(value)),
                "{name}: a slice moved more than the held L_z"
            );
            let k = if mapping == Mapping::Warped {
                0.5 * value * value + line
            } else {
                k_max * C[1]
            };
            let d = if ctl == SliceCtl::WarpLz && mapping == Mapping::Warped {
                let moved = 2.0 * value * (2.0 * line).sqrt();
                chart.decode_lz(C[0], moved, above_floor(moved) + line)
            } else {
                s.decode::<Defaults, f64>(C[0], C[1])
            };
            assert_eq!(d.degenerate, None, "{name}: L_z = {value}");
            assert_eq!(s.validate(C[0], C[1]), ValidationResult::Valid, "{name}");
            let tol = TOL * (1.0 + k);
            assert!(
                (lz_of(&d) - value).abs() < tol,
                "{name}: the sliced L_z: L_z(p) = {}, expected {value}",
                lz_of(&d)
            );
            assert!(
                (k_of(&d) - k).abs() < tol,
                "{name}: K(p) = {} is not the row's {k} at L_z = {value}",
                k_of(&d)
            );
        }
    }
}

/// The held `K` slid by value on [`HeldK`], and the tilt's `t₀` recomputed from the new `K₀` (R-720's reading of R-633).
fn slice_k(ctl: SliceCtl) {
    let c = theta_lz(0.5);
    let h = HeldK::new(c, 1.0, 1.5);
    let s = h.sliced(HELD_K, 0.8).expect("K is sliceable");
    assert_eq!(
        s,
        HeldK {
            chart: BurrauThetaLz { k: 0.8, ..c },
            ..h
        }
    );
    assert_eq!(s.held(HELD_K), Some(0.8));
    let d = if ctl == SliceCtl::KInT {
        // K₀ = 0.5 moved by d = 0.3 in t, not in K.
        let k = (0.5f64.powf(1.0 / 1.5) + 0.3).powf(1.5);
        let lz = (2.0 * C[1] - 1.0) * (2.0 * k).sqrt();
        decode_theta_lz_at(&c, C[0], lz, k).0
    } else {
        s.decode::<Defaults, f64>(C[0], C[1])
    };
    assert!(
        (k_of(&d) - 0.8).abs() < TOL,
        "the sliced K: K(p) = {}, expected K₀ + d = 0.8",
        k_of(&d)
    );
    let span = 1.0;
    let tv = view(
        if ctl == SliceCtl::StaleT0 { h } else { s },
        Arm::SheetT1,
        HELD_K,
        span,
    );
    let t0 = 0.8f64.powf(1.0 / 1.5);
    for [u, v] in vertices() {
        let (_, _, d) = Arm::SheetT1.place(u, v, span);
        let k = (t0 + d).max(0.0).powf(1.5);
        let got = k_of(&tv.decode::<Defaults, f64>(u, v));
        assert!(
            (got - k).abs() < TOL * (1.0 + k),
            "t₀ is not recomputed from the sliced K₀: K(p) = {got}, expected {k}"
        );
    }
}

fn check_slice(ctl: SliceCtl) {
    // (ν, K) and Φ_θ,K: the held L_z.
    slice_lz("(ν, K)", nu_k(), [1.0, 1.5], ctl);
    slice_lz("Φ_θ,K", theta_k(), [1.0, 1.5], ctl);
    let a = HeldLz::new(nu_k());
    // Φ_θ,L_z: the held K, its band following.
    let c = theta_lz(0.5);
    let s = c.sliced(HELD_K, 0.8).expect("K is sliceable");
    assert_eq!(
        (s.theta_range, s.lz_fraction, s.held(HELD_K)),
        (c.theta_range, c.lz_fraction, Some(0.8))
    );
    let d = s.decode::<Defaults, f64>(C[0], C[1]);
    let want_k = if ctl == SliceCtl::StaleK { 0.5 } else { 0.8 };
    assert!(
        (k_of(&d) - want_k).abs() < TOL,
        "the sliced K: K(p) = {}, expected {want_k}",
        k_of(&d)
    );
    assert!((lz_of(&d) - (2.0 * C[1] - 1.0) * 1.6f64.sqrt()).abs() < TOL);
    slice_k(ctl);
    // The mass simplex's ν₀: the triangle at the sliced value, the controls unmoved.
    let m = RestStart::new(simplex(0.5));
    let s = m.sliced(NU0, 0.3).expect("ν₀ is sliceable");
    assert_eq!((s.chart.nu0, s.chart.eps_m, s.z_mom), (0.3, 1e-4, [0.0; 4]));
    assert_eq!(
        bits(&s.decode::<Defaults, f64>(C[0], C[1])),
        bits(&simplex(0.3).decode::<Defaults, f64>(C[0], C[1]))
    );
    // A momentum control on each rest start: only that one moves; the centre's momenta are D_mom of it.
    fn rest<Ch>(r: RestStart<Ch, f64>)
    where
        Ch: kernel::chart::construction::RestChart<Param = f64>
            + Held<f64>
            + PartialEq
            + std::fmt::Debug,
    {
        for k in 2u32..6 {
            let s = r.sliced(k, 0.4).expect("a momentum control is sliceable");
            assert_eq!(s.chart, r.chart);
            for j in 2u32..6 {
                assert_eq!(s.held(j), Some(if j == k { 0.4 } else { 0.0 }));
            }
            let d = s.decode::<Defaults, f64>(C[0], C[1]);
            let mut z = [0.0; 4];
            z[(k - 2) as usize] = 0.4;
            let want = momentum::decode_at::<Defaults, f64>(z, &d.masses, &LinkParams::DEFAULT);
            // Equal as values: the rest start's `+0` added to `D_mom` turns its one `−0` into `+0`.
            assert_eq!(
                d.ic.p, want.p,
                "the sliced control's momenta are not D_mom of it"
            );
        }
    }
    rest(m);
    rest(RestStart::new(euclid()));
    rest(RestStart::new(delta_m()));
    // No other index is offered.
    for k in (0..16).filter(|k| !(2..6).contains(k) && *k != NU0) {
        assert!(m.sliced(k, 0.4).is_none(), "the simplex sliced {k}");
    }
    for k in (0..16).filter(|k| *k != HELD_LZ) {
        assert!(a.sliced(k, 0.4).is_none(), "(ν, K) sliced {k}");
    }
    for k in (0..16).filter(|k| *k != HELD_K) {
        assert!(c.sliced(k, 0.4).is_none(), "Φ_θ,L_z sliced {k}");
        assert!(
            HeldK::new(c, 1.0, 1.5).sliced(k, 0.4).is_none(),
            "HeldK sliced {k}"
        );
    }
}

/// How [`check_slice_edge`] reads the edge.
#[derive(Clone, Copy, PartialEq)]
enum Edge {
    AsRuled,
    /// The centres past the edge expected to validate.
    PastAsValid,
    /// The warped held `L_z` past the band expected refused, the reading R-720 replaces.
    WarpedRefused,
}

/// REQ-CHART-065's "subject to REQ-CHART-060's stop": REQ-CHART-060 stops a slice where the centre, `Φ` at the slice
/// centre's hidden coordinates, fails `validate`. So a physical slice past the construction's edge must leave a centre
/// that fails it, and one inside must pass: `ν₀ ∉ (0, 1)`; a held `K < 0`, below every `L_z(v)²/2`, the stop at
/// `K₀ = 0` in either mode; in linear mode a held `L_z` with `K(v) < L_z²/2`. In warped mode the held `L_z` has no edge:
/// the floor keeps every pixel a system at any `L_z` (R-720).
fn check_slice_edge(edge: Edge) {
    let ok = |r: ValidationResult| r == ValidationResult::Valid;
    let refused = |r: ValidationResult| matches!(r, ValidationResult::Reject { .. });
    let past = |r: ValidationResult, what: &str| {
        let want = if edge == Edge::PastAsValid {
            ok(r)
        } else {
            refused(r)
        };
        assert!(
            want,
            "a slice past the edge ({what}) leaves a centre that validates as {r:?}"
        );
    };
    let m = RestStart::new(simplex(0.5));
    for nu in [0.05, 0.3, 0.95] {
        assert!(
            ok(m.sliced(NU0, nu).unwrap().validate(C[0], C[1])),
            "ν₀ = {nu}"
        );
    }
    for nu in [0.0, 1.0, 1.2, -0.1] {
        past(m.sliced(NU0, nu).unwrap().validate(C[0], C[1]), "ν₀");
    }
    // Warped: no edge in L_z, at either sign, at 0.5, 1.1, 10 and 1000 times the untilted band's L_max(K_max).
    let (a, t) = (HeldLz::new(nu_k()), HeldLz::new(theta_k()));
    let band = 2f64.sqrt();
    for lz in [0.5, 1.1, 10.0, 1e3]
        .into_iter()
        .flat_map(|f| [f * band, -f * band])
    {
        for r in [
            a.sliced(HELD_LZ, lz).unwrap().validate(C[0], C[1]),
            t.sliced(HELD_LZ, lz).unwrap().validate(C[0], C[1]),
        ] {
            let want = if edge == Edge::WarpedRefused {
                refused(r)
            } else {
                ok(r)
            };
            assert!(
                want,
                "a warped slice of the held L_z = {lz} validates as {r:?}"
            );
        }
    }
    // Linear: the row K(v) = K_max·v is fixed, so L_z has its edge there.
    let (al, tl) = (
        HeldLz {
            mapping: Mapping::Linear,
            ..a
        },
        HeldLz {
            mapping: Mapping::Linear,
            ..t
        },
    );
    let lin_edge = (2.0 * nu_k().k_max * C[1]).sqrt();
    for (name, r) in [
        (
            "L_z",
            al.sliced(HELD_LZ, 1.1 * lin_edge)
                .unwrap()
                .validate(C[0], C[1]),
        ),
        (
            "−L_z",
            al.sliced(HELD_LZ, -1.1 * lin_edge)
                .unwrap()
                .validate(C[0], C[1]),
        ),
        (
            "Φ_θ,K's L_z",
            tl.sliced(HELD_LZ, 1.1 * lin_edge)
                .unwrap()
                .validate(C[0], C[1]),
        ),
        (
            "Φ_θ,K's −L_z",
            tl.sliced(HELD_LZ, -1.1 * lin_edge)
                .unwrap()
                .validate(C[0], C[1]),
        ),
    ] {
        past(r, name);
    }
    for lz in [0.9 * lin_edge, -0.9 * lin_edge] {
        assert!(ok(al.sliced(HELD_LZ, lz).unwrap().validate(C[0], C[1])));
        assert!(ok(tl.sliced(HELD_LZ, lz).unwrap().validate(C[0], C[1])));
    }
    // The held K, bare and held, in either mode: a stop at K₀ = 0, itself a system, the rest start.
    let c = theta_lz(0.5);
    let h = HeldK::new(c, 1.0, 1.5);
    let hl = HeldK {
        mapping: Mapping::Linear,
        ..h
    };
    assert!(ok(c.sliced(HELD_K, 0.05).unwrap().validate(C[0], C[1])));
    past(c.sliced(HELD_K, -0.5).unwrap().validate(C[0], C[1]), "K");
    for held in [h, hl] {
        assert!(ok(held.sliced(HELD_K, 0.05).unwrap().validate(C[0], C[1])));
        let zero = held.sliced(HELD_K, 0.0).unwrap();
        assert!(ok(zero.validate(C[0], C[1])), "K₀ = 0 is the stop itself");
        let d = zero.decode::<Defaults, f64>(C[0], C[1]);
        assert_eq!(d.degenerate, None, "K₀ = 0");
        assert!(d.ic.p.iter().flatten().all(|&p| p == 0.0), "K₀ = 0 at rest");
        past(
            held.sliced(HELD_K, -0.1).unwrap().validate(C[0], C[1]),
            "HeldK's K",
        );
    }
}

#[test]
fn qa_slice_every_chart_construction_edge() {
    check_slice_edge(Edge::AsRuled);
}

negative_control!(
    qa_slice_every_chart_construction_edge,
    "a centre past the edge expected to validate must fail",
    expected = "a slice past the edge",
    {
        check_slice_edge(Edge::PastAsValid);
    }
);

mod warped_refused {
    use super::*;
    negative_control!(
        qa_slice_every_chart_construction_edge,
        "a warped held L_z past the band expected refused must fail: the floor keeps it a system",
        expected = "a warped slice of the held L_z",
        {
            check_slice_edge(Edge::WarpedRefused);
        }
    );
}

#[test]
fn qa_slice_every_chart_construction() {
    check_slice(SliceCtl::None);
}

negative_control!(
    qa_slice_every_chart_construction,
    "a K slice expected to leave the momenta's K unchanged must fail",
    expected = "the sliced K",
    {
        check_slice(SliceCtl::StaleK);
    }
);

mod slice_warp_lz {
    use super::*;
    negative_control!(
        qa_slice_every_chart_construction,
        "a slice of the held L_z moved in a warp coordinate must fail",
        expected = "the sliced L_z",
        {
            check_slice(SliceCtl::WarpLz);
        }
    );
}

mod slice_k_in_t {
    use super::*;
    negative_control!(
        qa_slice_every_chart_construction,
        "a slice of the held K moved in t must fail",
        expected = "the sliced K",
        {
            check_slice(SliceCtl::KInT);
        }
    );
}

mod slice_stale_t0 {
    use super::*;
    negative_control!(
        qa_slice_every_chart_construction,
        "a tilt whose t₀ is not recomputed from the sliced K₀ must fail",
        expected = "t₀ is not recomputed",
        {
            check_slice(SliceCtl::StaleT0);
        }
    );
}

// ── The fix round: the shifted energy's L_z bound, a finite 2K*, the chart's own precision ───────────────────────────

/// `Φ_θ,L_z` over a window past the band, `v ∈ [−0.4, 1.4]`, tilted into the held `K`: by value (`bare`, or `HeldK` in
/// linear mode) a cell is a system exactly where `2(K₀ + d) ≥ L_z(b)²`, `L_z(b) = (2b − 1)·√(2K₀)` the centre's band
/// (§5.4: "past `L_z`'s the formula continues and `L_z² > 2IK` is not a system"). `centre` reads the chart's own check at
/// `K₀` in the view's place (the control).
fn check_shifted_bound<Ch: Tiltable<Param = f64> + Copy>(
    name: &str,
    chart: Ch,
    k0: f64,
    centre: bool,
) {
    let span = 4.0;
    let tv = view(chart, Arm::SheetT1, HELD_K, span);
    let w = tv.tilt().weight;
    let (mut past_valid, mut past_refused) = (0, 0);
    for i in 0..33 {
        for j in 0..33 {
            let (u, v) = (f64::from(i) / 32.0, -0.4 + 1.8 * f64::from(j) / 32.0);
            let (a, b, d) = Arm::SheetT1.place(u, v, span);
            let lz = (2.0 * b - 1.0) * (2.0 * k0).sqrt();
            let k = k0 + d;
            let gap = 2.0 * k - lz * lz;
            if gap.abs() < 1e-9 {
                continue; // On §2.2's boundary.
            }
            let result = if centre {
                chart.validate(a, b)
            } else {
                tv.validate(u, v)
            };
            let past = !(0.0..=1.0).contains(&b);
            if gap > 0.0 {
                past_valid += usize::from(past);
                assert_eq!(
                    result,
                    ValidationResult::Valid,
                    "{name} at ({u}, {v}): K + d = {k} reaches L_z = {lz}, yet the cell is no system"
                );
                assert_eq!(tv.weight(u, v), w, "{name} at ({u}, {v})");
                let got = tv.decode::<Defaults, f64>(u, v);
                assert_eq!(got.degenerate, None, "{name} at ({u}, {v})");
                assert!(
                    (k_of(&got) - k).abs() < TOL && (lz_of(&got) - lz).abs() < TOL,
                    "{name} at ({u}, {v}): (K, L_z)(p) = ({}, {}), expected ({k}, {lz})",
                    k_of(&got),
                    lz_of(&got)
                );
            } else {
                past_refused += usize::from(past);
                assert!(
                    centre || matches!(result, ValidationResult::Reject { .. }),
                    "{name} at ({u}, {v}): K + d = {k} short of L_z = {lz} validates as {result:?}"
                );
                assert_eq!(tv.weight(u, v), 0.0, "{name} at ({u}, {v})");
            }
        }
    }
    assert!(
        past_valid > 20 && past_refused > 20,
        "{name}: past the band, {past_valid} systems and {past_refused} refused"
    );
}

fn check_shifted_bound_all(centre: bool) {
    let k0 = 0.5;
    check_shifted_bound("Φ_θ,L_z", theta_lz(k0), k0, centre);
    let linear = HeldK {
        mapping: Mapping::Linear,
        ..HeldK::new(theta_lz(k0), 1.0, 1.5)
    };
    check_shifted_bound("Φ_θ,L_z, linear", linear, k0, centre);
    // Warped, the band is the pixel's own: |2b − 1| > 1 is refused at every K > 0.
    let warped = view(
        HeldK::new(theta_lz(k0), 1.0, 1.5),
        Arm::SheetT1,
        HELD_K,
        1.0,
    );
    for (u, v) in [(0.9, 1.2), (0.9, -0.2), (C[0] + 0.1, 1.05)] {
        assert!(
            matches!(warped.validate(u, v), ValidationResult::Reject { .. }),
            "warped past the band at ({u}, {v})"
        );
        assert_eq!(
            warped.weight(u, v),
            0.0,
            "warped past the band at ({u}, {v})"
        );
    }
}

#[test]
fn qa_tilt_every_chart_shifted_energy_bound() {
    check_shifted_bound_all(false);
}

negative_control!(
    qa_tilt_every_chart_shifted_energy_bound,
    "the L_z bound read at the centre's K₀ takes a cell K + d reaches for no system and must fail",
    expected = "yet the cell is no system",
    {
        check_shifted_bound_all(true);
    }
);

/// `no_construction` against the oracle `(k + k).is_finite()`, at `$t`, with `L_z = 0`, and [`infeasible`]'s own arm kept.
/// `$old` reads [`infeasible`] alone, the predicate before the fix (the control).
macro_rules! finite_2k {
    ($t:ty, $old:expr) => {{
        let test = |k: $t, l: $t| {
            if $old {
                infeasible(1.0, k, l)
            } else {
                no_construction(k, l)
            }
        };
        let last = <$t>::MAX * 0.5;
        let first = <$t>::from_bits(last.to_bits() + 1);
        for k in [first, <$t>::MAX, <$t>::INFINITY, <$t>::NAN] {
            assert!(
                test(k, 0.0),
                "a K* with no finite 2K* has a construction: {k}"
            );
        }
        for k in [0.0, 1.0, last] {
            assert!(!test(k, 0.0), "K* = {k} has a finite 2K*");
        }
        assert!(
            test(-1.0, 0.0) && test(1.0, 2.0) && !test(2.0, 2.0),
            "the exact test"
        );
        // Across every binade, the mantissa stepped.
        let bits = <$t>::MAX.to_bits();
        let mut x = 1;
        while x < 4096 {
            let k = <$t>::from_bits(bits / 4096 * x + bits / 8191);
            assert_eq!(test(k, 0.0), !(k + k).is_finite(), "at K* = {k}");
            x += 1;
        }
    }};
}

fn check_finite_2k(old: bool) {
    finite_2k!(f64, old);
    finite_2k!(f32, old);
}

#[test]
fn qa_no_construction_finite_2k() {
    check_finite_2k(false);
}

negative_control!(
    qa_no_construction_finite_2k,
    "the exact test alone, the predicate before the fix, gives a K* with no finite 2K* a construction",
    expected = "a K* with no finite 2K* has a construction",
    {
        check_finite_2k(true);
    }
);

/// The decode on each side of the two edges at f64: `K* ≥ 2^1023`, and `|L_z| ≥ 2^512`. `shift` puts the edge one float
/// low, so that the last system is expected labelled (the control).
fn check_decode_2k(shift: bool) {
    let last = f64::MAX * 0.5;
    let first = f64::from_bits(last.to_bits() + 1);
    let chart = nu_k();
    let no_system = |d: &Decoded<f64>, what: &str| {
        assert_eq!(
            d.degenerate,
            Some(Degenerate::InvariantInfeasible),
            "{what} is not labelled"
        );
        assert!(
            d.ic.p.iter().flatten().all(|&p| p == 0.0),
            "{what}: momenta {:?} are not 0",
            d.ic.p
        );
    };
    for k in [first, f64::MAX, f64::INFINITY, f64::NAN] {
        no_system(&chart.decode_lz(0.5, 0.0, k), "K* past the edge");
        no_system(&chart.decode_lz(0.5, 0.3, k), "K* past the edge, L_z = 0.3");
        no_system(&theta_k().decode_lz(0.5, 0.0, k), "Φ_θ,K, K* past the edge");
    }
    let d = chart.decode_lz(0.5, 0.0, last);
    if shift {
        no_system(&d, "the last K*");
    }
    assert_eq!(d.degenerate, None, "the last K* is a system");
    assert!(d.ic.p.iter().flatten().all(|p| p.is_finite()));
    assert!((k_of(&d) / last - 1.0).abs() < TOL, "K(p) = {}", k_of(&d));
    // `2^512`, the first |L_z| with no finite square, and the float below it.
    let square_edge = 2f64.powi(512);
    for l in [square_edge, -square_edge, 1e300] {
        no_system(
            &chart.decode_lz(0.5, l, f64::MAX * 0.25),
            "an L_z with no finite square",
        );
    }
    let l = f64::from_bits(square_edge.to_bits() - 1);
    let d = chart.decode_lz(0.5, l, last);
    assert_eq!(d.degenerate, None, "the last L_z is a system");
    assert!(
        (lz_of(&d) / l - 1.0).abs() < TOL,
        "the last L_z keeps its spin: L_z(p) = {}",
        lz_of(&d)
    );
}

#[test]
fn qa_decode_no_finite_2k() {
    check_decode_2k(false);
}

negative_control!(
    qa_decode_no_finite_2k,
    "the last K* with a finite 2K* expected labelled must fail: it is a system",
    expected = "the last K* is not labelled",
    {
        check_decode_2k(true);
    }
);

/// 2K overflows f32 at this K, not f64.
const F32_EDGE_K: f32 = 2.0e38;

fn narrow_tilt(arm: Arm, x: u32, span: f64) -> Tilt<f32> {
    let w = Tilt::new(&arm.spec(x), C, [span, span]);
    Tilt {
        on: w.on,
        two: w.two,
        rank_loss: w.rank_loss,
        centre: w.centre.map(|x| x as f32),
        columns: w.columns.map(|c| c.map(|x| x as f32)),
        x: w.x,
        y: w.y,
        span: w.span.map(|x| x as f32),
        weight: w.weight as f32,
        normal_zero: w.normal_zero,
        strip: w.strip,
    }
}

/// One pixel of an f32 view with a `K*` past f32's edge and inside f64's: refused, weight 0, the f32 decode labelled
/// with finite momenta, where f64 alone sees a system. `wide` reads the f64 predicate in the view's place (the control).
fn refused_at_f32<Ch: Tiltable<Param = f32>>(
    name: &str,
    tv: &Tilted<Ch, f32>,
    (u, v): (f64, f64),
    k: f64,
    wide: bool,
) {
    assert!(
        !no_construction(k, 0.0),
        "{name}: the fixture's K* = {k} must have a construction at f64"
    );
    let refused = if wide {
        no_construction(k, 0.0)
    } else {
        matches!(tv.validate(u, v), ValidationResult::Reject { .. })
    };
    assert!(
        refused,
        "{name} at ({u}, {v}): K* = {k} is not refused at the chart's own precision"
    );
    assert_eq!(tv.weight(u, v), 0.0, "{name} at ({u}, {v})");
    let d = tv.decode::<Defaults, f32>(u as f32, v as f32);
    assert_eq!(
        d.degenerate,
        Some(Degenerate::InvariantInfeasible),
        "{name} at ({u}, {v}): the f32 decode's label"
    );
    assert!(d.ic.p.iter().flatten().all(|p| p.is_finite()), "{name}");
}

fn check_own_precision(wide: bool) {
    let burrau = Burrau::<f32> {
        nu_range: [0.1, 0.9],
        k_max: F32_EDGE_K,
        gamma_k: 1.5,
    };
    let acute = BurrauThetaK::<f32> {
        theta_range: [0.2, std::f32::consts::FRAC_PI_4],
        k_max: F32_EDGE_K,
        gamma_k: 1.5,
    };
    let top = (0.5, 1.0);
    let edge = f64::from(F32_EDGE_K);
    // The held L_z's charts, the top row past f32's edge, in each mode; v = 1/8 a system.
    for mapping in [Mapping::Warped, Mapping::Linear] {
        let a = HeldLz {
            mapping,
            ..HeldLz::new(burrau)
        };
        let t = HeldLz {
            mapping,
            ..HeldLz::new(acute)
        };
        let (va, vt) = (
            Tilted::new(a, narrow_tilt(Arm::SheetT1, HELD_LZ, 1.0)).unwrap(),
            Tilted::new(t, narrow_tilt(Arm::SheetT1, HELD_LZ, 1.0)).unwrap(),
        );
        refused_at_f32("(ν, K)", &va, top, edge, wide);
        refused_at_f32("Φ_θ,K", &vt, top, edge, wide);
        assert_eq!(va.validate(0.5, 0.125), ValidationResult::Valid);
        assert_eq!(vt.validate(0.5, 0.125), ValidationResult::Valid);
    }
    // Φ_θ,L_z by value: K₀ + d past f32's edge at the window's far side, the centre a system.
    let bare = BurrauThetaLz::<f32> {
        theta_range: [0.2, std::f32::consts::FRAC_PI_4],
        k: 1.0e38,
        lz_fraction: 1.0,
    };
    let span = 3.0e38;
    let by_value = Tilted::new(bare, narrow_tilt(Arm::SheetT1, HELD_K, span)).unwrap();
    let (_, _, d) = Arm::SheetT1.place(1.0, 0.5, span);
    refused_at_f32("Φ_θ,L_z", &by_value, (1.0, 0.5), 1.0e38 + d, wide);
    assert_eq!(by_value.validate(C[0], C[1]), ValidationResult::Valid);
    // Warped: K_max·t^γ past f32's edge.
    let warped = Tilted::new(
        HeldK::new(bare, 3.0e38, 1.5),
        narrow_tilt(Arm::SheetT1, HELD_K, 1.0),
    )
    .unwrap();
    let (_, _, d) = Arm::SheetT1.place(1.0, 0.5, 1.0);
    let k = 3.0e38 * ((1.0e38f64 / 3.0e38).powf(1.0 / 1.5) + d).powf(1.5);
    refused_at_f32("Φ_θ,L_z, warped", &warped, (1.0, 0.5), k, wide);
    assert_eq!(warped.validate(C[0], C[1]), ValidationResult::Valid);
}

#[test]
fn qa_refusal_at_chart_precision() {
    check_own_precision(false);
}

negative_control!(
    qa_refusal_at_chart_precision,
    "a K* read by the f64 predicate alone has a construction, so a refusal at f64 misses it and must fail",
    expected = "is not refused at the chart's own precision",
    {
        check_own_precision(true);
    }
);

/// Linear f32 `(ν, K)` at the same f32 target: walking `b` float by float across the floor at each offset, the f32
/// decode labels a target exactly where the chart's refusal, given that target widened, refuses it (the fix's "a target
/// that has no construction only there is refused"). `inverted` expects the opposite (the control).
fn check_target_refused(inverted: bool) {
    let chart = HeldLz {
        mapping: Mapping::Linear,
        ..HeldLz::new(Burrau::<f32> {
            nu_range: [0.1, 0.9],
            k_max: 1.0,
            gamma_k: 1.5,
        })
    };
    let params = LinkParams::<f32>::DEFAULT;
    let shift = |dx: f32| kernel::chart::tilt::Shift {
        x: HELD_LZ,
        dx,
        y: HELD_LZ,
        dy: 0.0f32,
        two: false,
    };
    let wide = |dx: f64| kernel::chart::tilt::Shift {
        x: HELD_LZ,
        dx,
        y: HELD_LZ,
        dy: 0.0f64,
        two: false,
    };
    let (mut labelled, mut systems) = (0, 0);
    for i in 1..200 {
        let dx = i as f32 * 0.00613;
        let floor = (0.5 * f64::from(dx) * f64::from(dx)) as f32;
        for s in 0..48u32 {
            let b = f32::from_bits(floor.to_bits() + s - 24);
            let got = chart
                .decode_shifted::<Defaults, f32>(0.5, b, &shift(dx), &params)
                .degenerate
                .is_some();
            let refused = chart
                .refusal(0.5, f64::from(b), &wide(f64::from(dx)))
                .is_some();
            assert_eq!(
                refused,
                got != inverted,
                "at b = {b}, d = {dx}: the f32 decode's label and the chart's refusal disagree"
            );
            labelled += usize::from(got);
            systems += usize::from(!got);
        }
    }
    assert!(
        labelled > 500 && systems > 500,
        "{labelled} labelled, {systems} systems"
    );
}

#[test]
fn qa_refusal_at_chart_precision_target() {
    check_target_refused(false);
}

negative_control!(
    qa_refusal_at_chart_precision_target,
    "the label and the refusal expected opposite must fail",
    expected = "the f32 decode's label and the chart's refusal disagree",
    {
        check_target_refused(true);
    }
);

/// One f32 view, along each column: the rows are bisected to where the f32 decode's label flips and walked float by
/// float about it, and every pixel it labels is refused and weighted 0, the check on the view's own placement at f32.
/// The placement at f32 and the f64 one narrowed differ there, so `wide` reads the chart's refusal of the f64 placement
/// in the view's place (the control). The columns that crossed.
fn walk_floor<Ch: Tiltable<Param = f32>>(name: &str, tv: &Tilted<Ch, f32>, wide: bool) -> usize {
    let tagged = |u: f32, v: f32| tv.decode::<Defaults, f32>(u, v).degenerate.is_some();
    let mut crossed = 0;
    for i in 0..257 {
        let u = i as f32 / 256.0;
        let (mut lo, mut hi) = (0.0f32, 0.5f32);
        if !tagged(u, lo) || tagged(u, hi) {
            continue;
        }
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if tagged(u, mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        crossed += 1;
        for s in 0..48u32 {
            let v = f32::from_bits(lo.to_bits() + s - 24);
            if !tagged(u, v) {
                continue;
            }
            let (wu, wv) = (f64::from(u), f64::from(v));
            let refused = if wide {
                let ([a, b], shift) = tv.at::<f64>(wu, wv);
                tv.chart().refusal(a, b, &shift.unwrap()).is_some()
            } else {
                matches!(tv.validate(wu, wv), ValidationResult::Reject { .. })
            };
            assert!(
                refused,
                "{name}: a pixel the f32 decode labels is a system at ({u}, {v})"
            );
            if !wide {
                assert_eq!(tv.weight(wu, wv), 0.0, "{name} at ({u}, {v})");
            }
        }
    }
    crossed
}

fn check_view_refused(wide: bool) {
    let burrau = Burrau::<f32> {
        nu_range: [0.1, 0.9],
        k_max: 1.0,
        gamma_k: 1.5,
    };
    let acute = BurrauThetaK::<f32> {
        theta_range: [0.2, std::f32::consts::FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 1.5,
    };
    let bare = BurrauThetaLz::<f32> {
        theta_range: [0.2, std::f32::consts::FRAC_PI_4],
        k: 0.5,
        lz_fraction: 1.0,
    };
    let a = HeldLz {
        mapping: Mapping::Linear,
        ..HeldLz::new(burrau)
    };
    let t = HeldLz {
        mapping: Mapping::Linear,
        ..HeldLz::new(acute)
    };
    let linear_k = HeldK {
        mapping: Mapping::Linear,
        ..HeldK::new(bare, 1.0, 1.5)
    };
    let tilt_lz = narrow_tilt(Arm::SheetT1, HELD_LZ, 2.0);
    let tilt_k = narrow_tilt(Arm::SheetT1, HELD_K, 1.0);
    let crossed = [
        walk_floor("(ν, K)", &Tilted::new(a, tilt_lz).unwrap(), wide),
        walk_floor("Φ_θ,K", &Tilted::new(t, tilt_lz).unwrap(), wide),
        walk_floor("Φ_θ,L_z", &Tilted::new(bare, tilt_k).unwrap(), wide),
        walk_floor(
            "Φ_θ,L_z, linear",
            &Tilted::new(linear_k, tilt_k).unwrap(),
            wide,
        ),
    ];
    assert!(
        crossed.iter().all(|&c| c > 50),
        "columns that crossed the floor: {crossed:?}"
    );
}

#[test]
fn qa_refusal_at_chart_precision_view() {
    check_view_refused(false);
}

negative_control!(
    qa_refusal_at_chart_precision_view,
    "the chart's refusal of the f64 placement misses a pixel the f32 decode labels, so it must fail",
    expected = "a pixel the f32 decode labels is a system",
    {
        check_view_refused(true);
    }
);

/// The f32 tilt of `spec`, about `C`, at the span `span`, as a chart at f32 holds it.
fn f32_tilt(spec: &TiltSpec, span: f64) -> Tilt<f32> {
    let w = Tilt::new(spec, C, [span, span]);
    Tilt {
        on: w.on,
        two: w.two,
        rank_loss: w.rank_loss,
        centre: w.centre.map(|x| x as f32),
        columns: w.columns.map(|c| c.map(|x| x as f32)),
        x: w.x,
        y: w.y,
        span: w.span.map(|x| x as f32),
        weight: w.weight as f32,
        normal_zero: w.normal_zero,
        strip: w.strip,
    }
}

/// One domain end of one chart.
struct End<'a> {
    name: &'a str,
    /// The pixel axis walked, 0 for `u`, 1 for `v`.
    axis: usize,
    /// The placed value of that axis at the end, at f64.
    at: f64,
    /// A pixel on the other axis.
    other: f32,
    /// Whether the fixture's angles reach a pixel outside at f32 alone. A sum at the simplex's upper ends rounds to 1, on
    /// the domain, before it can pass it, so a pixel past them is past them at f64 too.
    rounds: bool,
    /// Whether `(a, b)` placed at f32 is outside the domain.
    outside: &'a dyn Fn(f32, f32) -> bool,
}

/// `end` through `tv`, a tilt `cos` of the axis: bisected at f32 to where the placement leaves the domain, then walked
/// float by float. `wide` reads the check at the f64 placement in the view's place (the control). The pixels outside at
/// f32, and those of them the check at the f64 placement passes.
fn walk_end<Ch: Tiltable<Param = f32>>(
    tv: &Tilted<Ch, f32>,
    end: &End,
    cos: f64,
    wide: bool,
) -> [usize; 2] {
    let c = C[end.axis];
    let edge = c + (end.at - c) / cos;
    let pixel = |x: f32| {
        if end.axis == 0 {
            (x, end.other)
        } else {
            (end.other, x)
        }
    };
    let out = |x: f32| {
        let (u, v) = pixel(x);
        let ([a, b], _) = tv.at::<f32>(u, v);
        (end.outside)(a, b)
    };
    let side = if end.at > c { 0.2 } else { -0.2 };
    let (mut lo, mut hi) = ((edge - side) as f32, (edge + side) as f32);
    assert!(!out(lo) && out(hi), "{}: the walk's start", end.name);
    for _ in 0..48 {
        let mid = 0.5 * (lo + hi);
        if out(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let params = LinkParams::<f64>::DEFAULT;
    let ulp = f32::EPSILON * hi.abs().max(1e-3) * 0.5;
    let mut tally = [0, 0];
    for k in -32..32 {
        let x = hi + k as f32 * ulp;
        let (u, v) = pixel(x);
        let (wu, wv) = (f64::from(u), f64::from(v));
        let ([a, b], shift) = tv.at::<f64>(wu, wv);
        let at_f64 = tv.chart().validate_shifted(a, b, &shift.unwrap(), &params);
        let passes = at_f64 == ValidationResult::Valid;
        if out(x) {
            tally[0] += 1;
            tally[1] += usize::from(passes);
            let refused = if wide {
                !passes
            } else {
                matches!(tv.validate(wu, wv), ValidationResult::Reject { .. })
            };
            assert!(
                refused,
                "{}: a pixel placed outside at f32 is a system at ({u}, {v})",
                end.name
            );
        } else if passes
            && tv
                .decode::<Defaults, f32>(u, v)
                .ic
                .m
                .iter()
                .all(|&m| m > 0.0)
        {
            // Near θ = 0 a mass can round to 0 at f32 inside the fold: layer 3's to refuse.
            assert_eq!(
                tv.validate(wu, wv),
                ValidationResult::Valid,
                "{}: a pixel inside at both is refused at ({u}, {v})",
                end.name
            );
        }
    }
    tally
}

/// Every end of `ends` under each of `angles`, all into the hidden target `x`, on `map`.
fn walk_ends<Ch: Tiltable<Param = f32> + Copy>(
    chart: Ch,
    x: u32,
    span: f64,
    ends: &[End],
    wide: bool,
) {
    for end in ends {
        let mut tally = [0, 0];
        for i in 0..64 {
            let tau = 17.0 + 1.0625 * f64::from(i);
            let spec = TiltSpec {
                tau1: if end.axis == 0 { tau } else { 0.0 },
                tau2: if end.axis == 0 { 0.0 } else { tau },
                map: if end.axis == 0 {
                    TiltMap::Sheet
                } else {
                    TiltMap::Lean
                },
                x,
                y: x,
            };
            let tv = Tilted::new(chart, f32_tilt(&spec, span)).expect("a hidden target");
            let got = walk_end(&tv, end, tau.to_radians().cos(), wide);
            tally = [tally[0] + got[0], tally[1] + got[1]];
        }
        assert!(
            tally[0] > 100 && (tally[1] > 0 || !end.rounds),
            "{} at {}: outside at f32 {}, of them passing at the f64 placement {}",
            end.name,
            end.at,
            tally[0],
            tally[1]
        );
    }
}

fn check_placed_ends(wide: bool) {
    // θ inside the fold, read at f32 and on the placed value exact at f64: f32's π/4 lies above the fold's end.
    let fold = std::f64::consts::FRAC_PI_4;
    let theta_ok = |th: f32, wide: f64| th > 0.0 && wide > 0.0 && th <= fold as f32 && wide <= fold;
    // The ends of θ on the range [0.2, 0.7]: θ = 0 and θ = π/4.
    let (low, high) = (-0.4, (std::f64::consts::FRAC_PI_4 - 0.2) / 0.5);

    let burrau = Burrau::<f32> {
        nu_range: [0.25, 0.75],
        k_max: 1.0,
        gamma_k: 1.0,
    };
    let nu_out = |a: f32, _: f32| {
        let nu = burrau.nu::<f32>(a);
        !(nu > 0.0 && nu < 1.0)
    };
    let below = |_: f32, b: f32| b < 0.0;
    let held = |at, axis, outside| End {
        name: "(ν, K)",
        axis,
        at,
        other: 0.5,
        rounds: true,
        outside,
    };
    walk_ends(
        HeldLz::new(burrau),
        HELD_LZ,
        1.0,
        &[
            held(1.5, 0, &nu_out),
            held(-0.5, 0, &nu_out),
            held(0.0, 1, &below),
        ],
        wide,
    );

    let acute = BurrauThetaK::<f32> {
        theta_range: [0.2, 0.7],
        k_max: 1.0,
        gamma_k: 1.0,
    };
    let acute_out = |a: f32, _: f32| !theta_ok(acute.theta::<f32>(a), acute.theta(f64::from(a)));
    let held = |at, axis, outside| End {
        name: "Φ_θ,K",
        axis,
        at,
        other: 0.5,
        rounds: true,
        outside,
    };
    walk_ends(
        HeldLz::new(acute),
        HELD_LZ,
        1.0,
        &[
            held(low, 0, &acute_out),
            held(high, 0, &acute_out),
            held(0.0, 1, &below),
        ],
        wide,
    );

    let bare = BurrauThetaLz::<f32> {
        theta_range: [0.2, 0.7],
        k: 0.5,
        lz_fraction: 1.0,
    };
    let bare_out = |a: f32, _: f32| !theta_ok(bare.theta::<f32>(a), bare.theta(f64::from(a)));
    let lz_end = |name, at| End {
        name,
        axis: 0,
        at,
        other: 0.5,
        rounds: true,
        outside: &bare_out,
    };
    let bare_ends = [lz_end("Φ_θ,L_z", low), lz_end("Φ_θ,L_z", high)];
    walk_ends(bare, HELD_K, 0.1, &bare_ends, wide);
    let warped = HeldK::new(bare, 1.0, 1.5);
    let warped_ends = [lz_end("HeldK", low), lz_end("HeldK", high)];
    walk_ends(warped, HELD_K, 0.1, &warped_ends, wide);
    let linear = HeldK {
        mapping: Mapping::Linear,
        ..warped
    };
    let linear_ends = [lz_end("HeldK, linear", low), lz_end("HeldK, linear", high)];
    walk_ends(linear, HELD_K, 0.1, &linear_ends, wide);

    let square = |a: f32, b: f32| !((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b));
    let simplex_end = |axis, at, rounds| End {
        name: "the mass simplex",
        axis,
        at,
        other: 0.3,
        rounds,
        outside: &square,
    };
    walk_ends(
        RestStart::new(MassSimplex::<f32> {
            nu0: 0.5,
            eps_m: 1e-4,
        }),
        2,
        1.0,
        &[
            simplex_end(0, 0.0, true),
            simplex_end(0, 1.0, false),
            simplex_end(1, 0.0, true),
            simplex_end(1, 1.0, false),
        ],
        wide,
    );

    let plane = EuclidPlane::<f32> {
        m_range: [1.0, 32.0],
        nu_range: [0.25, 0.75],
    };
    let plane_out = |_: f32, b: f32| {
        let nu = plane.nu::<f32>(b);
        !(nu > 0.0 && nu < 1.0)
    };
    let plane_end = |at| End {
        name: "the Euclid plane",
        axis: 1,
        at,
        other: 0.5,
        rounds: true,
        outside: &plane_out,
    };
    walk_ends(
        RestStart::new(plane),
        2,
        1.0,
        &[plane_end(1.5), plane_end(-0.5)],
        wide,
    );

    let delta = BurrauThetaDeltaM::<f32> {
        theta_range: [0.2, 0.7],
        m_target: [0.5, 0.5, 0.0],
    };
    let delta_theta = |a: f32, _: f32| !theta_ok(delta.theta::<f32>(a), delta.theta(f64::from(a)));
    let delta_mass = |a: f32, b: f32| delta.masses::<f32>(a, b).iter().any(|&m| m <= 0.0);
    let delta_end = |axis, at, outside| End {
        name: "the δm strip",
        axis,
        at,
        other: 0.5,
        rounds: true,
        outside,
    };
    walk_ends(
        RestStart::new(delta),
        2,
        1.0,
        &[
            delta_end(0, low, &delta_theta),
            delta_end(0, high, &delta_theta),
            delta_end(1, 1.0, &delta_mass),
        ],
        wide,
    );
}

#[test]
fn qa_refusal_at_placed_domain_end() {
    check_placed_ends(false);
}

negative_control!(
    qa_refusal_at_placed_domain_end,
    "the check at the f64 placement passes a pixel the f32 placement puts outside the domain, so it must fail",
    expected = "a pixel placed outside at f32 is a system",
    {
        check_placed_ends(true);
    }
);

// ── The view's parameters, and what the weight reads ─────────────────────────────────────────────────────────────────

/// `x` at f32, rounded, a chart's own precision.
fn at_f32(x: f64) -> f32 {
    x as f32
}

/// `x` at f64.
fn at_f64(x: f64) -> f64 {
    x
}

/// The tilt of `spec` at the span `span` at f64, as [`f32_tilt`] holds it at f32.
fn f64_tilt(spec: &TiltSpec, span: f64) -> Tilt<f64> {
    Tilt::new(spec, C, [span, span])
}

/// Layer 3 from §4.5: positive masses, finite positions and momenta, and a construction.
fn is_system<R: Float + PayloadReal>(d: &Decoded<R>) -> bool {
    d.ic.m.iter().all(|&m| m > R::zero())
        && d.ic.r.iter().flatten().all(|x| x.is_finite())
        && d.ic.p.iter().flatten().all(|x| x.is_finite())
        && d.degenerate != Some(Degenerate::InvariantInfeasible)
}

/// The `(L_z, E)` chart at `mapping`: its `K*` reads the potential at the pixel's masses, which `μ_max` moves.
fn le_chart<P: Copy>(n: fn(f64) -> P, mapping: [Mapping; 2]) -> Mapped<InvariantLE<P>, Defaults>
where
    f64: From<P>,
{
    Mapped::new(
        InvariantLE(Invariant {
            z_cfg: [n(0.25), n(-0.45)],
            z_mass: [n(0.6), n(-0.35)],
            k_max: n(2.5),
            gamma_k: n(1.5),
        }),
        mapping,
    )
}

/// The view's answer against the chart's check at `p` and layer 3 at `p`, each at the view's own precision, on a 129²
/// grid. `old` puts layer 3 at the defaults in the view's place (the control). The pixels accepted by the chart's check,
/// and those of them whose layer 3 the defaults would answer differently.
fn check_own_p<Ch, P>(
    name: &str,
    view: &Tilted<Ch, P>,
    n: fn(f64) -> P,
    p: &LinkParams<f64>,
    old: bool,
) -> (u32, u32)
where
    Ch: Tiltable<Param = P>,
    P: LinkReal + PayloadReal,
    f64: From<P>,
{
    let own = LinkParams {
        mu_max: n(p.mu_max),
        q_max: n(p.q_max),
    };
    let chart = view.chart();
    let (mut checked, mut moved) = (0, 0);
    for j in 0..=128 {
        for i in 0..=128 {
            let (u, v) = (i as f64 / 128.0, j as f64 / 128.0);
            let ([a, b], shift) = view.at::<f64>(u, v);
            let ([wa, wb], placed) = view.at::<P>(n(u), n(v));
            let placed = placed.expect("tilted");
            let wide = Shift {
                x: placed.x,
                dx: f64::from(placed.dx),
                y: placed.y,
                dy: f64::from(placed.dy),
                two: placed.two,
            };
            let passes = chart.validate_shifted(a, b, &shift.expect("tilted"), p)
                == ValidationResult::Valid
                && chart.validate_shifted(f64::from(wa), f64::from(wb), &wide, p)
                    == ValidationResult::Valid
                && chart.refusal_own(wa, wb, &placed).is_none();
            let at_p = is_system(&view.decode_params::<Defaults, P>(n(u), n(v), &own));
            let defaults = LinkParams::DEFAULT;
            let at_default = is_system(&view.decode_params::<Defaults, P>(n(u), n(v), &defaults));
            let got = if old {
                passes && at_default
            } else {
                view.validate_params(u, v, p) == ValidationResult::Valid
            };
            assert_eq!(
                got,
                passes && at_p,
                "{name}: the check misreads the system at μ_max {}, at ({u}, {v})",
                p.mu_max
            );
            checked += u32::from(passes);
            moved += u32::from(passes && at_p != at_default);
        }
    }
    (checked, moved)
}

/// Both of the arms at each `μ_max`, at one precision.
fn own_p_arms<P>(name: &str, n: fn(f64) -> P, tilt: fn(&TiltSpec, f64) -> Tilt<P>, old: bool)
where
    P: LinkReal + PayloadReal,
    f64: From<P>,
{
    let sheet = |tau, x| TiltSpec {
        tau1: tau,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x,
        y: x,
    };
    for mu_max in [1.0, 1.7, 8.0] {
        let p = LinkParams { mu_max, q_max: 2.0 };
        let linear = Tilted::new(
            le_chart(n, [Mapping::Linear; 2]),
            tilt(&sheet(30.0, 0), 2.0),
        );
        let top = Tilted::new(
            le_chart(n, [Mapping::Warped, Mapping::Linear]),
            tilt(&sheet(45.0, 6), 6.0),
        );
        let a = check_own_p(name, &linear.expect("hidden"), n, &p, old);
        let b = check_own_p(name, &top.expect("hidden"), n, &p, old);
        assert!(a.0 > 1_000 && b.0 > 1_000, "{name}: {a:?}, {b:?}");
        assert!(
            a.1 + b.1 > 0,
            "{name}: μ_max {mu_max} changes no pixel's answer: {a:?}, {b:?}"
        );
    }
}

fn check_own_parameters(old: bool) {
    own_p_arms("f64", at_f64, f64_tilt, old);
    own_p_arms("f32", at_f32, f32_tilt, old);
}

#[test]
fn qa_view_check_reads_own_parameters() {
    check_own_parameters(false);
}

negative_control!(
    qa_view_check_reads_own_parameters,
    "layer 3 read at the defaults answers for another system, so it must fail",
    expected = "the check misreads the system",
    {
        check_own_parameters(true);
    }
);

/// `visit(name, view, args…)` on each construction chart, warped and linear, tilted into a hidden target of each kind, at
/// the float `$p`.
macro_rules! each_construction {
    ($p:ty, $n:expr, $tilt:expr, $visit:ident $(, $arg:expr)*) => {{
        let n: fn(f64) -> $p = $n;
        let tilt: fn(&TiltSpec, f64) -> Tilt<$p> = $tilt;
        let sheet = |x| Arm::SheetT1.spec(x);
        let theta_range = [n(0.2), n(0.7)];
        let nu_range = [n(0.1), n(0.9)];
        let nu_k = Burrau { nu_range, k_max: n(1.0), gamma_k: n(1.5) };
        let th_k = BurrauThetaK { theta_range, k_max: n(1.0), gamma_k: n(1.5) };
        let th_lz = BurrauThetaLz { theta_range, k: n(0.5), lz_fraction: n(1.0) };
        let held_lz = tilt(&sheet(HELD_LZ), 2.0);
        let held_k = tilt(&sheet(HELD_K), 0.4);
        let control = tilt(&sheet(2), 1.0);
        let linear_nu = HeldLz { mapping: Mapping::Linear, ..HeldLz::new(nu_k) };
        let linear_th = HeldLz { mapping: Mapping::Linear, ..HeldLz::new(th_k) };
        let warped_k = HeldK::new(th_lz, n(1.0), n(1.5));
        let linear_k = HeldK { mapping: Mapping::Linear, ..warped_k };
        let simplex = MassSimplex { nu0: n(0.5), eps_m: n(1e-4) };
        let plane = EuclidPlane { m_range: [n(1.0), n(32.0)], nu_range };
        let strip = BurrauThetaDeltaM { theta_range, m_target: [n(0.5), n(0.5), n(0.0)] };
        $visit("(ν, K)", &Tilted::new(HeldLz::new(nu_k), held_lz).unwrap() $(, $arg)*);
        $visit("Φ_θ,K", &Tilted::new(HeldLz::new(th_k), held_lz).unwrap() $(, $arg)*);
        $visit("(ν, K), linear", &Tilted::new(linear_nu, held_lz).unwrap() $(, $arg)*);
        $visit("Φ_θ,K, linear", &Tilted::new(linear_th, held_lz).unwrap() $(, $arg)*);
        $visit("Φ_θ,L_z", &Tilted::new(th_lz, held_k).unwrap() $(, $arg)*);
        $visit("Φ_θ,L_z, warped", &Tilted::new(warped_k, held_k).unwrap() $(, $arg)*);
        $visit("Φ_θ,L_z, linear", &Tilted::new(linear_k, held_k).unwrap() $(, $arg)*);
        let at_nu0 = Tilted::new(RestStart::new(simplex), tilt(&sheet(NU0), 1.0));
        $visit("the simplex, ν₀", &at_nu0.unwrap() $(, $arg)*);
        $visit("the simplex", &Tilted::new(RestStart::new(simplex), control).unwrap() $(, $arg)*);
        $visit("the plane", &Tilted::new(RestStart::new(plane), control).unwrap() $(, $arg)*);
        $visit("the strip", &Tilted::new(RestStart::new(strip), control).unwrap() $(, $arg)*);
    }};
}

/// The check at each non-default `p` against the check at the defaults, over the window past the square.
fn check_p_free<Ch, P>(name: &str, view: &Tilted<Ch, P>)
where
    Ch: Tiltable<Param = P>,
    P: LinkReal + PayloadReal,
    f64: From<P>,
{
    let at = |k: u32| -0.5 + k as f64 / 32.0;
    for (mu_max, q_max) in [(1.0, 2.0), (8.0, 2.0), (5.0, 4.0)] {
        let p = LinkParams { mu_max, q_max };
        for j in 0..=64 {
            for i in 0..=64 {
                let (u, v) = (at(i), at(j));
                assert_eq!(
                    view.validate_params(u, v, &p),
                    view.validate(u, v),
                    "{name}: the check moved with μ_max {mu_max}, q_max {q_max} at ({u}, {v})"
                );
            }
        }
    }
}

#[test]
fn qa_construction_check_free_of_parameters() {
    each_construction!(f64, at_f64, f64_tilt, check_p_free);
    each_construction!(f32, at_f32, f32_tilt, check_p_free);
}

negative_control!(
    qa_construction_check_free_of_parameters,
    "a chart whose check reads the link parameters must fail the same comparison",
    expected = "the check moved",
    {
        let spec = Arm::SheetT1.spec(0);
        let chart = le_chart(at_f64, [Mapping::Linear; 2]);
        check_p_free(
            "(L_z, E)",
            &Tilted::new(chart, f64_tilt(&spec, 2.0)).unwrap(),
        );
    }
);

/// What `check_weight_pixel` reads as the weight: the view's, or a stand-in (the controls).
#[derive(Clone, Copy, PartialEq)]
enum WeightRead {
    View,
    /// 0 wherever the check rejects, the weight of a rule that reads the check.
    Check,
    /// The refusals, then 0 wherever the decode at the defaults, at the view's precision, is no system: layer 3 read at a
    /// default no one gave.
    Layer3,
    /// The tilt's everywhere, a weight that reads no refusal.
    Tilt,
}

/// Pixels seen by `qa_weight_reads_refusals_only`.
#[derive(Default)]
struct WeightTally {
    /// Refused: weight 0, rejected.
    refused: u32,
    /// Rejected by no refusal: the tilt's weight.
    rejected: u32,
    /// Accepted.
    valid: u32,
}

/// The rule at one pixel: the weight is 0 exactly where the chart refuses, at f64 or at the view's own precision, and
/// the check rejects where it is.
fn check_weight_pixel<Ch, P>(
    name: &str,
    view: &Tilted<Ch, P>,
    n: fn(f64) -> P,
    read: WeightRead,
    (u, v): (f64, f64),
    tally: &mut WeightTally,
) where
    Ch: Tiltable<Param = P>,
    P: LinkReal + PayloadReal,
    f64: From<P>,
{
    let tilt = f64::from(view.tilt().weight);
    assert!(tilt > 0.0, "{name}: the tilt's weight is {tilt}");
    let ([a, b], shift) = view.at::<f64>(u, v);
    let ([wa, wb], placed) = view.at::<P>(n(u), n(v));
    let refused = view
        .chart()
        .refusal(a, b, &shift.expect("tilted"))
        .is_some()
        || view
            .chart()
            .refusal_own(wa, wb, &placed.expect("tilted"))
            .is_some();
    let check = view.validate(u, v);
    let rejected = check != ValidationResult::Valid;
    let read_layer_3 = || {
        let d = view.decode_params::<Defaults, P>(n(u), n(v), &LinkParams::DEFAULT);
        refused || !is_system(&d)
    };
    let zero =
        (read == WeightRead::Check && rejected) || (read == WeightRead::Layer3 && read_layer_3());
    let weight = if read == WeightRead::View {
        view.weight(u, v)
    } else if read == WeightRead::Tilt || !zero {
        tilt
    } else {
        0.0
    };
    if refused {
        assert_eq!(
            weight, 0.0,
            "{name}: a refused pixel weighted, at ({u}, {v})"
        );
        assert!(
            rejected,
            "{name}: a refused pixel passes the check at ({u}, {v})"
        );
        tally.refused += 1;
    } else {
        assert_eq!(
            weight, tilt,
            "{name}: the weight reads more than the refusals at ({u}, {v})"
        );
        tally.rejected += u32::from(rejected);
        tally.valid += u32::from(!rejected);
    }
}

/// The rule over the window past the square, the domain ends inside it.
fn check_weight_window<Ch, P>(
    name: &str,
    view: &Tilted<Ch, P>,
    n: fn(f64) -> P,
    read: WeightRead,
    tally: &mut WeightTally,
) where
    Ch: Tiltable<Param = P>,
    P: LinkReal + PayloadReal,
    f64: From<P>,
{
    let at = |k: u32| -0.5 + k as f64 / 32.0;
    for j in 0..=64 {
        for i in 0..=64 {
            check_weight_pixel(name, view, n, read, (at(i), at(j)), tally);
        }
    }
}

/// `weight` takes the pixel and nothing else: a parameter added to it fails the build here.
fn pixel_only<Ch: Tiltable<Param = f64>>(_: fn(&Tilted<Ch, f64>, f64, f64) -> f64) {}

fn check_weight_rule(read: WeightRead) {
    pixel_only::<HeldLz<Burrau<f64>, f64>>(Tilted::weight);
    let mut wide = WeightTally::default();
    each_construction!(
        f64,
        at_f64,
        f64_tilt,
        check_weight_window,
        at_f64,
        read,
        &mut wide
    );
    assert!(
        wide.refused > 100 && wide.rejected > 100 && wide.valid > 100,
        "at f64: {} refused, {} rejected, {} valid",
        wide.refused,
        wide.rejected,
        wide.valid
    );
    let mut own = WeightTally::default();
    each_construction!(
        f32,
        at_f32,
        f32_tilt,
        check_weight_window,
        at_f32,
        read,
        &mut own
    );
    assert!(
        own.refused > 100 && own.rejected > 100 && own.valid > 100,
        "at f32: {} refused, {} rejected, {} valid",
        own.refused,
        own.rejected,
        own.valid
    );

    // A linear energy axis: its check rejects pixels past the parabola, and no refusal is a construction's.
    let spec = Arm::SheetT1.spec(0);
    let mut latent = WeightTally::default();
    let chart = le_chart(at_f64, [Mapping::Linear; 2]);
    let view = Tilted::new(chart, f64_tilt(&spec, 2.0)).unwrap();
    check_weight_window("(L_z, E)", &view, at_f64, read, &mut latent);
    let chart = le_chart(at_f32, [Mapping::Linear; 2]);
    let view = Tilted::new(chart, f32_tilt(&spec, 2.0)).unwrap();
    check_weight_window("(L_z, E) at f32", &view, at_f32, read, &mut latent);
    assert!(
        latent.refused == 0 && latent.rejected > 100 && latent.valid > 100,
        "(L_z, E): {} refused, {} rejected, {} valid",
        latent.refused,
        latent.rejected,
        latent.valid
    );
}

#[test]
fn qa_weight_reads_refusals_only() {
    check_weight_rule(WeightRead::View);
}

negative_control!(
    qa_weight_reads_refusals_only,
    "a weight that reads the check zeroes a pixel no refusal leaves empty, so it must fail",
    expected = "the weight reads more than the refusals",
    {
        check_weight_rule(WeightRead::Check);
    }
);

mod weight_layer_3 {
    use super::*;
    negative_control!(
        qa_weight_reads_refusals_only,
        "layer 3 at the defaults in the weight, a default it wasn't given, must fail",
        expected = "the weight reads more than the refusals",
        {
            check_weight_rule(WeightRead::Layer3);
        }
    );
}

mod weight_no_refusal {
    use super::*;
    negative_control!(
        qa_weight_reads_refusals_only,
        "a refused pixel weighted as the tilt must fail",
        expected = "a refused pixel weighted",
        {
            check_weight_rule(WeightRead::Tilt);
        }
    );
}
