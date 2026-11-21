//! The occupant-comparison harness (integrator_contract Part 2b § "The comparison discipline is part of the seam, NOT a
//! convention", § "One thing deliberately NOT decided"; pitfalls §3, §4.4, §5).
//!
//! **The discipline, enforced, not advised.** [`compare`] takes arms ([`Arm`]: a stepper, a regularisation, a step
//! control and a discretisation) and refuses, with a typed [`Refusal`] and no result, any set whose arms differ in
//! anything but the regularisation: the same stepper, the same step control and the same discretisation on every arm.
//! A GBS logH against an RK4 Heggie scores the integrator, not the regularisation.
//!
//! **Fixed ramp constants, shared.** Every panel of a [`Comparison`] is drawn through the one [`RampConstants`] the
//! caller passed in; nothing here ranges a ramp from the data (pitfalls §5: an auto-ranged ramp manufactures or hides the
//! difference it is meant to show).
//!
//! **Full provenance per arm.** [`provenance`] names the stepper, the regularisation with its time transformation, the
//! composed occupant's capability profile, its pair-table cell, every step-control value and the discretisation.
//!
//! **A diagnostic reuses the run's discretisation** (pitfalls §3; integrator_contract Part 2a, the sync grid).
//! [`rerun`] re-runs trajectories for a diagnostic (a persistence check, say) only with the run under test's own arm:
//! a rescaled `Δ_sync`, another horizon or any other step parameter is refused, field by field ([`Mismatch`]).
//!
//! **Which would have been better** ([`Comparison::better`]): the report Part 2b defines, judged per IC on the state
//! against an independent reference (the Precision ring's RK45 at f64), never in render space, and never acted on. No
//! function here returns an occupant chosen from a trajectory: the occupant is the caller's choice, and the report is
//! data.
//!
//! **What the runs are.** Each arm runs the occupant's own direct march at f64 (`kernel::regularisation::{az, heggie,
//! logh}::march`), the march the seam's `ADVANCE` is bitwise (`kernel/tests/sync_quantum.rs`), because an arm's step
//! control is a value here and the seam reads it from a type. The march gets no cadence: no projection, no terminal
//! detection (collision, escape); each trajectory ends on its last boundary, or where its march ended non-finite or
//! on its step budget, as its [`Trajectory::end`] says.

use engine::precision::rk45::{self, Rk45End, Rk45Params};
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, DebugOccupant, Occupant, Selection, State, Stepper};
use kernel::regularisation::az::{self, AzCount, AzParams, End, ExplicitTest};
use kernel::regularisation::heggie::{self as hg, HgParams};
use kernel::regularisation::logh::{self as lh, LhParams, LhStepper, Slot};
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode, Pair, PairTable};

use crate::az::{distance, Quiet};
use crate::pair_table::{label, stepper_name};

// ── An arm ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The values a regularised occupant's step control reads (integrator_contract Part 2b): the fields Heggie's, logH's
/// and Aarseth–Zare's parameters share, held once so that the check and the run read the same numbers. `Δ_sync` is
/// the [`Discretisation`]'s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepControl {
    /// The step factor `η`.
    pub eta: f64,
    /// The floor `TINY`.
    pub tiny: f64,
    /// The predictive limit's `f`; off where not positive.
    pub predictive_f: f64,
    /// The step budget's rate, steps per unit physical time per copy.
    pub step_budget_rate: f64,
    /// The step budget's floor `N_min`.
    pub n_min: u32,
    /// Whether the landing clamp is on.
    pub clamp: bool,
    /// The landing secant's most retakes.
    pub secant_tries: u32,
    /// The landing bisection's most halvings.
    pub bisect_tries: u32,
}

impl StepControl {
    /// The values the kernel's regularised tests run (`kernel/tests/sync_quantum.rs`, `heggie_seam.rs`): `η = 0.01`, the
    /// reference's f64 `TINY`, `f = 0.02`, the landing in force, with a finite budget, so that a logH arm whose predictive
    /// limit spends steps at a collision ends `Exhausted` rather than running on. Test inputs: the step control has no
    /// register entry yet (`pairs::DefaultOccupant`'s note).
    pub const TEST: StepControl = StepControl {
        eta: 0.01,
        tiny: 1e-300,
        predictive_f: 0.02,
        step_budget_rate: 1e5,
        n_min: 200_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
    };

    /// Heggie's parameters under the time transformation `time` on `d`.
    pub fn heggie(&self, d: &Discretisation, time: HeggieTime) -> HgParams<f64> {
        HgParams {
            eta: self.eta,
            delta_sync: d.delta_sync,
            tiny: self.tiny,
            predictive_f: self.predictive_f,
            step_budget_rate: self.step_budget_rate,
            n_min: self.n_min,
            clamp: self.clamp,
            secant_tries: self.secant_tries,
            bisect_tries: self.bisect_tries,
            time,
        }
    }

    /// logH's parameters in the time mode `mode` on `d`.
    pub fn logh(&self, d: &Discretisation, mode: LogHMode) -> LhParams<f64> {
        LhParams {
            eta: self.eta,
            delta_sync: d.delta_sync,
            tiny: self.tiny,
            predictive_f: self.predictive_f,
            step_budget_rate: self.step_budget_rate,
            n_min: self.n_min,
            clamp: self.clamp,
            secant_tries: self.secant_tries,
            bisect_tries: self.bisect_tries,
            mode,
        }
    }

    /// Aarseth–Zare's parameters on `d`.
    pub fn aarseth_zare(&self, d: &Discretisation) -> AzParams<f64> {
        AzParams {
            eta: self.eta,
            delta_sync: d.delta_sync,
            tiny: self.tiny,
            predictive_f: self.predictive_f,
            step_budget_rate: self.step_budget_rate,
            n_min: self.n_min,
            clamp: self.clamp,
            secant_tries: self.secant_tries,
            bisect_tries: self.bisect_tries,
        }
    }

    /// Each field by name, as [`Mismatch`] names the first that differs.
    fn fields(&self) -> [(&'static str, f64); 8] {
        [
            ("eta", self.eta),
            ("tiny", self.tiny),
            ("predictive_f", self.predictive_f),
            ("step_budget_rate", self.step_budget_rate),
            ("n_min", self.n_min as f64),
            ("clamp", self.clamp as u8 as f64),
            ("secant_tries", self.secant_tries as f64),
            ("bisect_tries", self.bisect_tries as f64),
        ]
    }
}

/// The discretisation a run is on: the sync quantum and the horizon. The march stops only on the grid `t = k·Δ_sync`,
/// so a run's last boundary is `⌊T/Δ_sync⌋`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Discretisation {
    /// The sync quantum `Δ_sync`.
    pub delta_sync: f64,
    /// The horizon `T`.
    pub horizon: f64,
}

impl Discretisation {
    /// The last boundary at or before the horizon, `⌊T/Δ_sync⌋`; `None` where either value is not finite and positive.
    pub fn boundaries(&self) -> Option<u32> {
        let ok = |x: f64| x.is_finite() && x > 0.0;
        (ok(self.delta_sync) && ok(self.horizon))
            .then(|| (self.horizon / self.delta_sync).floor())
            .filter(|k| *k >= 1.0 && *k <= u32::MAX as f64)
            .map(|k| k as u32)
    }

    /// The physical time of boundary `k`, `k·Δ_sync`, as the marches form it.
    pub fn time(&self, k: u32) -> f64 {
        k as f64 * self.delta_sync
    }

    fn fields(&self) -> [(&'static str, f64); 2] {
        [("delta_sync", self.delta_sync), ("horizon", self.horizon)]
    }
}

/// One arm of a comparison: the occupant (a stepper under a regularisation, with its time transformation) and what it
/// runs under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arm {
    /// The stepper.
    pub stepper: Selection,
    /// The regularisation, with its time transformation.
    pub regularisation: Choice,
    /// The step control's values.
    pub step_control: StepControl,
    /// The discretisation.
    pub discretisation: Discretisation,
}

impl Arm {
    /// The composed occupant's capability profile: the pair's, with `owns_time_mapping` the regularisation's
    /// (`kernel::regularisation::Composed::profile`).
    pub fn profile(&self) -> CapabilityProfile {
        let pair = match self.regularisation {
            Choice::None => self.stepper.profile(),
            Choice::AarsethZare | Choice::Heggie(_) => with_stepper(self.stepper, FormProfile),
            Choice::LogH(mode) => with_stepper(self.stepper, LogHProfile(mode)),
        };
        CapabilityProfile {
            owns_time_mapping: self.regularisation.kind().owns_time_mapping(),
            ..pair
        }
    }
}

// ── Refusals ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A field in which a diagnostic's arm differs from the run under test's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mismatch {
    /// The field's name: `delta_sync`, `horizon`, a [`StepControl`] field's, `stepper` or `regularisation`.
    pub field: &'static str,
    /// The run under test's value (`NaN` for the occupant's fields, which are not numbers).
    pub run: f64,
    /// The diagnostic's value.
    pub diagnostic: f64,
}

/// Why the harness gave no result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Refusal {
    /// Fewer than two arms: there is nothing to compare.
    TooFewArms(usize),
    /// Arm `arm` is unregularised (`none`). Its step control is the wrapper's substep law, not the regularised step
    /// control, so it cannot share one with any arm here.
    Unregularised {
        /// The arm.
        arm: usize,
    },
    /// Arm `arm`'s discretisation is not a run: `Δ_sync` or `T` not finite and positive, or `T < Δ_sync`.
    NoBoundary {
        /// The arm.
        arm: usize,
    },
    /// Arm `arm`'s stepper differs from arm 0's.
    StepperMismatch {
        /// The arm.
        arm: usize,
        /// Arm 0's stepper.
        first: Selection,
        /// This arm's.
        other: Selection,
    },
    /// Arm `arm`'s step control differs from arm 0's in `field`.
    StepControlMismatch {
        /// The arm.
        arm: usize,
        /// The first field that differs.
        field: &'static str,
    },
    /// Arm `arm`'s discretisation differs from arm 0's in `field`.
    DiscretisationMismatch {
        /// The arm.
        arm: usize,
        /// The first field that differs.
        field: &'static str,
    },
    /// The logH-vs-Heggie harness was handed arms that are not one Heggie and one logH.
    NotLogHHeggie,
    /// The logH-vs-Heggie harness's arms share a stepper, but it is not RK4 (pitfalls §4.4: "Run it under the same RK4").
    NotRk4(Selection),
    /// A diagnostic's arm differs from the run under test's.
    Diagnostic(Mismatch),
}

/// The first field, by name, in which `a` and `b` differ; exact comparison, no tolerance. Compared as bits, so a `NaN`
/// equals itself and `0` differs from `-0`.
fn first_difference<const N: usize>(
    a: [(&'static str, f64); N],
    b: [(&'static str, f64); N],
) -> Option<Mismatch> {
    a.into_iter()
        .zip(b)
        .find(|((_, x), (_, y))| x.to_bits() != y.to_bits())
        .map(|((field, run), (_, diagnostic))| Mismatch {
            field,
            run,
            diagnostic,
        })
}

/// The discipline (the module's note): `Ok` where `arms` may be compared, the first refusal otherwise. The checks run in
/// order: the count, the unregularised arms, each arm's grid, then each arm against arm 0 in stepper, step control and
/// discretisation.
pub fn admit(arms: &[Arm]) -> Result<(), Refusal> {
    if arms.len() < 2 {
        return Err(Refusal::TooFewArms(arms.len()));
    }
    // The plan is unclear here: Part 2b keeps `none` as the independent control, but its step control is the wrapper's
    // substep law, which no regularised arm shares. Chose to refuse a `none` arm, because admitting it would compare two
    // step controls, exactly what the discipline forbids; `none` adjudicates through its own runs, not this report.
    if let Some(arm) = arms.iter().position(|a| a.regularisation == Choice::None) {
        return Err(Refusal::Unregularised { arm });
    }
    if let Some(arm) = arms
        .iter()
        .position(|a| a.discretisation.boundaries().is_none())
    {
        return Err(Refusal::NoBoundary { arm });
    }
    let first = arms[0];
    for (arm, a) in arms.iter().enumerate().skip(1) {
        if a.stepper != first.stepper {
            return Err(Refusal::StepperMismatch {
                arm,
                first: first.stepper,
                other: a.stepper,
            });
        }
        if let Some(m) = first_difference(first.step_control.fields(), a.step_control.fields()) {
            return Err(Refusal::StepControlMismatch {
                arm,
                field: m.field,
            });
        }
        if let Some(m) = first_difference(first.discretisation.fields(), a.discretisation.fields())
        {
            return Err(Refusal::DiscretisationMismatch {
                arm,
                field: m.field,
            });
        }
    }
    Ok(())
}

/// The logH-vs-Heggie harness's own admission (pitfalls §4.4, the outstanding falsification): one Heggie arm and one
/// logH arm, admitted by [`admit`] (the same stepper and step control), and that stepper RK4. Returns the arms
/// ordered `[Heggie, logH]`.
pub fn admit_logh_heggie(arms: [Arm; 2]) -> Result<[Arm; 2], Refusal> {
    let pair = match (arms[0].regularisation, arms[1].regularisation) {
        (Choice::Heggie(_), Choice::LogH(_)) => arms,
        (Choice::LogH(_), Choice::Heggie(_)) => [arms[1], arms[0]],
        _ => return Err(Refusal::NotLogHHeggie),
    };
    admit(&pair)?;
    if pair[0].stepper != Selection::Production(Occupant::Rk4) {
        return Err(Refusal::NotRk4(pair[0].stepper));
    }
    Ok(pair)
}

/// Whether a diagnostic's `arm` may re-run the run under test's trajectories: it must be the run's own arm, field by
/// field (pitfalls §3: a persistence check that rescaled `n_sync` per window read as a flickering condition). The
/// discretisation is checked first (`delta_sync`, `horizon`), then the step control, then the occupant.
pub fn admit_diagnostic(under_test: &Arm, arm: &Arm) -> Result<(), Refusal> {
    let mismatch = first_difference(
        under_test.discretisation.fields(),
        arm.discretisation.fields(),
    )
    .or_else(|| first_difference(under_test.step_control.fields(), arm.step_control.fields()))
    .or_else(|| {
        (under_test.stepper != arm.stepper).then_some(Mismatch {
            field: "stepper",
            run: f64::NAN,
            diagnostic: f64::NAN,
        })
    })
    .or_else(|| {
        (under_test.regularisation != arm.regularisation).then_some(Mismatch {
            field: "regularisation",
            run: f64::NAN,
            diagnostic: f64::NAN,
        })
    });
    match mismatch {
        Some(m) => Err(Refusal::Diagnostic(m)),
        None => Ok(()),
    }
}

/// A diagnostic's re-run of `ics` under `arm`, refused unless `arm` is the run under test's own ([`admit_diagnostic`]).
/// A window of the diagnostic reads the re-run's boundary states inside it; it never re-integrates on a grid of its
/// own.
pub fn rerun(under_test: &Arm, arm: &Arm, ics: &[State<f64>]) -> Result<Vec<Trajectory>, Refusal> {
    admit_diagnostic(under_test, arm)?;
    under_test
        .discretisation
        .boundaries()
        .ok_or(Refusal::NoBoundary { arm: 0 })?;
    Ok(ics.iter().map(|ic| run(arm, ic)).collect())
}

// ── Running an arm ─────────────────────────────────────────────────────────────────────────────────────────────

/// One arm's trajectory from one IC.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trajectory {
    /// The state at the boundary it ended on (or the last finite one, where its march ended non-finite).
    pub state: State<f64>,
    /// The boundary it ended on.
    pub boundary: u32,
    /// How its march ended: `Marching` where it reached its last boundary.
    pub end: End,
    /// Its running step total, every stepper call and landing retake counted.
    pub steps: u32,
    /// Its registrations at boundaries after `t = 0` (integrator_contract Part 2b, "The profile gains a field"):
    /// Aarseth–Zare's count less the one at `t = 0`; 0 for Heggie and logH, registered once.
    pub re_registrations: u32,
}

impl Trajectory {
    /// Whether it reached boundary `k` still marching.
    pub fn reached(&self, k: u32) -> bool {
        self.end == End::Marching && self.boundary == k
    }
}

/// What to do with a stepper type, chosen at run time from a [`Selection`].
trait WithStepper {
    type Out;
    fn go<S: Stepper>(self) -> Self::Out;
}

/// Calls `f` with the stepper `stepper` names, monomorphised.
fn with_stepper<F: WithStepper>(stepper: Selection, f: F) -> F::Out {
    match stepper {
        Selection::Debug(DebugOccupant::Euler) => f.go::<Euler>(),
        Selection::Production(Occupant::Kdk) => f.go::<Kdk>(),
        Selection::Production(Occupant::Yoshida4) => f.go::<Yoshida4>(),
        Selection::Production(Occupant::Yoshida6) => f.go::<Yoshida6>(),
        Selection::Production(Occupant::Rk4) => f.go::<Rk4>(),
    }
}

/// The pair's profile under Heggie or Aarseth–Zare: the stepper's form's.
struct FormProfile;

impl WithStepper for FormProfile {
    type Out = CapabilityProfile;
    fn go<S: Stepper>(self) -> CapabilityProfile {
        S::FORM.pair_profile()
    }
}

/// The pair's profile under logH in a time mode.
struct LogHProfile(LogHMode);

impl WithStepper for LogHProfile {
    type Out = CapabilityProfile;
    fn go<S: Stepper>(self) -> CapabilityProfile {
        <Slot<S> as LhStepper>::pair_profile(self.0)
    }
}

/// One arm's march from one IC.
struct March<'a> {
    arm: &'a Arm,
    ic: &'a State<f64>,
    k: u32,
}

impl WithStepper for March<'_> {
    type Out = Trajectory;
    fn go<S: Stepper>(self) -> Trajectory {
        let (arm, d) = (self.arm, &self.arm.discretisation);
        let c = &arm.step_control;
        match arm.regularisation {
            Choice::Heggie(time) => {
                let out = hg::march::<S, f64, Quiet, ExplicitTest>(
                    self.ic,
                    self.k,
                    &c.heggie(d, time),
                    &mut Quiet,
                    &mut ExplicitTest,
                );
                Trajectory {
                    state: out.state,
                    boundary: out.boundary,
                    end: out.count.end,
                    steps: out.count.steps,
                    re_registrations: 0,
                }
            }
            Choice::LogH(mode) => {
                let out = lh::march::<Slot<S>, f64, Quiet, ExplicitTest>(
                    self.ic,
                    self.k,
                    &c.logh(d, mode),
                    &mut Quiet,
                    &mut ExplicitTest,
                );
                Trajectory {
                    state: out.state,
                    boundary: out.boundary,
                    end: out.count.end,
                    steps: out.count.steps,
                    re_registrations: 0,
                }
            }
            Choice::AarsethZare => {
                let out = az::march::<S, f64, Quiet, ExplicitTest>(
                    self.ic,
                    0,
                    self.k,
                    AzCount::new(),
                    &c.aarseth_zare(d),
                    &mut Quiet,
                    &mut ExplicitTest,
                );
                Trajectory {
                    state: out.state,
                    boundary: out.boundary,
                    end: out.count.end,
                    steps: out.count.steps,
                    re_registrations: out.count.re_registrations.saturating_sub(1),
                }
            }
            Choice::None => unreachable!("admit refuses a `none` arm"),
        }
    }
}

/// `arm`'s trajectory from `ic` to its last boundary. `arm` must have been admitted: a `none` arm or one with no
/// boundary panics.
pub fn run(arm: &Arm, ic: &State<f64>) -> Trajectory {
    let k = arm
        .discretisation
        .boundaries()
        .expect("an admitted arm has a boundary");
    with_stepper(arm.stepper, March { arm, ic, k })
}

// ── The reference ──────────────────────────────────────────────────────────────────────────────────────────────

/// The reference's tolerance: RK45 at f64, `rtol = atol`.
pub const REFERENCE_TOL: f64 = 1e-13;
/// The looser tolerance whose run bounds the reference's own error: the reference's uncertainty is the distance between
/// the two runs' states, an overestimate of the tighter run's error. One decade looser: measured on five of the tests'
/// Burrau ICs to `T = 2` against a `1e-15` run, the error falls in proportion to the tolerance from `1e-10` to `1e-13`
/// (e.g. `7.7e-9`, `1.1e-9`, `1.2e-10`, `7.7e-12`) and is irregular below it (rounding), so the `1e-12` run's distance
/// is about ten times the `1e-13` run's error; a check at `1e-11` overstated it about a hundredfold, leaving most ICs
/// unresolved. Where the reference does not converge (a close encounter it cannot pass), the distance says so.
pub const REFERENCE_CHECK_TOL: f64 = 1e-12;
/// The reference's step budget, attempts accepted and rejected together.
pub const REFERENCE_STEPS: u32 = 5_000_000;

/// The independent reference at one IC: the RK45 state at the last boundary, and its own uncertainty there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    /// The state at the last boundary, where both runs reached it.
    pub state: Option<State<f64>>,
    /// The position distance between the tight and the check runs there (`NaN` where either fell short).
    pub uncertainty: f64,
}

/// The reference from `ic` to boundary `k` of `d`.
pub fn reference(ic: &State<f64>, d: &Discretisation, k: u32) -> Reference {
    let at = |tol: f64| {
        let r = rk45::integrate(
            ic,
            &[d.time(k)],
            &Rk45Params {
                rtol: tol,
                atol: tol,
                max_steps: REFERENCE_STEPS,
            },
        );
        (r.end == Rk45End::Reached).then(|| r.states[0])
    };
    match (at(REFERENCE_TOL), at(REFERENCE_CHECK_TOL)) {
        (Some(tight), Some(check)) => Reference {
            state: Some(tight),
            uncertainty: distance(&tight.r, &check.r),
        },
        (tight, _) => Reference {
            state: tight,
            uncertainty: f64::NAN,
        },
    }
}

// ── The ramp and the panels ────────────────────────────────────────────────────────────────────────────────────

/// The fixed constants of a comparison's colour ramp, shared by every panel: a value `x` sits at `(log₁₀ x − lo)/(hi −
/// lo)`, clamped to `[0, 1]`. Chosen before the data is seen, never from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RampConstants {
    /// The decade at the ramp's low end.
    pub lo_decade: f64,
    /// The decade at its high end.
    pub hi_decade: f64,
}

impl RampConstants {
    /// The position errors' ramp: `1e-14` (about the f64 rounding of a unit-scale position) to `1` (the system's own
    /// scale, `R₀ = 1` in the gauge).
    pub const POSITION_ERROR: RampConstants = RampConstants {
        lo_decade: -14.0,
        hi_decade: 0.0,
    };

    /// Where `x` sits on the ramp; `NaN` for a value that is not finite and positive, which the panel draws as invalid.
    pub fn place(&self, x: f64) -> f64 {
        if !(x.is_finite() && x > 0.0) {
            return f64::NAN;
        }
        ((x.log10() - self.lo_decade) / (self.hi_decade - self.lo_decade)).clamp(0.0, 1.0)
    }
}

/// One arm's panel: its per-IC values placed on the ramp, with the constants it was drawn through.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    /// The arm.
    pub arm: usize,
    /// The ramp's constants, the comparison's own.
    pub constants: RampConstants,
    /// Each IC's place on the ramp.
    pub placed: Vec<f64>,
}

/// The panels of `errors` (one row of per-IC values per arm), every one drawn through `ramp`.
pub fn panels(errors: &[Vec<f64>], ramp: RampConstants) -> Vec<Panel> {
    errors
        .iter()
        .enumerate()
        .map(|(arm, row)| Panel {
            arm,
            constants: ramp,
            placed: row.iter().map(|&x| ramp.place(x)).collect(),
        })
        .collect()
}

// ── The comparison ─────────────────────────────────────────────────────────────────────────────────────────────

/// A comparison's result: each arm's provenance and trajectories, the reference, and the panels.
#[derive(Clone, Debug)]
pub struct Comparison {
    /// The arms, as admitted.
    pub arms: Vec<Arm>,
    /// Each arm's provenance block ([`provenance`]).
    pub provenance: Vec<String>,
    /// The ramp every panel was drawn through.
    pub ramp: RampConstants,
    /// The last boundary every arm marches to.
    pub boundary: u32,
    /// `runs[arm][ic]`.
    pub runs: Vec<Vec<Trajectory>>,
    /// The reference at each IC.
    pub reference: Vec<Reference>,
    /// `errors[arm][ic]`: the position distance from the reference at the last boundary; `NaN` where the arm or the
    /// reference did not reach it.
    pub errors: Vec<Vec<f64>>,
    /// One panel per arm, of `errors`.
    pub panels: Vec<Panel>,
}

/// The comparison of `arms` on `ics`, every panel through `ramp`; the first [`Refusal`] and no result where the arms
/// break the discipline ([`admit`]).
pub fn compare(
    arms: &[Arm],
    ics: &[State<f64>],
    ramp: RampConstants,
) -> Result<Comparison, Refusal> {
    admit(arms)?;
    let d = arms[0].discretisation;
    let k = d.boundaries().ok_or(Refusal::NoBoundary { arm: 0 })?;
    let runs: Vec<Vec<Trajectory>> = arms
        .iter()
        .map(|arm| ics.iter().map(|ic| run(arm, ic)).collect())
        .collect();
    let reference: Vec<Reference> = ics.iter().map(|ic| reference(ic, &d, k)).collect();
    let errors: Vec<Vec<f64>> = runs
        .iter()
        .map(|row| {
            row.iter()
                .zip(&reference)
                .map(|(t, r)| match r.state {
                    Some(s) if t.reached(k) => distance(&t.state.r, &s.r),
                    _ => f64::NAN,
                })
                .collect()
        })
        .collect();
    Ok(Comparison {
        arms: arms.to_vec(),
        provenance: arms.iter().map(provenance).collect(),
        ramp,
        boundary: k,
        panels: panels(&errors, ramp),
        runs,
        reference,
        errors,
    })
}

/// An arm's provenance block: the occupant, its profile and pair-table cell, every step-control value and the
/// discretisation, and how its runs were made.
pub fn provenance(arm: &Arm) -> String {
    let p = arm.profile();
    let pair = Pair::select(arm.stepper, arm.regularisation);
    let table = PairTable::part_2b();
    let c = &arm.step_control;
    let d = &arm.discretisation;
    let registration = match arm.regularisation {
        Choice::AarsethZare => "re-registers at every sync boundary",
        Choice::None => "no registration",
        _ => "registered once at t = 0",
    };
    let warning = pair.warning(&table).unwrap_or("none");
    format!(
        "occupant: {} × {}\n\
         profile: order {}, force_evals {}, symplectic {}, step_reversible {}, run_reversible {}, owns_time_mapping {}\n\
         pair table: {:?}; warning: {warning}; evidence: {}\n\
         step control: eta {:e}, tiny {:e}, predictive_f {:e}, step_budget_rate {:e}, n_min {}, clamp {}, \
         secant_tries {}, bisect_tries {}\n\
         discretisation: delta_sync {:e}, horizon {:e}, last boundary {}\n\
         registration: {registration}\n\
         run: the occupant's direct march at f64, no cadence (no projection, no terminal detection)",
        label(arm.regularisation),
        stepper_name(arm.stepper),
        p.order,
        p.force_evals,
        p.symplectic,
        p.step_reversible,
        p.run_reversible,
        p.owns_time_mapping,
        pair.status(&table),
        table.evidence(arm.stepper, arm.regularisation),
        c.eta,
        c.tiny,
        c.predictive_f,
        c.step_budget_rate,
        c.n_min,
        c.clamp,
        c.secant_tries,
        c.bisect_tries,
        d.delta_sync,
        d.horizon,
        d.boundaries().map_or("none".into(), |k| k.to_string()),
    )
}

// ── Which would have been better ───────────────────────────────────────────────────────────────────────────────

/// The factor by which one arm's error must exceed the other's for the other to be judged better: 10, the factor the
/// Heggie/Aarseth–Zare finding counted pixels by (`err>10`, integrator_contract Part 2b's table).
pub const BETTER_BY: f64 = 10.0;

/// The verdict at one IC between two arms `a` and `b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// `a`'s error is at least [`BETTER_BY`] below `b`'s, and `b`'s is resolved above the reference's uncertainty.
    A,
    /// `b`'s is at least [`BETTER_BY`] below `a`'s, likewise.
    B,
    /// Both resolved, neither better by the factor.
    Tie,
    /// The larger error is below [`BETTER_BY`] times the reference's uncertainty: the reference cannot tell them apart.
    Unresolved,
    /// Not judged: an arm or the reference did not reach the last boundary, or an error is not finite. Counted
    /// apart, never pooled with the judged ICs.
    NotJudged,
}

/// The verdict between errors `ea` and `eb` against a reference of uncertainty `u` (the module's note; the definition
/// integrator_contract Part 2b § "One thing deliberately NOT decided" states).
pub fn verdict(ea: f64, eb: f64, u: f64) -> Verdict {
    if !(ea.is_finite() && eb.is_finite() && u.is_finite()) {
        return Verdict::NotJudged;
    }
    let (lo, hi) = (ea.min(eb), ea.max(eb));
    if hi < BETTER_BY * u {
        Verdict::Unresolved
    } else if hi >= BETTER_BY * lo.max(u) {
        if ea < eb {
            Verdict::A
        } else {
            Verdict::B
        }
    } else {
        Verdict::Tie
    }
}

/// The "which would have been better" report between two arms of a comparison: the verdict at each IC and their tally.
#[derive(Clone, Debug, PartialEq)]
pub struct Better {
    /// The arms `(a, b)`.
    pub arms: (usize, usize),
    /// The verdict at each IC.
    pub verdicts: Vec<Verdict>,
    /// `log₁₀(e_b/e_a)` at each judged IC (`A`, `B` or `Tie`): positive where `a` did better. `NaN` elsewhere.
    pub log_ratio: Vec<f64>,
}

impl Better {
    /// The count of each verdict: `[A, B, Tie, Unresolved, NotJudged]`.
    pub fn tally(&self) -> [usize; 5] {
        let mut t = [0; 5];
        for v in &self.verdicts {
            t[match v {
                Verdict::A => 0,
                Verdict::B => 1,
                Verdict::Tie => 2,
                Verdict::Unresolved => 3,
                Verdict::NotJudged => 4,
            }] += 1;
        }
        t
    }
}

impl Comparison {
    /// The report between arms `a` and `b`. It is data: nothing reads it to choose an occupant.
    pub fn better(&self, a: usize, b: usize) -> Better {
        let verdicts: Vec<Verdict> = (0..self.reference.len())
            .map(|i| {
                verdict(
                    self.errors[a][i],
                    self.errors[b][i],
                    self.reference[i].uncertainty,
                )
            })
            .collect();
        let log_ratio = verdicts
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                Verdict::A | Verdict::B | Verdict::Tie => {
                    (self.errors[b][i] / self.errors[a][i]).log10()
                }
                _ => f64::NAN,
            })
            .collect();
        Better {
            arms: (a, b),
            verdicts,
            log_ratio,
        }
    }

    /// The printed report: each arm's provenance, then per IC each arm's end, steps, re-registrations and error with the
    /// reference's uncertainty, then each pair of arms' verdicts with their tally and the quantiles of `log₁₀` ratio.
    pub fn report(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.provenance.iter().enumerate() {
            out.push_str(&format!("── arm {i} ──\n{p}\n"));
        }
        out.push_str(&format!(
            "ramp: decades {} to {} (shared by every panel)\n",
            self.ramp.lo_decade, self.ramp.hi_decade
        ));
        for (i, r) in self.reference.iter().enumerate() {
            let mut line = format!("ic {i:3}: reference uncertainty {:.3e}", r.uncertainty);
            for (a, row) in self.runs.iter().enumerate() {
                let t = &row[i];
                line.push_str(&format!(
                    " | arm {a}: {:?} at {} steps {} re-reg {} error {:.3e}",
                    t.end, t.boundary, t.steps, t.re_registrations, self.errors[a][i]
                ));
            }
            out.push_str(&line);
            out.push('\n');
        }
        for a in 0..self.arms.len() {
            let mut steps: Vec<u32> = self.runs[a].iter().map(|t| t.steps).collect();
            steps.sort_unstable();
            out.push_str(&format!(
                "arm {a}: steps p50 {}, error quantiles {}\n",
                steps.get(steps.len() / 2).copied().unwrap_or(0),
                quantiles(&self.errors[a])
            ));
        }
        for a in 0..self.arms.len() {
            for b in a + 1..self.arms.len() {
                let better = self.better(a, b);
                let [wa, wb, tie, unresolved, not_judged] = better.tally();
                out.push_str(&format!(
                    "which would have been better, arm {a} vs arm {b}: arm {a} {wa}, arm {b} {wb}, tie {tie}, \
                     unresolved {unresolved}, not judged {not_judged}; log10(e_{b}/e_{a}) quantiles {}\n",
                    quantiles(&better.log_ratio)
                ));
            }
        }
        out
    }
}

/// The finite values' minimum, quartiles and maximum, with how many were finite.
pub fn quantiles(x: &[f64]) -> String {
    let mut v: Vec<f64> = x.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return "none finite".into();
    }
    v.sort_by(f64::total_cmp);
    let q = |f: f64| v[((v.len() - 1) as f64 * f).round() as usize];
    format!(
        "min {:.3e} p25 {:.3e} p50 {:.3e} p75 {:.3e} max {:.3e} ({} of {} finite)",
        q(0.0),
        q(0.25),
        q(0.5),
        q(0.75),
        q(1.0),
        v.len(),
        x.len()
    )
}
