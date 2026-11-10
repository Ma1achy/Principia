//! The trajectory interface and its exact fixtures (chart_composition § 18.7; R-918, R-919 F18; REQ-CHART-173;
//! TASK-M2-82), every test named `trajectory_interface_*`:
//! - `_row_reads_provider_bitwise`: a row reads the provider's state, bit for bit, on the Lagrange circular member;
//! - `_circular_rows_differ`: the circular member's rows read, every one within one period (`kΔ < 2π/ω`), differ
//!   pairwise, so a check that tells boundaries apart can fire on them; on the figure-eight it could not;
//! - `_figure_eight_row_at_k_period_is_row_zero`: the figure-eight's row at `kT₈` is its row at 0, bit for bit;
//! - `_lagrange_within_bound`: the Lagrange members' states within the bounds derived from the closed form's
//!   evaluation (each bound's derivation at its function);
//! - `_lagrange_starts`: each member's start is the configuration the closed form names, measured from the start:
//!   equilateral, at `I = 1`, its period or collapse time from its measured side, the circular one's speeds `ω·|rᵢ|`;
//! - `_figure_eight_period`: the figure-eight's quantum is `T₈` carried to the sim's units;
//! - `_terminal_index_and_kind`, `_terminal_held`: the homothetic collapse's terminal at its index and kind, and held;
//! - `_reads_order_free`: two reads in either order, and after other reads, give one bit pattern;
//! - `_grid`: the last boundary at the horizon, by products of indices, and the refusals;
//! - `_kepler_equation`: the eccentric anomaly solves Kepler's equation, the bracket's bisection included;
//! - `_validation_reads_ledger_constants`: `validation`'s `figure_eight()` is the start built from `ledger::fixtures`.

use std::f64::consts::TAU;

use engine::trajectory::{
    eccentric_anomaly, same_bits, FigureEight, Lagrange, Member, Provider, Refused, Terminal,
    Trajectory,
};
use kernel::decode::Ic;
use ledger::display::{Detail, State};
use validation::negative_control;

/// The unit roundoff, `2⁻⁵³`.
const U: f64 = f64::EPSILON / 2.0;

fn fixture() -> ledger::fixtures::Lagrange {
    ledger::fixtures::lagrange()
}

fn circular() -> Lagrange {
    Lagrange::new(fixture().masses, Member::Circular, fixture().per_period)
}

fn elliptic() -> Lagrange {
    let f = fixture();
    Lagrange::new(
        f.masses,
        Member::Elliptic { e: f.eccentricity },
        f.per_period,
    )
}

fn homothetic(divisions: f64) -> Lagrange {
    Lagrange::new(fixture().masses, Member::Homothetic, divisions)
}

/// `provider` from its start to `last` boundaries' worth of horizon, `last·Δ`.
fn over<P: Provider>(provider: &P, start: Ic<f64>, last: u64) -> Trajectory<'_, P> {
    let t = Trajectory::new(provider, start, last as f64 * provider.quantum())
        .unwrap_or_else(|e| panic!("refused: {e:?}"));
    assert_eq!(
        t.last(),
        last,
        "the horizon `last·Δ` ends at boundary {last}"
    );
    t
}

fn row<P: Provider>(t: &Trajectory<'_, P>, k: u64) -> Ic<f64> {
    t.state(k)
        .unwrap_or_else(|e| panic!("row {k} refused: {e:?}"))
}

// ── A row reads the provider's state ───────────────────────────────────────────────────────────────────────────────

/// The row read at `read` is the provider's state at boundary `k`, bit for bit.
fn check_row(provider: &Lagrange, k: u64, read: u64) {
    let t = over(provider, provider.start(), 7);
    let want = provider.state(&provider.start(), k).unwrap();
    assert!(
        same_bits(&row(&t, read), &want),
        "row {read} is not the provider's state at boundary {k}"
    );
}

#[test]
fn trajectory_interface_row_reads_provider_bitwise() {
    let p = circular();
    for k in 0..=7 {
        check_row(&p, k, k);
    }
    let t = over(&p, p.start(), 7);
    assert!(same_bits(&row(&t, 0), &p.start()), "row 0 is the start");
    assert_eq!(t.time(3).to_bits(), (3.0 * p.quantum()).to_bits());
    assert_eq!(t.quantum().to_bits(), p.quantum().to_bits());
}

negative_control!(
    trajectory_interface_row_reads_provider_bitwise,
    "a row read one boundary off fails",
    expected = "is not the provider's state",
    check_row(&circular(), 3, 4)
);

/// Every two of `rows` differ.
fn check_distinct(rows: &[Ic<f64>]) {
    for (i, a) in rows.iter().enumerate() {
        for (j, b) in rows.iter().enumerate().skip(i + 1) {
            assert!(!same_bits(a, b), "rows {i} and {j} are the same state");
        }
    }
}

#[test]
fn trajectory_interface_circular_rows_differ() {
    let p = circular();
    let period = p.period().unwrap();
    assert!(
        7.0 * p.quantum() < period,
        "every row read lies within one period"
    );
    let t = over(&p, p.start(), 7);
    check_distinct(&(0..=7).map(|k| row(&t, k)).collect::<Vec<_>>());
}

negative_control!(
    trajectory_interface_circular_rows_differ,
    "the figure-eight's rows, all its start, fail the check that tells boundaries apart",
    expected = "are the same state",
    {
        let p = FigureEight::new();
        let t = over(&p, p.start(), 3);
        check_distinct(&(0..=3).map(|k| row(&t, k)).collect::<Vec<_>>())
    }
);

// ── The figure-eight ───────────────────────────────────────────────────────────────────────────────────────────────

/// Every row of `t` up to `last` is its row 0, bit for bit.
fn check_rows_are_row_zero<P: Provider>(t: &Trajectory<'_, P>) {
    let zero = row(t, 0);
    for k in 1..=t.last() {
        assert!(same_bits(&row(t, k), &zero), "row {k} differs from row 0");
    }
}

#[test]
fn trajectory_interface_figure_eight_row_at_k_period_is_row_zero() {
    let p = FigureEight::new();
    let horizon = ledger::constants::T_HORIZON_MAX.number();
    let t = Trajectory::new(&p, p.start(), horizon).unwrap();
    assert!(t.last() >= 10, "the horizon cap holds many periods");
    assert!(same_bits(&row(&t, 0), &p.start()));
    assert_eq!(t.terminal(), None);
    check_rows_are_row_zero(&t);
}

negative_control!(
    trajectory_interface_figure_eight_row_at_k_period_is_row_zero,
    "the circular member's rows, which move, fail",
    expected = "differs from row 0",
    {
        let p = circular();
        check_rows_are_row_zero(&over(&p, p.start(), 2))
    }
);

// ── The Lagrange states within the derived bounds ──────────────────────────────────────────────────────────────────

/// `|v_x| + |v_y|`.
fn l1(v: [f64; 2]) -> f64 {
    v[0].abs() + v[1].abs()
}

/// `v` turned by `θ`.
fn turn(v: [f64; 2], theta: f64) -> [f64; 2] {
    let (s, c) = (libm::sin(theta), libm::cos(theta));
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// The circular member's bound: 51u per unit of a vector's `|v_x| + |v_y|`, for each component of `rᵢ` and `pᵢ` at
/// boundary `k ≤ 7` against the start's vector turned by the test's `θ = 2π·k/8`, `u = 2⁻⁵³`, libm's `sin` and `cos`
/// under one ulp, so at most `2u` on values of magnitude at most 1. The terms:
/// - the provider's angle `ω̂·fl(k·fl(P/8))`, `P = fl(2π/ω̂)`, is `θ(1 + δ)⁴` (`2π`'s representation, the division,
///   the product by `k`, the product by `ω̂`; `/8` exact), so off by at most `4.01u·θ ≤ 22.1u` at `θ ≤ 7π/4`; the
///   test's `fl(fl(2π·k)/8)` by `3.01u·θ ≤ 16.6u`: the two angles within `38.7u`;
/// - `sin` and `cos` each `2u` on either side: the two `(cos, sin)` within `42.7u`;
/// - a position: the provider's complex product, two products and a sum, `2u`, and the test's turn, `2u`: `46.7u`;
/// - a momentum: the provider's `ż = ω̂·(−sin, cos)`, `u`, its complex product, `2u`, and the mass, `u`; the start's
///   momentum, `2u`, and the test's turn, `2u`: `50.7u`. Both within 51u.
fn circular_bound(v: [f64; 2]) -> f64 {
    51.0 * U * l1(v)
}

/// Each `rᵢ` and `pᵢ` of `got` within `bound` of `want`'s.
fn check_within(got: &Ic<f64>, want: &Ic<f64>, bound: impl Fn([f64; 2]) -> f64, what: &str) {
    let pairs = got.r.iter().zip(&want.r).chain(got.p.iter().zip(&want.p));
    for (g, w) in pairs {
        for d in 0..2 {
            let (err, b) = ((g[d] - w[d]).abs(), bound(*w));
            assert!(err <= b, "{what}: {err:e} exceeds the bound {b:e}");
        }
    }
}

/// The circular member's row `k` against the start turned by `2π·at/8`.
fn check_circular(k: u64, at: u64) {
    let p = circular();
    let start = p.start();
    let t = over(&p, start, 7);
    let theta = TAU * at as f64 / fixture().per_period;
    let want = Ic {
        m: start.m,
        r: start.r.map(|v| turn(v, theta)),
        p: start.p.map(|v| turn(v, theta)),
    };
    check_within(
        &row(&t, k),
        &want,
        circular_bound,
        &format!("circular row {k}"),
    );
}

/// The 3-body angular momentum `Σ (xᵢp_yᵢ − yᵢp_xᵢ)`.
fn lz(s: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| s.r[i][0] * s.p[i][1] - s.r[i][1] * s.p[i][0])
        .sum()
}

/// The 3-body energy `Σ |pᵢ|²/2mᵢ − G·Σ mᵢmⱼ/rᵢⱼ`.
fn energy(s: &Ic<f64>) -> f64 {
    let g = ledger::constants::G.number();
    let kinetic: f64 = (0..3)
        .map(|i| (s.p[i][0].powi(2) + s.p[i][1].powi(2)) / (2.0 * s.m[i]))
        .sum();
    let potential: f64 = [(0, 1), (1, 2), (2, 0)]
        .iter()
        .map(|&(i, j)| {
            let d = [s.r[i][0] - s.r[j][0], s.r[i][1] - s.r[j][1]];
            s.m[i] * s.m[j] / (d[0] * d[0] + d[1] * d[1]).sqrt()
        })
        .sum();
    kinetic - g * potential
}

/// The elliptic member's `L_z` at each boundary within `123u·|L₀|` of the start's, at `e = 1/2`. `L_z` is conserved
/// at every eccentric anomaly, so the error of the anomaly drops out: at the float `E` the provider holds, the exact
/// `z × ż = A·b·n·(c² − e·c + s²)/(1 − e·c)` is `A·b·n` (`A = 1/(1 − e)`, `b = A·√(1 − e²)`). The terms, relative to
/// `|L₀|`: libm's `c`, `s` put `c² + s²` within `5u` of 1, `5u/(1 − e)` = `10u`; the components `A(c − e)`, `b·s`,
/// `A·n·s/d`, `b·n·c/d` within `2u`, `4u`, `10u`, `13u` relative (`fl(c − e)` rounds relative to its result; `n` to
/// `3.5u`, `b` to `3u`, `d = 1 − e·c` to `3u`), their cross product, two terms each at most `(1 + e)/(1 − e) = 3`
/// times the result, to `3·16u + u = 49u`; `n` and `b` against the start's `√(μ(1 + e))`, `9u`; each state's
/// assembly, `rᵢ = z·cᵢ` and `pᵢ = mᵢ·(ż·cᵢ)` with `|z||ż|/|z × ż| ≤ 1/√(1 − e²)` and `(|c_x| + |c_y|)² ≤ 2|c|²`,
/// `26u`, twice (the row and the start); and the sums, `3u`: `10 + 49 + 9 + 52 + 3 = 123u`.
fn elliptic_lz_bound(lz0: f64) -> f64 {
    123.0 * U * lz0.abs()
}

/// The elliptic member's closure at its Kepler period, boundary 8, `e = 1/2`, per unit of a vector's
/// `|v_x| + |v_y|`: positions within `(3A + 3)u + b·δE`, momenta within `(A/b)·δE + 26u`, `A = 2`, `b = √3`, with
/// `δE = 44u/(1 − e) + 8u = 96u` the anomaly's error at the period: the mean anomaly `n̂·fl(2π/n̂)` within `3.01u·2π
/// ≤ 20u` of `2π`, the bisection's stop within `24u` of residual (three ulps of `2π`), each divided by `1 − e·cos E =
/// 1 − e`, and one ulp of `2π`, `8u`. Then `z_x = A(cos E − e)` within `A·3u + u` of 1, `z_y = b·sin E` within
/// `b·δE`, `ż_x = −A·n·sin E/d` within `(A/b)·|ż₀|·δE`, `ż_y` within `20u` of `ż₀ = √(μ(1 + e))` (its two
/// evaluations' rounding), and each state's assembly `2u` for positions and `6u` for momenta (with the start's).
/// Positions `175u`, momenta `137u`: the bound reads `175u`.
fn elliptic_closure_bound(v: [f64; 2]) -> f64 {
    175.0 * U * l1(v)
}

/// The homothetic member's energy at a boundary where `|z| = z`, within `μ·u·(25(1/z − 1) + 12/z + 5/(4z²) + 16)` of
/// the start's, `E₀ = −μ` (`I = 1`). Energy is conserved at every eccentric anomaly: at the float `E`,
/// `ż²/2 − μ/z = (μ/A)(−1/2 + δ/(2(1 − c)²))`, `δ = s² + c² − 1`, `|δ| ≤ 5u` from libm, so `5u·μ/(1 − c)²` with
/// `1 − c = 2z`, `5μu/(4z²)`; the kinetic term `μ/z − μ` within `25u` relative (`ż` to `7.5u`, squared, and its
/// assembly and sum, `10u`); the potential `μ/z` within `12u` (positions, differences, the square root, the
/// division and the sum); the start's potential `μ`, `12u`, and `I`'s rounding at the start, `4u`.
fn homothetic_energy_bound(mu: f64, z: f64) -> f64 {
    mu * U * (25.0 * (1.0 / z - 1.0) + 12.0 / z + 5.0 / (4.0 * z * z) + 16.0)
}

/// The elliptic member's `L_z` at boundary `k` against the start's `lz0`.
fn check_lz(row: &Ic<f64>, lz0: f64, k: u64) {
    let (err, b) = ((lz(row) - lz0).abs(), elliptic_lz_bound(lz0));
    assert!(
        err <= b,
        "elliptic L_z at row {k}: {err:e} exceeds the bound {b:e}"
    );
}

#[test]
fn trajectory_interface_lagrange_within_bound() {
    for k in 0..=7 {
        check_circular(k, k);
    }
    let p = elliptic();
    let t = over(&p, p.start(), 8);
    assert_eq!(
        t.time(8).to_bits(),
        p.period().unwrap().to_bits(),
        "boundary 8 is the period"
    );
    let lz0 = lz(&p.start());
    for k in 1..=8 {
        check_lz(&row(&t, k), lz0, k);
    }
    check_within(
        &row(&t, 8),
        &p.start(),
        elliptic_closure_bound,
        "elliptic closure",
    );
    let h = homothetic(fixture().per_collapse);
    let t = over(&h, h.start(), 5);
    let e0 = energy(&h.start());
    let c0 = h.start().r[0];
    for k in 1..=5 {
        let s = row(&t, k);
        let z = l1(s.r[0]) / l1(c0);
        assert!(z < 1.0, "row {k} has collapsed inward");
        let (err, b) = ((energy(&s) - e0).abs(), homothetic_energy_bound(-e0, z));
        assert!(
            err <= b,
            "homothetic energy at row {k}: {err:e} exceeds the bound {b:e}"
        );
    }
}

negative_control!(
    trajectory_interface_lagrange_within_bound,
    "a circular row against the next boundary's closed form fails",
    expected = "exceeds the bound",
    check_circular(3, 4)
);

// ── The terminal ───────────────────────────────────────────────────────────────────────────────────────────────────

/// The homothetic member at `divisions` to `last` boundaries has its terminal `want`.
fn check_terminal(divisions: f64, last: u64, want: Option<u64>) {
    let p = homothetic(divisions);
    let got = over(&p, p.start(), last).terminal();
    let want = want.map(|index| Terminal {
        index,
        state: State::Collision,
        detail: Detail::D3,
    });
    assert_eq!(
        got, want,
        "the terminal at {divisions} divisions, horizon {last}"
    );
}

#[test]
fn trajectory_interface_terminal_index_and_kind() {
    let f = fixture();
    check_terminal(f.per_collapse, 10, Some(6));
    check_terminal(f.per_collapse, 6, Some(6));
    check_terminal(f.per_collapse, 5, None);
    check_terminal(f.per_collapse_on_boundary, 10, Some(4));
    for p in [circular(), elliptic()] {
        assert_eq!(over(&p, p.start(), 8).terminal(), None);
        assert_eq!(p.collapse(), None);
        assert_eq!(p.terminal(&p.start()), Ok(None));
    }
    let h = homothetic(f.per_collapse);
    assert_eq!(h.period(), None);
    assert_eq!(
        h.quantum().to_bits(),
        (h.collapse().unwrap() / f.per_collapse).to_bits()
    );
}

negative_control!(
    trajectory_interface_terminal_index_and_kind,
    "a terminal expected one boundary early fails",
    expected = "the terminal at",
    check_terminal(fixture().per_collapse, 10, Some(5))
);

/// Every row of `t` from `index` on is row `index`.
fn check_held<P: Provider>(t: &Trajectory<'_, P>, index: u64) {
    let held = row(t, index);
    for k in index + 1..=t.last() {
        assert!(
            same_bits(&row(t, k), &held),
            "row {k} past the terminal at {index} is not held"
        );
    }
}

#[test]
fn trajectory_interface_terminal_held() {
    let p = homothetic(fixture().per_collapse);
    let t = over(&p, p.start(), 10);
    check_held(&t, t.terminal().unwrap().index);
    for k in 0..=10 {
        assert!(
            row(&t, k).r.iter().flatten().all(|x| x.is_finite()),
            "row {k} is finite"
        );
    }
    check_distinct(&(0..=5).map(|k| row(&t, k)).collect::<Vec<_>>());
}

negative_control!(
    trajectory_interface_terminal_held,
    "rows that move past a claimed terminal fail",
    expected = "is not held",
    {
        let p = circular();
        check_held(&over(&p, p.start(), 7), 3)
    }
);

// ── Purity ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Reads `a` then `b`, and on a second trajectory `b` then `a` after other reads: the same bits.
fn check_order_free<P: Provider>(p: &P, start: Ic<f64>, last: u64, a: u64, b: u64) {
    let first = over(p, start, last);
    let (fa, fb) = (row(&first, a), row(&first, b));
    let second = over(p, start, last);
    let _ = (row(&second, last), row(&second, 0));
    let (sb, sa) = (row(&second, b), row(&second, a));
    assert!(
        same_bits(&fa, &sa) && same_bits(&fb, &sb),
        "a read depends on an earlier read"
    );
}

#[test]
fn trajectory_interface_reads_order_free() {
    check_order_free(&circular(), circular().start(), 7, 2, 5);
    check_order_free(&elliptic(), elliptic().start(), 8, 8, 3);
    let h = homothetic(fixture().per_collapse);
    check_order_free(&h, h.start(), 10, 9, 4);
    let f = FigureEight::new();
    check_order_free(&f, f.start(), 4, 1, 3);
}

/// A provider whose state moves with each read: what purity forbids.
#[cfg(feature = "controls")]
struct Remembers {
    inner: Lagrange,
    reads: std::cell::Cell<u32>,
}

#[cfg(feature = "controls")]
impl Provider for Remembers {
    fn quantum(&self) -> f64 {
        self.inner.quantum()
    }
    fn terminal(&self, start: &Ic<f64>) -> Result<Option<Terminal>, Refused> {
        self.inner.terminal(start)
    }
    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        self.reads.set(self.reads.get() + 1);
        let mut s = self.inner.state(start, k)?;
        s.r[0][0] += f64::from(self.reads.get()) * 1e-9;
        Ok(s)
    }
}

negative_control!(
    trajectory_interface_reads_order_free,
    "a provider whose reads depend on earlier reads fails",
    expected = "depends on an earlier read",
    {
        let p = Remembers {
            inner: circular(),
            reads: std::cell::Cell::new(0),
        };
        check_order_free(&p, circular().start(), 7, 2, 5)
    }
);

// ── The grid and the refusals ──────────────────────────────────────────────────────────────────────────────────────

/// A provider of any start at the quantum `dt`, every state its start.
struct Grid {
    dt: f64,
}

impl Provider for Grid {
    fn quantum(&self) -> f64 {
        self.dt
    }
    fn terminal(&self, _: &Ic<f64>) -> Result<Option<Terminal>, Refused> {
        Ok(None)
    }
    fn state(&self, start: &Ic<f64>, _: u64) -> Result<Ic<f64>, Refused> {
        Ok(*start)
    }
}

/// The last boundary at the quantum `dt` and the horizon `horizon`.
fn last(dt: f64, horizon: f64) -> Result<u64, Refused> {
    Trajectory::new(&Grid { dt }, circular().start(), horizon).map(|t| t.last())
}

/// At `T = fl(k·Δ)` the last boundary is `k`, and one float below it `k − 1`, for every `k` below 200 at the quantum
/// `dt`; returns how often the division's guess was low and high.
fn check_last(dt: f64, offset: u64) -> (u32, u32) {
    let (mut low, mut high) = (0, 0);
    for k in 1..200u64 {
        let at = k as f64 * dt;
        let below = f64::from_bits(at.to_bits() - 1);
        low += u32::from(((at / dt).floor() as u64) < k);
        high += u32::from(((below / dt).floor() as u64) >= k);
        assert_eq!(
            last(dt, at),
            Ok(k + offset),
            "the last boundary at T = {k}·Δ, Δ = {dt}"
        );
        assert_eq!(
            last(dt, below),
            Ok(k - 1),
            "the last boundary below {k}·Δ, Δ = {dt}"
        );
    }
    (low, high)
}

#[test]
fn trajectory_interface_grid() {
    let (mut low, mut high) = (0, 0);
    for dt in [
        0.1,
        0.7,
        1e-3,
        0.013,
        circular().quantum(),
        FigureEight::new().quantum(),
    ] {
        let (l, h) = check_last(dt, 0);
        (low, high) = (low + l, high + h);
    }
    assert!(
        low > 0 && high > 0,
        "the division's guess is corrected both ways: {low} low, {high} high"
    );
    let s = circular().start();
    assert_eq!(last(0.5, 0.0), Ok(0));
    for bad in [-1.0, -f64::from_bits(1), f64::NAN, f64::INFINITY] {
        assert_eq!(
            last(0.5, bad).err(),
            Some(Refused::Horizon),
            "horizon {bad}"
        );
    }
    for bad in [0.0, -0.5, f64::NAN, f64::INFINITY] {
        assert_eq!(last(bad, 1.0).err(), Some(Refused::Grid), "quantum {bad}");
    }
    assert_eq!(
        last(1.0, 4_503_599_627_370_495.0),
        Ok(4_503_599_627_370_495)
    );
    assert_eq!(
        last(1.0, 4_503_599_627_370_496.0).err(),
        Some(Refused::Grid)
    );
    let big = 134_217_728.0;
    assert_eq!(
        last(big, 5.0 * big),
        Ok(5),
        "a quantum of 2²⁷ holds index 5"
    );
    assert_eq!(
        last(0.0, 0.0).err(),
        Some(Refused::Grid),
        "quantum 0 at horizon 0"
    );
    let p = circular();
    let t = over(&p, s, 7);
    assert_eq!(
        t.state(8).err(),
        Some(Refused::PastHorizon { k: 8, last: 7 })
    );
    let f = FigureEight::new();
    assert_eq!(
        Trajectory::new(&p, f.start(), 1.0).err(),
        Some(Refused::Start)
    );
    assert_eq!(Trajectory::new(&f, s, 1.0).err(), Some(Refused::Start));
    assert_eq!(p.state(&f.start(), 1).err(), Some(Refused::Start));
    assert_eq!(f.state(&s, 1).err(), Some(Refused::Start));
    assert_eq!(
        FigureEight::default().quantum().to_bits(),
        f.quantum().to_bits()
    );
}

negative_control!(
    trajectory_interface_grid,
    "a last boundary one off at an exact multiple fails",
    expected = "the last boundary at",
    {
        check_last(0.1, 1);
    }
);

// ── Kepler's equation ──────────────────────────────────────────────────────────────────────────────────────────────

/// `E − e·sin E` within four ulps of `M`, the larger of the two, for `E` the solver's.
fn check_kepler(e: f64, mean: f64, shift: f64) {
    let x = eccentric_anomaly(e, mean) + shift;
    let residual = (x - e * libm::sin(x) - mean).abs();
    let ulp = |v: f64| f64::from_bits(v.abs().to_bits() + 1) - v.abs();
    let bound = 4.0 * ulp(x.abs().max(mean.abs()));
    assert!(
        residual <= bound,
        "Kepler's equation at e = {e}, M = {mean}: residual {residual:e} over {bound:e}"
    );
}

#[test]
fn trajectory_interface_kepler_equation() {
    for (e, mean) in [
        (0.5, 1.0),
        (0.5, 6.0),
        (0.9, 0.3),
        (1.0, 1e-3),
        (0.5, 0.0),
        (1.0, 4.0),
        (1.0, 6.2),
        (0.1, 3.0),
    ] {
        check_kepler(e, mean, 0.0);
    }
    assert_eq!(
        eccentric_anomaly(0.5, 0.0).to_bits(),
        0.0f64.to_bits(),
        "M = 0: the exact root 0"
    );
}

negative_control!(
    trajectory_interface_kepler_equation,
    "an anomaly a millionth off fails",
    expected = "residual",
    check_kepler(0.5, 1.0, 1e-6)
);

// ── validation reads the lower crate's constants ──────────────────────────────────────────────────────────────────

/// `validation`'s figure-eight start is the one built from `constants`.
fn check_validation_reads(constants: &ledger::fixtures::FigureEight) {
    let ours = FigureEight::from_constants(constants).start();
    assert!(
        same_bits(&validation::drift::figure_eight().ic, &ours),
        "validation's figure_eight() is not the start built from these constants"
    );
}

#[test]
fn trajectory_interface_validation_reads_ledger_constants() {
    check_validation_reads(&ledger::fixtures::figure_eight());
    let constants = ledger::fixtures::figure_eight();
    assert_eq!(
        validation::drift::FIGURE_EIGHT_X1.map(f64::to_bits),
        constants.x1.map(f64::to_bits)
    );
    assert_eq!(
        validation::drift::FIGURE_EIGHT_V3.map(f64::to_bits),
        constants.v3.map(f64::to_bits)
    );
    let f = FigureEight::new();
    assert!(same_bits(&f.start(), &validation::drift::figure_eight().ic));
}

/// The figure-eight's quantum is `T₈` carried to the sim's units, `√3·T₈·λ^{3/2}`, `λ = I^{−1/2}` with `I` the moment
/// of inertia of the published state at the masses `1/3`, within 16u relative: `λ` to 4u (`I`'s products and sum
/// 3u, its inverse square root 1u, step 0's power-of-two step exact), `λ^{3/2}` 7u, `√3` and the products 3u; the
/// test's `I^{−3/4}`, 2u, and its products, 2u: 16u. `skew` scales the expected period, for the control.
fn check_figure_eight_period(f: &FigureEight, skew: f64) {
    let c = ledger::fixtures::figure_eight();
    let inertia = 2.0 / 3.0 * (c.x1[0] * c.x1[0] + c.x1[1] * c.x1[1]);
    let want = c.period * 3f64.sqrt() * inertia.powf(-0.75) * (1.0 + skew);
    let err = (f.quantum() - want).abs() / want;
    assert!(
        err <= 16.0 * U,
        "the figure-eight's quantum is {err:e} from √3·T₈·λ^(3/2)"
    );
}

/// `|a − b|`, as a distance.
fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The Lagrange members' starts are the configurations the closed form names, measured from the start alone, each
/// within 32u relative (each position within 3u·a of the exact: a vertex's product 1u, the centre of mass's products,
/// sum and division 2u; so a side measured within 7.5u, with its norm): the three sides equal; at masses summing to 1,
/// `I = Σmᵢ|rᵢ|² = 1` (the scale gauge); the period `2π·√(a³/(G·M))` circular, `2π·√(A³a³/(G·M))` elliptic
/// (`A = 1/(1 − e)`), and the collapse `(π/2)·√(a³/(2G·M))` homothetic, each with the side `a` measured, against the
/// provider's (its `a` 1.5u, `μ` 7.5u, the square root, `2π` and the division 6.3u; the test's side cubed 22.5u, the
/// rest 3u: 31.3u); the circular start's speeds `ω·|rᵢ|` and its momenta perpendicular to its positions (16u, as for
/// the periods, each momentum's rounding 3u). `scale` multiplies the masses, so `M` need not be 1.
fn check_starts(scale: f64, skew: f64) {
    let m = fixture().masses.map(|x| x * scale);
    let e = fixture().eccentricity;
    let g = ledger::constants::G.number();
    let total = m[0] + m[1] + m[2];
    let rel = |got: f64, want: f64, what: &str| {
        let err = (got - want).abs() / want.abs();
        assert!(err <= 32.0 * U, "{what}: {err:e} relative, over the bound");
    };
    for (member, a_power) in [
        (Member::Circular, 1.0),
        (Member::Elliptic { e }, 1.0 / (1.0 - e)),
    ] {
        let p = Lagrange::new(m, member, 8.0);
        let s = p.start();
        let a = dist(s.r[0], s.r[1]);
        rel(dist(s.r[1], s.r[2]), a, "side 1-2 against side 0-1");
        rel(dist(s.r[2], s.r[0]), a, "side 2-0 against side 0-1");
        if scale == 1.0 {
            let inertia: f64 = (0..3)
                .map(|i| m[i] * (s.r[i][0].powi(2) + s.r[i][1].powi(2)))
                .sum();
            rel(inertia, 1.0, "the start's I");
        }
        let big_a = a * a_power;
        let want = TAU * (big_a * big_a * big_a / (g * total)).sqrt() * (1.0 + skew);
        rel(
            p.period().unwrap(),
            want,
            "the period from the measured side",
        );
        if member == Member::Circular {
            let w = TAU / want;
            for ((r, q), mi) in s.r.iter().zip(&s.p).zip(m) {
                let (radius, momentum) = (dist(*r, [0.0, 0.0]), dist(*q, [0.0, 0.0]));
                rel(momentum / mi, w * radius, "a circular start's speed");
                let dot = r[0] * q[0] + r[1] * q[1];
                assert!(
                    dot.abs() <= 32.0 * U * radius * momentum,
                    "a circular start's momentum is not perpendicular"
                );
            }
        }
    }
    let h = Lagrange::new(m, Member::Homothetic, 4.0);
    let s = h.start();
    let a = dist(s.r[0], s.r[1]);
    let want = std::f64::consts::FRAC_PI_2 * (a * a * a / (2.0 * g * total)).sqrt() * (1.0 + skew);
    rel(
        h.collapse().unwrap(),
        want,
        "the collapse from the measured side",
    );
    assert!(
        s.p.iter().flatten().all(|&x| x == 0.0),
        "the homothetic start is at rest"
    );
}

#[test]
fn trajectory_interface_lagrange_starts() {
    check_starts(1.0, 0.0);
    check_starts(2.0, 0.0);
}

negative_control!(
    trajectory_interface_lagrange_starts,
    "a period a millionth off fails",
    expected = "over the bound",
    check_starts(1.0, 1e-6)
);

#[test]
fn trajectory_interface_figure_eight_period() {
    check_figure_eight_period(&FigureEight::new(), 0.0);
}

negative_control!(
    trajectory_interface_figure_eight_period,
    "a period a millionth off fails",
    expected = "from √3·T₈·λ^(3/2)",
    check_figure_eight_period(&FigureEight::new(), 1e-6)
);

negative_control!(
    trajectory_interface_validation_reads_ledger_constants,
    "constants with one digit changed fail",
    expected = "is not the start built from these constants",
    {
        let mut c = ledger::fixtures::figure_eight();
        c.x1[0] += 1e-8;
        check_validation_reads(&c)
    }
);
