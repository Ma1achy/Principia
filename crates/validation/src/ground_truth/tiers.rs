//! The four tiers of ground truth (validation_ground_truth_note, "Four tiers"), each with its cases, their closed forms
//! or published values, and the independent measurements CHECK 2 reads. CPU-f64 only.
//!
//! 1. **Conservation** ([`conservation`]): `E`, `L_z` and `P` of the march's final state against its start, each
//!    computed here, the energy by the RK45 reference's own formula (`engine::precision::rk45::energy`), never from the
//!    stored f16 latches. Necessary, not sufficient.
//! 2. **Analytic** ([`lagrange`], [`euler`]): Lagrange's equilateral and Euler's collinear relative equilibria, rigid
//!    rotations at a known `ω` ([`rotated`]). Both are linearly unstable at these masses, so they are run for a fraction
//!    of a turn, where round-off has not grown.
//! 3. **Periodic** ([`figure_eight`]): the figure-eight returns to its start after its period.
//! 4. **Literature** ([`burrau`], [`outcome`]): the Burrau problem's outcome, the lightest body escaping and the other two
//!    bound, structurally, as chaos allows (the note's "Chaos forbids pointwise comparison").
//!
//! **The reference trajectory** ([`reference`]) is the Precision ring's RK45, independent of the kernel's stepper and
//! physics source.

use kernel::decode::Ic;
use kernel::occupant::State;
use kernel::payload::{sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_ESCAPE};

use engine::precision::rk45::{self, Rk45End, Rk45Params};

use crate::import::Start;

/// A tier (the note's numbering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Conservation of `E`, `L_z`, `P`.
    Conservation = 1,
    /// Closed-form solutions.
    Analytic = 2,
    /// Known periodic orbits.
    Periodic = 3,
    /// Literature benchmarks.
    Literature = 4,
}

/// The figure-eight's start as Chenciner & Montgomery (2000) give it, Simó's digits (`ledger::fixtures`), `G = 1`, unit
/// masses, **relabelled so the middle body is in the inner pair**: bodies `(x₁, 0, −x₁)` with velocities
/// `(v₁, v₃, v₂)`. The published labelling puts body 3, the middle one, at the inner pair's centre of mass, the `α = 0`
/// pole the encode clamps; this labelling is the same state, a permutation of three equal bodies, and is interior
/// (the note's "figure-eight (non-degenerate framing)").
pub fn figure_eight() -> Start {
    let f = ledger::fixtures::figure_eight();
    let [x, y] = f.x1;
    let v3 = f.v3;
    let v1 = [-v3[0] / 2.0, -v3[1] / 2.0];
    Start {
        name: "figure_eight".into(),
        citation: "Chenciner & Montgomery 2000, Ann. Math. 152, 881; Simó's digits".into(),
        g: 1.0,
        m: [1.0; 3],
        r: [[x, y], [0.0, 0.0], [-x, -y]],
        v: [v1, v3, v1],
    }
}

/// The figure-eight's published labelling (bodies `(x₁, −x₁, 0)`), the start Path A must not take: it is on the
/// `α = 0` pole.
pub fn figure_eight_published() -> Start {
    let mut s = figure_eight();
    s.r.swap(1, 2);
    s.v.swap(1, 2);
    s
}

/// The figure-eight's period in its published units, `T₈ = 6.32591398` (`ledger::fixtures`).
pub fn figure_eight_period() -> f64 {
    ledger::fixtures::figure_eight().period
}

/// A published start's time unit in the sim's: masses `/M` and velocities `/√(GM)` multiply time by `√(GM)`, then `C`'s
/// `λ` by `λ^{3/2}` (dd_validation_orbits §2, "The recorded rescale").
pub fn time_scale(s: &Start, lambda: f64) -> f64 {
    (s.g * (s.m[0] + s.m[1] + s.m[2])).sqrt() * lambda.powf(1.5)
}

/// The Burrau problem's start in Szebehely & Peters' units (dd_validation_orbits §2, the Burrau constants): `G = 1`,
/// masses `(3, 4, 5)` at rest at `(1, 3)`, `(−2, −1)`, `(1, −1)`, each body opposite the side of its own length.
pub fn burrau() -> Start {
    Start {
        name: "burrau".into(),
        citation: "Burrau 1913; Szebehely & Peters 1967, AJ 72, 876".into(),
        g: 1.0,
        m: [3.0, 4.0, 5.0],
        r: [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
        v: [[0.0; 2]; 3],
    }
}

/// The Burrau anchor's horizon, in Szebehely & Peters' time units: the window's upper edge ([`BURRAU_WINDOW`]).
pub const BURRAU_HORIZON: f64 = 100.0;

/// The Burrau anchor's coarse window for the escape label, in Szebehely & Peters' time units (dd_validation_orbits
/// §1.5: "SP-1 runs to its own break-up, `t_max` between 60 and 65 … outcome class, escaper and a coarse `t_end`
/// window"; dd_integrator §5 test 12): the literature's disruption is near 60, so an escape labelled before 50 is too
/// early and one not labelled by 100 is too late.
pub const BURRAU_WINDOW: [f64; 2] = [50.0, 100.0];

/// The Burrau anchor's criterion on a march's `SimState` (dd_integrator §5 test 12; the integrator suite's and the
/// ground-truth suite's alike): the march ends in `escape`, its escaper the lightest body of `m`, labelled within
/// [`BURRAU_WINDOW`]. `dt_macro` is the march's macro-step and `scale` the canonical time per unit of Szebehely &
/// Peters' ([`super::burrau_time_scale`]). Panics naming what failed.
pub fn check_burrau_label(state: &SimStateBaseOf<f64>, dt_macro: f64, scale: f64, m: [f64; 3]) {
    if let Err(e) = burrau_label(state, dt_macro, scale, m) {
        panic!("{e}");
    }
}

/// [`check_burrau_label`]'s verdict: `Err` naming what failed.
pub fn burrau_label(
    state: &SimStateBaseOf<f64>,
    dt_macro: f64,
    scale: f64,
    m: [f64; 3],
) -> Result<(), String> {
    let (code, detail) = (sd_state(state.packed_a), sd_detail(state.packed_a));
    let t = f64::from(tm_t_end_step(state.times)) * dt_macro / scale;
    let lightest = lightest(m);
    if !(code == STATE_ESCAPE && detail as usize == lightest) {
        return Err(format!(
            "the Burrau march does not end in the lightest body's escape: state {code}, detail {detail}, at {t:.3} of \
             S&P's units (the lightest is body {lightest})"
        ));
    }
    if !(BURRAU_WINDOW[0]..=BURRAU_WINDOW[1]).contains(&t) {
        return Err(format!(
            "the Burrau escape fires at {t:.3} of S&P's units, outside {BURRAU_WINDOW:?}"
        ));
    }
    Ok(())
}

/// The Burrau anchor's criterion with the label live by default (escape-live-label): `stopped`, the march under the
/// stop option, meets [`check_burrau_label`] (its first label, the label's time, in the window, naming the lightest
/// body), and `live`, the same march with the label live, ends at its horizon on that same escape. Panics naming what
/// failed.
pub fn check_burrau_live(
    stopped: &SimStateBaseOf<f64>,
    live: &SimStateBaseOf<f64>,
    dt_macro: f64,
    scale: f64,
    m: [f64; 3],
) {
    if let Err(e) = burrau_live(stopped, live, dt_macro, scale, m) {
        panic!("{e}");
    }
}

/// [`check_burrau_live`]'s verdict: `Err` naming what failed.
pub fn burrau_live(
    stopped: &SimStateBaseOf<f64>,
    live: &SimStateBaseOf<f64>,
    dt_macro: f64,
    scale: f64,
    m: [f64; 3],
) -> Result<(), String> {
    burrau_label(stopped, dt_macro, scale, m)?;
    let (a, b) = (
        (sd_state(live.packed_a), sd_detail(live.packed_a)),
        (sd_state(stopped.packed_a), sd_detail(stopped.packed_a)),
    );
    if a != b {
        return Err(format!(
            "the Burrau live label at the horizon is not the first label's escape: live {a:?}, first {b:?}"
        ));
    }
    Ok(())
}

/// The index of the lightest body: Szebehely & Peters' escaper.
pub fn lightest(m: [f64; 3]) -> usize {
    (0..3).fold(0, |k, i| if m[i] < m[k] { i } else { k })
}

/// `x` turned rigidly by `angle` about the origin, positions and momenta alike.
pub fn rotated(x: &Ic<f64>, angle: f64) -> Ic<f64> {
    let (s, c) = angle.sin_cos();
    let turn = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    Ic {
        m: x.m,
        r: x.r.map(turn),
        p: x.p.map(turn),
    }
}

/// A relative equilibrium: the start, turning rigidly at `ω` about its centre of mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    /// The start, in the canonical gauge: centre of mass at the origin and at rest, `I = 1`.
    pub start: Ic<f64>,
    /// The angular velocity.
    pub omega: f64,
}

impl Rigid {
    /// The closed form at time `t`: the start turned by `ωt`.
    pub fn at(&self, t: f64) -> Ic<f64> {
        rotated(&self.start, self.omega * t)
    }

    /// The period `2π/ω`.
    pub fn period(&self) -> f64 {
        std::f64::consts::TAU / self.omega
    }
}

/// `pᵢ = mᵢ·ω·ẑ × rᵢ`: each body on its circle about the origin at `ω`.
fn circular(m: [f64; 3], r: [[f64; 2]; 3], omega: f64) -> [[f64; 2]; 3] {
    std::array::from_fn(|i| [-m[i] * omega * r[i][1], m[i] * omega * r[i][0]])
}

/// Lagrange's equilateral relative equilibrium of masses `m` (`Σm = 1`), in the canonical gauge: the triangle of side
/// `a = β^{−1/2}`, `β = m₀m₁ + m₀m₂ + m₁m₂`, so `I = β·a² = 1`, turning at `ω = √(M/a³) = a^{−3/2}` (Lagrange 1772;
/// dd_validation_orbits §1.3).
pub fn lagrange(m: [f64; 3]) -> Rigid {
    let beta = m[0] * m[1] + m[0] * m[2] + m[1] * m[2];
    let a = beta.sqrt().recip();
    let h = a * 3f64.sqrt() / 2.0;
    let raw = [[0.0, 0.0], [a, 0.0], [a / 2.0, h]];
    let com = [0, 1].map(|k| m[0] * raw[0][k] + m[1] * raw[1][k] + m[2] * raw[2][k]);
    let r = raw.map(|v| [v[0] - com[0], v[1] - com[1]]);
    let omega = a.powf(-1.5);
    Rigid {
        start: Ic {
            m,
            r,
            p: circular(m, r, omega),
        },
        omega,
    }
}

/// Euler's collinear relative equilibrium at equal masses, the middle body labelled 2 at the origin: bodies 0 and 1 at
/// `(∓a, 0)`, `a = √(3/2)` so `I = 2·⅓·a² = 1`, turning at `ω = √(5m/(4a³))`, `m = ⅓`: each outer body is pulled by
/// `m²/a²` from the middle and `m²/(2a)²` from the other (Euler 1767). Its shape point is the Euler point `E₂`,
/// `n = (1, 0, 0)`: `λ̃ = 0` there.
pub fn euler() -> Rigid {
    let third = 1.0 / 3.0;
    let a = 1.5f64.sqrt();
    let m = [third; 3];
    let r = [[-a, 0.0], [a, 0.0], [0.0, 0.0]];
    let omega = (1.25 * third / (a * a * a)).sqrt();
    Rigid {
        start: Ic {
            m,
            r,
            p: circular(m, r, omega),
        },
        omega,
    }
}

/// The Euler point `E₂`, the shape of [`euler`] at every time: `λ̃ = 0`, so `n = ((|ρ̃|² − 0)/|ρ̃|², 0, 0)`.
pub const EULER_SHAPE: [f64; 3] = [1.0, 0.0, 0.0];

/// Lagrange's homothetic solution from rest (Lagrange 1772; dd_validation_orbits §1.3): an equilateral triangle of
/// side `a₀` released at rest keeps its shape and shrinks, every body at `s(t)·rᵢ(0)` about the centre of mass, `s`
/// the radial Kepler orbit `s̈ = −k/s²`, `k = M/a₀³`, from `s(0) = 1`: `s = (1 + cos η)/2`,
/// `t = (η + sin η)/√(8k)`, to the triple collision at `t_c = π/√(8k)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Homothetic {
    /// The start, at rest, its centre of mass at the origin.
    pub start: Ic<f64>,
    /// `k = M/a₀³`.
    pub k: f64,
}

impl Homothetic {
    /// The homothetic solution of the equilateral rest start `start` (its side read from bodies 0 and 1).
    pub fn of(start: &Ic<f64>) -> Self {
        let a0 = (start.r[1][0] - start.r[0][0]).hypot(start.r[1][1] - start.r[0][1]);
        let total = start.m[0] + start.m[1] + start.m[2];
        Homothetic {
            start: *start,
            k: total / (a0 * a0 * a0),
        }
    }

    /// The collapse time `t_c = π/√(8k)`.
    pub fn collapse(&self) -> f64 {
        std::f64::consts::PI / (8.0 * self.k).sqrt()
    }

    /// The closed form at `t` in `[0, t_c)`: `η` from Kepler's radial equation by Newton's method from `η = π·t/t_c`,
    /// then `s`, `ṡ = −sin η / ((1 + cos η)·√(2k)⁻¹·2)`, positions `s·rᵢ(0)`, momenta `mᵢ·ṡ·rᵢ(0)`.
    pub fn at(&self, t: f64) -> Ic<f64> {
        let w = (8.0 * self.k).sqrt();
        let mut eta = std::f64::consts::PI * t / self.collapse();
        for _ in 0..60 {
            let f = eta + eta.sin() - w * t;
            eta -= f / (1.0 + eta.cos());
        }
        let s = (1.0 + eta.cos()) / 2.0;
        // ds/dη = −sin η/2, dt/dη = (1 + cos η)/√(8k).
        let s_dot = -eta.sin() / 2.0 * w / (1.0 + eta.cos());
        let x = &self.start;
        Ic {
            m: x.m,
            r: x.r.map(|v| [s * v[0], s * v[1]]),
            p: std::array::from_fn(|i| [x.m[i] * s_dot * x.r[i][0], x.m[i] * s_dot * x.r[i][1]]),
        }
    }
}

/// Conservation's measurements: the march's final state against its start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drift {
    /// `|E − E₀| / |E₀|`.
    pub energy: f64,
    /// `|L_z − L_{z,0}| / max(|L_{z,0}|, Σ‖rᵢ‖‖pᵢ‖)`: relative to the angular momentum's own scale, since `L_z` is 0
    /// on the figure-eight.
    pub lz: f64,
    /// `‖Σpᵢ − Σpᵢ⁰‖ / max‖pᵢ⁰‖`.
    pub momentum: f64,
}

fn state(x: &Ic<f64>) -> State<f64> {
    State {
        m: x.m,
        r: x.r,
        p: x.p,
    }
}

fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// Tier 1: the drift of `E`, `L_z` and `P` from `start` to `end` (the module's note).
pub fn conservation(start: &Ic<f64>, end: &Ic<f64>) -> Drift {
    let e0 = rk45::energy(&state(start));
    let scale: f64 = (0..3)
        .map(|i| start.r[i][0].hypot(start.r[i][1]) * start.p[i][0].hypot(start.p[i][1]))
        .sum();
    let total = |x: &Ic<f64>| [0, 1].map(|k| x.p[0][k] + x.p[1][k] + x.p[2][k]);
    let (p0, p1) = (total(start), total(end));
    let pmax = start
        .p
        .iter()
        .map(|v| v[0].hypot(v[1]))
        .fold(0.0f64, f64::max);
    Drift {
        energy: ((rk45::energy(&state(end)) - e0) / e0).abs(),
        lz: (lz(end) - lz(start)).abs() / lz(start).abs().max(scale),
        momentum: (p1[0] - p0[0]).hypot(p1[1] - p0[1]) / pmax,
    }
}

/// The RK45 reference's tolerances for the suite: `rtol = 10⁻¹³`, `atol = 10⁻¹⁵`, the Precision ring test's, with a
/// budget of `5·10⁷` steps.
pub const REFERENCE: Rk45Params = Rk45Params {
    rtol: 1e-13,
    atol: 1e-15,
    max_steps: 50_000_000,
};

/// The reference trajectory from `start` at each of `times` (non-decreasing): the states reached, or why it stopped.
pub fn reference(
    start: &Ic<f64>,
    times: &[f64],
    params: &Rk45Params,
) -> Result<Vec<Ic<f64>>, Rk45End> {
    let run = rk45::integrate(&state(start), times, params);
    if run.end != Rk45End::Reached {
        return Err(run.end);
    }
    Ok(run
        .states
        .iter()
        .map(|s| Ic {
            m: s.m,
            r: s.r,
            p: s.p,
        })
        .collect())
}

/// A three-body state read structurally (the note's "Chaos forbids pointwise comparison"): which pair is bound and which
/// body has left it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    /// The escaper: the body whose two-body energy about the other pair's centre of mass is positive and whose distance
    /// from it exceeds ten times the pair's separation; `None` where no body is.
    pub escaper: Option<usize>,
    /// The closest pair's separation.
    pub pair_separation: f64,
    /// The escaper's distance from the pair's centre of mass (or the farthest body's, where none escapes).
    pub distance: f64,
    /// The escaper's two-body energy `½μ|Δv|² − μ·M/d` about the pair, `μ` the reduced mass, with `G = 1`.
    pub e_rel: f64,
}

/// [`Outcome`] of the state `x`: the body farthest from the closest pair's centre of mass, read independently of the
/// kernel's escape detector.
pub fn outcome(x: &Ic<f64>) -> Outcome {
    let sep = |i: usize, j: usize| (x.r[i][0] - x.r[j][0]).hypot(x.r[i][1] - x.r[j][1]);
    let pairs = [(1, 2, 0), (0, 2, 1), (0, 1, 2)];
    let (i, j, k) = pairs
        .into_iter()
        .min_by(|a, b| sep(a.0, a.1).total_cmp(&sep(b.0, b.1)))
        .expect("three pairs");
    let mp = x.m[i] + x.m[j];
    let com = [0, 1].map(|c| (x.m[i] * x.r[i][c] + x.m[j] * x.r[j][c]) / mp);
    let vcom = [0, 1].map(|c| (x.p[i][c] + x.p[j][c]) / mp);
    let d = [x.r[k][0] - com[0], x.r[k][1] - com[1]];
    let dv = [x.p[k][0] / x.m[k] - vcom[0], x.p[k][1] / x.m[k] - vcom[1]];
    let mu = x.m[k] * mp / (x.m[k] + mp);
    let dist = d[0].hypot(d[1]);
    let e_rel = 0.5 * mu * (dv[0] * dv[0] + dv[1] * dv[1]) - x.m[k] * mp / dist;
    let pair_separation = sep(i, j);
    Outcome {
        escaper: (e_rel > 0.0 && dist > 10.0 * pair_separation).then_some(k),
        pair_separation,
        distance: dist,
        e_rel,
    }
}

/// Body `k`'s two-body energy about the centre of mass of the other two, `½μ|Δv|² − m_k·M_pair/d`, `μ` the reduced
/// mass, `G = 1`: positive where `k` is unbound from them.
pub fn e_rel(x: &Ic<f64>, k: usize) -> f64 {
    let (i, j) = ((k + 1) % 3, (k + 2) % 3);
    let mp = x.m[i] + x.m[j];
    let com = [0, 1].map(|c| (x.m[i] * x.r[i][c] + x.m[j] * x.r[j][c]) / mp);
    let vcom = [0, 1].map(|c| (x.p[i][c] + x.p[j][c]) / mp);
    let d = [x.r[k][0] - com[0], x.r[k][1] - com[1]];
    let dv = [x.p[k][0] / x.m[k] - vcom[0], x.p[k][1] / x.m[k] - vcom[1]];
    let mu = x.m[k] * mp / (x.m[k] + mp);
    0.5 * mu * (dv[0] * dv[0] + dv[1] * dv[1]) - x.m[k] * mp / d[0].hypot(d[1])
}

/// The Burrau ejection's sampling times, in Szebehely & Peters' units: every 0.5 from 50 to 70, then every 5 to 100.
pub fn ejection_samples() -> Vec<f64> {
    (0..=40)
        .map(|k| 50.0 + 0.5 * f64::from(k))
        .chain((1..=6).map(|k| 70.0 + 5.0 * f64::from(k)))
        .collect()
}

/// The ejection time of body `k` over the samples `(t, state)`, in time order: the earliest sample time from which
/// `k`'s [`e_rel`] is positive at that sample and at every later one; `None` where it is not positive at the last.
pub fn ejection_time(samples: &[(f64, Ic<f64>)], k: usize) -> Option<f64> {
    let mut first = None;
    for (t, x) in samples.iter().rev() {
        if e_rel(x, k) > 0.0 {
            first = Some(*t);
        } else {
            break;
        }
    }
    first
}

/// The Burrau ejection's window, in Szebehely & Peters' units: the march and the reference both leave the lightest body
/// unbound from a time in it on (the physics the late escape label hides; measured on PR #29).
pub const BURRAU_EJECTION_WINDOW: [f64; 2] = [60.0, 65.0];
