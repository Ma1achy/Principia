//! QA's tests for TASK-M3-08 (REQ-INT-121): Heggie's regularised system built to integrator_contract Part 2b § "Heggie:
//! the equations, the time transformations, the maps and the step control (transcribed)", § "The time-reversal maps
//! (REQ-INT-095)", and Part 1 § "COM projection and invariant monitoring" (R-160, R-161, R-566); and the energies
//! Aarseth–Zare's march reports (R-566).
//!
//! Written from the requirement and the contract, not from the implementation. Every anchor is computed here, outside
//! the code under test:
//! - `qa_m308_round_trip`: random Cartesian states at unequal masses, registered and returned, give back the state; the
//!   relative vectors `qᵢ = rⱼ − rₖ`, `Rᵢ = |Qᵢ|² = |qᵢ|`, Eq. 8b's `pᵢ = (mⱼvⱼ − mₖvₖ)/3`, the clock at `0`, the
//!   registered `h` and the regularised energy against the Cartesian energy computed here; control: a quarter-turned `Q₀`.
//! - `qa_m308_energy_off_registration`: at momenta other than the registered ones, the regularised energy is the
//!   Cartesian energy of the image, and Eq. 7's relative velocities are the image's `vⱼ − vₖ`; control: the momenta of
//!   the image built with two masses swapped.
//! - `qa_m308_gamma_anchor`: `Γ* = (E − h)·R₀R₁R₂`, `E` the Cartesian energy of the image computed here, at an `h` off
//!   the energy, and `Γ* = 0` at registration; control: two masses swapped in the system.
//! - `qa_m308_rates_fd`: the rates under Eq. 20 and Eq. 22 against a central difference, step `1e-6`, within `1e-8`, of
//!   the Hamiltonian transcribed here from Part 2b (`Γ*`, `Γ** = Γ*·S^{−3/2}`), and `dt/dτ = −∂/∂h`; controls: a momentum
//!   rate's sign flipped, and Eq. 22's rates read against `Γ*` (the control term dropped).
//! - `qa_m308_dt_dtau`: `dt/dτ` under each transformation, unfloored itself, and `S` floored at `TINY`; control: Eq. 20's
//!   `dt/dτ` read for Eq. 22's.
//! - `qa_m308_floors`: the return and energy finite at `Rᵢ = 0`, `|qᵢ| = 0`; control: `TINY = 0`.
//! - `qa_m308_reversal`: the time-reversal map is an involution, keeps `Q`, negates `P` and `t`, and reverses the
//!   equations (`Q̇` odd in `P`, `Ṗ` and `ṫ` even), its image the ordinary time reversal; control: a map negating `Q`.
//! - `qa_m308_flow_sums`: the flow's sums on known values, the clock included; and `rates` equals `(q_rate, p_rate)`;
//!   control: a weighted sum with the weights `1, 1, 1, 1`.
//! - `qa_m308_default_eq22`: the default time transformation is Eq. 22 (R-161); control: Eq. 20 as the default.
//! - `qa_m308_az_energy_reported`: Aarseth–Zare's march reports its first registration's `E` bitwise and the
//!   regularised energy where it ended, against the Cartesian energy of the end state computed here; `0`, `0` where no
//!   interval was marched; control: the end state's registration read for the first.
//!
//! The comparisons other than the finite difference are identities, checked at f64 round-off (`ROUND`, a few hundred
//! ulps on O(1) quantities); the finite difference's step and tolerance are REQ-INT-121's.

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{self, AzCount, AzParams, ExplicitTest};
use kernel::regularisation::form::Flow;
use kernel::regularisation::heggie::{register, reversed, HgP, HgQ, HgSystem, DEFAULT_TIME};
use kernel::regularisation::pairs::HeggieTime;
use validation::negative_control;

/// The f64 round-off bound for the identities (relative, on O(1) quantities).
const ROUND: f64 = 1e-12;
/// REQ-INT-121's finite-difference step.
const FD_STEP: f64 = 1e-6;
/// REQ-INT-121's finite-difference tolerance.
const FD_TOL: f64 = 1e-8;
/// A floor far below every separation drawn here, so it never acts outside the floor test.
const TINY: f64 = 1e-300;

// ── Independent helpers ──────────────────────────────────────────────────────────────────────────────────────────

/// A xorshift generator, so the draws are fixed.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn in_range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// A random planar three-body state at unequal masses in `[0.5, 2]`, moved to its centre of mass and momentum, its
/// bodies at least `0.3` apart.
fn random_state(rng: &mut Rng) -> State<f64> {
    loop {
        let m = [0, 1, 2].map(|_| rng.in_range(0.5, 2.0));
        let r = [0, 1, 2].map(|_| [rng.in_range(-1.0, 1.0), rng.in_range(-1.0, 1.0)]);
        let v = [0, 1, 2].map(|_| [rng.in_range(-0.6, 0.6), rng.in_range(-0.6, 0.6)]);
        let total: f64 = m.iter().sum();
        let rc = [0, 1].map(|c| (0..3).map(|i| m[i] * r[i][c]).sum::<f64>() / total);
        let vc = [0, 1].map(|c| (0..3).map(|i| m[i] * v[i][c]).sum::<f64>() / total);
        let s = State {
            m,
            r: [0, 1, 2].map(|i| [0, 1].map(|c| r[i][c] - rc[c])),
            p: [0, 1, 2].map(|i| [0, 1].map(|c| m[i] * (v[i][c] - vc[c]))),
        };
        if [(0, 1), (0, 2), (1, 2)]
            .iter()
            .all(|&(i, j)| dist(s.r[i], s.r[j]) > 0.3)
        {
            return s;
        }
    }
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The Cartesian energy of `s`, `G = 1` (Part 1's `E`).
fn cartesian_energy(s: &State<f64>) -> f64 {
    let kinetic: f64 = (0..3)
        .map(|i| (s.p[i][0] * s.p[i][0] + s.p[i][1] * s.p[i][1]) / (2.0 * s.m[i]))
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| s.m[i] * s.m[j] / dist(s.r[i], s.r[j]))
        .sum();
    kinetic - potential
}

/// The potential's magnitude `Σ mᵢmⱼ/rᵢⱼ` of `s`.
fn potential(s: &State<f64>) -> f64 {
    [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| s.m[i] * s.m[j] / dist(s.r[i], s.r[j]))
        .sum()
}

/// `(j, k)`, the cyclic successors of `i`.
fn jk(i: usize) -> (usize, usize) {
    ((i + 1) % 3, (i + 2) % 3)
}

/// The largest component difference between two states, relative to the larger of `1` and the component.
fn state_gap(a: &State<f64>, b: &State<f64>) -> f64 {
    let mut gap = 0.0f64;
    for i in 0..3 {
        for c in 0..2 {
            gap = gap.max((a.r[i][c] - b.r[i][c]).abs() / a.r[i][c].abs().max(1.0));
            gap = gap.max((a.p[i][c] - b.p[i][c]).abs() / a.p[i][c].abs().max(1.0));
        }
    }
    gap
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1.0)
}

/// `L(u)w`, the Levi-Civita matrix product, written here from the contract.
fn l_mul(u: [f64; 2], w: [f64; 2]) -> [f64; 2] {
    [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]]
}

/// The flat vector `(Q₀, P₀, Q₁, P₁, Q₂, P₂)`: 12 numbers.
fn flatten(q: &HgQ<f64>, p: &HgP<f64>) -> [f64; 12] {
    let mut x = [0.0; 12];
    for i in 0..3 {
        x[4 * i] = q.q[i][0];
        x[4 * i + 1] = q.q[i][1];
        x[4 * i + 2] = p.p[i][0];
        x[4 * i + 3] = p.p[i][1];
    }
    x
}

fn unflatten(x: &[f64; 12], t: f64) -> (HgQ<f64>, HgP<f64>) {
    (
        HgQ {
            q: [0, 1, 2].map(|i| [x[4 * i], x[4 * i + 1]]),
            t,
        },
        HgP {
            p: [0, 1, 2].map(|i| [x[4 * i + 2], x[4 * i + 3]]),
        },
    )
}

/// `Γ*` transcribed here from Part 2b: `Σᵢ RⱼRₖ|Pᵢ|²/(8μᵢ) − Σᵢ Rᵢ Wⱼ·Wₖ/(4mᵢ) − Σᵢ mⱼmₖ RⱼRₖ − h·R₀R₁R₂`,
/// `Rᵢ = |Qᵢ|²`, `Wᵢ = L(Qᵢ)Pᵢ`, `μᵢ = mⱼmₖ/(mⱼ + mₖ)`.
fn gamma_star(x: &[f64; 12], m: [f64; 3], h: f64) -> f64 {
    let q = |i: usize| [x[4 * i], x[4 * i + 1]];
    let p = |i: usize| [x[4 * i + 2], x[4 * i + 3]];
    let rr = |i: usize| q(i)[0] * q(i)[0] + q(i)[1] * q(i)[1];
    let w = |i: usize| l_mul(q(i), p(i));
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let mut g = 0.0;
    for i in 0..3 {
        let (j, k) = jk(i);
        let mu = m[j] * m[k] / (m[j] + m[k]);
        g += rr(j) * rr(k) * dot(p(i), p(i)) / (8.0 * mu);
        g -= rr(i) * dot(w(j), w(k)) / (4.0 * m[i]);
        g -= m[j] * m[k] * rr(j) * rr(k);
    }
    g - h * rr(0) * rr(1) * rr(2)
}

/// The Hamiltonian of the time transformation: `Γ*` under Eq. 20, `Γ*·S^{−3/2}` under Eq. 22 (Part 2b).
fn hamiltonian(x: &[f64; 12], m: [f64; 3], h: f64, time: HeggieTime) -> f64 {
    let g = gamma_star(x, m, h);
    match time {
        HeggieTime::Eq20 => g,
        HeggieTime::Eq22 => {
            let s: f64 = (0..3)
                .map(|i| x[4 * i].powi(2) + x[4 * i + 1].powi(2))
                .sum();
            g * s.powf(-1.5)
        }
    }
}

// ── The round trip and the energy at registration ───────────────────────────────────────────────────────────────

/// Registers random states and returns them; `turn` quarter-turns `Q₀` before the return (`Q → (−Q₁, Q₀)` turns `q`
/// by a half turn, so the return differs).
fn check_round_trip(turn: bool) {
    let mut rng = Rng(0x5eed_08a1);
    for _ in 0..200 {
        let s = random_state(&mut rng);
        for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
            let reg = register(&s, TINY, time);
            let mut q = reg.q;
            if turn {
                q.q[0] = [-q.q[0][1], q.q[0][0]];
            }
            assert_eq!(reg.q.t, 0.0, "the clock is not 0 at registration");
            assert_eq!(
                reg.system.time, time,
                "the time transformation is not the one asked"
            );
            let back = reg.system.image(&q, &reg.p);
            assert_eq!(back.m, s.m, "the image's masses are not the state's");
            let gap = state_gap(&s, &back);
            assert!(gap < ROUND, "round trip gap {gap:e}");

            // The relative vectors and momenta (Part 2b, Eqs. 4, 8b), and Rᵢ = |Qᵢ|² = |qᵢ|.
            let (qv, pv) = reg.system.phys(&q, &reg.p);
            for i in 0..3 {
                let (j, k) = jk(i);
                for c in 0..2 {
                    let want_q = s.r[j][c] - s.r[k][c];
                    assert!(rel(qv[i][c], want_q) < ROUND, "q{i} is not r{j} − r{k}");
                    let want_p = (s.p[j][c] - s.p[k][c]) / 3.0;
                    assert!(rel(pv[i][c], want_p) < ROUND, "p{i} is not Eq. 8b's");
                }
                let ri = q.q[i][0].powi(2) + q.q[i][1].powi(2);
                assert!(rel(ri, dist(s.r[j], s.r[k])) < ROUND, "R{i} is not |q{i}|");
            }

            // The registered h and the regularised energy are the Cartesian energy (Part 1, R-566).
            let e = cartesian_energy(&s);
            assert!(
                rel(reg.system.h, e) < ROUND,
                "h {} is not the Cartesian energy {e}",
                reg.system.h
            );
            let e_reg = reg.system.energy(&q, &reg.p);
            assert!(
                rel(e_reg, e) < ROUND,
                "regularised energy {e_reg} is not the Cartesian {e}"
            );
        }
    }
}

#[test]
fn qa_m308_round_trip() {
    check_round_trip(false);
}

negative_control!(
    qa_m308_round_trip,
    "Q₀ quarter-turned before the return",
    expected = "round trip gap",
    check_round_trip(true)
);

// ── Off registration: the energy and Eq. 7's velocities against the image ──────────────────────────────────────

/// At the registered `Q` with random momenta `P`, the regularised energy is the Cartesian energy of the image, and
/// Eq. 7's relative velocities of the returned `pᵢ` are the image's `vⱼ − vₖ`. `swap` reads the image's velocities
/// with two masses swapped.
fn check_off_registration(swap: bool) {
    let mut rng = Rng(0x0ff_5e7);
    for _ in 0..200 {
        let s = random_state(&mut rng);
        let reg = register(&s, TINY, DEFAULT_TIME);
        let p = HgP {
            p: [0, 1, 2].map(|_| [rng.in_range(-2.0, 2.0), rng.in_range(-2.0, 2.0)]),
        };
        let image = reg.system.image(&reg.q, &p);
        // The image is in the centre-of-mass frame of momentum.
        for c in 0..2 {
            let total: f64 = (0..3).map(|i| image.p[i][c]).sum();
            assert!(total.abs() < ROUND, "the image's momenta do not sum to 0");
        }
        let e = cartesian_energy(&image);
        let e_reg = reg.system.energy(&reg.q, &p);
        assert!(
            rel(e_reg, e) < ROUND,
            "energy {e_reg} off registration is not the image's {e}"
        );

        let (_, pv) = reg.system.phys(&reg.q, &p);
        let qdot = reg.system.q_dot(&pv);
        let mut m = image.m;
        if swap {
            m.swap(0, 2);
        }
        for (i, qdot_i) in qdot.iter().enumerate() {
            let (j, k) = jk(i);
            for (c, &got) in qdot_i.iter().enumerate() {
                let want = image.p[j][c] / m[j] - image.p[k][c] / m[k];
                assert!(
                    rel(got, want) < ROUND,
                    "Eq. 7's q̇{i} {} is not the image's v{j} − v{k} {want}",
                    got
                );
            }
        }
    }
}

#[test]
fn qa_m308_energy_off_registration() {
    check_off_registration(false);
}

negative_control!(
    qa_m308_energy_off_registration,
    "the image's velocities read with two masses swapped",
    expected = "Eq. 7's",
    check_off_registration(true)
);

// ── Γ* = (E − h)·R₀R₁R₂ ───────────────────────────────────────────────────────────────────────────────────────────

/// `Γ*` of the system at random `P` and an `h` off the energy is `(E − h)·R₀R₁R₂`, `E` the Cartesian energy of the
/// image; at registration (`h = E`), `Γ* = 0`. `swap` builds the system with `m₀` and `m₁` swapped.
fn check_gamma(swap: bool) {
    let mut rng = Rng(0x6a33a);
    for _ in 0..200 {
        let s = random_state(&mut rng);
        let reg = register(&s, TINY, HeggieTime::Eq20);
        let scale: f64 = (0..3)
            .map(|i| reg.q.q[i][0].powi(2) + reg.q.q[i][1].powi(2))
            .product();
        let g0 = reg.system.gamma(&reg.q, &reg.p);
        // E = T − V cancels, so round-off is on the scale of T + V.
        let t_plus_v = cartesian_energy(&s) + 2.0 * potential(&s);
        assert!(
            g0.abs() < ROUND * scale * t_plus_v.max(1.0),
            "Γ* {g0:e} is not 0 at registration"
        );

        let h = rng.in_range(-3.0, 1.0);
        let mut m = s.m;
        if swap {
            m.swap(0, 1);
        }
        let system = HgSystem::new(m, h, TINY, HeggieTime::Eq20);
        let p = HgP {
            p: [0, 1, 2].map(|_| [rng.in_range(-2.0, 2.0), rng.in_range(-2.0, 2.0)]),
        };
        let image = reg.system.image(&reg.q, &p);
        let want = (cartesian_energy(&image) - h) * scale;
        let got = system.gamma(&reg.q, &p);
        assert!(
            rel(got, want) < ROUND,
            "Γ* {got} is not (E − h)·R₀R₁R₂ {want}"
        );
        // The transcription the finite difference reads is the same Γ*.
        let mine = gamma_star(&flatten(&reg.q, &p), s.m, h);
        assert!(
            rel(mine, want) < ROUND,
            "the test's Γ* {mine} is not (E − h)·R₀R₁R₂ {want}"
        );
    }
}

#[test]
fn qa_m308_gamma_anchor() {
    check_gamma(false);
}

negative_control!(
    qa_m308_gamma_anchor,
    "two masses swapped in the system",
    expected = "is not (E − h)·R₀R₁R₂",
    check_gamma(true)
);

// ── The rates against a central difference of the Hamiltonian ───────────────────────────────────────────────────

/// How a control corrupts the comparison.
#[derive(Clone, Copy, PartialEq)]
enum Fd {
    /// The comparison as the contract gives it.
    Honest,
    /// The momentum rates compared with `+∂H/∂Q`.
    SignFlipped,
    /// Eq. 22's rates compared with `Γ*`'s derivatives scaled by `S^{−3/2}`, the `3Γ*Qᵢ/S` term dropped.
    ControlTermDropped,
}

/// Over random states off the energy shell (random `Q`, `P`, `h`, masses), each rate of the time transformation
/// against the central difference of its Hamiltonian at `FD_STEP`, within `FD_TOL`: `dQ/dτ = ∂H/∂P`, `dP/dτ = −∂H/∂Q`,
/// `dt/dτ = −∂H/∂h`, and `dt/dτ` from [`HgSystem::dt_dtau`] the same.
fn check_rates(time: HeggieTime, how: Fd) {
    let mut rng = Rng(0xfd_2026);
    let mut worst = 0.0f64;
    for _ in 0..300 {
        let m = [0, 1, 2].map(|_| rng.in_range(0.5, 2.0));
        let h = rng.in_range(-2.0, 0.5);
        let mut x = [0.0; 12];
        for v in x.iter_mut() {
            *v = rng.in_range(-1.0, 1.0);
        }
        let t = rng.in_range(-1.0, 1.0);
        let (q, p) = unflatten(&x, t);
        let system = HgSystem::new(m, h, TINY, time);
        let (dq, dp) = system.rates_of(&q, &p);
        let analytic = flatten(&dq, &dp);
        // The Hamiltonian the comparison reads.
        let ham = |y: &[f64; 12], hh: f64| match how {
            Fd::ControlTermDropped => hamiltonian(y, m, hh, HeggieTime::Eq20),
            _ => hamiltonian(y, m, hh, time),
        };
        let s: f64 = (0..3)
            .map(|i| x[4 * i].powi(2) + x[4 * i + 1].powi(2))
            .sum();
        let post = if how == Fd::ControlTermDropped && time == HeggieTime::Eq22 {
            s.powf(-1.5)
        } else {
            1.0
        };
        for n in 0..12 {
            let mut a = x;
            let mut b = x;
            a[n] += FD_STEP;
            b[n] -= FD_STEP;
            let d = post * (ham(&a, h) - ham(&b, h)) / (2.0 * FD_STEP);
            // n % 4 ∈ {0, 1}: a Q component, whose −∂H/∂Q is the P rate at n + 2; {2, 3}: a P component, whose ∂H/∂P is
            // the Q rate at n − 2.
            let (rate_index, want) = if n % 4 < 2 {
                let sign = if how == Fd::SignFlipped { 1.0 } else { -1.0 };
                (n, sign * d)
            } else {
                (n, d)
            };
            let got = if n % 4 < 2 {
                analytic[rate_index + 2]
            } else {
                analytic[rate_index - 2]
            };
            let gap = (got - want).abs();
            worst = worst.max(gap);
            assert!(
                gap < FD_TOL,
                "{time:?}: rate of component {n} {got:e} against the finite difference {want:e}, gap {gap:e}"
            );
        }
        // dt/dτ = −∂H/∂h.
        let dh = -(ham(&x, h + FD_STEP) - ham(&x, h - FD_STEP)) / (2.0 * FD_STEP) * post;
        assert!(
            (dq.t - dh).abs() < FD_TOL,
            "{time:?}: dt/dτ {} is not −∂H/∂h {dh}",
            dq.t
        );
        // The same quantity, computed two ways (Eq. 22's `R₀R₁R₂·(1/S^{3/2})` and `R₀R₁R₂/S^{3/2}`), so within
        // round-off, not bitwise.
        assert!(
            rel(dq.t, system.dt_dtau(&q)) < ROUND,
            "the rate of t is not dt_dtau"
        );
    }
    println!("{time:?}: worst rate against the finite difference {worst:e} (tolerance {FD_TOL:e})");
}

#[test]
fn qa_m308_rates_fd() {
    check_rates(HeggieTime::Eq20, Fd::Honest);
    check_rates(HeggieTime::Eq22, Fd::Honest);
}

negative_control!(
    qa_m308_rates_fd,
    "a momentum rate's sign flipped",
    expected = "against the finite difference",
    check_rates(HeggieTime::Eq20, Fd::SignFlipped)
);

#[cfg(feature = "controls")]
mod qa_m308_rates_fd_eq22_control_term {
    use super::*;
    negative_control!(
        qa_m308_rates_fd,
        "Eq. 22's rates read against Γ*'s derivatives, the 3Γ*Qᵢ/S term dropped",
        expected = "Eq22: rate of component",
        check_rates(HeggieTime::Eq22, Fd::ControlTermDropped)
    );
}

// ── dt/dτ and the floors at TINY ───────────────────────────────────────────────────────────────────────────────

/// `dt/dτ` at known `Rᵢ`: Eq. 20's `R₀R₁R₂`, Eq. 22's `R₀R₁R₂/S^{3/2}`, unfloored itself even when tiny, and `S`
/// floored at `TINY`. `eq22` is the transformation read as Eq. 22's (the control's is Eq. 20).
fn check_dt_dtau(eq22: HeggieTime) {
    // Rᵢ = |Qᵢ|² = 1, 4, 0.25: S = 5.25.
    let q = HgQ {
        q: [[0.6, 0.8], [0.0, 2.0], [0.3, -0.4]],
        t: 0.0,
    };
    let product = 1.0 * 4.0 * 0.25;
    let eq20 = HgSystem::new([1.0, 2.0, 3.0], -1.0, TINY, HeggieTime::Eq20);
    assert!(
        rel(eq20.dt_dtau(&q), product) < ROUND,
        "Eq. 20's dt/dτ is not R₀R₁R₂"
    );
    let sys22 = HgSystem::new([1.0, 2.0, 3.0], -1.0, TINY, eq22);
    let want = product / 5.25f64.powf(1.5);
    assert!(
        rel(sys22.dt_dtau(&q), want) < ROUND,
        "Eq. 22's dt/dτ {} is not R₀R₁R₂/S^(3/2) {want}",
        sys22.dt_dtau(&q)
    );

    // Near a triple collision: S = 3·1e-8 below TINY = 1e-6 is floored; dt/dτ itself is not.
    let small = HgQ {
        q: [[1e-4, 0.0], [0.0, 1e-4], [1e-4, 0.0]],
        t: 0.0,
    };
    let tiny = 1e-6;
    let product = 1e-24;
    let eq20 = HgSystem::new([1.0, 2.0, 3.0], -1.0, tiny, HeggieTime::Eq20);
    assert!(
        rel(eq20.dt_dtau(&small), product) < ROUND,
        "Eq. 20's dt/dτ is floored"
    );
    let sys22 = HgSystem::new([1.0, 2.0, 3.0], -1.0, tiny, eq22);
    let want = product / tiny.powf(1.5);
    assert!(
        (sys22.dt_dtau(&small) - want).abs() < ROUND * want,
        "Eq. 22's dt/dτ {:e} does not floor S at TINY ({want:e})",
        sys22.dt_dtau(&small)
    );
}

#[test]
fn qa_m308_dt_dtau() {
    check_dt_dtau(HeggieTime::Eq22);
}

negative_control!(
    qa_m308_dt_dtau,
    "Eq. 20's dt/dτ read for Eq. 22's",
    expected = "Eq. 22's dt/dτ",
    check_dt_dtau(HeggieTime::Eq20)
);

/// At `Q₀ = 0` (`R₀ = 0`, `|q₀| = 0`) and at all `Q = 0` (`S = 0`), the return, the energy and the rates under both
/// transformations are finite with `TINY = tiny`: `Rᵢ` and `|qᵢ|` are floored where they divide, `S` where it does.
fn check_floors(tiny: f64) {
    let p = HgP {
        p: [[0.3, -0.2], [0.1, 0.5], [-0.4, 0.2]],
    };
    let one_zero = HgQ {
        q: [[0.0, 0.0], [0.5, 0.7], [-0.6, 0.2]],
        t: 0.0,
    };
    let all_zero = HgQ {
        q: [[0.0, 0.0]; 3],
        t: 0.0,
    };
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let system = HgSystem::new([1.0, 0.8, 1.3], -0.7, tiny, time);
        for q in [one_zero, all_zero] {
            let (qv, pv) = system.phys(&q, &p);
            let finite = |v: &[[f64; 2]; 3]| v.iter().flatten().all(|x| x.is_finite());
            assert!(
                finite(&qv) && finite(&pv),
                "{time:?}: the return is not finite where Rᵢ = 0"
            );
            let e = system.energy(&q, &p);
            assert!(
                e.is_finite(),
                "{time:?}: the energy is not finite where |qᵢ| = 0"
            );
            let (dq, dp) = system.rates_of(&q, &p);
            let x = flatten(&dq, &dp);
            assert!(
                x.iter().all(|v| v.is_finite()) && dq.t.is_finite(),
                "{time:?}: the rates are not finite where S = 0"
            );
            assert!(
                system.dt_dtau(&q).is_finite(),
                "{time:?}: dt/dτ is not finite where S = 0"
            );
        }
        // Where Q₀ = 0 the return of p₀ is L(0)P₀/(2·TINY) = 0, and the energy's potential term m₁m₂/TINY.
        let (_, pv) = system.phys(&one_zero, &p);
        assert_eq!(pv[0], [0.0, 0.0], "p₀ is not 0 where Q₀ = 0");
    }
}

#[test]
fn qa_m308_floors() {
    check_floors(1e-12);
}

negative_control!(
    qa_m308_floors,
    "TINY = 0, no floor",
    expected = "is not finite where",
    check_floors(0.0)
);

// ── The time-reversal map ──────────────────────────────────────────────────────────────────────────────────────

/// `map` is the time-reversal map (Part 2b, REQ-INT-095): an involution, `Q` kept, `P` and `t` negated, bitwise; the
/// equations reversed under it (`Q̇(Q, −P) = −Q̇(Q, P)`, `Ṗ(Q, −P) = Ṗ(Q, P)`, `ṫ` unchanged), under both
/// transformations; and its image the ordinary time reversal, positions kept and Cartesian momenta negated.
fn check_reversal(map: impl Fn(&HgQ<f64>, &HgP<f64>) -> (HgQ<f64>, HgP<f64>)) {
    let mut rng = Rng(0x007e_7e5e);
    for _ in 0..100 {
        let s = random_state(&mut rng);
        let reg = register(&s, TINY, DEFAULT_TIME);
        let q = HgQ {
            t: rng.in_range(0.1, 2.0),
            ..reg.q
        };
        let (rq, rp) = map(&q, &reg.p);
        let (qq, pp) = map(&rq, &rp);
        assert_eq!((qq, pp), (q, reg.p), "the map is not an involution");
        for i in 0..3 {
            for c in 0..2 {
                assert_eq!(rq.q[i][c].to_bits(), q.q[i][c].to_bits(), "Q is not kept");
                assert_eq!(
                    rp.p[i][c].to_bits(),
                    (-reg.p.p[i][c]).to_bits(),
                    "P is not negated"
                );
            }
        }
        assert_eq!(rq.t.to_bits(), (-q.t).to_bits(), "t is not negated");

        for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
            let system = HgSystem { time, ..reg.system };
            let (dq, dp) = system.rates_of(&q, &reg.p);
            let (dq_r, dp_r) = system.rates_of(&rq, &rp);
            for i in 0..3 {
                for c in 0..2 {
                    assert!(
                        (dq_r.q[i][c] + dq.q[i][c]).abs() <= ROUND * dq.q[i][c].abs().max(1.0),
                        "{time:?}: Q̇ is not reversed by the map"
                    );
                    assert!(
                        (dp_r.p[i][c] - dp.p[i][c]).abs() <= ROUND * dp.p[i][c].abs().max(1.0),
                        "{time:?}: Ṗ is not kept by the map"
                    );
                }
            }
            assert_eq!(
                dq_r.t.to_bits(),
                dq.t.to_bits(),
                "{time:?}: ṫ changed under the map"
            );
        }

        let a = reg.system.image(&q, &reg.p);
        let b = reg.system.image(&rq, &rp);
        for i in 0..3 {
            for c in 0..2 {
                assert!(
                    rel(b.r[i][c], a.r[i][c]) < ROUND,
                    "the reversed image moved a body"
                );
                assert!(
                    rel(b.p[i][c], -a.p[i][c]) < ROUND,
                    "the reversed image's momenta are not negated"
                );
            }
        }
    }
}

#[test]
fn qa_m308_reversal() {
    check_reversal(reversed);
}

negative_control!(
    qa_m308_reversal,
    "a map negating Q and t, keeping P",
    expected = "is not",
    check_reversal(|q: &HgQ<f64>, p: &HgP<f64>| (
        HgQ {
            q: q.q.map(|v| [-v[0], -v[1]]),
            t: -q.t
        },
        *p
    ))
);

// ── The flow's sums and rates ─────────────────────────────────────────────────────────────────────────────────

fn hq(base: f64) -> HgQ<f64> {
    HgQ {
        q: [
            [base, base + 1.0],
            [base + 2.0, base + 3.0],
            [base + 4.0, base + 5.0],
        ],
        t: base + 6.0,
    }
}

fn hp(base: f64) -> HgP<f64> {
    HgP {
        p: [
            [base, base + 1.0],
            [base + 2.0, base + 3.0],
            [base + 4.0, base + 5.0],
        ],
    }
}

/// The flow's sums on small integers (exact in f64): `q + h·dq` and `p + h·dp` by component, the clock among `Q`;
/// `k₁ + 2k₂ + 2k₃ + k₄` by component with `two` the weight. `two` is the weight passed (the control's is `1`).
fn check_flow(two: f64) {
    let a = HgSystem::<f64>::q_add(&hq(1.0), 0.5, &hq(10.0));
    let exp: [f64; 7] = core::array::from_fn(|n| (1.0 + n as f64) + 0.5 * (10.0 + n as f64));
    let got = [
        a.q[0][0], a.q[0][1], a.q[1][0], a.q[1][1], a.q[2][0], a.q[2][1], a.t,
    ];
    assert_eq!(got, exp, "q_add is not q + h·dq by component");
    let b = HgSystem::<f64>::p_add(&hp(1.0), -2.0, &hp(3.0));
    let gotp = [
        b.p[0][0], b.p[0][1], b.p[1][0], b.p[1][1], b.p[2][0], b.p[2][1],
    ];
    let expp: [f64; 6] = core::array::from_fn(|n| (1.0 + n as f64) - 2.0 * (3.0 + n as f64));
    assert_eq!(gotp, expp, "p_add is not p + h·dp by component");

    let (k1, k2, k3, k4) = (hq(1.0), hq(10.0), hq(100.0), hq(1000.0));
    let w = HgSystem::<f64>::q_weighted(two, [&k1, &k2, &k3, &k4]);
    let gotw = [
        w.q[0][0], w.q[0][1], w.q[1][0], w.q[1][1], w.q[2][0], w.q[2][1], w.t,
    ];
    let expw: [f64; 7] = core::array::from_fn(|n| {
        let n = n as f64;
        (1.0 + n) + 2.0 * (10.0 + n) + 2.0 * (100.0 + n) + (1000.0 + n)
    });
    assert_eq!(gotw, expw, "q_weighted is not k₁ + 2k₂ + 2k₃ + k₄");
    let (l1, l2, l3, l4) = (hp(1.0), hp(10.0), hp(100.0), hp(1000.0));
    let wp = HgSystem::<f64>::p_weighted(two, [&l1, &l2, &l3, &l4]);
    let gotwp = [
        wp.p[0][0], wp.p[0][1], wp.p[1][0], wp.p[1][1], wp.p[2][0], wp.p[2][1],
    ];
    let expwp: [f64; 6] = core::array::from_fn(|n| {
        let n = n as f64;
        (1.0 + n) + 2.0 * (10.0 + n) + 2.0 * (100.0 + n) + (1000.0 + n)
    });
    assert_eq!(gotwp, expwp, "p_weighted is not k₁ + 2k₂ + 2k₃ + k₄");

    // The flow's rates are the system's, both together and apart.
    let mut rng = Rng(0xf10e);
    let s = random_state(&mut rng);
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let reg = register(&s, TINY, time);
        let (dq, dp) = reg.system.rates_of(&reg.q, &reg.p);
        assert_eq!(
            Flow::rates(&reg.system, &reg.q, &reg.p),
            (dq, dp),
            "rates is not rates_of"
        );
        assert_eq!(
            reg.system.q_rate(&reg.q, &reg.p),
            dq,
            "q_rate is not rates_of's"
        );
        assert_eq!(
            reg.system.p_rate(&reg.q, &reg.p),
            dp,
            "p_rate is not rates_of's"
        );
    }
}

#[test]
fn qa_m308_flow_sums() {
    check_flow(2.0);
}

negative_control!(
    qa_m308_flow_sums,
    "a weighted sum with the weights 1, 1, 1, 1",
    expected = "is not k₁ + 2k₂ + 2k₃ + k₄",
    check_flow(1.0)
);

// ── The default time transformation (R-161) ─────────────────────────────────────────────────────────────────────

fn check_default(want: HeggieTime) {
    assert_eq!(
        DEFAULT_TIME, want,
        "the default time transformation is not Eq. 22 (R-161)"
    );
}

#[test]
fn qa_m308_default_eq22() {
    check_default(HeggieTime::Eq22);
}

negative_control!(
    qa_m308_default_eq22,
    "Eq. 20 as the default",
    expected = "is not Eq. 22 (R-161)",
    check_default(HeggieTime::Eq20)
);

// ── Aarseth–Zare's reported energies (R-566) ─────────────────────────────────────────────────────────────────────

struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// Aarseth–Zare's step control at f64 at test values: the reference's `TINY` (R-953) and the landing bisection's 64
/// halvings (Part 2b, step 6), a budget that does not bind.
fn az_params() -> AzParams<f64> {
    AzParams {
        eta: 0.1,
        delta_sync: 0.1,
        tiny: 1e-300,
        predictive_f: 0.02,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
    }
}

/// How the reported energies are read.
#[derive(Clone, Copy, PartialEq)]
enum AzRead {
    /// As reported.
    Honest,
    /// The end state's registration read as the first's.
    EndAsFirst,
    /// `registered` read for `regularised`.
    RegisteredAsRegularised,
}

/// A march of random states from boundary 0 to 4 reports `registered` bitwise the `E` of the registration of its
/// starting state, and `regularised` the energy where it ended, which is the Cartesian energy of its end state within
/// round-off (`1e-11`, the image's cancellation included); a march of no interval reports `0`, `0`. The marches drift
/// by more than that tolerance, so a `regularised` that repeats the registered energy cannot pass.
fn check_az(read: AzRead) {
    let p = az_params();
    let mut rng = Rng(0xa2_e0);
    let mut landed = 0;
    let mut largest_drift = 0.0f64;
    for _ in 0..6 {
        let s0 = random_state(&mut rng);
        let out = az::march::<Rk4, f64, Quiet, ExplicitTest>(
            &s0,
            0,
            4,
            AzCount::new(),
            &p,
            &mut Quiet,
            &mut ExplicitTest,
        );
        if out.boundary != 4 {
            continue;
        }
        landed += 1;
        let first = if read == AzRead::EndAsFirst {
            out.state
        } else {
            s0
        };
        let (sys, _, _) = az::register(&first, p.tiny);
        assert_eq!(
            out.registered.to_bits(),
            sys.e.to_bits(),
            "registered {:e} is not the first registration's E {:e}",
            out.registered,
            sys.e
        );
        assert!(
            rel(out.registered, cartesian_energy(&s0)) < ROUND,
            "registered is not the starting state's Cartesian energy"
        );
        let regularised = if read == AzRead::RegisteredAsRegularised {
            out.registered
        } else {
            out.regularised
        };
        let e_end = cartesian_energy(&out.state);
        largest_drift = largest_drift.max(rel(e_end, out.registered));
        assert!(
            rel(regularised, e_end) < 1e-11,
            "regularised {regularised:e} is not the end state's energy {e_end:e}"
        );
    }
    println!("{landed} of 6 marches landed on 4; largest relative drift {largest_drift:e}");
    assert!(landed >= 4, "only {landed} of 6 marches landed");
    assert!(
        largest_drift > 1e-11,
        "no march drifted past the tolerance, so the check cannot discriminate"
    );
    let none = az::march::<Rk4, f64, Quiet, ExplicitTest>(
        &random_state(&mut rng),
        2,
        2,
        AzCount::new(),
        &p,
        &mut Quiet,
        &mut ExplicitTest,
    );
    assert_eq!(
        (none.registered, none.regularised),
        (0.0, 0.0),
        "a march of no interval reports energies"
    );
}

#[test]
fn qa_m308_az_energy_reported() {
    check_az(AzRead::Honest);
}

negative_control!(
    qa_m308_az_energy_reported,
    "the end state's registration read as the first",
    expected = "is not the first registration's E",
    check_az(AzRead::EndAsFirst)
);

#[cfg(feature = "controls")]
mod qa_m308_az_energy_reported_regularised {
    use super::*;
    negative_control!(
        qa_m308_az_energy_reported,
        "the registered energy read for the regularised one",
        expected = "is not the end state's energy",
        check_az(AzRead::RegisteredAsRegularised)
    );
}
