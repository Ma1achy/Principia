//! QA's tests for TASK-M2-82 (REQ-CHART-173; chart_composition § 18.7; R-918, R-919 F18), written from the
//! requirement's statement and verify detail and § 18.7's table, not from the implementation. Every test is named
//! `trajectory_interface_qa_*`, so the task's acceptance filter `cargo test -p engine trajectory_interface` runs it.
//!
//! - the interface on a provider of QA's own (§ 18.7: "a test of the interface itself"): a row is the provider's state
//!   bitwise, the last boundary is the greatest `k` with `k·Δ ≤ T` by products, a read past the horizon is refused, a
//!   terminal is held at every later boundary, reads are order-free;
//! - the Lagrange circular member against QA's own closed form (the start rotated by `ωt`, `ω² = G·M/a³` from the
//!   start's measured side), with a bound derived below, every row read within one period, the rows pairwise apart,
//!   and the rows solving Newton's equations for the three bodies (each body's force `−mᵢω²rᵢ`);
//! - the elliptic member: homographic (equilateral at every row, a central configuration), `E` and `L_z` conserved, the
//!   apocentre at half the Kepler period at `(1 + e)/(1 − e)` times the side, back at the start after one period;
//! - the homothetic member: the collapse time `π/√(8μ)` from the measured side, the rows before it on QA's own radial
//!   Kepler solution (`|z| = cos²β`, `β + sin β cos β = √(2μ)·t`), the terminal at the first boundary at or after
//!   `t_c`, kind collision (triple), held, still falling (RQ-757 open: the held state's value is not asserted);
//! - the figure-eight: every boundary row its start bitwise, its quantum `T₈` carried to the sim's units, its start
//!   `validation`'s, and the constants `ledger`'s with the corpus's digits;
//! - controls (R-176): a row one boundary off, a terminal not held, a read depending on an earlier read, a figure-eight
//!   not read from `ledger`'s constants, a state not a rigid rotation, a terminal at the wrong index.

use std::cell::Cell;
use std::f64::consts::{PI, TAU};

use engine::trajectory::{
    same_bits, FigureEight, Lagrange, Member, Provider, Refused, Terminal, Trajectory,
};
use kernel::decode::Ic;
use ledger::display::{Detail, State};
use validation::negative_control;

/// The unit roundoff `u = 2⁻⁵³`.
const U: f64 = f64::EPSILON / 2.0;

fn g() -> f64 {
    ledger::constants::G.number()
}

// ── Small vector helpers ───────────────────────────────────────────────────────────────────────────────────────────

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn rotate(v: [f64; 2], theta: f64) -> [f64; 2] {
    let (s, c) = theta.sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// The three side lengths `|r₀ − r₁|`, `|r₁ − r₂|`, `|r₂ − r₀|`.
fn sides(s: &Ic<f64>) -> [f64; 3] {
    [
        norm(sub(s.r[0], s.r[1])),
        norm(sub(s.r[1], s.r[2])),
        norm(sub(s.r[2], s.r[0])),
    ]
}

fn mean_side(s: &Ic<f64>) -> f64 {
    sides(s).iter().sum::<f64>() / 3.0
}

fn total_mass(s: &Ic<f64>) -> f64 {
    s.m.iter().sum()
}

fn lz(s: &Ic<f64>) -> f64 {
    (0..3).map(|i| cross(s.r[i], s.p[i])).sum()
}

fn energy(s: &Ic<f64>) -> f64 {
    let kinetic: f64 = (0..3).map(|i| dot(s.p[i], s.p[i]) / (2.0 * s.m[i])).sum();
    let mut potential = 0.0;
    for (i, j) in [(0, 1), (1, 2), (2, 0)] {
        potential -= g() * s.m[i] * s.m[j] / norm(sub(s.r[i], s.r[j]));
    }
    kinetic + potential
}

/// The Newtonian force on body `i`: `Σⱼ G·mᵢ·mⱼ·(rⱼ − rᵢ)/|rⱼ − rᵢ|³`.
fn force(s: &Ic<f64>, i: usize) -> [f64; 2] {
    let mut f = [0.0, 0.0];
    for j in 0..3 {
        if j != i {
            let d = sub(s.r[j], s.r[i]);
            let r = norm(d);
            let k = g() * s.m[i] * s.m[j] / (r * r * r);
            f[0] += k * d[0];
            f[1] += k * d[1];
        }
    }
    f
}

/// `μ = G·M/a³` from the start's measured side `a`.
fn mu_of(start: &Ic<f64>) -> f64 {
    let a = mean_side(start);
    g() * total_mass(start) / (a * a * a)
}

// ── The fixtures as the ledger's inputs name them ──────────────────────────────────────────────────────────────────

fn inputs() -> ledger::fixtures::Lagrange {
    ledger::fixtures::lagrange()
}

fn circular() -> Lagrange {
    Lagrange::new(inputs().masses, Member::Circular, inputs().per_period)
}

fn elliptic() -> Lagrange {
    let f = inputs();
    Lagrange::new(
        f.masses,
        Member::Elliptic { e: f.eccentricity },
        f.per_period,
    )
}

fn homothetic(divisions: f64) -> Lagrange {
    Lagrange::new(inputs().masses, Member::Homothetic, divisions)
}

/// A trajectory of `p` from `start` whose last boundary is `last`: the horizon half a quantum past `last·Δ`, so the
/// last boundary does not hang on the product's rounding.
fn upto<P: Provider>(p: &P, start: Ic<f64>, last: u64) -> Trajectory<'_, P> {
    let t = Trajectory::new(p, start, (last as f64 + 0.5) * p.quantum())
        .unwrap_or_else(|e| panic!("the trajectory is refused: {e:?}"));
    assert_eq!(t.last(), last, "the horizon ends at boundary {last}");
    t
}

fn read<P: Provider>(t: &Trajectory<'_, P>, k: u64) -> Ic<f64> {
    t.state(k)
        .unwrap_or_else(|e| panic!("boundary {k} is refused: {e:?}"))
}

// ── A provider of QA's own: the interface itself (§ 18.7, "a test of the interface itself") ─────────────────────────

/// A provider whose state at `k` carries `k` in body 0's `x`, its terminal, if any, at `term`; its own `state` does
/// not hold the terminal, so the holding is the interface's.
struct Steps {
    dt: f64,
    term: Option<u64>,
}

fn steps_start() -> Ic<f64> {
    Ic {
        m: [0.5, 0.25, 0.25],
        r: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        p: [[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
    }
}

impl Provider for Steps {
    fn quantum(&self) -> f64 {
        self.dt
    }

    fn terminal(&self, _start: &Ic<f64>) -> Result<Option<Terminal>, Refused> {
        Ok(self.term.map(|index| Terminal {
            index,
            state: State::Collision,
            detail: Detail::D3,
        }))
    }

    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        let mut s = *start;
        s.r[0][0] += k as f64;
        Ok(s)
    }
}

/// A provider whose state depends on how many reads came before: impure, for the order control.
struct Leaky {
    reads: Cell<u64>,
}

impl Provider for Leaky {
    fn quantum(&self) -> f64 {
        0.25
    }

    fn terminal(&self, _start: &Ic<f64>) -> Result<Option<Terminal>, Refused> {
        Ok(None)
    }

    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        self.reads.set(self.reads.get() + 1);
        let mut s = *start;
        s.r[0][0] += k as f64 + self.reads.get() as f64 * 1e-3;
        Ok(s)
    }
}

/// The greatest `k` with `fl(k·Δ) ≤ T`, by a scan of products from 0: the definition of § 18.2's boundary, no
/// division anywhere.
fn last_by_scan(dt: f64, horizon: f64) -> u64 {
    let mut k = 0u64;
    while ((k + 1) as f64) * dt <= horizon {
        k += 1;
    }
    k
}

#[test]
fn trajectory_interface_qa_grid_by_products() {
    let quanta = [0.1, 0.7, 1.0 / 3.0, PI / 8.0, 1e-3, 0.3, 2.5];
    for dt in quanta {
        let p = Steps { dt, term: None };
        for j in 0..60u64 {
            let at = j as f64 * dt;
            for horizon in [
                at,
                f64::from_bits(at.to_bits().saturating_sub(1)),
                f64::from_bits(at.to_bits() + 1),
                at + 0.5 * dt,
            ] {
                if horizon < 0.0 {
                    continue;
                }
                let t = Trajectory::new(&p, steps_start(), horizon).unwrap();
                assert_eq!(
                    t.last(),
                    last_by_scan(dt, horizon),
                    "Δ = {dt:e}, T = {horizon:e}: the last boundary is the greatest k with k·Δ ≤ T"
                );
            }
        }
    }
    // The cases where a quotient and the products may disagree: 0.3/0.1 rounds below 3 and 3·0.1 above 0.3.
    let p = Steps {
        dt: 0.1,
        term: None,
    };
    assert_eq!(Trajectory::new(&p, steps_start(), 0.3).unwrap().last(), 2);
    assert_eq!(
        Trajectory::new(&p, steps_start(), 3.0 * 0.1)
            .unwrap()
            .last(),
        3
    );
    // T = 0: the start alone.
    let t = Trajectory::new(&p, steps_start(), 0.0).unwrap();
    assert_eq!(t.last(), 0);
    assert!(same_bits(&read(&t, 0), &steps_start()));
}

#[test]
fn trajectory_interface_qa_past_horizon_refused() {
    let p = Steps {
        dt: 0.25,
        term: None,
    };
    let t = upto(&p, steps_start(), 4);
    assert!(t.state(4).is_ok());
    assert_eq!(t.state(5), Err(Refused::PastHorizon { k: 5, last: 4 }));
    assert!(t.state(u64::MAX).is_err());
}

/// Every row `0..=last` read through `t` is the provider's state at that boundary, bit for bit.
fn check_rows_are_providers<P: Provider>(
    p: &P,
    start: &Ic<f64>,
    last: u64,
    row: impl Fn(u64) -> Ic<f64>,
) {
    for k in 0..=last {
        let want = p.state(start, k).unwrap();
        assert!(
            same_bits(&row(k), &want),
            "row {k} is not the provider's state at boundary {k}"
        );
    }
}

#[test]
fn trajectory_interface_qa_row_is_providers_state() {
    let p = Steps {
        dt: 0.125,
        term: None,
    };
    let t = upto(&p, steps_start(), 9);
    check_rows_are_providers(&p, &steps_start(), 9, |k| read(&t, k));
    for k in 0..=9 {
        assert_eq!(t.time(k).to_bits(), (k as f64 * 0.125).to_bits());
    }
    // And on the Lagrange circular member, whose rows all differ (below).
    let c = circular();
    let t = upto(&c, c.start(), 7);
    check_rows_are_providers(&c, &c.start(), 7, |k| read(&t, k));
}

negative_control!(
    trajectory_interface_qa_row_is_providers_state,
    "a row read one boundary off fails, on the Lagrange circular member",
    expected = "is not the provider's state",
    {
        let c = circular();
        let t = upto(&c, c.start(), 8);
        check_rows_are_providers(&c, &c.start(), 7, |k| read(&t, k + 1))
    }
);

/// Every row from `index` to `last` is row `index`, bit for bit: the terminal is held.
fn check_held(index: u64, last: u64, row: impl Fn(u64) -> Ic<f64>) {
    let held = row(index);
    for k in index..=last {
        assert!(
            same_bits(&row(k), &held),
            "row {k}, past the terminal at {index}, is not held"
        );
    }
}

#[test]
fn trajectory_interface_qa_interface_holds_terminal() {
    let p = Steps {
        dt: 0.5,
        term: Some(3),
    };
    let t = upto(&p, steps_start(), 8);
    let term = t
        .terminal()
        .expect("the terminal at 3 is within the horizon");
    assert_eq!(term.index, 3);
    // Before the terminal, the provider's rows; from it on, the terminal's.
    check_rows_are_providers(&p, &steps_start(), 3, |k| read(&t, k));
    check_held(3, 8, |k| read(&t, k));
}

negative_control!(
    trajectory_interface_qa_interface_holds_terminal,
    "a terminal not held fails: the provider's own unheld rows past it",
    expected = "is not held",
    {
        let p = Steps {
            dt: 0.5,
            term: Some(3),
        };
        check_held(3, 8, |k| p.state(&steps_start(), k).unwrap())
    }
);

/// Reads of `0..=last` through fresh trajectories in three orders, each read twice and between reads of another
/// provider, give one bit pattern per boundary.
fn check_order_free<P: Provider>(p: &P, start: Ic<f64>, last: u64) {
    let other = circular();
    let ot = upto(&other, other.start(), 3);
    let n = last + 1;
    let forward: Vec<Ic<f64>> = {
        let t = upto(p, start, last);
        (0..n).map(|k| read(&t, k)).collect()
    };
    let backward: Vec<Ic<f64>> = {
        let t = upto(p, start, last);
        let mut v: Vec<Ic<f64>> = (0..n).rev().map(|k| read(&t, k)).collect();
        v.reverse();
        v
    };
    let t = upto(p, start, last);
    // A stride coprime to n visits every boundary out of order.
    let stride = (1..n).rev().find(|s| gcd(*s, n) == 1).unwrap_or(1);
    let mut scattered = vec![None; n as usize];
    for i in 0..n {
        let k = (i * stride + 1) % n;
        let _ = read(&ot, i % 4);
        let a = read(&t, k);
        let b = read(&t, k);
        assert!(
            same_bits(&a, &b),
            "boundary {k} read twice: the reads depend on order"
        );
        scattered[k as usize] = Some(a);
    }
    for k in 0..n as usize {
        let s = scattered[k].unwrap();
        assert!(
            same_bits(&forward[k], &backward[k]) && same_bits(&forward[k], &s),
            "boundary {k}: the reads depend on order"
        );
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[test]
fn trajectory_interface_qa_reads_order_free() {
    let c = circular();
    check_order_free(&c, c.start(), 7);
    let e = elliptic();
    check_order_free(&e, e.start(), 8);
    let h = homothetic(inputs().per_collapse);
    check_order_free(&h, h.start(), 9);
    let f = FigureEight::new();
    check_order_free(&f, f.start(), 4);
    let s = Steps {
        dt: 0.5,
        term: Some(3),
    };
    check_order_free(&s, steps_start(), 6);
    // A provider built again gives the same bits: purity in `(start, T, k)`, not in the object.
    let (c1, c2) = (circular(), circular());
    let (t1, t2) = (upto(&c1, c1.start(), 7), upto(&c2, c2.start(), 7));
    for k in (0..=7).rev() {
        assert!(same_bits(&read(&t1, k), &read(&t2, k)));
    }
}

negative_control!(
    trajectory_interface_qa_reads_order_free,
    "a read depending on an earlier read fails",
    expected = "the reads depend on order",
    {
        let p = Leaky {
            reads: Cell::new(0),
        };
        check_order_free(&p, steps_start(), 5)
    }
);

// ── The Lagrange circular member ───────────────────────────────────────────────────────────────────────────────────

/// The start's gauge (§ 18.7, "the scale gauge"): the masses `ledger`'s, the centre of mass at 0, `Σp = 0`, `I = 1`,
/// equilateral. The bounds: each sum is a few roundings of terms below 1, so `16u`.
fn check_lagrange_start(s: &Ic<f64>) {
    assert_eq!(s.m.map(f64::to_bits), inputs().masses.map(f64::to_bits));
    let tol = 16.0 * U;
    for d in 0..2 {
        let com: f64 = (0..3).map(|i| s.m[i] * s.r[i][d]).sum();
        let mom: f64 = (0..3).map(|i| s.p[i][d]).sum();
        assert!(com.abs() <= tol, "the centre of mass is at {com:e}");
        assert!(mom.abs() <= tol, "the total momentum is {mom:e}");
    }
    let inertia: f64 = (0..3).map(|i| s.m[i] * dot(s.r[i], s.r[i])).sum();
    assert!((inertia - 1.0).abs() <= tol, "I = {inertia}");
    let [a, b, c] = sides(s);
    assert!(
        (a - b).abs() <= tol * a && (b - c).abs() <= tol * a,
        "the start is not equilateral: {a} {b} {c}"
    );
}

/// Row `k` against QA's closed form: the start rotated by `θ = ω·k·Δ`, `ω = √(G·M/a³)` from the start's measured side.
///
/// The bound, per component, `256u` times the body's `|r|` or `|p|`: `ω` from the measured side differs from the
/// provider's by a few `u` relatively (the side, a hypot of differences, `≤ 4u`; `ω ∝ a^{−3/2}`, `≤ 8u` with the
/// root and quotient), `θ ≤ 2π` within the period, so `|δθ| ≤ 2π·(8u + 4u) < 80u` with `Δ`'s and the product's
/// roundings; `sin` and `cos` within an ulp on either side, `≤ 4u`; the complex products, `≤ 4u`; under `100u`, so
/// `256u` leaves room and is still `10¹³` times below the `45°` between neighbouring rows.
fn check_circular_row(start: &Ic<f64>, dt: f64, k: u64, got: &Ic<f64>) {
    let w = mu_of(start).sqrt();
    let theta = w * (k as f64 * dt);
    for i in 0..3 {
        for (got, was, what) in [(got.r[i], start.r[i], "r"), (got.p[i], start.p[i], "p")] {
            let want = rotate(was, theta);
            let bound = 256.0 * U * norm(was);
            for d in 0..2 {
                assert!(
                    (got[d] - want[d]).abs() <= bound,
                    "row {k}: {what}{i}[{d}] = {} is not the closed form's rotation {} (bound {bound:e})",
                    got[d],
                    want[d]
                );
            }
        }
    }
}

#[test]
fn trajectory_interface_qa_circular_closed_form() {
    let c = circular();
    let s = c.start();
    check_lagrange_start(&s);
    let w = mu_of(&s).sqrt();
    // The circular velocity: pᵢ = mᵢ·ω·(−yᵢ, xᵢ).
    for i in 0..3 {
        let want = [-s.m[i] * w * s.r[i][1], s.m[i] * w * s.r[i][0]];
        assert!(
            norm(sub(s.p[i], want)) <= 32.0 * U * norm(s.p[i]),
            "body {i} is not at the circular velocity"
        );
    }
    // The period 2π/ω and the quantum: every row read (0 to 7) lies within one period, kΔ < 2π/ω.
    let period = TAU / w;
    assert!((c.period().unwrap() - period).abs() <= 64.0 * U * period);
    let t = upto(&c, s, 7);
    assert!(
        7.0 * t.quantum() < period,
        "a row read lies past one period"
    );
    for k in 0..=7 {
        check_circular_row(&s, t.quantum(), k, &read(&t, k));
    }
    assert!(same_bits(&read(&t, 0), &s), "row 0 is the start");
}

negative_control!(
    trajectory_interface_qa_circular_closed_form,
    "a row one boundary off is not the closed form's",
    expected = "is not the closed form's rotation",
    {
        let c = circular();
        let t = upto(&c, c.start(), 8);
        check_circular_row(&c.start(), t.quantum(), 3, &read(&t, 4))
    }
);

/// Every two rows are apart by more than a tenth of the smallest body's distance from the centre: the rows within one
/// period are rotations by distinct angles in `(0, 2π)`, at least `2π/8` apart.
fn check_rows_apart(rows: &[Ic<f64>]) {
    let scale = (0..3)
        .map(|i| norm(rows[0].r[i]))
        .fold(f64::INFINITY, f64::min);
    for (i, a) in rows.iter().enumerate() {
        for (j, b) in rows.iter().enumerate().skip(i + 1) {
            let gap = (0..3)
                .map(|n| norm(sub(a.r[n], b.r[n])))
                .fold(0.0, f64::max);
            assert!(gap > 0.1 * scale, "rows {i} and {j} do not differ");
        }
    }
}

#[test]
fn trajectory_interface_qa_circular_rows_apart() {
    let c = circular();
    let t = upto(&c, c.start(), 7);
    check_rows_apart(&(0..=7).map(|k| read(&t, k)).collect::<Vec<_>>());
}

negative_control!(
    trajectory_interface_qa_circular_rows_apart,
    "the figure-eight's rows, all its start, are not apart",
    expected = "do not differ",
    {
        let f = FigureEight::new();
        let t = upto(&f, f.start(), 3);
        check_rows_apart(&(0..=3).map(|k| read(&t, k)).collect::<Vec<_>>())
    }
);

/// `s` is a rigid rotation under gravity: each body's Newtonian force is `−mᵢ·ω²·rᵢ`, with `ω = |p₀|/(m₀·|r₀|)`
/// measured from the state. Relative `10⁻¹²`: the forces are sums of three terms of a few roundings each, the measured
/// `ω²` a few more, `~100u`, well inside.
fn check_rigid_under_gravity(s: &Ic<f64>, k: u64) {
    let w = norm(s.p[0]) / (s.m[0] * norm(s.r[0]));
    for i in 0..3 {
        let f = force(s, i);
        let want = [-s.m[i] * w * w * s.r[i][0], -s.m[i] * w * w * s.r[i][1]];
        assert!(
            norm(sub(f, want)) <= 1e-12 * norm(f),
            "row {k}: body {i} is not a rigid rotation under gravity: F = {f:?}, −mω²r = {want:?}"
        );
    }
}

#[test]
fn trajectory_interface_qa_circular_solves_newton() {
    let c = circular();
    let t = upto(&c, c.start(), 7);
    let lz0 = lz(&c.start());
    let e0 = energy(&c.start());
    assert!(lz0 > 0.0, "the start turns counter-clockwise");
    for k in 0..=7 {
        let s = read(&t, k);
        check_rigid_under_gravity(&s, k);
        // The mirror never folded: L_z keeps its sign and value.
        assert!((lz(&s) - lz0).abs() <= 1e-13 * lz0.abs(), "row {k}: L_z");
        assert!((energy(&s) - e0).abs() <= 1e-13 * e0.abs(), "row {k}: E");
    }
}

negative_control!(
    trajectory_interface_qa_circular_solves_newton,
    "a state spinning one per cent too fast is no rigid rotation under gravity",
    expected = "is not a rigid rotation under gravity",
    {
        let mut s = circular().start();
        for i in 0..3 {
            s.p[i] = s.p[i].map(|x| 1.01 * x);
        }
        check_rigid_under_gravity(&s, 0)
    }
);

// ── The Lagrange elliptic member ───────────────────────────────────────────────────────────────────────────────────

/// `s` is a central configuration: equilateral, and each body's force pointing at the centre of mass with
/// `|Fᵢ|/(mᵢ|rᵢ|)` the same for all three.
fn check_central(s: &Ic<f64>, k: u64) {
    let [a, b, c] = sides(s);
    assert!(
        (a - b).abs() <= 64.0 * U * a && (b - c).abs() <= 64.0 * U * a,
        "row {k} is not equilateral: {a} {b} {c}"
    );
    let ratio = |i: usize| norm(force(s, i)) / (s.m[i] * norm(s.r[i]));
    for i in 0..3 {
        let f = force(s, i);
        assert!(
            cross(f, s.r[i]).abs() <= 1e-12 * norm(f) * norm(s.r[i]) && dot(f, s.r[i]) < 0.0,
            "row {k}: body {i}'s force does not point at the centre of mass"
        );
        assert!(
            (ratio(i) - ratio(0)).abs() <= 1e-12 * ratio(0),
            "row {k}: not central"
        );
    }
}

#[test]
fn trajectory_interface_qa_elliptic_homographic() {
    let p = elliptic();
    let s = p.start();
    check_lagrange_start(&s);
    let e = inputs().eccentricity;
    let mu = mu_of(&s);
    // From pericentre at the side a: the Kepler period 2π/n, n = √(μ/A³), A = 1/(1 − e).
    let big_a = 1.0 / (1.0 - e);
    let period = TAU / (mu / (big_a * big_a * big_a)).sqrt();
    assert!((p.period().unwrap() - period).abs() <= 64.0 * U * period);
    assert!((8.0 * p.quantum() - period).abs() <= 64.0 * U * period);
    let t = upto(&p, s, 8);
    let (lz0, e0, a0) = (lz(&s), energy(&s), mean_side(&s));
    let rows: Vec<Ic<f64>> = (0..=8).map(|k| read(&t, k)).collect();
    for (k, r) in rows.iter().enumerate() {
        let k = k as u64;
        check_central(r, k);
        assert!(
            (lz(r) - lz0).abs() <= 1e-12 * lz0.abs(),
            "row {k}: L_z not conserved"
        );
        assert!(
            (energy(r) - e0).abs() <= 1e-12 * e0.abs(),
            "row {k}: E not conserved"
        );
    }
    // Half a period: the apocentre, the side (1 + e)/(1 − e) times the start's, the velocity perpendicular to it.
    let apo = &rows[4];
    let ratio = mean_side(apo) / a0;
    assert!(
        (ratio - (1.0 + e) / (1.0 - e)).abs() <= 1e-12,
        "the apocentre's side is {ratio} times the pericentre's"
    );
    for i in 0..3 {
        assert!(dot(apo.r[i], apo.p[i]).abs() <= 1e-12 * norm(apo.r[i]) * norm(apo.p[i]));
    }
    // Between: the side grows to the apocentre and shrinks back.
    for k in 0..4 {
        assert!(mean_side(&rows[k + 1]) > mean_side(&rows[k]));
        assert!(mean_side(&rows[k + 5]) < mean_side(&rows[k + 4]));
    }
    // One period: back at the start.
    for i in 0..3 {
        assert!(
            norm(sub(rows[8].r[i], s.r[i])) <= 1e-12 * norm(s.r[i]),
            "row 8: r{i}"
        );
        assert!(
            norm(sub(rows[8].p[i], s.p[i])) <= 1e-12 * norm(s.p[i]),
            "row 8: p{i}"
        );
    }
    check_rows_apart(&rows[..8]);
}

// ── The Lagrange homothetic member ─────────────────────────────────────────────────────────────────────────────────

/// QA's radial Kepler solution from rest at `|z| = 1` under `μ`: `|z| = cos²β`, `t = (β + sin β cos β)/√(2μ)`, `β` by
/// bisection on `[0, π/2]`, where the right side is monotone; the speed `|ż| = √(2μ(1/|z| − 1))` from the energy.
fn radial(mu: f64, t: f64) -> (f64, f64) {
    let target = (2.0 * mu).sqrt() * t;
    let (mut lo, mut hi) = (0.0f64, PI / 2.0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid + mid.sin() * mid.cos() < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let beta = 0.5 * (lo + hi);
    let z = beta.cos() * beta.cos();
    (z, (2.0 * mu * (1.0 / z - 1.0)).max(0.0).sqrt())
}

/// The terminal is a triple collision at the first boundary at or after `t_c`.
fn check_terminal(term: Option<Terminal>, dt: f64, tc: f64) {
    let term = term.expect("the collapse has a terminal within the horizon");
    let k = term.index;
    assert!(
        k as f64 * dt >= tc * (1.0 - 1e-12) && (k as f64 - 1.0) * dt < tc * (1.0 - 1e-12),
        "the terminal at {k} is not the first boundary at or after t_c"
    );
    assert_eq!(term.state, State::Collision, "the collapse is a collision");
    assert_eq!(
        term.detail,
        Detail::D3,
        "the collapse is a triple collision"
    );
}

#[test]
fn trajectory_interface_qa_homothetic_collapse() {
    let f = inputs();
    for (divisions, index) in [(f.per_collapse, 6u64), (f.per_collapse_on_boundary, 4u64)] {
        let h = homothetic(divisions);
        let s = h.start();
        check_lagrange_start(&s);
        assert!(
            s.p.iter().all(|p| *p == [0.0, 0.0]),
            "the collapse starts from rest"
        );
        let mu = mu_of(&s);
        let tc = PI / (8.0 * mu).sqrt();
        assert!(h.period().is_none(), "the homothetic member has no period");
        assert!(
            (h.collapse().unwrap() - tc).abs() <= 64.0 * U * tc,
            "t_c = π/√(8μ)"
        );
        assert!((divisions * h.quantum() - tc).abs() <= 64.0 * U * tc);
        let last = index + 4;
        let t = upto(&h, s, last);
        check_terminal(t.terminal(), t.quantum(), tc);
        assert_eq!(t.terminal().unwrap().index, index);
        let a0 = mean_side(&s);
        let mut previous = f64::INFINITY;
        for k in 0..index {
            let r = read(&t, k);
            let (z, zdot) = radial(mu, k as f64 * t.quantum());
            let got = mean_side(&r) / a0;
            assert!(
                (got - z).abs() <= 1e-12,
                "row {k}: |z| = {got}, the radial solution's {z}"
            );
            for i in 0..3 {
                let speed = norm(r.p[i]) / (r.m[i] * norm(s.r[i]));
                assert!(
                    (speed - zdot).abs() <= 1e-10 * zdot.max(1.0),
                    "row {k}: body {i}'s speed {speed}, the radial solution's {zdot}"
                );
            }
            assert!(got < previous, "row {k}: the collapse shrinks");
            previous = got;
            assert!((energy(&r) - energy(&s)).abs() <= 1e-12 * energy(&s).abs());
        }
        // Held from the terminal on, still falling: apart, finite, every body moving towards the centre of mass.
        check_held(index, last, |k| read(&t, k));
        let held = read(&t, index);
        for i in 0..3 {
            assert!(norm(held.r[i]) > 0.0 && norm(held.r[i]).is_finite());
            assert!(norm(held.p[i]).is_finite());
            assert!(
                dot(held.r[i], held.p[i]) < 0.0,
                "the held state's body {i} is not falling"
            );
        }
        // A horizon ending before the collapse: no terminal, nothing held.
        let before = upto(&h, s, index - 1);
        assert!(
            before.terminal().is_none(),
            "no terminal within a horizon before t_c"
        );
        // The collapse's rows are not rigid rotations, so not the circular member's gap check: each is its own bits.
        let rows: Vec<Ic<f64>> = (0..index).map(|k| read(&before, k)).collect();
        for (i, a) in rows.iter().enumerate() {
            for (j, b) in rows.iter().enumerate().skip(i + 1) {
                assert!(!same_bits(a, b), "rows {i} and {j} before t_c are held");
            }
        }
    }
}

negative_control!(
    trajectory_interface_qa_homothetic_collapse,
    "a terminal a boundary early, before t_c, fails",
    expected = "is not the first boundary at or after t_c",
    {
        let h = homothetic(inputs().per_collapse);
        let tc = PI / (8.0 * mu_of(&h.start())).sqrt();
        let p = Steps {
            dt: h.quantum(),
            term: Some(5),
        };
        let t = upto(&p, steps_start(), 9);
        check_terminal(t.terminal(), t.quantum(), tc)
    }
);

// ── The figure-eight ───────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn trajectory_interface_qa_figure_eight_rows_are_start() {
    let f = FigureEight::new();
    let t = upto(&f, f.start(), 6);
    let zero = read(&t, 0);
    assert!(same_bits(&zero, &f.start()));
    for k in 0..=6 {
        assert!(same_bits(&read(&t, k), &zero), "row {k}·T₈ is not row 0");
    }
    assert_eq!(t.time(5).to_bits(), (5.0 * f.quantum()).to_bits());
}

#[test]
fn trajectory_interface_qa_figure_eight_period_in_sim_units() {
    // The published period, masses 1/3 at G = 1 (√3 longer, the accelerations a third), then the canonical scale
    // λ = I^{−1/2} on the positions (λ^{3/2} longer): QA's own carrying of T₈.
    let c = ledger::fixtures::figure_eight();
    let inertia = (1.0 / 3.0) * 2.0 * dot(c.x1, c.x1);
    let lambda = 1.0 / inertia.sqrt();
    let want = c.period * 3f64.sqrt() * lambda.powf(1.5);
    let f = FigureEight::new();
    assert!(
        (f.quantum() - want).abs() <= 64.0 * U * want,
        "the quantum {} is not T₈ in the sim's units {want}",
        f.quantum()
    );
    // Its start: equal masses, I = 1, Σp = 0, L_z = 0 (dd_validation_orbits § 1.2).
    let s = f.start();
    assert!(s.m.iter().all(|m| (m - 1.0 / 3.0).abs() <= U));
    let inertia: f64 = (0..3).map(|i| s.m[i] * dot(s.r[i], s.r[i])).sum();
    assert!((inertia - 1.0).abs() <= 16.0 * U);
    assert!(lz(&s).abs() <= 1e-14, "L_z = {}", lz(&s));
}

/// `f`'s start is `validation`'s `figure_eight()`'s, bit for bit.
fn check_reads_validation(f: &FigureEight) {
    let v = validation::drift::figure_eight().ic;
    assert!(
        same_bits(&f.start(), &v),
        "the fixture's start is not validation's figure_eight(): it does not read ledger's constants"
    );
}

#[test]
fn trajectory_interface_qa_figure_eight_constants_in_ledger() {
    let c = ledger::fixtures::figure_eight();
    // The corpus's digits (dd_integrator § 5, test 2; dd_validation_orbits § 0).
    assert_eq!(c.x1, [0.970_004_36, -0.243_087_53]);
    assert_eq!(c.v3, [-0.932_407_37, -0.864_731_46]);
    assert_eq!(c.period, 6.325_913_98);
    // validation reads them from ledger.
    assert_eq!(validation::drift::FIGURE_EIGHT_X1, c.x1);
    assert_eq!(validation::drift::FIGURE_EIGHT_V3, c.v3);
    check_reads_validation(&FigureEight::new());
    check_reads_validation(&FigureEight::from_constants(&c));
}

negative_control!(
    trajectory_interface_qa_figure_eight_constants_in_ledger,
    "a figure-eight built from other constants is not validation's",
    expected = "does not read ledger's constants",
    {
        let mut c = ledger::fixtures::figure_eight();
        c.x1[0] += 1e-8;
        check_reads_validation(&FigureEight::from_constants(&c))
    }
);
