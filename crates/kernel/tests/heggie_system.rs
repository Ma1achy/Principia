//! Heggie's regularised system (`kernel::regularisation::heggie`; integrator_contract Part 2b § "Heggie: the equations,
//! the time transformations, the maps and the step control (transcribed)", § "The time-reversal maps (REQ-INT-095)",
//! Part 1; R-160, R-161, R-566), TASK-M3-08, and the energies Aarseth–Zare's march reports:
//! - `heggie_system_round_trip`: Cartesian → regularised → Cartesian at unequal masses, the registered `h` and the
//!   regularised energy against the Cartesian energy, and Eq. 7's relative velocities against the image's; control: a
//!   quarter-turned `Q₀`.
//! - `heggie_system_gamma_anchor`: `Γ* = (E − h)·R₀R₁R₂`, `E` the Cartesian energy of the image, at an `h` off the
//!   energy; control: two reduced masses swapped.
//! - `heggie_system_rates_fd`: the rates under Eq. 20 and Eq. 22 against a central difference of the Hamiltonian (`Γ*`,
//!   `Γ*·S^{−3/2}`) over random states, `dt/dτ` against `−∂/∂h`; control: a momentum rate's sign flipped.
//! - `heggie_system_dt_dtau`, `heggie_system_floors`: `dt/dτ` under each time transformation and the floors at `TINY`
//!   of `S`, `|qᵢ|` and `Rᵢ`; controls: Eq. 20's `dt/dτ` read for Eq. 22's, and an unfloored `Rᵢ`.
//! - `heggie_system_reversed`, `heggie_system_flow`, `heggie_system_default`: the time-reversal map an involution
//!   negating `P` and `t`, the flow's sums, and the default time transformation Eq. 22 (R-161); controls: a map keeping
//!   `t`, a sum dropping the clock, and a default of Eq. 20.
//! - `az_energy_reported`: Aarseth–Zare's march reports the `E` of its first registration and the regularised energy
//!   where it ended (R-566); control: the last registration's `E` read for the first's.

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{self, AzCount, AzParams, ExplicitTest};
use kernel::regularisation::form::Flow;
use kernel::regularisation::heggie::{
    register, reversed, HgP, HgQ, HgRegistered, HgSystem, DEFAULT_TIME,
};
use kernel::regularisation::pairs::HeggieTime;
use validation::negative_control;

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A generic bound triple at unequal masses `(1, 0.8, 1.3)`, moved to its centre of mass and momentum.
fn triple() -> State<f64> {
    let m = [1.0, 0.8, 1.3];
    let r = [[-0.6, 0.1], [0.5, -0.35], [0.15, 0.6]];
    let vel = [[0.05, 0.3], [-0.25, -0.1], [0.1, -0.2]];
    let total: f64 = m.iter().sum();
    let mut rc = [0.0; 2];
    let mut pc = [0.0; 2];
    for i in 0..3 {
        for c in 0..2 {
            rc[c] += m[i] * r[i][c] / total;
            pc[c] += m[i] * vel[i][c];
        }
    }
    State {
        m,
        r: [0, 1, 2].map(|i| [0, 1].map(|c| r[i][c] - rc[c])),
        p: [0, 1, 2].map(|i| [0, 1].map(|c| m[i] * vel[i][c] - m[i] * pc[c] / total)),
    }
}

/// The physical energy of `s`.
fn energy(s: &State<f64>) -> f64 {
    let kinetic: f64 = (0..3)
        .map(|i| (s.p[i][0].powi(2) + s.p[i][1].powi(2)) / (2.0 * s.m[i]))
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| s.m[i] * s.m[j] / (s.r[i][0] - s.r[j][0]).hypot(s.r[i][1] - s.r[j][1]))
        .sum();
    kinetic - potential
}

/// The largest component of `a − b` over positions and momenta, relative to the largest of `b`'s.
fn state_diff(a: &State<f64>, b: &State<f64>) -> f64 {
    let flat =
        |s: &State<f64>| -> Vec<f64> { s.r.iter().chain(s.p.iter()).flatten().copied().collect() };
    let (x, y) = (flat(a), flat(b));
    let scale = y.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    x.iter()
        .zip(&y)
        .fold(0.0f64, |m, (p, q)| m.max((p - q).abs()))
        / scale
}

/// A state with `Rᵢ = Qᵢₓ²` set by `q = [Q₀ₓ, Q₁ₓ, Q₂ₓ]` on the x axis, unit masses, `h = −1`.
fn at_r(q: [f64; 3], time: HeggieTime, tiny: f64) -> (HgSystem<f64>, HgQ<f64>, HgP<f64>) {
    (
        HgSystem::new([1.0; 3], -1.0, tiny, time),
        HgQ {
            q: q.map(|x| [x, 0.0]),
            t: 0.0,
        },
        HgP {
            p: [[0.5, 0.0], [0.0; 2], [0.0; 2]],
        },
    )
}

// ── The maps and the energy ────────────────────────────────────────────────────────────────────────────────────

/// The round trip of [`triple`] through registration and back within `1e-14` relative; the registered `h` and the
/// regularised energy the Cartesian energy within `1e-14`; Eq. 7's relative velocities the image's `vⱼ − vₖ` within
/// `1e-14`; the clock at `0`. `turn` quarter-turns `Q₀` first, the control's.
fn check_round_trip(turn: bool) {
    let s = triple();
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let HgRegistered { system, mut q, p } = register(&s, 1e-300, time);
        assert_eq!(
            (q.t, system.time),
            (0.0, time),
            "the registration's clock and time"
        );
        if turn {
            q.q[0] = [-q.q[0][1], q.q[0][0]];
        }
        let back = system.image(&q, &p);
        let d = state_diff(&back, &s);
        assert!(d < 1e-14, "the round trip misses the state by {d:.3e}");
        assert_eq!(back.m, s.m, "the masses");
        let e = energy(&s);
        assert!(
            ((system.h - e) / e).abs() < 1e-14,
            "h is not the Cartesian energy"
        );
        assert!(
            ((system.energy(&q, &p) - e) / e).abs() < 1e-14,
            "the regularised energy is not the Cartesian"
        );
        let (_, pv) = system.phys(&q, &p);
        let vel = system.q_dot(&pv);
        let w = |i: usize| [back.p[i][0] / back.m[i], back.p[i][1] / back.m[i]];
        for (i, (j, k)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
            let rel = [w(j)[0] - w(k)[0], w(j)[1] - w(k)[1]];
            let dv = (vel[i][0] - rel[0]).hypot(vel[i][1] - rel[1]) / rel[0].hypot(rel[1]);
            assert!(dv < 1e-14, "Eq. 7's q̇{i} misses vⱼ − vₖ by {dv:.3e}");
        }
    }
}

#[test]
fn heggie_system_round_trip() {
    check_round_trip(false);
}

negative_control!(
    heggie_system_round_trip,
    "Q₀ quarter-turned, which maps q₀ to −q₀",
    expected = "the round trip misses the state",
    check_round_trip(true)
);

/// `Γ*` against `(E − h)·R₀R₁R₂` at [`triple`]'s registration with `h` moved `0.3` off the energy, `E` the Cartesian
/// energy of the image (an anchor independent of the kernel's Eq. 6), within `1e-12` of `h·R₀R₁R₂`; `swap` swaps `μ₀`
/// and `μ₁` first, the control's.
fn check_gamma_anchor(swap: bool) {
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let r = register(&triple(), 1e-300, time);
        let mut system = HgSystem {
            h: r.system.h + 0.3,
            ..r.system
        };
        if swap {
            system.mu = [system.mu[1], system.mu[0], system.mu[2]];
        }
        let rr: Vec<f64> = r.q.q.iter().map(|x| x[0] * x[0] + x[1] * x[1]).collect();
        let product = rr[0] * rr[1] * rr[2];
        let e = energy(&system.image(&r.q, &r.p));
        let want = (e - system.h) * product;
        let got = system.gamma(&r.q, &r.p);
        assert!(
            (got - want).abs() <= 1e-12 * (system.h * product).abs(),
            "Γ* {got:.6e} is not (E − h)·R₀R₁R₂ {want:.6e}"
        );
    }
}

#[test]
fn heggie_system_gamma_anchor() {
    check_gamma_anchor(false);
}

negative_control!(
    heggie_system_gamma_anchor,
    "μ₀ and μ₁ swapped, a mispaired reduced mass",
    expected = "is not (E − h)·R₀R₁R₂",
    check_gamma_anchor(true)
);

// ── The rates against the Hamiltonian ──────────────────────────────────────────────────────────────────────────

/// SplitMix64: a deterministic stream of draws on `[lo, hi]`.
struct Draws(u64);

impl Draws {
    fn next(&mut self, lo: f64, hi: f64) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        lo + (hi - lo) * ((z >> 11) as f64 / (1u64 << 53) as f64)
    }
}

/// The system and state at a flat `x = (Q₀, Q₁, Q₂, P₀, P₁, P₂, h)`.
fn at(x: &[f64], m: [f64; 3], time: HeggieTime) -> (HgSystem<f64>, HgQ<f64>, HgP<f64>) {
    let v = |i: usize| [x[i], x[i + 1]];
    (
        HgSystem::new(m, x[12], 1e-300, time),
        HgQ {
            q: [v(0), v(2), v(4)],
            t: 0.0,
        },
        HgP {
            p: [v(6), v(8), v(10)],
        },
    )
}

/// The Hamiltonian in fictitious time at `x`: `Γ*` under Eq. 20, `Γ*·S^{−3/2}` under Eq. 22.
fn hamiltonian(x: &[f64], m: [f64; 3], time: HeggieTime) -> f64 {
    let (s, q, p) = at(x, m, time);
    let g = s.gamma(&q, &p);
    match time {
        HeggieTime::Eq20 => g,
        HeggieTime::Eq22 => {
            let sum: f64 = q.q.iter().map(|v| v[0] * v[0] + v[1] * v[1]).sum();
            g / (sum * sum.sqrt())
        }
    }
}

/// The largest discrepancy, over 32 random states under `time`, between the rates (`dQ/dτ = ∂Γ/∂P`, `dP/dτ = −∂Γ/∂Q`,
/// `dt/dτ = −∂Γ/∂h`) and a central difference at relative step `1e-6`, over the largest analytic rate; `flip` flips
/// `dP₀ₓ/dτ`, the control's.
fn rates_discrepancy(time: HeggieTime, flip: bool) -> f64 {
    let mut d = Draws(7);
    let mut worst = 0.0f64;
    for _ in 0..32 {
        let m = [d.next(0.3, 3.0), d.next(0.3, 3.0), d.next(0.3, 3.0)];
        let mut x: Vec<f64> = (0..6).map(|_| d.next(-1.2, 1.2)).collect();
        x.extend((0..6).map(|_| d.next(-1.0, 1.0)));
        x.push(d.next(-3.0, 1.0));
        let (s, q, p) = at(&x, m, time);
        let (dq, mut dp) = s.rates_of(&q, &p);
        assert_eq!(
            s.rates(&q, &p),
            (dq, dp),
            "the flow's rates are not rates_of"
        );
        assert_eq!(
            (s.q_rate(&q, &p), s.p_rate(&q, &p)),
            (dq, dp),
            "the flow's rates apart"
        );
        if flip {
            dp.p[0][0] = -dp.p[0][0];
        }
        let central = |c: usize| {
            let h = 1e-6 * x[c].abs().max(1.0);
            let (mut up, mut down) = (x.clone(), x.clone());
            up[c] += h;
            down[c] -= h;
            (hamiltonian(&up, m, time) - hamiltonian(&down, m, time)) / (up[c] - down[c])
        };
        let mut analytic: Vec<f64> = dq.q.iter().flatten().copied().collect();
        analytic.extend(dp.p.iter().flatten());
        analytic.push(dq.t);
        let mut fd: Vec<f64> = (6..12).map(central).collect();
        fd.extend((0..6).map(|c| -central(c)));
        fd.push(-central(12));
        let scale = analytic.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let off = analytic
            .iter()
            .zip(&fd)
            .fold(0.0f64, |a, (u, w)| a.max((u - w).abs()));
        worst = worst.max(off / scale);
    }
    worst
}

/// Each time transformation's rates within `1e-8` of the central difference.
fn check_rates(flip: bool) {
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let d = rates_discrepancy(time, flip);
        println!("heggie_system_rates_fd: {time:?}: discrepancy {d:.3e}");
        assert!(
            d < 1e-8,
            "{time:?}'s rates miss the finite difference by {d:.3e}"
        );
    }
}

#[test]
fn heggie_system_rates_fd() {
    check_rates(false);
}

negative_control!(
    heggie_system_rates_fd,
    "a momentum rate's sign flipped",
    expected = "rates miss the finite difference",
    check_rates(true)
);

// ── The time transformations and the floors ────────────────────────────────────────────────────────────────────

/// `dt/dτ`: `R₀R₁R₂` under Eq. 20, `R₀R₁R₂/S^{3/2}` under Eq. 22 (read under `time22`, the control's Eq. 20), `S`
/// floored at `TINY` below it.
fn check_dt_dtau(time22: HeggieTime) {
    let (s20, q, _) = at_r([1.0, 2.0, 0.5], HeggieTime::Eq20, 1e-300);
    let (s22, _, _) = at_r([1.0, 2.0, 0.5], time22, 1e-300);
    let sum: f64 = 1.0 + 4.0 + 0.25;
    assert_eq!(s20.dt_dtau(&q), 1.0, "dt/dτ under Eq. 20");
    assert_eq!(
        s22.dt_dtau(&q),
        1.0 / (sum * sum.sqrt()),
        "dt/dτ under Eq. 22"
    );
    let (floor, low, _) = at_r([0.1, 0.1, 0.1], HeggieTime::Eq22, 0.25);
    let r = 0.1 * 0.1;
    assert_eq!(
        floor.dt_dtau(&low),
        r * r * r / (0.25 * 0.5),
        "S below TINY floored to it"
    );
}

#[test]
fn heggie_system_dt_dtau() {
    check_dt_dtau(HeggieTime::Eq22);
}

negative_control!(
    heggie_system_dt_dtau,
    "Eq. 20's dt/dτ read for Eq. 22's",
    expected = "dt/dτ under Eq. 22",
    check_dt_dtau(HeggieTime::Eq20)
);

/// The floors at `TINY = 1/4`: the relative momentum `pᵢ = L(Qᵢ)Pᵢ/(2·max(Rᵢ, TINY))` below the floor and above it, and
/// the energy's `mⱼmₖ/max(|qᵢ|, TINY)`; `floor` is the floor the expected values divide by (the control's 0 leaves `Rᵢ`
/// unfloored).
fn check_floors(floor: f64) {
    let (s, q, p) = at_r([0.1, 1.0, 1.0], HeggieTime::Eq20, 0.25);
    let (qv, pv) = s.phys(&q, &p);
    let r0 = 0.1 * 0.1;
    assert_eq!(qv[0], [r0, 0.0], "q₀ = L(Q₀)Q₀");
    assert_eq!(
        pv[0],
        [0.1 * 0.5 / (2.0 * r0.max(floor)), 0.0],
        "p₀ with R₀ below TINY"
    );
    assert_eq!(pv[1], [0.0, 0.0], "p₁");
    let (s, q, p) = at_r([1.0, 1.0, 1.0], HeggieTime::Eq20, 0.25);
    assert_eq!(s.phys(&q, &p).1[0], [0.25, 0.0], "p₀ with R₀ above TINY");
    let e = s.energy_relative(&[[0.0; 2], [1.0, 0.0], [-1.0, 0.0]], &[[0.0; 2]; 3]);
    assert_eq!(
        e,
        -(1.0 / 0.25) - 1.0 - 1.0,
        "|q₀| = 0 floored at TINY in the energy"
    );
}

#[test]
fn heggie_system_floors() {
    check_floors(0.25);
}

negative_control!(
    heggie_system_floors,
    "R₀ left unfloored",
    expected = "p₀ with R₀ below TINY",
    check_floors(0.0)
);

// ── The map, the flow, the default ─────────────────────────────────────────────────────────────────────────────

/// The map `(Q, P, t) → (Q, −P, −t)`, an involution bitwise, as `map` gives it (the control's keeps `t`).
fn check_reversed(map: impl Fn(&HgQ<f64>, &HgP<f64>) -> (HgQ<f64>, HgP<f64>)) {
    let r = register(&triple(), 1e-300, DEFAULT_TIME);
    let q = HgQ { t: 0.375, ..r.q };
    let (a, b) = map(&q, &r.p);
    assert_eq!(a.q, q.q, "the map moves Q");
    assert_eq!(a.t, -q.t, "the map does not negate t");
    for i in 0..3 {
        assert_eq!(
            b.p[i],
            [-r.p.p[i][0], -r.p.p[i][1]],
            "the map does not negate P{i}"
        );
    }
    assert_eq!(map(&a, &b), (q, r.p), "the map is not an involution");
}

#[test]
fn heggie_system_reversed() {
    check_reversed(reversed);
}

negative_control!(
    heggie_system_reversed,
    "a map that keeps t",
    expected = "the map does not negate t",
    check_reversed(|q, p| (HgQ { t: q.t, ..*q }, reversed(q, p).1))
);

/// The flow's sums on every component: `q + h·dq`, `p + h·dp` and the weighted `k₀ + 2k₁ + 2k₂ + k₃`, the clock
/// included; `add_q` is the position sum read (the control's drops the clock).
fn check_flow(add_q: impl Fn(&HgQ<f64>, f64, &HgQ<f64>) -> HgQ<f64>) {
    let q = |a: f64| HgQ {
        q: [[a, a + 1.0], [a + 2.0, a + 3.0], [a + 4.0, a + 5.0]],
        t: a + 6.0,
    };
    let p = |a: f64| HgP {
        p: [[a, a + 1.0], [a + 2.0, a + 3.0], [a + 4.0, a + 5.0]],
    };
    let sum_q = add_q(&q(1.0), 2.0, &q(10.0));
    for (i, x) in sum_q.q.iter().flatten().enumerate() {
        assert_eq!(
            *x,
            (1.0 + i as f64) + 2.0 * (10.0 + i as f64),
            "q + h·dq, component {i}"
        );
    }
    assert_eq!(sum_q.t, 7.0 + 2.0 * 16.0, "the clock in q + h·dq");
    let sum_p = <HgSystem<f64> as Flow<f64>>::p_add(&p(1.0), 2.0, &p(10.0));
    for (i, x) in sum_p.p.iter().flatten().enumerate() {
        assert_eq!(
            *x,
            (1.0 + i as f64) + 2.0 * (10.0 + i as f64),
            "p + h·dp, component {i}"
        );
    }
    let k = [q(1.0), q(10.0), q(100.0), q(1000.0)];
    let wq = <HgSystem<f64> as Flow<f64>>::q_weighted(2.0, [&k[0], &k[1], &k[2], &k[3]]);
    let want = |a: [f64; 4], i: f64| (a[0] + i) + 2.0 * (a[1] + i) + 2.0 * (a[2] + i) + (a[3] + i);
    for (i, x) in wq.q.iter().flatten().enumerate() {
        assert_eq!(
            *x,
            want([1.0, 10.0, 100.0, 1000.0], i as f64),
            "weighted q, component {i}"
        );
    }
    assert_eq!(
        wq.t,
        want([1.0, 10.0, 100.0, 1000.0], 6.0),
        "the weighted clock"
    );
    let kp = [p(1.0), p(10.0), p(100.0), p(1000.0)];
    let wp = <HgSystem<f64> as Flow<f64>>::p_weighted(2.0, [&kp[0], &kp[1], &kp[2], &kp[3]]);
    for (i, x) in wp.p.iter().flatten().enumerate() {
        assert_eq!(
            *x,
            want([1.0, 10.0, 100.0, 1000.0], i as f64),
            "weighted p, component {i}"
        );
    }
}

#[test]
fn heggie_system_flow() {
    check_flow(<HgSystem<f64> as Flow<f64>>::q_add);
}

negative_control!(
    heggie_system_flow,
    "a position sum that drops the clock",
    expected = "the clock in q + h·dq",
    check_flow(|q, h, dq| HgQ {
        t: q.t,
        ..<HgSystem<f64> as Flow<f64>>::q_add(q, h, dq)
    })
);

/// Heggie's default time transformation is Eq. 22 at n = 3/2 (R-161).
fn check_default(time: HeggieTime) {
    assert_eq!(
        time,
        HeggieTime::Eq22,
        "the default time transformation is not Eq. 22"
    );
}

#[test]
fn heggie_system_default() {
    check_default(DEFAULT_TIME);
}

negative_control!(
    heggie_system_default,
    "a default of Eq. 20",
    expected = "the default time transformation is not Eq. 22",
    check_default(HeggieTime::Eq20)
);

// ── Aarseth–Zare's reported energies ───────────────────────────────────────────────────────────────────────────

/// A cadence that keeps nothing and never ends the march.
struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// Test values for Aarseth–Zare's step control at f64 (the reference's `TINY`, R-953, and landing tolerance, RQ-795,
/// meanwhile), a budget that does not bind.
fn az_params() -> AzParams<f64> {
    AzParams {
        eta: 0.01,
        delta_sync: 0.125,
        tiny: 1e-300,
        predictive_f: 0.02,
        r3_floor: 1e-300,
        land_eps_rel: 1e-14,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
    }
}

/// A march of [`triple`] to boundary 3 reports `registered`, the `E` of its first registration, bitwise, and
/// `regularised`, the energy of its last interval's regularised end, which a re-registration of its final state gives
/// back within `1e-12`; `first` is the registration read as the first (the control's is the last), and no march reports
/// both as `0`.
fn check_az_energy(first: impl Fn(&State<f64>, &State<f64>) -> State<f64>) {
    let p = az_params();
    let s0 = triple();
    let out = az::march::<Rk4, f64, Quiet, ExplicitTest>(
        &s0,
        0,
        3,
        AzCount::new(),
        &p,
        &mut Quiet,
        &mut ExplicitTest,
    );
    assert_eq!(out.boundary, 3, "the march did not land on 3");
    let (sys0, q0, p0) = az::register(&first(&s0, &out.state), p.tiny, p.r3_floor);
    assert_eq!(
        out.registered.to_bits(),
        sys0.e.to_bits(),
        "registered is not the first registration's E"
    );
    assert!(
        ((sys0.energy(&q0, &p0) - sys0.e) / sys0.e).abs() < 1e-14,
        "the energy at registration is not E"
    );
    let (sys1, _, _) = az::register(&out.state, p.tiny, p.r3_floor);
    assert!(
        ((out.regularised - sys1.e) / sys1.e).abs() < 1e-12,
        "regularised {:e} is not the final state's energy {:e}",
        out.regularised,
        sys1.e
    );
    assert_ne!(out.regularised, 0.0, "no regularised energy reported");
}

#[test]
fn az_energy_reported() {
    check_az_energy(|s0, _| *s0);
}

negative_control!(
    az_energy_reported,
    "the last registration read as the first",
    expected = "registered is not the first registration's E",
    check_az_energy(|_, last| *last)
);
