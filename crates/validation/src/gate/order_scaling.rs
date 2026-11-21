//! `order-scaling` (dd_integrator §5 test 1; REQ-VAL-030, its tolerance REQ-VAL-139's): on the Kepler-embedded fixture
//! the maximum relative energy error over one orbit, fitted against `dt` on a log–log ladder, has the slope of the
//! occupant's order: 1 for Euler, 2 for KDK, 4 for Yoshida-4 and RK4, 6 for Yoshida-6.
//!
//! **The fixture** ([`kepler_embedded`]): bodies 0 and 1, masses 0.6 and 0.4, a Kepler pair of semi-major axis 1 and
//! eccentricity 0.6 started at apocentre, and body 2 of mass `10⁻¹²` (`m₂ → ε`) on a circular orbit at distance 40,
//! built by [`crate::drift::hierarchy`] and canonicalised. Body 2's pull on the pair is of order `10⁻¹²`, below every
//! rung's error, so the pair is a Kepler orbit to the gate's resolution and its period is known in closed form. The
//! eccentricity makes the pericentre the step's hardest place, so the error is the step's at a resolved encounter and
//! not a circular orbit's, where symmetric steps cancel to higher order.
//!
//! **The quantity** (test 1a's definition): `max_t |E(t) − E₀| / |E₀|` over `n` bare `STEP`s of `dt = P/n`, `P` the
//! pair's period, at f64, with no wrapper: the order is the stepper's, and the wrapper's substep law and projection
//! would change the step being measured. The end-of-orbit error is not used: on a closed orbit a symmetric step's
//! oscillating error comes back near zero there, and the fit would read the cancellation.
//!
//! **The range is chosen before the fit** ([`REGIME`]): each input names its ladder, and every rung must be in the
//! asymptotic regime by two bounds written here, not read from the fit: the step at most a tenth of the pericentre's
//! two-body time `τ_p = √(r_p³ / (m₀ + m₁))` (the coarse end), and the error between [`ERROR_FLOOR`], well above f64's
//! accumulated rounding over a few thousand steps, and [`ERROR_CEILING`], where the error is still a perturbation (the
//! fine and coarse ends of the error). A ladder with any rung outside is refused whole: dropping rungs after seeing
//! their errors would be choosing the range after the fit.
//!
//! **The verdict:** the least-squares slope of `ln err` against `ln dt` within the threshold of the order (REQ-VAL-139,
//! provisional). The order is the occupant's profile's, or the input's `claimed_order` where it names one: the control,
//! a mislabelled order, which must fail. The report gives the slope's spread with it: its standard error from the fit's
//! residuals, and every adjacent pair of rungs' local slope, whose drift along the ladder would say the ladder is not
//! asymptotic.
//!
//! The plan is unclear here; chose `fixtures/gates/order-scaling/` for the fixture directory the task calls
//! `kepler-embedded/`, because the gate runner reads a gate's inputs from the directory of its name; the fixture's
//! state is built here, since it is two lines of Kepler's law, and its inputs there name the ladders.

use serde::Deserialize;

use kernel::occupant::{DebugOccupant, Geom, Occupant, Selection, State};
use kernel::physics::energy::energy;

use crate::drift::hierarchy;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The gate's name.
pub const NAME: &str = "order-scaling";

/// The pair's masses and the far body's, `m₂ → ε` (the module's note).
pub const MASSES: [f64; 3] = [0.6, 0.4, 1e-12];

/// The pair's semi-major axis.
pub const SEMI_MAJOR: f64 = 1.0;

/// The pair's eccentricity.
pub const ECCENTRICITY: f64 = 0.6;

/// The far body's distance from the pair's centre of mass.
pub const FAR: f64 = 40.0;

/// The asymptotic regime's bounds, written before any fit (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Regime {
    /// The largest `dt / τ_p`.
    pub max_step_ratio: f64,
    /// The smallest error a rung may have.
    pub floor: f64,
    /// The largest error a rung may have.
    pub ceiling: f64,
}

/// The rounding floor: `10⁻¹²`, about `10⁴` f64 epsilons, so a few thousand steps' accumulated rounding (`ε√n` to
/// `εn`, at most `10⁻¹²` at `n = 10⁴`) stays a small part of every rung's error.
pub const ERROR_FLOOR: f64 = 1e-12;

/// The perturbative ceiling: `10⁻²`, past which the error's next order is no longer small beside its leading one.
pub const ERROR_CEILING: f64 = 1e-2;

/// The coarse end: a step at most a tenth of the pericentre's two-body time.
pub const MAX_STEP_RATIO: f64 = 0.1;

/// The bounds every input's ladder is held to.
pub const REGIME: Regime = Regime {
    max_step_ratio: MAX_STEP_RATIO,
    floor: ERROR_FLOOR,
    ceiling: ERROR_CEILING,
};

/// The Kepler-embedded fixture: its state, the pair's period and its pericentre's two-body time, in canonical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kepler {
    /// The start, at the pair's apocentre.
    pub state: State<f64>,
    /// The pair's period `P = 2π√(a³/(m₀ + m₁))`.
    pub period: f64,
    /// The pericentre's two-body time `τ_p = √(r_p³/(m₀ + m₁))`.
    pub tau_peri: f64,
}

/// The Kepler-embedded fixture (the module's note).
pub fn kepler_embedded() -> Kepler {
    let (ic, peri) = hierarchy(MASSES, SEMI_MAJOR, ECCENTRICITY, FAR);
    // `hierarchy` returns the pericentre in canonical units, `a(1 − e)·λ`; the semi-major axis scales with it.
    let a = peri / (1.0 - ECCENTRICITY);
    let mu = ic.m[0] + ic.m[1];
    Kepler {
        state: State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        },
        period: std::f64::consts::TAU * (a * a * a / mu).sqrt(),
        tau_peri: (peri * peri * peri / mu).sqrt(),
    }
}

/// The state's energy at f64.
pub fn energy_of(s: &State<f64>, g: &Geom<f64>) -> f64 {
    energy(&s.m, &s.p, &g.pair)
}

/// `max_t |E(t) − E₀| / |E₀|` over `n` steps of `occupant` at `dt = P/n` from the fixture.
pub fn max_energy_error(occupant: Selection, kepler: &Kepler, n: usize) -> f64 {
    let mut s = kepler.state;
    let mut g = Geom::new(&s.m, &s.r);
    let e0 = energy_of(&s, &g);
    let dt = kepler.period / n as f64;
    let mut worst = 0.0f64;
    for _ in 0..n {
        (s, g) = occupant.step(&s, dt, &g);
        let e = ((energy_of(&s, &g) - e0) / e0).abs();
        // A non-finite error is kept, never lost to `max`: NaN poisons the rung.
        worst = if e.is_nan() { e } else { worst.max(e) };
        if worst.is_nan() {
            break;
        }
    }
    worst
}

/// The occupants the gate measures, by their input names.
pub fn occupant(name: &str) -> Option<Selection> {
    Some(match name {
        "euler" => Selection::Debug(DebugOccupant::Euler),
        "kdk" => Selection::Production(Occupant::Kdk),
        "yoshida4" => Selection::Production(Occupant::Yoshida4),
        "yoshida6" => Selection::Production(Occupant::Yoshida6),
        "rk4" => Selection::Production(Occupant::Rk4),
        _ => return None,
    })
}

/// The occupants' input names.
pub const OCCUPANTS: [&str; 5] = ["euler", "kdk", "yoshida4", "yoshida6", "rk4"];

/// One rung: steps per orbit, the step, its ratio to `τ_p`, and the error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rung {
    /// Steps per orbit.
    pub n: usize,
    /// `dt = P/n`.
    pub dt: f64,
    /// `dt / τ_p`.
    pub ratio: f64,
    /// The maximum relative energy error over the orbit.
    pub error: f64,
}

/// A least-squares fit of `ln err` against `ln dt`, with its spread.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    /// The slope.
    pub slope: f64,
    /// The slope's standard error, from the residuals (`k − 2` degrees of freedom).
    pub std_err: f64,
    /// The local slope between each adjacent pair of rungs, coarse to fine.
    pub local: Vec<f64>,
}

/// The least-squares fit of `y` against `x` over `points`, with the slope's standard error and the adjacent local
/// slopes. Needs at least three points.
pub fn fit(points: &[(f64, f64)]) -> Fit {
    let k = points.len() as f64;
    let mx = points.iter().map(|p| p.0).sum::<f64>() / k;
    let my = points.iter().map(|p| p.1).sum::<f64>() / k;
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let slope = sxy / sxx;
    let rss: f64 = points
        .iter()
        .map(|p| (p.1 - my - slope * (p.0 - mx)).powi(2))
        .sum();
    let std_err = (rss / (k - 2.0) / sxx).sqrt();
    let local = points
        .windows(2)
        .map(|w| (w[1].1 - w[0].1) / (w[1].0 - w[0].0))
        .collect();
    Fit {
        slope,
        std_err,
        local,
    }
}

/// An `order-scaling` input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The occupant ([`OCCUPANTS`]).
    pub occupant: String,
    /// Steps per orbit, coarse to fine.
    pub ladder: Vec<usize>,
    /// The order the slope is judged against, in place of the profile's: the mislabelled-order control.
    #[serde(default)]
    pub claimed_order: Option<u32>,
}

/// A judged ladder.
#[derive(Clone, Debug, PartialEq)]
pub struct Judged {
    /// The rungs.
    pub rungs: Vec<Rung>,
    /// The fit.
    pub fit: Fit,
    /// The order judged against.
    pub order: u32,
}

/// The rungs of `ladder` for `occupant`, refused when the ladder is not three or more strictly increasing step counts
/// or a rung is outside `regime` (the module's note: the whole ladder, never a rung dropped).
pub fn rungs(
    occupant: Selection,
    kepler: &Kepler,
    ladder: &[usize],
    regime: &Regime,
) -> Result<Vec<Rung>, String> {
    if ladder.len() < 3 {
        return Err(format!(
            "the ladder has {} rung(s); a slope and its spread need at least 3",
            ladder.len()
        ));
    }
    if ladder.windows(2).any(|w| w[1] <= w[0]) || ladder[0] == 0 {
        return Err(format!(
            "the ladder {ladder:?} is not strictly increasing positive step counts"
        ));
    }
    let mut out = Vec::with_capacity(ladder.len());
    for &n in ladder {
        let dt = kepler.period / n as f64;
        let ratio = dt / kepler.tau_peri;
        if ratio > regime.max_step_ratio {
            return Err(format!(
                "the rung n = {n} has dt/τ_p = {ratio:.3}, above the coarse bound {}: outside the asymptotic regime",
                regime.max_step_ratio
            ));
        }
        let error = max_energy_error(occupant, kepler, n);
        if !(error >= regime.floor && error <= regime.ceiling) {
            return Err(format!(
                "the rung n = {n} has error {error:.3e}, outside [{:e}, {:e}]: outside the asymptotic regime",
                regime.floor, regime.ceiling
            ));
        }
        out.push(Rung {
            n,
            dt,
            ratio,
            error,
        });
    }
    Ok(out)
}

/// Judges `input` on the fixture against `regime`.
pub fn judge(input: &Input, regime: &Regime) -> Result<Judged, String> {
    let selection = occupant(&input.occupant).ok_or_else(|| {
        format!(
            "no occupant `{}` ({})",
            input.occupant,
            OCCUPANTS.join(", ")
        )
    })?;
    let kepler = kepler_embedded();
    let rungs = rungs(selection, &kepler, &input.ladder, regime)?;
    let points: Vec<(f64, f64)> = rungs.iter().map(|r| (r.dt.ln(), r.error.ln())).collect();
    Ok(Judged {
        fit: fit(&points),
        rungs,
        order: input.claimed_order.unwrap_or(selection.profile().order),
    })
}

/// The `order-scaling` gate (REQ-VAL-030; its tolerance REQ-VAL-139's).
pub struct OrderScalingGate;

impl Gate for OrderScalingGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        let j = judge(&input, &REGIME).map_err(|e| format!("refused: {name}: {e}"))?;
        let tol = config.threshold.value;
        let off = (j.fit.slope - f64::from(j.order)).abs();
        let pass = off <= tol;
        let mut scatter: Vec<String> = j
            .rungs
            .iter()
            .map(|r| {
                format!(
                    "{}: n = {}, dt = {:.4e}, dt/τ_p = {:.3e}, max |ΔE/E₀| = {:.4e}",
                    input.occupant, r.n, r.dt, r.ratio, r.error
                )
            })
            .collect();
        let local: Vec<String> = j.fit.local.iter().map(|s| format!("{s:.3}")).collect();
        scatter.push(format!(
            "{}: local slopes, coarse to fine: {}",
            input.occupant,
            local.join(", ")
        ));
        let (lo, hi) = j
            .fit
            .local
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &s| {
                (a.min(s), b.max(s))
            });
        scatter.push(format!(
            "{}: fitted slope {:.4} ± {:.4} (standard error), local slopes from {lo:.3} to {hi:.3}; order {}{}",
            input.occupant,
            j.fit.slope,
            j.fit.std_err,
            j.order,
            if input.claimed_order.is_some() {
                " (claimed by the input, not the profile)"
            } else {
                " (the profile's)"
            }
        ));
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(j.rungs.len() as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![if pass {
                format!(
                    "the fitted slope {:.4} is within {tol} of the order {} (off by {off:.4})",
                    j.fit.slope, j.order
                )
            } else {
                format!(
                    "the fitted slope {:.4} is off the order {} by {off:.4}, above {tol}",
                    j.fit.slope, j.order
                )
            }],
        })
    }
}
