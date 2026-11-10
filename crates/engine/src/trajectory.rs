//! The trajectory interface (chart_composition § 18.7; R-918, R-919 F18; REQ-CHART-173; TASK-M2-82): M2's maths that
//! reads a trajectory reads it through a [`Provider`], never through the integrator, so that it is tested against
//! trajectories known exactly, and the real integrator plugs in after M2 (TASK-M3-67) with no change to that maths. It
//! is a CPU, engine-side interface: the horizon is an argument of each [`Trajectory`], never a uniform.
//!
//! **What a provider gives** for a signed start state `(m, r, p)` in the scale gauge and a horizon `T`:
//! - **the boundary grid**: integer boundary indices `k = 0, 1, …` at the quantum `Δ` the provider declares, boundary
//!   `k` at `t = k·Δ`, the last the greatest `k` with `k·Δ ≤ T`, found by multiplying indices, never by trusting a float
//!   division ([`Trajectory::last`]); a provider stands for one occupant, so the grid is per occupant;
//! - **the state at a boundary**, in the start's gauge, the mirror never folded;
//! - **the terminal**: none, or the boundary index at which the trajectory latches and its kind; every later boundary
//!   reads the terminal state ([`Trajectory::state`] holds it);
//! - **purity**: the same bits for the same `(start, T, k)`, in any order of reads and after any other read.
//!
//! **The fixtures** (R-919 F18), each a provider of exactly one start ([`FigureEight::start`], [`Lagrange::start`]),
//! refusing any other bit pattern:
//! - [`Lagrange`], the equilateral homographic solution (dd_validation_orbits § 1.3) in closed form at any masses:
//!   bodies `rᵢ(t) = z(t)·cᵢ`, `pᵢ(t) = mᵢ·ż(t)·cᵢ` in complex arithmetic, `cᵢ` the start's positions and `z` the
//!   Kepler orbit `z̈ = −μ·z/|z|³`, `μ = G·M/a³`, from `z(0) = 1`. Its [`Member::Circular`] turns rigidly at
//!   `ω = √μ`; its [`Member::Elliptic`] runs from pericentre through Kepler's equation; its [`Member::Homothetic`]
//!   collapses from rest through the radial Kepler equation to a triple collision at `t_c = π/√(8μ)`;
//! - [`FigureEight`], exactly periodic by declaration: its quantum is its period `T₈`, and its state at every boundary
//!   is its start (§ 18.7, item 2).

use kernel::canon::canonicalise;
use kernel::decode::Ic;
use ledger::display::{Detail, State};

/// Why a trajectory or a read is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The start is not the provider's: a fixture gives one start, bit for bit.
    Start,
    /// The horizon is not finite and non-negative.
    Horizon,
    /// The quantum is not finite and positive, or the grid's indices pass the integers an `f64` holds exactly.
    Grid,
    /// The boundary `k` lies past the horizon's last boundary `last`.
    PastHorizon { k: u64, last: u64 },
}

/// A terminal: the boundary index at which the trajectory latches, and its kind, a payload `state` and `detail` (the
/// outcome classes; `ledger::display`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terminal {
    pub index: u64,
    pub state: State,
    pub detail: Detail,
}

/// A trajectory's source: the real integrator after M2, an exact fixture now. Every method is pure in its arguments.
pub trait Provider {
    /// The quantum `Δ`: boundary `k` is at `t = k·Δ`.
    fn quantum(&self) -> f64;
    /// The terminal from `start`, horizon apart, or `None`.
    fn terminal(&self, start: &Ic<f64>) -> Result<Option<Terminal>, Refused>;
    /// The state at boundary `k` from `start`; at or past a terminal's index, the terminal state.
    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused>;
}

/// The greatest `k` with `k·Δ ≤ t`, both finite, `Δ > 0`, `t ≥ 0`: the division only guesses, the products decide.
/// Below `2⁵²` the rounded quotient's floor is within one of that `k` (the division and the product `k·Δ` each round
/// once, by at most half a unit of the quotient's last place), so the search starts two below it and only climbs,
/// `fl(k·Δ)` being monotone in `k`. Refused past `2⁵²`, where `k·Δ` no longer tells neighbouring indices apart.
fn last_at_or_before(dt: f64, t: f64) -> Result<u64, Refused> {
    let guess = (t / dt).floor();
    if guess >= (1u64 << 52) as f64 {
        return Err(Refused::Grid);
    }
    let mut k = (guess as u64).saturating_sub(2);
    while (k + 1) as f64 * dt <= t {
        k += 1;
    }
    Ok(k)
}

/// The least `k` with `k·Δ ≥ t`, both finite, `Δ > 0`, `t ≥ 0`.
fn first_at_or_after(dt: f64, t: f64) -> Result<u64, Refused> {
    let k = last_at_or_before(dt, t)?;
    Ok(if k as f64 * dt < t { k + 1 } else { k })
}

/// A provider read from one start to one horizon (§ 18.7's table).
#[derive(Clone, Copy, Debug)]
pub struct Trajectory<'p, P: Provider + ?Sized> {
    provider: &'p P,
    start: Ic<f64>,
    last: u64,
    terminal: Option<Terminal>,
}

impl<'p, P: Provider + ?Sized> Trajectory<'p, P> {
    /// `provider` from `start` to the horizon `horizon`; refused where the horizon or the grid is not one, or the
    /// provider refuses the start.
    pub fn new(provider: &'p P, start: Ic<f64>, horizon: f64) -> Result<Self, Refused> {
        if !(horizon.is_finite() && horizon >= 0.0) {
            return Err(Refused::Horizon);
        }
        let dt = provider.quantum();
        if !(dt.is_finite() && dt > 0.0) {
            return Err(Refused::Grid);
        }
        let last = last_at_or_before(dt, horizon)?;
        let terminal = provider.terminal(&start)?.filter(|t| t.index <= last);
        Ok(Self {
            provider,
            start,
            last,
            terminal,
        })
    }

    /// The quantum `Δ` the provider declares.
    pub fn quantum(&self) -> f64 {
        self.provider.quantum()
    }

    /// The last boundary index, the greatest `k` with `k·Δ ≤ T`.
    pub fn last(&self) -> u64 {
        self.last
    }

    /// Boundary `k`'s time, `k·Δ`, for ticks: rows are kept as indices (§ 18.2).
    pub fn time(&self, k: u64) -> f64 {
        k as f64 * self.quantum()
    }

    /// The terminal within the horizon, or `None`.
    pub fn terminal(&self) -> Option<Terminal> {
        self.terminal
    }

    /// The state at boundary `k`: the provider's, held at the terminal's from its index on; refused past the horizon.
    pub fn state(&self, k: u64) -> Result<Ic<f64>, Refused> {
        if k > self.last {
            return Err(Refused::PastHorizon { k, last: self.last });
        }
        let k = self.terminal.map_or(k, |t| k.min(t.index));
        self.provider.state(&self.start, k)
    }
}

/// Whether `a` and `b` are the same bits.
pub fn same_bits(a: &Ic<f64>, b: &Ic<f64>) -> bool {
    let bits = |s: &Ic<f64>| {
        let mut v = s.m.map(f64::to_bits).to_vec();
        v.extend(s.r.iter().chain(&s.p).flat_map(|x| x.map(f64::to_bits)));
        v
    };
    bits(a) == bits(b)
}

/// `Ok` where `start` is `own`, bit for bit.
fn own(start: &Ic<f64>, own: &Ic<f64>) -> Result<(), Refused> {
    if same_bits(start, own) {
        Ok(())
    } else {
        Err(Refused::Start)
    }
}

/// The figure-eight (§ 18.7, item 2): its start built from `ledger::fixtures` as `validation`'s `figure_eight()` builds
/// it (dd_integrator § 5, test 2: the masses `1/3`, the velocities scaled by `√(1/3)`, then canonicalised), and its
/// quantum the period `T₈` carried to those units: `√3·T₈` for the masses, then `λ^{3/2}` for the canonical scale
/// `λ`, at `G = M = 1`.
#[derive(Clone, Copy, Debug)]
pub struct FigureEight {
    start: Ic<f64>,
    period: f64,
}

impl FigureEight {
    /// The fixture from `ledger::fixtures::figure_eight()`.
    pub fn new() -> Self {
        Self::from_constants(&ledger::fixtures::figure_eight())
    }

    /// The fixture from `constants`.
    pub fn from_constants(constants: &ledger::fixtures::FigureEight) -> Self {
        let third: f64 = 1.0 / 3.0;
        let s = third.sqrt();
        let [x, y] = constants.x1;
        let [vx, vy] = constants.v3;
        let v = [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]];
        let ic = Ic {
            m: [third; 3],
            r: [[x, y], [-x, -y], [0.0, 0.0]],
            p: v.map(|[a, b]| [third * s * a, third * s * b]),
        };
        let c = canonicalise(&ic);
        Self {
            start: c.ic,
            period: constants.period * 3f64.sqrt() * c.lambda.powf(1.5),
        }
    }

    /// Its one start.
    pub fn start(&self) -> Ic<f64> {
        self.start
    }
}

impl Default for FigureEight {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for FigureEight {
    fn quantum(&self) -> f64 {
        self.period
    }

    fn terminal(&self, start: &Ic<f64>) -> Result<Option<Terminal>, Refused> {
        own(start, &self.start).map(|()| None)
    }

    fn state(&self, start: &Ic<f64>, _k: u64) -> Result<Ic<f64>, Refused> {
        own(start, &self.start).map(|()| self.start)
    }
}

/// A member of the Lagrange family (dd_validation_orbits § 1.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Member {
    /// The circular velocity: a rigid rotation at `ω = √μ`, period `2π/ω`.
    Circular,
    /// From pericentre at `|z| = 1`, eccentricity `e ∈ (0, 1)`: `ż(0) = i·√(μ(1 + e))`, the Kepler period `2π/n`.
    Elliptic { e: f64 },
    /// From rest: the radial collapse, a triple collision at `t_c = π/√(8μ)`.
    Homothetic,
}

/// The Lagrange equilateral homographic solution (§ 18.7, item 1): the masses `m`, the side `a` that puts the start at
/// `I = 1` (`I = a²·Σ_{i<j} mᵢmⱼ/M`), the vertices `(0, 0)`, `(a, 0)`, `(a/2, a√3/2)` less their centre of mass, and
/// the quantum `Δ` the member's period, or its collapse time, over `divisions`.
#[derive(Clone, Copy, Debug)]
pub struct Lagrange {
    member: Member,
    mu: f64,
    quantum: f64,
    start: Ic<f64>,
}

/// `a·b` in complex arithmetic, on pairs.
fn mul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]]
}

/// The bodies at `z` and `ż` from the positions `c` and masses `m`: `rᵢ = z·cᵢ`, `pᵢ = mᵢ·(ż·cᵢ)`.
fn bodies(m: [f64; 3], c: [[f64; 2]; 3], z: [f64; 2], zdot: [f64; 2]) -> Ic<f64> {
    Ic {
        m,
        r: c.map(|ci| mul(z, ci)),
        p: [0, 1, 2].map(|i| mul(zdot, c[i]).map(|x| m[i] * x)),
    }
}

/// The eccentric anomaly `E` with `E − e·sin E = M`, `e ∈ (0, 1]`, by bisection of the bracket `[M − e, M + e]`,
/// which holds the root (`f(E) = E − e·sin E − M` is monotone, `f(M − e) ≤ 0 ≤ f(M + e)`), to adjacent floats: the
/// upper end, `f ≥ 0`, within one ulp of where the float `f` changes sign, or a float at which `f` is exactly 0. Pure
/// in its arguments and the same bits everywhere (the soft-float `libm`).
pub fn eccentric_anomaly(e: f64, mean: f64) -> f64 {
    let (mut lo, mut hi) = (mean - e, mean + e);
    loop {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            return hi;
        }
        let f = mid - e * libm::sin(mid) - mean;
        if f == 0.0 {
            return mid;
        }
        if f < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
}

impl Lagrange {
    /// The member `member` at the masses `m` (`Σmᵢ = 1`), at the quantum its period, or its collapse time, over
    /// `divisions`.
    pub fn new(m: [f64; 3], member: Member, divisions: f64) -> Self {
        let total = m[0] + m[1] + m[2];
        let pairs = m[0] * m[1] + m[1] * m[2] + m[2] * m[0];
        let a = (total / pairs).sqrt();
        let g = ledger::constants::G.number();
        let mu = g * total / (a * a * a);
        let vertices = [[0.0, 0.0], [a, 0.0], [0.5 * a, 0.5 * a * 3f64.sqrt()]];
        let com = [0, 1].map(|d| (0..3).map(|i| m[i] * vertices[i][d]).sum::<f64>() / total);
        let c = vertices.map(|v| [v[0] - com[0], v[1] - com[1]]);
        let zdot = match member {
            Member::Circular => [0.0, mu.sqrt()],
            Member::Elliptic { e } => [0.0, (mu * (1.0 + e)).sqrt()],
            Member::Homothetic => [0.0, 0.0],
        };
        let mut fixture = Self {
            member,
            mu,
            quantum: 0.0,
            start: bodies(m, c, [1.0, 0.0], zdot),
        };
        let span = fixture.period().or(fixture.collapse()).unwrap_or(0.0);
        fixture.quantum = span / divisions;
        fixture
    }

    /// Its one start.
    pub fn start(&self) -> Ic<f64> {
        self.start
    }

    /// The period: `2π/ω` circular, the Kepler period `2π/n` elliptic, none homothetic.
    pub fn period(&self) -> Option<f64> {
        match self.member {
            Member::Circular => Some(core::f64::consts::TAU / self.mu.sqrt()),
            Member::Elliptic { e } => Some(core::f64::consts::TAU / self.mean_motion(e)),
            Member::Homothetic => None,
        }
    }

    /// The collapse time `t_c = π/n`, `n = √(8μ)` (the radial orbit's semi-major axis `1/2`), homothetic only.
    pub fn collapse(&self) -> Option<f64> {
        match self.member {
            Member::Homothetic => Some(core::f64::consts::PI / self.mean_motion(1.0)),
            _ => None,
        }
    }

    /// `n = √(μ/A³)`, `A = 1/(1 − e)` the orbit's semi-major axis from pericentre `1`, or `1/2` from apocentre `1` at
    /// `e = 1`.
    fn mean_motion(&self, e: f64) -> f64 {
        let big_a = Self::semi_major(e);
        (self.mu / (big_a * big_a * big_a)).sqrt()
    }

    fn semi_major(e: f64) -> f64 {
        if e < 1.0 {
            1.0 / (1.0 - e)
        } else {
            0.5
        }
    }

    /// `z` and `ż` at `t`, before any terminal.
    fn orbit(&self, t: f64) -> ([f64; 2], [f64; 2]) {
        match self.member {
            Member::Circular => {
                let w = self.mu.sqrt();
                let (s, c) = (libm::sin(w * t), libm::cos(w * t));
                ([c, s], [-w * s, w * c])
            }
            Member::Elliptic { e } => {
                let (big_a, n) = (Self::semi_major(e), self.mean_motion(e));
                let b = big_a * (1.0 - e * e).sqrt();
                let x = eccentric_anomaly(e, n * t);
                let (s, c) = (libm::sin(x), libm::cos(x));
                let d = 1.0 - e * c;
                (
                    [big_a * (c - e), b * s],
                    [-big_a * n * s / d, b * n * c / d],
                )
            }
            Member::Homothetic => {
                let n = self.mean_motion(1.0);
                let x = eccentric_anomaly(1.0, core::f64::consts::PI + n * t);
                let (s, c) = (libm::sin(x), libm::cos(x));
                ([0.5 * (1.0 - c), 0.0], [0.5 * n * s / (1.0 - c), 0.0])
            }
        }
    }

    /// The boundary at which the collapse latches: the first at or after `t_c`.
    fn collision_index(&self) -> Result<Option<u64>, Refused> {
        self.collapse()
            .map(|t| first_at_or_after(self.quantum, t))
            .transpose()
    }
}

impl Provider for Lagrange {
    fn quantum(&self) -> f64 {
        self.quantum
    }

    fn terminal(&self, start: &Ic<f64>) -> Result<Option<Terminal>, Refused> {
        own(start, &self.start)?;
        Ok(self.collision_index()?.map(|index| Terminal {
            index,
            state: State::Collision,
            detail: Detail::D3,
        }))
    }

    /// Boundary 0 is the start, by definition. At and past the collision's boundary the terminal state is the state at
    /// the last boundary before it (RQ-757: the closed form's own state at `t_c` has every body at the centre of mass
    /// and its momenta unbounded; applied meanwhile).
    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        own(start, &self.start)?;
        let k = match self.collision_index()? {
            Some(c) if k >= c => c.saturating_sub(1),
            _ => k,
        };
        if k == 0 {
            return Ok(self.start);
        }
        let (z, zdot) = self.orbit(k as f64 * self.quantum);
        let c = self.start.r;
        Ok(bodies(self.start.m, c, z, zdot))
    }
}
