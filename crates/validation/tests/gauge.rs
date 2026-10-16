//! The gauge group's action (`validation::gauge`; dd_encode §3.3; TASK-M2-15), the test utility T1 sweeps with:
//! - `gauge_action`: an element acts as `rᵢ' = s·R(θ)·Π·rᵢ + a`, `pᵢ' = s^{−1/2}·R(θ)·Π·pᵢ + mᵢ·u`, on a hand-computed
//!   case; the identity leaves a state as it is; and on random states and elements the similarity laws hold: with no
//!   boost, the energy scales as `1/s`, `L_z` as `s^{1/2}` with its sign flipped by `Π`, the CoM moves to
//!   `s·R(θ)·Π·R_com + a` and the total momentum to `s^{−1/2}·R(θ)·Π·P + u`; and a random draw is in its ranges.

use kernel::decode::Ic;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;

fn energy(x: &Ic<f64>) -> f64 {
    let kinetic: f64 = (0..3)
        .map(|i| (x.p[i][0].powi(2) + x.p[i][1].powi(2)) / (2.0 * x.m[i]))
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let d = (x.r[j][0] - x.r[i][0]).hypot(x.r[j][1] - x.r[i][1]);
            -x.m[i] * x.m[j] / d
        })
        .sum();
    kinetic + potential
}

fn angular_momentum(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// A random state: masses on the simplex, positions and momenta about the CoM frame, total momentum zero.
fn random_state(rng: &mut SplitMix) -> Ic<f64> {
    let a = uniform(rng, 0.1, 0.5);
    let b = uniform(rng, 0.1, 0.4);
    let m = [a, b, 1.0 - a - b];
    let mut r = [[0.0; 2]; 3];
    let mut p = [[0.0; 2]; 3];
    for i in 0..3 {
        r[i] = [uniform(rng, -1.0, 1.0), uniform(rng, -1.0, 1.0)];
        p[i] = [uniform(rng, -1.0, 1.0), uniform(rng, -1.0, 1.0)];
    }
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
    let total = [0, 1].map(|k| (0..3).map(|i| p[i][k]).sum::<f64>());
    for i in 0..3 {
        for k in 0..2 {
            r[i][k] -= com[k];
            p[i][k] -= m[i] * total[k];
        }
    }
    Ic { m, r, p }
}

/// `a` and `b` within `10⁻⁹` of `scale` (at least 1): a translation of up to 10 on a state scaled by `2⁻⁸` leaves its
/// differences about `10⁻¹³` relative rounding, and a close pair's potential amplifies it.
fn close(a: f64, b: f64, scale: f64) -> bool {
    (a - b).abs() <= 1e-9 * scale.max(1.0)
}

/// The similarity laws of `gx = g·x`, `g` without a boost: `E(gx) = E(x)/s`, `L_z(gx) = ±s^{1/2}·L_z(x)`, `−` iff `g`
/// mirrors; and, whatever the boost, the CoM and total momentum moved as `g` moves a point and a momentum of mass 1.
fn check_laws(x: &Ic<f64>, g: &Gauge, gx: &Ic<f64>) {
    let at = format!("g = {g:?}");
    let unboosted = Gauge { u: [0.0; 2], ..*g };
    let still = if g.u == [0.0; 2] {
        *gx
    } else {
        unboosted.act(x)
    };
    let (e, e1) = (energy(x), energy(&still));
    assert!(
        close(e1 * g.s, e, e.abs()),
        "E' = {e1}, not E/s = {} at {at}",
        e / g.s
    );
    let sign = if g.mirror { -1.0 } else { 1.0 };
    let (l, l1) = (angular_momentum(x), angular_momentum(&still));
    assert!(
        close(l1, sign * g.s.sqrt() * l, l.abs()),
        "L_z' = {l1}, not ±s^(1/2)·L_z = {} at {at}",
        sign * g.s.sqrt() * l
    );
    let com = |y: &Ic<f64>| [0, 1].map(|k| (0..3).map(|i| y.m[i] * y.r[i][k]).sum::<f64>());
    let total = |y: &Ic<f64>| [0, 1].map(|k| (0..3).map(|i| y.p[i][k]).sum::<f64>());
    let point = Ic {
        m: [1.0, 0.0, 0.0],
        r: [com(x), [0.0; 2], [0.0; 2]],
        p: [total(x), [0.0; 2], [0.0; 2]],
    };
    let moved = g.act(&point);
    for k in 0..2 {
        assert!(
            close(com(gx)[k], moved.r[0][k], g.s + g.a[k].abs()),
            "the CoM moved to {:?}, not {:?}, at {at}",
            com(gx),
            moved.r[0]
        );
        assert!(
            close(total(gx)[k], moved.p[0][k], 1.0 + g.u[k].abs()),
            "the total momentum moved to {:?}, not {:?}, at {at}",
            total(gx),
            moved.p[0]
        );
    }
}

#[test]
fn gauge_action() {
    // By hand: θ = π/2, s = 4, Π, a = (1, 2), u = (½, −1). `Π(x, y) = (x, −y)`, `R(π/2)(x, y) = (−y, x)`.
    let g = Gauge {
        a: [1.0, 2.0],
        u: [0.5, -1.0],
        theta: std::f64::consts::FRAC_PI_2,
        s: 4.0,
        mirror: true,
    };
    let x = Ic {
        m: [0.25, 0.25, 0.5],
        r: [[1.0, 3.0], [0.0, 0.0], [0.0, 0.0]],
        p: [[2.0, 4.0], [0.0, 0.0], [0.0, 0.0]],
    };
    let gx = g.act(&x);
    // r₀: Π → (1, −3); R → (3, 1); ·4 → (12, 4); +a → (13, 6). p₀: Π → (2, −4); R → (4, 2); /2 → (2, 1); +¼u → (2.125, 0.75).
    let want_r = [13.0, 6.0];
    let want_p = [2.125, 0.75];
    for k in 0..2 {
        assert!(
            close(gx.r[0][k], want_r[k], 13.0),
            "r₀' = {:?}, not {want_r:?}",
            gx.r[0]
        );
        assert!(
            close(gx.p[0][k], want_p[k], 2.0),
            "p₀' = {:?}, not {want_p:?}",
            gx.p[0]
        );
    }
    // r₁ = 0 moves to a; p₂ = 0 to m₂·u.
    assert!(
        close(gx.r[1][0], 1.0, 1.0) && close(gx.r[1][1], 2.0, 1.0),
        "r₁' = {:?}",
        gx.r[1]
    );
    assert!(
        close(gx.p[2][0], 0.25, 1.0) && close(gx.p[2][1], -0.5, 1.0),
        "p₂' = {:?}",
        gx.p[2]
    );
    assert_eq!(gx.m, x.m, "the masses moved");
    assert_eq!(Gauge::identity().act(&x), x, "the identity moved the state");

    let mut rng = SplitMix::new(7);
    let mut mirrors = 0;
    for _ in 0..1000 {
        let x = random_state(&mut rng);
        let g = Gauge::random(&mut rng, 10.0, 8.0);
        assert!(
            g.a.iter().chain(&g.u).all(|v| (-10.0..10.0).contains(v))
                && (-std::f64::consts::PI..std::f64::consts::PI).contains(&g.theta)
                && (2f64.powi(-8)..2f64.powi(8)).contains(&g.s),
            "a draw out of its ranges: {g:?}"
        );
        mirrors += usize::from(g.mirror);
        check_laws(&x, &g, &g.act(&x));
    }
    assert!(
        (400..600).contains(&mirrors),
        "{mirrors} of 1000 draws mirror"
    );
    let unit = uniform(&mut rng, 2.0, 2.0);
    assert_eq!(unit, 2.0, "a draw on [2, 2) is {unit}");
}

negative_control!(
    gauge_action,
    "an action whose momenta scale by s^{+1/2} must fail the energy's 1/s",
    expected = "not E/s",
    {
        let mut rng = SplitMix::new(8);
        let x = random_state(&mut rng);
        let g = Gauge {
            s: 4.0,
            ..Gauge::identity()
        };
        let mut gx = g.act(&x);
        for p in gx.p.iter_mut() {
            *p = [p[0] * g.s, p[1] * g.s];
        }
        check_laws(&x, &g, &gx);
    }
);
