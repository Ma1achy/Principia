//! The exact-cases gate, `exact-cases` (validation_ground_truth_note, "Four tiers" and "Chaos forbids pointwise
//! comparison"; dd_validation_orbits §1.3, §3; TASK-M3-28): a relative equilibrium is rigid, so its march must stay on
//! the closed form at every sampled `t`, not only at the end. Lagrange's equilateral triangle stays equilateral, its
//! sides at the closed form's length; Euler's collinear line stays collinear, its sides at theirs.
//!
//! **Why both residuals.** Every homographic solution keeps Lagrange's triangle equilateral: a start with the wrong
//! angular velocity pulses on Kepler ellipses and stays equilateral throughout. The equilateral residual alone therefore
//! cannot see a march that leaves the circular solution, so the gate judges the rigidity residual beside it, each side's
//! length against the closed form's, `ρ(t) = max_{ij} |s_ij(t)/s_ij(0) − 1|`, which a pulsing triangle fails.
//!
//! **The march** is the occupant's own direct march at f64 (the comparison harness's [`compare::run_sampled`]), from the
//! closed form's start in the canonical gauge (Path B's: the chart is CHECK 1's to test, in `tests/ground_truth.rs`),
//! sampled at every sync boundary it lands on, on a grid that lands on the horizon: `n = ⌈T/Δ_sync⌉` boundaries of
//! `T/n`, the kernel's `Δ_sync` rounded down to divide the horizon (as the closure gate's grid, the same reason).
//!
//! **The horizon** is one full turn. Both configurations are linearly unstable at these masses, so a perturbation of
//! the size of the truncation error grows over the turn; the RK45 reference's own residual at the same samples is
//! measured beside every run and reported, so that growth is read off the reference, not guessed.
//!
//! **Pass criteria, fixed before the first run:** the march lands on every one of the `n` boundaries, still marching;
//! at every sample the rigidity residual `ρ` is at most the threshold, and so is the shape residual: the equilateral
//! residual `(max side − min side)/mean side` for Lagrange, the collinearity residual `2·area/(max side)²` for Euler.
//!
//! **The proposal** (`--propose`, [`propose`]) measures each exact case at f64 and proposes the tolerance by a rule fixed
//! before the run: one decade above the worst residual over the inputs expected to pass, rounded up to a decade; it
//! states each structural window beside it, with the measurement it rests on.

use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;

use engine::precision::rk45::{self, Rk45End};
use kernel::decode::Ic;
use kernel::driver::advance::Cadence;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::End;
use kernel::regularisation::sync::delta_sync;

use crate::compare::{self, Discretisation};
use crate::gate::figure_eight::{self, Named};
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::ground_truth::tiers::{self, Rigid, REFERENCE};
use crate::ground_truth::{burrau_time_scale, path_a};
use crate::import::unit_state;
use kernel::generated::links::Defaults;

/// The gate's name.
pub const NAME: &str = "exact-cases";

/// A relative equilibrium the gate runs.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    /// Lagrange's equilateral triangle at the input's masses ([`tiers::lagrange`]).
    Lagrange,
    /// Euler's collinear line at equal masses ([`tiers::euler`]).
    Euler,
}

/// A displacement of one body from the closed form's start, a control's.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Displace {
    /// The body.
    pub body: usize,
    /// Its displacement.
    pub by: [f64; 2],
}

/// An input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The configuration.
    pub case: Case,
    /// The masses, `Σm = 1` (Lagrange's; Euler's are equal and this is refused for it).
    #[serde(default)]
    pub masses: Option<[f64; 3]>,
    /// The stepper's name ([`figure_eight::stepper`]).
    pub stepper: String,
    /// The regularisation's name ([`figure_eight::regularisation`]).
    pub regularisation: String,
    /// The step factor `η`.
    pub eta: f64,
    /// A displacement of the start, a control's.
    #[serde(default)]
    pub displace: Option<Displace>,
    /// Whether the input is a control, which must fail; the proposal measures the others.
    #[serde(default)]
    pub control: bool,
}

impl Input {
    /// The closed form and the start the march takes: the closed form's, displaced where the input says.
    pub fn rigid(&self) -> Result<(Rigid, State<f64>), String> {
        let rig = match (self.case, self.masses) {
            (Case::Lagrange, Some(m)) => {
                if !(m.iter().all(|&x| x > 0.0 && x.is_finite())
                    && (m.iter().sum::<f64>() - 1.0).abs() < 1e-15)
                {
                    return Err(format!(
                        "refused: the masses {m:?} are not positive with Σm = 1"
                    ));
                }
                tiers::lagrange(m)
            }
            (Case::Lagrange, None) => {
                return Err("refused: Lagrange's case names no masses".to_owned())
            }
            (Case::Euler, None) => tiers::euler(),
            (Case::Euler, Some(_)) => {
                return Err("refused: Euler's case is at equal masses and names none".to_owned())
            }
        };
        let s = rig.start;
        let mut state = State {
            m: s.m,
            r: s.r,
            p: s.p,
        };
        if let Some(d) = self.displace {
            let r = state
                .r
                .get_mut(d.body)
                .ok_or_else(|| format!("refused: no body {}", d.body))?;
            r[0] += d.by[0];
            r[1] += d.by[1];
        }
        Ok((rig, state))
    }

    /// The occupant.
    pub fn named(&self) -> Result<Named, String> {
        Ok(Named {
            stepper: figure_eight::stepper(&self.stepper)?,
            regularisation: figure_eight::regularisation(&self.regularisation)?,
            landing: true,
        })
    }
}

/// The three side lengths `|r₁ − r₂|`, `|r₀ − r₂|`, `|r₀ − r₁|`.
pub fn sides(r: &[[f64; 2]; 3]) -> [f64; 3] {
    let d = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    [d(1, 2), d(0, 2), d(0, 1)]
}

/// The rigidity residual: each side against the closed form's, `max |s/s₀ − 1|`.
pub fn rigidity(r: &[[f64; 2]; 3], closed: &[f64; 3]) -> f64 {
    let s = sides(r);
    (0..3)
        .map(|k| (s[k] / closed[k] - 1.0).abs())
        .fold(
            0.0,
            |a: f64, b| if b.is_nan() { f64::NAN } else { a.max(b) },
        )
}

/// The equilateral residual, `(max side − min side)/mean side`.
pub fn equilateral(r: &[[f64; 2]; 3]) -> f64 {
    let s = sides(r);
    let max = s[0].max(s[1]).max(s[2]);
    let min = s[0].min(s[1]).min(s[2]);
    (max - min) / ((s[0] + s[1] + s[2]) / 3.0)
}

/// The collinearity residual, `2·area/(max side)²`: 0 on a line, `√3/2` on an equilateral triangle.
pub fn collinear(r: &[[f64; 2]; 3]) -> f64 {
    let a = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let b = [r[2][0] - r[0][0], r[2][1] - r[0][1]];
    let s = sides(r);
    let max = s[0].max(s[1]).max(s[2]);
    (a[0] * b[1] - a[1] * b[0]).abs() / (max * max)
}

/// The shape residual of `case` at the positions `r`.
pub fn shape(case: Case, r: &[[f64; 2]; 3]) -> f64 {
    match case {
        Case::Lagrange => equilateral(r),
        Case::Euler => collinear(r),
    }
}

/// The states at each sync boundary a march lands on, by index.
#[derive(Default)]
pub struct Samples(pub Vec<(u32, State<f64>)>);

impl Cadence<f64> for Samples {
    fn after_step(&mut self, _state: &mut State<f64>, _geom: &mut Geom<f64>) {}
    fn done(&self) -> bool {
        false
    }
    fn sample(&mut self, state: &State<f64>, index: u32, _quantum: f64) {
        self.0.push((index, *state));
    }
}

/// One sample's residuals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The boundary.
    pub index: u32,
    /// Its time.
    pub t: f64,
    /// The march's rigidity residual.
    pub rigidity: f64,
    /// The march's shape residual.
    pub shape: f64,
    /// The march's distance from the closed form, `‖·‖_phys`: its phase as well as its shape.
    pub closed_form: f64,
    /// The reference's rigidity residual at the same time.
    pub reference_rigidity: f64,
    /// The reference's shape residual.
    pub reference_shape: f64,
}

/// A run of one input.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// The configuration.
    pub case: Case,
    /// The occupant.
    pub named: Named,
    /// The grid.
    pub grid: Discretisation,
    /// The horizon, one turn.
    pub horizon: f64,
    /// The samples, every boundary landed on.
    pub samples: Vec<Sample>,
    /// How the march ended.
    pub end: End,
    /// The boundary it ended on.
    pub boundary: u32,
    /// The steps taken.
    pub steps: u32,
}

/// The largest of `f` over `samples`; NaN where any is NaN or there are none.
pub fn worst(samples: &[Sample], f: impl Fn(&Sample) -> f64) -> f64 {
    if samples.is_empty() {
        return f64::NAN;
    }
    samples.iter().map(f).fold(0.0, |a: f64, b| {
        if a.is_nan() || b.is_nan() {
            f64::NAN
        } else {
            a.max(b)
        }
    })
}

impl Run {
    /// The worst residual the gate judges, rigidity or shape, over every sample.
    pub fn judged(&self) -> f64 {
        worst(&self.samples, |s| s.rigidity.max(s.shape))
    }

    /// The samples, as markdown lines: every eighth boundary and the last, and the worst of each column.
    pub fn table(&self) -> Vec<String> {
        let mut out = vec![
            format!(
                "{:?} by {}: one turn, T = {:.9}, on {} boundaries of Δ_sync {:.9}; the march ended {:?} on boundary {} \
                 after {} steps",
                self.case,
                self.named.describe(),
                self.horizon,
                self.grid.boundaries().unwrap_or(0),
                self.grid.delta_sync,
                self.end,
                self.boundary,
                self.steps
            ),
            "| boundary | t | rigidity | shape | closed form | reference rigidity | reference shape |".to_owned(),
            "|---|---|---|---|---|---|---|".to_owned(),
        ];
        let last = self.samples.len().saturating_sub(1);
        for (i, s) in self.samples.iter().enumerate() {
            if i % 8 == 0 || i == last {
                out.push(format!(
                    "| {} | {:.4} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} |",
                    s.index,
                    s.t,
                    s.rigidity,
                    s.shape,
                    s.closed_form,
                    s.reference_rigidity,
                    s.reference_shape
                ));
            }
        }
        out.push(format!(
            "| worst | | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} |",
            worst(&self.samples, |s| s.rigidity),
            worst(&self.samples, |s| s.shape),
            worst(&self.samples, |s| s.closed_form),
            worst(&self.samples, |s| s.reference_rigidity),
            worst(&self.samples, |s| s.reference_shape)
        ));
        out
    }
}

/// The input's run (the module's note).
pub fn run(input: &Input) -> Result<Run, String> {
    if !(input.eta.is_finite() && input.eta > 0.0) {
        return Err(format!(
            "refused: η {} is not finite and positive",
            input.eta
        ));
    }
    let (rig, start) = input.rigid()?;
    let named = input.named()?;
    let horizon = rig.period();
    let n = (horizon / delta_sync::<f64>()).ceil() as u32;
    let grid = figure_eight::grid(horizon, n)
        .ok_or_else(|| format!("refused: a grid of {n} boundaries does not land on {horizon}"))?;
    let mut sampled = Samples::default();
    let t = compare::run_sampled(&named.arm(input.eta, grid), &start, &mut sampled);
    let times: Vec<f64> = sampled.0.iter().map(|(k, _)| grid.time(*k)).collect();
    let reference = rk45::integrate(&start, &times, &REFERENCE);
    let closed = sides(&rig.start.r);
    let samples = sampled
        .0
        .iter()
        .enumerate()
        .map(|(i, (k, s))| {
            let t = grid.time(*k);
            let at = rig.at(t);
            let x = Ic {
                m: s.m,
                r: s.r,
                p: s.p,
            };
            let (rr, rs) = match (reference.end, reference.states.get(i)) {
                (Rk45End::Reached, Some(x)) => (rigidity(&x.r, &closed), shape(input.case, &x.r)),
                _ => (f64::NAN, f64::NAN),
            };
            Sample {
                index: *k,
                t,
                rigidity: rigidity(&s.r, &closed),
                shape: shape(input.case, &s.r),
                closed_form: crate::roundtrip::phys(&at, &x),
                reference_rigidity: rr,
                reference_shape: rs,
            }
        })
        .collect();
    Ok(Run {
        case: input.case,
        named,
        grid,
        horizon,
        samples,
        end: t.end,
        boundary: t.boundary,
        steps: t.steps,
    })
}

/// The gate's judgement of `run` under `threshold`: the verdict and the negative-results lines.
pub fn judge(run: &Run, threshold: f64) -> (Verdict, Vec<String>) {
    let n = run.grid.boundaries().unwrap_or(0);
    let indices: Vec<u32> = run.samples.iter().map(|s| s.index).collect();
    let every =
        indices == (1..=n).collect::<Vec<_>>() && run.end == End::Marching && run.boundary == n;
    let over: Vec<&Sample> = run
        .samples
        .iter()
        .filter(|s| !(s.rigidity <= threshold && s.shape <= threshold))
        .collect();
    let mut lines = vec![if every {
        format!("the march landed on every one of the {n} boundaries, still marching")
    } else {
        format!(
            "the march did not land on every one of the {n} boundaries: {} sampled, ended {:?} on boundary {}",
            indices.len(),
            run.end,
            run.boundary
        )
    }];
    match over.first() {
        None => lines.push(format!(
            "at every one of the {} samples the rigidity and shape residuals are within the threshold {threshold:e} \
             (worst {:.3e})",
            run.samples.len(),
            run.judged()
        )),
        Some(first) => lines.push(format!(
            "{} of {} samples exceed the threshold {threshold:e}, the first at boundary {} (t = {:.4}): rigidity \
             {:.3e}, shape {:.3e}; the worst {:.3e}",
            over.len(),
            run.samples.len(),
            first.index,
            first.t,
            first.rigidity,
            first.shape,
            run.judged()
        )),
    }
    let ok = every && over.is_empty() && !run.samples.is_empty();
    (if ok { Verdict::Pass } else { Verdict::Fail }, lines)
}

/// The gate.
pub struct ExactCasesGate;

impl Gate for ExactCasesGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input: Input =
            serde_json::from_value(fixture.clone()).map_err(|e| format!("{name}: {e}"))?;
        let run = run(&input)?;
        let (verdict, negative) = judge(&run, config.threshold.value);
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict,
            threshold: config.threshold.clone(),
            scatter: run.table(),
            regions: Some(Regions {
                count: RegionCount::Recorded(run.samples.len() as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => propose(root).map(|s| print!("{s}")),
            _ => Err(format!(
                "gate {NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}

/// The gate's inputs under `root`, by file, controls (`"control": true`) left out.
fn inputs(root: &Path) -> Result<Vec<(String, Input)>, String> {
    let dir = root.join("fixtures/gates").join(NAME);
    let config = crate::gate::config(root, NAME)?;
    let mut out = Vec::new();
    for file in &config.inputs {
        let path = dir.join(file);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(o) = v.as_object_mut() {
            o.remove("expected");
        }
        let input: Input =
            serde_json::from_value(v).map_err(|e| format!("{}: {e}", path.display()))?;
        if !input.control {
            out.push((file.clone(), input));
        }
    }
    Ok(out)
}

/// REQ-VAL-130's proposal (the module's note): each exact case at f64, its worst residual and the reference's, the
/// tolerance by the rule; the figure-eight's closure, from its own gate; and each structural window, with the
/// measurement it rests on.
pub fn propose(root: &Path) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(out, "# REQ-VAL-130: the exact-case tolerances and the structural windows, proposed ({NAME} --propose)\n");
    let _ = writeln!(
        out,
        "The rule, fixed before the run: the tolerance is one decade above the worst residual (rigidity or shape, over \
         every sample) of the inputs expected to pass, rounded up to a decade.\n"
    );
    let mut worst_all = 0.0f64;
    for (file, input) in inputs(root)? {
        let r = run(&input)?;
        let _ = writeln!(out, "## {file}\n");
        for line in r.table() {
            let _ = writeln!(out, "{line}");
        }
        let (_, lines) = judge(&r, f64::INFINITY);
        for line in lines {
            let _ = writeln!(out, "- {line}");
        }
        worst_all = worst_all.max(r.judged());
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "**Proposed:** the exact-case tolerance {:e} (the worst residual measured is {worst_all:.3e}).\n",
        figure_eight::proposed_threshold(worst_all)
    );
    let _ = writeln!(
        out,
        "The figure-eight's closure tolerance is the closure gate's threshold: `cargo xtask gate figure-eight-closure \
         --propose` measures it.\n"
    );
    let _ = writeln!(
        out,
        "## The structural windows (Burrau, Szebehely & Peters' time units)\n"
    );
    let scale = burrau_time_scale();
    // The start as CHECK 1 decodes it, in the canonical gauge, whose time unit `burrau_time_scale` converts to.
    let x = path_a::check1::<Defaults>(&unit_state(&tiers::burrau())?).decoded;
    let times: Vec<f64> = tiers::ejection_samples()
        .iter()
        .map(|t| t * scale)
        .collect();
    let state = State {
        m: x.m,
        r: x.r,
        p: x.p,
    };
    let reference = rk45::integrate(&state, &times, &REFERENCE);
    let ejected = (reference.end == Rk45End::Reached).then(|| {
        let samples: Vec<(f64, Ic<f64>)> = tiers::ejection_samples()
            .into_iter()
            .zip(&reference.states)
            .map(|(t, s)| {
                (
                    t,
                    Ic {
                        m: s.m,
                        r: s.r,
                        p: s.p,
                    },
                )
            })
            .collect();
        tiers::ejection_time(&samples, tiers::lightest(x.m))
    });
    let _ = writeln!(
        out,
        "- The escape label's window {:?}, its upper edge the horizon: the literature's disruption near 60 (dd_validation_orbits \
         §1.5), coarse, for gross breakage only.",
        tiers::BURRAU_WINDOW
    );
    let _ = writeln!(
        out,
        "- The ejection window {:?}: the RK45 reference leaves the lightest body unbound from {} on (the 0.5 grid of \
         `tiers::ejection_samples`).",
        tiers::BURRAU_EJECTION_WINDOW,
        match ejected {
            Some(Some(t)) => format!("{t}"),
            Some(None) => "no sample".to_owned(),
            None => format!("nowhere: the reference stopped {:?}", reference.end),
        }
    );
    Ok(out)
}
