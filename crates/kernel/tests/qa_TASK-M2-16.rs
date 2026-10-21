//! QA tests for TASK-M2-16, written from the requirements and their sources, not from the implementation: the encode
//! contract's theorems T2 and T3 in physical units (inverse-encode contract Parts 1, 3, 4, 6 and 8; dd_encode §3.2,
//! §3.5 and §5; dd_decoder §3.3, §3.4).
//!
//! Every expected state comes from an independent construction written here from the text:
//! - [`qa_phys`], Part 4's norm `‖·‖_phys` as Part 4 states it (REQ-ENC-029);
//! - [`qa_canon`], Part 6's canonical inverse policy in the normative order 1a → 0 → 1b → 2 → 3, every rigid
//!   operation on the full state (positions and momenta), `φ = 0` at `ρ = 0` by a select (R-437), mirror iff
//!   `λ̃_y < −δ_λ` (R-82), with its two classic bugs, a configuration-only mirror and a configuration-only rotation;
//! - [`qa_gauge`], dd_encode §3.3's gauge action `r' = s·R(θ)·Π·r + a`, `p' = s^{−1/2}·R(θ)·Π·p + m·u`;
//! - Part 3's forward momentum link `q = q_max·(2σ(z) − 1)` and the Jacobi map `p₂ = p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`,
//!   `p₀ = −p₁ − p₂` (dd_decoder §3.4).
//!
//! The tests:
//! - REQ-ENC-005, REQ-DEC-016 (`t3_qa_reference_canon`): random gauge on canonical states, both precisions:
//!   `D(E(x)) = C(x)` against [`qa_canon`], masses within `ε_phys`, `lookup_mirrored` the gauge's parity, `λ = 1/s`,
//!   `L_z = ±L_z,in·λ^{1/2}` with the sign `lookup_mirrored` gives, `E = E_in/λ`. Controls: a configuration-only mirror
//!   and a configuration-only rotation fail the `L_z` check.
//! - REQ-DEC-016, R-437 (`t3_qa_rho_zero`): the coincident inner pair at every sign of the zeros: `φ = 0`, the
//!   rotation the identity, `lookup_clamped`, at both precisions.
//! - REQ-ENC-001, REQ-ENC-004, REQ-ENC-018 (`encode_qa_golden_z0`, `encode_qa_burrau_345`): the golden pairs built from
//!   the text: `z = 0`'s equilateral triangle on the unit circle, and Burrau's physical (3, 4, 5) at rest, its `z` by
//!   Part 3's closed forms; the scale step at 2× (`λ = ½`, `E_canon = 2·E_in`, dd_encode §5 test 4).
//! - REQ-DEC-029, REQ-ENC-004 (`t2_qa_qmax_no_cap`): decode applies no per-body cap (a body's `|p|` above `q_max` is
//!   kept, the Jacobi map of `q` alone); `|qₖ| ≤ q_max` at any `z`; clamp-adjacent momentum controls set their
//!   `lookup_clamped` bit and interior ones hold T2 within `ε_phys`. Control: the rejected cap fails the check.
//! - REQ-VAL-013 (`conditioning_qa_physical_space`): `z_q0` near its clamp, `d logit/ds ~ 10⁵`: the physical residual
//!   passes `ε_phys`, a naive `z` residual exceeds it.
//!
//! **Tolerances.** `ε_phys` alone (REQ-ENC-024, calibrated: `validation::roundtrip::eps_phys`, the register's `k`
//! times the precision's ε, asserted in `validation/tests/qa_TASK-M2-16.rs`); a derived quantity (`L_z`, `E`) is held
//! to the first-order change a state within `ε_phys` in `‖·‖_phys` can make to it. No other tolerance is set.
//!
//! **Domain** (pitfalls §10): encode and decode on states held fixed; no trajectory. Each test has a registered
//! negative control (R-176).

use kernel::canon::canonicalise;
use kernel::decode::{self, Decoded, Ic, Latent};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use validation::negative_control;
use validation::roundtrip::{eps_phys, widen, Precision};

// ── Independent references ─────────────────────────────────────────────────────────────────────────────────────

/// `q_max` (R-10, settled at 2), the momentum clamp `ε_q` and the configuration clamp `ε_z` (Part 3, 10⁻⁶), the
/// mirror deadband `δ_λ` (R-82, 10⁻¹²).
const Q_MAX: f64 = 2.0;
const EPS_Q: f64 = 1e-6;
const DELTA_LAMBDA: f64 = 1e-12;

/// A tiny deterministic generator (splitmix64), uniform draws on `[lo, hi)`.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

fn len(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

/// Part 4: `maxᵢ max(|m′ᵢ − mᵢ|, ‖r′ᵢ − rᵢ‖/R_x, ‖p′ᵢ − pᵢ‖/P_x)`, `R_x = max(1, maxⱼ‖rⱼ‖)`, `P_x = max(1,
/// maxⱼ‖pⱼ‖)`, `x` the reference. A NaN gives NaN (so it fails every `≤`).
fn qa_phys(x: &Ic<f64>, xp: &Ic<f64>) -> f64 {
    let mut rx = 1.0f64;
    let mut px = 1.0f64;
    for j in 0..3 {
        rx = rx.max(len(x.r[j]));
        px = px.max(len(x.p[j]));
    }
    let mut worst = 0.0f64;
    for i in 0..3 {
        for d in [
            (xp.m[i] - x.m[i]).abs(),
            len([xp.r[i][0] - x.r[i][0], xp.r[i][1] - x.r[i][1]]) / rx,
            len([xp.p[i][0] - x.p[i][0], xp.p[i][1] - x.p[i][1]]) / px,
        ] {
            if d.is_nan() {
                return f64::NAN;
            }
            worst = worst.max(d);
        }
    }
    worst
}

/// Which of `C`'s rigid operations act on the momenta (Part 1): the contract's `FULL`, both; the classic bugs leave
/// the mirror (`.0`) or the rotation (`.1`) off the momenta.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Rigid(bool, bool);

const FULL: Rigid = Rigid(true, true);

/// `C(x)` by Part 6, in f64: 1a CoM subtraction; 0 rescale to `I = 1` (`r ← λr`, `p ← λ^{−1/2}p`, `λ = I_in^{−1/2}`);
/// 1b boost removal (`pᵢ ← pᵢ − mᵢP/M`); 2 rotate by `R(−φ)`, `φ = atan2(ρ_y, ρ_x)`, `φ = 0` where `ρ = 0` (R-437);
/// 3 mirror `y → −y` iff `λ̃_y < −δ_λ`. Returns the state, `λ`, and whether it mirrored.
fn qa_canon(x: &Ic<f64>, rigid: Rigid) -> (Ic<f64>, f64, bool) {
    let m = x.m;
    let mt: f64 = m.iter().sum();
    let mut r = x.r;
    let mut p = x.p;
    let com = [
        (0..3).map(|i| m[i] * r[i][0]).sum::<f64>() / mt,
        (0..3).map(|i| m[i] * r[i][1]).sum::<f64>() / mt,
    ];
    for v in &mut r {
        *v = [v[0] - com[0], v[1] - com[1]];
    }
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum();
    let lambda = 1.0 / inertia.sqrt();
    let pscale = 1.0 / lambda.sqrt();
    for i in 0..3 {
        r[i] = [r[i][0] * lambda, r[i][1] * lambda];
        p[i] = [p[i][0] * pscale, p[i][1] * pscale];
    }
    let ptot = [
        p.iter().map(|v| v[0]).sum::<f64>(),
        p.iter().map(|v| v[1]).sum::<f64>(),
    ];
    for i in 0..3 {
        p[i] = [p[i][0] - m[i] * ptot[0] / mt, p[i][1] - m[i] * ptot[1] / mt];
    }
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let phi = if rho[0] == 0.0 && rho[1] == 0.0 {
        0.0
    } else {
        rho[1].atan2(rho[0])
    };
    let (c, s) = (phi.cos(), phi.sin());
    let rot = |v: [f64; 2]| [c * v[0] + s * v[1], -s * v[0] + c * v[1]];
    for i in 0..3 {
        r[i] = rot(r[i]);
        if rigid.1 {
            p[i] = rot(p[i]);
        }
    }
    let m01 = m[0] + m[1];
    let lam_y = r[2][1] - (m[0] * r[0][1] + m[1] * r[1][1]) / m01;
    let mu_lam = m[2] * m01 / mt;
    let mirrored = mu_lam.sqrt() * lam_y < -DELTA_LAMBDA;
    if mirrored {
        for i in 0..3 {
            r[i][1] = -r[i][1];
            if rigid.0 {
                p[i][1] = -p[i][1];
            }
        }
    }
    (Ic { m, r, p }, lambda, mirrored)
}

/// dd_encode §3.3's gauge action on `x`.
fn qa_gauge(x: &Ic<f64>, a: [f64; 2], u: [f64; 2], theta: f64, s: f64, mirror: bool) -> Ic<f64> {
    let (c, sn) = (theta.cos(), theta.sin());
    let act = |v: [f64; 2], k: f64| {
        let w = if mirror { [v[0], -v[1]] } else { v };
        [k * (c * w[0] - sn * w[1]), k * (sn * w[0] + c * w[1])]
    };
    let mut y = *x;
    for i in 0..3 {
        let r = act(x.r[i], s);
        let p = act(x.p[i], 1.0 / s.sqrt());
        y.r[i] = [r[0] + a[0], r[1] + a[1]];
        y.p[i] = [p[0] + x.m[i] * u[0], p[1] + x.m[i] * u[1]];
    }
    y
}

/// `x`'s state in its own CoM frame, at rest: positions about the CoM, momenta less `mᵢP/M`.
fn com_frame(x: &Ic<f64>) -> Ic<f64> {
    let mt: f64 = x.m.iter().sum();
    let mut y = *x;
    for k in 0..2 {
        let c = (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>() / mt;
        let pt = (0..3).map(|i| x.p[i][k]).sum::<f64>();
        for i in 0..3 {
            y.r[i][k] = x.r[i][k] - c;
            y.p[i][k] = x.p[i][k] - x.m[i] * pt / mt;
        }
    }
    y
}

/// `L_z = Σ rᵢ × pᵢ` in the CoM frame.
fn lz(x: &Ic<f64>) -> f64 {
    let y = com_frame(x);
    (0..3)
        .map(|i| y.r[i][0] * y.p[i][1] - y.r[i][1] * y.p[i][0])
        .sum()
}

/// `E = Σ ‖pᵢ‖²/(2mᵢ) − Σ_{i<j} mᵢmⱼ/rᵢⱼ` in the CoM frame (`G = 1`).
fn energy(x: &Ic<f64>) -> f64 {
    let y = com_frame(x);
    let kin: f64 = (0..3)
        .map(|i| (y.p[i][0].powi(2) + y.p[i][1].powi(2)) / (2.0 * y.m[i]))
        .sum();
    let pot: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| y.m[i] * y.m[j] / len([y.r[i][0] - y.r[j][0], y.r[i][1] - y.r[j][1]]))
        .sum();
    kin - pot
}

/// The first-order bounds on `|ΔL_z|` and `|ΔE|` of a state within `δ` of `x` in Part 4's norm (`|Δmᵢ| ≤ δ`,
/// `‖Δrᵢ‖ ≤ δR`, `‖Δpᵢ‖ ≤ δP`), plus `δ` relative to the value itself for the rescale's own rounding of it.
fn derived_bounds(x: &Ic<f64>, delta: f64) -> (f64, f64) {
    let (mut rr, mut pp) = (1.0f64, 1.0f64);
    for j in 0..3 {
        rr = rr.max(len(x.r[j]));
        pp = pp.max(len(x.p[j]));
    }
    let mut dl = 0.0;
    let mut de = 0.0;
    for i in 0..3 {
        let (r, p, m) = (len(x.r[i]), len(x.p[i]), x.m[i]);
        dl += rr * p + pp * r;
        de += pp * p / m + p * p / (2.0 * m * m);
    }
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let d = len([x.r[i][0] - x.r[j][0], x.r[i][1] - x.r[j][1]]);
        de += 2.0 * rr * x.m[i] * x.m[j] / (d * d) + (x.m[i] + x.m[j]) / d;
    }
    (delta * (dl + lz(x).abs()), delta * (de + energy(x).abs()))
}

fn narrow<R: Precision>(x: &Ic<f64>) -> Ic<R> {
    Ic {
        m: x.m.map(R::of),
        r: x.r.map(|v| v.map(R::of)),
        p: x.p.map(|v| v.map(R::of)),
    }
}

fn dec<R: Precision>(z: [R; 8]) -> Decoded<R> {
    decode::decode::<Defaults, R>(&Latent::new(z))
}

fn bits<R: Precision>() -> u32 {
    if R::EPS == f64::EPSILON {
        64
    } else {
        32
    }
}

// ── T3 against the reference C (REQ-ENC-005, REQ-DEC-016) ────────────────────────────────────────────────────────

const T3_SAMPLES: u32 = 1500;

/// The T3 checks on input `x` (f64 view of what encode saw) and `back = D(E(x))`, with encode's `mirrored` and `λ`,
/// against `C(x)` built by [`qa_canon`] as the contract has it.
fn check_t3(
    what: &str,
    x: &Ic<f64>,
    back: &Ic<f64>,
    mirrored: bool,
    lambda: f64,
    parity: bool,
    eps: f64,
) {
    let (c, lam_ref, mir_ref) = qa_canon(x, FULL);
    // The half-mirror catch first (Part 8): |L_z| matches with its sign by lookup_mirrored.
    let sign = if mirrored { -1.0 } else { 1.0 };
    let want_l = sign * lz(x) * lam_ref.sqrt();
    let (bl, be) = derived_bounds(&c, eps);
    let got_l = lz(back);
    assert!(
        (got_l - want_l).abs() <= bl,
        "{what}: L_z of D(E(x)) is {got_l:e}, want {want_l:e} (sign by lookup_mirrored = {mirrored}), bound {bl:e}"
    );
    let r = qa_phys(&c, back);
    assert!(
        r <= eps,
        "{what}: D(E(x)) is {r:.3e} from C(x) in ‖·‖_phys, beyond ε_phys {eps:.3e}"
    );
    assert_eq!(
        mirrored, mir_ref,
        "{what}: lookup_mirrored against C's step 3"
    );
    assert_eq!(
        mirrored, parity,
        "{what}: lookup_mirrored against the gauge's parity"
    );
    assert!(
        (lambda / lam_ref - 1.0).abs() <= eps,
        "{what}: λ {lambda:e} against I_in^(−1/2) {lam_ref:e}"
    );
    let want_e = energy(x) / lam_ref;
    let got_e = energy(back);
    assert!(
        (got_e - want_e).abs() <= be,
        "{what}: E of D(E(x)) is {got_e:e}, want E_in/λ {want_e:e}, bound {be:e}"
    );
    for i in 0..3 {
        assert!(
            (back.m[i] - x.m[i]).abs() <= eps,
            "{what}: mass {i} {} against {}",
            back.m[i],
            x.m[i]
        );
    }
}

/// T3 over random `g·x₀` at `R`, `x₀ = D(z)` with `z` on `[−4, 4]⁸` (every control well inside its clamp, so `x₀` is
/// canonical and clear of the mirror's deadband); `g` with `s` on `2^[−3, 3)`, `θ` on `[−π, π)`, parity ½, offset on
/// `s·[−2, 2)²`, boost on `s^{−1/2}·[−1, 1)²`. With `rigid` a bug, `back` is that bug's `C(x)` instead of `D(E(x))`.
fn t3_run<R: Precision>(rigid: Rigid) -> (u32, u32) {
    let mut rng = Rng(0x5eed_2016);
    let eps = eps_phys::<R>();
    let mut mirrored_n = 0;
    let mut kept_n = 0;
    for i in 0..T3_SAMPLES {
        let z: [f64; 8] = std::array::from_fn(|_| rng.uniform(-4.0, 4.0));
        let x0 = widen(&dec::<f64>(z).ic);
        let s = rng.uniform(-3.0, 3.0).exp2();
        let theta = rng.uniform(-std::f64::consts::PI, std::f64::consts::PI);
        let parity = rng.next() & 1 == 1;
        let a = [s * rng.uniform(-2.0, 2.0), s * rng.uniform(-2.0, 2.0)];
        let u = [
            rng.uniform(-1.0, 1.0) / s.sqrt(),
            rng.uniform(-1.0, 1.0) / s.sqrt(),
        ];
        let x = narrow::<R>(&qa_gauge(&x0, a, u, theta, s, parity));
        let xw = widen(&x);
        let e = encode::<Defaults, R>(&x);
        let (back, mirrored) = if rigid == FULL {
            (widen(&dec::<R>(e.z.z).ic), e.report.mirrored)
        } else {
            let (c, _, m) = qa_canon(&xw, rigid);
            (c, m)
        };
        if mirrored {
            mirrored_n += 1;
        } else {
            kept_n += 1;
        }
        let what = format!(
            "f{} sample {i} (s {s:e}, θ {theta}, parity {parity})",
            bits::<R>()
        );
        check_t3(
            &what,
            &xw,
            &back,
            mirrored,
            e.report.lambda.into(),
            parity,
            eps,
        );
    }
    (mirrored_n, kept_n)
}

#[test]
fn t3_qa_reference_canon() {
    for (n, w) in [(t3_run::<f64>(FULL), 64), (t3_run::<f32>(FULL), 32)] {
        assert!(
            n.0 >= T3_SAMPLES / 4 && n.1 >= T3_SAMPLES / 4,
            "f{w}: the draws mirror {} and keep {}: both parities must be exercised",
            n.0,
            n.1
        );
    }
}

negative_control!(
    t3_qa_reference_canon,
    "a canonicaliser mirroring the positions only must fail the L_z check",
    expected = "L_z of D(E(x))",
    {
        t3_run::<f64>(Rigid(false, true));
    }
);

#[cfg(feature = "controls")]
mod t3_qa_config_only_rotation {
    use super::*;

    #[test]
    #[should_panic(expected = "L_z of D(E(x))")]
    fn negative_control() {
        validation::control::run("L_z of D(E(x))", || {
            t3_run::<f64>(Rigid(true, false));
        })
    }
}

// ── ρ = 0, every sign of its zeros (REQ-DEC-016, R-437) ─────────────────────────────────────────────────────────

/// The coincident inner pair at `R`: `r₀ = r₁ = (±0, ±0)`, body 2 off the pair, nonzero momenta. `C` takes `φ = 0`
/// (its rotation the identity: the result is [`qa_canon`]'s, whose `φ` is the select's 0) and raises `lookup_clamped`;
/// encode reports it and its `z` is finite.
fn rho_zero<R: Precision>(phi_of: fn(&Ic<R>) -> f64) {
    let eps = eps_phys::<R>();
    for sx in [0.0, -0.0] {
        for sy in [0.0, -0.0] {
            for side in [1.0, -1.0] {
                let x = Ic {
                    m: [0.3, 0.3, 0.4],
                    r: [[sx, sy], [sx, sy], [0.4, side * 0.9]],
                    p: [[0.2, -0.1], [-0.3, 0.25], [0.1, -0.15]],
                };
                let xr = narrow::<R>(&x);
                let what = format!("f{} ρ at ({sx:?}, {sy:?}), body 2 side {side}", bits::<R>());
                let phi = phi_of(&xr);
                assert!(phi == 0.0, "{what}: φ = {phi:e}, not R-437's 0");
                let c = canonicalise(&xr);
                assert!(c.clamped, "{what}: lookup_clamped not raised at ρ = 0");
                let (want, _, _) = qa_canon(&widen(&xr), FULL);
                let r = qa_phys(&want, &widen(&c.ic));
                assert!(
                    r <= eps,
                    "{what}: C(x) is {r:.3e} from the unrotated reference"
                );
                let e = encode::<Defaults, R>(&xr);
                assert!(
                    e.report.lookup_clamped() && e.report.coincident,
                    "{what}: encode does not report it"
                );
                let zf: [f64; 8] = e.z.z.map(Into::into);
                assert!(
                    zf.iter().all(|v| v.is_finite()),
                    "{what}: z {zf:?} not finite"
                );
            }
        }
    }
}

fn kernel_phi<R: Precision>(x: &Ic<R>) -> f64 {
    canonicalise(x).phi.into()
}

#[test]
fn t3_qa_rho_zero() {
    rho_zero::<f64>(kernel_phi::<f64>);
    rho_zero::<f32>(kernel_phi::<f32>);
}

negative_control!(
    t3_qa_rho_zero,
    "a φ taken from a nonzero branch value (π) must fail the R-437 check",
    expected = "not R-437's 0",
    rho_zero::<f64>(|_| std::f64::consts::PI)
);

// ── Golden pairs (dd_encode §5; REQ-ENC-001, REQ-ENC-004, REQ-ENC-018) ────────────────────────────────────────────

/// `z = 0`'s golden state, by dd_decoder §5 / Part 1's section: equal masses, the equilateral triangle on the unit
/// circle (side √3), `ρ̃` on `+x`, body 2 above, at rest, `I = 1`.
fn z0_state() -> Ic<f64> {
    let h = 3f64.sqrt() / 2.0;
    Ic {
        m: [1.0 / 3.0; 3],
        r: [[-h, -0.5], [h, -0.5], [0.0, 1.0]],
        p: [[0.0; 2]; 3],
    }
}

fn golden_z0<R: Precision>(z0: [f64; 8]) {
    let eps = eps_phys::<R>();
    let x = z0_state();
    let at_z = widen(&dec::<R>(z0.map(R::of)).ic);
    let r = qa_phys(&x, &at_z);
    assert!(
        r <= eps,
        "f{}: D(z) is {r:.3e} from the golden state",
        bits::<R>()
    );
    let e = encode::<Defaults, R>(&narrow::<R>(&x));
    let back = widen(&dec::<R>(e.z.z).ic);
    let r = qa_phys(&x, &back);
    assert!(
        r <= eps,
        "f{}: D(E(x)) is {r:.3e} from the golden state",
        bits::<R>()
    );
    assert!(
        !e.report.lookup_clamped() && !e.report.mirrored,
        "f{}: the golden state is interior",
        bits::<R>()
    );
}

#[test]
fn encode_qa_golden_z0() {
    golden_z0::<f64>([0.0; 8]);
    golden_z0::<f32>([0.0; 8]);
}

negative_control!(
    encode_qa_golden_z0,
    "a z moved off 0 by 10⁻⁶ in z_α does not decode to the golden state",
    expected = "D(z) is",
    {
        let mut z = [0.0; 8];
        z[0] = 1e-6;
        golden_z0::<f64>(z);
    }
);

/// Burrau's (3, 4, 5) (dd_encode §5 tests 4 and 5; chart_reference §4.2's convention as the fixture records it): the
/// right angle at body 0, `r₁ = (3, 0)`, `r₂ = (0, 4)`, masses `(5, 4, 3)/12`, at rest.
fn burrau_345() -> Ic<f64> {
    Ic {
        m: [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0],
        r: [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]],
        p: [[0.0; 2]; 3],
    }
}

/// Part 3's closed forms on `C(x)`: `z_α = logit(2α/π)`, `z_β = logit(β/π)`, `α = atan2(‖λ̃‖, ‖ρ̃‖)`,
/// `β = atan2(|λ̃_y|, λ̃_x)`, the momenta `z = logit(½(q/q_max + 1))`, `z_μk = artanh(ln(mₖ/m₀)/μ_max)`, `μ_max = 5`.
fn closed_form_z(x: &Ic<f64>) -> [f64; 8] {
    let (c, _, _) = qa_canon(x, FULL);
    let m = c.m;
    let m01 = m[0] + m[1];
    let rho = [c.r[1][0] - c.r[0][0], c.r[1][1] - c.r[0][1]];
    let lam = [
        c.r[2][0] - (m[0] * c.r[0][0] + m[1] * c.r[1][0]) / m01,
        c.r[2][1] - (m[0] * c.r[0][1] + m[1] * c.r[1][1]) / m01,
    ];
    let (wr, wl) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01).sqrt());
    let alpha = (wl * len(lam)).atan2(wr * len(rho));
    let beta = (wl * lam[1]).abs().atan2(wl * lam[0]);
    let logit = |s: f64| (s / (1.0 - s)).ln();
    let pi = std::f64::consts::PI;
    let pl = c.p[2];
    let pr = [
        c.p[1][0] + m[1] / m01 * pl[0],
        c.p[1][1] + m[1] / m01 * pl[1],
    ];
    let zq = |q: f64| logit(0.5 * (q / Q_MAX + 1.0));
    [
        logit(2.0 * alpha / pi),
        logit(beta / pi),
        zq(pr[0]),
        zq(pr[1]),
        zq(pl[0]),
        zq(pl[1]),
        ((m[1] / m[0]).ln() / 5.0).atanh(),
        ((m[2] / m[0]).ln() / 5.0).atanh(),
    ]
}

/// The Burrau pair at `R`, `x` at `scale`× its canonical size: `E(x)` decodes to `D(z)` of Part 3's `z`, which is
/// `C(x)`: masses `(5, 4, 3)/12`, the triangle kept (the right angle at body 0, sides in 3 : 4 : 5), at rest; the
/// report records `λ = 1/(scale·…)` with `E_canon = √I_in·E_in`.
fn burrau_pair<R: Precision>(scale: f64, z: [f64; 8]) {
    let eps = eps_phys::<R>();
    let what = format!("f{} Burrau (3, 4, 5) at {scale}×", bits::<R>());
    let (c0, _, _) = qa_canon(&burrau_345(), FULL);
    let x = narrow::<R>(&Ic {
        m: c0.m,
        r: c0.r.map(|v| v.map(|w| w * scale)),
        p: c0.p,
    });
    let xw = widen(&x);
    let at_z = widen(&dec::<R>(z.map(R::of)).ic);
    let e = encode::<Defaults, R>(&x);
    let back = widen(&dec::<R>(e.z.z).ic);
    let r = qa_phys(&at_z, &back);
    assert!(
        r <= eps,
        "{what}: D(E(x)) is {r:.3e} from D(z) of Part 3's z"
    );
    let (c, lam_ref, _) = qa_canon(&xw, FULL);
    let r = qa_phys(&c, &at_z);
    assert!(r <= eps, "{what}: D(z) is {r:.3e} from C(x)");
    let lambda: f64 = e.report.lambda.into();
    assert!(
        (lambda * scale - 1.0).abs() <= eps,
        "{what}: λ = {lambda}, not 1/{scale}"
    );
    assert!(
        (lam_ref * scale - 1.0).abs() <= eps,
        "{what}: the reference's λ"
    );
    let (_, be) = derived_bounds(&c, eps);
    let (ein, eb) = (energy(&xw), energy(&back));
    assert!(
        (eb - ein * scale).abs() <= be,
        "{what}: E_canon {eb:e} is not √I_in·E_in = {:e}",
        ein * scale
    );
    for (i, want) in [5.0, 4.0, 3.0].iter().enumerate() {
        assert!(
            (back.m[i] - want / 12.0).abs() <= eps,
            "{what}: mass {i} is {}",
            back.m[i]
        );
    }
    let d = |i: usize, j: usize| len([back.r[i][0] - back.r[j][0], back.r[i][1] - back.r[j][1]]);
    // `back` is within 2ε_phys of `C(x)` (through `D(z)`), whose sides are 3 : 4 : 5: each side moves by at most
    // 2·(2ε_phys·R), and a ratio a/b, a ≤ b, by at most twice that over b.
    let rr = back.r.iter().map(|&v| len(v)).fold(1.0f64, f64::max);
    let tol = 8.0 * eps * rr / d(1, 2);
    assert!(
        (d(0, 1) / d(1, 2) - 0.6).abs() <= tol,
        "{what}: |r₀r₁|/|r₁r₂| = {}",
        d(0, 1) / d(1, 2)
    );
    assert!(
        (d(0, 2) / d(1, 2) - 0.8).abs() <= tol,
        "{what}: |r₀r₂|/|r₁r₂| = {}",
        d(0, 2) / d(1, 2)
    );
    for i in 0..3 {
        assert!(len(back.p[i]) <= eps, "{what}: body {i} is not at rest");
    }
}

#[test]
fn encode_qa_burrau_345() {
    let z = closed_form_z(&burrau_345());
    for scale in [1.0, 2.0] {
        burrau_pair::<f64>(scale, z);
        burrau_pair::<f32>(scale, z);
    }
    // The fixture's z is Part 3's z: D of each within ε_phys.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/states/encode_golden.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read the golden pairs"))
            .expect("JSON");
    let pair = json["pairs"]
        .as_array()
        .expect("pairs")
        .iter()
        .find(|p| p["name"] == "burrau_345")
        .expect("the Burrau pair is listed");
    let fz: [f64; 8] = std::array::from_fn(|k| pair["z"][k].as_f64().expect("a number"));
    let r = qa_phys(&widen(&dec::<f64>(z).ic), &widen(&dec::<f64>(fz).ic));
    assert!(
        r <= eps_phys::<f64>(),
        "the fixture's z decodes {r:.3e} from Part 3's z"
    );
    let fm: [f64; 3] = std::array::from_fn(|i| pair["m"][i].as_f64().expect("a number"));
    assert!(
        qa_phys(
            &burrau_345(),
            &Ic {
                m: fm,
                ..burrau_345()
            }
        ) <= eps_phys::<f64>(),
        "the fixture's masses are not (5, 4, 3)/12"
    );
}

negative_control!(
    encode_qa_burrau_345,
    "Part 3's z with z_μ1 moved by 10⁻⁶ is not E(x)",
    expected = "from D(z) of Part 3's z",
    {
        let mut z = closed_form_z(&burrau_345());
        z[6] += 1e-6;
        burrau_pair::<f64>(1.0, z);
    }
);

// ── q_max alone bounds the momenta; no per-body cap (REQ-DEC-029, REQ-ENC-004) ──────────────────────────────────

const QMAX_SAMPLES: u32 = 3000;

/// The rejected cap (philosophy §8.2, R-11): every body's `|p|` capped at `q_max`, then the total momentum re-imposed.
#[cfg(feature = "controls")]
fn capped(x: Ic<f64>) -> Ic<f64> {
    let mut y = x;
    for v in &mut y.p {
        let n = len(*v);
        if n > Q_MAX {
            *v = [v[0] * Q_MAX / n, v[1] * Q_MAX / n];
        }
    }
    let pt = [
        y.p.iter().map(|v| v[0]).sum::<f64>(),
        y.p.iter().map(|v| v[1]).sum::<f64>(),
    ];
    for i in 0..3 {
        y.p[i] = [y.p[i][0] - y.m[i] * pt[0], y.p[i][1] - y.m[i] * pt[1]];
    }
    y
}

/// Random `z` at `R`, the momentum controls on `[−20, 20]` (across their clamps at `|z| = ln((1 − ε_q)/ε_q) ≈ 13.8`),
/// the rest on `[−3, 3]`: the decoded momenta are the Jacobi map of `q = q_max·(2σ(z) − 1)` within `ε_phys` (nothing
/// after it), whatever a body's `|p|`; `|qₖ| ≤ q_max`; a momentum control past its clamp (Part 4: `min(s, 1 − s) ≤
/// ε_q/2`) sets its bit and one clear of it (`≥ 2ε_q`) leaves it clear; on a sample whose every control is clear
/// nothing is clamped and `‖D(E(D(z))) − D(z)‖_phys ≤ ε_phys`. Returns the samples with a body's `|p|` above `q_max`, and the clamped ones.
fn qmax_run<R: Precision>(alter: fn(Ic<f64>) -> Ic<f64>) -> (u32, u32) {
    let mut rng = Rng(0x0dec_0029);
    let eps = eps_phys::<R>();
    let (mut over, mut clamped) = (0, 0);
    for i in 0..QMAX_SAMPLES {
        let z: [f64; 8] = std::array::from_fn(|k| {
            if (2..6).contains(&k) {
                rng.uniform(-20.0, 20.0)
            } else {
                rng.uniform(-3.0, 3.0)
            }
        });
        let zr = z.map(R::of);
        let d = dec::<R>(zr);
        let x = alter(widen(&d.ic));
        let what = format!("f{} sample {i} z {z:?}", bits::<R>());
        // The Jacobi map of q, which is in the state's own frame: the decoded state is canonical, so its momenta are
        // the frame's. The configuration is rotated by C at most to rounding: q is read in that frame.
        let sig = |t: f64| 1.0 / (1.0 + (-t).exp());
        let q: [f64; 4] = std::array::from_fn(|k| Q_MAX * (2.0 * sig(zr[2 + k].into()) - 1.0));
        let m = x.m;
        let w = m[1] / (m[0] + m[1]);
        let pl = [q[2], q[3]];
        let p1 = [q[0] - w * pl[0], q[1] - w * pl[1]];
        let want = Ic {
            m,
            r: x.r,
            p: [[-p1[0] - pl[0], -p1[1] - pl[1]], p1, pl],
        };
        let r = qa_phys(&want, &x);
        assert!(r <= eps, "{what}: the decoded momenta are {r:.3e} from the Jacobi map of q: a cap or rescale acts");
        for (k, qk) in q.iter().enumerate() {
            assert!(
                qk.abs() <= Q_MAX,
                "{what}: |q{k}| = {} above q_max",
                qk.abs()
            );
        }
        if x.p.iter().any(|&v| len(v) > Q_MAX) {
            over += 1;
        }
        let e = encode::<Defaults, R>(&d.ic);
        // Part 4: `s` within `ε_q/2` of 0 or 1 is past its clamp and sets its bit; `s` in `[2ε_q, 1 − 2ε_q]` leaves it
        // clear. `s` read from `q` as Part 3 does, `s = ½(q/q_max + 1)`.
        let mut interior = true;
        for k in 2..6 {
            let sk = 0.5 * (q[k - 2] / Q_MAX + 1.0);
            let margin = sk.min(1.0 - sk);
            let bit = e.report.clamped_controls & (1 << k) != 0;
            if margin <= EPS_Q / 2.0 {
                assert!(bit, "{what}: control {k} past its clamp not flagged");
            }
            if margin >= 2.0 * EPS_Q {
                assert!(!bit, "{what}: control {k} inside its clamp flagged");
            } else {
                interior = false;
            }
        }
        if e.report.lookup_clamped() {
            clamped += 1;
        }
        if interior {
            assert!(
                !e.report.lookup_clamped(),
                "{what}: an interior sample is flagged"
            );
            let back = widen(&dec::<R>(e.z.z).ic);
            let r = qa_phys(&widen(&d.ic), &back);
            assert!(
                r <= eps,
                "{what}: T2 residual {r:.3e} beyond ε_phys {eps:.3e}"
            );
        }
    }
    (over, clamped)
}

#[test]
fn t2_qa_qmax_no_cap() {
    for (n, w) in [(qmax_run::<f64>(|x| x), 64), (qmax_run::<f32>(|x| x), 32)] {
        assert!(
            n.0 >= QMAX_SAMPLES / 10 && n.1 >= QMAX_SAMPLES / 10,
            "f{w}: {} samples with a body above q_max and {} clamped: the boundary is not exercised",
            n.0,
            n.1
        );
    }
}

negative_control!(
    t2_qa_qmax_no_cap,
    "the rejected per-body cap with CoM re-enforcement must fail the Jacobi-map check",
    expected = "a cap or rescale acts",
    {
        qmax_run::<f64>(capped);
    }
);

// ── Conditioning: physical, never z (REQ-VAL-013) ────────────────────────────────────────────────────────────────

/// `z` with `z_q0 = t`, near its clamp, the other controls fixed inside: at `R`, `1/(s(1 − s))` is ~10⁵ (between
/// 10^4.5 and 10^5.5, the input's definition), the sample is unclamped, its physical residual is within `ε_phys` and its
/// `z` residual is returned. With `inject`, encode's `z` is moved by 10⁻⁶ in `z_β` first.
fn near_clamp<R: Precision>(t: f64, inject: bool) -> f64 {
    let eps = eps_phys::<R>();
    let z = [0.4, -0.3, t, 0.2, -0.5, 0.3, 0.1, -0.2];
    let s = 1.0 / (1.0 + (-t).exp());
    let gain = 1.0 / (s * (1.0 - s));
    assert!(
        (10f64.powf(4.5)..=10f64.powf(5.5)).contains(&gain),
        "the input's d logit/ds {gain:e} is not ~10⁵"
    );
    let zr = z.map(R::of);
    let x = dec::<R>(zr).ic;
    let e = encode::<Defaults, R>(&x);
    assert!(
        !e.report.lookup_clamped(),
        "f{} z_q0 = {t}: flagged inside the clamp",
        bits::<R>()
    );
    let mut ze = e.z.z;
    if inject {
        ze[1] = R::of(Into::<f64>::into(ze[1]) + 1e-6);
    }
    let back = widen(&dec::<R>(ze).ic);
    let r = qa_phys(&widen(&x), &back);
    assert!(
        r <= eps,
        "f{} z_q0 = {t}: the physical residual {r:.3e} exceeds ε_phys {eps:.3e}",
        bits::<R>()
    );
    (0..8)
        .map(|k| (Into::<f64>::into(ze[k]) - Into::<f64>::into(zr[k])).abs())
        .fold(0.0, f64::max)
}

fn conditioning<R: Precision>(inject: bool) {
    let eps = eps_phys::<R>();
    let mut naive_fails = 0;
    let ts: Vec<f64> = (0..41).map(|k| 10.6 + 0.05 * k as f64).collect();
    for &t in &ts {
        for sign in [1.0, -1.0] {
            if near_clamp::<R>(sign * t, inject) > eps {
                naive_fails += 1;
            }
        }
    }
    assert!(
        naive_fails >= ts.len(),
        "f{}: a naive z tolerance fails on only {naive_fails} of {} near-clamp inputs",
        bits::<R>(),
        2 * ts.len()
    );
}

#[test]
fn conditioning_qa_physical_space() {
    conditioning::<f64>(false);
    conditioning::<f32>(false);
}

negative_control!(
    conditioning_qa_physical_space,
    "an encode error of 10⁻⁶ in z_β must fail the physical tolerance",
    expected = "the physical residual",
    conditioning::<f64>(true)
);
