//! qa's tests for TASK-M2-52, written from REQ-CHART-062's R-633 arms and REQ-CHART-065 (chart_reference §4.5,
//! "Hidden coordinates under tilt and slice", and §2.2's construction), each with its negative control (R-176):
//!
//! - `qa_tilt_every_chart_hidden_set`: each of the six construction charts offers exactly §4.5's table's hidden
//!   coordinates: the mass simplex `ν₀` and the four momentum controls, the Euclid plane and the δm strip the four
//!   momentum controls, `(ν, K)` and `Φ_θ,K` the held `L_z`, `Φ_θ,L_z` the held `K`.
//! - `qa_tilt_every_chart_held_lz`: `(ν, K)` and `Φ_θ,K` tilted 30° into the held `L_z` (Tip as sheet on `τ₁`, Lean
//!   each axis on `τ₂`), and slid to a non-zero centre: each pixel's `(a, b, d)` is `R·(u, v, 0)` from the closed form;
//!   where `K(b) ≥ (L_z + d)²/2I` (`I = 1`) the momenta satisfy §2.2's three constraints, `Σp = 0`, `L_z(p) = L_z + d`
//!   and `K(p) = K(b)`, to machine precision, the cell valid and weighted as the tilt; where `K(b) < (L_z + d)²/2I` the
//!   cell is refused: `DEGENERATE(INVARIANT_INFEASIBLE)`, the view's check a rejection, area weight 0, and its momenta
//!   §2.2's `a = 0` ones (`L_z(p) = L_z + d`, `K(p) = K_min`). Both kinds occur on the window.
//! - `qa_tilt_every_chart_held_k`: `Φ_θ,L_z` tilted into the held `K`, the same with `K* = K + d` and
//!   `L_z(b) = (2b − 1)·λ_L·√(2K)` at the centre's `K`.
//! - `qa_tilt_every_chart_nu0`: the mass simplex tilted into `ν₀`: each pixel's masses are §4.5's ternary map at
//!   `(a, b)` and its triangle is the Burrau right triangle at `ν₀ + d` (side ratios `(1 − ν²) : 2ν : (1 + ν²)`
//!   opposite bodies 2, 1, 0), at rest; where `ν₀ + d ∉ (0, 1)` the cell is refused, weight 0.
//! - `qa_tilt_every_chart_rest_start`: each rest-start chart tilted into each momentum control: masses and positions
//!   the untilted chart's at `(a, b)`, momenta `D_mom` of the control at `d` (the others 0) under the pixel's masses,
//!   `Σp = 0`; every cell valid. The centre pixel of every arm is the untilted chart's, bitwise.
//! - `qa_slice_every_chart_construction_edge`: a physical slice past the construction's edge (`ν₀ ∉ (0, 1)`, a held
//!   `L_z` past `√(2IK(v))`, a held `K < 0`) leaves a centre that `validate` refuses, so REQ-CHART-060's stop can
//!   find it; one inside passes.
//! - `qa_slice_every_chart_construction`: a slice of each hidden coordinate moves only that held value; the decode at
//!   the centre follows (`L_z(p)` the sliced `L_z`; `K(p)` the sliced `K`, its band following; the triangle at the sliced
//!   `ν₀`; `D_mom` of the sliced control); a slice of any other index is not offered.
//!
//! Controls: the held `L_z` read without `d`; a refused cell weighted as the tilt; the triangle read at `ν₀` without
//! `d`; a momentum control's `d` put on the next control; a `K` slice that leaves the momenta's `K` unchanged.

use kernel::chart::construction::{Held, HELD_K, HELD_LZ, NU0};
use kernel::chart::tilt::{Hidden, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, EuclidPlane, HeldLz,
    MassSimplex, RestStart, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::decode::{momentum, Decoded, Degenerate};
use kernel::generated::links::{Defaults, LinkParams};
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
    let cases: [(&str, Vec<u32>, Vec<u32>); 6] = [
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

#[derive(Clone, Copy, PartialEq)]
enum Ctl {
    None,
    /// The held value read without `d`.
    NoD,
    /// A refused cell weighted as the tilt.
    WeightRefused,
}

/// Over a 33 × 33 grid of `view`: `target(a, b, d)` gives `(L_z, K*)`; the §2.2 constraints on each feasible cell,
/// refusal on each infeasible one.
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

fn check_held_lz(ctl: Ctl) {
    let span = 2.0;
    for centre_lz in [0.0, 0.15] {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            let mut a = HeldLz::new(nu_k());
            a.lz = centre_lz;
            let ka = a.chart;
            check_construction(
                "(ν, K)",
                &view(a, arm, HELD_LZ, span),
                arm,
                span,
                |_, b, d| (centre_lz + d, ka.k_max * b.powf(ka.gamma_k)),
                a.decode::<Defaults, f64>(C[0], C[1]),
                ctl,
            );
            let mut t = HeldLz::new(theta_k());
            t.lz = centre_lz;
            let kt = t.chart;
            check_construction(
                "Φ_θ,K",
                &view(t, arm, HELD_LZ, span),
                arm,
                span,
                |_, b, d| (centre_lz + d, kt.k_max * b.powf(kt.gamma_k)),
                t.decode::<Defaults, f64>(C[0], C[1]),
                ctl,
            );
        }
    }
}

#[test]
fn qa_tilt_every_chart_held_lz() {
    check_held_lz(Ctl::None);
}

negative_control!(
    qa_tilt_every_chart_held_lz,
    "the held L_z read without the tilt's d must fail",
    expected = "does not realise the tilted L_z",
    {
        check_held_lz(Ctl::NoD);
    }
);

mod refused_weighted {
    use super::*;
    negative_control!(
        qa_tilt_every_chart_held_lz,
        "a refused cell counted with the tilt's area weight must fail",
        expected = "refused cell weighted",
        {
            check_held_lz(Ctl::WeightRefused);
        }
    );
}

fn check_held_k(ctl: Ctl) {
    let span = 2.0;
    for centre_k in [0.5, 0.3] {
        for arm in [Arm::SheetT1, Arm::LeanT2] {
            let chart = theta_lz(centre_k);
            check_construction(
                "Φ_θ,L_z",
                &view(chart, arm, HELD_K, span),
                arm,
                span,
                // L_z(b) = (2b − 1)·λ_L·√(2IK) at the centre's K; K* = K + d.
                |_, b, d| ((2.0 * b - 1.0) * (2.0 * centre_k).sqrt(), centre_k + d),
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

fn check_slice(stale_k: bool) {
    // (ν, K) and Φ_θ,K: the held L_z.
    let a = HeldLz::new(nu_k());
    let s = a.sliced(HELD_LZ, 0.3).expect("L_z is sliceable");
    assert_eq!((s.chart, s.held(HELD_LZ)), (a.chart, Some(0.3)));
    let d = s.decode::<Defaults, f64>(C[0], C[1]);
    assert!((lz_of(&d) - 0.3).abs() < TOL, "L_z(p) = {}", lz_of(&d));
    assert!((k_of(&d) - nu_k().kinetic::<f64>(C[1])).abs() < TOL);
    let t = HeldLz::new(theta_k());
    let s = t.sliced(HELD_LZ, -0.2).expect("L_z is sliceable");
    assert_eq!((s.chart, s.held(HELD_LZ)), (t.chart, Some(-0.2)));
    let d = s.decode::<Defaults, f64>(C[0], C[1]);
    assert!((lz_of(&d) + 0.2).abs() < TOL);
    // Φ_θ,L_z: the held K, its band following.
    let c = theta_lz(0.5);
    let s = c.sliced(HELD_K, 0.8).expect("K is sliceable");
    assert_eq!(
        (s.theta_range, s.lz_fraction, s.held(HELD_K)),
        (c.theta_range, c.lz_fraction, Some(0.8))
    );
    let d = s.decode::<Defaults, f64>(C[0], C[1]);
    let want_k = if stale_k { 0.5 } else { 0.8 };
    assert!(
        (k_of(&d) - want_k).abs() < TOL,
        "the sliced K: K(p) = {}, expected {want_k}",
        k_of(&d)
    );
    assert!((lz_of(&d) - (2.0 * C[1] - 1.0) * 1.6f64.sqrt()).abs() < TOL);
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
    }
}

/// REQ-CHART-065's "subject to REQ-CHART-060's stop": REQ-CHART-060 stops a slice where the centre, `Φ` at the slice
/// centre's hidden coordinates, fails `validate`. So a physical slice past the construction's edge must leave a centre
/// that fails it, and one inside must pass: `ν₀ ∉ (0, 1)`; a held `L_z` with `K(v) < L_z²/2I`; a held `K < 0`, below
/// every `L_z(v)²/2I`. `past_as_valid` expects the centres past the edge to validate (the control).
fn check_slice_edge(past_as_valid: bool) {
    let ok = |r: ValidationResult| r == ValidationResult::Valid;
    let refused = |r: ValidationResult| matches!(r, ValidationResult::Reject { .. });
    let past = |r: ValidationResult, what: &str| {
        let want = if past_as_valid { ok(r) } else { refused(r) };
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
    let a = HeldLz::new(nu_k());
    let k = nu_k().kinetic::<f64>(C[1]);
    let edge = (2.0 * k).sqrt();
    assert!(ok(a
        .sliced(HELD_LZ, 0.9 * edge)
        .unwrap()
        .validate(C[0], C[1])));
    assert!(ok(a
        .sliced(HELD_LZ, -0.9 * edge)
        .unwrap()
        .validate(C[0], C[1])));
    past(
        a.sliced(HELD_LZ, 1.1 * edge).unwrap().validate(C[0], C[1]),
        "L_z",
    );
    past(
        a.sliced(HELD_LZ, -1.1 * edge).unwrap().validate(C[0], C[1]),
        "−L_z",
    );
    let t = HeldLz::new(theta_k());
    let k = theta_k().kinetic::<f64>(C[1]);
    let edge = (2.0 * k).sqrt();
    assert!(ok(t
        .sliced(HELD_LZ, 0.9 * edge)
        .unwrap()
        .validate(C[0], C[1])));
    past(
        t.sliced(HELD_LZ, 1.1 * edge).unwrap().validate(C[0], C[1]),
        "Φ_θ,K's L_z",
    );
    let c = theta_lz(0.5);
    assert!(ok(c.sliced(HELD_K, 0.05).unwrap().validate(C[0], C[1])));
    past(c.sliced(HELD_K, -0.5).unwrap().validate(C[0], C[1]), "K");
}

#[test]
fn qa_slice_every_chart_construction_edge() {
    check_slice_edge(false);
}

negative_control!(
    qa_slice_every_chart_construction_edge,
    "a centre past the edge expected to validate must fail",
    expected = "a slice past the edge",
    {
        check_slice_edge(true);
    }
);

#[test]
fn qa_slice_every_chart_construction() {
    check_slice(false);
}

negative_control!(
    qa_slice_every_chart_construction,
    "a K slice expected to leave the momenta's K unchanged must fail",
    expected = "the sliced K",
    {
        check_slice(true);
    }
);
