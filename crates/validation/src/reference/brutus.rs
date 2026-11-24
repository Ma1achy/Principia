//! The independent convergence reference, Brutus-style (validation_ground_truth_note, "The independent convergence
//! reference"; TASK-M3-25): a planar three-body integrator in arbitrary-precision binary floating point, run up a
//! ladder of rungs, each with a tighter tolerance and more bits than the last, until two successive rungs give the same
//! answer. That answer is the reference.
//!
//! **Independent of the kernel it judges.** This file uses `std` and `dashu_float` and nothing else: not `kernel`,
//! `engine` or `ledger`, and not `crate::` or `super::`, through which they could be reached. Its right-hand side, its
//! energy and its state are written here, from Newton's law, and the arbitrary-precision library is a dependency of
//! this crate alone, not of the kernel. `validation::reference::brutus_gate::independence` scans this file for any other
//! path and the gate `reference-convergence` runs that scan, with a control that injects a kernel import.
//!
//! **The integrator.** Bulirsch–Stoer (Bulirsch & Stoer 1966; Hairer, Nørsett & Wanner, *Solving ODEs I*, §II.9):
//! Gragg's modified midpoint rule over a step `H` in `n = 2, 4, 6, …` substeps, then Neville's polynomial extrapolation
//! to `h → 0` in `h²`. The state is Cartesian positions and velocities in the frame the start gives; no regularisation,
//! no softening, no change of time. The step is accepted at the first column `k ≥ 2` whose extrapolation-error estimate
//! `‖T_kk − T_k,k−1‖`, scaled ([`scaled`]), is within the rung's tolerance; if none is by the rung's last column, the step
//! is rejected and retaken shorter. The next step's size is chosen by the work per unit time over the columns tried
//! (Hairer, Nørsett & Wanner §II.9, eq. 9.25's rule), raised by an order where the last column tried was the cheapest. Each requested output time is
//! landed on exactly: the step before it is shortened to reach it.
//!
//! **The rungs** ([`Rung`]). A rung of `d` digits has the tolerance `ε = 10⁻ᵈ`, a working precision of
//! `⌈d·log₂10⌉ + `[`GUARD_BITS`] bits, so the arithmetic's own rounding sits 64 bits below the tolerance, and
//! `clamp(4 + d/2, 8, 24)` extrapolation columns, so the method's order rises with the digits asked for (the column `k`
//! is of order `2k`) rather than the step shrinking alone. The plan is unclear here (Brutus's own word-length rule is
//! not in the corpus); chose a guard of 64 bits over the tolerance because it keeps the rounding of a million steps,
//! `√10⁶·2⁻⁶⁴ ≈ 5·10⁻¹⁷` of the tolerance, out of the result, and the ladder's agreement test is what shows it sufficed.
//!
//! **The ladder and the stop rule** ([`Ladder`], [`converge`]). The rungs run in order, `d₀, d₀ + Δ, d₀ + 2Δ, …`. After
//! each rung past the first, the disagreement `δ` between it and the rung before ([`disagreement`]) is measured at
//! every output time, at full precision; the ladder stops at the first rung with `δ ≤ 10^−p` (`p` the ladder's
//! agreement digits), and that rung, the finer of the two, is the accepted result. A rung no finer than the one before,
//! or whose march took exactly the steps and the force evaluations the one before took, refuses the ladder
//! ([`NotConverged::Vacuous`]): its agreement would show only that the two runs were the same run. A rung that fails ([`Failure`]) refuses it, and so does a ladder spent
//! without agreement. A refused ladder has no result: the last rung is never returned in its place.

use dashu_float::round::mode::HalfEven;
use dashu_float::FBig;

/// A binary float, rounded to nearest-even at its own precision.
type Big = FBig<HalfEven, 2>;

/// The bits of working precision beyond the rung's tolerance digits.
pub const GUARD_BITS: usize = 64;

/// The bits beyond the tolerance's digits below which a rung is refused ([`Failure::Unresolvable`]).
pub const RESOLVE_BITS: usize = 8;

/// The fewest and most extrapolation columns a rung has.
pub const COLUMNS: [usize; 2] = [8, 24];

/// One rung of the ladder: a tolerance, a working precision, and how far the extrapolation may go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rung {
    /// The tolerance's digits `d`: `ε = 10⁻ᵈ`.
    pub digits: u32,
    /// The working precision, in bits of significand.
    pub bits: usize,
    /// The extrapolation columns: the step's substep counts are `2, 4, …, 2·columns`.
    pub columns: usize,
}

impl Rung {
    /// The rung of `digits` digits (the module's note).
    pub fn new(digits: u32) -> Self {
        let bits = digit_bits(digits) + GUARD_BITS;
        Self {
            digits,
            bits,
            columns: (4 + digits as usize / 2).clamp(COLUMNS[0], COLUMNS[1]),
        }
    }

    /// The tolerance `ε = 10⁻ᵈ`.
    pub fn tolerance(&self) -> f64 {
        10f64.powi(-(self.digits as i32))
    }
}

/// The bits `d` decimal digits take, `⌈d·log₂10⌉`.
pub fn digit_bits(digits: u32) -> usize {
    (f64::from(digits) * std::f64::consts::LOG2_10).ceil() as usize
}

/// What the start's third column holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Velocities `vᵢ`.
    Velocity,
    /// Momenta `pᵢ = mᵢvᵢ`; the velocity is taken as `pᵢ/mᵢ` at the rung's precision, so a start the kernel holds as
    /// momenta enters with the kernel's own bits.
    Momentum,
}

/// A start: three point masses in the plane, in any units with gravitational constant `g`. Every `f64` is taken as the
/// exact binary number it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start {
    /// The gravitational constant.
    pub g: f64,
    /// The masses, each positive.
    pub m: [f64; 3],
    /// The positions.
    pub r: [[f64; 2]; 3],
    /// The velocities or momenta, as `motion` says.
    pub w: [[f64; 2]; 3],
    /// What `w` holds.
    pub motion: Motion,
}

/// What a run does besides integrating its start (SP-2, TASK-M3-27): both default to nothing, and a run without them
/// is [`run`]'s, step for step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extras {
    /// A decimal offset added to one starting position at the rung's precision, so a published start such as
    /// `x = 1.0000000001` enters exactly as its digits say rather than as the f64 nearest them.
    pub nudge: Option<Nudge>,
    /// An output time at which, once its state is recorded, every velocity is reversed, `vᵢ → −vᵢ`, at the rung's
    /// precision. The march then runs on to the later output times: by time-reversal symmetry it retraces its path, so
    /// at twice this time the exact flow is back at the start.
    pub reverse_at: Option<f64>,
}

/// A decimal offset `numerator·10^−exponent` on body `body`'s coordinate `axis` (0 for `x`, 1 for `y`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nudge {
    /// The body.
    pub body: usize,
    /// The coordinate: 0 for `x`, 1 for `y`.
    pub axis: usize,
    /// The offset's numerator, signed.
    pub numerator: i64,
    /// The power of ten it is divided by.
    pub exponent: u32,
}

/// Why a rung's run stopped short of its last output time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Failure {
    /// A value of the start is not finite, a mass is not positive, `g` is not positive, or the output times are not
    /// finite, positive and increasing.
    Refused,
    /// The rung's precision cannot resolve its tolerance: fewer than `⌈d·log₂10⌉ + `[`RESOLVE_BITS`] bits. Its error
    /// estimates would be rounding, and its steps would shrink until they happened to fall below the tolerance.
    Unresolvable {
        /// The rung's digits.
        digits: u32,
        /// Its bits.
        bits: usize,
    },
    /// Two bodies coincide: the force is infinite.
    Collision {
        /// The time it happened at.
        t: f64,
    },
    /// The step shrank below the time's own resolution at the rung's precision without converging.
    Underflow {
        /// The time it happened at.
        t: f64,
    },
    /// The step budget was spent.
    Exhausted {
        /// The time reached.
        t: f64,
    },
}

/// The state at one output time.
#[derive(Clone, Debug)]
pub struct Output {
    /// The time.
    pub t: f64,
    /// The positions, rounded to f64.
    pub r: [[f64; 2]; 3],
    /// The velocities, rounded to f64.
    pub v: [[f64; 2]; 3],
    /// `(E − E₀)/|E₀|`, the energy `E = Σ½mᵢ|vᵢ|² − Σ g·mᵢmⱼ/rᵢⱼ` taken at the rung's precision.
    pub energy_error: f64,
    /// `L_z − L_z₀` over `Σ mᵢ|rᵢ||vᵢ|` at the start (or 1 where that is 0), at the rung's precision.
    pub lz_error: f64,
    /// The masses, for the agreement's scales.
    masses: [f64; 3],
    /// The phase-space point at the rung's precision: `x₀ y₀ x₁ y₁ x₂ y₂` then the velocities.
    y: Vec<Big>,
}

/// One rung's run.
#[derive(Clone, Debug)]
pub struct Run {
    /// The rung.
    pub rung: Rung,
    /// The state at each output time.
    pub outputs: Vec<Output>,
    /// The masses times `g`, in f64, for the agreement's velocity scale.
    gm: f64,
    /// Steps accepted.
    pub accepted: u64,
    /// Steps rejected.
    pub rejected: u64,
    /// Force evaluations, accepted and rejected steps together: the run's work.
    pub evaluations: u64,
}

/// The ladder: which rungs, how far up, and when two rungs agree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ladder {
    /// The first rung's digits `d₀`.
    pub first: u32,
    /// The digits added per rung `Δ`.
    pub step: u32,
    /// The most rungs run.
    pub rungs: u32,
    /// The agreement digits `p`: two successive rungs agree when their disagreement is at most `10^−p`.
    pub agree: u32,
    /// Each rung's step budget, accepted and rejected steps together.
    pub max_steps: u64,
    /// A control's alone: holds every rung's precision at this many bits while the tolerance tightens. `None` in every
    /// consumer's ladder.
    pub fixed_bits: Option<usize>,
}

impl Ladder {
    /// The `j`-th rung (from 0).
    pub fn rung(&self, j: u32) -> Rung {
        let mut rung = Rung::new(self.first + j * self.step);
        if let Some(bits) = self.fixed_bits {
            rung.bits = bits;
        }
        rung
    }

    /// The agreement threshold `10^−p`.
    pub fn threshold(&self) -> f64 {
        10f64.powi(-(self.agree as i32))
    }
}

/// A converged ladder: every rung run, the disagreement between each and the one before, and the rung accepted.
#[derive(Clone, Debug)]
pub struct Converged {
    /// The runs, in ladder order; the last is the accepted one.
    pub runs: Vec<Run>,
    /// `disagreement[j]` between `runs[j + 1]` and `runs[j]`.
    pub disagreement: Vec<f64>,
}

impl Converged {
    /// The accepted run: the finer of the two rungs that agreed.
    pub fn accepted(&self) -> &Run {
        self.runs
            .last()
            .expect("a converged ladder has run two rungs")
    }
}

/// Why the ladder has no result.
#[derive(Clone, Debug)]
pub enum NotConverged {
    /// The ladder was spent without two successive rungs agreeing.
    Spent {
        /// The runs.
        runs: Vec<Run>,
        /// The disagreements, as [`Converged::disagreement`].
        disagreement: Vec<f64>,
    },
    /// A rung is no finer than the rung before it, or marched exactly as it did: the same accepted steps and the same
    /// force evaluations.
    Vacuous {
        /// The rung's digits.
        digits: u32,
        /// The rung before's.
        before: u32,
        /// The rung's accepted steps.
        steps: u64,
        /// Its force evaluations.
        evaluations: u64,
    },
    /// A rung's run failed.
    Failed {
        /// The rung.
        rung: Rung,
        /// Why.
        failure: Failure,
        /// The runs before it, and the disagreements between them.
        runs: Vec<Run>,
        /// The disagreements.
        disagreement: Vec<f64>,
    },
}

impl std::fmt::Display for NotConverged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spent { runs, disagreement } => write!(
                f,
                "not converged: {} rungs run, up to {} digits, and no two successive ones agreed (disagreements {})",
                runs.len(),
                runs.last().map_or(0, |r| r.rung.digits),
                disagreement
                    .iter()
                    .map(|d| format!("{d:.3e}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Vacuous {
                digits,
                before,
                steps,
                evaluations,
            } => write!(
                f,
                "not converged: vacuous, the {digits}-digit rung ({steps} steps, {evaluations} evaluations) is no \
                 finer than the {before}-digit rung before it, or marched exactly as it did"
            ),
            Self::Failed { rung, failure, .. } => write!(
                f,
                "not converged: the {}-digit rung at {} bits failed: {failure:?}",
                rung.digits, rung.bits
            ),
        }
    }
}

// ── Arithmetic at the rung's precision ─────────────────────────────────────────────────────────────────────────────

/// Every number a run makes passes through here: `dashu_float` gives an `f64` 53 bits and an integer its own bit
/// length, and an operation the larger of its operands' precisions, so a constant not raised to the rung's precision
/// would round the arithmetic it enters at its own.
struct Ctx {
    bits: usize,
    g: Big,
    m: [Big; 3],
}

impl Ctx {
    fn big(&self, x: f64) -> Big {
        Big::try_from(x)
            .expect("a finite f64")
            .with_precision(self.bits)
            .value()
    }

    fn int(&self, n: u64) -> Big {
        Big::from(n).with_precision(self.bits).value()
    }

    /// The right-hand side: `ṙᵢ = vᵢ`, `v̇ᵢ = Σⱼ g·mⱼ(rⱼ − rᵢ)/|rⱼ − rᵢ|³`. `None` where two bodies coincide.
    fn rhs(&self, y: &[Big]) -> Option<Vec<Big>> {
        let mut out: Vec<Big> = Vec::with_capacity(12);
        out.extend(y[6..12].iter().cloned());
        let mut a: Vec<Big> = (0..6).map(|_| self.int(0)).collect();
        for i in 0..3 {
            for j in (i + 1)..3 {
                let dx = &y[2 * j] - &y[2 * i];
                let dy = &y[2 * j + 1] - &y[2 * i + 1];
                let d2 = &dx * &dx + &dy * &dy;
                if d2 == Big::ZERO {
                    return None;
                }
                let d3 = &d2 * d2.sqrt();
                let gx = &self.g * &dx / &d3;
                let gy = &self.g * &dy / &d3;
                a[2 * i] += &self.m[j] * &gx;
                a[2 * i + 1] += &self.m[j] * &gy;
                a[2 * j] -= &self.m[i] * &gx;
                a[2 * j + 1] -= &self.m[i] * &gy;
            }
        }
        out.extend(a);
        Some(out)
    }

    /// The energy `Σ½mᵢ|vᵢ|² − Σ g·mᵢmⱼ/rᵢⱼ`.
    fn energy(&self, y: &[Big]) -> Big {
        let half = self.int(1) / self.int(2);
        let mut e = self.int(0);
        for i in 0..3 {
            let v2 = &y[6 + 2 * i] * &y[6 + 2 * i] + &y[7 + 2 * i] * &y[7 + 2 * i];
            e += &half * &self.m[i] * v2;
        }
        for i in 0..3 {
            for j in (i + 1)..3 {
                let dx = &y[2 * j] - &y[2 * i];
                let dy = &y[2 * j + 1] - &y[2 * i + 1];
                let d = (&dx * &dx + &dy * &dy).sqrt();
                e -= &self.g * &self.m[i] * &self.m[j] / d;
            }
        }
        e
    }

    /// The angular momentum `Σ mᵢ(xᵢv_yᵢ − yᵢv_xᵢ)`.
    fn lz(&self, y: &[Big]) -> Big {
        let mut l = self.int(0);
        for i in 0..3 {
            l += &self.m[i] * (&y[2 * i] * &y[7 + 2 * i] - &y[2 * i + 1] * &y[6 + 2 * i]);
        }
        l
    }

    /// Gragg's modified midpoint rule over `big_h` in `n` substeps (Hairer, Nørsett & Wanner eq. 9.13), with its
    /// smoothing step.
    fn midpoint(&self, y: &[Big], big_h: &Big, n: u64) -> Option<Vec<Big>> {
        let h = big_h / self.int(n);
        let h2 = &h * self.int(2);
        let f0 = self.rhs(y)?;
        let mut z0: Vec<Big> = y.to_vec();
        let mut z1: Vec<Big> = y.iter().zip(&f0).map(|(a, b)| a + &h * b).collect();
        for _ in 1..n {
            let f = self.rhs(&z1)?;
            let z2: Vec<Big> = z0.iter().zip(&f).map(|(a, b)| a + &h2 * b).collect();
            z0 = std::mem::replace(&mut z1, z2);
        }
        let f = self.rhs(&z1)?;
        let half = self.int(1) / self.int(2);
        Some(
            z1.iter()
                .zip(&z0)
                .zip(&f)
                .map(|((a, b), c)| &half * (a + b + &h * c))
                .collect(),
        )
    }
}

/// `|x|` of a big as an f64.
fn f(x: &Big) -> f64 {
    x.to_f64().value()
}

/// The scales the error is measured in, from the phase-space point `y` (in f64): the system's size `R`, the largest
/// distance of a body from the centre of mass, and its speed `V`, the largest speed relative to the centre of mass's,
/// floored at the virial speed `√(gM/R)` so a start at rest has one.
fn scales(y: &[f64; 12], m: &[f64; 3], gm: f64) -> (f64, f64) {
    let total = m[0] + m[1] + m[2];
    let com =
        |o: usize| [0, 1].map(|c| (0..3).map(|i| m[i] * y[o + 2 * i + c]).sum::<f64>() / total);
    let (rc, vc) = (com(0), com(6));
    let far = |o: usize, c: [f64; 2]| {
        (0..3)
            .map(|i| (y[o + 2 * i] - c[0]).hypot(y[o + 2 * i + 1] - c[1]))
            .fold(0.0f64, f64::max)
    };
    let size = far(0, rc);
    let speed = far(6, vc).max((gm / size).sqrt());
    (size, speed)
}

/// The scaled distance between two phase-space points `a` and `b`, at full precision until the differences are taken:
/// the largest, over the bodies, of `|Δrᵢ|/R` and `|Δvᵢ|/V`, the scales [`scales`] of `b`.
pub fn scaled(a: &[Big], b: &[Big], m: &[f64; 3], gm: f64) -> f64 {
    let bf: [f64; 12] = std::array::from_fn(|k| f(&b[k]));
    let (size, speed) = scales(&bf, m, gm);
    let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| f(&(x - y))).collect();
    (0..3)
        .map(|i| {
            let dr = d[2 * i].hypot(d[2 * i + 1]) / size;
            let dv = d[6 + 2 * i].hypot(d[7 + 2 * i]) / speed;
            dr.max(dv)
        })
        .fold(0.0f64, f64::max)
}

/// [`scaled`] on two phase-space points held in f64, `x₀ y₀ x₁ y₁ x₂ y₂` then the velocities: the largest, over the
/// bodies, of `|Δrᵢ|/R` and `|Δvᵢ|/V`, the scales [`scales`] of `b`. For a state compared with a published one, whose
/// digits past f64's are not the comparison's.
pub fn scaled_f64(a: &[f64; 12], b: &[f64; 12], m: &[f64; 3], gm: f64) -> f64 {
    let (size, speed) = scales(b, m, gm);
    (0..3)
        .map(|i| {
            let dr = (a[2 * i] - b[2 * i]).hypot(a[2 * i + 1] - b[2 * i + 1]) / size;
            let dv = (a[6 + 2 * i] - b[6 + 2 * i]).hypot(a[7 + 2 * i] - b[7 + 2 * i]) / speed;
            dr.max(dv)
        })
        .fold(0.0f64, f64::max)
}

/// The [`scaled`] distance between two runs' states at output `k`, at full precision, the scales the second run's:
/// [`disagreement`] at one output time. Infinite where either run has no output `k`.
pub fn disagreement_at(coarse: &Run, fine: &Run, k: usize) -> f64 {
    match (coarse.outputs.get(k), fine.outputs.get(k)) {
        (Some(a), Some(b)) => scaled(&a.y, &b.y, &b.masses, fine.gm),
        _ => f64::INFINITY,
    }
}

/// The [`scaled`] distance of `run`'s state at output `k` from a published state given as decimal strings,
/// `x₀ y₀ x₁ y₁ x₂ y₂` then the velocities, each read exactly and rounded to the run's precision, so the comparison
/// resolves digits past f64's. The scales are the run's state's. `Err` names a string that is not a decimal number,
/// or an output the run does not have.
pub fn scaled_to_decimal(run: &Run, k: usize, b: &[&str; 12]) -> Result<f64, String> {
    let o = run
        .outputs
        .get(k)
        .ok_or_else(|| format!("the run has no output {k}"))?;
    let bits = run.rung.bits;
    let big: Vec<Big> = b
        .iter()
        .map(|s| {
            s.trim()
                .parse::<FBig<HalfEven, 10>>()
                .map(|d| {
                    d.with_base_and_precision::<2>(bits)
                        .value()
                        .with_precision(bits)
                        .value()
                })
                .map_err(|e| format!("`{s}` is not a decimal number: {e:?}"))
        })
        .collect::<Result<_, _>>()?;
    Ok(scaled(&big, &o.y, &o.masses, run.gm))
}

/// The disagreement between two runs of the same start and output times: the largest [`scaled`] distance between their
/// states at any output time, the scales the second run's. Infinite where they reached different numbers of outputs.
pub fn disagreement(coarse: &Run, fine: &Run) -> f64 {
    if coarse.outputs.len() != fine.outputs.len() {
        return f64::INFINITY;
    }
    coarse
        .outputs
        .iter()
        .zip(&fine.outputs)
        .map(|(a, b)| scaled(&a.y, &b.y, &b.masses, fine.gm))
        .fold(0.0f64, f64::max)
}

impl Output {
    fn new(
        ctx: &Ctx,
        t: f64,
        y: Vec<Big>,
        e0: &Big,
        l0: &Big,
        l_scale: f64,
        masses: [f64; 3],
    ) -> Self {
        let e = ctx.energy(&y);
        let magnitude = if f(e0) < 0.0 { -e0.clone() } else { e0.clone() };
        let energy_error = f(&((&e - e0) / magnitude));
        let lz_error = f(&(ctx.lz(&y) - l0)) / l_scale;
        Self {
            t,
            r: [0, 1, 2].map(|i| [f(&y[2 * i]), f(&y[2 * i + 1])]),
            v: [0, 1, 2].map(|i| [f(&y[6 + 2 * i]), f(&y[7 + 2 * i])]),
            energy_error,
            lz_error,
            masses,
            y,
        }
    }
}

/// `start` integrated by one rung, landing on each of `times` (finite, positive, increasing), within `max_steps` step
/// attempts.
pub fn run(start: &Start, times: &[f64], rung: Rung, max_steps: u64) -> Result<Run, Failure> {
    run_with(start, times, rung, max_steps, &Extras::default())
}

/// [`run`] with `extras`: a nudge on the start, a velocity reversal at an output time ([`Extras`]). A reversal time that
/// is not one of `times` is refused.
pub fn run_with(
    start: &Start,
    times: &[f64],
    rung: Rung,
    max_steps: u64,
    extras: &Extras,
) -> Result<Run, Failure> {
    let finite = start
        .m
        .iter()
        .chain(start.r.iter().flatten())
        .chain(start.w.iter().flatten())
        .chain([&start.g])
        .all(|x| x.is_finite());
    let increasing = times.first().is_some_and(|&t| t > 0.0)
        && times.iter().all(|t| t.is_finite())
        && times.windows(2).all(|w| w[0] < w[1]);
    let reverse_ok = extras.reverse_at.is_none_or(|t| times.contains(&t));
    let nudge_ok = extras.nudge.is_none_or(|n| n.body < 3 && n.axis < 2);
    if !finite
        || start.m.iter().any(|&m| m <= 0.0)
        || start.g <= 0.0
        || !increasing
        || !reverse_ok
        || !nudge_ok
    {
        return Err(Failure::Refused);
    }
    let bits = rung.bits;
    if bits < digit_bits(rung.digits) + RESOLVE_BITS {
        return Err(Failure::Unresolvable {
            digits: rung.digits,
            bits,
        });
    }
    let mut ctx = Ctx {
        bits,
        g: Big::ZERO,
        m: [Big::ZERO, Big::ZERO, Big::ZERO],
    };
    ctx.g = ctx.big(start.g);
    ctx.m = start.m.map(|m| ctx.big(m));
    let mut y: Vec<Big> = Vec::with_capacity(12);
    for i in 0..3 {
        y.push(ctx.big(start.r[i][0]));
        y.push(ctx.big(start.r[i][1]));
    }
    if let Some(n) = extras.nudge {
        let ten = ctx.int(10);
        let mut scale = ctx.int(1);
        for _ in 0..n.exponent {
            scale = &scale * &ten;
        }
        let offset = ctx.int(n.numerator.unsigned_abs()) / scale;
        let k = 2 * n.body + n.axis;
        y[k] = if n.numerator < 0 {
            &y[k] - &offset
        } else {
            &y[k] + &offset
        };
    }
    for i in 0..3 {
        for c in 0..2 {
            let w = ctx.big(start.w[i][c]);
            y.push(match start.motion {
                Motion::Velocity => w,
                Motion::Momentum => w / &ctx.m[i],
            });
        }
    }
    let gm = start.g * (start.m[0] + start.m[1] + start.m[2]);
    let e0 = ctx.energy(&y);
    let mut l0 = ctx.lz(&y);
    let l_scale = {
        let s: f64 = (0..3)
            .map(|i| {
                start.m[i]
                    * f(&y[2 * i]).hypot(f(&y[2 * i + 1]))
                    * f(&y[6 + 2 * i]).hypot(f(&y[7 + 2 * i]))
            })
            .sum();
        if s > 0.0 {
            s
        } else {
            1.0
        }
    };
    let eps = rung.tolerance();
    let columns = rung.columns;
    let seq: Vec<u64> = (1..=columns as u64).map(|j| 2 * j).collect();
    // The work to reach column k: Σ_{i ≤ k} (n_i + 1) force evaluations.
    let work: Vec<f64> = seq
        .iter()
        .scan(0.0, |acc, &n| {
            *acc += (n + 1) as f64;
            Some(*acc)
        })
        .collect();
    // Neville's coefficients 1/((n_k/n_{k−j})² − 1), at the rung's precision.
    let coef = |k: usize, j: usize| -> Big {
        let (a, b) = (seq[k], seq[k - j]);
        ctx.int(b * b) / ctx.int(a * a - b * b)
    };

    let yf = |y: &[Big]| -> [f64; 12] { std::array::from_fn(|k| f(&y[k])) };
    let (size, speed) = scales(&yf(&y), &start.m, gm);
    let mut h = 1e-2 * size / speed;
    let mut t = ctx.int(0);
    let mut outputs = Vec::with_capacity(times.len());
    let (mut accepted, mut rejected, mut evaluations) = (0u64, 0u64, 0u64);
    let resolution = 2f64.powi(-(bits.min(1000) as i32) + 8);

    for &t_out in times {
        let target = ctx.big(t_out);
        loop {
            let tf = f(&t);
            if accepted + rejected >= max_steps {
                return Err(Failure::Exhausted { t: tf });
            }
            if h <= resolution * tf.abs().max(t_out) {
                return Err(Failure::Underflow { t: tf });
            }
            let remaining = &target - &t;
            let landing = h >= f(&remaining);
            let big_h = if landing { remaining } else { ctx.big(h) };
            let hf = f(&big_h);
            // The extrapolation tableau's last row, and each column's error estimate.
            let mut row: Vec<Vec<Big>> = Vec::with_capacity(columns);
            let mut errs: Vec<f64> = Vec::with_capacity(columns);
            let mut done: Option<(usize, Vec<Big>)> = None;
            for k in 0..columns {
                evaluations += seq[k] + 1;
                let Some(base) = ctx.midpoint(&y, &big_h, seq[k]) else {
                    return Err(Failure::Collision { t: tf });
                };
                let mut new_row: Vec<Vec<Big>> = Vec::with_capacity(k + 1);
                new_row.push(base);
                for j in 1..=k {
                    let c = coef(k, j);
                    let next: Vec<Big> = new_row[j - 1]
                        .iter()
                        .zip(&row[j - 1])
                        .map(|(a, b)| a + (a - b) * &c)
                        .collect();
                    new_row.push(next);
                }
                if k >= 1 {
                    let err = scaled(&new_row[k], &new_row[k - 1], &start.m, gm);
                    errs.push(err);
                    if k >= 2 && err <= eps {
                        done = Some((k, new_row.pop().expect("row k")));
                        break;
                    }
                }
                row = new_row;
            }
            match done {
                Some((k, y_new)) => {
                    accepted += 1;
                    y = y_new;
                    t = if landing { target.clone() } else { &t + &big_h };
                    // The next step by the least work per unit time over the columns that may accept (j ≥ 2;
                    // errs[j − 1] is column j's), raising the order by one where the last column tried is the
                    // cheapest and one is left (Hairer, Nørsett & Wanner §II.9, eq. 9.25 and its order rule).
                    let mut best = (f64::INFINITY, hf, 0);
                    for j in 2..=k {
                        let err = errs[j - 1];
                        let factor = if err > 0.0 {
                            (0.94 * (0.65 * eps / err).powf(1.0 / (2 * j + 1) as f64))
                                .clamp(0.1, 4.0)
                        } else {
                            4.0
                        };
                        let hj = hf * factor;
                        let w = work[j] / hj;
                        if w < best.0 {
                            best = (w, hj, j);
                        }
                    }
                    if best.2 == k && k + 1 < columns {
                        best.1 *= work[k + 1] / work[k];
                    }
                    // A landing step was shortened to land; the free step it stood in for sets the next.
                    h = if landing {
                        best.1.max(h.min(4.0 * best.1))
                    } else {
                        best.1
                    };
                    if landing {
                        break;
                    }
                }
                None => {
                    rejected += 1;
                    let err = errs.last().copied().unwrap_or(f64::INFINITY);
                    let factor = if err.is_finite() && err > 0.0 {
                        (0.94 * (0.65 * eps / err).powf(1.0 / (2 * columns - 1) as f64))
                            .clamp(0.1, 0.5)
                    } else {
                        0.1
                    };
                    h = hf * factor;
                }
            }
        }
        outputs.push(Output::new(
            &ctx,
            t_out,
            y.clone(),
            &e0,
            &l0,
            l_scale,
            start.m,
        ));
        // The reversal: `vᵢ → −vᵢ` exactly (a sign at full precision), after the state is recorded. `L_z` changes sign
        // with the velocities, so its reference does too; the energy is even in them and keeps its own.
        if extras.reverse_at == Some(t_out) {
            for v in &mut y[6..12] {
                *v = -v.clone();
            }
            l0 = -l0;
        }
    }
    Ok(Run {
        rung,
        outputs,
        gm,
        accepted,
        rejected,
        evaluations,
    })
}

/// The ladder run on `start` to `times` (the module's note): the converged result, or why there is none.
pub fn converge(start: &Start, times: &[f64], ladder: &Ladder) -> Result<Converged, NotConverged> {
    converge_with(start, times, ladder, &Extras::default())
}

/// [`converge`], each rung run with `extras` ([`run_with`]).
pub fn converge_with(
    start: &Start,
    times: &[f64],
    ladder: &Ladder,
    extras: &Extras,
) -> Result<Converged, NotConverged> {
    let mut runs: Vec<Run> = Vec::new();
    let mut disagreement_seen: Vec<f64> = Vec::new();
    for j in 0..ladder.rungs {
        let rung = ladder.rung(j);
        let this = match run_with(start, times, rung, ladder.max_steps, extras) {
            Ok(r) => r,
            Err(failure) => {
                return Err(NotConverged::Failed {
                    rung,
                    failure,
                    runs,
                    disagreement: disagreement_seen,
                })
            }
        };
        if let Some(before) = runs.last() {
            // The runs must be different computations: a finer rung, and a different march (steps or work).
            let same_march =
                this.accepted == before.accepted && this.evaluations == before.evaluations;
            if rung.digits <= before.rung.digits || same_march {
                return Err(NotConverged::Vacuous {
                    digits: rung.digits,
                    before: before.rung.digits,
                    steps: this.accepted,
                    evaluations: this.evaluations,
                });
            }
            let d = disagreement(before, &this);
            disagreement_seen.push(d);
            runs.push(this);
            if d <= ladder.threshold() {
                return Ok(Converged {
                    runs,
                    disagreement: disagreement_seen,
                });
            }
        } else {
            runs.push(this);
        }
    }
    Err(NotConverged::Spent {
        runs,
        disagreement: disagreement_seen,
    })
}
