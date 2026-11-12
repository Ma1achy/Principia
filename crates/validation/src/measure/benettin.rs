//! The Benettin FTLE shadow measured (TASK-M3-14; `kernel::driver::benettin`): dd_integrator §5 test 10 at candidate
//! `(‖δ₀‖, n_renorm)`, f32 and f64, behind REQ-INT-076's proposal, and its eight-orbit report (reported, never gated);
//! and the absolute and displacement f32 shadows' FTLE error against the known exponent on periodic orbits (payload §8;
//! REQ-VAL-038).

use kernel::burrau::euclid;
use kernel::canon::canonicalise;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::Advance;
use kernel::driver::benettin::{
    integrate_ftle_with, Absolute, BenettinParams, Displacement, ShadowForm,
};
use kernel::driver::wrapper::WrapperReal;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::State;
use kernel::payload::tm_t_end_step;

use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::drift::{self, ic_at, sim_key, AsR};
use crate::gate::{
    Gate, GateConfig, GateReport, MeasurementConfig, MeasurementControl, MeasurementGate,
    RegionCount, Regions, Verdict,
};

/// An orbit measured: its name, its canonical IC at f64, its horizon `T`, and its known Lyapunov exponent where it has
/// one.
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub name: &'static str,
    pub ic: Ic<f64>,
    pub horizon: f64,
    pub known: Option<f64>,
}

/// `ic` in the canonical frame, the masses kept.
fn canonical(ic: &Ic<f64>) -> Ic<f64> {
    canonicalise(ic).ic
}

/// The Kepler-embedded orbit, the quasi-regular one of dd_integrator §5 test 10: test 1's near-Kepler pair, `m₂ → ε`
/// (bodies 0 and 1 of masses `(1 − ε)/2` on a Kepler orbit of semi-major axis 1 and eccentricity 1/2, started at
/// apocentre), with body 2, of mass `ε = 10⁻⁹`, on a circular orbit of radius 20 about the pair's centre of mass
/// ([`drift::hierarchy`]), so that every body's motion is regular; canonicalised, to the default horizon.
pub fn kepler_embedded() -> Orbit {
    let eps = 1e-9;
    let m = [(1.0 - eps) / 2.0, (1.0 - eps) / 2.0, eps];
    Orbit {
        name: "Kepler-embedded",
        ic: drift::hierarchy(m, 1.0, 0.5, 20.0).0,
        horizon: drift::T_DEFAULT,
        known: None,
    }
}

/// The Burrau IC (dd_integrator §5's golden anchor): the (3, 4, 5) triangle at rest, in the normalised convention,
/// canonicalised, to the default horizon.
pub fn burrau() -> Orbit {
    pythagorean("Burrau", 2, 1)
}

/// The rest-start triangle of the Pythagorean family that Euclid's pair `(m, n)` gives (`kernel::burrau::euclid`; the
/// pair (2, 1) gives Burrau's (3, 4, 5)), in the normalised convention, canonicalised, to the default horizon, named
/// `name`.
///
/// # Panics
///
/// Where `(m, n)` is not a Euclid pair (coprime, `m > n`, of opposite parity).
pub fn pythagorean(name: &'static str, m: u32, n: u32) -> Orbit {
    let s = euclid(m, n).expect("a Euclid pair").burrau::<f64>();
    let ic = Ic {
        m: s.m,
        r: s.r,
        p: [[0.0; 2]; 3],
    };
    Orbit {
        name,
        ic: canonical(&ic),
        horizon: drift::T_DEFAULT,
        known: None,
    }
}

/// `β = (m₀m₁ + m₀m₂ + m₁m₂)/M²`, the mass parameter of Routh's criterion.
pub fn routh_beta(m: [f64; 3]) -> f64 {
    let total = m[0] + m[1] + m[2];
    (m[0] * m[1] + m[0] * m[2] + m[1] * m[2]) / (total * total)
}

/// The largest Lyapunov exponent of Lagrange's circular equilateral orbit of masses `m` and angular velocity `ω`, in
/// units of `ω`: the largest real part of the roots of `s⁴ + s² + (27/4)β = 0`, the linearisation's characteristic
/// equation in the rotating frame (Routh 1875), `½·√(√(27β) − 1)` where `27β > 1`, and 0 where `27β ≤ 1`, the stable
/// case, whose roots are imaginary.
pub fn lagrange_exponent(m: [f64; 3]) -> f64 {
    // `√(27β) − 1 ≤ 0` exactly where the roots are imaginary, so the floor at 0 is the stable case.
    0.5 * ((27.0 * routh_beta(m)).sqrt() - 1.0).max(0.0).sqrt()
}

/// Lagrange's circular equilateral orbit of masses `m` (summing to 1, so that `ω = 1` at side 1), canonicalised, to
/// the horizon `horizon`, its known exponent [`lagrange_exponent`] times its canonical `ω = L_z / I = |L_z|` (`I = 1`,
/// rigid rotation).
pub fn lagrange(name: &'static str, m: [f64; 3], horizon: f64) -> Orbit {
    // The unit triangle turning rigidly at `ω = √(M/a³) = 1` about the origin; canonicalising takes it to its centre of
    // mass and removes the bulk momentum, which leaves the rotation about the centre of mass.
    let r = [[0.0, 0.0], [1.0, 0.0], [0.5, 3f64.sqrt() / 2.0]];
    let p = [0, 1, 2].map(|i| [-m[i] * r[i][1], m[i] * r[i][0]]);
    let ic = canonical(&Ic { m, r, p });
    let lz: f64 = (0..3)
        .map(|i| ic.r[i][0] * ic.p[i][1] - ic.r[i][1] * ic.p[i][0])
        .sum();
    Orbit {
        name,
        ic,
        horizon,
        known: Some(lagrange_exponent(m) * lz.abs()),
    }
}

/// The figure-eight (dd_integrator §5 test 2's), linearly stable, its exponent 0, to the horizon `horizon`.
pub fn figure_eight(horizon: f64) -> Orbit {
    Orbit {
        name: "figure-eight",
        ic: drift::figure_eight().ic,
        horizon,
        known: Some(0.0),
    }
}

/// One FTLE measurement: `λ_T`, `S`, the completed macro-steps and the shadow's final separation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ftle {
    pub lambda: f64,
    pub s_sum: f64,
    pub steps: u32,
    pub delta: f64,
}

/// `λ_T` of `orbit` by the occupant `A` at `R`, the shadow in the form `F`, at `‖δ₀‖ = delta_0` and `n_renorm`, through
/// `kernel::driver::benettin::integrate_ftle_with` at the orbit's sim key ([`sim_key`]) with no collision: the FTLE
/// finalised at the run's end, in the form's own `δ`.
pub fn measure<A: Advance, R: WrapperReal + Into<f64>, F: ShadowForm>(
    orbit: &Orbit,
    delta_0: f64,
    n_renorm: u32,
) -> Ftle
where
    f64: AsR<R>,
{
    let ic = ic_at::<R>(&orbit.ic);
    let key = sim_key::<R>(orbit.horizon);
    let params = BenettinParams {
        delta_0: delta_0.to_r(),
        n_renorm,
    };
    let (out, shadow) = integrate_ftle_with::<A, R, F>(&ic, &key, CollisionRadius::none(), params);
    let x = State {
        m: ic.m,
        r: out.r,
        p: out.p,
    };
    let steps = tm_t_end_step(out.times);
    Ftle {
        lambda: shadow.ftle(&x, steps, key.dt_macro).into(),
        s_sum: shadow.s_sum().into(),
        steps,
        delta: shadow.separation(&x).into(),
    }
}

/// The precisions dd test 10 is run at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    F32,
    F64,
}

impl Precision {
    /// "f32" or "f64".
    pub fn word(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

/// `λ_T` of `orbit` by Yoshida-4 at `precision`, the shadow in the payload's form ([`Absolute`]).
pub fn lambda(orbit: &Orbit, precision: Precision, delta_0: f64, n_renorm: u32) -> f64 {
    match precision {
        Precision::F32 => measure::<Yoshida4, f32, Absolute>(orbit, delta_0, n_renorm).lambda,
        Precision::F64 => measure::<Yoshida4, f64, Absolute>(orbit, delta_0, n_renorm).lambda,
    }
}

/// dd_integrator §5 test 10 on one orbit at one precision and candidate `(‖δ₀‖, n_renorm)`: `λ_T` there, and with
/// `‖δ₀‖` halved, `n_renorm` doubled, and both.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Test10 {
    pub delta_0: f64,
    pub n_renorm: u32,
    pub horizon: f64,
    /// `λ_T` at the candidate, at `‖δ₀‖/2`, at `2·n_renorm`, and at both.
    pub lambdas: [f64; 4],
}

impl Test10 {
    /// Runs it.
    pub fn run(orbit: &Orbit, precision: Precision, delta_0: f64, n_renorm: u32) -> Self {
        let at = |d: f64, n: u32| lambda(orbit, precision, d, n);
        Test10 {
            delta_0,
            n_renorm,
            horizon: orbit.horizon,
            lambdas: [
                at(delta_0, n_renorm),
                at(delta_0 / 2.0, n_renorm),
                at(delta_0, 2 * n_renorm),
                at(delta_0 / 2.0, 2 * n_renorm),
            ],
        }
    }

    /// The change: the largest `|λ_T' − λ_T|` over the three variants, relative to `max(|λ_T|, 1/T)`, `1/T` the
    /// resolution of an exponent read over `T` (one neper of growth); infinite where any `λ_T` is not finite.
    pub fn change(&self) -> f64 {
        let [l, rest @ ..] = self.lambdas;
        if !self.lambdas.iter().all(|x| x.is_finite()) {
            return f64::INFINITY;
        }
        let scale = l.abs().max(1.0 / self.horizon);
        rest.iter().map(|x| (x - l).abs()).fold(0.0, f64::max) / scale
    }
}

// ── REQ-INT-076: the calibration of ‖δ₀‖ and n_renorm ──────────────────────────────────────────────────────────

/// The candidate `‖δ₀‖`s: the grid first written was `10⁻³ … 10⁻⁸` by decades; its best lay on its edge, so it was
/// extended to `10⁻²` and by half-decades, before the rule below was applied.
pub const GRID_DELTA_0: [f64; 7] = [1e-2, 3e-3, 1e-3, 3e-4, 1e-4, 1e-5, 1e-6];

/// The candidate `n_renorm`s, in macro-steps: the grid first written was 1, 10, 100, extended as [`GRID_DELTA_0`].
pub const GRID_N_RENORM: [u32; 6] = [1, 10, 30, 100, 300, 1000];

/// The proposed `‖δ₀‖` (REQ-INT-076, R-71; proposed, to be confirmed at the M3 gate): [`select`]'s pick on the grid.
pub const PROPOSED_DELTA_0: f64 = 3e-3;

/// The proposed `n_renorm` (REQ-INT-076, R-71; proposed, to be confirmed at the M3 gate): [`select`]'s pick.
pub const PROPOSED_N_RENORM: u32 = 300;

/// The proposed tolerance of dd test 10's change ([`Test10::change`]; REQ-INT-076, R-71; proposed, to be confirmed at
/// the M3 gate): [`tolerance`] of the selected candidate's worst change, the gate's provisional threshold.
pub const PROPOSED_TOLERANCE: f64 = 0.2;

/// The horizon of the Lagrange orbits: short enough that the f32 march, started a rounding away from the unstable
/// orbit, is still on it at the end (`e^{λT}` times f32's `6·10⁻⁸` stays below `10⁻²` at the equal-mass exponent).
pub const LAGRANGE_HORIZON: f64 = 30.0;

/// The orbits dd test 10 is calibrated on: the Kepler-embedded orbit and the Burrau IC, test 10's own, and Lagrange's
/// equal-mass orbit, whose exponent is known.
pub fn calibration_orbits() -> [Orbit; 3] {
    [
        kepler_embedded(),
        burrau(),
        lagrange("Lagrange, equal masses", [1.0 / 3.0; 3], LAGRANGE_HORIZON),
    ]
}

/// The calibration orbit named `name` (`kepler`, `burrau` or `lagrange`).
pub fn calibration_orbit(name: &str) -> Option<Orbit> {
    let [kepler, burrau, lagrange] = calibration_orbits();
    match name {
        "kepler" => Some(kepler),
        "burrau" => Some(burrau),
        "lagrange" => Some(lagrange),
        _ => None,
    }
}

/// One candidate `(‖δ₀‖, n_renorm)`: dd test 10 on each calibration orbit at f32 and f64.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub delta_0: f64,
    pub n_renorm: u32,
    /// `(orbit, precision, test 10)`.
    pub runs: Vec<(&'static str, Precision, Test10)>,
}

impl Candidate {
    /// Runs dd test 10 at `(delta_0, n_renorm)` on every calibration orbit at f32 and f64.
    pub fn run(delta_0: f64, n_renorm: u32) -> Self {
        Self::run_on(&calibration_orbits(), delta_0, n_renorm)
    }

    /// Runs dd test 10 at `(delta_0, n_renorm)` on each of `orbits` at f32 and f64, each run under its orbit's name.
    pub fn run_on(orbits: &[Orbit], delta_0: f64, n_renorm: u32) -> Self {
        let mut runs = Vec::new();
        for orbit in orbits {
            for p in [Precision::F32, Precision::F64] {
                runs.push((orbit.name, p, Test10::run(orbit, p, delta_0, n_renorm)));
            }
        }
        Candidate {
            delta_0,
            n_renorm,
            runs,
        }
    }

    /// The worst change over its runs ([`Test10::change`]).
    pub fn worst(&self) -> f64 {
        self.runs.iter().map(|r| r.2.change()).fold(0.0, f64::max)
    }

    /// `λ_T` at the candidate itself on the orbit named `orbit` at `precision`.
    pub fn lambda(&self, orbit: &str, precision: Precision) -> Option<f64> {
        self.runs
            .iter()
            .find(|r| r.0 == orbit && r.1 == precision)
            .map(|r| r.2.lambdas[0])
    }

    /// The candidate with the runs of the orbit named `orbit` left out.
    pub fn without(&self, orbit: &str) -> Candidate {
        Candidate {
            runs: self.runs.iter().filter(|r| r.0 != orbit).cloned().collect(),
            ..self.clone()
        }
    }
}

/// Every candidate of the grid on the calibration orbits, each on a thread of its own.
pub fn sweep() -> Vec<Candidate> {
    sweep_on(&calibration_orbits())
}

/// Every candidate of the grid on `orbits`, each on a thread of its own.
pub fn sweep_on(orbits: &[Orbit]) -> Vec<Candidate> {
    let pairs: Vec<(f64, u32)> = GRID_DELTA_0
        .iter()
        .flat_map(|&d| GRID_N_RENORM.iter().map(move |&n| (d, n)))
        .collect();
    std::thread::scope(|s| {
        let handles: Vec<_> = pairs
            .iter()
            .map(|&(d, n)| s.spawn(move || Candidate::run_on(orbits, d, n)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a sweep thread completes"))
            .collect()
    })
}

/// `x` to two significant digits, the precision two candidates' worst changes are told apart at.
fn two_digits(x: f64) -> f64 {
    format!("{x:.1e}").parse().unwrap_or(x)
}

/// The rule, written before the sweep was run: the candidate whose worst change is smallest, two candidates whose
/// worst changes agree to two significant digits going to the larger `n_renorm` (fewer renormalisations), then to the
/// larger `‖δ₀‖` (further from the precision's rounding). `None` for no candidates.
pub fn select(candidates: &[Candidate]) -> Option<&Candidate> {
    candidates.iter().min_by(|a, b| {
        two_digits(a.worst())
            .total_cmp(&two_digits(b.worst()))
            .then(b.n_renorm.cmp(&a.n_renorm))
            .then(b.delta_0.total_cmp(&a.delta_0))
    })
}

/// The tolerance the rule proposes from the selected candidate's worst change `worst`: the smallest value of one
/// significant digit at or above `2·worst`, a margin of two for the scatter a second orbit or machine would show.
pub fn tolerance(worst: f64) -> f64 {
    // In decimal, from the shortest representation of `2·worst`, so that 0.08 stays 0.08 rather than reading 8.000…2
    // hundredths and rising to 0.09.
    let text = format!("{:e}", 2.0 * worst);
    let (mantissa, exponent) = text.split_once('e').unwrap_or((text.as_str(), "0"));
    let (mantissa, exponent): (f64, i32) = (
        mantissa.parse().unwrap_or(f64::NAN),
        exponent.parse().unwrap_or(0),
    );
    let digit = mantissa.ceil();
    format!("{digit}e{exponent}").parse().unwrap_or(f64::NAN)
}

/// Whether the sweep confirms the gate whose threshold is `tol`: the rule picked the proposal (`chosen`), the
/// tolerance the rule gives from its worst change is `tol`, and `tol` is the proposed tolerance.
pub fn sweep_confirms(chosen: &Candidate, tol: f64) -> bool {
    chosen.delta_0 == PROPOSED_DELTA_0
        && chosen.n_renorm == PROPOSED_N_RENORM
        && tolerance(chosen.worst()) == tol
        && tol == PROPOSED_TOLERANCE
}

// ── REQ-INT-076: the eight-orbit report (PR #493, physics F3 (c) and F5; reported, never gated, R-649) ─────────────

/// The chaotic orbits of the eight-orbit report, its regions: Burrau and five more rest-start triangles of the
/// Pythagorean family ([`pythagorean`]), each a free fall through close encounters, named by their sides `(a, b, c)`.
pub fn chaotic_orbits() -> [Orbit; 6] {
    [
        burrau(),
        pythagorean("(5, 12, 13) at rest", 3, 2),
        pythagorean("(15, 8, 17) at rest", 4, 1),
        pythagorean("(7, 24, 25) at rest", 4, 3),
        pythagorean("(21, 20, 29) at rest", 5, 2),
        pythagorean("(9, 40, 41) at rest", 5, 4),
    ]
}

/// The report's eight starting conditions: the [`calibration_orbits`], then the [`chaotic_orbits`] after Burrau.
pub fn eight_orbits() -> Vec<Orbit> {
    let mut orbits = calibration_orbits().to_vec();
    orbits.extend(chaotic_orbits().into_iter().skip(1));
    orbits
}

/// The `‖δ₀‖` of the converged reference the bias is read against: the grid's smallest, where f64's `λ_T` no longer
/// moves with `n_renorm` (the linear limit).
pub const REFERENCE_DELTA_0: f64 = 1e-6;

/// The bias of `lambda` against the reference `reference` over `horizon`, on dd test 10's scale:
/// `(λ_T − λ_ref) / max(|λ_ref|, 1/T)`.
pub fn bias(lambda: f64, reference: f64, horizon: f64) -> f64 {
    (lambda - reference) / reference.abs().max(1.0 / horizon)
}

/// The reference `λ_T` of the orbit named `orbit` at `n_renorm`: f64 at [`REFERENCE_DELTA_0`], read from `candidates`.
pub fn reference(candidates: &[Candidate], orbit: &str, n_renorm: u32) -> Option<f64> {
    candidates
        .iter()
        .find(|c| c.delta_0 == REFERENCE_DELTA_0 && c.n_renorm == n_renorm)?
        .lambda(orbit, Precision::F64)
}

/// The rule's pick ([`select`]) with the orbit named `orbit` left out of every candidate, `(‖δ₀‖, n_renorm)`.
pub fn select_without(candidates: &[Candidate], orbit: &str) -> Option<(f64, u32)> {
    let fewer: Vec<Candidate> = candidates.iter().map(|c| c.without(orbit)).collect();
    select(&fewer).map(|c| (c.delta_0, c.n_renorm))
}

/// The least, the median (the upper of the two middle values for an even count) and the largest of `values`, a value
/// that is not finite sorting as infinite; NaN for none.
pub fn spread(values: &[f64]) -> [f64; 3] {
    let mut v: Vec<f64> = values
        .iter()
        .map(|x| if x.is_finite() { *x } else { f64::INFINITY })
        .collect();
    v.sort_by(f64::total_cmp);
    match (v.first(), v.get(v.len() / 2), v.last()) {
        (Some(&lo), Some(&mid), Some(&hi)) => [lo, mid, hi],
        _ => [f64::NAN; 3],
    }
}

/// Over the orbits named in `chaotic`, candidate `c`'s runs at `precision`: the [`spread`] of the change, then of the
/// bias against the reference ([`reference`], [`bias`]), NaN where there is no reference.
pub fn chaotic_spread(
    candidates: &[Candidate],
    c: &Candidate,
    precision: Precision,
    chaotic: &[&str],
) -> [[f64; 3]; 2] {
    let runs: Vec<&(&str, Precision, Test10)> = c
        .runs
        .iter()
        .filter(|r| r.1 == precision && chaotic.contains(&r.0))
        .collect();
    let changes: Vec<f64> = runs.iter().map(|r| r.2.change()).collect();
    let biases: Vec<f64> = runs
        .iter()
        .map(|r| {
            reference(candidates, r.0, c.n_renorm)
                .map_or(f64::NAN, |x| bias(r.2.lambdas[0], x, r.2.horizon))
        })
        .collect();
    [spread(&changes), spread(&biases)]
}

/// One line per candidate: its worst change and, over the orbits named in `chaotic` at f32 and at f64, the least,
/// median and largest change and bias ([`chaotic_spread`]).
pub fn spread_lines(candidates: &[Candidate], chaotic: &[&str]) -> Vec<String> {
    let fmt = |s: [f64; 3]| format!("{:+.3} / {:+.3} / {:+.3}", s[0], s[1], s[2]);
    candidates
        .iter()
        .map(|c| {
            let [c32, b32] = chaotic_spread(candidates, c, Precision::F32, chaotic);
            let [c64, b64] = chaotic_spread(candidates, c, Precision::F64, chaotic);
            format!(
                "‖δ₀‖ = {:e}, n_renorm = {}: worst change {:.4}; over the chaotic orbits (least / median / largest) \
                 f32 change {}, bias {}; f64 change {}, bias {}",
                c.delta_0,
                c.n_renorm,
                c.worst(),
                fmt(c32),
                fmt(b32),
                fmt(c64),
                fmt(b64)
            )
        })
        .collect()
}

/// One line per orbit named in `chaotic`: the rule's pick with that orbit left out ([`select_without`]), and whether it
/// is `chosen`, the full set's pick.
pub fn leave_one_out_lines(
    candidates: &[Candidate],
    chosen: &Candidate,
    chaotic: &[&str],
) -> Vec<String> {
    chaotic
        .iter()
        .map(|key| match select_without(candidates, key) {
            Some((d, n)) => format!(
                "without {key}: the rule picks ‖δ₀‖ = {d:e}, n_renorm = {n}, {} the full set's pick",
                if (d, n) == (chosen.delta_0, chosen.n_renorm) {
                    "which is"
                } else {
                    "which is not"
                }
            ),
            None => format!("without {key}: no candidate"),
        })
        .collect()
}

/// One line per candidate: its worst change, and each run's change, `λ_T` and bias against the reference.
pub fn run_lines(candidates: &[Candidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|c| {
            let runs: Vec<String> = c
                .runs
                .iter()
                .map(|(o, p, t)| {
                    let b = reference(candidates, o, c.n_renorm)
                        .map_or(f64::NAN, |r| bias(t.lambdas[0], r, t.horizon));
                    format!(
                        "{o} {}: {:.3} (λ_T {:.4}, bias {b:+.3})",
                        p.word(),
                        t.change(),
                        t.lambdas[0]
                    )
                })
                .collect();
            format!(
                "‖δ₀‖ = {:e}, n_renorm = {}: worst change {:.4}; {}",
                c.delta_0,
                c.n_renorm,
                c.worst(),
                runs.join("; ")
            )
        })
        .collect()
}

/// Whether `candidates` hold the whole report on `orbits`: at least one candidate, and each with exactly one run of
/// every orbit at f32 and at f64 and no other run.
pub fn report_complete(candidates: &[Candidate], orbits: &[Orbit]) -> bool {
    !candidates.is_empty()
        && candidates.iter().all(|c| {
            c.runs.len() == 2 * orbits.len()
                && orbits.iter().all(|o| {
                    // Two runs, one at each precision. Written so (R-197): a count of exactly one per precision, over
                    // two precisions, passed the same with its precision test inverted, an equivalent mutant.
                    let mine: Vec<Precision> = c
                        .runs
                        .iter()
                        .filter(|r| r.0 == o.name)
                        .map(|r| r.1)
                        .collect();
                    mine.len() == 2
                        && mine.contains(&Precision::F32)
                        && mine.contains(&Precision::F64)
                })
        })
}

/// The eight-orbit report's candidates: `sweep` run on `orbits`, with the runs of the orbit named `drop` left out where
/// one is named (a control). Refused, before `sweep` runs, where `drop` names none of `orbits`.
pub fn eight_orbit_candidates(
    orbits: &[Orbit],
    drop: Option<&str>,
    sweep: impl FnOnce(&[Orbit]) -> Vec<Candidate>,
) -> Result<Vec<Candidate>, String> {
    match drop {
        None => Ok(sweep(orbits)),
        Some(gone) if orbits.iter().any(|o| o.name == gone) => {
            Ok(sweep(orbits).iter().map(|c| c.without(gone)).collect())
        }
        Some(gone) => Err(format!("no orbit `{gone}` to drop")),
    }
}

/// The eight-orbit report on `candidates` over `orbits`, its regions the orbits named in `chaotic`, against `config`:
/// reported, never gated (R-649), its verdict the report's completeness alone ([`report_complete`]). Its description
/// gives the rule's pick over every orbit and the tolerance the rule gives, beside the proposal, which it leaves as it
/// is; its negative results, the non-finite count and the pick with each chaotic orbit left out; its scatter, each
/// candidate's spread over the chaotic orbits and each run. Refused for no candidates.
pub fn eight_orbit_report(
    config: &GateConfig,
    name: &str,
    description: &str,
    candidates: &[Candidate],
    orbits: &[Orbit],
    chaotic: &[&str],
) -> Result<GateReport, String> {
    let chosen = select(candidates).ok_or("the sweep ran no candidate")?;
    let unfinite = candidates
        .iter()
        .filter(|c| c.worst().is_infinite())
        .count();
    let mut negative_results = vec![format!(
        "{unfinite} of {} candidates gave a non-finite λ_T somewhere (the f32 shadow's δ lost to rounding)",
        candidates.len()
    )];
    negative_results.extend(leave_one_out_lines(candidates, chosen, chaotic));
    let mut scatter = spread_lines(candidates, chaotic);
    scatter.extend(run_lines(candidates));
    Ok(report(
        config,
        name,
        format!(
            "{description}: reported, not gated (R-649; the verdict is the report's completeness alone). Over the \
             {} starting conditions the rule picks ‖δ₀‖ = {:e}, n_renorm = {} (worst change {:.4}), tolerance {}; \
             the proposal stays ‖δ₀‖ = {PROPOSED_DELTA_0:e}, n_renorm = {PROPOSED_N_RENORM}, tolerance \
             {PROPOSED_TOLERANCE}, for the owner at the M3 gate (REQ-INT-076). Bias is against f64 at ‖δ₀‖ = \
             {REFERENCE_DELTA_0:e} at the same n_renorm",
            orbits.len(),
            chosen.delta_0,
            chosen.n_renorm,
            chosen.worst(),
            tolerance(chosen.worst())
        ),
        report_complete(candidates, orbits),
        scatter,
        negative_results,
        chaotic.len() as u32,
    ))
}

// ── REQ-VAL-038: the absolute and displacement f32 shadows ────────────────────────────────────────────────────────

/// The periodic-orbit set, each orbit's exponent known: Lagrange's equal-mass orbit (`ω/√2`), its `(0.5, 0.3, 0.2)`
/// orbit (`0.688·ω`), both unstable (`27β > 1`), its `(0.98, 0.01, 0.01)` orbit, linearly stable (`27β < 1`, exponent
/// 0), and the figure-eight, linearly stable (exponent 0); to [`LAGRANGE_HORIZON`].
pub fn periodic_set() -> [Orbit; 4] {
    [
        lagrange("Lagrange, equal masses", [1.0 / 3.0; 3], LAGRANGE_HORIZON),
        lagrange(
            "Lagrange, (0.5, 0.3, 0.2)",
            [0.5, 0.3, 0.2],
            LAGRANGE_HORIZON,
        ),
        lagrange(
            "Lagrange, (0.98, 0.01, 0.01)",
            [0.98, 0.01, 0.01],
            LAGRANGE_HORIZON,
        ),
        figure_eight(LAGRANGE_HORIZON),
    ]
}

/// One periodic orbit measured: its known exponent, `λ_T` at f64 in the absolute form (the finite-time value the f32
/// forms are read against), and at f32 in the absolute and displacement forms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormRow {
    pub orbit: &'static str,
    pub known: f64,
    pub f64_absolute: f64,
    pub f32_absolute: f64,
    pub f32_displacement: f64,
}

impl FormRow {
    /// Measures `orbit` at `(delta_0, n_renorm)`.
    pub fn run(orbit: &Orbit, delta_0: f64, n_renorm: u32) -> Self {
        FormRow {
            orbit: orbit.name,
            known: orbit.known.unwrap_or(f64::NAN),
            f64_absolute: measure::<Yoshida4, f64, Absolute>(orbit, delta_0, n_renorm).lambda,
            f32_absolute: measure::<Yoshida4, f32, Absolute>(orbit, delta_0, n_renorm).lambda,
            f32_displacement: measure::<Yoshida4, f32, Displacement>(orbit, delta_0, n_renorm)
                .lambda,
        }
    }

    /// Whether every value is finite.
    pub fn finite(&self) -> bool {
        [
            self.known,
            self.f64_absolute,
            self.f32_absolute,
            self.f32_displacement,
        ]
        .iter()
        .all(|x| x.is_finite())
    }

    /// The row in the measurement's table.
    pub fn line(&self) -> String {
        format!(
            "| {} | {:.5} | {:.5} | {:.5} ({:+.5}, {:+.5}) | {:.5} ({:+.5}, {:+.5}) |",
            self.orbit,
            self.known,
            self.f64_absolute,
            self.f32_absolute,
            self.f32_absolute - self.known,
            self.f32_absolute - self.f64_absolute,
            self.f32_displacement,
            self.f32_displacement - self.known,
            self.f32_displacement - self.f64_absolute,
        )
    }
}

/// The `‖δ₀‖`s the forms are measured at: the proposed value, and `10⁻⁵`, where the f32 state's rounding is a larger
/// part of the separation, the regime a displacement form would help in.
pub const FORM_DELTA_0: [f64; 2] = [PROPOSED_DELTA_0, 1e-5];

/// The table's header.
pub const FORM_HEADER: &str =
    "| orbit | known λ | f64 absolute λ_T | f32 absolute λ_T (− known, − f64) | \
                               f32 displacement λ_T (− known, − f64) |\n|---|---|---|---|---|";

/// The periodic set measured at `(delta_0, n_renorm)`.
pub fn shadow_forms(delta_0: f64, n_renorm: u32) -> Vec<FormRow> {
    periodic_set()
        .iter()
        .map(|o| FormRow::run(o, delta_0, n_renorm))
        .collect()
}

/// The two horizons the finite-time gap to Routh's exponent is read at: [`LAGRANGE_HORIZON`] and twice it, both short
/// enough at f64 for the march to stay on the unstable orbit. The march leaves the orbit by `e^{λT}` times each step's
/// error, rounding and the stepper's truncation alike: at `T = 60` on the equal-mass orbit `e^{λT} ≈ 1.2·10⁸`, so f64's
/// unit roundoff alone gives about `1.3·10⁻⁸`, and the truncation error adds to it, amplified by the same factor. That
/// the gap times `T` is the same at both horizons shows the march still on the orbit.
pub const GAP_HORIZONS: [f64; 2] = [LAGRANGE_HORIZON, 2.0 * LAGRANGE_HORIZON];

/// One unstable Lagrange orbit's f64 absolute `λ_T` at the two [`GAP_HORIZONS`], against its known exponent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapRow {
    pub orbit: &'static str,
    pub known: f64,
    pub lambdas: [f64; 2],
}

impl GapRow {
    /// Measures `orbit` at `(delta_0, n_renorm)` at each horizon.
    pub fn run(orbit: &Orbit, delta_0: f64, n_renorm: u32) -> Self {
        GapRow {
            orbit: orbit.name,
            known: orbit.known.unwrap_or(f64::NAN),
            lambdas: GAP_HORIZONS.map(|horizon| {
                let o = Orbit { horizon, ..*orbit };
                measure::<Yoshida4, f64, Absolute>(&o, delta_0, n_renorm).lambda
            }),
        }
    }

    /// `known − λ_T` at each horizon.
    pub fn gaps(&self) -> [f64; 2] {
        self.lambdas.map(|l| self.known - l)
    }

    /// The row: `λ_T`, the gap and the gap times `T` at each horizon, and the ratio of the two gaps, ½ for a gap that
    /// falls as `1/T`.
    pub fn line(&self) -> String {
        let [g0, g1] = self.gaps();
        let [t0, t1] = GAP_HORIZONS;
        format!(
            "| {} | {:.5} | {:.5} ({:+.5}, ×T {:+.3}) | {:.5} ({:+.5}, ×T {:+.3}) | {:.3} |",
            self.orbit,
            self.known,
            self.lambdas[0],
            g0,
            g0 * t0,
            self.lambdas[1],
            g1,
            g1 * t1,
            g1 / g0
        )
    }
}

/// The finite-time table's header.
pub const GAP_HEADER: &str =
    "| orbit | known λ | f64 λ_T at T = 30 (gap, gap × T) | f64 λ_T at T = 60 (gap, gap × T) \
                              | gap ratio |\n|---|---|---|---|---|";

/// The unstable orbits of the periodic set (`27β > 1`) at `(delta_0, n_renorm)` and the two horizons.
pub fn finite_time_gaps(delta_0: f64, n_renorm: u32) -> Vec<GapRow> {
    periodic_set()
        .iter()
        .filter(|o| o.known.is_some_and(|k| k > 0.0))
        .map(|o| GapRow::run(o, delta_0, n_renorm))
        .collect()
}

// ── The gates ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `cargo xtask gate benettin-calibration` (REQ-INT-076): dd test 10 at the proposed `(‖δ₀‖, n_renorm)` on each
/// calibration orbit at f32 and f64, against the proposed tolerance, the gate's provisional threshold; the sweep,
/// which passes only where the rule picks the proposed pair and tolerance; and the eight-orbit report, the grid on the
/// [`eight_orbits`] with each candidate's spread over the [`chaotic_orbits`] and the pick with each left out, which
/// asserts nothing of its results (R-649) and changes neither the proposal nor the other inputs.
pub struct BenettinCalibrationGate;

/// A `benettin-calibration` input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "lowercase")]
enum CalibrationInput {
    /// The sweep and the rule.
    Sweep { description: String },
    /// The eight-orbit report, reported and never gated (R-649): its verdict is the report's completeness alone, and
    /// `drop`, a control, leaves the named orbit's runs out of it.
    #[serde(rename = "eight-orbits")]
    EightOrbits {
        description: String,
        #[serde(default)]
        drop: Option<String>,
    },
    /// dd test 10 on one orbit at one precision, at the proposed values unless named.
    Test10 {
        description: String,
        orbit: String,
        precision: String,
        #[serde(default)]
        delta_0: Option<f64>,
        #[serde(default)]
        n_renorm: Option<u32>,
    },
}

fn precision(word: &str) -> Result<Precision, String> {
    match word {
        "f32" => Ok(Precision::F32),
        "f64" => Ok(Precision::F64),
        _ => Err(format!("precision `{word}` is neither f32 nor f64")),
    }
}

fn report(
    config: &GateConfig,
    name: &str,
    description: String,
    pass: bool,
    scatter: Vec<String>,
    negative_results: Vec<String>,
    regions: u32,
) -> GateReport {
    GateReport {
        gate: config.gate.clone(),
        input: name.to_owned(),
        description,
        verdict: if pass { Verdict::Pass } else { Verdict::Fail },
        threshold: config.threshold.clone(),
        scatter,
        regions: Some(Regions {
            count: RegionCount::Recorded(regions),
            minimum: config.min_regions.clone(),
        }),
        negative_results,
    }
}

impl Gate for BenettinCalibrationGate {
    fn name(&self) -> &'static str {
        "benettin-calibration"
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = CalibrationInput::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let tol = config.threshold.value;
        match input {
            CalibrationInput::Test10 {
                description,
                orbit,
                precision: p,
                delta_0,
                n_renorm,
            } => {
                let o = calibration_orbit(&orbit)
                    .ok_or_else(|| format!("{name}: no calibration orbit `{orbit}`"))?;
                let p = precision(&p)?;
                let d = delta_0.unwrap_or(PROPOSED_DELTA_0);
                let n = n_renorm.unwrap_or(PROPOSED_N_RENORM);
                let t = Test10::run(&o, p, d, n);
                let change = t.change();
                let [l, h, dbl, both] = t.lambdas;
                Ok(report(
                    config,
                    name,
                    format!("{description}: {} at {}, ‖δ₀‖ = {d:e}, n_renorm = {n}, T = {}", o.name, p.word(), o.horizon),
                    change <= tol,
                    vec![
                        format!("λ_T = {l:.6} at (‖δ₀‖, n_renorm)"),
                        format!("λ_T = {h:.6} at ‖δ₀‖/2"),
                        format!("λ_T = {dbl:.6} at 2·n_renorm"),
                        format!("λ_T = {both:.6} at both"),
                    ],
                    vec![format!(
                        "the largest change |λ_T' − λ_T| / max(|λ_T|, 1/T) is {change:.4}, against the tolerance {tol}"
                    )],
                    1,
                ))
            }
            CalibrationInput::Sweep { description } => {
                let candidates = sweep();
                let chosen = select(&candidates).ok_or("the sweep ran no candidate")?;
                let proposed = tolerance(chosen.worst());
                let pass = sweep_confirms(chosen, tol);
                let scatter = candidates
                    .iter()
                    .map(|c| {
                        let runs: Vec<String> = c
                            .runs
                            .iter()
                            .map(|(o, p, t)| {
                                format!(
                                    "{o} {}: {:.3} (λ_T {:.4})",
                                    p.word(),
                                    t.change(),
                                    t.lambdas[0]
                                )
                            })
                            .collect();
                        format!(
                            "‖δ₀‖ = {:e}, n_renorm = {}: worst change {:.4}; {}",
                            c.delta_0,
                            c.n_renorm,
                            c.worst(),
                            runs.join("; ")
                        )
                    })
                    .collect();
                let unfinite = candidates
                    .iter()
                    .filter(|c| c.worst().is_infinite())
                    .count();
                Ok(report(
                    config,
                    name,
                    format!(
                        "{description}: the rule picks ‖δ₀‖ = {:e}, n_renorm = {} (worst change {:.4}), tolerance {proposed}; \
                         proposed ‖δ₀‖ = {PROPOSED_DELTA_0:e}, n_renorm = {PROPOSED_N_RENORM}, tolerance {tol}",
                        chosen.delta_0,
                        chosen.n_renorm,
                        chosen.worst()
                    ),
                    pass,
                    scatter,
                    vec![format!(
                        "{unfinite} of {} candidates gave a non-finite λ_T somewhere (the f32 shadow's δ lost to rounding)",
                        candidates.len()
                    )],
                    1,
                ))
            }
            CalibrationInput::EightOrbits { description, drop } => {
                // The grid runs once per process, the report and its control sharing it.
                static SWEEP: OnceLock<Vec<Candidate>> = OnceLock::new();
                let orbits = eight_orbits();
                let candidates = eight_orbit_candidates(&orbits, drop.as_deref(), |o| {
                    SWEEP.get_or_init(|| sweep_on(o)).clone()
                })
                .map_err(|e| format!("{name}: {e}"))?;
                eight_orbit_report(
                    config,
                    name,
                    &description,
                    &candidates,
                    &orbits,
                    &chaotic_orbits().map(|o| o.name),
                )
            }
        }
    }
}

/// `cargo xtask gate shadow-form` (REQ-VAL-038; R-649): a measurement. Its controls check the comparison, and its
/// `periodic-set` input measures both f32 shadow forms against the known exponents at the proposed values; the switch
/// to the displacement form waits on REQ-PAY-085's criterion, so nothing here decides it.
pub struct ShadowFormGate;

/// A `shadow-form` control.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
enum FormControl {
    /// One form read against itself, or against itself at `scale·‖δ₀‖`: passes where the two reads are identical.
    Compare { description: String, scale: f64 },
    /// The periodic set measured: passes where every value is finite.
    PeriodicSet { description: String },
}

impl MeasurementGate for ShadowFormGate {
    fn name(&self) -> &'static str {
        "shadow-form"
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        match FormControl::deserialize(fixture).map_err(|e| format!("{name}: {e}"))? {
            FormControl::Compare { description, scale } => {
                let orbit = &periodic_set()[0];
                let read =
                    |d: f64| measure::<Yoshida4, f32, Absolute>(orbit, d, PROPOSED_N_RENORM).lambda;
                let (a, b) = (read(PROPOSED_DELTA_0), read(scale * PROPOSED_DELTA_0));
                let same = a.to_bits() == b.to_bits();
                Ok(MeasurementControl {
                    description: format!("{description}: {a:.6} against {b:.6}"),
                    verdict: if same { Verdict::Pass } else { Verdict::Fail },
                    markdown: format!(
                        "# shadow-form: {name}\n\n{description}\n\nf32 absolute λ_T on {}: {a} and {b}\n",
                        orbit.name
                    ),
                })
            }
            FormControl::PeriodicSet { description } => {
                let mut markdown = format!(
                    "# shadow-form: {name}\n\n{description}; T = {LAGRANGE_HORIZON}, Yoshida-4, n_renorm = \
                     {PROPOSED_N_RENORM}\n"
                );
                let mut every = Vec::new();
                for d in FORM_DELTA_0 {
                    let rows = shadow_forms(d, PROPOSED_N_RENORM);
                    let table: Vec<String> = rows.iter().map(FormRow::line).collect();
                    markdown += &format!("\n‖δ₀‖ = {d:e}\n\n{FORM_HEADER}\n{}\n", table.join("\n"));
                    every.extend(rows);
                }
                markdown += "\nThe displacement column holds δ⃗ but forms x + δ⃗ at each macro-step and marches that \
                             absolute state, so it agrees with the absolute column by construction (PR #493, physics \
                             F2): the table does not settle REQ-VAL-038.\n";
                markdown += "\nThe gap to Routh's exponent at two horizons, f64, the absolute form (a measurement, \
                             R-649: reported, never tuned). A gap that is finite-time falls as 1/T, a ratio near ½ \
                             and gap × T near constant.\n";
                for d in FORM_DELTA_0 {
                    let table: Vec<String> = finite_time_gaps(d, PROPOSED_N_RENORM)
                        .iter()
                        .map(GapRow::line)
                        .collect();
                    markdown += &format!("\n‖δ₀‖ = {d:e}\n\n{GAP_HEADER}\n{}\n", table.join("\n"));
                }
                let finite = every.iter().all(FormRow::finite);
                print!("{markdown}");
                Ok(MeasurementControl {
                    description,
                    verdict: if finite { Verdict::Pass } else { Verdict::Fail },
                    markdown,
                })
            }
        }
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        Err(format!(
            "gate shadow-form: takes no flags (`{}`); its measurement is its `periodic-set` input",
            args.join(" ")
        ))
    }
}
