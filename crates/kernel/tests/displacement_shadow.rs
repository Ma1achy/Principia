//! The FTLE shadow's joint step on the default occupant, Heggie × RK4 (`kernel::regularisation::heggie_joint`;
//! R-974, R-560; dd_integrator §3.8, "Heggie × RK4's joint step and its displacement"; REQ-VAL-355), TASK-M3-74:
//! - `displacement_shadow_same_time`: on a joint-step run, at every renormalisation the shadow's elapsed physical time
//!   equals the sample's, bit for bit, at f32 and f64, on three orbits, the clocks having moved apart between;
//!   control: a renormalisation that rescales without resynchronising, the shadow advanced by the sample's `Δτ` alone.
//! - `displacement_shadow_sample_bitwise`: the joint step leaves the sample exactly RK4's step on Heggie's system,
//!   at f32 and f64; control: held against KDK's.
//! - `displacement_shadow_equal_time_separation`: the resynchronised `J·δy` is the separation of an absolute shadow,
//!   marched on its own and brought to the sample's physical time, to second order in the separation; control: `J·δy`
//!   read without the resynchronisation.
//! - `displacement_shadow_frozen_energy`: the joint step's `δy` is the absolute shadow's state less the sample's, the
//!   shadow marched under its own frozen energy, but for rounding; control: the displacement's `δh` dropped.
//! - `displacement_shadow_tangent`: `J` against a central difference of the return to the Cartesian state, with the
//!   floor at `TINY` off, on, and at the kink; `J` after the start the Cartesian offset in the centre-of-mass frame;
//!   control: `J` read at twice the displacement.
//! - `displacement_shadow_resync`: `J` of the resynchronised displacement is §3.8's Cartesian rule,
//!   `δx − f(x)·δt`; control: `δt` zeroed with no move.
//! - `displacement_shadow_rescale`: the rescaled displacement's `‖J·δy‖` is `δ₀`, its `δt` scaled, its `δh` the rescaled
//!   shadow's energy offset; control: `δh` kept from before the rescale.
//!
//! Each test has a registered negative control naming the panic it expects (R-176).

use kernel::driver::benettin::offset;
use kernel::occupant::State;
use kernel::regularisation::az::AzReal;
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::heggie::{register, HgP, HgQ, HgRegistered, HgSystem, DEFAULT_TIME};
use kernel::regularisation::heggie_joint::{
    cartesian, joint_step, renormalise, rescale, resynchronise, separation, start, HgDisplacement,
};
use validation::measure::benettin::{burrau, figure_eight};
use validation::negative_control;

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: AzReal + Into<f64> {
    fn of(x: f64) -> Self;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// A generic bound triple at unequal masses `(1, 0.8, 1.3)`, moved to its centre of mass and momentum.
fn triple() -> State<f64> {
    let m = [1.0, 0.8, 1.3];
    let r = [[-0.6, 0.1], [0.5, -0.35], [0.15, 0.6]];
    let vel = [[0.05, 0.3], [-0.25, -0.1], [0.1, -0.2]];
    let total: f64 = m.iter().sum();
    let rc = [0, 1].map(|c| (0..3).map(|i| m[i] * r[i][c]).sum::<f64>() / total);
    let pc = [0, 1].map(|c| (0..3).map(|i| m[i] * vel[i][c]).sum::<f64>());
    State {
        m,
        r: [0, 1, 2].map(|i| [0, 1].map(|c| r[i][c] - rc[c])),
        p: [0, 1, 2].map(|i| [0, 1].map(|c| m[i] * vel[i][c] - m[i] * pc[c] / total)),
    }
}

/// The fixture states, in the centre-of-mass frame: Burrau's rest start, the figure-eight and [`triple`].
fn states() -> Vec<State<f64>> {
    let ic = |o: validation::measure::benettin::Orbit| State {
        m: o.ic.m,
        r: o.ic.r,
        p: o.ic.p,
    };
    vec![ic(burrau()), ic(figure_eight(1.0)), triple()]
}

/// `s` at `R`.
fn cast<R: P>(s: &State<f64>) -> State<R> {
    State {
        m: s.m.map(R::of),
        r: s.r.map(|v| v.map(R::of)),
        p: s.p.map(|v| v.map(R::of)),
    }
}

/// The floor `TINY` the tests register with: below every `Rᵢ` the runs reach.
fn tiny<R: P>() -> R {
    R::of(1e-30)
}

/// `s` registered under Heggie's default time transformation (Eq. 22, R-161).
fn registered<R: P>(s: &State<f64>) -> HgRegistered<R> {
    register(&cast::<R>(s), tiny(), DEFAULT_TIME)
}

/// §3.8's Cartesian offset `‖δ₀‖·u`.
fn offset_of<R: P>(m: [R; 3], delta_0: R) -> State<R> {
    let zero = State {
        m,
        r: [[R::default(); 2]; 3],
        p: [[R::default(); 2]; 3],
    };
    offset(&zero, delta_0)
}

/// The physical step each joint step of a run is sized to: `dτ = PHYS_STEP / g(y)`.
const PHYS_STEP: f64 = 1e-3;

/// `dτ` for the sample at `q`: `PHYS_STEP / (dt/dτ)`.
fn dtau<R: P>(system: &HgSystem<R>, q: &HgQ<R>) -> R {
    R::of(PHYS_STEP) / system.dt_dtau(q)
}

/// A state's twelve components at f64.
fn flat<R: P>(s: &State<R>) -> Vec<f64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| x.into())
        .collect()
}

/// The Euclidean norm of a flat vector.
fn norm(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum::<f64>().sqrt()
}

/// `‖a − b‖ / ‖b‖`.
fn rel(a: &[f64], b: &[f64]) -> f64 {
    let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    norm(&d) / norm(b)
}

/// The regularised components `(Q, t, P)` at f64.
fn reg<R: P>(q: &HgQ<R>, p: &HgP<R>) -> Vec<f64> {
    q.q.iter()
        .flatten()
        .chain(core::iter::once(&q.t))
        .chain(p.p.iter().flatten())
        .map(|&x| x.into())
        .collect()
}

/// A renormalisation: the displacement after it and the `δ` it read.
type Renorm<R> =
    fn(&HgSystem<R>, &HgQ<R>, &HgP<R>, &HgDisplacement<R>, R) -> (HgDisplacement<R>, R);

// ── displacement_shadow_same_time ──────────────────────────────────────────────────────────────────────────────

/// A joint-step run of each fixture at `R`: `‖δ₀‖ = 1e-5`, 2000 joint steps of `dτ = PHYS_STEP/g`, renormalised by
/// `renorm` every 50. At each renormalisation the shadow's elapsed physical time, the sample's clock plus `δt`, must
/// be the sample's, bit for bit; over the run the clocks must have moved apart before some renormalisation.
fn check_same_time<R: P>(renorm: Renorm<R>) {
    for s in states() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = registered::<R>(&s);
        let delta_0 = R::of(1e-5);
        let mut d = start(&system, &q, &p, &offset_of(system.m, delta_0));
        let (mut renorms, mut apart) = (0, false);
        for n in 1..=2000 {
            let h = dtau(&system, &q);
            (q, p, d) = joint_step(&system, &q, &p, &d, h);
            if n % 50 == 0 {
                apart |= d.q.t != R::default();
                (d, _) = renorm(&system, &q, &p, &d, delta_0);
                let shadow_t = q.t + d.q.t;
                assert!(
                    shadow_t.into().to_bits() == q.t.into().to_bits(),
                    "the shadow's elapsed physical time {:e} is not the sample's {:e}",
                    shadow_t.into(),
                    q.t.into()
                );
                renorms += 1;
            }
        }
        assert_eq!(renorms, 40, "the renormalisations");
        assert!(apart, "the run never moved the clocks apart");
    }
}

#[test]
fn displacement_shadow_same_time() {
    check_same_time::<f32>(renormalise);
    check_same_time::<f64>(renormalise);
}

negative_control!(
    displacement_shadow_same_time,
    "a renormalisation that rescales without resynchronising leaves the shadow at its own Δτ's time",
    expected = "is not the sample's",
    check_same_time::<f64>(rescale)
);

// ── displacement_shadow_sample_bitwise ─────────────────────────────────────────────────────────────────────────

/// Along a 300-step joint run of each fixture at `R`, every joint step's sample is `form`'s step from the same state,
/// bit for bit.
fn check_bitwise<R: P>(form: Form) {
    for s in states() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = registered::<R>(&s);
        let mut d = start(&system, &q, &p, &offset_of(system.m, R::of(1e-4)));
        for _ in 0..300 {
            let h = dtau(&system, &q);
            let (sq, sp) = form.step(&system, &q, &p, h);
            (q, p, d) = joint_step(&system, &q, &p, &d, h);
            let bits = |x: Vec<f64>| x.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
            assert!(
                bits(reg(&q, &p)) == bits(reg(&sq, &sp)),
                "the joint step's sample is not its RK4 step's"
            );
        }
    }
}

#[test]
fn displacement_shadow_sample_bitwise() {
    check_bitwise::<f32>(Form::Rk4);
    check_bitwise::<f64>(Form::Rk4);
}

negative_control!(
    displacement_shadow_sample_bitwise,
    "the joint step held against KDK's step on Heggie's system",
    expected = "is not its RK4 step's",
    check_bitwise::<f64>(Form::Kdk)
);

// ── displacement_shadow_equal_time_separation ──────────────────────────────────────────────────────────────────

/// What the run reads as the Cartesian displacement at a renormalisation.
type Read = fn(&HgSystem<f64>, &HgQ<f64>, &HgP<f64>, &HgDisplacement<f64>) -> State<f64>;

/// The resynchronised `J·δy`, §3.8's read.
fn resynced(s: &HgSystem<f64>, q: &HgQ<f64>, p: &HgP<f64>, d: &HgDisplacement<f64>) -> State<f64> {
    cartesian(s, q, p, &resynchronise(s, q, p, d))
}

/// `J·δy` as carried, no resynchronisation: the control's read.
#[cfg(feature = "controls")]
fn unsynced(s: &HgSystem<f64>, q: &HgQ<f64>, p: &HgP<f64>, d: &HgDisplacement<f64>) -> State<f64> {
    cartesian(s, q, p, d)
}

/// At f64, `‖δ₀‖ = 1e-7`: over ten intervals of 100 joint steps of each fixture, an absolute shadow started at each
/// interval's `y + δy` with its own system (energy `h + δh`) is marched by the same `dτ`s, then by one RK4 step of
/// `(t − t')/g(y')` to the sample's clock `t`. Its Cartesian state less the sample's must be `read`'s displacement
/// within `1e-4` relative: the resynchronisation's dropped terms are second order in the separation, `O(δ)` relative,
/// about `1e-5` at the separations reached, and the absolute difference's rounding `ε·‖x‖/δ`, about `1e-9`.
/// [`separation`] must read that norm within the same bound.
fn check_equal_time(read: Read) {
    for s in states() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = registered::<f64>(&s);
        let delta_0 = 1e-7;
        let mut d = start(&system, &q, &p, &offset_of(system.m, delta_0));
        for _ in 0..10 {
            let shadow = HgSystem {
                h: system.h + d.h,
                ..system
            };
            let mut sq = HgSystem::q_add(&q, 1.0, &d.q);
            let mut sp = HgSystem::p_add(&p, 1.0, &d.p);
            for _ in 0..100 {
                let h = dtau(&system, &q);
                (sq, sp) = Form::Rk4.step(&shadow, &sq, &sp, h);
                (q, p, d) = joint_step(&system, &q, &p, &d, h);
            }
            let back = (q.t - sq.t) / shadow.dt_dtau(&sq);
            let (eq, ep) = Form::Rk4.step(&shadow, &sq, &sp, back);
            let sep: Vec<f64> = flat(&shadow.image(&eq, &ep))
                .iter()
                .zip(flat(&system.image(&q, &p)))
                .map(|(a, b)| a - b)
                .collect();
            let err = rel(&flat(&read(&system, &q, &p, &d)), &sep);
            assert!(
                err < 1e-4,
                "J·δy is not the shadow's separation at the sample's time: {err:.3e} relative"
            );
            let read_err = (separation(&system, &q, &p, &d) - norm(&sep)).abs() / norm(&sep);
            assert!(read_err < 1e-4, "the read's δ misses by {read_err:.3e}");
            (d, _) = renormalise(&system, &q, &p, &d, delta_0);
        }
    }
}

#[test]
fn displacement_shadow_equal_time_separation() {
    check_equal_time(resynced);
}

negative_control!(
    displacement_shadow_equal_time_separation,
    "J·δy read with δt carried, the flow component ẋ·δt left in",
    expected = "is not the shadow's separation at the sample's time",
    check_equal_time(unsynced)
);

// ── displacement_shadow_frozen_energy ──────────────────────────────────────────────────────────────────────────

/// At f64, `‖δ₀‖ = 1e-6`: over 200 joint steps of each fixture, `δy` (positions, clock and momenta) must be the
/// absolute shadow's state less the sample's within `1e-6` relative, the shadow started at `y + δy₀` and marched by RK4
/// under its own system, energy `h + δh`: the stages are the same and only the rounding of the two subtractions
/// differs, `ε·‖y‖/‖δy‖`, about `1e-10` a step. `keep_h` false drops `δh` from the joint step's displacement, the
/// control's.
fn check_frozen_energy(keep_h: bool) {
    for s in states() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = registered::<f64>(&s);
        let d0 = start(&system, &q, &p, &offset_of(system.m, 1e-6));
        let shadow = HgSystem {
            h: system.h + d0.h,
            ..system
        };
        let mut sq = HgSystem::q_add(&q, 1.0, &d0.q);
        let mut sp = HgSystem::p_add(&p, 1.0, &d0.p);
        let mut d = HgDisplacement {
            h: if keep_h { d0.h } else { 0.0 },
            ..d0
        };
        for _ in 0..200 {
            let h = dtau(&system, &q);
            (sq, sp) = Form::Rk4.step(&shadow, &sq, &sp, h);
            (q, p, d) = joint_step(&system, &q, &p, &d, h);
        }
        let diff: Vec<f64> = reg(&sq, &sp)
            .iter()
            .zip(reg(&q, &p))
            .map(|(a, b)| a - b)
            .collect();
        let err = rel(&reg(&d.q, &d.p), &diff);
        assert!(
            err < 1e-6,
            "δy is not the shadow's own-energy march less the sample's: {err:.3e} relative"
        );
    }
}

#[test]
fn displacement_shadow_frozen_energy() {
    check_frozen_energy(true);
}

negative_control!(
    displacement_shadow_frozen_energy,
    "the shadow marched at the sample's frozen energy",
    expected = "is not the shadow's own-energy march",
    check_frozen_energy(false)
);

// ── displacement_shadow_tangent ────────────────────────────────────────────────────────────────────────────────

/// A seeded pseudo-random number in `[−1, 1)` (a 64-bit LCG), so the cases are fixed.
fn next(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
}

/// The map under test: `J·δy`.
type Map = fn(&HgSystem<f64>, &HgQ<f64>, &HgP<f64>, &HgDisplacement<f64>) -> State<f64>;

/// `J` read at twice the displacement: the control's map.
#[cfg(feature = "controls")]
fn doubled(s: &HgSystem<f64>, q: &HgQ<f64>, p: &HgP<f64>, d: &HgDisplacement<f64>) -> State<f64> {
    let two = HgDisplacement {
        q: HgSystem::q_add(&d.q, 1.0, &d.q),
        p: HgSystem::p_add(&d.p, 1.0, &d.p),
        h: d.h,
    };
    cartesian(s, q, p, &two)
}

/// At f64, on each fixture registered and on eight seeded displacements of all twelve regularised components:
/// `j(δy)` against the central difference `(image(y + εδy) − image(y − εδy))/2ε`, `ε = 1e-6`, within `1e-7` relative
/// (the difference's truncation `O(ε²)` and rounding `O(1e-16/ε)`), with `TINY` below every `Rᵢ` and above every
/// `Rᵢ`, the floor then constant; at `TINY` exactly the least `Rᵢ`, where the floor is the identity, `J` is the unfloored
/// one's, bit for bit. Then `j` after [`start`] is the Cartesian offset less its centre-of-mass parts within `1e-13`,
/// and the start's `δt` is 0 and its `δh` the shadow's energy less the sample's, bit for bit.
fn check_tangent(j: Map) {
    let mut seed = 7;
    for s in states() {
        let HgRegistered { system, q, p } = registered::<f64>(&s);
        for _ in 0..8 {
            let d = HgDisplacement {
                q: HgQ {
                    q: [[0.0; 2]; 3].map(|_| [next(&mut seed), next(&mut seed)]),
                    t: next(&mut seed),
                },
                p: HgP {
                    p: [[0.0; 2]; 3].map(|_| [next(&mut seed), next(&mut seed)]),
                },
                h: 0.0,
            };
            for floor in [1e-300, 1e3] {
                let sys = HgSystem {
                    tiny: floor,
                    ..system
                };
                let eps = 1e-6;
                let at = |e: f64| {
                    flat(&sys.image(&HgSystem::q_add(&q, e, &d.q), &HgSystem::p_add(&p, e, &d.p)))
                };
                let fd: Vec<f64> = at(eps)
                    .iter()
                    .zip(at(-eps))
                    .map(|(a, b)| (a - b) / (2.0 * eps))
                    .collect();
                let err = rel(&flat(&j(&sys, &q, &p, &d)), &fd);
                assert!(
                    err < 1e-7,
                    "J·δy misses the image's derivative by {err:.3e} at TINY {floor:e}"
                );
            }
            let least =
                q.q.iter()
                    .map(|u| u[0] * u[0] + u[1] * u[1])
                    .fold(f64::INFINITY, f64::min);
            let kink = HgSystem {
                tiny: least,
                ..system
            };
            let open = HgSystem {
                tiny: 0.0,
                ..system
            };
            assert!(
                flat(&j(&kink, &q, &p, &d)) == flat(&j(&open, &q, &p, &d)),
                "J at the floor's kink is not the unfloored map's"
            );
        }
        let dx = State {
            m: s.m,
            r: [[0.0; 2]; 3].map(|_| [next(&mut seed), next(&mut seed)]),
            p: [[0.0; 2]; 3].map(|_| [next(&mut seed), next(&mut seed)]),
        };
        let total: f64 = s.m.iter().sum();
        let com = [0, 1].map(|c| (0..3).map(|i| s.m[i] * dx.r[i][c]).sum::<f64>() / total);
        let mean = [0, 1].map(|c| (0..3).map(|i| dx.p[i][c]).sum::<f64>() / 3.0);
        let want = State {
            m: s.m,
            r: dx.r.map(|v| [v[0] - com[0], v[1] - com[1]]),
            p: dx.p.map(|v| [v[0] - mean[0], v[1] - mean[1]]),
        };
        let d0 = start(&system, &q, &p, &dx);
        let back = j(&system, &q, &p, &d0);
        let err = rel(&flat(&back), &flat(&want));
        assert!(
            err < 1e-13,
            "J·start(δx) is not δx in the centre-of-mass frame: {err:.3e}"
        );
        let dh = system.energy(
            &HgSystem::q_add(&q, 1.0, &d0.q),
            &HgSystem::p_add(&p, 1.0, &d0.p),
        ) - system.energy(&q, &p);
        assert!(
            d0.q.t == 0.0 && d0.h == dh && dh != 0.0,
            "the start's δt is not 0 or its δh not the shadow's energy offset"
        );
    }
}

#[test]
fn displacement_shadow_tangent() {
    check_tangent(cartesian);
}

negative_control!(
    displacement_shadow_tangent,
    "J read at twice the displacement",
    expected = "misses the image's derivative",
    check_tangent(doubled)
);

// ── displacement_shadow_resync ─────────────────────────────────────────────────────────────────────────────────

/// A resynchronisation under test.
type Resync = fn(&HgSystem<f64>, &HgQ<f64>, &HgP<f64>, &HgDisplacement<f64>) -> HgDisplacement<f64>;

/// `δt` zeroed with no move: the control's.
#[cfg(feature = "controls")]
fn zeroed(
    _: &HgSystem<f64>,
    _: &HgQ<f64>,
    _: &HgP<f64>,
    d: &HgDisplacement<f64>,
) -> HgDisplacement<f64> {
    HgDisplacement {
        q: HgQ { q: d.q.q, t: 0.0 },
        ..*d
    }
}

/// The physical-time vector field at `x`, `f(x) = (p/m, m·a(r))`, Newtonian, `G = 1`.
fn field(x: &State<f64>) -> Vec<f64> {
    let mut v = vec![0.0; 12];
    for i in 0..3 {
        for c in 0..2 {
            v[2 * i + c] = x.p[i][c] / x.m[i];
        }
        for j in 0..3 {
            if i != j {
                let d = [x.r[j][0] - x.r[i][0], x.r[j][1] - x.r[i][1]];
                let n3 = d[0].hypot(d[1]).powi(3);
                for c in 0..2 {
                    v[6 + 2 * i + c] += x.m[i] * x.m[j] * d[c] / n3;
                }
            }
        }
    }
    v
}

/// At f64, on each fixture after 300 joint steps, with `δt` set to `1e-6` beside the carried displacement: `resync`
/// must leave `δt = 0` and `J·δy` §3.8's Cartesian rule, `J·δy − f(x)·δt`, within `1e-9` of `‖f(x)·δt‖` (`J·F(y)` is
/// `g·f(x)` on the sample's energy surface, so the rule holds but for rounding and the energy's own drift).
fn check_resync(resync: Resync) {
    for s in states() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = registered::<f64>(&s);
        let mut d = start(&system, &q, &p, &offset_of(system.m, 1e-6));
        for _ in 0..300 {
            let h = dtau(&system, &q);
            (q, p, d) = joint_step(&system, &q, &p, &d, h);
        }
        let dt = 1e-6;
        d.q.t = dt;
        let out = resync(&system, &q, &p, &d);
        assert_eq!(out.q.t, 0.0, "δt after the resynchronisation");
        let f = field(&system.image(&q, &p));
        let want: Vec<f64> = flat(&cartesian(&system, &q, &p, &d))
            .iter()
            .zip(&f)
            .map(|(a, b)| a - b * dt)
            .collect();
        let got = flat(&cartesian(&system, &q, &p, &out));
        let miss: Vec<f64> = got.iter().zip(&want).map(|(a, b)| a - b).collect();
        let err = norm(&miss) / (norm(&f) * dt);
        assert!(
            err < 1e-9,
            "J of the resynchronised δy is not δx − f(x)·δt: {err:.3e}"
        );
    }
}

#[test]
fn displacement_shadow_resync() {
    check_resync(resynchronise);
}

negative_control!(
    displacement_shadow_resync,
    "δt zeroed with no move along the flow",
    expected = "is not δx − f(x)·δt",
    check_resync(zeroed)
);

// ── displacement_shadow_rescale ────────────────────────────────────────────────────────────────────────────────

/// A rescale under test.
type Rescale = Renorm<f64>;

/// The rescale with `δh` kept from before it: the control's.
#[cfg(feature = "controls")]
fn kept(
    s: &HgSystem<f64>,
    q: &HgQ<f64>,
    p: &HgP<f64>,
    d: &HgDisplacement<f64>,
    delta_0: f64,
) -> (HgDisplacement<f64>, f64) {
    let (out, delta) = rescale(s, q, p, d, delta_0);
    (HgDisplacement { h: d.h, ..out }, delta)
}

/// At f64, on each fixture after 300 joint steps, the separation grown from `‖δ₀‖ = 1e-6`, with `δt = 1e-7` set:
/// `rescale` must return `δ = ‖J·δy‖` bit for bit, leave `‖J·δy‖ = δ₀` within `1e-14` relative, scale `δt` by `δ₀/δ`
/// bit for bit, and re-form `δh` as the rescaled shadow's energy less the sample's, bit for bit.
fn check_rescale(rescale: Rescale) {
    for s in states() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = registered::<f64>(&s);
        let delta_0 = 1e-6;
        let mut d = start(&system, &q, &p, &offset_of(system.m, delta_0));
        for _ in 0..300 {
            let h = dtau(&system, &q);
            (q, p, d) = joint_step(&system, &q, &p, &d, h);
        }
        d.q.t = 1e-7;
        let before = norm(&flat(&cartesian(&system, &q, &p, &d)));
        let (out, delta) = rescale(&system, &q, &p, &d, delta_0);
        let x = cartesian(&system, &q, &p, &d);
        let r2: f64 = x.r.iter().map(|v| v[0] * v[0] + v[1] * v[1]).sum();
        let p2: f64 = x.p.iter().map(|v| v[0] * v[0] + v[1] * v[1]).sum();
        assert_eq!(delta, (r2 + p2).sqrt(), "δ is not ‖J·δy‖");
        assert!(
            ((before - delta) / delta).abs() < 1e-15,
            "δ in the read side's order"
        );
        let after = norm(&flat(&cartesian(&system, &q, &p, &out)));
        assert!(
            ((after - delta_0) / delta_0).abs() < 1e-14,
            "‖J·δy‖ after the rescale is {after:e}, not δ₀"
        );
        assert_eq!(out.q.t, (delta_0 / delta) * 1e-7, "δt is not scaled");
        let e = |q: &HgQ<f64>, p: &HgP<f64>| system.energy(q, p);
        let dh = e(
            &HgSystem::q_add(&q, 1.0, &out.q),
            &HgSystem::p_add(&p, 1.0, &out.p),
        ) - e(&q, &p);
        assert!(
            out.h == dh && dh != d.h,
            "δh is not the rescaled shadow's energy offset"
        );
    }
}

#[test]
fn displacement_shadow_rescale() {
    check_rescale(rescale);
}

negative_control!(
    displacement_shadow_rescale,
    "δh kept from before the rescale",
    expected = "is not the rescaled shadow's energy offset",
    check_rescale(kept)
);
